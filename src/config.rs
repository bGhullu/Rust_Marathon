use ethers::types::U256;
use anyhow::{anyhow,Context,Result};
use std::{
    time::Duration,
    str::FromStr,
};


const COOL_DOWN_PERIOD: Duration = Duration::from_secs(60);
const CIRCUIT_BREAKER_THRESHOLD : usize = 5;

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    primary_rpc_url: String,
    fallback_rpc_url: String,
    max_trade_size: U256,
    min_profit_threshold: f64,
    max_slippage: f64,
    private_key: String,
    circuit_break_threshold: usize,
    circuit_breaker_cooldown_seconds: Duration,
}



fn prase_env_var<T>(key: &str, default: T) -> T 
where 
    T: FromStr,
{
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)

}

impl ScannerConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenv::dotenv();
        let primary_rpc_url = std::env::var("WS_URL")
            .context("Missing WS_URL in enviornment")?;
        let fallback_rpc_url= std::env::var("HTTP_URL")
            .context("Missing RPC_URL in environmnet")?;
        let private_key = std::env::var("PRIVATE_KEY")
            .context("Missing PRIVATE_KEY in environment")?;
        
        if !primary_rpc_url.starts_with("ws"){
            return Err(anyhow!("WS_URL must start with ws:// or wss://"));
        }
        if !fallback_rpc_url.starts_with("http"){
            return Err(anyhow!("RPC_URL must start with http://or https://"));
        }
        if private_key.is_empty(){
            return Err(anyhow!("PRIVATE_KEY cannot be empty"));
        }

        Ok(Self { 
           primary_rpc_url,
           fallback_rpc_url,
            max_trade_size: std::env::var("MAX_TRADE_SIZE")
                .map(|s| U256::from_dec_str(&s).unwrap_or(U256::from(10).pow(U256::from(18u64))))
                .unwrap_or(U256::from(10).pow(U256::from(18u64))),
            min_profit_threshold: prase_env_var("MIN_PROFIT_THRESHOLD", 0.001),
            max_slippage: prase_env_var("MAX_SLIPPAGE", 0.005),
            circuit_breaker_cooldown_seconds: COOL_DOWN_PERIOD,
            circuit_break_threshold: CIRCUIT_BREAKER_THRESHOLD,
            private_key,

        })
    }
}

