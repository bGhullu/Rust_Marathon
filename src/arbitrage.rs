use anyhow::Result;


use crate::{
    models::{Pool, Token},
    oracle::PriceOracel,
};

pub struct ArbitrageDetector {
    min_profit_threshold: f64,
}

impl ArbitrageDetector{
    pub fn new(min_profit_threshold: f64) -> Self {
        Self {
            min_profit_threshold,
        }
    }

    pub async fn find_arbitrage(
        &self,
        oracle: &mut PriceOracel<impl ethers::providers::Middleware>,
        token_a: &Token,
        token_b: &Token,
    ) -> Result<Option<f64>> {
       
    }
}
