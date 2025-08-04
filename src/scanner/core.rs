//! Core MEV scanner implementation
//!
//! The MevScanner orchestrates all components to detect arbitrage opportunities
//! by monitoring blockchain state changes and anaylyzing price differences.

use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}}, 
    time::{Duration, Instant},
};
use tokio::{
    time::{sleep, timeout},
    sync::{Notify, Mutex},
};
use ethers::{
    providers::{Provider, Ws,Http, Middleware, StreamExt},
    types::{Address,Block,H256,Transaction,Log, U256},
};

use anyhow::{anyhow, Result};
use tracing::{info,debug,warn,error};


use crate::{
    pools::{PoolManger, PoolState},
    arbitrage::{ArbitrageDetector, ArbitrageOpporutnity},
    mempool::MempoolWatcher,
    providers::ProviderManager,
    cache::StateCache,
    config::ScannerConfig,
}; 
use super::{BloomFilter, CircuitBreaker};


const WS_TIMEOUT: Duration = Duration::from_secs(30);
const BLOCK_PROCESSING_TIMEOUT: Duration= Duration::from_secs(10);
const MAX_BLOCK_BATCH_SIZE: usize = 5;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_CONSECUTIVE_ERRORS: usize = 3;
const MAX_ERRORS_BEFORE_TRIP: usize = 5;
const HTTP_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SLOT_CACHE_SIZE: usize =  10_000;
/// Main MEV scanner that coordinates all components
pub struct MevScanner {

    /// Primary WebSocket provider (wrapped in Mutex for reconnection)
    primary_provider: Arc<Mutex<Arc<Provider<Ws>>>>,

    // Fallback HTTP provider
    fallback_provider: Arc<Provider<Http>>,

    /// WebSocket url
    ws_endpoint: String,

    ///Manages RPC providers with failover
    provider_manager: ProviderManager,

    /// Handles pool state and reserver data 
    pool_manager: PoolManger,

    ///Detect arbtirage opportunities
    arbitrage_detector: ArbitrageDetector,

    /// Monitors mempool for relevant transactions
    mempool_watcher: Arc<Mutex<MempoolWatcher>>,

    /// Caches blockchain state
    state_cache: StateCache,
    slot_cache: SlotCache,

    /// Circuit breaker for fault tolerance
    circuit_breaker: CircuitBreaker,

    /// Scanner configuration
    config: ScannerConfig,

    /// Last processed block number
    last_block: AtomicU64,

    /// Connection state
    connection_state: Arc<Mutex<ConnectionState>>,

    /// Notifies main loop when WS reconnects
    ws_reconnected: Arc<Notify>,
}

struct ConnectionState {
    ws_connected: bool, 
    last_success: Instant,
    consecutive_errors: usize
    reconnect_attempts: usize,
}

impl ConnectionState {
    fn default () -> Self{
        Self{
            ws_connected: false,
            consecutive_errors: 0,
            last_success: Instant::now(),

            reconnect_attempts:0,
        }
    }
}

impl MevScanner {
    /// Creates a new MEV scanner with the given configuration
    pub async fn new(config: ScannerConfig) -> Result<Self> {

        // Extract endpoints using config
        let ws_endpoing = config.primary_provider.clone()
            .ok_or_else(|| anyhow!("WebSocket URL required in config!!!!"))?;
        let http_endpoint = config.fallback_provider.clone();

        // Initialize HTTP provider (always available)
        let fallback_provider = Arc::new(
            Provider::<Http>::try_from(&http_endpoint)
                .context("Failed to create HTTP provider")?
        );

        // Try to initalize WebSocket provider
        let primary_provider = match Provider::<Ws>::connect(&ws_endpoint).await{
            Ok(ws_provider) => {
                info!("✅ WebSocket provider connected successfully......");
                Arc::new(Mutex::new(Arc::new(ws_provider)))
            }
            Err(e) => {
                warn!("⚠️ WebSocket connection failed, will retry: {}", e);
                // Create a placeholder - will be replaced on reconnect
                let dummy_ws =  create_dummy_ws_provider().await?;
                Arc::new(Mutex::new(Arc::new(dummy_ws)))
            }
        };

        let provider_manager = ProviderManager::new(
            &config.primary_rpc_url,
            &config.fallback_rpc_url,
        ).await?;

        let ws_endpoint = &config.primary_rpc_url;

        let pool_manager = PoolManger::new();
        let arbitrage_detector = ArbitrageDetector::new(config.min_profit_threshold);

        let mempool_watcher = if let Some(ws_url) = &config.primary_rpc_url {
            let mut watcher = MempoolWatcher::new();
            watcher.initialize(ws_url).await?;
            Arc::new(Mutex::new(watcher))
        } else {
            Arc::new(Mutex::new(MempoolWatcher::new()))
        };

        let state_cache = StateCache::new(
            config.cache_capacity,
            Duration::from_secs(config.cache_ttl_seconds),
        );

        let slot_cache = SlotCache::new(SLOT_CACHE_SIZE);

        let circut_breaker = CircuitBreaker::new(
            config.circuit_breaker_threshold,
            Duration::from_secs(config.circuit_breaker_cooldown_seconds),
        );

        Ok(Self {
            ws_endpoint,
            primary_provider,
            fallback_provider,
            provider_manager,
            pool_manager,
            arbitrage_detector,
            mempool_watcher,
            state_cache,
            slot_cache,
            circut_breaker,
            config,
            last_block: Mutex::new(0),
            connection_state: Arc::new(Mutex::new(ConnectionState::default())),
            ws_reconnected: Arc::new(Notify::new()),
        })
    }


