use ethers::{
    contract::abigen,
    types::{Address, U256},
};
use once_cell::sync::Lazy;
use crate::models::{Pool,Token};


pub static WETH: Lazy<Token> = Lazy::new( || {
    Token::new(
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse()
            .expect("valid WETH address"),
        "WETH",
        18,
    )
});

pub static USDC: Lazy<Token> = Lazy::new( || {
    Token::new(
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        .parse()
        .expect("valid USDC address"),
    "USDC",
    6,
)
});


pub static POOLS: Lazy<Vec<Pool>> = Lazy::new( || {
    vec![
        Pool::new(
            "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
            .parse()
            .expect("valid pool address"),
            WETH.clone(),
            USDC.clone(),
            500,
        ),
    ]
});

// Protocol Addresses
pub static UNISWAP_V3_FACTORY: Lazy<Address> = Lazy::new(|| {
    "0x1F98431c8aD98523631AE4a59f267346ea31F984"
        .parse()
        .unwrap()
});

pub static UNISWAP_V3_ROUTER: Lazy<Address> = Lazy::new(|| {
    "0xE592427A0AEce92De3Edee1F18E0157C05861564"
        .parse()
        .unwrap()
});

pub static SUSHI_FACTORY: Lazy<Address> = Lazy::new(|| {
    "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"
        .parse()
        .unwrap()
});

pub static SUSHI_ROUTER: Lazy<Address> = Lazy::new(|| {
    "0xd9e1cE17f6638c9A13a9a05dE046D742f52C256b"
        .parse()
        .unwrap()
});

pub static AAVE_LENDING_POOL: Lazy<Address> = Lazy::new(|| {
    "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9"
        .parse()
        .unwrap()
});

// Common Token Addresses
pub static WETH: Lazy<Address> = Lazy::new(|| {
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        .parse()
        .unwrap()
});

pub static USDC: Lazy<Address> = Lazy::new(|| {
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        .parse()
        .unwrap()
});

// Generate contract bindings
abigen!(
    IUniswapV2Router02,
    r#"[
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
    ]"#
);

abigen!(
    IUniswapV3Pool,
    r#"[
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)
        function liquidity() external view returns (uint128)
    ]"#
);

abigen!(
    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
    ]"#
);

abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#
);

abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    ]"#
);

abigen!(
    ISushiRouter,
    r#"[
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
    ]"#
);