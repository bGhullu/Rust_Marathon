mod config;
mod providers;
mod health;
mod models;
mod oracle;
mod arbitrage;
mod contracts;

use config::BotConfig;
use providers::Providers;
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

    // Setup providers
    let providers =Arc::new(Providers::new(&config).await
        .map_err(|e| anyhow::anyhow!("Provider Configuration error: {}",e))?);

    let mut oracle =oracle::PriceOracel::new(providers.ws.clone(),contracts::POOLS.clone());
    let detector =arbitrage::ArbitrageDetector::new(config.min_profit_threshold);



    // Setup health check
    let health = Arc::new(health::HealthCheck::new(Duration::from_secs(10)));

    // Normal execution or health check
    // if std::env::args().any(|arg| arg == "--health-check") {
    //     health_simulate::simulate_health_check().await;
    // } 

    // Start health monitor 
    let health_monitor = tokio::spawn({
        let health = health.clone();
        async move{
            loop {
                if let Err(e) = health.check().await{
                    tracing::error!("Health check failed: {}",e);
             
                }
                tracing::info!("010100101010101010101010101010010101010101010101010101001010101010101010101");
                sleep(Duration::from_secs(1)).await;
            }
        }
    });

    // Check for arbitrage opportunities
    match detector.find_arbitrage(&mut oracle,&contracts::WETH, &contracts::USDC).await {
        Ok(Some(profit))=>{
            tracing::info!("Found arbitrage opportunity with profit: {:.2}%", profit *100.0);
        }
        Ok(None)=> {
            tracing::info!("No opportunity found so far.....");
        }
        Err(e)=> {
            tracing::error!("Error finding arbitrage: {}",e);
        }
    }

    oracle.print_latest_block().await?;

    // Main loop
    loop {
        // Update health status
        health.update().await;

        // Check provider health
        if let Err(e) = providers.check_health().await{
            tracing::error!("Provider health check failed: {}",e);
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        // TODO: Impletment arbitrage logic here
        tracing::info!("✅ Health check passed. Waiting for arbitrage logic...");
        sleep(Duration::from_millis(100)).await;
    }

    #[allow(unreachable_code)]
    health_monitor.await?;
    Ok(())
}