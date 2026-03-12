use netcap_core::capture::exchange::{CapturedExchange, CapturedRequest, CapturedResponse};
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Serialize an HTTP version to a string representation.
fn version_to_string(version: http::Version) -> &'static str {
    match version {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2.0",
        http::Version::HTTP_3 => "HTTP/3.0",
        _ => "HTTP/unknown",
    }
}

/// Serialize an `http::HeaderMap` to a JSON string.
/// Headers with multiple values for the same key are stored as comma-separated values.
fn headers_to_json(headers: &http::HeaderMap) -> String {
    let mut map: HashMap<&str, String> = HashMap::new();
    for (name, value) in headers.iter() {
        let key: &str = name.as_str();
        let val: String = String::from_utf8_lossy(value.as_bytes()).into_owned();
        map.entry(key)
            .and_modify(|existing| {
                existing.push_str(", ");
                existing.push_str(&val);
            })
            .or_insert(val);
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Insert a captured request into the database.
pub fn insert_request(conn: &Connection, req: &CapturedRequest) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO http_requests (id, session_id, connection_id, sequence_number, method, url, http_version, headers_json, body, body_truncated, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            req.id.to_string(),
            req.session_id.to_string(),
            req.connection_id.to_string(),
            req.sequence_number as i64,
            req.method.to_string(),
            req.uri.to_string(),
            version_to_string(req.version),
            headers_to_json(&req.headers),
            req.body.as_ref(),
            req.body_truncated as i32,
            req.timestamp.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Insert a captured response into the database.
pub fn insert_response(conn: &Connection, resp: &CapturedResponse) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO http_responses (id, request_id, status_code, http_version, headers_json, body, body_truncated, latency_us, ttfb_us, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            resp.id.to_string(),
            resp.request_id.to_string(),
            resp.status.as_u16() as i64,
            version_to_string(resp.version),
            headers_to_json(&resp.headers),
            resp.body.as_ref(),
            resp.body_truncated as i32,
            resp.latency.as_micros() as i64,
            resp.ttfb.as_micros() as i64,
            resp.timestamp.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Insert a complete exchange (request + optional response) into the database.
pub fn insert_exchange(conn: &Connection, exchange: &CapturedExchange) -> Result<(), rusqlite::Error> {
    insert_request(conn, &exchange.request)?;
    if let Some(ref resp) = exchange.response {
        insert_response(conn, resp)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::initialize_schema;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, HeaderValue, Method, StatusCode, Version};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup_db() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        initialize_schema(&conn).unwrap();
        (dir, conn)
    }

    fn make_request() -> CapturedRequest {
        CapturedRequest {
            id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            connection_id: Uuid::now_v7(),
            sequence_number: 1,
            timestamp: Utc::now(),
            method: Method::GET,
            uri: "https://example.com/api".parse().unwrap(),
            version: Version::HTTP_11,
            headers: {
                let mut h = HeaderMap::new();
                h.insert("content-type", HeaderValue::from_static("application/json"));
                h
            },
            body: Bytes::from("request body"),
            body_truncated: false,
            tls_info: None,
        }
    }

    fn make_response(request_id: Uuid) -> CapturedResponse {
        CapturedResponse {
            id: Uuid::now_v7(),
            request_id,
            timestamp: Utc::now(),
            status: StatusCode::OK,
            version: Version::HTTP_11,
            headers: {
                let mut h = HeaderMap::new();
                h.insert("content-type", HeaderValue::from_static("text/plain"));
                h
            },
            body: Bytes::from("response body"),
            body_truncated: false,
            latency: std::time::Duration::from_millis(150),
            ttfb: std::time::Duration::from_millis(50),
        }
    }

    #[test]
    fn insert_and_read_request() {
        let (_dir, conn) = setup_db();
        let req = make_request();
        let req_id = req.id.to_string();

        insert_request(&conn, &req).unwrap();

        let (method, url, body): (String, String, Vec<u8>) = conn
            .query_row(
                "SELECT method, url, body FROM http_requests WHERE id = ?1",
                params![req_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(method, "GET");
        assert_eq!(url, "https://example.com/api");
        assert_eq!(body, b"request body");
    }

    #[test]
    fn insert_and_read_response() {
        let (_dir, conn) = setup_db();
        let req = make_request();
        insert_request(&conn, &req).unwrap();

        let resp = make_response(req.id);
        let resp_id = resp.id.to_string();

        insert_response(&conn, &resp).unwrap();

        let (status_code, latency_us, ttfb_us): (i64, i64, i64) = conn
            .query_row(
                "SELECT status_code, latency_us, ttfb_us FROM http_responses WHERE id = ?1",
                params![resp_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(status_code, 200);
        assert_eq!(latency_us, 150_000);
        assert_eq!(ttfb_us, 50_000);
    }

    #[test]
    fn insert_exchange_with_response() {
        let (_dir, conn) = setup_db();
        let req = make_request();
        let resp = make_response(req.id);
        let exchange = CapturedExchange {
            request: req,
            response: Some(resp),
        };

        insert_exchange(&conn, &exchange).unwrap();

        let req_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        let resp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_responses", [], |row| row.get(0))
            .unwrap();

        assert_eq!(req_count, 1);
        assert_eq!(resp_count, 1);
    }

    #[test]
    fn insert_exchange_without_response() {
        let (_dir, conn) = setup_db();
        let req = make_request();
        let exchange = CapturedExchange {
            request: req,
            response: None,
        };

        insert_exchange(&conn, &exchange).unwrap();

        let req_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
            .unwrap();
        let resp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM http_responses", [], |row| row.get(0))
            .unwrap();

        assert_eq!(req_count, 1);
        assert_eq!(resp_count, 0);
    }

    #[test]
    fn headers_serialized_as_json() {
        let (_dir, conn) = setup_db();
        let req = make_request();
        let req_id = req.id.to_string();

        insert_request(&conn, &req).unwrap();

        let headers_json: String = conn
            .query_row(
                "SELECT headers_json FROM http_requests WHERE id = ?1",
                params![req_id],
                |row| row.get(0),
            )
            .unwrap();

        let parsed: HashMap<String, String> = serde_json::from_str(&headers_json).unwrap();
        assert_eq!(parsed.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn version_string_formatting() {
        assert_eq!(version_to_string(Version::HTTP_09), "HTTP/0.9");
        assert_eq!(version_to_string(Version::HTTP_10), "HTTP/1.0");
        assert_eq!(version_to_string(Version::HTTP_11), "HTTP/1.1");
        assert_eq!(version_to_string(Version::HTTP_2), "HTTP/2.0");
        assert_eq!(version_to_string(Version::HTTP_3), "HTTP/3.0");
    }

    #[test]
    fn body_stored_as_blob() {
        let (_dir, conn) = setup_db();
        let mut req = make_request();
        req.body = Bytes::from(vec![0u8, 1, 2, 255, 254, 253]);
        let req_id = req.id.to_string();

        insert_request(&conn, &req).unwrap();

        let body: Vec<u8> = conn
            .query_row(
                "SELECT body FROM http_requests WHERE id = ?1",
                params![req_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(body, vec![0u8, 1, 2, 255, 254, 253]);
    }

    #[test]
    fn body_truncated_flag_stored() {
        let (_dir, conn) = setup_db();
        let mut req = make_request();
        req.body_truncated = true;
        let req_id = req.id.to_string();

        insert_request(&conn, &req).unwrap();

        let truncated: i32 = conn
            .query_row(
                "SELECT body_truncated FROM http_requests WHERE id = ?1",
                params![req_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(truncated, 1);
    }
}
