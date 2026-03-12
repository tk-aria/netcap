use rusqlite::Connection;

const PRAGMA_STATEMENTS: &str = "\
    PRAGMA journal_mode=WAL;\
    PRAGMA foreign_keys=ON;\
";

const CREATE_HTTP_REQUESTS: &str = "\
    CREATE TABLE IF NOT EXISTS http_requests (\
        id TEXT PRIMARY KEY NOT NULL,\
        session_id TEXT NOT NULL,\
        connection_id TEXT NOT NULL,\
        sequence_number INTEGER NOT NULL,\
        method TEXT NOT NULL,\
        url TEXT NOT NULL,\
        http_version TEXT NOT NULL,\
        headers_json TEXT NOT NULL,\
        body BLOB NOT NULL,\
        body_truncated INTEGER NOT NULL,\
        timestamp TEXT NOT NULL\
    );\
";

const CREATE_HTTP_RESPONSES: &str = "\
    CREATE TABLE IF NOT EXISTS http_responses (\
        id TEXT PRIMARY KEY NOT NULL,\
        request_id TEXT NOT NULL,\
        status_code INTEGER NOT NULL,\
        http_version TEXT NOT NULL,\
        headers_json TEXT NOT NULL,\
        body BLOB NOT NULL,\
        body_truncated INTEGER NOT NULL,\
        latency_us INTEGER NOT NULL,\
        ttfb_us INTEGER NOT NULL,\
        timestamp TEXT NOT NULL,\
        FOREIGN KEY (request_id) REFERENCES http_requests(id)\
    );\
";

const CREATE_INDEX_SESSION_ID: &str = "\
    CREATE INDEX IF NOT EXISTS idx_requests_session_id ON http_requests(session_id);\
";

const CREATE_INDEX_REQUEST_ID: &str = "\
    CREATE INDEX IF NOT EXISTS idx_responses_request_id ON http_responses(request_id);\
";

const CREATE_INDEX_REQUEST_TIMESTAMP: &str = "\
    CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON http_requests(timestamp);\
";

const CREATE_INDEX_RESPONSE_TIMESTAMP: &str = "\
    CREATE INDEX IF NOT EXISTS idx_responses_timestamp ON http_responses(timestamp);\
";

/// Initialize the database schema, creating tables and indexes if they do not exist.
pub fn initialize_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(PRAGMA_STATEMENTS)?;
    conn.execute_batch(CREATE_HTTP_REQUESTS)?;
    conn.execute_batch(CREATE_HTTP_RESPONSES)?;
    conn.execute_batch(CREATE_INDEX_SESSION_ID)?;
    conn.execute_batch(CREATE_INDEX_REQUEST_ID)?;
    conn.execute_batch(CREATE_INDEX_REQUEST_TIMESTAMP)?;
    conn.execute_batch(CREATE_INDEX_RESPONSE_TIMESTAMP)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_db(dir: &TempDir) -> Connection {
        let path = dir.path().join("test.db");
        Connection::open(&path).expect("failed to open db")
    }

    #[test]
    fn initialize_creates_tables() {
        let dir = TempDir::new().unwrap();
        let conn = open_db(&dir);
        initialize_schema(&conn).unwrap();

        // Verify tables exist by querying sqlite_master
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
    fn initialize_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let conn = open_db(&dir);
        initialize_schema(&conn).unwrap();
        initialize_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='http_requests'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn wal_mode_enabled() {
        let dir = TempDir::new().unwrap();
        let conn = open_db(&dir);
        initialize_schema(&conn).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");
    }

    #[test]
    fn indexes_created() {
        let dir = TempDir::new().unwrap();
        let conn = open_db(&dir);
        initialize_schema(&conn).unwrap();

        let indexes: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(indexes.contains(&"idx_requests_session_id".to_string()));
        assert!(indexes.contains(&"idx_responses_request_id".to_string()));
        assert!(indexes.contains(&"idx_requests_timestamp".to_string()));
        assert!(indexes.contains(&"idx_responses_timestamp".to_string()));
    }

    #[test]
    fn foreign_keys_enabled() {
        let dir = TempDir::new().unwrap();
        let conn = open_db(&dir);
        initialize_schema(&conn).unwrap();

        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}
