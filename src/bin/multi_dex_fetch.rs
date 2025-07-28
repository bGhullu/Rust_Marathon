use anyhow::Result;
use dotenv::dotenv;
use ethers::prelude::*;
use serde::{ Deserialize};
use tokio::time::{timeout,Duration};
use std::str::FromStr;
use std::sync::Arc;
use ethers::abi::Abi;

#[derive(Debug, Deserialize)]
struct Slot0{
    #[serde(rename= "sqrtPriceX96")]
    sqrt_price_x96: U256,
    tick: i32,
}   

async fn fetch_price(
    provider: Arc<Provider<Http>>,
    pool_address: Address,
    abi: &Abi,
    is_token0_usdc: bool,
)->Result<f64>{
    let contract = Contract::new(pool_address,abi.clone(),provider.clone());
    let slot0: (U256,i32) = match timeout(Duration::from_secs(5), contract.method::<_, (U256,i32)>("slot0",())?.call()).await{
        Ok(Ok(slot0))=>slot0,
        Ok(Err(e))=> return Err(anyhow::anyhow!("slot0 call failed: {}",e)),
        Err(_)=>return Err(anyhow::anyhow!("slot0 call timed out")),
    };
    let slot_data = Slot0{
        sqrt_price_x96: slot0.0,
        tick: slot0.1,
    };

    let sqrt_price = slot_data.sqrt_price_x96;
    let price = if is_token0_usdc {
        let denominator = (sqrt_price * sqrt_price)/U256::from(10).pow(U256::from(12));
        (U256::from(1)<< 192) * U256::from(10).pow(U256::from(6))/ denominator 
    } else {
        (sqrt_price* sqrt_price * U256::from(10).pow(U256::from(6)))
        / ((U256::from(1)<<192)*U256::from(10).pow(U256::from(18)))
    };
 
    Ok(price.as_u128() as f64/ 10u128.pow(6) as f64)

}

#[tokio::main]
async fn main()-> Result<()>{
    dotenv().ok();
    let infura_key = std::env::var("INFURA_KEY")
        .map_err(|e| anyhow::anyhow!("Missing Infura key: {}",e))?;
    let eth_provider = Provider::<Http>::try_from(
        format!("https://mainnet.infura.io/v3/{}",infura_key))
        .map_err(|e| anyhow::anyhow!("Failed connecting mainnet: {}",e))?;
    let eth_provider = Arc::new(eth_provider);

    let poly_provider = Provider::<Http>::try_from(format!(
        "https://polygon-mainnet.infura.io/v3/{}",infura_key
    ))?;
    let poly_provider = Arc::new(poly_provider);

    let abi: Abi = serde_json::from_slice(include_bytes!("../../uniswap_v3_pool.json"))
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}",e))?;

    let uniswap_pool: Address = H160::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?; // USDC/WETH
    let quickswap_pool: Address =H160::from_str("0xa374094527e1673a86de625aa59517c5de346d32")?; // USDC/WETH

    let uniswap_price = fetch_price(eth_provider, uniswap_pool, &abi, true).await?;
    let quickswap_price = fetch_price(poly_provider, quickswap_pool, &abi, true).await?;
    println!("Uniswap USDC/WETH price: ${}", uniswap_price);
    println!("Quickswap USDC/WETH price: ${}", quickswap_price);

    let price_diff = (uniswap_price- quickswap_price).abs();
    if price_diff> 10.0{
        println!("Arbitrage opportunity! Price diff: ${:.2}", price_diff);
    }

    Ok(())
  
   
}
