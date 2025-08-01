use anyhow::Result;
use ethers::{
            providers::Middleware,
            types::Address
};
use std::{collections::HashMap, sync::Arc};

use crate::models::{Pool, Token};

pub struct PriceOracel<M> {
    provider: Arc<M>,
    pools: Vec<Pool>,
    cache: HashMap<(Address, Address), f64>,
}

impl <M: Middleware> PriceOracel<M> {
    pub fn new(provider: Arc<M>, pools: Vec<Pool>) -> Self {
        Self {
            provider,
            pools,
            cache: HashMap::new(),
        }
    }

    pub async fn get_price(&mut self, token_in: &Token, token_out: &Token) -> Result<f64> {
        let key = (token_in.address, token_out.address);

        // Check cache first
        if let Some(price) = self.cache.get(&key) {
            return Ok(*price);
        }

        // Find a pool that contains both tokens
        let pool = self.pools.iter()
            .find(|p| p.contains_token(token_in) && p.contains_token(token_out))
            .ok_or_else(|| anyhow::anyhow!("No pool found for token pair"))?;

        // Simple price calculation (for now)
        let price = if pool.token0.address == token_in.address {
            1.0 // TODO: Implement real price calculation
        } else {
            2.0 // TODO: Implement real price calculation
        };

        // Cache the price
        self.cache.insert(key,price);
        Ok(price)
    }

    pub async fn print_latest_block(&self) -> Result<()>
    where 
        <M as Middleware>::Error: 'static,
    {
    let block_number = self.provider.get_block_number().await?;
    println!("Latest block number: {}", block_number);
    Ok(())
}

}