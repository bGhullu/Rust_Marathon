mod contracts;
use contracts::{
    UNISWAP_V3_FACTORY, UNISWAP_V3_ROUTER, SUSHI_FACTORY, SUSHI_ROUTER, AAVE_LENDING_POOL,
    WETH, USDC,
    IUniswapV2Router02, IUniswapV3Pool, IUniswapV3Factory, IUniswapV2Factory, IUniswapV2Pair, ISushiRouter
};

use anyhow::Result;
use async_trait::async_trait;
use ethers::{
    contract::abigen,
    prelude::*,
    providers::{Http, Middleware, Provider, Ws},
    types::{Address, TransactionRequest, U256},
};
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{Mutex, RwLock},
    time::{sleep, Instant},
};
use tracing::{error, info, warn};
use dotenv::dotenv;
use ethers::types::transaction::eip2718::TypedTransaction;

// Type aliases
type PoolAddress = Address;
type TokenAddress = Address;
type GasCost = U256;



// Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub max_trade_size: U256,
    pub min_profit_threshold: f64,
    pub max_slippage: f64,
    pub max_price_impact: f64,
    pub gas_price_multiplier: f64,
    pub max_concurrent_trades: usize,
    pub max_trade_retries: u32,
    pub max_tx_wait_time: Duration,
    pub flash_loan_fee: f64,
    pub profit_estimation_buffer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: TokenAddress,
    pub symbol: String,
    pub decimals: u8,
    pub is_stable: bool,
    pub price_usd: Option<f64>,
    pub volatility: f64,
}

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: PoolAddress,
    pub dex: DexType,
    pub token0: TokenInfo,
    pub token1: TokenInfo,
    pub fee: u32,
    pub tick_spacing: i32,
    pub last_updated: Instant,
    pub historical_volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DexType {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    PancakeSwap,
    Curve,
    BalancerV2,
    Dodo,
    Bancor,
}

#[derive(Debug, Clone)]
pub struct PriceQuote {
    pub pool: PoolAddress,
    pub dex: DexType,
    pub price: f64,
    pub liquidity: U256,
    pub impact: f64,
    pub timestamp: Instant,
    pub gas_cost: GasCost,
    pub confidence: f64,
    pub block_number: u64,
}

#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub path: Vec<PoolInfo>,
    pub token_path: Vec<TokenInfo>,
    pub expected_prices: Vec<f64>,
    pub expected_profit: f64,
    pub expected_profit_percentage: f64,
    pub optimal_amount: U256,
    pub total_gas_cost: GasCost,
    pub net_profit: f64,
    pub confidence_score: f64,
    pub risk_score: f64,
    pub max_slippage: f64,
    pub min_amount_out: U256,
    pub first_seen: Instant,
    pub last_updated: Instant,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    pub max_daily_loss: f64,
    pub max_position_size: U256,
    pub min_profit_threshold: f64,
    pub max_concurrent_trades: usize,
    pub current_metrics: Arc<RwLock<TradingMetrics>>,
    pub is_active: Arc<RwLock<bool>>,
}

impl CircuitBreaker {
    pub async fn should_halt_trading(&self) -> bool {
        let metrics = self.current_metrics.read().await;
        let is_active = *self.is_active.read().await;
        
        !is_active || 
        metrics.daily_pnl < -self.max_daily_loss ||
        metrics.concurrent_trades >= self.max_concurrent_trades
    }

    pub async fn record_trade(&self, profit: f64, volume: U256) {
        let mut metrics = self.current_metrics.write().await;
        metrics.total_trades += 1;
        metrics.daily_pnl += profit;
        metrics.weekly_pnl += profit;
        metrics.total_volume += volume;
        
        if profit > 0.0 {
            metrics.successful_trades += 1;
        } else {
            metrics.failed_trades += 1;
        }
    }

    pub async fn pause(&self) {
        *self.is_active.write().await = false;
        info!("Circuit breaker: Trading paused");
    }

    pub async fn resume(&self) {
        *self.is_active.write().await = true;
        info!("Circuit breaker: Trading resumed");
    }

