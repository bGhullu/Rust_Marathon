use anyhow::Result; // Advanced error handling
use reqwest::Client; // HTTP client for API calls
use serde::Deserialize; // JSON parsing
use tokio::sync::mpsc; // Channel for price collection
use tokio::time::{interval, Duration}; // Time for polling

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

// Main async funciton
#[tokio::main]
async fn main () -> Result<()>{
    let client = Client::new(); // Reusable HTTP client
    let (tx, mut rx) = mpsc::channel::<(String, f64)>(32); // Channel for (source, price)

    // Spawn Coingecko task
    let tx1 = tx.clone();
    tokio::spawn(async move{
        loop{
            match tokio::time::timeout(Duration::from_secs(5), fetch_coingecko_price(&client)).await{
                Ok(Ok(price)) => {
                    if let Err(e) = tx1.send(("CoinGecko".to_string(),pricce)).await{
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

}



