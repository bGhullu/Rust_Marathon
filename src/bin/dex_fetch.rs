use anyhow::Result;
use dotenv::dotenv;
use ethers::prelude::*;
use serde::Deserialize;
use tokio::time::{timeout, Duration};
use std::str::FromStr;
use std::sync::Arc;
use ethers::abi::Abi;
use ethers::types::U256;


#[derive(Debug,Deserialize)]
struct Slot0{
    #[serde(rename = "sqrtPriceX96")]
    sqrt_price_x96: U256,
    tick: i32,
}

#[tokio::main]
async fn main () -> Result<()>{
    dotenv().ok();

    let infura_key = std::env::var("INFURA_KEY")
        .map_err(|e| anyhow::anyhow!("Missing Infura key: {}",e))?;

    let provider = Provider::<Http>::try_from(
        format!("https://mainnet.infura.io/v3/{}", infura_key)
    )?;

    let provider = Arc::new(provider);
    let uniswap_address: Address= H160::from_str("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8")?;
    let abi: Abi = serde_json::from_slice(include_bytes!("../../uniswap_v3_pool.json"))
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}",e))?;
    let contract= Contract::new(uniswap_address,abi, provider.clone());
    println!("Contract Initialized");
    let slot0: (U256, i32) = match timeout(Duration::from_secs(5),
         contract.method::<(),(U256, i32)>("slot0", ())?.call()
    ).await {
        Ok(Ok(slot0))=> slot0,
        Ok(Err(e))=> return Err(anyhow::anyhow!("slot0 call failed: {}",e)),
        Err(_)=> return Err(anyhow::anyhow!("slot0 call timed out after 5s")),
    };

    let slot0_data = Slot0 {
        sqrt_price_x96: slot0.0,
        tick: slot0.1,
    };
    println!("Uniswap slot0: {:?}", slot0_data);

    // Convert sqrtPriceX96 to ETH/USDC price
    let sqrt_price_64 = slot0_data.sqrt_price_x96.as_u128() as f64;
    let q96 = 2_f64.powf(96.0);
    let price_raw = (sqrt_price_64/q96).powf(2.0);
    let price_adjusted = price_raw* 10_f64.powf(18.0-6.0);

    println!("USDC per WETH: {:.2}",price_adjusted);
    println!("ETH price: ${:.2}", price_adjusted);



    Ok(())
}