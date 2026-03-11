// crates/netcap-core/tests/integration_test.rs
// Integration tests for Phase 1 modules

use netcap_core::capture::body::{decode_body, truncate_body};
use netcap_core::capture::exchange::{CapturedExchange, CapturedRequest, CapturedResponse, TlsInfo};
use netcap_core::config::{DefaultAction, ProxyConfig, SessionConfig, StorageBackendType};
use netcap_core::error::{CaptureError, CertError, FilterError, ProxyError, StorageError};
use netcap_core::filter::pattern::DomainPattern;
use netcap_core::filter::{CaptureDecision, DomainFilter, DomainMatcher, FilterRule, FilterType};
use netcap_core::storage::FanoutWriter;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use http::{HeaderMap, Method, StatusCode, Version};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================
// Error type integration tests
// ============================================================

#[test]
fn error_hierarchy_proxy_to_capture() {
    let proxy_err = ProxyError::AlreadyRunning;
    let capture_err: CaptureError = proxy_err.into();
    assert!(capture_err.to_string().contains("proxy error"));
}

#[test]
fn error_hierarchy_storage_to_capture() {
    let storage_err = StorageError::WriteFailed("disk full".into());
    let capture_err: CaptureError = storage_err.into();
    assert!(capture_err.to_string().contains("storage error"));
}

#[test]
fn error_hierarchy_cert_to_capture() {
    let cert_err = CertError::CaGenerationFailed("bad key".into());
    let capture_err: CaptureError = cert_err.into();
    assert!(capture_err.to_string().contains("TLS error"));
}

#[test]
fn error_hierarchy_filter_to_capture() {
    let filter_err = FilterError::InvalidPattern("bad".into());
    let capture_err: CaptureError = filter_err.into();
    assert!(capture_err.to_string().contains("filter error"));
}

#[test]
fn error_hierarchy_io_to_capture() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let capture_err: CaptureError = io_err.into();
    assert!(capture_err.to_string().contains("IO error"));
}

#[test]
fn all_error_variants_display() {
    // ProxyError variants
    let _ = ProxyError::AlreadyRunning.to_string();
    let _ = ProxyError::NotRunning.to_string();
    let _ = ProxyError::UpstreamConnection("fail".into()).to_string();

    // StorageError variants
    let _ = StorageError::InitFailed("fail".into()).to_string();
    let _ = StorageError::WriteFailed("fail".into()).to_string();
    let _ = StorageError::FlushFailed("fail".into()).to_string();
    let _ = StorageError::ConnectionLost("fail".into()).to_string();

    // CertError variants
    let _ = CertError::CaGenerationFailed("fail".into()).to_string();
    let _ = CertError::StoreAccessFailed("fail".into()).to_string();
    let _ = CertError::ServerCertFailed {
        domain: "test.com".into(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "inner")),
    }
    .to_string();

    // FilterError variants
    let _ = FilterError::InvalidPattern("fail".into()).to_string();
    let regex_err = regex::Regex::new("[bad").unwrap_err();
    let _ = FilterError::from(regex_err).to_string();
}

// ============================================================
// Capture exchange integration tests
// ============================================================

fn make_test_request() -> CapturedRequest {
    CapturedRequest {
        id: Uuid::now_v7(),
        session_id: Uuid::now_v7(),
        connection_id: Uuid::now_v7(),
        sequence_number: 1,
        timestamp: Utc::now(),
        method: Method::POST,
        uri: "https://api.example.com/data".parse().unwrap(),
        version: Version::HTTP_2,
        headers: {
            let mut h = HeaderMap::new();
            h.insert("content-type", "application/json".parse().unwrap());
            h
        },
        body: Bytes::from(r#"{"key": "value"}"#),
        body_truncated: false,
        tls_info: Some(TlsInfo {
            sni: "api.example.com".into(),
            protocol_version: "TLSv1.3".into(),
            cipher_suite: "TLS_AES_256_GCM_SHA384".into(),
        }),
    }
}

fn make_test_response(request_id: Uuid) -> CapturedResponse {
    CapturedResponse {
        id: Uuid::now_v7(),
        request_id,
        timestamp: Utc::now(),
        status: StatusCode::OK,
        version: Version::HTTP_2,
        headers: {
            let mut h = HeaderMap::new();
            h.insert("content-type", "application/json".parse().unwrap());
            h
        },
        body: Bytes::from(r#"{"result": "ok"}"#),
        body_truncated: false,
        latency: std::time::Duration::from_millis(150),
        ttfb: std::time::Duration::from_millis(50),
    }
}

#[test]
fn captured_exchange_full_lifecycle() {
    let req = make_test_request();
    let req_id = req.id;
    let resp = make_test_response(req_id);

    let exchange = CapturedExchange {
        request: req,
        response: Some(resp),
    };

    assert_eq!(exchange.request.method, Method::POST);
    assert!(exchange.request.tls_info.is_some());
    let resp = exchange.response.as_ref().unwrap();
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.request_id, req_id);
    assert_eq!(resp.latency.as_millis(), 150);
}

#[test]
fn captured_exchange_without_response() {
    let req = make_test_request();
    let exchange = CapturedExchange {
        request: req,
        response: None,
    };
    assert!(exchange.response.is_none());
}

#[test]
fn tls_info_json_roundtrip() {
    let info = TlsInfo {
        sni: "test.example.com".into(),
        protocol_version: "TLSv1.2".into(),
        cipher_suite: "ECDHE-RSA-AES128-GCM-SHA256".into(),
    };
    let json = serde_json::to_string(&info).unwrap();
    let deserialized: TlsInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.sni, "test.example.com");
    assert_eq!(deserialized.protocol_version, "TLSv1.2");
    assert_eq!(deserialized.cipher_suite, "ECDHE-RSA-AES128-GCM-SHA256");
}

