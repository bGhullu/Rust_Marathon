
mod config;
mod scanner;
mod macros; 
mod const_addr;

use config::ScannerConfig;
use anyhow::Result;
use dotenv::dotenv;

use crate::scanner::MevScanner;

#[tokio::main]
async fn main () -> Result<()>{

    // Initialize logging
    tracing_subscriber::fmt::init();
   
    // Load configuration
    dotenv().ok();
    let config = ScannerConfig::from_env()?;
    let scanner = MevScanner::new(config).await?;
    Ok(())

}