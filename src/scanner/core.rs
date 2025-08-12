//! Core MEV scanner implementation
//!
//! The MevScanner orchestrates all components to detect arbitrage opportunities
//! by monitoring blockchain state changes and anaylyzing price differences.

use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}}, 
    time::{Duration, Instant},
};
use tokio::{
    time::{sleep, timeout},
    sync::{Notify, Mutex,Semaphore},
};
use ethers::{
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    types::{transaction, Address, Block, Log, Transaction, TransactionReceipt, H256, U256},
};
use futures::stream::{self, StreamExt as FuturesStreamExt};
use anyhow::{anyhow, Result, Context};
use tracing::{info,debug,warn,error};
use once_cell::sync::Lazy;



use crate::{
    
    // arbitrage::{ArbitrageDetector,ArbitrageOpporutnity},
    // cache::StateCache, 
    config::ScannerConfig, 
    const_and_addr::{self, MAX_RECEIPT_CONCURRENCY}, 
    // mempool::MempoolWatcher, 
    // pools::{PoolManger, PoolState}, 
    // providers::ProviderManager,
    storage::{
        StorageDriftDetector, SlotDriftEvent,  SlotKey, StorageDelta,
        StorageChagneType, SlotSemantic, CriticalLevel,
    },
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
const DRIFT_CONFIDENCE_THRESHOLD: f64 = 0.8;
/// Main MEV scanner that coordinates all components
pub struct MevScanner {

    /// Primary WebSocket provider (wrapped in Mutex for reconnection)
    primary_provider: Arc<Mutex<Arc<Provider<Ws>>>>,

    // Fallback HTTP provider
    fallback_provider: Arc<Provider<Http>>,

    /// WebSocket url
    ws_endpoint: String,

    ///Manages RPC providers with failover
    // provider_manager: ProviderManager,

    /// Handles pool state and reserver data 
    // pool_manager: PoolManger,

    ///Detect arbtirage opportunities
    // arbitrage_detector: ArbitrageDetector,

    /// Monitors mempool for relevant transactions
    // mempool_watcher: Arc<Mutex<MempoolWatcher>>,

    /// Caches blockchain state
    // state_cache: StateCache,
    // slot_cache: SlotCache,

    /// Storage drift detector - SINGLE SOURCE OF TRUTH for all storage data
    storage_drift_detector: Arc<StorageDriftDetector>,

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

    /// Recent drift events for analysis (lightweight storage)
    recent_drift_events:  Arc<RwLock<Vec<SlotDriftEvent>>>,
}

struct ConnectionState {
    ws_connected: bool, 
    last_success: Instant,
    consecutive_errors: usize,
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
   
        let ws_endpoint = config.primary_rpc_url();
            
        let http_endpoint = config.fallback_rpc_url();

        let mut initial_connection_state = ConnectionState::default();
        // Initialize HTTP provider (always available)
        let fallback_provider = Arc::new(
            Provider::<Http>::try_from(http_endpoint)
                .context("Failed to create HTTP provider")?
        );

        // Try to initalize WebSocket provider
        let primary_provider = match Provider::<Ws>::connect(ws_endpoint).await{
            Ok(ws_provider) => {
                info!("✅ WebSocket provider connected successfully......");
                initial_connection_state.ws_connected = true;
                Arc::new(Mutex::new(Arc::new(ws_provider)))
            }
            Err(e) => {
                warn!("⚠️ WebSocket connection failed, will retry: {}", e);
                // Create a placeholder - will be replaced on reconnect
                let dummy_ws =  create_dummy_ws_provider().await?; // ------------------------------------------------------
                Arc::new(Mutex::new(Arc::new(dummy_ws)))
            }
        };

        let provider_manager = ProviderManager::new(
            ws_endpoint,
            http_endpoint,
        ).await?;

        let ws_endpoint = &config.primary_rpc_url();

        // let pool_manager = PoolManger::new();
        // let arbitrage_detector = ArbitrageDetector::new(config.min_profit_threshold());

        
        // let mempool_watcher = Arc::new(
        //     Mutex::new(MempoolWatcher::new(ws_endpoint).await?
        // ));
       

        // let state_cache = StateCache::new(
        //     const_and_addr::DEFAULT_CACHE_SIZE,
        //     Duration::from_secs(const_and_addr::DEFAULT_CACHE_TTL_SECONDS),
        // );

        // let slot_cache = SlotCache::new(const_and_addr::SLOT_CACHE_SIZE);

        let storage_drift_detector = Arc::new(StorageDriftDetector::new());

        let circuit_breaker = CircuitBreaker::new(
            const_and_addr::CIRCUIT_BREAKER_THRESHOLD,
            const_and_addr::COOL_DOWN_PERIOD,
            true,
        );
        let ws_url = ws_endpoint.to_string();
        Ok(Self {
            ws_endpoint: ws_url,
            primary_provider,
            fallback_provider,
            // provider_manager,
            // pool_manager,
            // arbitrage_detector,
            // mempool_watcher,
            // state_cache,
            // slot_cache,
            storage_drift_detector,
            circuit_breaker,
            config,
            last_block: AtomicU64::new(0),
            connection_state: Arc::new(Mutex::new(initial_connection_state)),
            ws_reconnected: Arc::new(Notify::new()),
            recent_drift_events:  Arc::new(RwLock::new(Vec::new()));
        })
    }


    /// Adds pools to monitor for arbitrage opportunities 
    pub async fn run_cycle(
        &self, 
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
) -> Result<()> {
        info!("Starting MEV Scanner main loop .................");

        // Start mempool monitoring task 
        // let mempool_task =self.start_mempool_monitoring();

        // Start storage drift monitoring task 
        let drift_task = self.start_drift_monitoring();

        loop{
            tokio::select!{
                _ = shutdown_rx.recv() => {
                    info!("🛑 Shutdown signal received. Exiting run cycle loop....");
                    mempool_task.abort();
                    break Ok(());
                }
                _ = async {
                    if self.circuit_breaker.is_tripped().await {
                        warn!("⚠️ Circuit breaker tripped, cooling down.....");
                        sleep(const_and_addr::COOL_DOWN_PERIOD).await;
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
                                self.circuit_breaker.trip().await;
                                sleep(RECONNECT_DELAY).await;
                            }
                        }
                    }

                } =>{}
            }
        }
        Ok(())
    }

    fn start_mempool_monitoring(&self) -> tokio::task::JoinHandle<()>{
        let mempool_watcher = self.mempool_watcher.clone();
        let arbitrage_detector = self.arbitrage_detector.clone();

        tokio::spawn(async move{
            loop {
                if let Ok(mut watcher) = mempool_watcher.try_lock() {
                    if let Ok(pending_txs) = watcher.get_pending_transactions().await {
                        for tx in pending_txs {
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

    pub async fn process_ws_blocks(&self) -> Result<()> {
        let provider = self.primary_provider.lock().await.clone();
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

    async fn process_http_polling(&self) -> Result<()> {
        info!("📊 Falling back to HTTP polling mode");

        loop {  
            tokio::select!{
                _ = sleep(HTTP_POLL_INTERVAL) => {
                    let latest_block = self.fallback_provider.get_block_number().await?.as_u64();
                    let last_processed = self.last_block.load(Ordering::Relaxed);

                    if latest_block > last_processed {
                        // Process all missed blocks individually for maximum speed 
                        for block_num in last_processed+1..=latest_block {
                            if let Err(e) = self.process_single_block_by_number(block_num).await {
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
                },
                _ = self.ws_reconnected.notified() => {
                    info!("✅ WebSocket reconnected, exiting HTTP fallback");
                    break; 
                }

            }
        }

        Ok(())
      
    }

    async fn process_block_immediately(&self, block: Block<H256>) -> Result<()>{
        let start_time = Instant::now();
        if let Some(number) = block.number.map(|n| n.as_u64()) {
            debug!("⚡️ Processing block {} immediately", number);

            match timeout(BLOCK_PROCESSING_TIMEOUT, self.process_single_block(number)).await {
                Ok(Ok(opportunities)) => {
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

    async fn process_single_block_by_number(&self, number: u64) -> Result<Vec<ArbitrageOpportunity>> {
        let block = self.fallback_provider
            .get_block(number)
            .await?
            .ok_or_else(|| anyhow!("Block {} not found", number))?;
        self.process_single_block(block).await
    }


    async fn process_single_block(&self, block: Block<H256>) -> Result<Vec<ArbitrageOpportunity>> {
        let start_time = Instan::now();
        let block_number = block.number
            .ok_or_else(|| anyhow!("Block missing number!!!!"))?
            .as_u64();

        debug!("🔍 Processing block {} with draft detection", block_number);

        // 1. Get Transaction receipts for storage analysis
        let receipts = self.get_block_receipts(&block).await?;

        // 2. Perform comprehensive storage drift analysis 
        let drift_events = self.storage_drift_detector
            .analyse_block(&block, receipts)
            .await?;

        let high_confidence_drifts = self.filter_high_confidence_drifts(&drift_events).await;












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
        if let Some(transactions) = &block.transactions {
            for tx in transactions {
                if let Ok(tx_opportunities) = self.analyze_transaction_for_mev(tx).await {
                    opportunities.extend(tx_opportunities);
                }
            }
        }
   
        Ok(opportunities)
    }

    async fn get_block_receipts(&self, block: &Block<H256>) -> Result<Vec<TransactionReceipt>> {
        static RPC_SEMAPHORE: Lazy<Semaphore> = Lazy::new(||
            Semaphore::new(MAX_RECEIPT_CONCURRENCY)
        );

        let provider = self.primary_provider.lock().await.clone();

        let receipts: Vec<TransactionReceipt> = stream::iter(&block.transactions)
            .map(|tx_hash| {
                let provider = provider.clone();
                async move {
                    let _permit = RPC_SEMAPHORE.acquire().await.unwrap();
                    provider
                        .get_transaction_receipt(*tx_hash)
                        .await
                        .ok()
                        .flatten()
                }
            })
            .buffered(MAX_RECEIPT_CONCURRENCY)
            .filter_map(|x| async move { x })
            .collect()
            .await;

        Ok(receipts)
    }

    async fn filter_high_confidence_drifts(&self, drift_events: &[SlotDriftEvent]) -> Vec<SlotDriftEvent> {
        drift_events
            .iter()
            .filter(|event| event.confidence >= DRIFT_CONFIDENCE_THRESHOLD)
            .cloned()
            .collect()
    }
    async fn try_reconnect_ws(&self) {
        let mut state = self.connection_state.lock().await;
        if state.ws_connected || state.reconnect_attempts > 5 {
            return;
        }

        info!("🔃 Attempting WebSocket reconnection (attempt {})", state.reconnect_attempts +1);
        state.reconnect_attempts += 1;
        drop(state);

        match Provider::<Ws>::connect(&self.ws_endpoint).await {
            Ok(new_provider) => {
                info!("✅ WebSocket reconnection successful!");
                *self.primary_provider.lock().await =Arc::new(new_provider);

                let mut state = self.connection_state.lock().await;
                state.ws_connected = true;
                state.reconnect_attempts = 0;
                state.consecutive_errors =0;
                self.ws_reconnected.notify_waiters();
            }
            Err(e)=> {
                warn!("❌ WebSocket reconnection failed:  {}",  e);
            }
        }
    }

    async fn should_attempt_ws_reconnect(&self) -> bool {
        let state = self.connection_state.lock().await;
        let time_since_last_success = state.last_success.elapsed();
        
        // Attempt reconnect every 30 seconds if WS is down
        !state.ws_connected && time_since_last_success > Duration::from_secs(30)
    }

    async fn update_connection_success(&self) {
        let mut state = self.connection_state.lock().await;
        state.ws_connected = true;
        state.last_success = Instant::now();
        state.consecutive_errors = 0;
    }

    async fn handle_connection_error(&self) {
        let mut state = self.connection_state.lock().await;
        state.consecutive_errors += 1;
        if state.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            warn!("🔌 Too many consecutive errors, marking WS as disconnected");
            state.ws_connected = false;
        }
    }

//   async fn detect_changed_slots(&self, block: &Block<H256>) -> Result<Vec<(u64, U256)>> {
//         static RPC_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(const_and_addr::MAX_RPC_INFLIGHT));
//         let provider = self.primary_provider.lock().await.clone();
        
//         // 1. Fetch receipts with concurrency control
//         let receipts: Vec<TransactionReceipt> = stream::iter(&block.transactions)
//             .map(|tx_hash| {
//                 let provider = provider.clone();
//                 async move {
//                     let _permit = RPC_SEMAPHORE.acquire().await.unwrap();
//                     provider
//                         .get_transaction_receipt(*tx_hash)
//                         .await
//                         .ok()
//                         .flatten()
//                 }
//             })
//             .buffered(const_and_addr::MAX_RECEIPT_CONCURRENCY)
//             .filter_map(|x| async move { x })
//             .collect()
//             .await;

//         // 2. Process receipts and logs
//         let mut all_changed_slots = Vec::new();
        
//         for receipt in receipts {
//             let to = receipt.to else { continue };  
//             if !self.pool_manager.is_monitored_pool(to).await {
//                 continue;
//             }
            
//             // Process logs for this receipt
//             let log_results: Vec<(u64, U256)> = stream::iter(receipt.logs)
//                 .map(|log| async move {
//                     self.parse_log_for_storage_changes(&log).await.unwrap_or_default()
//                 })
//                 .buffered(const_and_addr::MAX_LOG_CONCURRENCY)
//                 .flat_map(|changes| stream::iter(changes))
//                 .collect()
//                 .await;
            
//             // Flatten and add to results
//             all_changed_slots.extend(log_results);
//         }

//         Ok(all_changed_slots)
//     } 

//     async fn parse_log_for_storage_changes(&self, log: &Log) -> Result<Vec<(u64, U256)>> {
//         let mut changes = Vec::new();

//         // Uniswap V2 Sync event: topic0 =  keccak256("Sync(uint112,uint112)")
//         let sync_topic = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

//         if log.topics.len() > 0 && format!("{:#x}", log.topics[0]) == sync_topic {
//             // Parse reserve data from Sync event
//             if log.data.len() >= 64 {
//                 let reserve0 = U256::from_big_endian(&log.data[0..32]);
//                 let reserve1 = U256::from_big_endian(&log.data[32..64]);

//                 // Map to storage slots (simplified - actual slots depend on contract layout)
//                 changes.push((8, reserve0)); // Slot 8: reserve0
//                 changes.push((9, reserve1)); // SLot 9: reserve1

//             }
//         }
//         Ok(changes)
//     }

//       /// Adds pools to monitor for arbitrage opportunities 
//     pub async fn add_pools(&self, pools: Vec<(Address, Address,Address)>) -> Result<()> {
//         self.pool_manager.add_pools(pools).await?;

//         // Also add to mempool watcher
//         let pool_addresses: Vec<Address> = self.pool_manager.get_all_addresses().await;
//         if let Ok(watcher) = self.mempool_watcher.try_lock() {
//             watcher.add_pool(pool_addresses);
//         }

//         Ok(())
//     }
}