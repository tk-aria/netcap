use netcap_core::capture::exchange::CapturedExchange;
use serde::Serialize;

pub struct FfiProxyConfig {
    pub listen_port: u16,
    pub storage_path: String,
    pub include_domains: Vec<String>,
    pub exclude_domains: Vec<String>,
}

#[derive(Debug)]
pub struct FfiCaptureStats {
    pub total_requests: u64,
    pub total_responses: u64,
    pub active_connections: u32,
    pub bytes_captured: u64,
}

impl Default for FfiCaptureStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_responses: 0,
            active_connections: 0,
            bytes_captured: 0,
        }
    }
}

#[derive(Serialize)]
struct ExchangeEvent {
    id: String,
    method: String,
    url: String,
    status: Option<u16>,
    request_headers: Vec<(String, String)>,
    response_headers: Vec<(String, String)>,
    timestamp: String,
}

pub fn exchanges_to_json(exchanges: &[CapturedExchange], offset: u64, limit: u64) -> String {
    let events: Vec<ExchangeEvent> = exchanges
        .iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|ex| {
            let req = &ex.request;
            let resp = &ex.response;
            ExchangeEvent {
                id: req.id.to_string(),
                method: req.method.to_string(),
                url: req.uri.to_string(),
                status: resp.as_ref().map(|r| r.status.as_u16()),
                request_headers: req
                    .headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect(),
                response_headers: resp
                    .as_ref()
                    .map(|r| {
                        r.headers
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                timestamp: req.timestamp.to_rfc3339(),
            }
        })
        .collect();

    serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stats_zeroed() {
        let stats = FfiCaptureStats::default();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_responses, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.bytes_captured, 0);
    }

    #[test]
    fn config_fields() {
        let config = FfiProxyConfig {
            listen_port: 8080,
            storage_path: "/tmp/test".into(),
            include_domains: vec!["*.example.com".into()],
            exclude_domains: vec![],
        };
        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.include_domains.len(), 1);
    }

    #[test]
    fn exchanges_to_json_empty() {
        let result = exchanges_to_json(&[], 0, 10);
        assert_eq!(result, "[]");
    }

    #[test]
    fn exchanges_to_json_with_offset() {
        let result = exchanges_to_json(&[], 5, 10);
        assert_eq!(result, "[]");
    }
}
