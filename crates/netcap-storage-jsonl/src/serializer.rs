use std::collections::HashMap;

use chrono::{DateTime, Utc};
use http::{HeaderMap, Version};
use netcap_core::capture::exchange::{CapturedExchange, CapturedRequest, CapturedResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SerializableTlsInfo {
    sni: String,
    protocol_version: String,
    cipher_suite: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableRequest {
    id: Uuid,
    session_id: Uuid,
    connection_id: Uuid,
    sequence_number: u64,
    timestamp: DateTime<Utc>,
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, Vec<String>>,
    body_base64: String,
    body_truncated: bool,
    tls_info: Option<SerializableTlsInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableResponse {
    id: Uuid,
    request_id: Uuid,
    timestamp: DateTime<Utc>,
    status: u16,
    version: String,
    headers: HashMap<String, Vec<String>>,
    body_base64: String,
    body_truncated: bool,
    latency_ms: u128,
    ttfb_ms: u128,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableExchange {
    request: SerializableRequest,
    response: Option<SerializableResponse>,
}

fn version_to_string(version: &Version) -> String {
    match *version {
        Version::HTTP_09 => "HTTP/0.9".to_string(),
        Version::HTTP_10 => "HTTP/1.0".to_string(),
        Version::HTTP_11 => "HTTP/1.1".to_string(),
        Version::HTTP_2 => "HTTP/2.0".to_string(),
        Version::HTTP_3 => "HTTP/3.0".to_string(),
        _ => format!("{:?}", version),
    }
}

fn serialize_headers(headers: &HeaderMap) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_string();
        let val = value.to_str().unwrap_or("<binary>").to_string();
        map.entry(key).or_default().push(val);
    }
    map
}

fn encode_body(body: &bytes::Bytes) -> String {
    String::from_utf8(body.to_vec()).unwrap_or_else(|_| {
        // Encode as hex for binary data
        body.iter().map(|b| format!("{:02x}", b)).collect()
    })
}

fn from_request(req: &CapturedRequest) -> SerializableRequest {
    SerializableRequest {
        id: req.id,
        session_id: req.session_id,
        connection_id: req.connection_id,
        sequence_number: req.sequence_number,
        timestamp: req.timestamp,
        method: req.method.as_str().to_string(),
        uri: req.uri.to_string(),
        version: version_to_string(&req.version),
        headers: serialize_headers(&req.headers),
        body_base64: encode_body(&req.body),
        body_truncated: req.body_truncated,
        tls_info: req.tls_info.as_ref().map(|t| SerializableTlsInfo {
            sni: t.sni.clone(),
            protocol_version: t.protocol_version.clone(),
            cipher_suite: t.cipher_suite.clone(),
        }),
    }
}

fn from_response(resp: &CapturedResponse) -> SerializableResponse {
    SerializableResponse {
        id: resp.id,
        request_id: resp.request_id,
        timestamp: resp.timestamp,
        status: resp.status.as_u16(),
        version: version_to_string(&resp.version),
        headers: serialize_headers(&resp.headers),
        body_base64: encode_body(&resp.body),
        body_truncated: resp.body_truncated,
        latency_ms: resp.latency.as_millis(),
        ttfb_ms: resp.ttfb.as_millis(),
    }
}

fn from_exchange(exchange: &CapturedExchange) -> SerializableExchange {
    SerializableExchange {
        request: from_request(&exchange.request),
        response: exchange.response.as_ref().map(from_response),
    }
}

/// Serialize a `CapturedExchange` to a single JSON line (no trailing newline).
pub fn to_jsonl(exchange: &CapturedExchange) -> Result<String, serde_json::Error> {
    let serializable = from_exchange(exchange);
    serde_json::to_string(&serializable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, HeaderValue, Method, StatusCode, Version};
    use netcap_core::capture::exchange::TlsInfo;
    use uuid::Uuid;

    fn make_test_request() -> CapturedRequest {
        CapturedRequest {
            id: Uuid::nil(),
            session_id: Uuid::nil(),
            connection_id: Uuid::nil(),
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

    fn make_test_response(request_id: Uuid) -> CapturedResponse {
        CapturedResponse {
            id: Uuid::nil(),
            request_id,
            timestamp: Utc::now(),
            status: StatusCode::OK,
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            body: Bytes::from("world"),
            body_truncated: false,
            latency: std::time::Duration::from_millis(100),
            ttfb: std::time::Duration::from_millis(50),
        }
    }

    #[test]
    fn to_jsonl_produces_valid_json() {
        let exchange = CapturedExchange {
            request: make_test_request(),
            response: None,
        };
        let json = to_jsonl(&exchange).unwrap();
        // Must be valid JSON
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.is_object());
        // Must not contain newlines (single line)
        assert!(!json.contains('\n'));
    }

    #[test]
    fn to_jsonl_round_trip() {
        let exchange = CapturedExchange {
            request: make_test_request(),
            response: Some(make_test_response(Uuid::nil())),
        };
        let json = to_jsonl(&exchange).unwrap();
        let parsed: SerializableExchange = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.request.method, "GET");
        assert_eq!(parsed.request.uri, "https://example.com/api");
        assert_eq!(parsed.request.version, "HTTP/1.1");
        assert_eq!(parsed.request.body_base64, "hello");
        assert_eq!(parsed.response.as_ref().unwrap().status, 200);
        assert_eq!(parsed.response.as_ref().unwrap().body_base64, "world");
        assert_eq!(parsed.response.as_ref().unwrap().latency_ms, 100);
        assert_eq!(parsed.response.as_ref().unwrap().ttfb_ms, 50);
    }

    #[test]
    fn to_jsonl_with_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.append("accept", HeaderValue::from_static("text/html"));
        headers.append("accept", HeaderValue::from_static("application/json"));

        let mut req = make_test_request();
        req.headers = headers;

        let exchange = CapturedExchange {
            request: req,
            response: None,
        };
        let json = to_jsonl(&exchange).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let req_headers = &parsed["request"]["headers"];
        assert_eq!(
            req_headers["content-type"],
            serde_json::json!(["application/json"])
        );
        // accept should have two values
        let accept = req_headers["accept"].as_array().unwrap();
        assert_eq!(accept.len(), 2);
    }

    #[test]
    fn to_jsonl_with_tls_info() {
        let mut req = make_test_request();
        req.tls_info = Some(TlsInfo {
            sni: "example.com".into(),
            protocol_version: "TLSv1.3".into(),
            cipher_suite: "TLS_AES_256_GCM_SHA384".into(),
        });

        let exchange = CapturedExchange {
            request: req,
            response: None,
        };
        let json = to_jsonl(&exchange).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["request"]["tls_info"]["sni"], "example.com");
    }

    #[test]
    fn to_jsonl_no_response() {
        let exchange = CapturedExchange {
            request: make_test_request(),
            response: None,
        };
        let json = to_jsonl(&exchange).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["response"].is_null());
    }

    #[test]
    fn to_jsonl_binary_body() {
        let mut req = make_test_request();
        req.body = Bytes::from(vec![0xff, 0xfe, 0x00, 0x01]);

        let exchange = CapturedExchange {
            request: req,
            response: None,
        };
        let json = to_jsonl(&exchange).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // Binary body should be hex-encoded
        assert_eq!(parsed["request"]["body_base64"], "fffe0001");
    }
}
