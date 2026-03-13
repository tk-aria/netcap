use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigQueryRow {
    pub request_id: String,
    pub session_id: String,
    pub connection_id: String,
    pub timestamp: String,
    pub method: String,
    pub uri: String,
    pub host: String,
    pub path: String,
    pub status_code: Option<u16>,
    pub request_headers: String,
    pub request_body_size: u64,
    pub response_headers: Option<String>,
    pub response_body_size: Option<u64>,
    pub latency_ms: Option<f64>,
    pub ttfb_ms: Option<f64>,
    pub tls_sni: Option<String>,
    pub tls_version: Option<String>,
    pub tls_cipher: Option<String>,
}

pub fn exchange_to_row(
    exchange: &netcap_core::capture::exchange::CapturedExchange,
) -> BigQueryRow {
    let req = &exchange.request;
    let resp = exchange.response.as_ref();

    let request_headers: Vec<(String, String)> = req
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let response_headers: Option<Vec<(String, String)>> = resp.map(|r| {
        r.headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect()
    });

    BigQueryRow {
        request_id: req.id.to_string(),
        session_id: req.session_id.to_string(),
        connection_id: req.connection_id.to_string(),
        timestamp: req.timestamp.to_rfc3339(),
        method: req.method.to_string(),
        uri: req.uri.to_string(),
        host: req.uri.host().unwrap_or("").to_string(),
        path: req.uri.path().to_string(),
        status_code: resp.map(|r| r.status.as_u16()),
        request_headers: serde_json::to_string(&request_headers).unwrap_or_default(),
        request_body_size: req.body.len() as u64,
        response_headers: response_headers
            .map(|h| serde_json::to_string(&h).unwrap_or_default()),
        response_body_size: resp.map(|r| r.body.len() as u64),
        latency_ms: resp.map(|r| r.latency.as_secs_f64() * 1000.0),
        ttfb_ms: resp.map(|r| r.ttfb.as_secs_f64() * 1000.0),
        tls_sni: req.tls_info.as_ref().map(|t| t.sni.clone()),
        tls_version: req.tls_info.as_ref().map(|t| t.protocol_version.clone()),
        tls_cipher: req.tls_info.as_ref().map(|t| t.cipher_suite.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use netcap_core::capture::exchange::{
        CapturedExchange, CapturedRequest, CapturedResponse,
    };
    use uuid::Uuid;

    fn make_exchange() -> CapturedExchange {
        CapturedExchange {
            request: CapturedRequest {
                id: Uuid::now_v7(),
                session_id: Uuid::now_v7(),
                connection_id: Uuid::now_v7(),
                sequence_number: 0,
                timestamp: Utc::now(),
                method: Method::GET,
                uri: "http://example.com/api?q=1".parse().unwrap(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("hello"),
                body_truncated: false,
                tls_info: None,
            },
            response: Some(CapturedResponse {
                id: Uuid::now_v7(),
                request_id: Uuid::now_v7(),
                timestamp: Utc::now(),
                status: StatusCode::OK,
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("world"),
                body_truncated: false,
                latency: std::time::Duration::from_millis(42),
                ttfb: std::time::Duration::from_millis(20),
            }),
        }
    }

    #[test]
    fn exchange_to_row_basic() {
        let exchange = make_exchange();
        let row = exchange_to_row(&exchange);
        assert_eq!(row.method, "GET");
        assert_eq!(row.host, "example.com");
        assert_eq!(row.path, "/api");
        assert_eq!(row.status_code, Some(200));
        assert_eq!(row.request_body_size, 5);
        assert_eq!(row.response_body_size, Some(5));
        assert!(row.latency_ms.unwrap() > 40.0);
    }

    #[test]
    fn exchange_to_row_no_response() {
        let mut exchange = make_exchange();
        exchange.response = None;
        let row = exchange_to_row(&exchange);
        assert!(row.status_code.is_none());
        assert!(row.latency_ms.is_none());
        assert!(row.response_headers.is_none());
    }
}
