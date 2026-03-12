pub mod rotation;
pub mod serializer;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;

/// Configuration for the JSONL storage backend.
#[derive(Debug, Clone)]
pub struct JsonlStorageConfig {
    /// Path to the output JSONL file.
    pub output_path: PathBuf,
    /// Optional size threshold (in bytes) at which to rotate the file.
    pub rotate_size: Option<u64>,
}

/// JSONL file storage backend for captured HTTP exchanges.
///
/// Each exchange is serialized as a single JSON line appended to the output file.
/// Optionally rotates the file when it exceeds a configured size threshold.
pub struct JsonlStorage {
    config: JsonlStorageConfig,
    writer: Mutex<tokio::fs::File>,
    bytes_written: AtomicU64,
}

impl JsonlStorage {
    /// Create a new `JsonlStorage`, opening the output file in append mode.
    pub async fn new(config: JsonlStorageConfig) -> Result<Self, StorageError> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.output_path)
            .await
            .map_err(|e| StorageError::InitFailed(format!("failed to open {}: {}", config.output_path.display(), e)))?;

        // Get current file size for bytes_written tracking
        let metadata = file
            .metadata()
            .await
            .map_err(|e| StorageError::InitFailed(format!("failed to read metadata: {}", e)))?;
        let initial_size = metadata.len();

        Ok(Self {
            config,
            writer: Mutex::new(file),
            bytes_written: AtomicU64::new(initial_size),
        })
    }

    /// Check if file rotation is needed and perform it if so.
    async fn maybe_rotate(&self) -> Result<(), StorageError> {
        if let Some(rotate_size) = self.config.rotate_size {
            if self.bytes_written.load(Ordering::Relaxed) >= rotate_size {
                // Drop the current writer lock, rotate, then reopen
                let mut writer = self.writer.lock().await;

                // Flush before rotation
                writer
                    .flush()
                    .await
                    .map_err(|e| StorageError::FlushFailed(format!("pre-rotation flush failed: {}", e)))?;

                // Perform rotation
                rotation::rotate(&self.config.output_path)
                    .await
                    .map_err(|e| StorageError::WriteFailed(format!("rotation failed: {}", e)))?;

                // Reopen the file (rotation created a new empty file)
                let new_file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.config.output_path)
                    .await
                    .map_err(|e| StorageError::WriteFailed(format!("failed to reopen after rotation: {}", e)))?;

                *writer = new_file;
                self.bytes_written.store(0, Ordering::Relaxed);

                tracing::info!(
                    path = %self.config.output_path.display(),
                    "file rotated successfully"
                );
            }
        }
        Ok(())
    }

    /// Write a single line to the file, tracking bytes written.
    async fn write_line(&self, line: &str) -> Result<(), StorageError> {
        let mut data = line.as_bytes().to_vec();
        data.push(b'\n');

        let mut writer = self.writer.lock().await;
        writer
            .write_all(&data)
            .await
            .map_err(|e| StorageError::WriteFailed(format!("write failed: {}", e)))?;

        self.bytes_written
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for JsonlStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        tracing::info!(
            path = %self.config.output_path.display(),
            "JSONL storage initialized"
        );
        Ok(())
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let line = serializer::to_jsonl(exchange)
            .map_err(|e| StorageError::WriteFailed(format!("serialization failed: {}", e)))?;

        self.write_line(&line).await?;
        self.maybe_rotate().await?;

        Ok(())
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        // Serialize all lines first so we fail fast on serialization errors
        let lines: Vec<String> = exchanges
            .iter()
            .map(|ex| {
                serializer::to_jsonl(ex)
                    .map_err(|e| StorageError::WriteFailed(format!("serialization failed: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Build a single buffer for efficient I/O
        let mut buf = Vec::new();
        for line in &lines {
            buf.extend_from_slice(line.as_bytes());
            buf.push(b'\n');
        }

        {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(&buf)
                .await
                .map_err(|e| StorageError::WriteFailed(format!("batch write failed: {}", e)))?;
        }

        self.bytes_written
            .fetch_add(buf.len() as u64, Ordering::Relaxed);

        self.maybe_rotate().await?;

        Ok(())
    }

    async fn flush(&self) -> Result<(), StorageError> {
        let mut writer = self.writer.lock().await;
        writer
            .flush()
            .await
            .map_err(|e| StorageError::FlushFailed(format!("flush failed: {}", e)))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.flush().await?;
        tracing::info!(
            path = %self.config.output_path.display(),
            bytes_written = self.bytes_written.load(Ordering::Relaxed),
            "JSONL storage closed"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{Method, StatusCode, Version};
    use netcap_core::capture::exchange::{CapturedRequest, CapturedResponse};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn make_exchange() -> CapturedExchange {
        CapturedExchange {
            request: CapturedRequest {
                id: Uuid::nil(),
                session_id: Uuid::nil(),
                connection_id: Uuid::nil(),
                sequence_number: 1,
                timestamp: Utc::now(),
                method: Method::POST,
                uri: "https://example.com/api/data".parse().unwrap(),
                version: Version::HTTP_11,
                headers: http::HeaderMap::new(),
                body: Bytes::from(r#"{"key":"value"}"#),
                body_truncated: false,
                tls_info: None,
            },
            response: Some(CapturedResponse {
                id: Uuid::nil(),
                request_id: Uuid::nil(),
                timestamp: Utc::now(),
                status: StatusCode::OK,
                version: Version::HTTP_11,
                headers: http::HeaderMap::new(),
                body: Bytes::from(r#"{"result":"ok"}"#),
                body_truncated: false,
                latency: std::time::Duration::from_millis(42),
                ttfb: std::time::Duration::from_millis(10),
            }),
        }
    }

    #[tokio::test]
    async fn write_creates_file_and_writes_jsonl() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.jsonl");

        let config = JsonlStorageConfig {
            output_path: path.clone(),
            rotate_size: None,
        };

        let storage = JsonlStorage::new(config).await.unwrap();
        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        // Each line must be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed["request"]["method"], "POST");
        assert_eq!(parsed["response"]["status"], 200);
    }

    #[tokio::test]
    async fn write_batch_writes_multiple_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("batch.jsonl");

        let config = JsonlStorageConfig {
            output_path: path.clone(),
            rotate_size: None,
        };

        let storage = JsonlStorage::new(config).await.unwrap();
        let exchanges = vec![make_exchange(), make_exchange(), make_exchange()];
        storage.write_batch(&exchanges).await.unwrap();
        storage.flush().await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);

        // All lines must be valid JSON
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
    }

    #[tokio::test]
    async fn rotation_triggers_at_threshold() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("rotate.jsonl");

        let config = JsonlStorageConfig {
            output_path: path.clone(),
            rotate_size: Some(50), // very small threshold to trigger rotation
        };

        let mut storage = JsonlStorage::new(config).await.unwrap();
        storage.initialize().await.unwrap();

        let exchange = make_exchange();
        // First write should exceed 50 bytes and trigger rotation
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        // After rotation, the current file should be empty or very small
        // and there should be a rotated file
        let mut entries = tokio::fs::read_dir(dir.path()).await.unwrap();
        let mut file_count = 0;
        while let Some(_entry) = entries.next_entry().await.unwrap() {
            file_count += 1;
        }
        // Should have at least 2 files: original (recreated) + rotated
        assert!(file_count >= 2, "expected at least 2 files after rotation, got {}", file_count);
    }

    #[tokio::test]
    async fn append_mode_preserves_existing_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("append.jsonl");

        // Write initial content
        tokio::fs::write(&path, "{\"existing\":true}\n").await.unwrap();

        let config = JsonlStorageConfig {
            output_path: path.clone(),
            rotate_size: None,
        };

        let storage = JsonlStorage::new(config).await.unwrap();
        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "{\"existing\":true}");
    }

    #[tokio::test]
    async fn close_flushes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("close.jsonl");

        let config = JsonlStorageConfig {
            output_path: path.clone(),
            rotate_size: None,
        };

        let mut storage = JsonlStorage::new(config).await.unwrap();
        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();
        storage.close().await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(!content.is_empty());
    }
}