    /// Adds pools to monitor for arbitrage opportunities 
    pub async fn run_cycle(
        &self, 
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<Vec<ArbitrageOpporutnity>, ScannerError> {
        info!("Starting MEV Scanner main loop .................");

        // Start mempool monitoring task 
        let mempool_task =self.start_mempool_monitoring();

        loop{
            tokio::select!{
                _ = shutdown_rx.recv() => {
                    info!("🛑 Shutdown signal received. Exiting run cycle loop....");
                    mempool_taks.abort();
                    break Ok(());
                }
                _ = async {
                    if self.circuit_breaker.is_tripped().await {
                        warn!("⚠️ Circuit breaker tripped, cooling down.....");
                        sleep(self.circuit_breaker.cool_down).await;
                        return;
                    }
                    if self.connection_state.lock().await.ws_connected{
                        match self.process_ws_blocks().await{
                            Ok(_) => {
                                let mut state = self.connection_state.lock().await;
                                state.last_success= Instant::now();
                                state.consecutive_errors = 0;
                                self.circuit_breaker.reset(); // do we need to reset it all the time ???
                            }
                            Err(e) => {
                                warn!("❌ WS processing error: {:?}",e);
                                self.connection_state.lock().await.ws_connected = false;
                                self.circuit_breaker.trip().await;
                            }
                        }
                        
                    } else {
                        match self.process_http_polling().await {
                            Ok(_) => {
                                self.connection_state.lock().await.last_success = Instant::now();
                                self.try_reconnect_ws().await;
                            }
                            Err(e) => {
                                error! ("HTTP process error: {:?}", e);
                                self.circut_breaker.trip().await;
                                sleep(RECONNECT_DELAY).await;
                            }
                        }
                    }

                } =>{}
            }

        }
    }

    fn start_mempool_monitoring(&self) -> tokio::task::JoinHandle<()>{
        let mempool_watcher = self.mempool_watcher.clone();
        let arbitrage_detector = self.arbitrage_detector.clone();

        tokio::spawn(async move{
            loop {
                if let Ok(mut watcher) = mempool_watcher.try_lock() {
                    if let Ok(pending_txs) = watcher.get_pending_transactions().await {
                        for tx in pendint_txs {
                            if let Ok(opportunities) = arbitrage_detector.analyze_transaction(&tx).await {
                                info!("📊 Mempool arbitrage opportunity")
                                self.handle_opportunities(opportunities).await;
                            }
                        }
                    }
                }
            }
        })
    }

    pub async fn process_ws_blocks(&self) -> Resutl<()> {
        let provider = self.primary_provider.lock().clone();
        let mut stream = provider
            .subscribe_blocks()
            .await
            .context("Failed to subscribe to blocks")?;
        info!("📡 WebSocket block subscription established");

        while let Some(block) = stream.next().await{
            self.update_connection_success().await;

            if let Err(e) = self.process_block_immediately(block).await {
                error!("❌ Block processing error: {;?}, e");
                self.handle_connection_error().await;

                if self.connection_state.lock().await.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    return Err(e);
                }
            }
        }

        warn!("⚠️ WebSocket stream ended");
        Err(anyhow!("WebSocket stream terminated"))
    }

