use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use hudsucker::async_trait::async_trait;
use hudsucker::hyper::{Body, Request, Response};
use hudsucker::{HttpContext, HttpHandler, RequestOrResponse};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::capture::exchange::{CapturedExchange, CapturedRequest, CapturedResponse};
use crate::filter::{CaptureDecision, DomainMatcher};

/// Conversion helpers: hudsucker uses http 0.2, our structs use http 1.x
mod convert {
    pub fn method(m: &hudsucker::hyper::Method) -> http::Method {
        http::Method::from_bytes(m.as_str().as_bytes()).unwrap_or(http::Method::GET)
    }

    pub fn uri(u: &hudsucker::hyper::Uri) -> http::Uri {
        u.to_string().parse().unwrap_or(http::Uri::from_static("/"))
    }

    pub fn version(v: hudsucker::hyper::Version) -> http::Version {
        match v {
            hudsucker::hyper::Version::HTTP_09 => http::Version::HTTP_09,
            hudsucker::hyper::Version::HTTP_10 => http::Version::HTTP_10,
            hudsucker::hyper::Version::HTTP_11 => http::Version::HTTP_11,
            hudsucker::hyper::Version::HTTP_2 => http::Version::HTTP_2,
            hudsucker::hyper::Version::HTTP_3 => http::Version::HTTP_3,
            _ => http::Version::HTTP_11,
        }
    }

    pub fn status(s: hudsucker::hyper::StatusCode) -> http::StatusCode {
        http::StatusCode::from_u16(s.as_u16()).unwrap_or(http::StatusCode::OK)
    }

    pub fn headers(h: &hudsucker::hyper::HeaderMap) -> http::HeaderMap {
        let mut map = http::HeaderMap::new();
        for (key, value) in h.iter() {
            if let (Ok(k), Ok(v)) = (
                http::header::HeaderName::from_bytes(key.as_str().as_bytes()),
                http::header::HeaderValue::from_bytes(value.as_bytes()),
            ) {
                map.insert(k, v);
            }
        }
        map
    }
}

#[derive(Clone)]
pub struct NetcapHandler {
    filter: Arc<dyn DomainMatcher>,
    event_tx: mpsc::Sender<CapturedExchange>,
    session_id: Uuid,
    max_body_size: usize,
}

impl NetcapHandler {
    pub fn new(
        filter: Arc<dyn DomainMatcher>,
        event_tx: mpsc::Sender<CapturedExchange>,
        session_id: Uuid,
        max_body_size: usize,
    ) -> Self {
        Self {
            filter,
            event_tx,
            session_id,
            max_body_size,
        }
    }

    fn extract_host(req: &Request<Body>) -> String {
        req.uri()
            .host()
            .map(|h| h.to_string())
            .or_else(|| {
                req.headers()
                    .get("host")
                    .and_then(|v| v.to_str().ok())
                    .map(|h| h.split(':').next().unwrap_or(h).to_string())
            })
            .unwrap_or_default()
    }

    fn truncate_body(body: &[u8], max_size: usize) -> (Bytes, bool) {
        if body.len() > max_size {
            (Bytes::copy_from_slice(&body[..max_size]), true)
        } else {
            (Bytes::copy_from_slice(body), false)
        }
    }

    /// Capture a request and send it to the event channel.
    /// Returns the body bytes so the caller can reconstruct the request.
    pub(crate) async fn capture_request(
        &self,
        parts: &hudsucker::hyper::http::request::Parts,
        body_bytes: &[u8],
    ) -> Option<CapturedExchange> {
        let (truncated_body, body_truncated) =
            Self::truncate_body(body_bytes, self.max_body_size);

        let captured = CapturedRequest {
            id: Uuid::now_v7(),
            session_id: self.session_id,
            connection_id: Uuid::now_v7(),
            sequence_number: 0,
            timestamp: Utc::now(),
            method: convert::method(&parts.method),
            uri: convert::uri(&parts.uri),
            version: convert::version(parts.version),
            headers: convert::headers(&parts.headers),
            body: truncated_body,
            body_truncated,
            tls_info: None,
        };

        Some(CapturedExchange {
            request: captured,
            response: None,
        })
    }

