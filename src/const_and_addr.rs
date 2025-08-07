
use std::time::Duration;
use futures::stream::{self, StreamExt, TryStreamExt};
use tokio::sync::Semaphore;
use ethers::types::{Address, H256, TransactionReceipt};
use std::str::FromStr;

// Network connection
pub const COOL_DOWN_PERIOD: Duration = Duration::from_secs(30);
pub const CIRCUIT_BREAKER_THRESHOLD : usize = 5;

// Tuned for self-hosted nodes (adjust based on your hardware)

pub const MAX_RECEIPT_CONCURRENCY: usize = 150;  // Geth/Erigon can handle 500+ RPC calls
pub const MAX_LOG_CONCURRENCY: usize = 384;     // Memory-bound processing
pub const MAX_RPC_INFLIGHT: usize = 400;        // Total concurrent RPCs

// Common token addresses on Ethereum mainnet
pub const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
pub const USDC_ADDRESS: &str = "0xA0b86a33E6441E4e8B1e25D88A9b6C3D1B8b9c50";
pub const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
pub const DAI_ADDRESS: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
pub const WBTC_ADDRESS: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";

// Uniswap V2 addresses
pub const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
pub const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

// Uniswap V3 addresses
pub const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
pub const UNISWAP_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";

// SushiSwap addresses
pub const SUSHISWAP_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac";
pub const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";

// Event signatures
pub const SWAP_EVENT_SIGNATURE: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
pub const SYNC_EVENT_SIGNATURE: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
pub const TRANSFER_EVENT_SIGNATURE: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

// Gas constants
pub const DEFAULT_GAS_LIMIT: u64 = 300_000;
pub const PRIORITY_GAS_LIMIT: u64 = 500_000;
pub const MAX_GAS_PRICE: u64 = 500_000_000_000; // 500 Gwei

// Storage slot constants for Uniswap V2 pairs
pub const UNISWAP_V2_RESERVE0_SLOT: u64 = 8;
pub const UNISWAP_V2_RESERVE1_SLOT: u64 = 9;
pub const UNISWAP_V2_BLOCKTIMESTAMP_SLOT: u64 = 10;

// Minimum profit thresholds
pub const MIN_PROFIT_WEI: u64 = 1_000_000_000_000_000; // 0.001 ETH
pub const MIN_PROFIT_PERCENTAGE: f64 = 0.5; // 0.5%

// Network constants
pub const ETHEREUM_CHAIN_ID: u64 = 1;
pub const POLYGON_CHAIN_ID: u64 = 137;
pub const BSC_CHAIN_ID: u64 = 56;
pub const ARBITRUM_CHAIN_ID: u64 = 42161;

// Block time constants (in seconds)
pub const ETHEREUM_BLOCK_TIME: u64 = 12;
pub const POLYGON_BLOCK_TIME: u64 = 2;
pub const BSC_BLOCK_TIME: u64 = 3;
pub const ARBITRUM_BLOCK_TIME: u64 = 1;

// Cache constants
pub const DEFAULT_CACHE_SIZE: usize = 10_000;
pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 300;
pub const SLOT_CACHE_SIZE: usize = 100_000;

// Retry constants
pub const MAX_RETRIES: usize = 3;
pub const RETRY_DELAY_MS: u64 = 1000;
pub const BACKOFF_MULTIPLIER: f64 = 2.0;

// Helper functions to convert string addresses to Address type
pub fn weth() -> Address {
    Address::from_str(WETH_ADDRESS).unwrap()
}

pub fn usdc() -> Address {
    Address::from_str(USDC_ADDRESS).unwrap()
}

pub fn usdt() -> Address {
    Address::from_str(USDT_ADDRESS).unwrap()
}

pub fn dai() -> Address {
    Address::from_str(DAI_ADDRESS).unwrap()
}

pub fn wbtc() -> Address {
    Address::from_str(WBTC_ADDRESS).unwrap()
}

pub fn uniswap_v2_factory() -> Address {
    Address::from_str(UNISWAP_V2_FACTORY).unwrap()
}

pub fn uniswap_v2_router() -> Address {
    Address::from_str(UNISWAP_V2_ROUTER).unwrap()
}

pub fn uniswap_v3_factory() -> Address {
    Address::from_str(UNISWAP_V3_FACTORY).unwrap()
}

pub fn sushiswap_factory() -> Address {
    Address::from_str(SUSHISWAP_FACTORY).unwrap()
}

// Event signature helpers
pub fn swap_event_signature() -> H256 {
    H256::from_str(SWAP_EVENT_SIGNATURE).unwrap()
}

pub fn sync_event_signature() -> H256 {
    H256::from_str(SYNC_EVENT_SIGNATURE).unwrap()
}

pub fn transfer_event_signature() -> H256 {
    H256::from_str(TRANSFER_EVENT_SIGNATURE).unwrap()
}

// Top trading pairs on Ethereum
pub fn get_top_pairs() -> Vec<(Address, Address)> {
    vec![
        (weth(), usdc()),
        (weth(), usdt()),
        (weth(), dai()),
        (usdc(), usdt()),
        (usdc(), dai()),
        (weth(), wbtc()),
    ]
}

// Common DEX factory addresses
pub fn get_dex_factories() -> Vec<Address> {
    vec![
        uniswap_v2_factory(),
        sushiswap_factory(),
        // Add more DEX factories as needed
    ]
}

// Gas price tiers
pub const GAS_PRICE_SLOW: u64 = 20_000_000_000; // 20 Gwei
pub const GAS_PRICE_STANDARD: u64 = 40_000_000_000; // 40 Gwei
pub const GAS_PRICE_FAST: u64 = 60_000_000_000; // 60 Gwei
pub const GAS_PRICE_INSTANT: u64 = 100_000_000_000; // 100 Gwei

// Pool fee tiers (in basis points)
pub const UNISWAP_V2_FEE: u16 = 30; // 0.3%
pub const SUSHISWAP_FEE: u16 = 30; // 0.3%
pub const UNISWAP_V3_FEE_LOW: u16 = 5; // 0.05%
pub const UNISWAP_V3_FEE_MEDIUM: u16 = 30; // 0.3%
pub const UNISWAP_V3_FEE_HIGH: u16 = 100; // 1%