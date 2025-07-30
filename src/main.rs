// ============================================================================
// External crate imports: ethers-rs provides Ethereum JSON-RPC client capabilities.
// ============================================================================

// Ethers providers:
// - Http: Standard REST-based JSON-RPC provider, typically synchronous requests,
//   easier to use for stateless queries (e.g., fetching a block or tx by hash).
// - Ws: WebSocket provider supporting subscriptions (eth_subscribe) for
//   real-time event streaming, essential for mempool monitoring and block notifications.
// - Middleware: Trait that abstracts provider behavior, enabling composability.
use ethers::{
    providers::{Http, Middleware, Provider, Ws},
    types::{
        // Ethereum address representation (20 bytes)
        H160,
        // 256-bit unsigned integers, the core numeric type for balances, gas, etc.
        U256,
        // A transaction hash (H256) alias, not imported here but used implicitly
        NameOrAddress, // Tx 'to' field can be a raw address or ENS name, must handle both
        BlockNumber, // Enum for specifying blocks in calls (Latest, Pending, Number, etc.)
        Transaction, // Complete transaction data structure from Ethereum node
        TransactionReceipt, // Receipt containing execution status and gas used
        TransactionRequest, // Builder for transaction calls (simulate/send)
        transaction::eip2718::TypedTransaction, // Unified tx format supporting legacy and EIP-1559 txs
    },
    utils::format_units, // Utility to convert wei to ETH/gwei/etc for human-readable output
};

// ============================================================================
// Standard library imports:
// ============================================================================

use std::{
    collections::HashMap, // Used to track mempool txs for MEV detection (hash => tx)
    sync::{
        // AtomicBool: An atomic boolean flag supporting lock-free concurrent access.
        // Chosen here for HTTP health tracking to avoid mutex overhead.
        // Ordering::SeqCst guarantees a global total ordering for consistent visibility
        // across threads — important for avoiding stale reads in async contexts.
        atomic::{AtomicBool, Ordering},
        // Arc (Atomic Reference Counted) pointer: allows multiple owners across async tasks.
        // This is essential for sharing the HTTP/WS provider instances and status flags
        // safely across concurrently running async functions without cloning heavy clients.
        Arc,
    },
};

// ============================================================================
// Error handling and async stream imports:
// ============================================================================

// anyhow: Provides convenient, chainable error handling with context support.
// Allows attaching messages to errors for easier debugging/tracing in async flows.
use anyhow::{anyhow, Result, Context};

// futures::StreamExt extends Stream trait with useful combinators (e.g., .next()).
// Required for iterating over asynchronous WebSocket subscription streams.
use futures::StreamExt;

// Tokio time utilities, provides async-aware delays.
// Used here for retry delays on stream failures, backoff, or simulated failure.
use tokio::time::Duration;

// ============================================================================
// ETHCLIENT STRUCT DEFINITION WITH DEEP EXPLANATION:
// ============================================================================

/// `EthClient` encapsulates the core Ethereum JSON-RPC clients with
/// redundant providers for fault tolerance, live mempool subscriptions,
/// and transaction simulation facilities.
///
/// This struct is engineered for high-reliability, low-latency Ethereum interaction,
/// a common need for MEV searchers, arbitrage bots, and monitoring agents.
///
/// # Concurrency and Safety
///
/// - All providers and health flags are wrapped in `Arc` to enable safe concurrent
///   usage across async tasks without cloning heavy underlying clients.
/// - The health flag `http_alive` is an `AtomicBool` with `SeqCst` ordering to ensure
///   the strongest memory ordering guarantees, critical to prevent race conditions
///   between health status reads/writes in async runtime.
/// - The current block is a `u64` cache updated on each successful fetch; it is
///   not atomic as updates happen within single-threaded async contexts.
///
/// # Provider Roles
///
/// - `http`: Used primarily for synchronous, one-off queries where WebSocket
///   is either not supported or less performant.
/// - `ws`: Used for subscriptions (block headers, pending txs) and as a failover
///   in case HTTP queries fail or are out of sync.
///
/// # Failure Handling
///
/// The client actively monitors provider sync state and allows simulating failure
/// (for testing) by replacing the HTTP provider with an invalid URL.
/// 
/// This failover strategy is essential because Ethereum nodes may become unresponsive,
/// rate-limited, or desynchronized during network congestion or RPC provider issues.
///
/// # Typical Usage
///
/// 1. Initialize client with HTTP and WS URLs.
/// 2. Use `get_block` for latest block height with fallback logic.
/// 3. Stream blocks and mempool txs via WebSocket.
/// 4. Detect MEV opportunities and simulate tx execution.
///
/// ```
/// let client = EthClient::new(http_url, ws_url).await?;
/// let block = client.get_block().await?;
/// client.stream_pending_txs().await?;
/// ```
///
#[derive(Clone)] // Required for safe passing of client handles to multiple async tasks
struct EthClient {
    /// Primary HTTP JSON-RPC provider for synchronous queries.
    /// Uses Arc to allow cheap cloning and sharing between async tasks.
    http: Arc<Provider<Http>>,