    pub async fn emergency_stop(&self) {
        *self.is_active.write().await = false;
        error!("EMERGENCY STOP ACTIVATED");
    }
}

#[derive(Debug)]
pub struct TradingMetrics {
    pub daily_pnl: f64,
    pub weekly_pnl: f64,
    pub total_trades: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub total_volume: U256,
    pub last_reset: u64,
    pub concurrent_trades: usize,
}

impl Default for TradingMetrics {
    fn default() -> Self {
        Self {
            daily_pnl: 0.0,
            weekly_pnl: 0.0,
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            total_volume: U256::zero(),
            last_reset: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            concurrent_trades: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct PriceDataPoint {
    price: f64,
    timestamp: Instant,
    block_number: u64,
    sources: usize,
    liquidity: U256,
}

#[async_trait]
pub trait StablecoinOracle: Send + Sync {
    fn get_price(&self, token: TokenAddress) -> BoxFuture<'_, Result<f64>>;
}

pub struct PriceValidator {
    arbitrage_checker: Arc<ArbitrageChecker>,
    max_price_deviation: f64,
}

pub struct ArbitrageChecker {
    uniswap_router: Address,
    sushi_router: Address,
    client: Arc<Provider<Http>>,
}

impl ArbitrageChecker {
    pub async fn check_arbitrage(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
        price: f64,
    ) -> Result<bool> {
        let uniswap_price = self.get_uniswap_price(token_a, token_b).await?;
        let sushi_price = self.get_sushi_price(token_a, token_b).await?;

        let uniswap_diff = (price - uniswap_price).abs() / uniswap_price;
        let sushi_diff = (price - sushi_price).abs() / sushi_price;

        Ok(uniswap_diff > 0.01 || sushi_diff > 0.01)
    }

    async fn get_uniswap_price(&self, token_in: TokenAddress, token_out: TokenAddress) -> Result<f64> {
        let router = IUniswapV2Router02::new(self.uniswap_router, Arc::new(self.client.clone()));
        let path = vec![token_in, token_out];
        let amounts = router.get_amounts_out(U256::from(10).pow(18.into()), path).call().await?;
        Ok(amounts[1].as_u128() as f64 / 1e18)
    }

    async fn get_sushi_price(&self, token_in: TokenAddress, token_out: TokenAddress) -> Result<f64> {
        let router = ISushiRouter::new(self.sushi_router, Arc::new(self.client.clone()));
        let path = vec![token_in, token_out];
        let amounts = router.get_amounts_out(U256::from(10).pow(18.into()), path).call().await?;
        Ok(amounts[1].as_u128() as f64 / 1e18)
    }
}

pub struct PriceOracle {
    providers: HashMap<DexType, Arc<Provider<Ws>>>,
    price_cache: Arc<RwLock<HashMap<(TokenAddress, TokenAddress), PriceDataPoint>>>,
    stablecoin_oracle: Arc<dyn StablecoinOracle + Send + Sync>,
    validator: Arc<PriceValidator>,
    config: Arc<BotConfig>,
}

impl PriceOracle {
    pub async fn get_real_price(&self, token_in: TokenAddress, token_out: TokenAddress) -> Result<f64> {
        // Check cache first
        if let Some(cached) = self.price_cache.read().await.get(&(token_in, token_out)) {
            if cached.timestamp.elapsed() < Duration::from_secs(5) {
                return Ok(cached.price);
            }
        }

        let mut prices = Vec::new();
        let mut liquidities = Vec::new();

        for (dex, provider) in &self.providers {
            match dex {
                DexType::UniswapV3 => {
                    if let Ok(pool) = self.get_uniswap_v3_pool(token_in, token_out).await {
                        if let Ok((price, liquidity)) = self.get_uniswap_v3_price(provider, pool).await {
                            prices.push(price);
                            liquidities.push(liquidity);
                        }
                    }
                }
                DexType::SushiSwap => {
                    if let Ok((price, liquidity)) = self.get_sushiswap_price(provider, token_in, token_out).await {
                        prices.push(price);
                        liquidities.push(liquidity);
                    }
                }
                _ => continue,
            }
        }

        if prices.is_empty() {
            return Err(anyhow::anyhow!("No price data available"));
        }

        let total_liquidity: U256 = liquidities.iter().fold(U256::zero(), |acc, x| acc + *x);
        let weighted_price = if total_liquidity.is_zero() {
            prices.iter().sum::<f64>() / prices.len() as f64
        } else {
            prices.iter().zip(liquidities.iter())
                .map(|(p, l)| p * l.as_u128() as f64)
                .sum::<f64>() / total_liquidity.as_u128() as f64
        };

        // Update cache
        let mut cache = self.price_cache.write().await;
        cache.insert(
            (token_in, token_out),
            PriceDataPoint {
                price: weighted_price,
                timestamp: Instant::now(),
                block_number: self.providers[&DexType::UniswapV3]
                    .get_block_number()
                    .await?
                    .as_u64(),
                sources: prices.len(),
                liquidity: total_liquidity,
            },
        );

        Ok(weighted_price)
    }

    async fn get_uniswap_v3_pool(&self, token_a: TokenAddress, token_b: TokenAddress) -> Result<PoolAddress> {
        let factory = IUniswapV3Factory::new(*UNISWAP_V3_FACTORY, self.providers[&DexType::UniswapV3].clone());
        let pool = factory.get_pool(token_a, token_b, 3000).call().await?;
        Ok(pool)
    }

    async fn get_uniswap_v3_price(&self, provider: &Provider<Ws>, pool: PoolAddress) -> Result<(f64, U256)> {
        let pool_contract = IUniswapV3Pool::new(pool, Arc::new(provider.clone()));
        let slot0 = pool_contract.slot_0().call().await?;
        let liquidity = pool_contract.liquidity().call().await?;

        let sqrt_price_x96 = slot0.0;
        let price = (sqrt_price_x96.as_u128() as f64).powi(2) / 2f64.powi(192);

        Ok((price, U256::from(liquidity)))
    }

    async fn get_sushiswap_price(&self, provider: &Provider<Ws>, token_in: TokenAddress, token_out: TokenAddress) -> Result<(f64, U256)> {
        let router = ISushiRouter::new(*SUSHI_ROUTER, Arc::new(provider.clone()));
        let path = vec![token_in, token_out];
        let amounts = router.get_amounts_out(U256::from(10).pow(18.into()), path).call().await?;
        
        let factory = IUniswapV2Factory::new(*SUSHI_FACTORY, Arc::new(provider.clone()));
        let pair = factory.get_pair(token_in, token_out).call().await?;
        let pair_contract = IUniswapV2Pair::new(pair, Arc::new(provider.clone()));
        let reserves = pair_contract.get_reserves().call().await?;
        let liquidity = U256::from(reserves.0) + U256::from(reserves.1);

        Ok((amounts[1].as_u128() as f64 / 1e18, liquidity))
    }
}

pub struct MevProtection {
    pub max_priority_fee_per_gas: U256,
    pub min_priority_fee_per_gas: U256,
    pub flashbots_enabled: bool,
    pub private_rpc_url: Option<String>,
}

pub struct ArbitrageEngine {
    oracle: Arc<PriceOracle>,
    circuit_breaker: Arc<CircuitBreaker>,
    pools: Vec<PoolInfo>,
    config: Arc<BotConfig>,
    trade_executor: Arc<TradeExecutor>,
    opportunity_queue: Arc<Mutex<VecDeque<ArbitrageOpportunity>>>,
    historical_opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
    mev_protection: Arc<MevProtection>,
}

impl ArbitrageEngine {
    pub async fn find_multi_hop_arbitrage(&self, max_hops: usize) -> Result<Vec<ArbitrageOpportunity>> {
        let mut adjacency = HashMap::new();
        for pool in &self.pools {
            adjacency.entry(pool.token0.address).or_insert_with(Vec::new).push(pool);
            adjacency.entry(pool.token1.address).or_insert_with(Vec::new).push(pool);
        }

        let mut opportunities = Vec::new();
        for start_token in adjacency.keys() {
            let mut visited = VecDeque::new();
            visited.push_back(*start_token);
            self.dfs_find_arbitrage(
                *start_token,
                *start_token,
                max_hops,
                &mut Vec::new(),
                &mut visited,
                &mut opportunities,
                &adjacency,
            )
            .await?;
        }

        Ok(opportunities)
    }

    async fn dfs_find_arbitrage(
        &self,
        current_token: TokenAddress,
        start_token: TokenAddress,
        remaining_hops: usize,
        current_path: &mut Vec<PoolInfo>,
        visited: &mut VecDeque<TokenAddress>,
        opportunities: &mut Vec<ArbitrageOpportunity>,
        adjacency: &HashMap<TokenAddress, Vec<&PoolInfo>>,
    ) -> Result<()> {
        if remaining_hops == 0 {
            return Ok(());
        }

        if let Some(pools) = adjacency.get(&current_token) {
            for pool in pools {
                let next_token = if pool.token0.address == current_token {
                    pool.token1.address
                } else {
                    pool.token0.address
                };

                let mut new_path = current_path.clone();
                new_path.push((*pool).clone());

                if next_token == start_token && new_path.len() > 1 {
                    if let Some(opportunity) = self.analyze_arbitrage_loop(&new_path).await? {
                        opportunities.push(opportunity);
                    }
                } else if !visited.contains(&next_token) {
                    visited.push_back(next_token);
                    Box::pin(self.dfs_find_arbitrage(
                        next_token,
                        start_token,
                        remaining_hops - 1,
                        &mut new_path,
                        visited,
                        opportunities,
                        adjacency,
                    ))
                    .await?;
                    visited.pop_back();
                }
            }
        }

        Ok(())
    }

    async fn analyze_arbitrage_loop(&self, path: &[PoolInfo]) -> Result<Option<ArbitrageOpportunity>> {
        let mut amount_in = self.config.max_trade_size;
        let mut token_path = Vec::new();
        let mut expected_prices = Vec::new();
        let mut total_gas = U256::zero();

        if let Some(first_pool) = path.first() {
            token_path.push(if path.len() > 1 {
                if path[1].token0.address == first_pool.token0.address 
                    || path[1].token1.address == first_pool.token0.address {
                    first_pool.token1.clone()
                } else {
                    first_pool.token0.clone()
                }
            } else {
                first_pool.token0.clone()
            });
        }

        for (i, pool) in path.iter().enumerate() {
            let token_in = if i == 0 {
                token_path[0].address
            } else {
                token_path[i-1].address
            };

            let (amount_out, gas_cost) = self.simulate_swap(
                pool.address,
                token_in,
                amount_in,
                pool.dex,
            ).await?;

            expected_prices.push(amount_out.as_u128() as f64 / amount_in.as_u128() as f64);
            total_gas += gas_cost;

            if i < path.len() - 1 {
                amount_in = amount_out;
                token_path.push(
                    if pool.token0.address == token_in {
                        pool.token1.clone()
                    } else {
                        pool.token0.clone()
                    }
                );
            }
        }

        let profit = amount_in.as_u128() as f64 / self.config.max_trade_size.as_u128() as f64 - 1.0;
        let net_profit = profit - total_gas.as_u128() as f64 / 1e18;

        if net_profit > self.config.min_profit_threshold {
            Ok(Some(ArbitrageOpportunity {
                path: path.to_vec(),
                token_path,
                expected_prices,
                expected_profit: profit,
                expected_profit_percentage: profit * 100.0,
                optimal_amount: self.config.max_trade_size,
                total_gas_cost: total_gas,
                net_profit,
                confidence_score: 0.9,
                risk_score: 0.1,
                max_slippage: self.config.max_slippage,
                min_amount_out: amount_in * (U256::from(10000) - U256::from((self.config.max_slippage * 100.0) as u128)) / U256::from(10000),
                first_seen: Instant::now(),
                last_updated: Instant::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn simulate_swap(
        &self,
        pool: PoolAddress,
        token_in: TokenAddress,
        amount_in: U256,
        dex: DexType,
    ) -> Result<(U256, U256)> {
        match dex {
            DexType::UniswapV3 => {
                let quoter = IUniswapV2Router02::new(*UNISWAP_V3_ROUTER, self.trade_executor.client.clone());
                let path = vec![token_in, token_in]; // Simplified for example
                let amounts = quoter.get_amounts_out(amount_in, path).call().await?;
                Ok((amounts[1], U256::from(100_000)))
            }
            DexType::SushiSwap => {
                let router = ISushiRouter::new(*SUSHI_ROUTER, self.trade_executor.client.clone());
                let path = vec![token_in, token_in]; // Simplified for example
                let amounts = router.get_amounts_out(amount_in, path).call().await?;
                Ok((amounts[1], U256::from(120_000)))
            }
            _ => Err(anyhow::anyhow!("DEX not implemented")),
        }
    }

    pub async fn simulate_trade(&self, opportunity: &ArbitrageOpportunity) -> Result<SimulationResult> {
        let mut amount_in = opportunity.optimal_amount;
        let mut total_gas = U256::zero();

        for (i, pool) in opportunity.path.iter().enumerate() {
            let token_in = opportunity.token_path[i].address;
            let (amount_out, gas_cost) = self.simulate_swap(
                pool.address,
                token_in,
                amount_in,
                pool.dex,
            ).await?;
            amount_in = amount_out;
            total_gas += gas_cost;
        }

        let profit = amount_in.as_u128() as f64 / opportunity.optimal_amount.as_u128() as f64 - 1.0;
        let net_profit = profit - total_gas.as_u128() as f64 / 1e18;

        Ok(SimulationResult {
            success: net_profit > self.config.min_profit_threshold,
            actual_profit: net_profit,
            gas_used: total_gas,
            revert_reason: None,
        })
    }

    pub async fn monitor_opportunity(&self, opportunity: ArbitrageOpportunity) {
        let mut queue = self.opportunity_queue.lock().await;
        queue.push_back(opportunity);
    }
}

#[async_trait]
pub trait FlashLoanProvider: Send + Sync {
    fn execute_flash_loan(
        &self,
        amount: U256,
        token: TokenAddress,
        pools: Vec<PoolAddress>,
    ) -> BoxFuture<'_, Result<TxHash>>;
    
    fn estimate_flash_loan_fee(
        &self,
        amount: U256,
        token: TokenAddress,
    ) -> BoxFuture<'_, Result<U256>>;
}

pub struct AaveFlashLoanProvider {
    client: Arc<Provider<Http>>,
    lending_pool: Address,
}

#[async_trait]
impl FlashLoanProvider for AaveFlashLoanProvider {
    fn execute_flash_loan(
        &self,
        amount: U256,
        token: TokenAddress,
        pools: Vec<PoolAddress>,
    ) -> BoxFuture<'_, Result<TxHash>> {
        Box::pin(async move {
            let tx = TransactionRequest::new()
                .to(self.lending_pool)
                .data(vec![]); // Simplified for example
            let pending_tx = self.client.send_transaction(tx, None).await?;
            Ok(pending_tx.tx_hash())
        })
    }

    fn estimate_flash_loan_fee(
        &self,
        amount: U256,
        _token: TokenAddress,
    ) -> BoxFuture<'_, Result<U256>> {
        Box::pin(async move {
            Ok(amount * U256::from(9) / U256::from(10000)) // 0.09% fee
        })
    }
}

pub struct TradeExecutor {
    client: Arc<Provider<Http>>,
    flash_loan_providers: Vec<Arc<dyn FlashLoanProvider + Send + Sync>>,
    config: Arc<BotConfig>,
    pending_txs: Arc<Mutex<HashMap<TxHash, Instant>>>,
    mev_protection: Arc<MevProtection>,
}

impl TradeExecutor {
    pub async fn execute_with_mev_protection(
        &self,
        tx: TransactionRequest,
        mev_params: &MevProtection,
    ) -> Result<TxHash> {
        if mev_params.flashbots_enabled {
            error!("Flashbots not implemented in this example");
            Err(anyhow::anyhow!("Flashbots not implemented"))
        } else if let Some(private_rpc) = &mev_params.private_rpc_url {
            let provider = Provider::<Http>::try_from(private_rpc)?;
            let pending_tx = provider.send_transaction(tx, None).await?;
            Ok(pending_tx.tx_hash())
        } else {
            let pending_tx = self.client.send_transaction(tx, None).await?;
            Ok(pending_tx.tx_hash())
        }
    }

    pub async fn wait_for_tx_with_retries(&self, tx_hash: TxHash) -> Result<()> {
        for _ in 0..self.config.max_trade_retries {
            if let Some(receipt) = self.client.get_transaction_receipt(tx_hash).await? {
                if receipt.status == Some(1.into()) {
                    return Ok(());
                } else {
                    return Err(anyhow::anyhow!("Transaction failed"));
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
        Err(anyhow::anyhow!("Transaction timeout"))
    }

    pub async fn execute(&self, opportunity: ArbitrageOpportunity) -> Result<TxHash> {
        let flash_loan_tx = self.flash_loan_providers[0]
            .execute_flash_loan(
                opportunity.optimal_amount,
                opportunity.token_path[0].address,
                opportunity.path.iter().map(|p| p.address).collect(),
            )
            .await?;

        let tx = TransactionRequest::new()
            .to(*AAVE_LENDING_POOL)
            .data(vec![]); // Simplified for example

        self.execute_with_mev_protection(tx, &self.mev_protection).await
    }
}

pub struct SimulationResult {
    pub success: bool,
    pub actual_profit: f64,
    pub gas_used: U256,
    pub revert_reason: Option<String>,
}

pub struct HealthCheck {
    last_heartbeat: Arc<RwLock<Instant>>,
    max_latency: Duration,
    heartbeat_interval: Duration,
    component_status: Arc<RwLock<HashMap<String, bool>>>,
}

impl HealthCheck {
    pub fn new(max_latency: Duration) -> Self {
        Self {
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            max_latency,
            heartbeat_interval: max_latency / 2,
            component_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_heartbeat(&self) {
        *self.last_heartbeat.write().await = Instant::now();
    }

    pub async fn check(&self) -> Result<()> {
        let now = Instant::now();
        let last = *self.last_heartbeat.read().await;
        
        if now.duration_since(last) > self.max_latency {
            return Err(anyhow::anyhow!("Health check failed - latency too high"));
        }

        let components = self.component_status.read().await;
        for (name, healthy) in components.iter() {
            if !healthy {
                return Err(anyhow::anyhow!("Component {} is unhealthy", name));
            }
        }

        Ok(())
    }

    pub async fn update_component_status(&self, name: &str, healthy: bool) {
        let mut components = self.component_status.write().await;
        components.insert(name.to_string(), healthy);
    }

    pub async fn run_heartbeat_updater(&self) {
        let heartbeat = self.last_heartbeat.clone();
        let interval = self.heartbeat_interval;
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                *heartbeat.write().await = Instant::now();
            }
        });
    }
}

struct MockStablecoinOracle;

#[async_trait]
impl StablecoinOracle for MockStablecoinOracle {
    fn get_price(&self, _token: TokenAddress) -> BoxFuture<'_, Result<f64>> {
        Box::pin(async move { Ok(1.0) })
    }
}

pub struct ArbitrageBot {
    engine: Arc<ArbitrageEngine>,
    executor: Arc<TradeExecutor>,
    config: Arc<BotConfig>,
    health_check: Arc<HealthCheck>,
}

impl ArbitrageBot {
    pub async fn run(&self) -> Result<()> {
        // Start the heartbeat updater
        self.health_check.run_heartbeat_updater().await;

        // Run all components
        let finder = self.find_opportunities();
        let executor = self.execute_opportunities();
        let health = self.monitor_health();

        tokio::try_join!(finder, executor, health)?;
        Ok(())
    }

    async fn find_opportunities(&self) -> Result<()> {
        loop {
            if self.engine.circuit_breaker.should_halt_trading().await {
                warn!("Circuit breaker active - pausing opportunity search");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            match self.engine.find_multi_hop_arbitrage(3).await {
                Ok(opportunities) => {
                    for opp in opportunities {
                        if opp.confidence_score > 0.9 && opp.risk_score < 0.2 {
                            self.engine.monitor_opportunity(opp).await;
                        }
                    }
                }
                Err(e) => error!("Error finding opportunities: {}", e),
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn execute_opportunities(&self) -> Result<()> {
        loop {
            if self.engine.circuit_breaker.should_halt_trading().await {
                warn!("Circuit breaker active - pausing execution");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let opportunity = {
                let mut queue = self.engine.opportunity_queue.lock().await;
                queue.pop_front()
            };

            if let Some(opp) = opportunity {
                match self.executor.execute(opp.clone()).await {
                    Ok(tx_hash) => {
                        info!("Executed arbitrage: {:?}", tx_hash);
                        self.engine.circuit_breaker.record_trade(
                            opp.net_profit,
                            opp.optimal_amount,
                        ).await;
                    }
                    Err(e) => error!("Execution failed: {}", e),
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    }

    async fn monitor_health(&self) -> Result<()> {
        // Monitor individual components
        let components = vec!["oracle", "executor", "engine"];
        let check_intervals = Duration::from_secs(3);

        loop {
            // Update component statuses
            self.health_check.update_component_status(
                "oracle", 
                self.check_oracle_health().await.is_ok()
            ).await;
            
            self.health_check.update_component_status(
                "executor",
                self.check_executor_health().await.is_ok()
            ).await;

            self.health_check.update_component_status(
                "engine",
                self.check_engine_health().await.is_ok()
            ).await;

            // Perform overall health check
            if let Err(e) = self.health_check.check().await {
                error!("Health check failed: {}", e);
                self.handle_health_failure(e).await;
            }

            sleep(check_intervals).await;
        }
    }

    async fn check_oracle_health(&self) -> Result<()> {
        let test_token = Address::zero(); // Replace with actual token address
        match self.engine.oracle.get_real_price(test_token, test_token).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Oracle health check failed: {}", e)),
        }
    }

    async fn check_executor_health(&self) -> Result<()> {
        match self.executor.client.estimate_gas(
            &TypedTransaction::Legacy(TransactionRequest::new()),
            None
        ).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Executor health check failed: {}", e)),
        }
    }

    async fn check_engine_health(&self) -> Result<()> {
        match self.engine.find_multi_hop_arbitrage(1).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Engine health check failed: {}", e)),
        }
    }

    async fn handle_health_failure(&self, error: anyhow::Error) {
        error!("Handling health failure: {}", error);
        
        // 1. Pause trading through circuit breaker
        self.engine.circuit_breaker.pause().await;
        
        // 2. Attempt to reconnect providers
        if let Err(e) = self.reconnect_providers().await {
            error!("Failed to reconnect providers: {}", e);
        }
        
        // 3. Reset health status after recovery attempts
        self.health_check.update_heartbeat().await;
        self.engine.circuit_breaker.resume().await;
    }

    async fn reconnect_providers(&self) -> Result<()> {
        info!("Attempting to reconnect providers...");
        // Implementation would depend on your provider setup
        Ok(())
    }
}

fn create_sample_pools() -> Vec<PoolInfo> {
    vec![
        PoolInfo {
            address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".parse().unwrap(),
            dex: DexType::UniswapV3,
            token0: TokenInfo {
                address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
                symbol: "USDC".to_string(),
                decimals: 6,
                is_stable: true,
                price_usd: Some(1.0),
                volatility: 0.01,
            },
            token1: TokenInfo {
                address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
                symbol: "WETH".to_string(),
                decimals: 18,
                is_stable: false,
                price_usd: Some(3000.0),
                volatility: 0.05,
            },
            fee: 500,
            tick_spacing: 10,
            last_updated: Instant::now(),
            historical_volume: 1000000.0,
        },
        PoolInfo {
            address: "0x06da0fd433C1A5d7a4faa01111c044910A184553".parse().unwrap(),
            dex: DexType::SushiSwap,
            token0: TokenInfo {
                address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
                symbol: "USDC".to_string(),
                decimals: 6,
                is_stable: true,
                price_usd: Some(1.0),
                volatility: 0.01,
            },
            token1: TokenInfo {
                address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
                symbol: "WETH".to_string(),
                decimals: 18,
                is_stable: false,
                price_usd: Some(3000.0),
                volatility: 0.05,
            },
            fee: 300,
            tick_spacing: 1,
            last_updated: Instant::now(),
            historical_volume: 500000.0,
        },
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let infura_key = std::env::var("INFURA_KEY")
        .expect("INFURA_KEY must be set in .env file");
    tracing_subscriber::fmt::init();

    // Initialize configuration
    let config = Arc::new(BotConfig {
        max_trade_size: U256::from(10).pow(18.into()),
        min_profit_threshold: 0.1, // 0.1% minimum profit
        max_slippage: 0.5,
        max_price_impact: 1.0,
        gas_price_multiplier: 1.2,
        max_concurrent_trades: 3,
        max_trade_retries: 3,
        max_tx_wait_time: Duration::from_secs(30),
        flash_loan_fee: 0.0009,
        profit_estimation_buffer: 0.1,
    });

    // Set up providers
    let http_url = format!("https://mainnet.infura.io/v3/{}", infura_key);
    let http_provider = Provider::<Http>::try_from(http_url)
        .map_err(|e| anyhow::anyhow!("HTTP provider error: {}", e))?;

    let ws_url = format!("wss://mainnet.infura.io/ws/v3/{}", infura_key);
    let ws_provider = Provider::<Ws>::connect(&ws_url).await
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;

    // Set up components
    let stablecoin_oracle = Arc::new(MockStablecoinOracle);
    let arbitrage_checker = Arc::new(ArbitrageChecker {
        uniswap_router: *UNISWAP_V3_ROUTER,
        sushi_router: *SUSHI_ROUTER,
        client: Arc::new(http_provider.clone()),
    });
    let price_validator = Arc::new(PriceValidator {
        arbitrage_checker,
        max_price_deviation: 0.05,
    });

    let oracle = Arc::new(PriceOracle {
        providers: HashMap::from([
            (DexType::UniswapV3, Arc::new(ws_provider.clone())),
            (DexType::SushiSwap, Arc::new(ws_provider)),
        ]),
        price_cache: Arc::new(RwLock::new(HashMap::new())),
        stablecoin_oracle,
        validator: price_validator,
        config: config.clone(),
    });

    let circuit_breaker = Arc::new(CircuitBreaker {
        max_daily_loss: 1000.0,
        max_position_size: U256::from(10).pow(18.into()) * U256::from(10),
        min_profit_threshold: config.min_profit_threshold,
        max_concurrent_trades: config.max_concurrent_trades,
        current_metrics: Arc::new(RwLock::new(TradingMetrics::default())),
        is_active: Arc::new(RwLock::new(true)),
    });

    let flash_loan = Arc::new(AaveFlashLoanProvider {
        client: Arc::new(http_provider.clone()),
        lending_pool: *AAVE_LENDING_POOL,
    });

    let mev_protection = Arc::new(MevProtection {
        max_priority_fee_per_gas: U256::from(2_000_000_000),
        min_priority_fee_per_gas: U256::from(1_000_000_000),
        flashbots_enabled: false,
        private_rpc_url: Some("https://your-private-rpc.com".to_string()),
    });

    let trade_executor = Arc::new(TradeExecutor {
        client: Arc::new(http_provider),
        flash_loan_providers: vec![flash_loan],
        config: config.clone(),
        pending_txs: Arc::new(Mutex::new(HashMap::new())),
        mev_protection: mev_protection.clone(),
    });

    let engine = Arc::new(ArbitrageEngine {
        oracle,
        circuit_breaker,
        pools: create_sample_pools(),
        config: config.clone(),
        trade_executor: trade_executor.clone(),
        opportunity_queue: Arc::new(Mutex::new(VecDeque::new())),
        historical_opportunities: Arc::new(RwLock::new(Vec::new())),
        mev_protection,
    });

    let health_check = Arc::new(HealthCheck::new(Duration::from_secs(10)));

    let bot = ArbitrageBot {
        engine,
        executor: trade_executor,
        config,
        health_check,
    };

    // Run the bot with error handling
    println!("Starting arbitrage bot...");
    if let Err(e) = bot.run().await {
        error!("Bot crashed: {}", e);
        Err(e)
    } else {
        Ok(())
    }
}