    /// Capture a response and send it to the event channel.
    pub(crate) fn capture_response(
        &self,
        parts: &hudsucker::hyper::http::response::Parts,
        body_bytes: &[u8],
    ) -> CapturedExchange {
        let (truncated_body, body_truncated) =
            Self::truncate_body(body_bytes, self.max_body_size);

        let request_id = Uuid::now_v7();
        let now = Utc::now();

        let captured_response = CapturedResponse {
            id: Uuid::now_v7(),
            request_id,
            timestamp: now,
            status: convert::status(parts.status),
            version: convert::version(parts.version),
            headers: convert::headers(&parts.headers),
            body: truncated_body,
            body_truncated,
            latency: std::time::Duration::from_millis(0),
            ttfb: std::time::Duration::from_millis(0),
        };

        CapturedExchange {
            request: CapturedRequest {
                id: request_id,
                session_id: self.session_id,
                connection_id: Uuid::now_v7(),
                sequence_number: 0,
                timestamp: now,
                method: http::Method::GET,
                uri: http::Uri::from_static("http://unknown"),
                version: convert::version(parts.version),
                headers: http::HeaderMap::new(),
                body: Bytes::new(),
                body_truncated: false,
                tls_info: None,
            },
            response: Some(captured_response),
        }
    }
}

#[async_trait]
impl HttpHandler for NetcapHandler {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let host = Self::extract_host(&req);
        let decision = self.filter.evaluate(&host);

