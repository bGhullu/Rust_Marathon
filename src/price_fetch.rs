use reqwest::Client; // Imports the HTTP client for making API calls
use serde::Deserialize; // Imports serde's Deserialize trait to parse JSON
use std::error::Error; // Imports the Error trait for error handling

// Defines a struct to match CoinGecko's JSON structure
#[derive(Deserialize, Debug)]
struct CoinGeckoPrice {
    ethereum: PriceData, // Field for ETH price data
}

// Nested struct for the price value
#[derive(Deserialize, Debug)]
struct PriceData {
    usd: f64, // ETH price in USD (floating-point number)
}

// Marks the main function as async with Tokio runtime
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Creates a reusable HTTP client
    let client = Client::new();

    // Spawns two async tasks to fetch prices concurrently
    let coingecko_future = fetch_coingecko_price(&client);
    let coinmarketcap_future = fetch_coinmarketcap_price(&client);

    // Waits for both tasks to complete, collecting results
    let (coingecko_price, coinmarketcap_price) = tokio::join!(
        coingecko_future,
        coinmarketcap_future
    );

    // Prints the fetched prices
    println!("CoinGecko ETH Price: ${}", coingecko_price?);
    println!("CoinMarketCap ETH Price: ${}", coinmarketcap_price?);

    // Returns Ok to indicate success
    Ok(())
}

// Async function to fetch ETH price from CoinGecko
async fn fetch_coingecko_price(client: &Client) -> Result<f64, Box<dyn Error>> {
    // Defines the CoinGecko API URL
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
    // Sends HTTP GET request and awaits response
    let response = client.get(url).send().await?;
    // Parses JSON into CoinGeckoPrice struct
    let price_data = response.json::<CoinGeckoPrice>().await?;
    // Returns the USD price
    Ok(price_data.ethereum.usd)
}

// Placeholder function for CoinMarketCap (uses CoinGecko for now)
async fn fetch_coinmarketcap_price(client: &Client) -> Result<f64, Box<dyn Error>> {
    // Note: CoinMarketCap needs an API key; using CoinGecko as placeholder
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
    let response = client.get(url).send().await?;
    let price_data = response.json::<CoinGeckoPrice>().await?;
    Ok(price_data.ethereum.usd)
}