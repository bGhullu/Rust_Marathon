use ethers::providers::{Http, Middleware, Provider, Ws};
use std::sync::Arc;
use anyhow::{Result, Context};
use ethers::types::{Block, Transaction};
use futures::StreamExt;
use tokio::sync::broadcast;


#[derive(Clone)]
/// Professional Ethereum client with failover
struct EthClient{
    http: Arc<Provider<Http>>,  
    ws: Arc<Provider<Ws>>,
    current_block: u64,
}

impl EthClient{
   /// Initialize with automatic health checks 
    pub async fn new(rpc_url: &str,ws_url: &str) -> Result<Self>{
        // 1. Initialize providers
        let http = Provider::<Http>::try_from(rpc_url)
            .context("Failed to create HTTP provide")?; 
        let ws = Provider::<Ws>::connect(ws_url)
            .await
            .context("Failed to connect to WS provider")?;
        
        // 2. Verify chain synchronization
        let http_block = http
            .get_block_number()
            .await
            .context("HTTP block fetch failed")?
            .as_u64();
        let ws_block = ws
            .get_block_number()
            .await
            .context("WS block fetch failed")?
            .as_u64();

        if http_block.abs_diff(ws_block) > 3{
            anyhow::bail!("Providers out of sync (HTTP: {}, ws:{})",http_block,ws_block);
        }

        // 3. Return initialized client
        Ok(Self{
            http: Arc::new(http),
            ws: Arc::new(ws),
            current_block: http_block,
        })

    }
    /// Get latest block with automatic retry
    pub async fn get_block(&mut self) -> Result<u64>{
        // Try HTTP first
        match self.http.get_block_number().await{
            Ok(block) => {
                self.current_block = block.as_u64();
                Ok(self.current_block)
            },
            Err(e) => {
                // Fallback to websocket
                eprintln!("HTTP failed, falling back to WS: {}", e);
                let block = self.ws.get_block_number()
                    .await?
                    .as_u64();
                self.current_block = block;
                Ok(block)
            }
        }
    }
    /// Simulate HTTP provide failure 
    pub fn kill_http(&mut self){
        // Create new empty Arc to replace the existing one 
        self.http = Arc::new(Provider::<Http>::try_from("http://invalid.url").unwrap());
    }

    pub async fn stream_blocks(&self) ->Result<()>{
        let mut stream = self.ws.subscribe_blocks().await?;
        while let Some(block) = stream.next().await{
            println!(
                "New Block #{}: {} txs, {} gas used",
                block.number.unwrap(),
                block.transactions.len(),
                block.gas_used
            );
        }
        Ok(())
    }

    pub async fn stream_pending_txs(&self)->Result<()>{
        let mut stream = self.ws.subscribe_pending_txs().await?;
        while let Some(tx_hash) = stream.next().await{
                match self.ws.get_transaction(tx_hash).await{
                    Ok(Some(tx))=>{
                        println!(
                            "Pending Tx: {} => {}, gas:{}, value:{} ETH",
                            tx.from,
                            tx.to.unwrap_or_default(),
                            tx.gas,
                            ethers::utils::format_units(tx.value,"ether")?
                        );
                    }
                    Ok(None)=> println!("Transaction disappeared from mempool"),
                    Err(e)=> println!("Error fetching transaction: {}", e),
                }
            }
            Ok(())    
    }
    
}

#[tokio::main]

async fn main()-> Result<()>{
    let rpc_url = "https://eth.llamarpc.com"; 
    let ws_url = "wss://eth.llamarpc.com";

    let mut client = EthClient::new(rpc_url, ws_url).await?;
    println!("Current block: {}", client.get_block().await?);
    
    // Simulate HTTP failure
    println!("Simulate HTTP failure...");
    client.kill_http();
   
    // std::thread::sleep(std::time::Duration::from_secs(10));

    println!("Fallback block: {}", client.get_block().await?);

    let client_clone= client.clone();


    tokio::spawn(async move {
        client_clone.stream_blocks().await.unwrap();
    });
    client.stream_pending_txs().await?;

    Ok(())
}

