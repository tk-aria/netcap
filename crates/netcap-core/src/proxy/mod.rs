pub mod connection;
pub mod handler;

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::filter::DomainFilter;
use crate::storage::StorageBackend;
use crate::tls::CertificateProvider;

pub struct ProxyServer {
    config: ProxyConfig,
    _cert_provider: Arc<dyn CertificateProvider>,
    _domain_filter: Arc<DomainFilter>,
    _storage: Arc<dyn StorageBackend>,
    shutdown_tx: broadcast::Sender<()>,
}

pub struct ProxyServerBuilder {
    config: ProxyConfig,
    cert_provider: Option<Arc<dyn CertificateProvider>>,
    domain_filter: Option<Arc<DomainFilter>>,
    storage: Option<Arc<dyn StorageBackend>>,
}

impl ProxyServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ProxyConfig::default(),
            cert_provider: None,
            domain_filter: None,
            storage: None,
        }
    }

    pub fn config(mut self, config: ProxyConfig) -> Self {
        self.config = config;
        self
    }

    pub fn cert_provider(mut self, provider: Arc<dyn CertificateProvider>) -> Self {
        self.cert_provider = Some(provider);
        self
    }

    pub fn domain_filter(mut self, filter: Arc<DomainFilter>) -> Self {
        self.domain_filter = Some(filter);
        self
    }

    pub fn storage(mut self, storage: Arc<dyn StorageBackend>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn build(self) -> Result<ProxyServer, ProxyError> {
        let (shutdown_tx, _) = broadcast::channel(1);
        Ok(ProxyServer {
            config: self.config,
            _cert_provider: self.cert_provider.ok_or(ProxyError::NotRunning)?,
            _domain_filter: self
                .domain_filter
                .unwrap_or_else(|| Arc::new(DomainFilter::new())),
            _storage: self.storage.ok_or(ProxyError::NotRunning)?,
            shutdown_tx,
        })
    }
}

impl Default for ProxyServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyServer {
    pub fn builder() -> ProxyServerBuilder {
        ProxyServerBuilder::new()
    }

    pub fn listen_addr(&self) -> std::net::SocketAddr {
        self.config.listen_addr
    }

    pub async fn run(&self) -> Result<(), ProxyError> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tracing::info!("Proxy starting on {}", self.config.listen_addr);
        let _ = shutdown_rx.recv().await;
        tracing::info!("Proxy shutting down");
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), ProxyError> {
        self.shutdown_tx
            .send(())
            .map_err(|_| ProxyError::NotRunning)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::exchange::CapturedExchange;
    use crate::error::StorageError;
    use crate::tls::ca::RcgenCaProvider;
    use crate::tls::CertificateProvider;
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockStorage;

    #[async_trait]
    impl StorageBackend for MockStorage {
        async fn initialize(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn write(&self, _: &CapturedExchange) -> Result<(), StorageError> {
            Ok(())
        }
        async fn write_batch(&self, _: &[CapturedExchange]) -> Result<(), StorageError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), StorageError> {
            Ok(())
        }
        async fn close(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    fn mock_cert_provider() -> Arc<dyn CertificateProvider> {
        let tmp = tempfile::TempDir::new().unwrap();
        Arc::new(RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap())
    }

    #[test]
    fn builder_success() {
        let server = ProxyServer::builder()
            .cert_provider(mock_cert_provider())
            .storage(Arc::new(MockStorage))
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn builder_missing_cert_provider() {
        let result = ProxyServer::builder()
            .storage(Arc::new(MockStorage))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_missing_storage() {
        let result = ProxyServer::builder()
            .cert_provider(mock_cert_provider())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_with_custom_config() {
        let mut config = ProxyConfig::default();
        config.listen_addr = "0.0.0.0:9090".parse().unwrap();
        let server = ProxyServer::builder()
            .config(config)
            .cert_provider(mock_cert_provider())
            .storage(Arc::new(MockStorage))
            .build()
            .unwrap();
        assert_eq!(server.listen_addr().to_string(), "0.0.0.0:9090");
    }

    #[tokio::test]
    async fn shutdown_sends_signal() {
        let server = ProxyServer::builder()
            .cert_provider(mock_cert_provider())
            .storage(Arc::new(MockStorage))
            .build()
            .unwrap();
        // subscribe so there's an active receiver
        let mut rx = server.shutdown_tx.subscribe();
        assert!(server.shutdown().is_ok());
        assert!(rx.recv().await.is_ok());
    }
}
