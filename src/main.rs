mod config;

use config::ScannerConfig;
use anyhow::Result;
use dotenv::dotenv;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main () -> Result<()>{

    // Initialize logging
    tracing_subscriber::fmt::init();
   
    // Load configuration
    dotenv().ok();
    let config = BotConfig::from_env()?;



    Ok(())

}