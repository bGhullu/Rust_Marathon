
use tokio::sync::Mutex;
use std::{
    sync::atomic::{AtomicUsize,AtomicBool,Ordering},
    time::{Duration,Instant},
};

pub struct CircuitBreaker {
    error_count: AtomicUsize,
    circuit_breaker_threshold: usize,
    cool_down: Duration,
    last_tripped: Mutex<Option<Instant>>,
    auto_reset: AtomicBool,
}

impl CircuitBreaker {
    pub fn new(max_errors: usize, cool_down: Duration, auto_reset: bool) -> Self {
        Self {
            error_count: AtomicUsize::new(0),
            circuit_breaker_threshold: max_errors,
            cool_down,
            last_tripped: Mutex::new(None),
            auto_reset: AtomicBool::new(auto_reset),


        }
    }

    pub async fn is_tripped(&self) -> bool {
        let count = self.error_count.load(Ordering::Relaxed);
        if count < self.circuit_breaker_threshold {
            return false;
        }

        let guard = self.last_tripped.lock().await;
        match *guard {
            Some(last) if last.elapsed() < self.cool_down => true,
            Some (_) if self.auto_reset.load(Ordering::Relaxed) => {
                drop(guard); // ---------------------
                self.reset().await;
                false
            },
            Some(_) => true,
            None => false,
        }
    }

    pub async fn trip(&self) {
        let new_count = self.error_count.fetch_add(1, Ordering::Relaxed) +1;
        if new_count >= self.circuit_breaker_threshold{
            let mut guard = self.last_tripped.lock().await;
            *guard = Some(Instant::now());
        }
    }

    pub async fn reset(&self) {
        self.error_count.store(0,Ordering::Relaxed);
        let mut guard = self.last_tripped.lock().await;
        *guard = None;
    }

    pub fn error_count(&self) -> usize {
        self.error_count.load(Ordering::Relaxed)
    }
}
