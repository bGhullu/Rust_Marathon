
mod config;
mod macros; 
mod const_and_addr;


use config::ScannerConfig;
use anyhow::Result;
use dotenv::dotenv;

use crate::scanner::MevScanner;
use crate::storage::StorageDriftDetector;
use crate::storage::SimpleStateCache;

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