
mod config;
mod scanner; 

use config::ScannerConfig;
use anyhow::Result;
use dotenv::dotenv;

#[tokio::main]
async fn main () -> Result<()>{

    // Initialize logging
    tracing_subscriber::fmt::init();
   
    // Load configuration
    dotenv().ok();
    let config = ScannerConfig::from_env()?;

    Ok(())

}