    async fn process_http_polling(&self) -> Resutl<()> {
        info!("📊 Falling back to HTTP polling mode");
        loop {  
            tokio::select!{
                _ = sleep(HTTP_POLL_INTERVAL).await;
                let latest_block = self.fallback_provider.get_block_number().await?.as_u64();
                let last_processed = self.last_block.load(Ordering::Relaxed);

                if lastest_block > last_processed {
                    // Process all missed blocks individually for maximum speed 
                    for block_num in (last_processed+1..=lastest_block) {
                        if let Err(e) = self.process_single_block_by_number(block_number).await {
                            error!("HTTP block {} processing failed: {:?}", block_num, e);
                            continue;
                        }
                        self.last_block.store(block_num, Ordering::Relaxed);
                    }
                }

                // Try to reconnect WebSocket periodically
                if self.should_attempt_ws_reconnect().await {
                    info!("🔃 Time to attempt WebSocket reconnect, exiting HTTP fallback");
                    break; // Exit HTTP polling to retry Websocket
                }

                _ = self.ws_connected.notified() = {
                    info!("✅ WebSocket reconnected, exiting HTTP fallback");
                   
                }

            }
        }

        Ok(())
      
    }

    async fn process_block_immediately(&self, block: Block<H256>) -> Resutl<()>{
        let start_time = Instant::now();
        if let Some(number) = block.number.map(|n| n.as_u64) {
            debug!("⚡️ Processing block {} immediately", number);

            match timeout(BLOCK_PROCESSING_TIMEOUT, self.process_single_block(number)).await {
                Ok(OK(opportunities)) => {
                    if !opportunities.is_emppty() {
                        info!("💰 Found {} MEV opportunities in block {} ({}ms)",opportunities.len(), number, start_time.elapsed().as_millis());
                        self.handle_opportunities(opportunities).await;
                    }
                    self.last_block.store(number,Ordering::Relaxed);
                }
                Ok(Err(e))=> {
                    error!("❌ Block {} processing failed: {:?}", number, e);
                    return Err(e);
                }
                Err(_) => {
                    warn!("⏱️ Block {} processing timed out after {}ms", number, BLOCK_PROCESSING_TIMEOUT.as_millis());
                }
            }
        }

        Ok(())
    }

    async fn process_single_block_by_number(number: u64) -> Result<Vec<ArbitrageOpportunity>> {
        let block = self.fallback_provider
            .get_block_with_txs(number)
            .await?
            .ok_or_else(|| anyhow!("Block {} not found", block_number))?;
        self.process_single_block(block).await
    }


    async fn process_single_block(&self, block: Block<H256>) -> Resutl<Vec<ArbitrageOpportunity>> {
        let block_number = block.number
            .ok_or_else(|| anyhow!("Block missing number!!!!"))?
            .as_u64();
        // 1. Detect changed storage slots via bloom filter and transaction analysis
        let changed_slots = self.detect_changed_slots(&block).await?;

        // 2. Update slot cache
        changed_slots
            .iter()
            .for_each(|&(slot_key,value)|{
                self.slot_cache.insert(slot_key,value);
                self.bloom_filter.insert(&slot_key.to_le_bytes());
            });

        // 3. Get modified pools based on chanted slots
        let modified_pools = self.get_modified_pools(&changed_slots).await?;

        // 4. Find arbitrge opportunities
        let mut opportunities = self.find_arbitrage(modified_pools).await?;
        
        // 5. Analyze individual transactions for MEV opportunities
        if let Some(transactions) = &block.transaction {
            for tx in transactions {
                if let Ok(tx_transaction) = self.analyze_transaction_for_mev(tx).await {
                    opportunities.extend(tx_opportunities);
                }
            }
        }
   
        Ok(opportunities)
    }


      /// Adds pools to monitor for arbitrage opportunities 
    pub async fn add_pools(&self, pools: Vec<(Address, Address,Address)>) -> Result<(), ScannerError> {
        self.pool_manager.add_pools(pools).await?;

        // Also add to mempool watcher
        let pool_addresses: Vec<Address> = self.pool_manager.get_all_addresses().await;
        if let Ok(watcher) = self.mempool_watcher.try_lock() {
            watcher.add_pool(pool_addresses);
        }

        Ok(())
    }
}