// ============================================================
// Body processing integration tests
// ============================================================

#[test]
fn truncate_body_under_limit() {
    let body = Bytes::from("short");
    let (result, truncated) = truncate_body(&body, 100);
    assert_eq!(result.len(), 5);
    assert!(!truncated);
}

#[test]
fn truncate_body_over_limit() {
    let body = Bytes::from(vec![b'x'; 200]);
    let (result, truncated) = truncate_body(&body, 50);
    assert_eq!(result.len(), 50);
    assert!(truncated);
}

#[test]
fn decode_body_gzip_roundtrip() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let original = b"Hello, gzip world! This is a test message for compression.";
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    let decoded = decode_body(&compressed, "gzip").unwrap();
    assert_eq!(decoded.as_slice(), original);
}

#[test]
fn decode_body_deflate_roundtrip() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let original = b"Hello, deflate world!";
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    let decoded = decode_body(&compressed, "deflate").unwrap();
    assert_eq!(decoded.as_slice(), original);
}

#[test]
fn decode_body_brotli_roundtrip() {
    let original = b"Hello, brotli world!";
    let mut compressed = Vec::new();
    {
        let mut writer =
            brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
        std::io::Write::write_all(&mut writer, original).unwrap();
    }

    let decoded = decode_body(&compressed, "br").unwrap();
    assert_eq!(decoded.as_slice(), original);
}

#[test]
fn decode_body_identity() {
    let original = b"plain text";
    let decoded = decode_body(original, "identity").unwrap();
    assert_eq!(decoded.as_slice(), original);
}

// ============================================================
// Config integration tests
// ============================================================

#[test]
fn proxy_config_default_values() {
    let config = ProxyConfig::default();
    assert_eq!(config.listen_addr.to_string(), "127.0.0.1:8080");
    assert_eq!(config.max_connections, 1024);
    assert_eq!(config.max_body_size, 10 * 1024 * 1024);
    assert_eq!(config.request_timeout, std::time::Duration::from_secs(30));
    assert!(config.upstream_proxy.is_none());
}

#[test]
fn proxy_config_toml_like_deserialization() {
    let json = r#"{
        "listen_addr": "0.0.0.0:3128",
        "max_connections": 2048,
        "max_body_size": 5242880,
        "request_timeout": 60
    }"#;
    let config: ProxyConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.listen_addr.to_string(), "0.0.0.0:3128");
    assert_eq!(config.max_connections, 2048);
    assert_eq!(config.max_body_size, 5242880);
    assert_eq!(config.request_timeout, std::time::Duration::from_secs(60));
}

#[test]
fn session_config_with_multiple_backends() {
    let config = SessionConfig {
        name: Some("multi-backend-session".into()),
        capture_request_body: true,
        capture_response_body: true,
        max_body_size_bytes: 2048,
        storage_backends: vec![
            StorageBackendType::Sqlite {
                path: "/tmp/cap.db".into(),
            },
            StorageBackendType::Jsonl {
                path: "/tmp/cap.jsonl".into(),
                rotate_size: Some(10 * 1024 * 1024),
            },
            StorageBackendType::Pcap {
                path: "/tmp/cap.pcap".into(),
            },
        ],
        default_action: DefaultAction::Capture,
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SessionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.storage_backends.len(), 3);
    assert_eq!(deserialized.name.unwrap(), "multi-backend-session");
}

#[test]
fn default_action_variants() {
    let capture = DefaultAction::Capture;
    let passthrough = DefaultAction::Passthrough;
    let default = DefaultAction::default();

    let _ = serde_json::to_string(&capture).unwrap();
    let _ = serde_json::to_string(&passthrough).unwrap();
    matches!(default, DefaultAction::Capture);
}

// ============================================================
// Domain filter integration tests
// ============================================================

#[test]
fn domain_filter_exact_match() {
    let mut filter = DomainFilter::new();
    filter.add_rule(FilterRule {
        id: Uuid::now_v7(),
        name: "api".into(),
        filter_type: FilterType::Include,
        pattern: DomainPattern::new_exact("api.example.com"),
        priority: 0,
        enabled: true,
    });
    assert!(matches!(
        filter.evaluate("api.example.com"),
        CaptureDecision::Capture(_)
    ));
    assert!(matches!(
        filter.evaluate("other.example.com"),
        CaptureDecision::Default
    ));
}