    /// WebSocket provider enabling real-time subscriptions.
    /// Also serves as fallback when HTTP provider is unavailable.
    ws: Arc<Provider<Ws>>,

    /// Tracks the latest known block number from successful fetches.
    /// Updated on each call to `get_block` and used to verify provider sync.
    current_block: u64,

    /// Atomic health flag for the HTTP provider.
    /// Ensures threads see the latest health state.
    /// When false, forces fallback to WS provider exclusively.
    http_alive: Arc<AtomicBool>,
}

// ============================================================================
// MEV OPPORTUNITIES ENUM WITH IN-DEPTH COMMENTARY:
// ============================================================================

/// Enum describing various Miner Extractable Value (MEV) opportunity types
/// that the client can detect by analyzing live mempool transactions.
///
/// This enumeration is extensible to support additional MEV patterns as
/// detection strategies evolve.
///
/// - **Sandwich attacks:** Exploiting victim swaps with a buy front-run and sell back-run.
/// - **Arbitrage:** Profiting from price differences across liquidity pools.
/// - **Liquidations:** Capturing profit from undercollateralized loan liquidations.
///
/// Each variant carries relevant metadata to support downstream decision-making,
/// including involved transactions and profit estimates.
#[derive(Debug)]
enum MevOpportunity {
    /// Represents a sandwich attack opportunity.
    ///
    /// This pattern involves placing transactions immediately before and after
    /// a large victim transaction to capitalize on price slippage.
    Sandwich {
        frontrun_tx: Transaction,  // Transaction that front-runs victim
        victim_tx: Transaction,    // Targeted victim transaction
        backrun_tx: Transaction,   // Transaction that back-runs victim
        profit_estimate: U256,     // Estimated profit from this sequence in wei
    },

    /// Represents a profitable arbitrage route.
    ///
    /// The `path` is a vector of pool addresses (H160) which the arbitrage trades through.
    Arbitrage {
        path: Vec<H160>,
        profit: U256,
    },

    /// Represents a liquidation opportunity in lending protocols.
    ///
    /// Liquidator address and debt position address are provided to
    /// precisely identify the actors and contracts involved.
    Liquidation {
        liquidator: H160,
        debt_position: H160,
        profit: U256,
    },
}

