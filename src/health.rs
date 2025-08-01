use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

pub struct HealthCheck {
    last_ok: Arc<RwLock<Instant>>,
    max_downtime: Duration,
}

impl HealthCheck {
    pub fn new(max_downtime: Duration) -> Self {
        Self{
            last_ok: Arc::new(RwLock::new(Instant::now())),
            max_downtime,
        }
    }


    pub async fn update(&self) {
        *self.last_ok.write().await = Instant::now();
    }

    pub async fn check(&self) -> Result<()>{
        let last = *self.last_ok.read().await;
        if Instant::now().duration_since(last) > self.max_downtime{
            anyhow::bail!("Health check failed -no updates in {:?}", self.max_downtime);
        }
        Ok(())
    }

}