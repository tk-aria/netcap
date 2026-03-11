use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: SocketAddr,
    pub upstream_proxy: Option<String>,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    #[serde(default = "default_request_timeout", with = "duration_secs")]
    pub request_timeout: Duration,
}

fn default_listen_addr() -> SocketAddr {
    "127.0.0.1:8080".parse().unwrap()
}
fn default_max_connections() -> usize {
    1024
}
fn default_max_body_size() -> usize {
    10 * 1024 * 1024
} // 10MB
fn default_request_timeout_secs() -> u64 {
    30
}
fn default_request_timeout() -> Duration {
    Duration::from_secs(default_request_timeout_secs())
}

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;
    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            upstream_proxy: None,
            max_connections: default_max_connections(),
            max_body_size: default_max_body_size(),
            request_timeout: Duration::from_secs(default_request_timeout_secs()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub name: Option<String>,
    pub capture_request_body: bool,
    pub capture_response_body: bool,
    pub max_body_size_bytes: usize,
    pub storage_backends: Vec<StorageBackendType>,
    pub default_action: DefaultAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackendType {
    Sqlite {
        path: PathBuf,
    },
    Jsonl {
        path: PathBuf,
        rotate_size: Option<u64>,
    },
    Pcap {
        path: PathBuf,
    },
    BigQuery {
        project_id: String,
        dataset_id: String,
        table_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DefaultAction {
    #[default]
    Capture,
    Passthrough,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(
            config.listen_addr,
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap()
        );
        assert!(config.upstream_proxy.is_none());
        assert_eq!(config.max_connections, 1024);
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }

    #[test]
    fn proxy_config_json_roundtrip() {
        let config = ProxyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.listen_addr, config.listen_addr);
        assert_eq!(deserialized.max_connections, config.max_connections);
    }

    #[test]
    fn proxy_config_partial_json() {
        let json = r#"{"listen_addr": "0.0.0.0:9090"}"#;
        let config: ProxyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.listen_addr,
            "0.0.0.0:9090".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(config.max_connections, 1024); // default
        assert_eq!(config.request_timeout, Duration::from_secs(30)); // default
    }

    #[test]
    fn session_config_serialization() {
        let config = SessionConfig {
            name: Some("test-session".into()),
            capture_request_body: true,
            capture_response_body: true,
            max_body_size_bytes: 1024,
            storage_backends: vec![StorageBackendType::Sqlite {
                path: PathBuf::from("/tmp/test.db"),
            }],
            default_action: DefaultAction::Capture,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-session"));
    }

    #[test]
    fn storage_backend_type_variants() {
        let sqlite = StorageBackendType::Sqlite {
            path: PathBuf::from("test.db"),
        };
        let json = serde_json::to_string(&sqlite).unwrap();
        assert!(json.contains("Sqlite"));

        let jsonl = StorageBackendType::Jsonl {
            path: PathBuf::from("out.jsonl"),
            rotate_size: Some(1024 * 1024),
        };
        let json = serde_json::to_string(&jsonl).unwrap();
        assert!(json.contains("Jsonl"));
    }

    #[test]
    fn default_action_default() {
        let action = DefaultAction::default();
        matches!(action, DefaultAction::Capture);
    }

    #[test]
    fn invalid_addr_deserialization() {
        let json = r#"{"listen_addr": "not-an-addr"}"#;
        let result = serde_json::from_str::<ProxyConfig>(json);
        assert!(result.is_err());
    }
}
