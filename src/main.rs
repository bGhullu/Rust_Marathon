use ethers::{ 
    providers::{Http, Middleware, Provider, Ws}, 
    types:: {
        NameOrAddress,
        BlockNumber, 
        Transaction, 
        TransactionReceipt,
        TransactionRequest, 
        H160, U256,
        transaction::eip2718::TypedTransaction
    }, 
    utils::format_units
};
use std::{
    collections::HashMap,
    sync::{
    atomic::{AtomicBool, Ordering},
    Arc
    }
};
use anyhow::{anyhow,Result, Context};
use futures::StreamExt;
use tokio::time::Duration;



#[derive(Clone)]
/// Professional Ethereum client with failover
struct EthClient{
    http: Arc<Provider<Http>>,  
    ws: Arc<Provider<Ws>>,
    current_block: u64,
    http_alive: Arc<AtomicBool>,
}

#[derive(Debug)]
enum MevOpportunity{
    Sandwich{
        frontrun_tx: Transaction,
        victim_tx: Transaction,
        backrun_tx: Transaction,
        profit_estimate: U256,
    },
    Arbitrage{
        path: Vec<H160>, // Pool address
        profit: U256,
    },
    Liquidation{
        liquidator: H160,
        debt_position: H160,
        profit: U256,
    }

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
            http_alive: Arc::new(AtomicBool::new(true)),
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

    pub async fn get_transaction(&self, tx_hash: ethers::types::TxHash) -> Result<Option<Transaction>>{
        if self.http_alive.load(Ordering::SeqCst){
            match self.http.get_transaction(tx_hash).await{
                Ok(tx)=> Ok(tx),
                Err(e)=>{
                    eprintln!("HTTP failed, falling back to WS; {}", e);
                    self.ws.get_transaction(tx_hash).await
                        .context("Failed to get transaction via WS")
                }
            } 
        } else {
            self.ws.get_transaction(tx_hash).await
                .context("Failed to get transaction via WS")
        }
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
                match self.get_transaction(tx_hash).await{
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

    pub async fn detect_mev(&self) -> Result<Vec<MevOpportunity>>{
        let mut opportunities = Vec::new();
        let mut pending_txs = HashMap::new();

        let mut stream= self.ws.subscribe_pending_txs().await?;
        while let Some(tx_hash) = stream.next().await{
            if let Ok(Some(tx)) = self.get_transaction(tx_hash).await{
                //1. Detect sandwich attacks
                if let Some(opp) = self.detect_sandwich(&tx, &pending_txs).await
                {
                    opportunities.push(opp);
                }
                // 2. Track transactions in mempool
            pending_txs.insert(tx.hash,tx);
            }
        }
        Ok(opportunities)

    }

    async fn detect_sandwich(
        &self,
        new_tx: &Transaction,
        mempool: &HashMap<ethers::types::TxHash, Transaction>
    ) -> Option<MevOpportunity>{
        //Implement sandwich detecion logic
        None // Placeholder
    }

    pub async fn get_optimal_gas_price(&self) -> Result<U256> {
        let fee_history = self.ws.fee_history(5,BlockNumber::Latest,&[50.0])
            .await
            .context("Failed to get fee history")?;
        let base_fee = *fee_history.base_fee_per_gas.first()
            .ok_or_else(|| anyhow::anyhow!("No base fee!"))?;
        let max_priority = U256::from(2_000_000_000); //2 Gwei tip
        let max_fee = base_fee
            .checked_mul(U256::from(125))
            .and_then(|v| v.checked_div(U256::from(100)))
            .ok_or_else( || anyhow!("Gas price calculation overflow"))?;//25% over base

        Ok(max_fee + max_priority)
    }

    pub async fn simulate_tx_request(
        &self,
        tx_request: &TransactionRequest
    ) -> Result<(bool, U256)>{
        let typed_tx: TypedTransaction = tx_request.clone().into();
        // let tx_hash = typed_tx.hash();
         
         let mock_tx = Transaction {
            hash: typed_tx.sighash(),
            nonce: typed_tx.nonce().cloned().unwrap_or_default(),
            from: typed_tx.from().map(|f| *f).unwrap_or_default(),
            to: typed_tx.to().and_then(|addr| match addr {
                NameOrAddress::Address(a) => Some (*a),
                NameOrAddress::Name(_) => None,
            }),
            value: typed_tx.value().cloned().unwrap_or_default(),
            gas: typed_tx.gas().cloned().unwrap_or_default(),
            gas_price: typed_tx.gas_price().map(|gp| gp.clone()),
            input: typed_tx.data().cloned().unwrap_or_default(),
            ..Default::default()
         };

         self.simulate_tx(&mock_tx).await  
        
    }

    pub async fn simulate_tx(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, U256)> {
        let result = self.ws.call(&tx.into(), None).await?;
        let receipt = self.ws.get_transaction_receipt(tx.hash)
            .await?
            .ok_or_else(|| anyhow!("Transaction receipt not found!"))?;

    
        let gas_used = receipt.gas_used
            .ok_or_else(|| anyhow!("No gas used not available"))?;
        let success = !result.is_empty();
        Ok((success, U256::from(gas_used)))
    }


}


#[tokio::main]

async fn main()-> Result<()>{
    let rpc_url = "https://eth.llamarpc.com"; 
    let ws_url = "wss://eth.llamarpc.com";

    let mut client = EthClient::new(rpc_url, ws_url).await?;

 

    match client.get_optimal_gas_price().await{
        Ok(gas_price)=> println!("Optimal gas price: {} Gwei", format_units(gas_price, "gwei")?),
        Err(e)=> eprintln!("Error getting gas price: {}", e),
    }

    let sample_tx = TransactionRequest::new()
        .to("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse::<H160>()?)
        .value(100_000_000_000_000_000_u64);

    let typed_tx: TypedTransaction = sample_tx.clone().into();

    let  gas_estimate = client.http.estimate_gas(&typed_tx,None).await?;
    let sample_tx = sample_tx
        .gas(gas_estimate)
        .gas_price(client.get_optimal_gas_price().await?);
    let (success,gas) = client.simulate_tx_request(&sample_tx).await?;
    println!("Simulation result: success = {}, gas = {}", success, gas);
    println!("Current block: {}", client.get_block().await?);
    
    // Simulate HTTP failure
    // println!("Simulate HTTP failure...");
    // client.kill_http();
   
    // std::thread::sleep(std::time::Duration::from_secs(10));

    println!("Fallback block: {}", client.get_block().await?);

    let client_clone= client.clone();


    tokio::spawn(async move {
        client_clone.stream_blocks().await.unwrap();
    });
    
    loop {
        match client.stream_pending_txs().await {
            Ok(_) => break,
            Err(e) => {
                eprintln!("Stream failed, restarting: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    // client.stream_pending_txs().await?;



    Ok(())
}