#[test]
fn domain_filter_wildcard_match() {
    let mut filter = DomainFilter::new();
    filter.add_rule(FilterRule {
        id: Uuid::now_v7(),
        name: "all-example".into(),
        filter_type: FilterType::Include,
        pattern: DomainPattern::new_wildcard("*.example.com"),
        priority: 0,
        enabled: true,
    });
    assert!(matches!(
        filter.evaluate("api.example.com"),
        CaptureDecision::Capture(_)
    ));
    assert!(matches!(
        filter.evaluate("www.example.com"),
        CaptureDecision::Capture(_)
    ));
    // Bare domain should NOT match wildcard
    assert!(matches!(
        filter.evaluate("example.com"),
        CaptureDecision::Default
    ));
}

#[test]
fn domain_filter_regex_match() {
    let mut filter = DomainFilter::new();
    filter.add_rule(FilterRule {
        id: Uuid::now_v7(),
        name: "api-pattern".into(),
        filter_type: FilterType::Include,
        pattern: DomainPattern::new_regex(r"^api\d+\.example\.com$").unwrap(),
        priority: 0,
        enabled: true,
    });
    assert!(matches!(
        filter.evaluate("api123.example.com"),
        CaptureDecision::Capture(_)
    ));
    assert!(matches!(
        filter.evaluate("web.example.com"),
        CaptureDecision::Default
    ));
}

#[test]
fn domain_filter_priority_override() {
    let mut filter = DomainFilter::new();

    // Low-priority: include all subdomains
    filter.add_rule(FilterRule {
        id: Uuid::now_v7(),
        name: "include-all".into(),
        filter_type: FilterType::Include,
        pattern: DomainPattern::new_wildcard("*.example.com"),
        priority: 1,
        enabled: true,
    });

    // High-priority: exclude internal
    filter.add_rule(FilterRule {
        id: Uuid::now_v7(),
        name: "exclude-internal".into(),
        filter_type: FilterType::Exclude,
        pattern: DomainPattern::new_exact("internal.example.com"),
        priority: 10,
        enabled: true,
    });

    // api.example.com -> captured (matches wildcard)
    assert!(matches!(
        filter.evaluate("api.example.com"),
        CaptureDecision::Capture(_)
    ));
    // internal.example.com -> passthrough (higher priority exclude)
    assert!(matches!(
        filter.evaluate("internal.example.com"),
        CaptureDecision::Passthrough
    ));
}

// ============================================================
// FanoutWriter integration tests
// ============================================================

#[derive(Debug)]
struct MockBackend {
    written: Arc<Mutex<Vec<Uuid>>>,
    flushed: Arc<Mutex<bool>>,
}

impl MockBackend {
    fn new() -> (Self, Arc<Mutex<Vec<Uuid>>>, Arc<Mutex<bool>>) {
        let written = Arc::new(Mutex::new(Vec::new()));
        let flushed = Arc::new(Mutex::new(false));
        (
            Self {
                written: written.clone(),
                flushed: flushed.clone(),
            },
            written,
            flushed,
        )
    }
}

#[async_trait]
impl netcap_core::storage::StorageBackend for MockBackend {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        self.written.lock().unwrap().push(exchange.request.id);
        Ok(())
    }
    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        for e in exchanges {
            self.written.lock().unwrap().push(e.request.id);
        }
        Ok(())
    }
    async fn flush(&self) -> Result<(), StorageError> {
        *self.flushed.lock().unwrap() = true;
        Ok(())
    }
    async fn close(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[tokio::test]
async fn fanout_writer_writes_to_all_backends() {
    let (backend1, written1, _flushed1) = MockBackend::new();
    let (backend2, written2, _flushed2) = MockBackend::new();

    let fanout = FanoutWriter::new(vec![Box::new(backend1), Box::new(backend2)]);

    let req = make_test_request();
    let req_id = req.id;
    let exchange = CapturedExchange {
        request: req,
        response: None,
    };

    let results = fanout.write_all(&exchange).await;
    assert!(results.iter().all(|r| r.is_ok()));

    assert_eq!(written1.lock().unwrap().len(), 1);
    assert_eq!(written1.lock().unwrap()[0], req_id);
    assert_eq!(written2.lock().unwrap().len(), 1);
    assert_eq!(written2.lock().unwrap()[0], req_id);
}

#[tokio::test]
async fn fanout_writer_flushes_all_backends() {
    let (backend1, _written1, flushed1) = MockBackend::new();
    let (backend2, _written2, flushed2) = MockBackend::new();

    let fanout = FanoutWriter::new(vec![Box::new(backend1), Box::new(backend2)]);

    let results = fanout.flush_all().await;
    assert!(results.iter().all(|r| r.is_ok()));
    assert!(*flushed1.lock().unwrap());
    assert!(*flushed2.lock().unwrap());
}

#[tokio::test]
async fn fanout_writer_empty_backends() {
    let fanout = FanoutWriter::new(vec![]);
    let req = make_test_request();
    let exchange = CapturedExchange {
        request: req,
        response: None,
    };
    let results = fanout.write_all(&exchange).await;
    assert!(results.is_empty());
}
