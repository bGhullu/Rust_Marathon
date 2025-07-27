

use anyhow::Result; // Advanced Error handling
use dotenv::dotenv; 
use ethers::prelude::*;
use tokio::sync::mpsc;
use primitive_types::U256;
use  tokio::time::{timeout,Duration};
use std::str::FromStr; // for H160 parsing

#[tokio::main]

async fn main() -> Result<()> {
    dotenv().ok();
    let infura_key= std::env::var("INFURA_KEY").map_err(|e| anyhow::anyhow!("Missing Infura Key:{}",e))?;
    let eth_provider = Provider::<Http>::try_from(
        format!("https://mainnet.infura.io/v3/{}",infura_key)
    )?;
    let arb_provider=Provider::<Http>::try_from(
        format!("https://arbitrum-mainnet.infura.io/v3/{}",infura_key)
    )?;

    let (tx, mut rx)= mpsc::channel::<(String, u64,u64,U256)>(32);

    let tx1 = tx.clone();
    tokio::spawn(async move {
        match timeout(Duration::from_secs(5),fetch_eth_data(&eth_provider)).await{
           Ok(Ok((block_number, timestamp, balance)))=>{
                if let Err(e) = tx1.send(("Ethereum".to_string(),block_number,timestamp,balance)).await{
                    eprintln!("Failed to send Ethereum data:{}",e);
                } else {
                    println!("Sent Ethereum data: block: {}, balance: {} ETH", block_number, balance);
                }
           }
           Ok(Err(e))=>eprintln!("Ethereum fetch error:{}",e),
           Err(_) => eprintln!("Etherem timout after 5 seconds"),

        }
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        match timeout(Duration::from_secs(5),fetch_arb_data(&arb_provider)).await{
            Ok(Ok((block_number, timestamp, balance)))=>{
                if let Err(e) =  tx2.send(("Arbiturm".to_string(),block_number, timestamp, balance)).await{
                    eprintln!("Failed to send Arbitrum Data:{}",e);
                } else {
                    println!("Sent Arbitrum data: block{}, balance{} ETH", block_number, balance);
                }
            }
            Ok(Err(e)) => eprintln!("Failed to fetch Arbitrum data{}", e),
            Err(_)=> eprintln!("Arbitrum timeout after 5 seconds"),
        }
    });

    let mut data = Vec::new();
    for _ in 0..2{
        if let Some((chain, block_number, timestamp, balance))= rx.recv().await{
            data.push((chain,block_number,timestamp,balance));
        }
    }

    if data.len() ==2{
        for (chain, block_number, timestamp, balance) in &data{
            println!(
                "{}: Block #{}, Timestamp: {}, Balance: {} ETH",
                chain, block_number, timestamp, ethers::utils::format_ether(*balance)
            );
        }
    } else {
        println!("Not enough data received!");
    }

    Ok(())

}

async fn fetch_eth_data(provider: &Provider<Http>)-> Result<(u64,u64,U256)>{
    let block = provider.get_block(BlockNumber::Latest).await?.ok_or_else(|| anyhow::anyhow!("No block found"))?;
    let block_number = block.number.ok_or_else(|| anyhow::anyhow!("No block number"))?.as_u64();
    let timestamp = block.timestamp.as_u64();
    let address: H160= "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse()?;
    let balance =provider.get_balance(address, None).await?;
    // let balance = provider.get_balance(H160::from_str("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?.into(),None).await?;
    Ok((block_number,timestamp, balance))
}

async fn fetch_arb_data(provider: &Provider<Http>) -> Result<(u64,u64,U256)> {
    let block = provider.get_block(BlockNumber::Latest).await?.ok_or_else(|| anyhow::anyhow!("No block found"))?;
    let block_number= block.number.ok_or_else(|| anyhow::anyhow!("No block number"))?.as_u64();
    let timestamp = block.timestamp.as_u64();
    let address:H160= "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse()?;
    let balance = provider.get_balance(address,None).await?;
    // let balance= provider.get_balance(H160::from_str("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?.into(),None).await?;
    Ok((block_number, timestamp, balance))
}