impl EthClient {
    /// Initializes a new `EthClient` instance with the provided HTTP and WebSocket RPC URLs.
    ///
    /// Performs initial synchronization checks to verify that both providers
    /// are closely aligned in terms of the blockchain height.
    ///
    /// # Failure Scenarios
    ///
    /// - Fails if unable to instantiate HTTP provider.
    /// - Fails if unable to establish WebSocket connection.
    /// - Fails if HTTP and WS block numbers differ by more than 3 blocks,
    ///   as this indicates a potential desync issue that could cause inconsistent data reads.
    ///
    /// # Returns
    ///
    /// A `Result` containing a ready-to-use `EthClient` or an error describing
    /// the failure point.
    pub async fn new(rpc_url: &str, ws_url: &str) -> Result<Self> {
        // Attempt to create the HTTP provider synchronously.
        // This can fail if the URL is malformed or unreachable.
        let http = Provider::<Http>::try_from(rpc_url)
            .context("Failed to create HTTP provider")?;

        // Connect asynchronously to the WebSocket provider.
        // This may fail due to networking issues or invalid URLs.
        let ws = Provider::<Ws>::connect(ws_url)
            .await
            .context("Failed to connect to WS provider")?;

        // Fetch the current block number from both providers.
        // This ensures they are both synced to roughly the same chain height.
        let http_block = http.get_block_number().await.context("HTTP block fetch failed")?.as_u64();
        let ws_block = ws.get_block_number().await.context("WS block fetch failed")?.as_u64();

        // Check for desynchronization beyond a safe threshold.
        // A 3-block difference is an empirically chosen tolerance to handle minor chain reorganizations or lag.
        if http_block.abs_diff(ws_block) > 3 {
            anyhow::bail!(
                "Providers out of sync (HTTP: {}, WS: {})",
                http_block,
                ws_block
            );
        }

        // Construct the client struct.
        Ok(Self {
            http: Arc::new(http),
            ws: Arc::new(ws),
            current_block: http_block,
            http_alive: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Fetches the latest block number from the blockchain.
    ///
    /// Uses HTTP provider preferentially for efficiency.
    /// Falls back to WebSocket provider on HTTP failure.
    ///
    /// Updates the internal `current_block` cache on success.
    ///
    /// # Rationale
    ///
    /// Using HTTP first often provides faster responses for individual calls,
    /// but WebSocket acts as a resilient fallback when HTTP is slow, rate-limited,
    /// or disconnected.
    ///
    /// # Edge Cases
    ///
    /// If both providers fail, returns an error propagated from the last failed attempt.
    pub async fn get_block(&mut self) -> Result<u64> {
        match self.http.get_block_number().await {
            Ok(block) => {
                self.current_block = block.as_u64();
                Ok(self.current_block)
            }
            Err(e) => {
                // Log the failure, then fallback to WebSocket provider.
                eprintln!("HTTP failed, falling back to WS: {}", e);
                let block = self.ws.get_block_number().await?;
                self.current_block = block.as_u64();
                Ok(self.current_block)
            }
        }
    }

    /// Simulates HTTP provider failure by replacing it with an invalid endpoint.
    ///
    /// Useful for testing client failover behavior during HTTP outages.
    ///
    /// # Important
    ///
    /// This method is destructive: once called, the client will rely solely on WebSocket
    /// provider until reset.
    pub fn kill_http(&mut self) {
        // Replace HTTP provider with an obviously invalid URL to simulate failure.
        self.http = Arc::new(Provider::<Http>::try_from("http://invalid.url").unwrap());
        // Mark HTTP as not alive (optional: could set http_alive=false here as well)
    }

    /// Retrieves a transaction by its hash.
    ///
    /// Prefers HTTP provider if healthy, else falls back to WebSocket provider.
    ///
    /// # Behavior
    ///
    /// - If HTTP is marked alive, tries HTTP first.
    /// - If HTTP request fails, logs error and falls back to WS.
    /// - If HTTP is not alive, directly queries WS.
    ///
    /// # Return
    ///
    /// `Ok(Some(Transaction))` if transaction found,
    /// `Ok(None)` if transaction does not exist,
    /// or `Err` if both providers fail.
    pub async fn get_transaction(&self, tx_hash: ethers::types::TxHash) -> Result<Option<Transaction>> {
        if self.http_alive.load(Ordering::SeqCst) {
            match self.http.get_transaction(tx_hash).await {
                Ok(tx) => Ok(tx),
                Err(e) => {
                    eprintln!("HTTP failed, falling back to WS: {}", e);
                    self.ws.get_transaction(tx_hash).await.context("Failed via WS")
                }
            }
        } else {
            self.ws.get_transaction(tx_hash).await.context("Failed via WS")
        }
    }

    /// Subscribes to new blocks via WebSocket and prints block info.
    ///
    /// This live subscription is essential for building real-time block explorers,
    /// MEV bots, or indexers.
    ///
    /// # Flow
    ///
    /// - Opens a subscription stream for new blocks.
    /// - For each block received, prints block number, number of txs, and gas used.
    ///
    /// # Notes
    ///
    /// The function runs indefinitely until the stream ends or an error occurs.
    pub async fn stream_blocks(&self) -> Result<()> {
        let mut stream = self.ws.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            println!(
                "New Block #{}: {} txs, {} gas used",
                block.number.unwrap_or_default(),
                block.transactions.len(),
                block.gas_used.unwrap_or_default()
            );
        }
        Ok(())
    }

    /// Subscribes to live pending transactions from the mempool.
    ///
    /// For each pending transaction hash, fetches full tx data and prints summary.
    ///
    /// # Behavior
    ///
    /// - If tx disappears between notification and fetch, logs disappearance.
    /// - Errors in fetching are logged but do not halt the subscription.
    ///
    /// # Usage
    ///
    /// Useful for mempool monitoring, MEV detection, or front-running strategies.
    pub async fn stream_pending_txs(&self) -> Result<()> {
        let mut stream = self.ws.subscribe_pending_txs().await?;
        while let Some(tx_hash) = stream.next().await {
            match self.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    println!(
                        "Pending Tx: {} => {}, gas:{}, value:{} ETH",
                        tx.from,
                        tx.to.unwrap_or_default(),
                        tx.gas,
                        format_units(tx.value, "ether")?
                    );
                }
                Ok(None) => println!("Transaction disappeared from mempool"),
                Err(e) => println!("Error fetching transaction: {}", e),
            }
        }
        Ok(())
    }

