use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::storage::buffer::BufferReceiver;
use crate::storage::StorageBackend;

pub struct StorageDispatcher {
    backends: Vec<Arc<dyn StorageBackend>>,
    receiver: BufferReceiver,
    batch_size: usize,
    flush_interval: Duration,
}

impl StorageDispatcher {
    pub fn new(
        backends: Vec<Arc<dyn StorageBackend>>,
        receiver: BufferReceiver,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        Self {
            backends,
            receiver,
            batch_size,
            flush_interval,
        }
    }

    pub async fn run(&mut self) {
        let mut interval = time::interval(self.flush_interval);
        loop {
            tokio::select! {
                batch = self.receiver.recv_batch(self.batch_size) => {
                    if batch.is_empty() {
                        break;
                    }
                    self.dispatch_batch(&batch).await;
                }
                _ = interval.tick() => {
                    for backend in &self.backends {
                        let _ = backend.flush().await;
                    }
                }
            }
        }
    }

    async fn dispatch_batch(
        &self,
        batch: &[crate::capture::exchange::CapturedExchange],
    ) {
        let futures: Vec<_> = self
            .backends
            .iter()
            .map(|backend| {
                let backend = Arc::clone(backend);
                let batch = batch.to_vec();
                tokio::spawn(async move {
                    if let Err(e) = backend.write_batch(&batch).await {
                        tracing::error!("Storage write error: {}", e);
                    }
                })
            })
            .collect();

        for f in futures {
            let _ = f.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::exchange::{CapturedExchange, CapturedRequest};
    use crate::error::StorageError;
    use crate::storage::buffer::CaptureBuffer;
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, Version};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    fn make_exchange() -> CapturedExchange {
        CapturedExchange {
            request: CapturedRequest {
                id: Uuid::now_v7(),
                session_id: Uuid::now_v7(),
                connection_id: Uuid::now_v7(),
                sequence_number: 1,
                timestamp: Utc::now(),
                method: Method::GET,
                uri: "https://example.com/".parse().unwrap(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::new(),
                body_truncated: false,
                tls_info: None,
            },
            response: None,
        }
    }

    #[derive(Debug)]
    struct CountingBackend {
        write_count: AtomicUsize,
    }

    impl CountingBackend {
        fn new() -> Self {
            Self {
                write_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl StorageBackend for CountingBackend {
        async fn initialize(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn write(&self, _: &CapturedExchange) -> Result<(), StorageError> {
            self.write_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn write_batch(&self, batch: &[CapturedExchange]) -> Result<(), StorageError> {
            self.write_count.fetch_add(batch.len(), Ordering::SeqCst);
            Ok(())
        }
        async fn flush(&self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn close(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingBackend;

    #[async_trait]
    impl StorageBackend for FailingBackend {
        async fn initialize(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn write(&self, _: &CapturedExchange) -> Result<(), StorageError> {
            Err(StorageError::WriteFailed("simulated".into()))
        }
        async fn write_batch(&self, _: &[CapturedExchange]) -> Result<(), StorageError> {
            Err(StorageError::WriteFailed("simulated".into()))
        }
        async fn flush(&self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn close(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn dispatch_to_multiple_backends() {
        let backend1 = Arc::new(CountingBackend::new());
        let backend2 = Arc::new(CountingBackend::new());
        let (tx, rx) = CaptureBuffer::new(100);
        let mut dispatcher = StorageDispatcher::new(
            vec![backend1.clone(), backend2.clone()],
            rx,
            10,
            Duration::from_secs(60),
        );
        for _ in 0..3 {
            tx.send(make_exchange()).await.unwrap();
        }
        drop(tx);
        dispatcher.run().await;
        assert_eq!(backend1.write_count.load(Ordering::SeqCst), 3);
        assert_eq!(backend2.write_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn failing_backend_does_not_affect_others() {
        let good = Arc::new(CountingBackend::new());
        let bad: Arc<dyn StorageBackend> = Arc::new(FailingBackend);
        let (tx, rx) = CaptureBuffer::new(100);
        let mut dispatcher = StorageDispatcher::new(
            vec![good.clone(), bad],
            rx,
            10,
            Duration::from_secs(60),
        );
        tx.send(make_exchange()).await.unwrap();
        drop(tx);
        dispatcher.run().await;
        assert_eq!(good.write_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn empty_on_sender_drop() {
        let backend = Arc::new(CountingBackend::new());
        let (tx, rx) = CaptureBuffer::new(100);
        drop(tx);
        let mut dispatcher = StorageDispatcher::new(
            vec![backend.clone()],
            rx,
            10,
            Duration::from_secs(60),
        );
        dispatcher.run().await;
        assert_eq!(backend.write_count.load(Ordering::SeqCst), 0);
    }
}
