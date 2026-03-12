pub mod queries;
pub mod schema;

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use tracing::{debug, info};

use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;

/// Configuration for the SQLite storage backend.
#[derive(Debug, Clone)]
pub struct SqliteStorageConfig {
    pub db_path: PathBuf,
}

/// SQLite-based storage backend for captured HTTP exchanges.
pub struct SqliteStorage {
    conn: Mutex<Connection>,
    config: SqliteStorageConfig,
}

impl std::fmt::Debug for SqliteStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStorage")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl SqliteStorage {
    /// Create a new `SqliteStorage` instance, opening the database and initializing the schema.
    pub fn new(config: SqliteStorageConfig) -> Result<Self, StorageError> {
        let conn = Connection::open(&config.db_path).map_err(|e| {
            StorageError::InitFailed(format!(
                "failed to open database at {}: {}",
                config.db_path.display(),
                e
            ))
        })?;

        schema::initialize_schema(&conn).map_err(|e| {
            StorageError::InitFailed(format!("failed to initialize schema: {}", e))
        })?;

        info!(path = %config.db_path.display(), "SQLite storage initialized");

        Ok(Self {
            conn: Mutex::new(conn),
            config,
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::InitFailed(format!("failed to acquire lock: {}", e))
        })?;
        schema::initialize_schema(&conn).map_err(|e| {
            StorageError::InitFailed(format!("failed to initialize schema: {}", e))
        })?;
        debug!("SQLite schema re-initialized");
        Ok(())
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::WriteFailed(format!("failed to acquire lock: {}", e))
        })?;
        queries::insert_exchange(&conn, exchange).map_err(|e| {
            StorageError::WriteFailed(format!("failed to insert exchange: {}", e))
        })?;
        debug!(request_id = %exchange.request.id, "exchange written to SQLite");
        Ok(())
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::WriteFailed(format!("failed to acquire lock: {}", e))
        })?;
        let tx = conn.unchecked_transaction().map_err(|e| {
            StorageError::WriteFailed(format!("failed to begin transaction: {}", e))
        })?;
        for exchange in exchanges {
            queries::insert_exchange(&tx, exchange).map_err(|e| {
                StorageError::WriteFailed(format!(
                    "failed to insert exchange {}: {}",
                    exchange.request.id, e
                ))
            })?;
        }
        tx.commit().map_err(|e| {
            StorageError::WriteFailed(format!("failed to commit transaction: {}", e))
        })?;
        debug!(count = exchanges.len(), "batch written to SQLite");
        Ok(())
    }

    async fn flush(&self) -> Result<(), StorageError> {
        // SQLite with WAL mode handles durability automatically.
        // An explicit WAL checkpoint can be performed if needed.
        let conn = self.conn.lock().map_err(|e| {
            StorageError::FlushFailed(format!("failed to acquire lock: {}", e))
        })?;
        conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
            .map_err(|e| {
                StorageError::FlushFailed(format!("WAL checkpoint failed: {}", e))
            })?;
        debug!("SQLite WAL checkpoint completed");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        // Flush before closing
        self.flush().await?;
        info!(path = %self.config.db_path.display(), "SQLite storage closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, HeaderValue, Method, StatusCode, Version};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn make_config(dir: &TempDir) -> SqliteStorageConfig {
        SqliteStorageConfig {
            db_path: dir.path().join("test.db"),
        }
    }

    fn make_request() -> netcap_core::capture::exchange::CapturedRequest {
        netcap_core::capture::exchange::CapturedRequest {
            id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            connection_id: Uuid::now_v7(),
            sequence_number: 1,
            timestamp: Utc::now(),
            method: Method::POST,
            uri: "https://example.com/data".parse().unwrap(),
            version: Version::HTTP_11,
            headers: {
                let mut h = HeaderMap::new();
                h.insert("content-type", HeaderValue::from_static("application/json"));
                h
            },
            body: Bytes::from(r#"{"key":"value"}"#),
            body_truncated: false,
            tls_info: None,
        }
    }

    fn make_response(
        request_id: Uuid,
    ) -> netcap_core::capture::exchange::CapturedResponse {
        netcap_core::capture::exchange::CapturedResponse {
            id: Uuid::now_v7(),
            request_id,
            timestamp: Utc::now(),
            status: StatusCode::CREATED,
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            body: Bytes::from("created"),
            body_truncated: false,
            latency: std::time::Duration::from_millis(200),
            ttfb: std::time::Duration::from_millis(80),
        }
    }

    fn make_exchange() -> CapturedExchange {
        let req = make_request();
        let resp = make_response(req.id);
        CapturedExchange {
            request: req,
            response: Some(resp),
        }
    }

    #[test]
    fn new_creates_database() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let db_path = config.db_path.clone();

        let _storage = SqliteStorage::new(config).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn new_initializes_schema() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        let conn = storage.conn.lock().unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"http_requests".to_string()));
        assert!(tables.contains(&"http_responses".to_string()));
    }

    #[test]
    fn new_fails_on_bad_path() {
        let config = SqliteStorageConfig {
            db_path: PathBuf::from("/nonexistent/dir/that/does/not/exist/test.db"),
        };
        let result = SqliteStorage::new(config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, StorageError::InitFailed(_)));
    }

    #[tokio::test]
    async fn write_single_exchange() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();

        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let resp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_responses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(resp_count, 1);
    }

    #[tokio::test]
    async fn write_exchange_without_response() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        let exchange = CapturedExchange {
            request: make_request(),
            response: None,
        };
        storage.write(&exchange).await.unwrap();

        let conn = storage.conn.lock().unwrap();
        let req_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        let resp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_responses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(req_count, 1);
        assert_eq!(resp_count, 0);
    }

    #[tokio::test]
    async fn write_batch_multiple_exchanges() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        let exchanges: Vec<CapturedExchange> = (0..5).map(|_| make_exchange()).collect();
        storage.write_batch(&exchanges).await.unwrap();

        let conn = storage.conn.lock().unwrap();
        let req_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        let resp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_responses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(req_count, 5);
        assert_eq!(resp_count, 5);
    }

    #[tokio::test]
    async fn write_batch_empty() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        storage.write_batch(&[]).await.unwrap();
    }

    #[tokio::test]
    async fn flush_succeeds() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        storage.flush().await.unwrap();
    }

    #[tokio::test]
    async fn close_succeeds() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let mut storage = SqliteStorage::new(config).unwrap();

        storage.close().await.unwrap();
    }

    #[tokio::test]
    async fn initialize_reinitializes() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let mut storage = SqliteStorage::new(config).unwrap();

        // Write data then re-initialize (should not fail; tables have IF NOT EXISTS)
        let exchange = make_exchange();
        storage.write(&exchange).await.unwrap();
        storage.initialize().await.unwrap();

        // Data should still be there
        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn write_batch_is_atomic() {
        let dir = TempDir::new().unwrap();
        let config = make_config(&dir);
        let storage = SqliteStorage::new(config).unwrap();

        // Create exchanges where the second one has a duplicate request ID to cause a failure
        let exchange1 = make_exchange();
        let mut exchange2 = make_exchange();
        exchange2.request.id = exchange1.request.id; // duplicate primary key

        let result = storage.write_batch(&[exchange1, exchange2]).await;
        assert!(result.is_err());

        // Due to transaction rollback, no rows should have been inserted
        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
