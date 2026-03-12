pub mod connection;
pub mod handler;

use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::filter::DomainFilter;
use crate::storage::buffer::CaptureBuffer;
use crate::storage::dispatcher::StorageDispatcher;
use crate::storage::StorageBackend;
use crate::tls::ca::RcgenCaProvider;
use crate::tls::CertificateProvider;

use handler::NetcapHandler;

pub struct ProxyServer {
    config: ProxyConfig,
    cert_provider: Arc<dyn CertificateProvider>,
    domain_filter: Arc<DomainFilter>,
    storage: Arc<dyn StorageBackend>,
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
            cert_provider: self.cert_provider.ok_or(ProxyError::NotRunning)?,
            domain_filter: self
                .domain_filter
                .unwrap_or_else(|| Arc::new(DomainFilter::new())),
            storage: self.storage.ok_or(ProxyError::NotRunning)?,
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
        use hudsucker::certificate_authority::RcgenAuthority;

        // Get CA certificate from provider
        let ca = self
            .cert_provider
            .get_or_create_ca()
            .await
            .map_err(|e| ProxyError::UpstreamConnection(format!("CA error: {}", e)))?;

        // Convert PEM to DER for hudsucker's RcgenAuthority
        let cert_der = RcgenCaProvider::pem_to_der(&ca.cert_pem);
        let key_der = RcgenCaProvider::pem_to_der(&ca.key_pem);

        let private_key = hudsucker::rustls::PrivateKey(key_der);
        let ca_cert = hudsucker::rustls::Certificate(cert_der);

        let authority = RcgenAuthority::new(private_key, ca_cert, 1000)
            .map_err(|e| ProxyError::UpstreamConnection(format!("RcgenAuthority: {}", e)))?;

        // Create capture buffer & dispatcher
        let (buffer_tx, buffer_rx) = CaptureBuffer::new(self.config.max_connections);
        let mut dispatcher = StorageDispatcher::new(
            vec![Arc::clone(&self.storage)],
            buffer_rx,
            100,
            std::time::Duration::from_millis(100),
        );

        // Create handler
        let session_id = Uuid::now_v7();
        let netcap_handler = NetcapHandler::new(
            self.domain_filter.clone(),
            buffer_tx.into_inner(),
            session_id,
            self.config.max_body_size,
        );

        // Build hudsucker proxy
        let proxy = hudsucker::Proxy::builder()
            .with_addr(self.config.listen_addr)
            .with_rustls_client()
            .with_ca(authority)
            .with_http_handler(netcap_handler)
            .build();

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tracing::info!("Proxy starting on {}", self.config.listen_addr);

        // Run dispatcher in background
        let dispatcher_handle = tokio::spawn(async move {
            dispatcher.run().await;
        });

        // Run proxy with shutdown signal
        let proxy_result = proxy
            .start(async move {
                let _ = shutdown_rx.recv().await;
            })
            .await;

        // Wait for dispatcher to finish
        let _ = dispatcher_handle.await;

        tracing::info!("Proxy shut down");

        proxy_result.map_err(|e| ProxyError::UpstreamConnection(format!("Proxy error: {}", e)))
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
        let mut rx = server.shutdown_tx.subscribe();
        assert!(server.shutdown().is_ok());
        assert!(rx.recv().await.is_ok());
    }

    #[tokio::test]
    async fn run_and_shutdown() {
        let mut config = ProxyConfig::default();
        config.listen_addr = "127.0.0.1:0".parse().unwrap();
        let server = Arc::new(
            ProxyServer::builder()
                .config(config)
                .cert_provider(mock_cert_provider())
                .storage(Arc::new(MockStorage))
                .build()
                .unwrap(),
        );
        let server_clone = Arc::clone(&server);
        let handle = tokio::spawn(async move { server_clone.run().await });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(server.shutdown().is_ok());
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
