use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub proxy: ProxySection,
    #[serde(default)]
    pub session: SessionSection,
    #[serde(default)]
    pub filters: Vec<FilterEntry>,
    #[serde(default)]
    pub storage: Vec<StorageEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ProxySection {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
}

impl Default for ProxySection {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            max_connections: default_max_connections(),
            max_body_size: default_max_body_size(),
            request_timeout: default_request_timeout(),
        }
    }
}

fn default_listen_addr() -> String {
    "127.0.0.1:8080".to_string()
}
fn default_max_connections() -> usize {
    1024
}
fn default_max_body_size() -> usize {
    10 * 1024 * 1024
}
fn default_request_timeout() -> u64 {
    30
}

#[derive(Debug, Deserialize)]
pub struct SessionSection {
    #[serde(default = "default_true")]
    pub capture_request_body: bool,
    #[serde(default = "default_true")]
    pub capture_response_body: bool,
    #[serde(default = "default_capture")]
    pub default_action: String,
}

impl Default for SessionSection {
    fn default() -> Self {
        Self {
            capture_request_body: true,
            capture_response_body: true,
            default_action: "capture".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_capture() -> String {
    "capture".to_string()
}

#[derive(Debug, Deserialize)]
pub struct FilterEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub filter_type: String,
    pub pattern: String,
    #[serde(default = "default_pattern_type")]
    pub pattern_type: String,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_pattern_type() -> String {
    "wildcard".to_string()
}
fn default_priority() -> i32 {
    100
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StorageEntry {
    #[serde(rename = "sqlite")]
    Sqlite { path: PathBuf },
    #[serde(rename = "jsonl")]
    Jsonl {
        path: PathBuf,
        rotate_size: Option<u64>,
    },
    #[serde(rename = "pcap")]
    Pcap { path: PathBuf },
}

pub fn load_config(path: &std::path::Path) -> anyhow::Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[proxy]
listen_addr = "0.0.0.0:9090"
max_connections = 2048

[session]
capture_request_body = true
capture_response_body = false
default_action = "capture"

[[filters]]
name = "include-example"
type = "include"
pattern = "*.example.com"

[[storage]]
type = "sqlite"
path = "./netcap.db"

[[storage]]
type = "jsonl"
path = "./netcap.jsonl"
rotate_size = 104857600
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.proxy.listen_addr, "0.0.0.0:9090");
        assert_eq!(config.proxy.max_connections, 2048);
        assert!(!config.session.capture_response_body);
        assert_eq!(config.filters.len(), 1);
        assert_eq!(config.storage.len(), 2);
    }

    #[test]
    fn empty_config_uses_defaults() {
        let config: AppConfig = toml::from_str("").unwrap();
        assert_eq!(config.proxy.listen_addr, "127.0.0.1:8080");
        assert_eq!(config.proxy.max_connections, 1024);
        assert!(config.session.capture_request_body);
    }

    #[test]
    fn missing_file_returns_default() {
        let config = load_config(std::path::Path::new("/nonexistent/file.toml")).unwrap();
        assert_eq!(config.proxy.listen_addr, "127.0.0.1:8080");
    }
}