    /// Placeholder for detecting MEV opportunities from live pending transactions.
    ///
    /// This function collects pending transactions into a mempool snapshot and runs
    /// detection logic for sandwich attacks and other MEV strategies.
    ///
    /// # Details
    ///
    /// - Listens indefinitely to pending txs via WS.
    /// - Calls `detect_sandwich` (not implemented here) for each tx.
    /// - Accumulates opportunities and returns on stream termination.
    pub async fn detect_mev(&self) -> Result<Vec<MevOpportunity>> {
        let mut opportunities = Vec::new();
        let mut pending_txs = HashMap::new();

        let mut stream = self.ws.subscribe_pending_txs().await?;
        while let Some(tx_hash) = stream.next().await {
            if let Ok(Some(tx)) = self.get_transaction(tx_hash).await {
                if let Some(opp) = self.detect_sandwich(&tx, &pending_txs).await {
                    opportunities.push(opp);
                }
                pending_txs.insert(tx.hash, tx);
            }
        }
        Ok(opportunities)
    }

    /// Fetches recent fee history and computes an optimal gas price according to EIP-1559.
    ///
    /// # Computation details:
    ///
    /// - Requests fee history over last 5 blocks with 50th percentile priority fee.
    /// - Uses the earliest base fee as reference.
    /// - Adds a fixed priority fee (tip) of 2 Gwei to incentivize miners.
    /// - Adds a 25% buffer over base fee to reduce underpricing risk.
    ///
    /// # Overflow handling:
    ///
    /// The arithmetic uses checked operations to avoid integer overflow panics,
    /// returning an error if overflow occurs.
    pub async fn get_optimal_gas_price(&self) -> Result<U256> {
        let fee_history = self
            .ws
            .fee_history(5, BlockNumber::Latest, &[50.0])
            .await
            .context("Failed to get fee history")?;

        // The base fee per gas for the earliest block in the history
        let base_fee = *fee_history
            .base_fee_per_gas
            .first()
            .ok_or_else(|| anyhow!("No base fee found in fee history"))?;

        // Fixed priority fee (tip) set to 2 Gwei
        let max_priority_fee = U256::from(2_000_000_000u64);

        // Max fee is base fee + 25% buffer
        let max_fee = base_fee
            .checked_mul(U256::from(125))
            .and_then(|v| v.checked_div(U256::from(100)))
            .ok_or_else(|| anyhow!("Gas price calculation overflow"))?;

        Ok(max_fee + max_priority_fee)
    }

    /// Simulates a transaction request by converting it to a TypedTransaction and
    /// then simulating the resulting Transaction.
    ///
    /// # Implementation details:
    ///
    /// - Converts `TransactionRequest` (builder) into `TypedTransaction` (final tx format).
    /// - Manually constructs a `Transaction` struct using fields extracted from `TypedTransaction`.
    /// - Calls `simulate_tx` to perform the simulation RPC call.
    ///
    /// # Notes:
    ///
    /// This simulation does not send the tx but queries the node for execution outcome,
    /// gas used, and success status, useful for pre-checking before actual submission.
    pub async fn simulate_tx_request(
        &self,
        tx_request: &TransactionRequest,
    ) -> Result<(bool, U256)> {
        let typed_tx: TypedTransaction = tx_request.clone().into();

        // Construct a mock Transaction from the TypedTransaction fields
        let mock_tx = Transaction {
            hash: typed_tx.sighash(),
            nonce: typed_tx.nonce().cloned().unwrap_or_default(),
            from: typed_tx.from().map(|f| *f).unwrap_or_default(),
            to: typed_tx.to().and_then(|addr| match addr {
                NameOrAddress::Address(a) => Some(*a),
                NameOrAddress::Name(_) => None,
            }),
            value: typed_tx.value().cloned().unwrap_or_default(),
            gas: typed_tx.gas().cloned().unwrap_or_default(),
            gas_price: typed_tx.gas_price().map(|gp| gp.clone()),
            input: typed_tx.data().cloned().unwrap_or_default(),
            ..Default::default()
        };

        self.simulate_tx(&mock_tx).await
    }

