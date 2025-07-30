# Ethereum MEV Detection Client

[![Rust](https://img.shields.io/badge/Rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![Ethers-rs](https://img.shields.io/badge/Ethers.rs-2.0+-orange.svg)](https://github.com/gakonst/ethers-rs)

A high-performance Ethereum client specializing in MEV opportunity detection with multi-strategy support.

## 🔍 MEV Detection Strategies

### 1. Arbitrage Detection
- Identifies price discrepancies across DEX pools
- Supports Uniswap, Sushiswap, and Curve
- Profitability simulation with gas costs

### 2. Liquidation Monitoring
- Tracks undercollateralized positions
- Supports Aave, Compound, and MakerDAO
- Calculates liquidation profitability

### 3. Frontrun Protection
- Detects predatory pending transactions
- Gas price spike monitoring
- Transaction replacement strategies

### 4. Bundle Profitability
- Simulates transaction bundle outcomes
- Gas cost vs reward analysis
- Failed bundle detection

## 🛠️ Features

- **Multi-Provider Connectivity**
  ```rust
  // Dual provider initialization
  EthClient::new(http_url, ws_url)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/ethereum-mev-client.git
   cd ethereum-mev-client

2. Set up environment variables:
    cp .env.example .env
    # Edit with your Infura/Alchemy URLs

3. Build and run:
    cargo build --release
    cargo run -- --help
    

## 🌐 System Architecture

graph TD
    A[Ethereum Node] -->|JSON-RPC| B[EthClient]
    B --> C[MEV Detector Engine]
    C --> D[Arbitrage Scanner]
    C --> E[Liquidation Monitor]
    C --> F[Frontrun Analyzer]
    B --> G[Transaction Simulator]
    B --> H[Gas Optimizer]
    D --> I[Profit Calculator]
    E --> I
    F --> I
    I --> J[Opportunity Queue]


### Key Diagram Features:

1. **Clear Data Flow**: Shows how node data passes through detection pipelines
2. **Modular Design**: Highlights separable components
3. **Visual Metrics**: Pie chart shows performance characteristics
4. **GitHub Compatible**: Renders natively in GitHub/GitLab Markdown

### Alternate Text-Only Version
If you prefer no diagrams, replace the Mermaid sections with:

```markdown
## Text Architecture Overview

1. Ethereum Node (JSON-RPC)
   └─> EthClient (HTTP/WS)
       ├─> MEV Detector
       │   ├─> Arbitrage Scanner
       │   ├─> Liquidation Monitor
       │   └─> Frontrun Analyzer
       ├─> Transaction Simulator
       └─> Gas Optimizer



License

MIT License - see LICENSE for details