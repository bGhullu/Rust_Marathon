use anyhow::Result;
use dotenv::dotenv;
use ethers::prelude::*;
use serde::Deserialize;
use tokio::time::{timeout, Duration};
use std::{str::FromStr, time};



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

    let uniswap_address: Address= H160::from_str("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8")?;
    let contract = Contract::from_json(&provider,uniswap_address, include_bytes!("uniswap_v3_pool.json"))?;

    let slot0: (U256, i32) = match timeout(Duration::from_secs(5), contract.method::<_,(U256, i32)>("slot0", ())?.call()).await {

    };


    Ok(())
}