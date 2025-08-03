//! Core MEV scanner implementation
//!
//! The MevScanner orchestrates all components to detect arbitrage opportunities
//! by monitoring blockchain state changes and anaylyzing price differences.

use std::{sync::Arc, time::Duration};
use tokio::sync::{Notify, Mutex};
use ethers::{
    providers::{Provider, Http},
    types::{Address, BlockID, U256},
};

use crate::{
    pools::{PoolManger, PoolState},
    arbitrage::{ArbitrageDetector, ArbitrageOpporutnity},
    mempool::MempoolWatcher,
    providers::ProviderManager,
    cache::StateCache,
    config::ScannerConfig,
}; 

use tracing::{info,debug,warn,error};
use super::{BloomFilter, CircuitBreaker};

const WS_TIMEOUT: Duration = Duration::from_secs(30);
const BLOCK_PROCESSING_TIMEOUT: Duration= Duration::from_secs(10);
const MAX_BLOCK_BATCH_SIZE: usize = 5;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_CONSECUTIVE_ERRORS: usize = 3;
const MAX_ERRORS_BEFORE_TRIP: usize = 5;

const HTTP_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Main MEV scanner that coordinates all components
pub struct MevScanner {
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

    /// Circuit breaker for fault tolerance
    circuit_breaker: CircuitBreaker,

    /// Scanner configuration
    config: ScannerConfig,

    /// Last processed block number
    last_block: Mutex<u64>,

    /// Connection state
    connection_state: Mutex<ConnectionState>,

    /// Notifies main loop when WS reconnects
    ws_reconnected: Arc<Notify>,
}

struct ConnectionState {
    ws_connected: bool, 
    last_success: Instant,
    consecutive_errors: usize,
    last_block_received: Instant,
}

impl MevScanner {
    /// Creates a new MEV scanner with the given configuration
    pub async fn new(config: ScannerConfig) -> Result<Self, ScannerError> {
        let provider_manager = ProviderManager::new(
            &config.primary_rpc_url,
            &config.fallback_rpc_url,
        ).await?;

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

        let circut_breaker = CircuitBreaker::new(
            config.circuit_breaker_threshold,
            Duration::from_secs(config.circuit_breaker_cooldown_seconds),
        );

        Ok(Self {
            provider_manager,
            pool_manager,
            arbitrage_detector,
            mempool_watcher,
            state_cache,
            circut_breaker,
            config,
            last_block: Mutex::new(0),
            ws_reconnected: Arc::new(Notify::new()),
        })
    }

    /// Adds pools to monitor for arbitrage opportunities 
    pub async fn run_cycle(
        &self, 
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<Vec<ArbitrageOpporutnity>, ScannerError> {
        info!("Starting MEV Scanner main loop .................");

        loop{
            tokio::select!{
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received. Exiting run cycle loop....");
                    break Ok(());
                }
                _ = async {
                    if self.circuit_breaker.is_tripped().await {
                        warn!("Circuit breaker tripped, cooling down.....");
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
                                warn!("WS processing error: {:?}",e);
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

    pub async fn process_ws_blocks(&self) -> Resutl<()> {
        
    }
}