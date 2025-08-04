use ethers::types::U256;
use std::time::Duration;


// Cache constants
pub const DEFAULT_CACHE_SIZE: usize = 10_000;
pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 300;
pub const SLOT_CACHE_SIZE: usize = 100_000;

// Retry constants
pub const MAX_RETRIES: usize = 3;
pub const RETRY_DELAY_MS: u64 = 1000;
pub const BACKOFF_MULTIPLIER: f64 = 2.0;


pub const COOL_DOWN_PERIOD: Duration = Duration::from_secs(30);
pub const CIRCUIT_BREAKER_THRESHOLD : usize = 5;

// Tuned for self-hosted nodes (adjust based on your hardware)

pub const MAX_RECEIPT_CONCURRENCY: usize = 150;  // Geth/Erigon can handle 500+ RPC calls
pub const MAX_LOG_CONCURRENCY: usize = 384;     // Memory-bound processing
pub const MAX_RPC_INFLIGHT: usize = 400;        // Total concurrent RPCs


use futures::stream::{self, StreamExt, TryStreamExt};
use tokio::sync::Semaphore;

// Tuned for self-hosted nodes (adjust based on your hardware)
const MAX_RECEIPT_CONCURRENCY: usize = 500;  // Geth/Erigon can handle 500+ RPC calls
const MAX_LOG_CONCURRENCY: usize = 1024;     // Memory-bound processing
const MAX_RPC_INFLIGHT: usize = 1000;        // Total concurrent RPCs

