use std::fmt;

#[derive(Debug, Clone)]
pub enum FfiError {
    InitFailed(String),
    ProxyError(String),
    AlreadyRunning,
    NotRunning,
    StorageError(String),
    CertError(String),
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiError::InitFailed(msg) => write!(f, "Init failed: {}", msg),
            FfiError::ProxyError(msg) => write!(f, "Proxy error: {}", msg),
            FfiError::AlreadyRunning => write!(f, "Proxy is already running"),
            FfiError::NotRunning => write!(f, "Proxy is not running"),
            FfiError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            FfiError::CertError(msg) => write!(f, "Certificate error: {}", msg),
        }
    }
}

impl std::error::Error for FfiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_init_failed() {
        let err = FfiError::InitFailed("bad port".into());
        assert!(err.to_string().contains("Init failed"));
        assert!(err.to_string().contains("bad port"));
    }

    #[test]
    fn error_display_already_running() {
        let err = FfiError::AlreadyRunning;
        assert!(err.to_string().contains("already running"));
    }

    #[test]
    fn error_display_not_running() {
        let err = FfiError::NotRunning;
        assert!(err.to_string().contains("not running"));
    }

    #[test]
    fn error_display_proxy_error() {
        let err = FfiError::ProxyError("bind failed".into());
        assert!(err.to_string().contains("Proxy error"));
    }

    #[test]
    fn error_display_storage_error() {
        let err = FfiError::StorageError("disk full".into());
        assert!(err.to_string().contains("Storage error"));
    }

    #[test]
    fn error_display_cert_error() {
        let err = FfiError::CertError("invalid CA".into());
        assert!(err.to_string().contains("Certificate error"));
    }
}
