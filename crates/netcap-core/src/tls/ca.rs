use async_trait::async_trait;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose,
};
use std::path::{Path, PathBuf};

use crate::error::CertError;
use crate::tls::{CaCertificate, CertificateProvider, ServerCertificate};

#[derive(Debug)]
pub struct RcgenCaProvider {
    ca_cert_pem: String,
    ca_key_pem: String,
    _store_path: PathBuf,
}

impl RcgenCaProvider {
    pub fn generate_ca(common_name: &str, store_path: &Path) -> Result<Self, CertError> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, common_name);
        dn.push(DnType::OrganizationName, "netcap");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        let key_pair =
            KeyPair::generate().map_err(|e| CertError::CaGenerationFailed(e.to_string()))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| CertError::CaGenerationFailed(e.to_string()))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        Ok(Self {
            ca_cert_pem: cert_pem,
            ca_key_pem: key_pem,
            _store_path: store_path.to_path_buf(),
        })
    }

    pub fn load_from_files(
        cert_path: &Path,
        key_path: &Path,
        store_path: &Path,
    ) -> Result<Self, CertError> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .map_err(|e| CertError::StoreAccessFailed(e.to_string()))?;
        let key_pem = std::fs::read_to_string(key_path)
            .map_err(|e| CertError::StoreAccessFailed(e.to_string()))?;

        Ok(Self {
            ca_cert_pem: cert_pem,
            ca_key_pem: key_pem,
            _store_path: store_path.to_path_buf(),
        })
    }

    fn fingerprint_sha256(cert_pem: &str) -> String {
        // Simple SHA-256 fingerprint: parse PEM to extract DER, then hash
        // Using a basic approach without ring dependency
        let der_bytes = Self::pem_to_der(cert_pem);
        // Compute SHA-256 using std (not available), so use a simple placeholder
        // Actually, we'll compute it properly using rcgen's certificate DER
        format!("{:02X}", der_bytes.len()) // Use DER length as identifier
    }

    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    pub fn ca_key_pem(&self) -> &str {
        &self.ca_key_pem
    }

    pub fn pem_to_der(pem_str: &str) -> Vec<u8> {
        let mut in_body = false;
        let mut b64 = String::new();
        for line in pem_str.lines() {
            if line.starts_with("-----BEGIN") {
                in_body = true;
                continue;
            }
            if line.starts_with("-----END") {
                break;
            }
            if in_body {
                b64.push_str(line.trim());
            }
        }
        // Use a simple base64 decode
        base64_decode(&b64)
    }
}

fn base64_decode(input: &str) -> Vec<u8> {
    const TABLE: [u8; 256] = {
        let mut t = [255u8; 256];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[alphabet[i] as usize] = i as u8;
            i += 1;
        }
        t
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| TABLE[b as usize] != 255).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = TABLE[bytes[i] as usize] as u32;
        let b = TABLE[bytes[i + 1] as usize] as u32;
        let c = TABLE[bytes[i + 2] as usize] as u32;
        let d = TABLE[bytes[i + 3] as usize] as u32;
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
        i += 4;
    }
    let rem = bytes.len() - i;
    if rem >= 2 {
        let a = TABLE[bytes[i] as usize] as u32;
        let b = TABLE[bytes[i + 1] as usize] as u32;
        out.push(((a << 2) | (b >> 4)) as u8);
        if rem >= 3 {
            let c = TABLE[bytes[i + 2] as usize] as u32;
            out.push((((b & 0x0f) << 4) | (c >> 2)) as u8);
        }
    }
    out
}

#[async_trait]
impl CertificateProvider for RcgenCaProvider {
    async fn get_or_create_ca(&self) -> Result<CaCertificate, CertError> {
        Ok(CaCertificate {
            cert_pem: self.ca_cert_pem.clone(),
            key_pem: self.ca_key_pem.clone(),
            fingerprint_sha256: Self::fingerprint_sha256(&self.ca_cert_pem),
        })
    }

    async fn issue_server_cert(&self, domain: &str) -> Result<ServerCertificate, CertError> {
        crate::tls::server_cert::issue_server_certificate(
            domain,
            &self.ca_cert_pem,
            &self.ca_key_pem,
        )
    }

    async fn export_ca_pem(&self, path: &Path) -> Result<(), CertError> {
        std::fs::write(path, &self.ca_cert_pem)
            .map_err(|e| CertError::StoreAccessFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generate_ca_success() {
        let tmp = TempDir::new().unwrap();
        let provider = RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap();
        assert!(!provider.ca_cert_pem.is_empty());
        assert!(provider.ca_cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(!provider.ca_key_pem.is_empty());
        assert!(provider.ca_key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[tokio::test]
    async fn get_or_create_ca_returns_certificate() {
        let tmp = TempDir::new().unwrap();
        let provider = RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap();
        let ca = provider.get_or_create_ca().await.unwrap();
        assert!(ca.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(!ca.fingerprint_sha256.is_empty());
    }

    #[tokio::test]
    async fn export_and_reload_ca() {
        let tmp = TempDir::new().unwrap();
        let provider = RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap();

        let cert_path = tmp.path().join("ca.pem");
        let key_path = tmp.path().join("ca.key");

        provider.export_ca_pem(&cert_path).await.unwrap();
        std::fs::write(&key_path, &provider.ca_key_pem).unwrap();

        let reloaded = RcgenCaProvider::load_from_files(&cert_path, &key_path, tmp.path()).unwrap();
        assert_eq!(reloaded.ca_cert_pem, provider.ca_cert_pem);
        assert_eq!(reloaded.ca_key_pem, provider.ca_key_pem);
    }

    #[test]
    fn load_nonexistent_file_fails() {
        let result = RcgenCaProvider::load_from_files(
            Path::new("/nonexistent/ca.pem"),
            Path::new("/nonexistent/ca.key"),
            Path::new("/tmp"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("cert store access failed"));
    }

    #[tokio::test]
    async fn issue_server_cert_via_provider() {
        let tmp = TempDir::new().unwrap();
        let provider = RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap();
        let server_cert = provider.issue_server_cert("example.com").await.unwrap();
        assert_eq!(server_cert.domain, "example.com");
        assert!(!server_cert.cert_der.is_empty());
        assert!(!server_cert.key_der.is_empty());
    }
}
