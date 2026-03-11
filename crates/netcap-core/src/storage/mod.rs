use async_trait::async_trait;

use crate::capture::exchange::CapturedExchange;
use crate::error::StorageError;

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn initialize(&mut self) -> Result<(), StorageError>;
    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError>;
    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError>;
    async fn flush(&self) -> Result<(), StorageError>;
    async fn close(&mut self) -> Result<(), StorageError>;
}

pub struct FanoutWriter {
    backends: Vec<Box<dyn StorageBackend>>,
}

impl FanoutWriter {
    pub fn new(backends: Vec<Box<dyn StorageBackend>>) -> Self {
        Self { backends }
    }

    pub async fn write_all(&self, exchange: &CapturedExchange) -> Vec<Result<(), StorageError>> {
        let mut results = Vec::new();
        for backend in &self.backends {
            results.push(backend.write(exchange).await);
        }
        results
    }

    pub async fn flush_all(&self) -> Vec<Result<(), StorageError>> {
        let mut results = Vec::new();
        for backend in &self.backends {
            results.push(backend.flush().await);
        }
        results
    }
}
