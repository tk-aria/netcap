use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("proxy error: {0}")]
    Proxy(#[from] ProxyError),

    #[error("TLS error: {0}")]
    Tls(#[from] CertError),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("filter error: {0}")]
    Filter(#[from] FilterError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("bind failed on {addr}: {source}")]
    BindFailed {
        addr: std::net::SocketAddr,
        source: std::io::Error,
    },

    #[error("upstream connection failed: {0}")]
    UpstreamConnection(String),

    #[error("proxy already running")]
    AlreadyRunning,

    #[error("proxy not running")]
    NotRunning,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("initialization failed: {0}")]
    InitFailed(String),

    #[error("write failed: {0}")]
    WriteFailed(String),

    #[error("flush failed: {0}")]
    FlushFailed(String),

    #[error("connection lost: {0}")]
    ConnectionLost(String),
}

#[derive(Debug, Error)]
pub enum CertError {
    #[error("CA generation failed: {0}")]
    CaGenerationFailed(String),

    #[error("server cert failed for {domain}: {source}")]
    ServerCertFailed {
        domain: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("cert store access failed: {0}")]
    StoreAccessFailed(String),
}

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("regex compile error: {0}")]
    RegexError(#[from] regex::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_error_display() {
        let err = ProxyError::AlreadyRunning;
        assert_eq!(err.to_string(), "proxy already running");
    }

    #[test]
    fn proxy_error_bind_failed_display() {
        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let io_err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "in use");
        let err = ProxyError::BindFailed {
            addr,
            source: io_err,
        };
        assert!(err.to_string().contains("127.0.0.1:8080"));
        assert!(err.to_string().contains("in use"));
    }

    #[test]
    fn storage_error_display() {
        let err = StorageError::InitFailed("db not found".into());
        assert_eq!(err.to_string(), "initialization failed: db not found");
    }

    #[test]
    fn cert_error_display() {
        let err = CertError::CaGenerationFailed("key too short".into());
        assert_eq!(err.to_string(), "CA generation failed: key too short");
    }

    #[test]
    fn cert_error_server_cert_failed() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let err = CertError::ServerCertFailed {
            domain: "example.com".into(),
            source: Box::new(inner),
        };
        assert!(err.to_string().contains("example.com"));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn filter_error_display() {
        let err = FilterError::InvalidPattern("bad pattern".into());
        assert_eq!(err.to_string(), "invalid pattern: bad pattern");
    }

    #[test]
    fn filter_error_from_regex() {
        let regex_err = regex::Regex::new("[invalid").unwrap_err();
        let err = FilterError::from(regex_err);
        assert!(err.to_string().contains("regex compile error"));
    }

    #[test]
    fn from_proxy_to_capture() {
        let proxy_err = ProxyError::AlreadyRunning;
        let capture_err = CaptureError::from(proxy_err);
        assert!(capture_err.to_string().contains("proxy error"));
    }

    #[test]
    fn from_storage_to_capture() {
        let storage_err = StorageError::WriteFailed("disk full".into());
        let capture_err = CaptureError::from(storage_err);
        assert!(capture_err.to_string().contains("storage error"));
    }

    #[test]
    fn server_cert_failed_source_chain() {
        use std::error::Error;
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "root cause");
        let err = CertError::ServerCertFailed {
            domain: "test.com".into(),
            source: Box::new(inner),
        };
        let source = err.source().unwrap();
        assert!(source.to_string().contains("root cause"));
    }
}
