use anyhow::{anyhow, Context, Result};
use ethers::{providers::{Http, Middleware, Provider, Ws}, types::BlockNumber};
use std::sync::Arc;
use tokio::{
    sync::Mutex,
    time::{timeout,Duration,Instant},
};
use futures::StreamExt;
use crate::config::BotConfig;



#[derive(Debug)]
pub enum HealthCheckError {
    Http(anyhow::Error),
    Ws(anyhow::Error),
}

impl std::fmt::Display for HealthCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthCheckError::Http(e)=>write!(f, "HTTP Healthcheck faipled: {}",e ),
            HealthCheckError::Ws(e)=> write!(f, "WebSocket Healthcheck failed : {}",e),
        }
    }
}

#[derive(Debug,Clone)]
pub struct Providers {
    pub http: Arc<Provider<Http>>,
    pub ws: Arc<Provider<Ws>>,
    ws_url: String,
    last_ws_check: Arc<Mutex<Instant>>,
}

impl Providers {
    pub async fn new(config: &BotConfig) -> Result<Self> {
        println!("Attempting to connect to HTTP provider....");
        let http = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| anyhow::anyhow!("HTTP connection failed: {}. URL: {}", e, config.rpc_url))?;
        println!("Attempting to connect to WebSocket provider.....");
        let ws_url = config.ws_url.clone();
        let ws = Provider::<Ws>::connect(&ws_url).await
            .map_err(|e|anyhow::anyhow!("Websocket connection failed: {}",e))?;


        println!("Successfully connected to both providers");
        Ok(Self{
            http: Arc::new(http),
            ws: Arc::new(ws),
            ws_url,
            last_ws_check: Arc::new(Mutex::new(Instant::now())),
        }) 
    }

    pub async fn check_health(&self) -> Result<(), HealthCheckError> {
        //Check HTTP connection
        let http_health = self.check_http().await;
        //Check WebSocket connection
        let ws_health= self.check_ws().await;

        match (http_health, ws_health) {
            (Ok(_), Ok(_)) =>Ok(()),
            (Err(e),_) => Err(HealthCheckError::Http(e)),
            (_, Err(e))=> Err(HealthCheckError::Ws(e)),
        }
    }

    pub async fn check_http(&self) -> Result<()> {
            timeout(Duration::from_secs(5),self.http.get_block(BlockNumber::Latest))
                .await
                .context("HTTP timeout")?
                .context("HTTP provide error")?;
            Ok(())
        }

    pub async fn check_ws(&self) -> Result<()>{
        let mut last_check = self.last_ws_check.lock().await;
        if last_check.elapsed()< Duration::from_secs(1){
            return Ok(());
        }


        let pint_result = timeout(
            Duration::from_secs(2),
    self.ws.request::<_, bool>("net_listening",())
            ).await;
        
        match pint_result{
            Ok(Ok(true)) => {
                *last_check = Instant::now();
                Ok(())
            },
            _ =>{
                println!("WebSocket ping failed.....Attempting reconnect....");
                let fresh = Provider::<Ws>::connect(&self.ws_url).await?;
                timeout(
                    Duration::from_secs(2),
            fresh.request::<_,bool>("net listening", ())
                    )
                    .await
                    .context("WS timeout")?
                    .context("Ws post-reconnect check failed")?;

                    *last_check = Instant::now();
                    Ok(())
                }
            }
          
            // let mut sub = self.ws.subscribe_blocks().await?;
            // match timeout(Duration::from_secs(5), sub.next()).await{
            //     Ok(Some(_))=> Ok(()),
            //     Ok(None) => Err(anyhow!("WebSocket subscription ended unexpectedly")),
            //     Err(_) => Err(anyhow!("WebSocket connection timeout")),
            // }
          
        }

}

