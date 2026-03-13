use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::config::ProxyConfig;
use netcap_core::filter::{DomainFilter, DomainMatcher, FilterRule, FilterType};
use netcap_core::filter::pattern::DomainPattern;
use netcap_core::proxy::ProxyServer;
use netcap_core::storage::StorageBackend;
use netcap_core::tls::ca::RcgenCaProvider;
use netcap_core::tls::CertificateProvider;

use crate::error::FfiError;
use crate::types::{exchanges_to_json, FfiCaptureStats, FfiProxyConfig};

/// Thread-safe exchange collector for stats & event queries
#[derive(Debug)]
struct ExchangeCollector {
    exchanges: Mutex<Vec<CapturedExchange>>,
    stats: Mutex<FfiCaptureStats>,
}

impl ExchangeCollector {
    fn new() -> Self {
        Self {
            exchanges: Mutex::new(Vec::new()),
            stats: Mutex::new(FfiCaptureStats::default()),
        }
    }
}

#[async_trait]
impl StorageBackend for ExchangeCollector {
    async fn initialize(&mut self) -> Result<(), netcap_core::error::StorageError> {
        Ok(())
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), netcap_core::error::StorageError> {
        let mut exs = self.exchanges.lock().unwrap();
        exs.push(exchange.clone());

        let mut stats = self.stats.lock().unwrap();
        stats.total_requests += 1;
        if exchange.response.is_some() {
            stats.total_responses += 1;
        }
        let req_size = exchange.request.body.len() as u64;
        let resp_size = exchange
            .response
            .as_ref()
            .map(|r| r.body.len())
            .unwrap_or(0) as u64;
        stats.bytes_captured += req_size + resp_size;

        Ok(())
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), netcap_core::error::StorageError> {
        for ex in exchanges {
            self.write(ex).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), netcap_core::error::StorageError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), netcap_core::error::StorageError> {
        Ok(())
    }
}

pub struct NetcapProxy {
    runtime: tokio::runtime::Runtime,
    server: Option<Arc<ProxyServer>>,
    collector: Arc<ExchangeCollector>,
    ca_pem: String,
    is_running: Mutex<bool>,
}

