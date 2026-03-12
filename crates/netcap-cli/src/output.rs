use netcap_core::capture::exchange::CapturedExchange;

pub fn format_exchange(exchange: &CapturedExchange) -> String {
    let req = &exchange.request;
    let status = exchange
        .response
        .as_ref()
        .map(|r| r.status.as_u16().to_string())
        .unwrap_or_else(|| "---".to_string());
    let latency = exchange
        .response
        .as_ref()
        .map(|r| format!("{:.1}ms", r.latency.as_secs_f64() * 1000.0))
        .unwrap_or_else(|| "---".to_string());
    let host = req.uri.host().unwrap_or("-");
    let path = req.uri.path();
    let query = req
        .uri
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    format!(
        "{} {}{}{} → {} ({})",
        req.method, host, path, query, status, latency
    )
}

pub fn colorized_status(status: u16) -> &'static str {
    match status {
        200..=299 => "\x1b[32m", // green
        300..=399 => "\x1b[33m", // yellow
        400..=499 => "\x1b[31m", // red
        500..=599 => "\x1b[35m", // magenta
        _ => "\x1b[0m",
    }
}

pub fn format_exchange_colored(exchange: &CapturedExchange) -> String {
    let req = &exchange.request;
    let (status_str, color, reset) = match &exchange.response {
        Some(r) => {
            let code = r.status.as_u16();
            (
                code.to_string(),
                colorized_status(code),
                "\x1b[0m",
            )
        }
        None => ("---".to_string(), "", ""),
    };
    let latency = exchange
        .response
        .as_ref()
        .map(|r| format!("{:.1}ms", r.latency.as_secs_f64() * 1000.0))
        .unwrap_or_else(|| "---".to_string());
    let host = req.uri.host().unwrap_or("-");
    let path = req.uri.path();
    let query = req
        .uri
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    format!(
        "{} {}{}{} → {}{}{} ({})",
        req.method, host, path, query, color, status_str, reset, latency
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use netcap_core::capture::exchange::{CapturedRequest, CapturedResponse};
    use uuid::Uuid;

    fn make_request(uri: &str) -> CapturedRequest {
        CapturedRequest {
            id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            connection_id: Uuid::now_v7(),
            sequence_number: 0,
            timestamp: Utc::now(),
            method: Method::GET,
            uri: uri.parse().unwrap(),
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            body_truncated: false,
            tls_info: None,
        }
    }

    fn make_response(status: u16, latency_ms: u64) -> CapturedResponse {
        CapturedResponse {
            id: Uuid::now_v7(),
            request_id: Uuid::now_v7(),
            timestamp: Utc::now(),
            status: StatusCode::from_u16(status).unwrap(),
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            body_truncated: false,
            latency: std::time::Duration::from_millis(latency_ms),
            ttfb: std::time::Duration::from_millis(latency_ms / 2),
        }
    }

    #[test]
    fn format_get_request_with_response() {
        let exchange = CapturedExchange {
            request: make_request("http://example.com/api"),
            response: Some(make_response(200, 12)),
        };
        let out = format_exchange(&exchange);
        assert!(out.contains("GET"));
        assert!(out.contains("example.com"));
        assert!(out.contains("/api"));
        assert!(out.contains("200"));
    }

    #[test]
    fn format_no_response() {
        let exchange = CapturedExchange {
            request: make_request("http://example.com/api"),
            response: None,
        };
        let out = format_exchange(&exchange);
        assert!(out.contains("→ --- (---)"));
    }

    #[test]
    fn format_query_params() {
        let exchange = CapturedExchange {
            request: make_request("http://example.com/search?q=hello&page=2"),
            response: Some(make_response(200, 50)),
        };
        let out = format_exchange(&exchange);
        assert!(out.contains("?q=hello&page=2"));
    }
}
