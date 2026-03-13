pub mod batch;
pub mod schema;

use async_trait::async_trait;
use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;
use std::path::PathBuf;
use std::sync::Mutex;

use schema::exchange_to_row;

#[derive(Debug, Clone)]
pub struct BigQueryStorageConfig {
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
    pub fallback_jsonl_path: Option<PathBuf>,
}

pub struct BigQueryStorage {
    config: BigQueryStorageConfig,
    pending_rows: Mutex<Vec<schema::BigQueryRow>>,
    fallback_writer: Mutex<Option<std::fs::File>>,
}

impl std::fmt::Debug for BigQueryStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BigQueryStorage")
            .field("config", &self.config)
            .finish()
    }
}

impl BigQueryStorage {
    pub fn new(config: BigQueryStorageConfig) -> Result<Self, StorageError> {
        let fallback_writer = if let Some(ref path) = config.fallback_jsonl_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| StorageError::InitFailed(e.to_string()))?;
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| StorageError::InitFailed(e.to_string()))?;
            Some(file)
        } else {
            None
        };

        Ok(Self {
            config,
            pending_rows: Mutex::new(Vec::new()),
            fallback_writer: Mutex::new(fallback_writer),
        })
    }

    fn write_to_fallback(&self, rows: &[schema::BigQueryRow]) -> Result<(), StorageError> {
        use std::io::Write;
        let mut guard = self
            .fallback_writer
            .lock()
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        if let Some(ref mut file) = *guard {
            for row in rows {
                let json = serde_json::to_string(row)
                    .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
                writeln!(file, "{}", json)
                    .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
            }
            file.flush()
                .map_err(|e| StorageError::FlushFailed(e.to_string()))?;
            tracing::warn!(
                "Wrote {} rows to JSONL fallback (BigQuery unavailable)",
                rows.len()
            );
            Ok(())
        } else {
            Err(StorageError::WriteFailed(
                "No fallback JSONL path configured".into(),
            ))
        }
    }

    async fn send_to_bigquery(
        &self,
        rows: &[schema::BigQueryRow],
    ) -> Result<(), StorageError> {
        // In production, this would use gcp-bigquery-client to send rows.
        // For now, we store the rows and attempt retry logic.
        // This will be connected to actual BigQuery API when credentials are available.
        let _ = (&self.config, rows);
        Err(StorageError::WriteFailed(
            "BigQuery client not configured (no credentials)".into(),
        ))
    }
}

#[async_trait]
impl StorageBackend for BigQueryStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        tracing::info!(
            "BigQuery storage initialized: {}.{}.{}",
            self.config.project_id,
            self.config.dataset_id,
            self.config.table_id
        );
        Ok(())
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let row = exchange_to_row(exchange);
        let mut pending = self
            .pending_rows
            .lock()
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        pending.push(row);
        Ok(())
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        let rows: Vec<schema::BigQueryRow> = exchanges.iter().map(exchange_to_row).collect();

        // Try BigQuery with retry
        let rows_clone = rows.clone();
        let result = batch::retry_with_backoff(|| {
            let r = rows_clone.clone();
            async move { self.send_to_bigquery(&r).await }
        })
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(_e) => {
                // Fallback to JSONL
                self.write_to_fallback(&rows)
            }
        }
    }

    async fn flush(&self) -> Result<(), StorageError> {
        let rows: Vec<schema::BigQueryRow> = {
            let mut pending = self
                .pending_rows
                .lock()
                .map_err(|e| StorageError::FlushFailed(e.to_string()))?;
            std::mem::take(&mut *pending)
        };

        if rows.is_empty() {
            return Ok(());
        }

        // Try BigQuery with retry
        let rows_clone = rows.clone();
        let result = batch::retry_with_backoff(|| {
            let r = rows_clone.clone();
            async move { self.send_to_bigquery(&r).await }
        })
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(_e) => self.write_to_fallback(&rows),
        }
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use netcap_core::capture::exchange::{CapturedRequest, CapturedResponse};
    use uuid::Uuid;

    fn make_exchange() -> CapturedExchange {
        CapturedExchange {
            request: CapturedRequest {
                id: Uuid::now_v7(),
                session_id: Uuid::now_v7(),
                connection_id: Uuid::now_v7(),
                sequence_number: 0,
                timestamp: Utc::now(),
                method: Method::POST,
                uri: "https://api.example.com/data".parse().unwrap(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("request body"),
                body_truncated: false,
                tls_info: None,
            },
            response: Some(CapturedResponse {
                id: Uuid::now_v7(),
                request_id: Uuid::now_v7(),
                timestamp: Utc::now(),
                status: StatusCode::CREATED,
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("response"),
                body_truncated: false,
                latency: std::time::Duration::from_millis(150),
                ttfb: std::time::Duration::from_millis(50),
            }),
        }
    }

    #[test]
    fn new_with_fallback() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = BigQueryStorageConfig {
            project_id: "test-project".into(),
            dataset_id: "test-dataset".into(),
            table_id: "test-table".into(),
            fallback_jsonl_path: Some(tmp.path().join("fallback.jsonl")),
        };
        let storage = BigQueryStorage::new(config);
        assert!(storage.is_ok());
    }

    #[test]
    fn new_without_fallback() {
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: None,
        };
        let storage = BigQueryStorage::new(config);
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn write_queues_rows() {
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: None,
        };
        let storage = BigQueryStorage::new(config).unwrap();
        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();
        let pending = storage.pending_rows.lock().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].method, "POST");
    }

    #[tokio::test]
    async fn write_batch_falls_back_to_jsonl() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fallback_path = tmp.path().join("fallback.jsonl");
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: Some(fallback_path.clone()),
        };
        let storage = BigQueryStorage::new(config).unwrap();

        let exchanges = vec![make_exchange(), make_exchange()];
        // BigQuery is not configured, so it should fallback to JSONL
        let result = storage.write_batch(&exchanges).await;
        assert!(result.is_ok());

        // Verify JSONL fallback file has content
        let content = std::fs::read_to_string(&fallback_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line should be valid JSON
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
    }

    #[tokio::test]
    async fn flush_writes_pending_to_fallback() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fallback_path = tmp.path().join("fallback.jsonl");
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: Some(fallback_path.clone()),
        };
        let storage = BigQueryStorage::new(config).unwrap();

        storage.write(&make_exchange()).await.unwrap();
        storage.write(&make_exchange()).await.unwrap();
        assert_eq!(storage.pending_rows.lock().unwrap().len(), 2);

        storage.flush().await.unwrap();
        assert_eq!(storage.pending_rows.lock().unwrap().len(), 0);

        let content = std::fs::read_to_string(&fallback_path).unwrap();
        assert_eq!(content.lines().count(), 2);
    }

    #[tokio::test]
    async fn flush_empty_is_noop() {
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: None,
        };
        let storage = BigQueryStorage::new(config).unwrap();
        let result = storage.flush().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn write_batch_no_fallback_returns_error() {
        let config = BigQueryStorageConfig {
            project_id: "p".into(),
            dataset_id: "d".into(),
            table_id: "t".into(),
            fallback_jsonl_path: None,
        };
        let storage = BigQueryStorage::new(config).unwrap();
        let result = storage.write_batch(&[make_exchange()]).await;
        assert!(result.is_err());
    }
}
