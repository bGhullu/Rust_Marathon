use anyhow::Result; // Advanced error handling
use reqwest::Client; // HTTP client for API calls
use serde::Deserialize; // JSON parsing
use std::sync::Arc; // Shared ownership for client
use tokio::sync::mpsc; // Channel for price collection
use tokio::time::{interval, Duration}; // Time for polling
use std::collections::HashMap; 
use std::env;
use dotenv::dotenv;
// Struct for CoinGecko JSON

#[derive(Deserialize, Debug)]
struct CoingeckoPrice{
    ethereum: PriceData,
}

// Nested price struct
#[derive(Deserialize, Debug)]
struct PriceData{
    usd: f64,
}
 #[derive(Debug,Deserialize)]
 struct CoinmarketcapResponse{
    data: HashMap<String, CointmarketcapData>
 }

 #[derive(Debug,Deserialize)]
 struct CointmarketcapData{
    quote: CoinmarketcapQuote,
 }

 #[derive(Debug,Deserialize)]
 struct CoinmarketcapQuote{
    #[serde(rename="USD")]
    usd: CoinmarketcapUsd,
 }
 
 #[derive(Debug,Deserialize)]
 struct CoinmarketcapUsd{
    price: f64,
 }

// Main async funciton
#[tokio::main]
async fn main () -> Result<()>{
    let client = Arc::new(Client::new()); // Reusable HTTP client
    let (tx, mut rx) = mpsc::channel::<(String, f64)>(32); // Channel for (source, price)
    dotenv().ok();

    // Spawn Coingecko task
    let tx1 = tx.clone();
    let client1 = Arc::clone(&client);
    tokio::spawn(async move{
        loop{
            match tokio::time::timeout(Duration::from_secs(5), fetch_coingecko_price(&client1)).await{
                Ok(Ok(price)) => {
                    if let Err(e) = tx1.send(("CoinGecko".to_string(),price)).await{
                        eprintln!("Failed to send Coingeccko price:{}", e);
                        break;
                    }
                    println!("Sent Cointgecko price: ${}", price);
                }
                Ok(Err(e))=> eprintln!("Coingecko fetch error:{}", e),
                Err(_) => eprintln!("Coingecko timeout after 5s"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await; // Wait before retry
        }
    });

    let tx2 = tx.clone();
    let client2 = Arc::clone(&client);
    tokio::spawn(async move {
        loop {
            match tokio::time::timeout(Duration::from_secs(5), fetch_coinmarketcap_price(&client2)).await{
                Ok(Ok(price)) => {
                    if let Err(e) = tx2.send(("CoinMarketCap".to_string(), price)).await {
                        eprintln!("Failed to send CoinMarketCap price: {}",price);
                        break;
                    }
                    println!("Sent CoinMarketCap price: ${}", price);
                }
                Ok(Err(e))=> eprintln!("CoinMarketCap fetch error: {}",e),
                Err(_) => eprintln!("CointMarketCap timout after 5s"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Price Collection loop

    let mut interval = interval(Duration::from_secs(10)); // Check every 10s
    let mut prices = Vec::new();
    loop {
        interval.tick().await; // Wait for next tick
        prices.clear();
        while let Ok((source,price)) = rx.try_recv() {
            prices.push((source,price)); 
        }
        if prices.len()>=2{
            println!("Prices: {:?}", prices);
            let diff = (prices[0].1 - prices[1].1).abs();
            println!("Price Difference: ${}", diff);
            if diff >10.0 {
                println!("Arbitrage opportunity detected!");
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

}


async fn fetch_coingecko_price(client: &Client) -> Result<f64>{
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
    let response = client.get(url).send().await?;
    let price_data = response.json::<CoingeckoPrice>().await?;
    Ok(price_data.ethereum.usd)
}

async fn fetch_coinmarketcap_price(client: &Client) -> Result<f64>{
    let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol=ETH";
    let api_key = env::var("CMC_API_KEY")?;
    let response = client.get(url).header("X-CMC_PRO_API_KEY", api_key).send().await?;
    let price_data = response.json::<CoinmarketcapResponse>().await?;
    let eth_data = price_data.data.get("ETH").ok_or_else (|| anyhow::anyhow!("ETH data is missing"))?;
    Ok(eth_data.quote.usd.price)


}


