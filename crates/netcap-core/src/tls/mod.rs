pub mod ca;
pub mod server_cert;
pub mod store;

use async_trait::async_trait;

use crate::error::CertError;

pub struct CaCertificate {
    pub cert_pem: String,
    pub key_pem: String,
    pub fingerprint_sha256: String,
}

#[derive(Debug, Clone)]
pub struct ServerCertificate {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
    pub domain: String,
}

#[async_trait]
pub trait CertificateProvider: Send + Sync + 'static {
    async fn get_or_create_ca(&self) -> Result<CaCertificate, CertError>;
    async fn issue_server_cert(&self, domain: &str) -> Result<ServerCertificate, CertError>;
    async fn export_ca_pem(&self, path: &std::path::Path) -> Result<(), CertError>;
}
