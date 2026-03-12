use tokio::sync::mpsc;

use crate::capture::exchange::CapturedExchange;

pub struct CaptureBuffer;

impl CaptureBuffer {
    pub fn new(capacity: usize) -> (BufferSender, BufferReceiver) {
        let (tx, rx) = mpsc::channel(capacity);
        (BufferSender { tx }, BufferReceiver { rx })
    }
}

#[derive(Clone)]
pub struct BufferSender {
    tx: mpsc::Sender<CapturedExchange>,
}

impl BufferSender {
    pub async fn send(&self, exchange: CapturedExchange) -> Result<(), CapturedExchange> {
        self.tx.send(exchange).await.map_err(|e| e.0)
    }

    pub fn try_send(&self, exchange: CapturedExchange) -> Result<(), CapturedExchange> {
        self.tx.try_send(exchange).map_err(|e| match e {
            mpsc::error::TrySendError::Full(ex) => ex,
            mpsc::error::TrySendError::Closed(ex) => ex,
        })
    }

    pub fn into_inner(self) -> mpsc::Sender<CapturedExchange> {
        self.tx
    }
}

pub struct BufferReceiver {
    rx: mpsc::Receiver<CapturedExchange>,
}

impl BufferReceiver {
    pub async fn recv_batch(&mut self, max_size: usize) -> Vec<CapturedExchange> {
        let mut batch = Vec::with_capacity(max_size);
        if let Some(ex) = self.rx.recv().await {
            batch.push(ex);
        }
        while batch.len() < max_size {
            match self.rx.try_recv() {
                Ok(ex) => batch.push(ex),
                Err(_) => break,
            }
        }
        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::exchange::{CapturedExchange, CapturedRequest};
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, Version};
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

    #[tokio::test]
    async fn send_and_recv() {
        let (tx, mut rx) = CaptureBuffer::new(10);
        tx.send(make_exchange()).await.unwrap();
        let batch = rx.recv_batch(10).await;
        assert_eq!(batch.len(), 1);
    }

    #[tokio::test]
    async fn recv_batch_up_to_max() {
        let (tx, mut rx) = CaptureBuffer::new(10);
        for _ in 0..5 {
            tx.send(make_exchange()).await.unwrap();
        }
        tokio::task::yield_now().await;
        let batch = rx.recv_batch(3).await;
        assert_eq!(batch.len(), 3);
    }

    #[tokio::test]
    async fn try_send_full_buffer() {
        let (tx, _rx) = CaptureBuffer::new(1);
        tx.send(make_exchange()).await.unwrap();
        let result = tx.try_send(make_exchange());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sender_drop_returns_empty() {
        let (tx, mut rx) = CaptureBuffer::new(10);
        drop(tx);
        let batch = rx.recv_batch(10).await;
        assert!(batch.is_empty());
    }
}
