use anyhow::Result;
use dotenv::dotenv;
use ethers::abi::Abi;
use ethers::prelude::*;
use tokio::time::{timeout,Duration};
use std::str::FromStr;
use std::sync::Arc;
use serde::Deserialize;

#[derive(Debug,Deserialize)]
struct Slot0{
    #[serde(rename="sqrtPriceX96")]
    sqrt_price_x96: U256,
    tick: i32,
}

async fn fetch_price(
    provider: Arc<Provider<Http>>,
    abi: &Abi,
    pool_address: Address,
    usdc_is_token0: bool,
)-> Result<f64>{

    let contract = Contract::new(pool_address,  abi.clone(),provider.clone());
    let slot0: (U256,i32)= match timeout(Duration::from_secs(5), contract.method::<_,(U256,i32)>("slot0",())?.call()).await{
        Ok(Ok(slot0))=>slot0,
        Ok(Err(e))=> return Err(anyhow::anyhow!("No data available: {}",e)),
        Err(_)=> return Err(anyhow::anyhow!("Contract method timed out")),
    };
    let slot_data = Slot0{
        sqrt_price_x96: slot0.0,
        tick: slot0.1,
    };

    let sqrt_price = slot_data.sqrt_price_x96;
    let price = if usdc_is_token0 {
        let denomintor  = (sqrt_price *sqrt_price)/ U256::from(10).pow(U256::from(12));
        (U256::from(1)<<192) * U256::from(10).pow(U256::from(6))/ denomintor
    } else {
        (sqrt_price * sqrt_price) * U256::from(10).pow(U256::from(18))/ (U256::from(1)<<192) * U256::from(10).pow(U256::from(6))
    };
    Ok(price.as_u128() as f64 / 10u128.pow(6) as f64)

}

async fn execute_swap(
    client: Arc<SignerMiddleware<Arc<Provider<Http>>,LocalWallet>>,
    pool_address: Address,
    abi:&Abi,
    amount_in: U256,
    is_buy_eth: bool,
)-> Result<()>{

    let contract = Contract::new(pool_address,abi.clone(),client.clone());
    let receipient= client.address();
    let deadline = U256::from(9999999999u64);
    let params = if is_buy_eth{
        (
            receipient, 
            amount_in, 
            U256::zero(), 
            U256::zero(),
            false
        )
    }else {
        (
            receipient, 
            U256::zero(), 
            amount_in, 
            U256::zero(), 
            true
        )
    };
    let call = contract.method::<_,(I256, I256)>("swap", params)?;
    let gas_estimate = call.estimate_gas().await
        .map_err(|e|anyhow::anyhow!("Gas estimation failed: {}",e))?;
    let tx= call.gas(gas_estimate * 120 / 100) // 20% buffer
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Swap failed: {}",e ))?;
    let receipt = tx.await
        .map_err(|e| anyhow::anyhow!("Transaction failed: {}", e))?;
    println!("Swap executed: {:?}", receipt);

    Ok(())
}



#[tokio::main]
async fn main()-> Result<()>{
    dotenv().ok();
    let infura_key = std::env::var("INFURA_KEY")
        .map_err(|e| anyhow::anyhow!("No infura key found: {}",e))?;
    let private_key = std::env::var("PRIVATE_KEY")
        .map_err(|e| anyhow::anyhow!("Missing Privagte key: {}", e));
    let private_key_str = private_key?;
    let provider = Provider::<Http>::try_from(format!(
        "https://sepolia.infura.io/v3/{}",infura_key))
        .map_err(|e|anyhow::anyhow!("Cannot connect to the provider: {}",e))?;

    let provider = Arc::new(provider);
    let wallet = private_key_str.parse::<LocalWallet>()?.with_chain_id(5u64);
    let client = Arc::new(SignerMiddleware::new(provider.clone(),wallet));
    let abi: Abi = serde_json::from_slice(include_bytes!("../../uniswap_v3_pool.json"))
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}", e))?;
    let pool_address: Address = H160::from_str("0x02352390892360269283692")?;
    let price = fetch_price(provider, &abi, pool_address, true).await?;
    println!("USDC/WETH price: {:.2}",price);
    let amount_in = U256::from(1_000_000);  // 1 USDC is 6 decimals
    execute_swap(client, pool_address, &abi, amount_in, true).await?;


    Ok(())
}

