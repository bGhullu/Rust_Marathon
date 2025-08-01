use ethers::types::U256;
use anyhow::{anyhow,Context,Result};
use std::str::FromStr;


#[derive(Debug, Clone)]
pub struct BotConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub max_trade_size: U256,
    pub min_profit_threshold: f64,
    pub max_slippage: f64,
    pub private_key: String,
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

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenv::dotenv();
        let rpc_url= std::env::var("RPC_URL")
            .context("Missing RPC_URL in environmnet")?;
        let ws_url = std::env::var("WS_URL")
            .context("Missing WS_URL in enviornment")?;
        let private_key = std::env::var("PRIVATE_KEY")
            .context("Missing PRIVATE_KEY in environment")?;
        
        if !rpc_url.starts_with("http"){
            return Err(anyhow!("RPC_URL must start with http://or https://"));
        }
        if !ws_url.starts_with("ws"){
            return Err(anyhow!("WS_URL must start with ws:// or wss://"));
        }
        if private_key.is_empty(){
            return Err(anyhow!("PRIVATE_KEY cannot be empty"));
        }

        Ok(Self { 
            rpc_url, 
            ws_url,
            max_trade_size: std::env::var("MAX_TRADE_SIZE")
                .map(|s| U256::from_dec_str(&s).unwrap_or(U256::from(10).pow(U256::from(18u64))))
                .unwrap_or(U256::from(10).pow(U256::from(18u64))),
            min_profit_threshold: prase_env_var("MIN_PROFIT_THRESHOLD", 0.001),
            max_slippage: prase_env_var("MAX_SLIPPAGE", 0.005),
            private_key,
        })
    }
}

