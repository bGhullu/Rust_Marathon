use anyhow::Result;
use dotenv::dotenv;
use ethers::abi::Abi;
use ethers::prelude::*;
use tokio::time::{timeout, Duration};
use std::str::FromStr;
use std::sync::Arc;
use serde::Deserialize;

// Struct to represent Uniswap V3's slot0 return data
#[derive(Debug, Deserialize)]
struct Slot0 {
    #[serde(rename = "sqrtPriceX96")]
    sqrt_price_x96: U256,  // Square root of the price in Q64.96 fixed-point format
    tick: i32,            // Current tick representing the price
}

/// Fetches the current price from a Uniswap V3 pool
/// 
/// Parameters:
/// - provider: Ethereum provider (HTTP, Websocket, etc.)
/// - abi: The Uniswap V3 Pool ABI
/// - pool_address: Address of the Uniswap V3 pool
/// - usdc_is_token0: Whether USDC is token0 in the pool (affects price calculation)
async fn fetch_price(
    provider: Arc<Provider<Http>>,
    abi: &Abi,
    pool_address: Address,
    usdc_is_token0: bool,
) -> Result<f64> {
    // Create contract instance
    let contract = Contract::new(pool_address, abi.clone(), provider.clone());
    
    // Call slot0() with 5 second timeout to prevent hanging
    let slot0: (U256, i32) = match timeout(Duration::from_secs(5), contract.method::<_, (U256, i32)>("slot0", ())?.call()).await {
        Ok(Ok(slot0)) => slot0,  // Success case
        Ok(Err(e)) => return Err(anyhow::anyhow!("Contract call failed: {}", e)),
        Err(_) => return Err(anyhow::anyhow!("Contract method timed out")),
    };

    // Parse into our Slot0 struct
    let slot_data = Slot0 {
        sqrt_price_x96: slot0.0,
        tick: slot0.1,
    };

    // Calculate price from sqrtPriceX96
    let sqrt_price = slot_data.sqrt_price_x96;
    let price = if usdc_is_token0 {
        // If USDC is token0: price = (1 << 192) * 1e6 / (sqrtPriceX96^2 / 1e12)
        let denominator = (sqrt_price * sqrt_price) / U256::from(10).pow(U256::from(12));
        (U256::from(1) << 192) * U256::from(10).pow(U256::from(6)) / denominator
    } else {
        // If USDC is token1: price = (sqrtPriceX96^2 * 1e18) / (1 << 192) * 1e6
        (sqrt_price * sqrt_price) * U256::from(10).pow(U256::from(18)) / (U256::from(1) << 192) * U256::from(10).pow(U256::from(6))
    };

    // Convert to f64 with proper decimal places
    Ok(price.as_u128() as f64 / 10u128.pow(6) as f64)
}

/// Executes a swap on Uniswap V3
/// 
/// Parameters:
/// - client: Authenticated Ethereum client with signer
/// - pool_address: Address of the Uniswap V3 pool
/// - abi: The Uniswap V3 Pool ABI
/// - amount_in: Amount to swap (in USDC decimals if selling USDC)
/// - is_buy_eth: Whether we're buying ETH (true) or selling ETH (false)
async fn execute_swap(
    client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
    pool_address: Address,
    abi: &Abi,
    amount_in: U256,
    is_buy_eth: bool,
) -> Result<()> {
    // Create contract instance
    let contract = Contract::new(pool_address, abi.clone(), client.clone());
    
    // Set swap parameters
    let recipient = client.address();  // Send tokens to our own address
    let deadline = U256::from(9999999999u64);  // Far future deadline
    
    // Prepare swap parameters based on direction
    let params = if is_buy_eth {
        // Buying ETH: exact USDC in, minimum ETH out
        (
            recipient,    // recipient
            amount_in,    // amountIn (USDC)
            U256::zero(), // amountOutMinimum (we accept any ETH amount)
            U256::zero(), // sqrtPriceLimitX96 (no price limit)
            false         // zeroForOne: USDC -> ETH
        )
    } else {
        // Selling ETH: exact ETH in, minimum USDC out
        (
            recipient,
            U256::zero(), // amountIn (will be ETH value sent)
            amount_in,    // amountOutMinimum (USDC)
            U256::zero(),
            true          // zeroForOne: ETH -> USDC
        )
    };

    // Create the swap call
    let swap_call = contract.method::<_, (I256, I256)>("swap", params)?;
    
    // Estimate gas with 20% buffer
    let gas_estimate = swap_call.estimate_gas().await
        .map_err(|e| anyhow::anyhow!("Gas estimation failed: {}", e))?;
    
    // Execute swap with gas buffer
    let pending_tx = swap_call
        .gas(gas_estimate * 120 / 100)  // 20% buffer
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Swap failed: {}", e))?;
    
    // Wait for transaction receipt
    let receipt = pending_tx
        .await?
        .ok_or_else(|| anyhow::anyhow!("Transaction dropped from mempool"))?;
    
    println!("Swap executed: {:?}", receipt);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Infura key from environment
    let infura_key = std::env::var("INFURA_KEY")
        .map_err(|e| anyhow::anyhow!("No Infura key found: {}", e))?;
    
    // Get private key from environment
    let private_key = std::env::var("PRIVATE_KEY")
        .map_err(|e| anyhow::anyhow!("Missing private key: {}", e))?;
    
    // Create provider (connection to Ethereum node)
    let provider = Provider::<Http>::try_from(format!(
        "https://sepolia.infura.io/v3/{}", infura_key
    )).map_err(|e| anyhow::anyhow!("Cannot connect to provider: {}", e))?;
    
    // Wrap provider in Arc for thread-safe sharing
    let provider = Arc::new(provider);
    
    // Create wallet from private key (chain ID 5 = Goerli)
    let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(5u64);
    
    // Create authenticated client
    let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));
    
    // Load Uniswap V3 Pool ABI
    let abi: Abi = serde_json::from_slice(include_bytes!("../../uniswap_v3_pool.json"))
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}", e))?;
    
    // Example pool address (replace with actual address)
    let pool_address: Address = H160::from_str("0x123...")?;
    
    // Fetch current price
    let price = fetch_price(provider.clone(), &abi, pool_address, true).await?;
    println!("USDC/WETH price: {:.2}", price);
    
    // Execute sample swap (1 USDC for ETH)
    let amount_in = U256::from(1_000_000);  // 1 USDC (6 decimals)
    execute_swap(client, pool_address, &abi, amount_in, true).await?;
    
    Ok(())
}