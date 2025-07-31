
// src/contracts.rs
use ethers::{
    contract::abigen,
    types::{Address, U256},
};





// Generate type-safe bindings for Uniswap V2 Router
abigen!(
    IUniswapV2Router02,
    r#"[
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
    ]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);

// Generate type-safe bindings for Uniswap V3 Pool
abigen!(
    IUniswapV3Pool,
    r#"[
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)
        function liquidity() external view returns (uint128)
    ]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);


// Uniswap V3 Factory
abigen!(
    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
    ]"#,
);

// Sushiswap Factory
abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#,
);

// Sushiswap Pair
abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    ]"#,
);

// 3. Then modify your imports in test1.rs to include:
use crate::contracts::{
    IUniswapV2Router02, IUniswapV3Pool, IUniswapV3Factory, 
    IUniswapV2Factory, IUniswapV2Pair
};




// Define constants by specifying the 20 bytes directly (no function calls)
const UNISWAP_V3_FACTORY: Address = Address([
    0x1F, 0x98, 0x43, 0x1c, 0x8a, 0xD9, 0x85, 0x23, 0x63, 0x1A,
    0xE4, 0xA5, 0x9f, 0x26, 0x73, 0x46, 0xEA, 0x31, 0xF9, 0x84,
]);

const UNISWAP_V3_ROUTER: Address = Address([
    0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D, 0xE3, 0xEE,
    0xDF, 0x18, 0xE0, 0x15, 0x7C, 0x05, 0x86, 0x15, 0x64,
    0x00, // Missing one byte here, you need exactly 20 bytes!
]);

const SUSHI_FACTORY: Address = Address([
    0xC0, 0xAE, 0xE4, 0x78, 0xE3, 0x65, 0x8E, 0x26, 0x10, 0xC5,
    0xF7, 0xA4, 0xA2, 0xE1, 0x77, 0x7C, 0xE9, 0xE4, 0xF2, 0xAc,
]);

const SUSHI_ROUTER: Address = Address([
    0xd9, 0xE1, 0xCE, 0x17, 0xF2, 0x64, 0x1f, 0x24, 0xaE, 0x83,
    0x63, 0x7a, 0xb6, 0x6a, 0x2c, 0xca, 0x9C, 0x37, 0x8B, 0x9F,
]);

const AAVE_LENDING_POOL: Address = Address([
    0x7d, 0x27, 0x68, 0xDe, 0x32, 0xb0, 0xb8, 0x0b, 0x7a, 0x34,
    0x54, 0xc0, 0x6b, 0xda, 0xc9, 0x4a, 0x69, 0xdd, 0xc7, 0xa9,
]);