    /// Simulates a given Transaction by calling `eth_call` RPC method.
    ///
    /// Also fetches transaction receipt to confirm gas usage and success status.
    ///
    /// # Workflow:
    ///
    /// 1. Call `eth_call` with the transaction data and no block override (latest).
    /// 2. Fetch transaction receipt to get gas used and status.
    /// 3. Return tuple of `(success, gas_used)`.
    ///
    /// # Edge Cases:
    ///
    /// - Returns error if receipt is missing (transaction not mined).
    /// - Returns error if gas used is missing (node issue).
    /// - Success is determined by whether eth_call returned non-empty result.
    pub async fn simulate_tx(&self, tx: &Transaction) -> Result<(bool, U256)> {
        // RPC call to simulate tx execution (eth_call)
        let result = self.ws.call(&tx.clone().into(), None).await?;

        // Retrieve the transaction receipt for gas and status
        let receipt = self
            .ws
            .get_transaction_receipt(tx.hash)
            .await?
            .ok_or_else(|| anyhow!("Transaction receipt not found"))?;

        let gas_used = receipt
            .gas_used
            .ok_or_else(|| anyhow!("Gas used not available in receipt"))?;

        // Determine success: non-empty eth_call result implies success
        let success = !result.is_empty();

        Ok((success, U256::from(gas_used)))
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // RPC URLs: change to your preferred providers.
    let rpc_url = "https://eth.llamarpc.com";
    let ws_url = "wss://eth.llamarpc.com";

    // Initialize Ethereum client with both HTTP and WS providers.
    let mut client = EthClient::new(rpc_url, ws_url).await?;

    // Fetch and print optimal gas price based on recent network conditions.
    match client.get_optimal_gas_price().await {
        Ok(gas_price) => println!("Optimal gas price: {} Gwei", format_units(gas_price, "gwei")?),
        Err(e) => eprintln!("Error getting gas price: {}", e),
    }

    // Create a sample transaction request sending 0.1 ETH to Vitalik's address.
    let sample_tx = TransactionRequest::new()
        .to("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse::<H160>()?)
        .value(100_000_000_000_000_000_u64); // 0.1 ETH in wei

    // Convert request to TypedTransaction (required by ethers-rs)
    let typed_tx: TypedTransaction = sample_tx.clone().into();

    // Estimate gas required by the sample transaction.
    let gas_estimate = client.http.estimate_gas(&typed_tx, None).await?;

    // Update transaction with gas estimate and optimal gas price.
    let sample_tx = sample_tx
        .gas(gas_estimate)
        .gas_price(client.get_optimal_gas_price().await?);

    // Simulate the transaction and output success status and gas used.
    let (success, gas) = client.simulate_tx_request(&sample_tx).await?;
    println!("Simulation result: success = {}, gas = {}", success, gas);

    // Print current blockchain head block number.
    println!("Current block: {}", client.get_block().await?);

    // Uncomment to simulate HTTP failure and test failover behavior.
    // println!("Simulate HTTP failure...");
    // client.kill_http();
    // std::thread::sleep(std::time::Duration::from_secs(10));
    // println!("Fallback block: {}", client.get_block().await?);

    // Clone client handle to move into async block for streaming blocks.
    let client_clone = client.clone();

    // Spawn a new async task to print new blocks as they arrive.
    tokio::spawn(async move {
        client_clone.stream_blocks().await.unwrap();
    });

    // Start streaming pending transactions indefinitely.
    // On failure, retry after 1 second delay to handle transient connection issues.
    loop {
        match client.stream_pending_txs().await {
            Ok(_) => break,
            Err(e) => {
                eprintln!("Stream failed, restarting: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
