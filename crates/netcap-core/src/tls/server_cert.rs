use rcgen::{CertificateParams, DistinguishedName, DnType, Issuer, KeyPair, SanType};

use crate::error::CertError;
use crate::tls::ServerCertificate;

pub fn issue_server_certificate(
    domain: &str,
    _ca_cert_pem: &str,
    ca_key_pem: &str,
) -> Result<ServerCertificate, CertError> {
    // Parse CA key
    let ca_key = KeyPair::from_pem(ca_key_pem).map_err(|e| CertError::ServerCertFailed {
        domain: domain.to_string(),
        source: Box::new(e),
    })?;

    // Reconstruct CA params for issuer
    let mut ca_params = CertificateParams::default();
    ca_params.distinguished_name = {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "netcap CA");
        dn.push(DnType::OrganizationName, "netcap");
        dn
    };
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    // Create issuer from CA params and key
    let issuer = Issuer::from_params(&ca_params, &ca_key);

    // Create server certificate params
    let mut server_params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domain);
    server_params.distinguished_name = dn;
    server_params.subject_alt_names = vec![SanType::DnsName(domain.try_into().map_err(
        |e: rcgen::Error| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        },
    )?)];

    // Generate server key pair
    let server_key =
        KeyPair::generate().map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;

    // Sign the server certificate with the CA
    let server_cert = server_params
        .signed_by(&server_key, &issuer)
        .map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;

    Ok(ServerCertificate {
        cert_der: server_cert.der().to_vec(),
        key_der: server_key.serialize_der(),
        domain: domain.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::ca::RcgenCaProvider;
    use crate::tls::CertificateProvider;
    use tempfile::TempDir;

    fn setup_ca() -> (String, String) {
        let tmp = TempDir::new().unwrap();
        let provider = RcgenCaProvider::generate_ca("Test CA", tmp.path()).unwrap();
        let ca = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { provider.get_or_create_ca().await.unwrap() });
        (ca.cert_pem, ca.key_pem)
    }

    #[test]
    fn issue_server_certificate_success() {
        let (ca_cert, ca_key) = setup_ca();
        let cert = issue_server_certificate("example.com", &ca_cert, &ca_key).unwrap();
        assert_eq!(cert.domain, "example.com");
        assert!(!cert.cert_der.is_empty());
        assert!(!cert.key_der.is_empty());
    }

    #[test]
    fn issue_wildcard_certificate() {
        let (ca_cert, ca_key) = setup_ca();
        let cert = issue_server_certificate("*.example.com", &ca_cert, &ca_key).unwrap();
        assert_eq!(cert.domain, "*.example.com");
        assert!(!cert.cert_der.is_empty());
    }

    #[test]
    fn issue_certificate_invalid_ca_key() {
        let result = issue_server_certificate("example.com", "not-a-pem", "not-a-key");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("example.com"));
    }

    #[test]
    fn issue_multiple_domains() {
        let (ca_cert, ca_key) = setup_ca();
        let cert1 = issue_server_certificate("a.example.com", &ca_cert, &ca_key).unwrap();
        let cert2 = issue_server_certificate("b.example.com", &ca_cert, &ca_key).unwrap();
        assert_eq!(cert1.domain, "a.example.com");
        assert_eq!(cert2.domain, "b.example.com");
        assert_ne!(cert1.cert_der, cert2.cert_der);
    }
}
