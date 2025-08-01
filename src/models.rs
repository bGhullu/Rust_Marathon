use ethers::{
    types::{Address, U256},
    abi::ethereum_types::BigEndianHash,
};
use std::{
    fmt,
    time::Instant,
};



#[derive(Debug, Clone)]
pub struct Token {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,

}

impl Token{
    pub fn new(address: Address, symbol: &str, decimals: u8) -> Self {
        Self {
            address,
            symbol: symbol.to_string(),
            decimals,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.symbol, self.address)
    }
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub fee: u32,
    pub reserve0: U256,
    pub reserve1: U256,
    pub last_update: Instant,
}

impl Pool {
    pub fn new(address: Address, token0: Token, token1: Token, fee: u32) -> Self {
        Self {
            address,
            token0,
            token1,
            fee,
            reserve0: U256::zero(),
            reserve1: U256::zero(),
            last_update: Instant::now(),
        }
    }

    pub fn update_reserve(&mut self, reserve0: U256, reserve1: U256) {
        self.reserve0 = reserve0;
        self.reserve1 = reserve1;
        self.last_update = Instant::now();
    }

    pub fn calculate_price(&self, token_in: &Token) -> Option<f64> {

        if token_in.address == self.token0.address {
            Some(self.reserve1.as_u128() as f64 / self.reserve0.as_u128() as f64)
        } else if token_in.address == self.token1.address {
            Some(self.reserve0.as_u128() as f64 / self.reserve1.as_u128() as f64)
        } else {
            None
        }
    }
    pub fn contains_token(&self, token: &Token) -> bool {
        self.token0.address == token.address || self.token1.address == token.address
    }

    pub fn get_other_token(&self, token: &Token) -> Option<&Token> {
        if token.address == self.token0.address{
            Some(&self.token1)
        } else if token.address == self.token1.address{
            Some(&self.token0)
        } else {
            None
        }
    }
}