        match decision {
            CaptureDecision::Capture(_) | CaptureDecision::Default => {
                let (parts, body) = req.into_parts();
                let body_bytes = hudsucker::hyper::body::to_bytes(body)
                    .await
                    .unwrap_or_default();

                if let Some(exchange) = self.capture_request(&parts, &body_bytes).await {
                    if let Err(e) = self.event_tx.try_send(exchange) {
                        tracing::warn!(
                            client_addr = %ctx.client_addr,
                            "Failed to send captured request: {}",
                            e
                        );
                    }
                }

                let req = Request::from_parts(parts, Body::from(body_bytes));
                RequestOrResponse::Request(req)
            }
            CaptureDecision::Passthrough => RequestOrResponse::Request(req),
        }
    }

    async fn handle_response(
        &mut self,
        ctx: &HttpContext,
        res: Response<Body>,
    ) -> Response<Body> {
        let (parts, body) = res.into_parts();
        let body_bytes = hudsucker::hyper::body::to_bytes(body)
            .await
            .unwrap_or_default();

        let exchange = self.capture_response(&parts, &body_bytes);
        if let Err(e) = self.event_tx.try_send(exchange) {
            tracing::warn!(
                client_addr = %ctx.client_addr,
                "Failed to send captured response: {}",
                e
            );
        }

        Response::from_parts(parts, Body::from(body_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::pattern::DomainPattern;
    use crate::filter::{DomainFilter, DomainMatcher, FilterRule, FilterType};

    fn make_handler(
        filter: Arc<dyn DomainMatcher>,
        buffer_size: usize,
    ) -> (NetcapHandler, mpsc::Receiver<CapturedExchange>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let handler = NetcapHandler::new(filter, tx, Uuid::now_v7(), 1024);
        (handler, rx)
    }

    fn include_filter(domain: &str) -> Arc<dyn DomainMatcher> {
        let mut filter = DomainFilter::new();
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: "include".to_string(),
            filter_type: FilterType::Include,
            pattern: DomainPattern::new_exact(domain),
            priority: 0,
            enabled: true,
        });
        Arc::new(filter)
    }

    fn exclude_filter(domain: &str) -> Arc<dyn DomainMatcher> {
        let mut filter = DomainFilter::new();
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: "exclude".to_string(),
            filter_type: FilterType::Exclude,
            pattern: DomainPattern::new_exact(domain),
            priority: 0,
            enabled: true,
        });
        Arc::new(filter)
    }

    #[test]
    fn truncate_body_no_truncation() {
        let (body, truncated) = NetcapHandler::truncate_body(b"hello", 10);
        assert_eq!(body, Bytes::from("hello"));
        assert!(!truncated);
    }

    #[test]
    fn truncate_body_with_truncation() {
        let (body, truncated) = NetcapHandler::truncate_body(b"hello world", 5);
        assert_eq!(body, Bytes::from("hello"));
        assert!(truncated);
    }

    #[test]
    fn extract_host_from_uri() {
        let req = Request::builder()
            .uri("http://example.com/path")
            .body(Body::empty())
            .unwrap();
        assert_eq!(NetcapHandler::extract_host(&req), "example.com");
    }

    #[test]
    fn extract_host_from_header() {
        let req = Request::builder()
            .uri("/path")
            .header("host", "example.com:443")
            .body(Body::empty())
            .unwrap();
        assert_eq!(NetcapHandler::extract_host(&req), "example.com");
    }

    #[test]
    fn extract_host_empty() {
        let req = Request::builder()
            .uri("/path")
            .body(Body::empty())
            .unwrap();
        assert_eq!(NetcapHandler::extract_host(&req), "");
    }

    #[test]
    fn filter_include_evaluates() {
        let filter = include_filter("example.com");
        assert!(matches!(
            filter.evaluate("example.com"),
            CaptureDecision::Capture(_)
        ));
    }

    #[test]
    fn filter_exclude_evaluates() {
        let filter = exclude_filter("example.com");
        assert!(matches!(
            filter.evaluate("example.com"),
            CaptureDecision::Passthrough
        ));
    }

    #[tokio::test]
    async fn capture_request_creates_exchange() {
        let filter: Arc<dyn DomainMatcher> = Arc::new(DomainFilter::new());
        let (handler, _rx) = make_handler(filter, 10);
        let req = Request::builder()
            .uri("http://example.com/api")
            .body(Body::from("test body"))
            .unwrap();
        let (parts, _body) = req.into_parts();
        let exchange = handler.capture_request(&parts, b"test body").await.unwrap();
        assert_eq!(exchange.request.body, Bytes::from("test body"));
        assert!(!exchange.request.body_truncated);
        assert!(exchange.response.is_none());
    }

    #[test]
    fn capture_response_creates_exchange() {
        let filter: Arc<dyn DomainMatcher> = Arc::new(DomainFilter::new());
        let (handler, _rx) = make_handler(filter, 10);
        let res = Response::builder()
            .status(200)
            .body(Body::from("resp"))
            .unwrap();
        let (parts, _body) = res.into_parts();
        let exchange = handler.capture_response(&parts, b"resp");
        assert!(exchange.response.is_some());
        let resp = exchange.response.unwrap();
        assert_eq!(resp.status, http::StatusCode::OK);
        assert_eq!(resp.body, Bytes::from("resp"));
    }

    #[test]
    fn capture_response_truncation() {
        let filter: Arc<dyn DomainMatcher> = Arc::new(DomainFilter::new());
        let (tx, _rx) = mpsc::channel(10);
        let handler = NetcapHandler::new(filter, tx, Uuid::now_v7(), 3);
        let res = Response::builder()
            .status(200)
            .body(Body::from("long response"))
            .unwrap();
        let (parts, _body) = res.into_parts();
        let exchange = handler.capture_response(&parts, b"long response");
        let resp = exchange.response.unwrap();
        assert_eq!(resp.body.len(), 3);
        assert!(resp.body_truncated);
    }

    #[test]
    fn convert_method() {
        assert_eq!(
            convert::method(&hudsucker::hyper::Method::POST),
            http::Method::POST
        );
    }

    #[test]
    fn convert_status() {
        assert_eq!(
            convert::status(hudsucker::hyper::StatusCode::NOT_FOUND),
            http::StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn convert_version() {
        assert_eq!(
            convert::version(hudsucker::hyper::Version::HTTP_2),
            http::Version::HTTP_2
        );
    }
}