impl NetcapProxy {
    pub fn new(config: FfiProxyConfig) -> Result<Self, FfiError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| FfiError::InitFailed(e.to_string()))?;

        let listen_addr: SocketAddr = format!("127.0.0.1:{}", config.listen_port)
            .parse()
            .map_err(|e: std::net::AddrParseError| FfiError::InitFailed(e.to_string()))?;

        let mut proxy_config = ProxyConfig::default();
        proxy_config.listen_addr = listen_addr;

        // Generate CA certificate
        let storage_path = PathBuf::from(&config.storage_path);
        let cert_dir = storage_path.join("certs");
        std::fs::create_dir_all(&cert_dir)
            .map_err(|e| FfiError::CertError(e.to_string()))?;

        let ca_provider = RcgenCaProvider::generate_ca("netcap FFI CA", &cert_dir)
            .map_err(|e| FfiError::CertError(e.to_string()))?;
        let ca_pem = ca_provider.ca_cert_pem().to_string();

        // Set up domain filter
        let mut domain_filter = DomainFilter::new();
        for pattern in &config.include_domains {
            let rule = FilterRule {
                id: uuid::Uuid::now_v7(),
                name: format!("include_{}", pattern),
                filter_type: FilterType::Include,
                pattern: DomainPattern::new_wildcard(pattern),
                priority: 0,
                enabled: true,
            };
            domain_filter.add_rule(rule);
        }
        for pattern in &config.exclude_domains {
            let rule = FilterRule {
                id: uuid::Uuid::now_v7(),
                name: format!("exclude_{}", pattern),
                filter_type: FilterType::Exclude,
                pattern: DomainPattern::new_wildcard(pattern),
                priority: 0,
                enabled: true,
            };
            domain_filter.add_rule(rule);
        }

        // Create exchange collector as storage
        let collector = Arc::new(ExchangeCollector::new());

        // Build proxy server
        let server = ProxyServer::builder()
            .config(proxy_config)
            .cert_provider(Arc::new(ca_provider) as Arc<dyn CertificateProvider>)
            .domain_filter(Arc::new(domain_filter))
            .storage(collector.clone() as Arc<dyn StorageBackend>)
            .build()
            .map_err(|e| FfiError::InitFailed(e.to_string()))?;

        Ok(Self {
            runtime,
            server: Some(Arc::new(server)),
            collector,
            ca_pem,
            is_running: Mutex::new(false),
        })
    }

    pub fn start(&self) -> Result<(), FfiError> {
        let mut running = self.is_running.lock().unwrap();
        if *running {
            return Err(FfiError::AlreadyRunning);
        }

        let server = self
            .server
            .as_ref()
            .ok_or_else(|| FfiError::ProxyError("Server not initialized".into()))?
            .clone();

        self.runtime.spawn(async move {
            if let Err(e) = server.run().await {
                tracing::error!("Proxy error: {}", e);
            }
        });

        *running = true;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), FfiError> {
        let mut running = self.is_running.lock().unwrap();
        if !*running {
            return Err(FfiError::NotRunning);
        }

        let server = self
            .server
            .as_ref()
            .ok_or_else(|| FfiError::ProxyError("Server not initialized".into()))?;

        server
            .shutdown()
            .map_err(|e| FfiError::ProxyError(e.to_string()))?;

        *running = false;
        Ok(())
    }

    pub fn get_ca_certificate_pem(&self) -> Result<String, FfiError> {
        Ok(self.ca_pem.clone())
    }

    pub fn get_stats(&self) -> Result<FfiCaptureStats, FfiError> {
        let stats = self.collector.stats.lock().unwrap();
        Ok(FfiCaptureStats {
            total_requests: stats.total_requests,
            total_responses: stats.total_responses,
            active_connections: stats.active_connections,
            bytes_captured: stats.bytes_captured,
        })
    }

    pub fn get_capture_events(&self, offset: u64, limit: u64) -> Result<String, FfiError> {
        let exchanges = self.collector.exchanges.lock().unwrap();
        Ok(exchanges_to_json(&exchanges, offset, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FfiProxyConfig {
        let tmp = tempfile::TempDir::new().unwrap();
        FfiProxyConfig {
            listen_port: 0, // OS will pick a free port
            storage_path: tmp.path().to_string_lossy().to_string(),
            include_domains: vec![],
            exclude_domains: vec![],
        }
    }

    #[test]
    fn new_creates_proxy() {
        let config = test_config();
        let proxy = NetcapProxy::new(config);
        assert!(proxy.is_ok());
    }

    #[test]
    fn get_ca_pem_returns_certificate() {
        let config = test_config();
        let proxy = NetcapProxy::new(config).unwrap();
        let pem = proxy.get_ca_certificate_pem().unwrap();
        assert!(pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn get_stats_returns_zeroed() {
        let config = test_config();
        let proxy = NetcapProxy::new(config).unwrap();
        let stats = proxy.get_stats().unwrap();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_responses, 0);
    }

    #[test]
    fn get_capture_events_returns_empty() {
        let config = test_config();
        let proxy = NetcapProxy::new(config).unwrap();
        let events = proxy.get_capture_events(0, 10).unwrap();
        assert_eq!(events, "[]");
    }

    #[test]
    fn stop_without_start_returns_not_running() {
        let config = test_config();
        let proxy = NetcapProxy::new(config).unwrap();
        let result = proxy.stop();
        assert!(result.is_err());
        if let Err(FfiError::NotRunning) = result {
            // expected
        } else {
            panic!("Expected NotRunning error");
        }
    }

    #[test]
    fn new_with_domain_filters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = FfiProxyConfig {
            listen_port: 0,
            storage_path: tmp.path().to_string_lossy().to_string(),
            include_domains: vec!["*.example.com".into()],
            exclude_domains: vec!["*.ads.com".into()],
        };
        let proxy = NetcapProxy::new(config);
        assert!(proxy.is_ok());
    }

    #[test]
    fn start_then_stop_lifecycle() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = FfiProxyConfig {
            listen_port: 0,
            storage_path: tmp.path().to_string_lossy().to_string(),
            include_domains: vec![],
            exclude_domains: vec![],
        };
        let proxy = NetcapProxy::new(config).unwrap();
        assert!(proxy.start().is_ok());
        // Allow proxy to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(proxy.stop().is_ok());
    }

    #[test]
    fn double_start_returns_already_running() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = FfiProxyConfig {
            listen_port: 0,
            storage_path: tmp.path().to_string_lossy().to_string(),
            include_domains: vec![],
            exclude_domains: vec![],
        };
        let proxy = NetcapProxy::new(config).unwrap();
        assert!(proxy.start().is_ok());
        let result = proxy.start();
        assert!(result.is_err());
        if let Err(FfiError::AlreadyRunning) = result {
            // expected
        } else {
            panic!("Expected AlreadyRunning error");
        }
        // Cleanup
        let _ = proxy.stop();
    }
}
