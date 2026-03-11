use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    pub sni: String,
    pub protocol_version: String,
    pub cipher_suite: String,
}

#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub id: Uuid,
    pub session_id: Uuid,
    pub connection_id: Uuid,
    pub sequence_number: u64,
    pub timestamp: DateTime<Utc>,
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub body_truncated: bool,
    pub tls_info: Option<TlsInfo>,
}

#[derive(Debug, Clone)]
pub struct CapturedResponse {
    pub id: Uuid,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub status: StatusCode,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub body_truncated: bool,
    pub latency: std::time::Duration,
    pub ttfb: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct CapturedExchange {
    pub request: CapturedRequest,
    pub response: Option<CapturedResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            headers: HeaderMap::new(),
            body: Bytes::from("hello"),
            body_truncated: false,
            tls_info: None,
        }
    }

    #[test]
    fn create_captured_request() {
        let req = make_request();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.sequence_number, 1);
        assert!(!req.body_truncated);
    }

    #[test]
    fn create_captured_response() {
        let resp = CapturedResponse {
            id: Uuid::now_v7(),
            request_id: Uuid::now_v7(),
            timestamp: Utc::now(),
            status: StatusCode::OK,
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            body: Bytes::from("world"),
            body_truncated: false,
            latency: std::time::Duration::from_millis(100),
            ttfb: std::time::Duration::from_millis(50),
        };
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.latency.as_millis(), 100);
    }

    #[test]
    fn create_captured_exchange() {
        let req = make_request();
        let exchange = CapturedExchange {
            request: req,
            response: None,
        };
        assert!(exchange.response.is_none());
    }

    #[test]
    fn tls_info_serialization() {
        let info = TlsInfo {
            sni: "example.com".into(),
            protocol_version: "TLSv1.3".into(),
            cipher_suite: "TLS_AES_256_GCM_SHA384".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: TlsInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sni, "example.com");
    }
}
