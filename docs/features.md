# netcap 実装タスク一覧 (features.md)

> 作成日: 2026-03-11
> リポジトリ: https://github.com/tk-aria/netcap
> 設計ドキュメント: [docs/00_sow.md](./00_sow.md)

---

## 凡例

- `[ ]` 未着手
- `[x]` 完了
- 各Stepには対象ファイル・サンプルコード・テストケースを記載
- ⚠️ 300行超の可能性がある場合はファイル分割を提案
- 各Phaseの末尾にカバレッジ・ビルド・Docker検証ステップを配置

---

## Phase 1: プロジェクト基盤 & コアデータ型

**目的:** Cargoワークスペースの初期化、エラー型・データ型・設定型の定義

### Step 1.1: ワークスペース初期化

- [x] ルート `Cargo.toml` にワークスペース定義と共通依存を記述 (2026-03-11)
- [x] `rust-toolchain.toml` を作成 (2026-03-11)
- [x] `.gitignore` を更新（target/, *.pem, *.db 等） (2026-03-11)
- [x] 8クレートのディレクトリとスケルトン `Cargo.toml` を作成 (2026-03-11)

**対象ファイル:**
```
Cargo.toml (ルート)
rust-toolchain.toml
crates/netcap-core/Cargo.toml
crates/netcap-storage-sqlite/Cargo.toml
crates/netcap-storage-jsonl/Cargo.toml
crates/netcap-storage-pcap/Cargo.toml
crates/netcap-storage-bigquery/Cargo.toml
crates/netcap-ffi/Cargo.toml
crates/netcap-cli/Cargo.toml
crates/netcap-tui/Cargo.toml
```

**サンプルコード (Cargo.toml ルート):**
```toml
[workspace]
resolver = "2"
members = [
    "crates/netcap-core",
    "crates/netcap-storage-sqlite",
    "crates/netcap-storage-jsonl",
    "crates/netcap-storage-pcap",
    "crates/netcap-storage-bigquery",
    "crates/netcap-ffi",
    "crates/netcap-cli",
    "crates/netcap-tui",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/tk-aria/netcap"

[workspace.dependencies]
tokio = { version = "1.50", features = ["full"] }
async-trait = "0.1"
hyper = { version = "1", features = ["full"] }
hyper-util = "0.1"
http = "1"
bytes = "1"
rustls = "0.23"
rcgen = "0.14"
hudsucker = { version = "0.21", features = ["rcgen-ca", "rustls-client", "decoder"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v7", "serde"] }
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
regex = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
pcap-file = "2"
gcp-bigquery-client = "0.27"
uniffi = "0.28"
clap = { version = "4.5", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.28"
tokio-test = "0.4"
wiremock = "0.6"
tempfile = "3"
```

**サンプルコード (rust-toolchain.toml):**
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = [
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "x86_64-linux-android",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "x86_64-apple-ios",
]
```

---

### Step 1.2: エラー型定義 (netcap-core)

- [ ] `crates/netcap-core/src/error.rs` にエラー型階層を定義
- [ ] `CaptureError`, `ProxyError`, `StorageError`, `CertError`, `FilterError` を定義
- [ ] 各エラーに `thiserror` の `#[error]` アトリビュートを付与
- [ ] `From` トレイト変換を実装

**対象ファイル:** `crates/netcap-core/src/error.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/error.rs
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
```

**テストケース:**
- 正常系: 各エラー型の `Display` 出力が期待どおりか
- 正常系: `From` 変換が正しく動作するか (`ProxyError` → `CaptureError`)
- 異常系: `ServerCertFailed` にネストされたエラーの `source()` チェーンが正しく辿れるか

---

### Step 1.3: コアデータ型定義 (capture モジュール)

- [ ] `crates/netcap-core/src/capture/mod.rs` に `CaptureHandler` trait を定義
- [ ] `crates/netcap-core/src/capture/exchange.rs` に `CapturedRequest`, `CapturedResponse`, `CapturedExchange`, `TlsInfo` 構造体を定義
- [ ] `crates/netcap-core/src/capture/body.rs` にボディ処理ユーティリティ（圧縮解凍、サイズ制限）を実装

**対象ファイル:**
```
crates/netcap-core/src/capture/mod.rs
crates/netcap-core/src/capture/exchange.rs
crates/netcap-core/src/capture/body.rs
```

**⚠️ ファイル分割提案:** `exchange.rs` は構造体4つ + impl ブロック + Serialize/Deserialize で200行前後。ボディ処理は `body.rs` に分離済み。

**サンプルコード (exchange.rs):**
```rust
// crates/netcap-core/src/capture/exchange.rs
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    pub sni: String,
    pub protocol_version: String,
    pub cipher_suite: String,
}

#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub id: Uuid,
    pub session_id: Uuid,
    pub connection_id: Uuid,
    pub sequence_number: u64,
    pub timestamp: DateTime<Utc>,
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub body_truncated: bool,
    pub tls_info: Option<TlsInfo>,
}

#[derive(Debug, Clone)]
pub struct CapturedResponse {
    pub id: Uuid,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub status: StatusCode,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub body_truncated: bool,
    pub latency: std::time::Duration,
    pub ttfb: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct CapturedExchange {
    pub request: CapturedRequest,
    pub response: Option<CapturedResponse>,
}
```

**サンプルコード (body.rs):**
```rust
// crates/netcap-core/src/capture/body.rs
use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder};
use std::io::Read;

pub fn truncate_body(body: &Bytes, max_size: usize) -> (Bytes, bool) {
    if body.len() <= max_size {
        (body.clone(), false)
    } else {
        (body.slice(..max_size), true)
    }
}

pub fn decode_body(body: &[u8], encoding: &str) -> Result<Vec<u8>, std::io::Error> {
    match encoding {
        "gzip" => {
            let mut decoder = GzDecoder::new(body);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;
            Ok(decoded)
        }
        "deflate" => {
            let mut decoder = DeflateDecoder::new(body);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;
            Ok(decoded)
        }
        "br" => {
            let mut decoded = Vec::new();
            brotli::BrotliDecompress(&mut &body[..], &mut decoded)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(decoded)
        }
        _ => Ok(body.to_vec()),
    }
}
```

**追加依存 (netcap-core):** `flate2 = "1"`, `brotli = "7"`

**テストケース:**
- 正常系: `truncate_body` で max_size より小さいボディはそのまま返る
- 正常系: `truncate_body` で max_size を超えるボディが切り詰められ `truncated=true`
- 正常系: `decode_body` で gzip 圧縮データが正しくデコードされる
- 正常系: `decode_body` で identity (未圧縮) はそのまま返る
- 異常系: `decode_body` で不正な gzip データを渡すと `Err` が返る
- 異常系: `decode_body` で未知の encoding を渡すとそのまま返る

---

### Step 1.4: 設定型定義 (config モジュール)

- [ ] `crates/netcap-core/src/config.rs` にプロキシ設定・セッション設定を定義
- [ ] TOML デシリアライズに対応 (`serde::Deserialize`)
- [ ] デフォルト値を `Default` trait で提供

**対象ファイル:** `crates/netcap-core/src/config.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/config.rs
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
    #[serde(default = "default_request_timeout_secs", with = "duration_secs")]
    pub request_timeout: Duration,
}

fn default_listen_addr() -> SocketAddr {
    "127.0.0.1:8080".parse().unwrap()
}
fn default_max_connections() -> usize { 1024 }
fn default_max_body_size() -> usize { 10 * 1024 * 1024 } // 10MB
fn default_request_timeout_secs() -> u64 { 30 }

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
    Sqlite { path: PathBuf },
    Jsonl { path: PathBuf, rotate_size: Option<u64> },
    Pcap { path: PathBuf },
    BigQuery { project_id: String, dataset_id: String, table_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DefaultAction {
    #[default]
    Capture,
    Passthrough,
}
```

**テストケース:**
- 正常系: `ProxyConfig::default()` のデフォルト値が仕様どおりか
- 正常系: TOMLからのデシリアライズが正しく動作するか
- 正常系: 一部フィールドを省略したTOMLでもデフォルト値が適用されるか
- 異常系: 不正なアドレス文字列でデシリアライズエラーが返るか
- 異常系: 負のタイムアウト値でエラーが返るか

---

### Step 1.5: ドメインフィルタ実装 (filter モジュール)

- [ ] `crates/netcap-core/src/filter/mod.rs` に `DomainMatcher` trait と `DomainFilter` を実装
- [ ] `crates/netcap-core/src/filter/pattern.rs` に `DomainPattern` (exact/wildcard/regex) を実装
- [ ] フィルタの優先度制御、`CaptureDecision` enum を定義

**対象ファイル:**
```
crates/netcap-core/src/filter/mod.rs
crates/netcap-core/src/filter/pattern.rs
```

**サンプルコード (pattern.rs):**
```rust
// crates/netcap-core/src/filter/pattern.rs
use regex::Regex;
use crate::error::FilterError;

#[derive(Debug, Clone)]
pub enum DomainPattern {
    Exact(String),
    Wildcard(String),
    Regex(Regex),
}

impl DomainPattern {
    pub fn new_exact(domain: &str) -> Self {
        Self::Exact(domain.to_lowercase())
    }

    pub fn new_wildcard(pattern: &str) -> Self {
        Self::Wildcard(pattern.to_lowercase())
    }

    pub fn new_regex(pattern: &str) -> Result<Self, FilterError> {
        let regex = Regex::new(pattern)?;
        Ok(Self::Regex(regex))
    }

    pub fn matches(&self, domain: &str) -> bool {
        let domain = domain.to_lowercase();
        match self {
            Self::Exact(p) => *p == domain,
            Self::Wildcard(p) => {
                if let Some(suffix) = p.strip_prefix("*.") {
                    domain.ends_with(suffix)
                        && (domain.len() > suffix.len() + 1
                            || domain == suffix)
                } else {
                    *p == domain
                }
            }
            Self::Regex(r) => r.is_match(&domain),
        }
    }
}
```

**サンプルコード (mod.rs):**
```rust
// crates/netcap-core/src/filter/mod.rs
pub mod pattern;

use pattern::DomainPattern;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum CaptureDecision {
    Capture(Uuid),   // マッチしたfilter_id
    Passthrough,
    Default,
}

#[derive(Debug, Clone)]
pub enum FilterType {
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub id: Uuid,
    pub name: String,
    pub filter_type: FilterType,
    pub pattern: DomainPattern,
    pub priority: i32,
    pub enabled: bool,
}

pub trait DomainMatcher: Send + Sync + 'static {
    fn evaluate(&self, domain: &str) -> CaptureDecision;
    fn add_rule(&mut self, rule: FilterRule);
    fn remove_rule(&mut self, id: &Uuid) -> bool;
    fn clear(&mut self);
    fn rules(&self) -> &[FilterRule];
}

pub struct DomainFilter {
    rules: Vec<FilterRule>,
}

impl DomainFilter {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
}

impl DomainMatcher for DomainFilter {
    fn evaluate(&self, domain: &str) -> CaptureDecision {
        let mut sorted: Vec<&FilterRule> = self.rules.iter()
            .filter(|r| r.enabled)
            .collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted {
            if rule.pattern.matches(domain) {
                return match rule.filter_type {
                    FilterType::Include => CaptureDecision::Capture(rule.id),
                    FilterType::Exclude => CaptureDecision::Passthrough,
                };
            }
        }
        CaptureDecision::Default
    }

    fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    fn remove_rule(&mut self, id: &Uuid) -> bool {
        let len = self.rules.len();
        self.rules.retain(|r| r.id != *id);
        self.rules.len() < len
    }

    fn clear(&mut self) {
        self.rules.clear();
    }

    fn rules(&self) -> &[FilterRule] {
        &self.rules
    }
}
```

**テストケース:**
- 正常系: `Exact("example.com")` が `"example.com"` にマッチする
- 正常系: `Wildcard("*.example.com")` が `"api.example.com"` にマッチする
- 正常系: `Wildcard("*.example.com")` が `"example.com"` にはマッチしない
- 正常系: `Regex("^api\\..*")` が `"api.example.com"` にマッチする
- 正常系: 大文字小文字を区別しない (`"API.Example.Com"` → マッチ)
- 正常系: include ルールが優先度どおりに評価される
- 正常系: exclude ルールでパススルーが返る
- 正常系: ルール無しで `CaptureDecision::Default` が返る
- 正常系: `remove_rule` で指定IDのルールが削除される
- 異常系: 不正な正規表現で `FilterError::RegexError` が返る
- 異常系: 空のドメイン文字列に対するマッチ動作

---

### Step 1.6: lib.rs クレートルートと mod 宣言

- [ ] `crates/netcap-core/src/lib.rs` に pub mod 宣言と re-export を記述
- [ ] `crates/netcap-core/src/storage/mod.rs` に `StorageBackend` trait と `CaptureHandler` trait を定義（実装は別クレート）

**対象ファイル:**
```
crates/netcap-core/src/lib.rs
crates/netcap-core/src/storage/mod.rs
```

**サンプルコード (lib.rs):**
```rust
// crates/netcap-core/src/lib.rs
pub mod capture;
pub mod config;
pub mod error;
pub mod filter;
pub mod proxy;
pub mod storage;
pub mod tls;
```

**サンプルコード (storage/mod.rs):**
```rust
// crates/netcap-core/src/storage/mod.rs
use async_trait::async_trait;
use crate::capture::exchange::CapturedExchange;
use crate::error::StorageError;

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn initialize(&mut self) -> Result<(), StorageError>;
    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError>;
    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError>;
    async fn flush(&self) -> Result<(), StorageError>;
    async fn close(&mut self) -> Result<(), StorageError>;
}

pub struct FanoutWriter {
    backends: Vec<Box<dyn StorageBackend>>,
}

impl FanoutWriter {
    pub fn new(backends: Vec<Box<dyn StorageBackend>>) -> Self {
        Self { backends }
    }

    pub async fn write_all(&self, exchange: &CapturedExchange) -> Vec<Result<(), StorageError>> {
        let mut results = Vec::new();
        for backend in &self.backends {
            results.push(backend.write(exchange).await);
        }
        results
    }

    pub async fn flush_all(&self) -> Vec<Result<(), StorageError>> {
        let mut results = Vec::new();
        for backend in &self.backends {
            results.push(backend.flush().await);
        }
        results
    }
}
```

---

### Step 1.7: Phase 1 テスト・ビルド検証

- [ ] `crates/netcap-core/tests/` に単体テストを実装（error, capture, config, filter, storage 各モジュール）
- [ ] テストカバレッジ90%以上を達成。未テスト部分を洗い出し追加テストを実装
- [ ] `cargo build --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `cargo clippy --workspace -- -D warnings` で警告ゼロ
- [ ] Dockerfile を作成し `docker build` が正常完了すること
- [ ] **skip/TODO残留チェック:** 実装コード内に `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` が残っていないか `grep -rn` で検索し、残留があれば実装を完了させる
- [ ] **Phase 1 機能検証チェックリスト:**
  - [ ] `CaptureError`, `ProxyError`, `StorageError`, `CertError`, `FilterError` が正しく構築・表示されること
  - [ ] `CapturedRequest`, `CapturedResponse`, `CapturedExchange` が正しくインスタンス化されること
  - [ ] `truncate_body` / `decode_body` が各エンコーディングで正しく動作すること
  - [ ] `ProxyConfig::default()` / TOML デシリアライズが正しく動作すること
  - [ ] `DomainFilter` の exact/wildcard/regex マッチが正しく動作すること
  - [ ] `FanoutWriter` が複数バックエンドに書き出しできること
  - [ ] 上記を実際に `cargo test` で実行し、全テストがパスすることを確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase1_report.md` にまとめる（テスト結果出力、カバレッジ、ビルドログ）

**Dockerfile サンプル:**
```dockerfile
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin netcap

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/netcap /usr/local/bin/
ENTRYPOINT ["netcap"]
```

**カバレッジ計測:**
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html --output-dir coverage/
# カバレッジが90%未満の場合、不足テストを特定して追加
```

---

## Phase 2: TLS & 証明書管理

**目的:** CA証明書の生成・管理、動的サーバー証明書発行、証明書キャッシュの実装

### Step 2.1: CertificateProvider trait 定義

- [ ] `crates/netcap-core/src/tls/mod.rs` に `CertificateProvider` trait を定義
- [ ] `CaCertificate`, `ServerCertificate` 構造体を定義

**対象ファイル:** `crates/netcap-core/src/tls/mod.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/tls/mod.rs
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
```

---

### Step 2.2: CA証明書生成・管理

- [ ] `crates/netcap-core/src/tls/ca.rs` に `RcgenCaProvider` を実装
- [ ] rcgen を使用して自己署名CA証明書を生成
- [ ] CA証明書の PEM 形式エクスポートを実装
- [ ] CA証明書のファイルへの保存・読み込みを実装

**対象ファイル:** `crates/netcap-core/src/tls/ca.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/tls/ca.rs
use rcgen::{
    CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    BasicConstraints, KeyUsagePurpose,
};
use crate::error::CertError;
use crate::tls::{CaCertificate, CertificateProvider, ServerCertificate};
use std::path::{Path, PathBuf};
use async_trait::async_trait;

pub struct RcgenCaProvider {
    ca_cert_pem: String,
    ca_key_pem: String,
    store_path: PathBuf,
}

impl RcgenCaProvider {
    pub fn generate_ca(
        common_name: &str,
        store_path: &Path,
    ) -> Result<Self, CertError> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, common_name);
        dn.push(DnType::OrganizationName, "netcap");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];

        let key_pair = KeyPair::generate()
            .map_err(|e| CertError::CaGenerationFailed(e.to_string()))?;
        let cert = params.self_signed(&key_pair)
            .map_err(|e| CertError::CaGenerationFailed(e.to_string()))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        Ok(Self {
            ca_cert_pem: cert_pem,
            ca_key_pem: key_pem,
            store_path: store_path.to_path_buf(),
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
            store_path: store_path.to_path_buf(),
        })
    }
}
```

**テストケース:**
- 正常系: CA証明書生成が成功し、PEM 形式で取得できる
- 正常系: CA証明書を保存→再読み込みで同一内容が得られる
- 正常系: 生成された証明書が `is_ca=true` である
- 異常系: 存在しないファイルからの読み込みで `StoreAccessFailed` が返る
- 異常系: 不正なPEM文字列でのパースエラー

---

### Step 2.3: 動的サーバー証明書発行

- [ ] `crates/netcap-core/src/tls/server_cert.rs` に CA 署名によるサーバー証明書の動的生成を実装
- [ ] SAN (Subject Alternative Name) にドメインを設定
- [ ] 有効期限の設定を実装

**対象ファイル:** `crates/netcap-core/src/tls/server_cert.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/tls/server_cert.rs
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use crate::error::CertError;
use crate::tls::ServerCertificate;

pub fn issue_server_certificate(
    domain: &str,
    ca_cert_pem: &str,
    ca_key_pem: &str,
) -> Result<ServerCertificate, CertError> {
    let ca_key = KeyPair::from_pem(ca_key_pem)
        .map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;

    let ca_params = CertificateParams::from_ca_cert_pem(ca_cert_pem)
        .map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;
    let ca_cert = ca_params.self_signed(&ca_key)
        .map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;

    let mut server_params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domain);
    server_params.distinguished_name = dn;
    server_params.subject_alt_names = vec![SanType::DnsName(domain.try_into().unwrap())];

    let server_key = KeyPair::generate()
        .map_err(|e| CertError::ServerCertFailed {
            domain: domain.to_string(),
            source: Box::new(e),
        })?;

    let server_cert = server_params.signed_by(&server_key, &ca_cert, &ca_key)
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
```

**テストケース:**
- 正常系: CA証明書を使ってサーバー証明書が生成される
- 正常系: 生成された証明書のSANに指定ドメインが含まれる
- 正常系: ワイルドカードドメイン (`*.example.com`) でも証明書が生成される
- 異常系: 不正なCA PEM文字列で `ServerCertFailed` が返る

---

### Step 2.4: 証明書キャッシュ (TTL付き)

- [ ] `crates/netcap-core/src/tls/store.rs` に証明書キャッシュを実装
- [ ] TTL ベースの期限切れ管理
- [ ] `DashMap` または `RwLock<HashMap>` による並行安全なキャッシュ

**対象ファイル:** `crates/netcap-core/src/tls/store.rs`

**追加依存:** `dashmap = "6"`

**サンプルコード:**
```rust
// crates/netcap-core/src/tls/store.rs
use dashmap::DashMap;
use std::time::{Duration, Instant};
use crate::tls::ServerCertificate;

struct CacheEntry {
    cert: ServerCertificate,
    created_at: Instant,
}

pub struct CertificateCache {
    cache: DashMap<String, CacheEntry>,
    ttl: Duration,
}

impl CertificateCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, domain: &str) -> Option<ServerCertificate> {
        self.cache.get(domain).and_then(|entry| {
            if entry.created_at.elapsed() < self.ttl {
                Some(entry.cert.clone())
            } else {
                drop(entry);
                self.cache.remove(domain);
                None
            }
        })
    }

    pub fn insert(&self, domain: String, cert: ServerCertificate) {
        self.cache.insert(domain, CacheEntry {
            cert,
            created_at: Instant::now(),
        });
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}
```

**テストケース:**
- 正常系: 証明書をキャッシュし `get` で取得できる
- 正常系: TTL 期限内の証明書が返る
- 正常系: TTL 期限切れの証明書が `None` になる
- 正常系: `clear` で全キャッシュが削除される
- 正常系: 異なるドメインの証明書が独立してキャッシュされる
- 異常系: 存在しないドメインで `None` が返る

---

### Step 2.5: Phase 2 テスト・ビルド検証

- [ ] tls モジュール全体の単体テスト実装 (ca, server_cert, store)
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了し、コンテナが起動すること
- [ ] **skip/TODO残留チェック:** `crates/netcap-core/src/tls/` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を `grep -rn` で検索し、残留があれば実装を完了させる
- [ ] **Phase 2 機能検証チェックリスト:**
  - [ ] `RcgenCaProvider::generate_ca()` でCA証明書が生成されること
  - [ ] CA証明書のファイル保存・再読み込みが正しく動作すること
  - [ ] `issue_server_certificate()` でCA署名のサーバー証明書が生成されること
  - [ ] 生成されたサーバー証明書のSANに指定ドメインが含まれること
  - [ ] `CertificateCache` の TTL 付きキャッシュが正しく動作すること
  - [ ] 上記を実際に `cargo test` で実行し、全テストがパスすることを確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase2_report.md` にまとめる

---

## Phase 3: MITMプロキシエンジン

**目的:** hudsucker ベースのプロキシサーバー、HTTP handler、接続管理、Ring Buffer の実装

### Step 3.1: ProxyServer & ProxyServerBuilder

- [ ] `crates/netcap-core/src/proxy/mod.rs` に `ProxyServer` と `ProxyServerBuilder` を実装
- [ ] hudsucker の `Proxy` をラップし、設定に基づくプロキシ起動を実装
- [ ] Graceful Shutdown (`tokio::sync::broadcast`) を実装

**対象ファイル:** `crates/netcap-core/src/proxy/mod.rs`

**⚠️ ファイル分割提案:** `mod.rs` は ProxyServer + Builder で250行程度。handler.rs, connection.rs と分離済みなので問題なし。

**サンプルコード:**
```rust
// crates/netcap-core/src/proxy/mod.rs
pub mod handler;
pub mod connection;

use hudsucker::Proxy;
use tokio::sync::broadcast;
use std::sync::Arc;
use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::filter::DomainFilter;
use crate::tls::CertificateProvider;
use crate::storage::StorageBackend;

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
            cert_provider: self.cert_provider
                .ok_or(ProxyError::NotRunning)?,
            domain_filter: self.domain_filter.unwrap_or_else(|| {
                Arc::new(DomainFilter::new())
            }),
            storage: self.storage
                .ok_or(ProxyError::NotRunning)?,
            shutdown_tx,
        })
    }
}

impl ProxyServer {
    pub fn builder() -> ProxyServerBuilder {
        ProxyServerBuilder::new()
    }

    pub async fn run(&self) -> Result<(), ProxyError> {
        // hudsucker Proxy 起動ロジック
        // handler::NetcapHandler を HttpHandler として登録
        // shutdown_rx を監視して停止
        todo!("Step 3.2 で実装")
    }

    pub fn shutdown(&self) -> Result<(), ProxyError> {
        self.shutdown_tx.send(()).map_err(|_| ProxyError::NotRunning)?;
        Ok(())
    }
}
```

**テストケース:**
- 正常系: `ProxyServerBuilder` で全フィールドを設定して `build()` が成功する
- 正常系: `shutdown()` が broadcast を送信して正常終了する
- 異常系: `cert_provider` 未設定で `build()` がエラーを返す
- 異常系: `storage` 未設定で `build()` がエラーを返す

---

### Step 3.2: HTTP Handler 実装 (hudsucker HttpHandler)

- [ ] `crates/netcap-core/src/proxy/handler.rs` に hudsucker の `HttpHandler` trait を実装
- [ ] `handle_request` でリクエストをキャプチャ
- [ ] `handle_response` でレスポンスをキャプチャ
- [ ] ドメインフィルタとの連携
- [ ] CapturedExchange を生成して Ring Buffer に送信

**対象ファイル:** `crates/netcap-core/src/proxy/handler.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/proxy/handler.rs
use hudsucker::{
    async_trait::async_trait,
    hyper::{Request, Response, Body},
    HttpContext, HttpHandler, RequestOrResponse,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;
use crate::capture::exchange::{CapturedRequest, CapturedResponse, CapturedExchange};
use crate::filter::{DomainMatcher, CaptureDecision};

pub struct NetcapHandler {
    filter: Arc<dyn DomainMatcher>,
    event_tx: mpsc::Sender<CapturedExchange>,
    session_id: Uuid,
    max_body_size: usize,
}

#[async_trait]
impl HttpHandler for NetcapHandler {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let host = req.uri().host().unwrap_or("").to_string();
        let decision = self.filter.evaluate(&host);

        match decision {
            CaptureDecision::Capture(_filter_id) => {
                // リクエストをキャプチャ (ボディをバッファリング)
                // CapturedRequest を構築
                // ctx に request_id を保存
                RequestOrResponse::Request(req)
            }
            CaptureDecision::Passthrough => {
                // パススルー: キャプチャしない
                RequestOrResponse::Request(req)
            }
            CaptureDecision::Default => {
                RequestOrResponse::Request(req)
            }
        }
    }

    async fn handle_response(
        &mut self,
        ctx: &HttpContext,
        res: Response<Body>,
    ) -> Response<Body> {
        // CapturedResponse を構築
        // CapturedExchange を組み立てて event_tx に送信
        res
    }
}
```

**テストケース:**
- 正常系: include ドメインへのリクエストがキャプチャされる
- 正常系: exclude ドメインへのリクエストがパススルーされる
- 正常系: レスポンスが正しくペアリングされ CapturedExchange が生成される
- 正常系: ボディが max_body_size で切り詰められる
- 異常系: event_tx がクローズされている場合のハンドリング

---

### Step 3.3: 接続管理

- [ ] `crates/netcap-core/src/proxy/connection.rs` に接続トラッキングを実装
- [ ] Connection 構造体にTLS情報 (SNI, cipher suite, ALPN) を保持
- [ ] 接続ごとのリクエストカウントを管理

**対象ファイル:** `crates/netcap-core/src/proxy/connection.rs`

**サンプルコード:**
```rust
// crates/netcap-core/src/proxy/connection.rs
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub session_id: Uuid,
    pub client_addr: SocketAddr,
    pub server_hostname: String,
    pub server_addr: Option<SocketAddr>,
    pub is_tls: bool,
    pub tls_version: Option<String>,
    pub cipher_suite: Option<String>,
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub established_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub close_reason: Option<String>,
    pub request_count: u64,
}

pub struct ConnectionTracker {
    connections: DashMap<Uuid, ConnectionInfo>,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self { connections: DashMap::new() }
    }

    pub fn track(&self, info: ConnectionInfo) -> Uuid {
        let id = info.id;
        self.connections.insert(id, info);
        id
    }

    pub fn increment_request_count(&self, id: &Uuid) {
        if let Some(mut conn) = self.connections.get_mut(id) {
            conn.request_count += 1;
        }
    }

    pub fn close(&self, id: &Uuid, reason: &str) {
        if let Some(mut conn) = self.connections.get_mut(id) {
            conn.closed_at = Some(Utc::now());
            conn.close_reason = Some(reason.to_string());
        }
    }

    pub fn get(&self, id: &Uuid) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|c| c.clone())
    }

    pub fn active_count(&self) -> usize {
        self.connections.iter().filter(|c| c.closed_at.is_none()).count()
    }
}
```

**テストケース:**
- 正常系: 接続を追跡し `get` で取得できる
- 正常系: `increment_request_count` でカウントが増加する
- 正常系: `close` で `closed_at` と `close_reason` が設定される
- 正常系: `active_count` がオープン接続のみをカウントする
- 異常系: 存在しない接続IDへの操作は無視される

---

### Step 3.4: Ring Buffer & イベントディスパッチャ

- [ ] `crates/netcap-core/src/storage/buffer.rs` に lock-free Ring Buffer を実装
- [ ] `crates/netcap-core/src/storage/dispatcher.rs` にバッチディスパッチャを実装
- [ ] バッチ間隔 (100ms) またはバッファ閾値でのバッチ取り出し
- [ ] 複数 `StorageBackend` への並行書き出し

**対象ファイル:**
```
crates/netcap-core/src/storage/buffer.rs
crates/netcap-core/src/storage/dispatcher.rs
```

**⚠️ ファイル分割:** storage/mod.rs (trait定義 ~80行), buffer.rs (~120行), dispatcher.rs (~150行) で分割済み。

**サンプルコード (buffer.rs):**
```rust
// crates/netcap-core/src/storage/buffer.rs
use tokio::sync::mpsc;
use crate::capture::exchange::CapturedExchange;

pub struct CaptureBuffer {
    tx: mpsc::Sender<CapturedExchange>,
    rx: mpsc::Receiver<CapturedExchange>,
}

impl CaptureBuffer {
    pub fn new(capacity: usize) -> (BufferSender, BufferReceiver) {
        let (tx, rx) = mpsc::channel(capacity);
        (BufferSender { tx }, BufferReceiver { rx })
    }
}

#[derive(Clone)]
pub struct BufferSender {
    tx: mpsc::Sender<CapturedExchange>,
}

impl BufferSender {
    pub async fn send(&self, exchange: CapturedExchange) -> Result<(), CapturedExchange> {
        self.tx.send(exchange).await.map_err(|e| e.0)
    }

    pub fn try_send(&self, exchange: CapturedExchange) -> Result<(), CapturedExchange> {
        self.tx.try_send(exchange).map_err(|e| match e {
            mpsc::error::TrySendError::Full(ex) => ex,
            mpsc::error::TrySendError::Closed(ex) => ex,
        })
    }
}

pub struct BufferReceiver {
    rx: mpsc::Receiver<CapturedExchange>,
}

impl BufferReceiver {
    pub async fn recv_batch(&mut self, max_size: usize) -> Vec<CapturedExchange> {
        let mut batch = Vec::with_capacity(max_size);
        // 最初の1件はブロッキングで待機
        if let Some(ex) = self.rx.recv().await {
            batch.push(ex);
        }
        // 残りはノンブロッキングで取得
        while batch.len() < max_size {
            match self.rx.try_recv() {
                Ok(ex) => batch.push(ex),
                Err(_) => break,
            }
        }
        batch
    }
}
```

**サンプルコード (dispatcher.rs):**
```rust
// crates/netcap-core/src/storage/dispatcher.rs
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use crate::storage::StorageBackend;
use crate::storage::buffer::BufferReceiver;

pub struct StorageDispatcher {
    backends: Vec<Arc<dyn StorageBackend>>,
    receiver: BufferReceiver,
    batch_size: usize,
    flush_interval: Duration,
}

impl StorageDispatcher {
    pub fn new(
        backends: Vec<Arc<dyn StorageBackend>>,
        receiver: BufferReceiver,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        Self { backends, receiver, batch_size, flush_interval }
    }

    pub async fn run(&mut self) {
        let mut interval = time::interval(self.flush_interval);
        loop {
            tokio::select! {
                batch = self.receiver.recv_batch(self.batch_size) => {
                    if batch.is_empty() {
                        break; // sender dropped
                    }
                    self.dispatch_batch(&batch).await;
                }
                _ = interval.tick() => {
                    // flush all backends periodically
                    for backend in &self.backends {
                        let _ = backend.flush().await;
                    }
                }
            }
        }
    }

    async fn dispatch_batch(&self, batch: &[crate::capture::exchange::CapturedExchange]) {
        let futures: Vec<_> = self.backends.iter().map(|backend| {
            let backend = Arc::clone(backend);
            let batch = batch.to_vec();
            tokio::spawn(async move {
                if let Err(e) = backend.write_batch(&batch).await {
                    tracing::error!("Storage write error: {}", e);
                }
            })
        }).collect();

        for f in futures {
            let _ = f.await;
        }
    }
}
```

**テストケース:**
- 正常系: `BufferSender::send` でイベントが送信され `recv_batch` で受信される
- 正常系: `recv_batch(10)` で10件以下のイベントが一括取得される
- 正常系: `dispatch_batch` で複数バックエンドに並行書き出しされる
- 正常系: flush_interval ごとに `flush` が呼ばれる
- 異常系: `try_send` でバッファ満杯時にエラーが返る
- 異常系: sender がドロップされた後に `recv_batch` が空を返す
- 異常系: 1つのバックエンドがエラーを返しても他は影響を受けない

---

### Step 3.5: ProxyServer.run() の完全実装

- [ ] Step 3.1 の `run()` メソッドを完全に実装
- [ ] hudsucker の `ProxyBuilder` を使用してプロキシを構築
- [ ] `NetcapHandler` を HttpHandler として登録
- [ ] TLS 証明書プロバイダとの統合
- [ ] Graceful Shutdown のフルフロー実装

**対象ファイル:** `crates/netcap-core/src/proxy/mod.rs` (run メソッドの実装)

**テストケース:**
- 正常系: プロキシが指定アドレスで起動する
- 正常系: HTTP リクエストがプロキシ経由で転送される
- 正常系: `shutdown()` 呼び出しでプロキシが停止する
- 異常系: 既に使用中のポートで `BindFailed` が返る
- 異常系: 2重起動で `AlreadyRunning` が返る

---

### Step 3.6: Phase 3 テスト・ビルド検証

- [ ] proxy モジュール全体の結合テスト実装
- [ ] 実際のHTTP通信をプロキシ経由でキャプチャするE2Eテスト
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了し、コンテナ内でプロキシが起動すること
- [ ] **skip/TODO残留チェック:** `crates/netcap-core/src/proxy/` および `crates/netcap-core/src/storage/` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を検索し、残留があれば実装を完了させる
- [ ] **Phase 3 機能検証チェックリスト:**
  - [ ] `ProxyServerBuilder` で全フィールドを設定して `build()` が成功すること
  - [ ] `ProxyServer::run()` でプロキシが指定ポートで起動すること
  - [ ] HTTP リクエストがプロキシ経由で正しく転送されること
  - [ ] `NetcapHandler` がリクエスト/レスポンスをキャプチャし `CapturedExchange` を生成すること
  - [ ] ドメインフィルタと連携し include/exclude/default が正しく判定されること
  - [ ] `ConnectionTracker` が接続を追跡・カウントすること
  - [ ] `BufferSender` / `BufferReceiver` でイベントが送受信されること
  - [ ] `StorageDispatcher` が複数バックエンドへ並行書き出しすること
  - [ ] `shutdown()` で Graceful Shutdown が動作すること
  - [ ] 上記を実際に `cargo test` + 手動プロキシ起動で確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase3_report.md` にまとめる

---

## Phase 4: ストレージバックエンド実装

**目的:** SQLite, JSONL, PCAP の各ストレージバックエンドを実装

### Step 4.1: SQLite ストレージ — スキーマ定義

- [ ] `crates/netcap-storage-sqlite/src/schema.rs` にテーブル作成SQLとマイグレーションを実装
- [ ] 6テーブル + 1中間テーブルの CREATE TABLE を定義
- [ ] インデックス作成SQLを定義
- [ ] WALモード設定のプラグマを追加

**対象ファイル:** `crates/netcap-storage-sqlite/src/schema.rs`

**サンプルコード:**
```rust
// crates/netcap-storage-sqlite/src/schema.rs

pub const PRAGMA_WAL: &str = "PRAGMA journal_mode=WAL;";
pub const PRAGMA_FOREIGN_KEYS: &str = "PRAGMA foreign_keys=ON;";

pub const CREATE_CAPTURE_SESSION: &str = r#"
CREATE TABLE IF NOT EXISTS capture_sessions (
    id TEXT PRIMARY KEY,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    created_at TEXT NOT NULL,
    stopped_at TEXT,
    proxy_listen_addr TEXT NOT NULL,
    proxy_port INTEGER NOT NULL,
    platform TEXT NOT NULL,
    capture_request_body INTEGER NOT NULL DEFAULT 1,
    capture_response_body INTEGER NOT NULL DEFAULT 1,
    max_body_size_bytes INTEGER NOT NULL DEFAULT 10485760,
    storage_backends TEXT NOT NULL,
    ca_id TEXT REFERENCES certificate_authorities(id),
    metadata TEXT
);
"#;

pub const CREATE_HTTP_REQUEST: &str = r#"
CREATE TABLE IF NOT EXISTS http_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES capture_sessions(id),
    connection_id TEXT NOT NULL REFERENCES connections(id),
    sequence_number INTEGER NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    scheme TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    path TEXT NOT NULL,
    query_string TEXT,
    http_version TEXT NOT NULL,
    headers TEXT NOT NULL,
    content_length INTEGER,
    content_type TEXT,
    body BLOB,
    body_truncated INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL,
    timestamp_unix_us INTEGER NOT NULL,
    matched_filter_id TEXT
);
"#;

// ... (他テーブルも同様に定義)

pub const CREATE_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_req_session_ts ON http_requests(session_id, timestamp_unix_us);",
    "CREATE INDEX IF NOT EXISTS idx_req_host ON http_requests(host);",
    "CREATE INDEX IF NOT EXISTS idx_req_conn ON http_requests(connection_id);",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_res_req ON http_responses(request_id);",
    "CREATE INDEX IF NOT EXISTS idx_res_status ON http_responses(status_code);",
    "CREATE INDEX IF NOT EXISTS idx_conn_session ON connections(session_id);",
    "CREATE INDEX IF NOT EXISTS idx_conn_host ON connections(server_hostname);",
];

pub fn initialize_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(PRAGMA_WAL)?;
    conn.execute_batch(PRAGMA_FOREIGN_KEYS)?;
    conn.execute_batch(CREATE_CAPTURE_SESSION)?;
    conn.execute_batch(CREATE_HTTP_REQUEST)?;
    // ... 残りのテーブル
    for idx in CREATE_INDEXES {
        conn.execute_batch(idx)?;
    }
    Ok(())
}
```

**テストケース:**
- 正常系: `initialize_schema` でテーブルが作成される
- 正常系: 2回呼んでも `IF NOT EXISTS` でエラーにならない
- 正常系: WALモードが有効になっている (`PRAGMA journal_mode` → `wal`)
- 異常系: 読み取り専用DBでの初期化エラー

---

### Step 4.2: SQLite ストレージ — クエリ & StorageBackend 実装

- [ ] `crates/netcap-storage-sqlite/src/queries.rs` に INSERT / SELECT の SQL 定数とヘルパー関数を実装
- [ ] `crates/netcap-storage-sqlite/src/lib.rs` に `SqliteStorage` 構造体と `StorageBackend` trait 実装
- [ ] `tokio::task::spawn_blocking` で同期 rusqlite 呼び出しを非同期コンテキストで実行
- [ ] バッチ INSERT をトランザクション内で実行

**対象ファイル:**
```
crates/netcap-storage-sqlite/src/lib.rs
crates/netcap-storage-sqlite/src/queries.rs
```

**⚠️ ファイル分割提案:** `queries.rs` は SQL文 + bind パラメータ処理で200行超の可能性。テーブルごとに `queries/request.rs`, `queries/response.rs` 等に分割を検討。ただし初期はフラットファイルで開始し、300行超えた時点で分割。

**サンプルコード (lib.rs):**
```rust
// crates/netcap-storage-sqlite/src/lib.rs
pub mod schema;
pub mod queries;

use async_trait::async_trait;
use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct SqliteStorageConfig {
    pub db_path: PathBuf,
    pub batch_size: usize,
}

pub struct SqliteStorage {
    conn: Mutex<Connection>,
    config: SqliteStorageConfig,
}

impl SqliteStorage {
    pub fn new(config: SqliteStorageConfig) -> Result<Self, StorageError> {
        let conn = Connection::open(&config.db_path)
            .map_err(|e| StorageError::InitFailed(e.to_string()))?;
        schema::initialize_schema(&conn)
            .map_err(|e| StorageError::InitFailed(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
            config,
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        Ok(()) // already initialized in new()
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let exchange = exchange.clone();
        let conn = self.conn.lock().map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        queries::insert_exchange(&conn, &exchange)
            .map_err(|e| StorageError::WriteFailed(e.to_string()))
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        let exchanges = exchanges.to_vec();
        let conn = self.conn.lock().map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        let tx = conn.unchecked_transaction()
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        for ex in &exchanges {
            queries::insert_exchange(&tx, ex)
                .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        }
        tx.commit().map_err(|e| StorageError::WriteFailed(e.to_string()))
    }

    async fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}
```

**テストケース:**
- 正常系: `SqliteStorage::new` で DB ファイルが作成される
- 正常系: `write` で CapturedExchange が INSERT される
- 正常系: `write_batch` で複数レコードがトランザクション内で INSERT される
- 正常系: 書き込み後に SELECT で同一データが取得できる
- 異常系: 存在しないパスでの `InitFailed` エラー
- 異常系: 重複 ID での INSERT エラー

---

### Step 4.3: JSONL ストレージ実装

- [ ] `crates/netcap-storage-jsonl/src/lib.rs` に `JsonlStorage` と `StorageBackend` 実装
- [ ] `crates/netcap-storage-jsonl/src/serializer.rs` に CapturedExchange → JSON 変換
- [ ] `crates/netcap-storage-jsonl/src/rotation.rs` にファイルローテーション（サイズベース）

**対象ファイル:**
```
crates/netcap-storage-jsonl/src/lib.rs
crates/netcap-storage-jsonl/src/serializer.rs
crates/netcap-storage-jsonl/src/rotation.rs
```

**サンプルコード (lib.rs):**
```rust
// crates/netcap-storage-jsonl/src/lib.rs
pub mod serializer;
pub mod rotation;

use async_trait::async_trait;
use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub struct JsonlStorageConfig {
    pub output_path: PathBuf,
    pub rotate_size: Option<u64>,
}

pub struct JsonlStorage {
    writer: Mutex<File>,
    config: JsonlStorageConfig,
    bytes_written: Mutex<u64>,
}

impl JsonlStorage {
    pub async fn new(config: JsonlStorageConfig) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.output_path)
            .await
            .map_err(|e| StorageError::InitFailed(e.to_string()))?;
        let metadata = file.metadata().await
            .map_err(|e| StorageError::InitFailed(e.to_string()))?;
        Ok(Self {
            writer: Mutex::new(file),
            bytes_written: Mutex::new(metadata.len()),
            config,
        })
    }
}

#[async_trait]
impl StorageBackend for JsonlStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> { Ok(()) }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let json = serializer::to_jsonl(exchange)
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        let bytes = json.as_bytes();
        let mut writer = self.writer.lock().await;
        writer.write_all(bytes).await
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
        writer.write_all(b"\n").await
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

        let mut written = self.bytes_written.lock().await;
        *written += bytes.len() as u64 + 1;

        // ローテーションチェック
        if let Some(max) = self.config.rotate_size {
            if *written >= max {
                drop(writer);
                rotation::rotate(&self.config.output_path).await
                    .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
                *written = 0;
            }
        }
        Ok(())
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        for ex in exchanges {
            self.write(ex).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), StorageError> {
        let mut writer = self.writer.lock().await;
        writer.flush().await.map_err(|e| StorageError::FlushFailed(e.to_string()))
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.flush().await
    }
}
```

**テストケース:**
- 正常系: JSONL ファイルに1行ずつ追記される
- 正常系: 各行が有効な JSON としてパースできる
- 正常系: `rotate_size` 超過でファイルローテーションが発生する
- 正常系: ローテーション後に新ファイルが作成される
- 正常系: `flush` で未書き込みデータがディスクに反映される
- 異常系: 読み取り専用ディレクトリでの `InitFailed` エラー
- 異常系: ディスク満杯時の `WriteFailed` エラーハンドリング

---

### Step 4.4: PCAP ストレージ実装

- [ ] `crates/netcap-storage-pcap/src/lib.rs` に `PcapStorage` と `StorageBackend` 実装
- [ ] `crates/netcap-storage-pcap/src/converter.rs` に HTTP データ → PCAP パケット変換
- [ ] TCP/IP ヘッダの擬似パケット構築

**対象ファイル:**
```
crates/netcap-storage-pcap/src/lib.rs
crates/netcap-storage-pcap/src/converter.rs
```

**サンプルコード (converter.rs):**
```rust
// crates/netcap-storage-pcap/src/converter.rs
use pcap_file::pcap::{PcapHeader, PcapPacket};
use std::time::SystemTime;

/// HTTP データから擬似 TCP/IP パケットを構築する
pub fn build_pcap_packet(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    timestamp: SystemTime,
) -> PcapPacket<'static> {
    let mut packet = Vec::new();
    // Ethernet header (14 bytes)
    packet.extend_from_slice(&[0u8; 6]); // dst mac
    packet.extend_from_slice(&[0u8; 6]); // src mac
    packet.extend_from_slice(&[0x08, 0x00]); // IPv4

    // IPv4 header (20 bytes)
    let total_len = 20 + 20 + payload.len(); // IP + TCP + payload
    packet.push(0x45); // version + IHL
    packet.push(0x00); // DSCP
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.extend_from_slice(&[0; 4]); // ID, flags, fragment
    packet.push(64); // TTL
    packet.push(6);  // TCP
    packet.extend_from_slice(&[0; 2]); // checksum
    packet.extend_from_slice(&src_ip);
    packet.extend_from_slice(&dst_ip);

    // TCP header (20 bytes, simplified)
    packet.extend_from_slice(&src_port.to_be_bytes());
    packet.extend_from_slice(&dst_port.to_be_bytes());
    packet.extend_from_slice(&[0; 8]); // seq, ack
    packet.push(0x50); // data offset
    packet.push(0x18); // flags (PSH+ACK)
    packet.extend_from_slice(&[0xFF, 0xFF]); // window
    packet.extend_from_slice(&[0; 4]); // checksum, urgent

    // Payload
    packet.extend_from_slice(payload);

    let duration = timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    PcapPacket::new(
        duration,
        packet.len() as u32,
        &packet,
    ).into_owned()
}
```

**テストケース:**
- 正常系: PCAP ファイルが有効なフォーマットで作成される
- 正常系: Wireshark で開けるPCAPファイルが生成される (手動確認)
- 正常系: 複数パケットが1つのPCAPファイルに追記される
- 正常系: PcapHeader のスナップショット長が設定値と一致する
- 異常系: 空ペイロードでもパケットが生成される

---

### Step 4.5: Phase 4 テスト・ビルド検証

- [ ] 各ストレージバックエンドの単体テスト実装
- [ ] FanoutWriter による3バックエンド同時書き出しの結合テスト
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了し、コンテナが起動すること
- [ ] **skip/TODO残留チェック:** `crates/netcap-storage-sqlite/`, `crates/netcap-storage-jsonl/`, `crates/netcap-storage-pcap/` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を検索し、残留があれば実装を完了させる
- [ ] **Phase 4 機能検証チェックリスト:**
  - [ ] `SqliteStorage::new()` で DB ファイルが作成され、テーブル・インデックスが存在すること
  - [ ] `SqliteStorage::write()` / `write_batch()` で CapturedExchange が正しく INSERT されること
  - [ ] WAL モードが有効になっていること (`PRAGMA journal_mode`)
  - [ ] `JsonlStorage::write()` で JSONL ファイルに1行追記されること
  - [ ] 各行が有効な JSON としてパースできること
  - [ ] `rotate_size` 超過でファイルローテーションが動作すること
  - [ ] `PcapStorage::write()` で PCAP ファイルにパケットが追記されること
  - [ ] 生成された PCAP ファイルが `pcap-file` crate で再読み込み可能なこと
  - [ ] `FanoutWriter::write_all()` で3バックエンドに同時書き出しされること
  - [ ] 上記を実際に `cargo test` で確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase4_report.md` にまとめる

---

## Phase 5: CLIアプリケーション

**目的:** clap ベースの CLI、サブコマンド、TOML設定、標準出力ログを実装

### Step 5.1: CLI 引数定義 (clap derive)

- [ ] `crates/netcap-cli/src/args.rs` に clap derive でコマンド定義
- [ ] `capture`, `cert` サブコマンドの引数を定義
- [ ] グローバルオプション (config, verbose, format) を定義

**対象ファイル:** `crates/netcap-cli/src/args.rs`

**サンプルコード:**
```rust
// crates/netcap-cli/src/args.rs
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "netcap", version, about = "Cross-platform HTTP/HTTPS capture tool")]
pub struct Cli {
    /// 設定ファイルパス
    #[arg(short, long, default_value = "netcap.toml")]
    pub config: PathBuf,

    /// ログ出力レベル
    #[arg(short, long, default_value = "info")]
    pub verbose: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// HTTP/HTTPSキャプチャを開始
    Capture {
        /// リッスンアドレス
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        listen: String,

        /// ドメインフィルタ (include)
        #[arg(short = 'i', long = "include", value_delimiter = ',')]
        include_domains: Vec<String>,

        /// ドメインフィルタ (exclude)
        #[arg(short = 'e', long = "exclude", value_delimiter = ',')]
        exclude_domains: Vec<String>,

        /// ストレージ出力先
        #[arg(short, long, value_enum, default_value = "sqlite")]
        storage: Vec<StorageType>,

        /// 出力ディレクトリ
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,
    },
    /// CA証明書の管理
    Cert {
        #[command(subcommand)]
        action: CertAction,
    },
}

#[derive(Subcommand)]
pub enum CertAction {
    /// CA証明書を生成
    Generate {
        #[arg(short, long, default_value = "netcap CA")]
        common_name: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// CA証明書をエクスポート
    Export {
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
pub enum StorageType {
    Sqlite,
    Jsonl,
    Pcap,
}
```

**テストケース:**
- 正常系: `netcap capture` のデフォルト引数がパースされる
- 正常系: `netcap capture -i example.com,api.test.com` で複数ドメインがパースされる
- 正常系: `netcap cert generate -o ./ca.pem` がパースされる
- 異常系: 未知のサブコマンドでエラーが返る
- 異常系: 必須引数 (`cert export -o`) 欠落でエラーが返る

---

### Step 5.2: capture サブコマンド実装

- [ ] `crates/netcap-cli/src/commands/capture.rs` にキャプチャ実行ロジックを実装
- [ ] ProxyServer の構築・起動
- [ ] ドメインフィルタの設定
- [ ] ストレージバックエンドの初期化
- [ ] Ctrl+C による Graceful Shutdown

**対象ファイル:** `crates/netcap-cli/src/commands/capture.rs`

**サンプルコード:**
```rust
// crates/netcap-cli/src/commands/capture.rs
use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use netcap_core::config::ProxyConfig;
use netcap_core::filter::{DomainFilter, DomainMatcher, FilterRule, FilterType};
use netcap_core::filter::pattern::DomainPattern;
use netcap_core::proxy::ProxyServer;
use netcap_core::tls::ca::RcgenCaProvider;
use uuid::Uuid;

pub async fn execute(
    listen: &str,
    include_domains: &[String],
    exclude_domains: &[String],
    storage_types: &[super::super::args::StorageType],
    output_dir: &std::path::Path,
) -> Result<()> {
    // 1. CA証明書の準備
    let ca_path = output_dir.join("netcap-ca");
    let ca_provider = Arc::new(
        RcgenCaProvider::generate_ca("netcap CA", &ca_path)?
    );

    // 2. ドメインフィルタ設定
    let mut filter = DomainFilter::new();
    for domain in include_domains {
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: format!("include:{}", domain),
            filter_type: FilterType::Include,
            pattern: DomainPattern::new_wildcard(domain),
            priority: 100,
            enabled: true,
        });
    }
    for domain in exclude_domains {
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: format!("exclude:{}", domain),
            filter_type: FilterType::Exclude,
            pattern: DomainPattern::new_wildcard(domain),
            priority: 200,
            enabled: true,
        });
    }

    // 3. ストレージ初期化
    // storage_types に応じて FanoutWriter を構築

    // 4. ProxyServer 起動
    let config = ProxyConfig {
        listen_addr: listen.parse()?,
        ..Default::default()
    };

    let server = ProxyServer::builder()
        .config(config)
        .cert_provider(ca_provider)
        .domain_filter(Arc::new(filter))
        // .storage(...)
        .build()?;

    tracing::info!("Proxy listening on {}", listen);

    // 5. Ctrl+C で Graceful Shutdown
    tokio::select! {
        result = server.run() => {
            result?;
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down...");
            server.shutdown()?;
        }
    }

    Ok(())
}
```

---

### Step 5.3: cert サブコマンド実装

- [ ] `crates/netcap-cli/src/commands/cert.rs` に CA証明書の生成・エクスポートを実装
- [ ] `generate`: 新規CA証明書を生成してファイルに保存
- [ ] `export`: 既存CA証明書を PEM 形式でエクスポート

**対象ファイル:** `crates/netcap-cli/src/commands/cert.rs`

**テストケース:**
- 正常系: `cert generate` で PEM ファイルが生成される
- 正常系: `cert export` で既存証明書がコピーされる
- 異常系: 出力先が書き込み不可の場合のエラー

---

### Step 5.4: 標準出力ログフォーマッタ

- [ ] `crates/netcap-cli/src/output.rs` にリアルタイムログ出力フォーマッタを実装
- [ ] HTTP メソッド・ステータスコード・ドメイン・パス・レイテンシを1行で表示
- [ ] カラー出力対応 (ステータスコード別)

**対象ファイル:** `crates/netcap-cli/src/output.rs`

**サンプルコード:**
```rust
// crates/netcap-cli/src/output.rs
use netcap_core::capture::exchange::CapturedExchange;

pub fn format_exchange(exchange: &CapturedExchange) -> String {
    let req = &exchange.request;
    let status = exchange.response.as_ref()
        .map(|r| r.status.as_u16().to_string())
        .unwrap_or_else(|| "---".to_string());
    let latency = exchange.response.as_ref()
        .map(|r| format!("{:.1}ms", r.latency.as_secs_f64() * 1000.0))
        .unwrap_or_else(|| "---".to_string());
    let host = req.uri.host().unwrap_or("-");
    let path = req.uri.path();

    format!(
        "{} {} {}{} → {} ({})",
        req.method, host, path,
        req.uri.query().map(|q| format!("?{}", q)).unwrap_or_default(),
        status, latency
    )
}
```

**テストケース:**
- 正常系: GET リクエストが `GET example.com/api → 200 (12.3ms)` 形式で出力される
- 正常系: レスポンスなしの場合 `→ --- (---)` と表示される
- 正常系: クエリパラメータ付きURLが正しく表示される

---

### Step 5.5: main.rs エントリポイント & TOML 設定ファイル対応

- [ ] `crates/netcap-cli/src/main.rs` でコマンドディスパッチを実装
- [ ] TOML 設定ファイルの読み込みを実装 (toml クレート)
- [ ] CLI 引数と設定ファイルのマージ（CLI優先）
- [ ] tracing-subscriber の初期化

**対象ファイル:** `crates/netcap-cli/src/main.rs`

**追加依存:** `toml = "0.8"`

**サンプルコード:**
```rust
// crates/netcap-cli/src/main.rs
mod args;
mod commands;
mod output;

use args::{Cli, Commands};
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&cli.verbose)))
        .init();

    match cli.command {
        Commands::Capture {
            listen, include_domains, exclude_domains, storage, output_dir,
        } => {
            commands::capture::execute(
                &listen, &include_domains, &exclude_domains, &storage, &output_dir,
            ).await?;
        }
        Commands::Cert { action } => {
            commands::cert::execute(action).await?;
        }
    }

    Ok(())
}
```

---

### Step 5.6: 統合テスト & 設定テンプレート

- [ ] `tests/integration/proxy_test.rs` にプロキシ起動 → HTTP キャプチャの E2E テスト
- [ ] `tests/integration/filter_test.rs` にドメインフィルタの E2E テスト
- [ ] `tests/integration/storage_test.rs` に各ストレージバックエンドの E2E テスト
- [ ] `config/netcap.example.toml` に設定ファイルテンプレートを作成

**設定テンプレート:**
```toml
# config/netcap.example.toml

[proxy]
listen_addr = "127.0.0.1:8080"
max_connections = 1024
max_body_size = 10485760  # 10MB
request_timeout = 30

[session]
capture_request_body = true
capture_response_body = true
default_action = "capture"

[[filters]]
name = "capture-example"
type = "include"
pattern = "*.example.com"
pattern_type = "wildcard"
priority = 100

[[storage]]
type = "sqlite"
path = "./netcap.db"

[[storage]]
type = "jsonl"
path = "./netcap.jsonl"
rotate_size = 104857600  # 100MB
```

---

### Step 5.7: Phase 5 テスト・ビルド検証

- [ ] CLI の全サブコマンドのテスト実装（正常系・異常系）
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --release --bin netcap` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了すること
- [ ] Docker コンテナ内で `netcap capture --help` が正常出力されること
- [ ] Docker コンテナ内で `netcap capture` が起動し、HTTPリクエストがキャプチャされること
- [ ] **skip/TODO残留チェック:** `crates/netcap-cli/` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を検索し、残留があれば実装を完了させる
- [ ] **Phase 5 全機能検証チェックリスト (CLI コマンド):**
  - [ ] `netcap --help` がヘルプメッセージを表示すること
  - [ ] `netcap --version` がバージョンを表示すること
  - [ ] `netcap capture --help` がキャプチャ用ヘルプを表示すること
  - [ ] `netcap capture` がデフォルト設定 (127.0.0.1:8080) でプロキシ起動すること
  - [ ] `netcap capture -l 0.0.0.0:9090` で指定アドレスで起動すること
  - [ ] `netcap capture -i "*.example.com"` で include フィルタが適用されること
  - [ ] `netcap capture -e "*.ads.com"` で exclude フィルタが適用されること
  - [ ] `netcap capture -s sqlite -s jsonl` で複数ストレージに出力されること
  - [ ] `netcap capture -c custom.toml` でTOML設定ファイルが読み込まれること
  - [ ] `netcap cert generate -o ./ca.pem` でCA証明書が生成されること
  - [ ] `netcap cert export -o ./exported.pem` でCA証明書がエクスポートされること
  - [ ] Ctrl+C で Graceful Shutdown し、バッファがフラッシュされること
  - [ ] キャプチャ中のHTTP通信がstdoutにリアルタイム出力されること
  - [ ] 上記を実際に実行して動作確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase5_report.md` にまとめる（コマンド実行ログ、出力結果のスクリーンショットまたはテキスト）

---

## Phase 6: BigQuery ストレージ & TUI

**目的:** BigQuery Streaming Insert、ratatui TUI ダッシュボード、replay サブコマンドの実装

### Step 6.1: BigQuery ストレージ実装

- [ ] `crates/netcap-storage-bigquery/src/lib.rs` に `BigQueryStorage` と `StorageBackend` 実装
- [ ] `crates/netcap-storage-bigquery/src/schema.rs` に BigQuery テーブルスキーマ定義
- [ ] `crates/netcap-storage-bigquery/src/batch.rs` に バッチ挿入 & エクスポネンシャルバックオフリトライ
- [ ] JSONL フォールバック (3回リトライ失敗時)

**対象ファイル:**
```
crates/netcap-storage-bigquery/src/lib.rs
crates/netcap-storage-bigquery/src/schema.rs
crates/netcap-storage-bigquery/src/batch.rs
```

**サンプルコード (batch.rs):**
```rust
// crates/netcap-storage-bigquery/src/batch.rs
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 100;

pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RETRIES {
                    tracing::error!(
                        "BigQuery write failed after {} retries: {}",
                        MAX_RETRIES, e
                    );
                    return Err(e);
                }
                let delay = Duration::from_millis(
                    BASE_DELAY_MS * 2u64.pow(attempt - 1)
                );
                tracing::warn!(
                    "BigQuery write attempt {} failed: {}. Retrying in {:?}",
                    attempt, e, delay
                );
                sleep(delay).await;
            }
        }
    }
}
```

**テストケース:**
- 正常系: BigQuery への Streaming Insert が成功する (wiremock でモック)
- 正常系: バッチ挿入で複数レコードが一括送信される
- 正常系: リトライが設定回数まで実行される
- 異常系: 3回リトライ失敗で JSONL フォールバックが動作する
- 異常系: 認証失敗で `InitFailed` が返る
- 異常系: 不正なスキーマでの書き込みエラー

---

### Step 6.2: TUI ダッシュボード実装

- [ ] `crates/netcap-tui/src/main.rs` にエントリポイント
- [ ] `crates/netcap-tui/src/app.rs` にアプリケーション状態管理
- [ ] `crates/netcap-tui/src/ui/mod.rs` に UI レイアウト
- [ ] `crates/netcap-tui/src/ui/request_list.rs` にリクエスト一覧パネル
- [ ] `crates/netcap-tui/src/ui/detail_view.rs` にリクエスト/レスポンス詳細パネル
- [ ] `crates/netcap-tui/src/ui/status_bar.rs` にステータスバー
- [ ] `crates/netcap-tui/src/event.rs` にキーイベントハンドラ

**対象ファイル:**
```
crates/netcap-tui/src/main.rs
crates/netcap-tui/src/app.rs
crates/netcap-tui/src/ui/mod.rs
crates/netcap-tui/src/ui/request_list.rs
crates/netcap-tui/src/ui/detail_view.rs
crates/netcap-tui/src/ui/status_bar.rs
crates/netcap-tui/src/event.rs
```

**⚠️ ファイル分割提案:** UI コンポーネントは既にファイル分割済み。`app.rs` が状態管理 + イベント処理で300行を超える可能性あり。その場合、状態管理は `state.rs` に分離。

**サンプルコード (app.rs):**
```rust
// crates/netcap-tui/src/app.rs
use netcap_core::capture::exchange::CapturedExchange;

pub enum AppTab {
    RequestList,
    Detail,
}

pub struct App {
    pub tab: AppTab,
    pub exchanges: Vec<CapturedExchange>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub stats: CaptureStats,
}

pub struct CaptureStats {
    pub total_requests: u64,
    pub total_responses: u64,
    pub active_connections: u32,
    pub bytes_captured: u64,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: AppTab::RequestList,
            exchanges: Vec::new(),
            selected_index: 0,
            should_quit: false,
            stats: CaptureStats {
                total_requests: 0,
                total_responses: 0,
                active_connections: 0,
                bytes_captured: 0,
            },
        }
    }

    pub fn next(&mut self) {
        if self.selected_index < self.exchanges.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn add_exchange(&mut self, exchange: CapturedExchange) {
        self.stats.total_requests += 1;
        if exchange.response.is_some() {
            self.stats.total_responses += 1;
        }
        self.exchanges.push(exchange);
    }
}
```

**テストケース:**
- 正常系: `next()` / `previous()` でインデックスが正しく移動する
- 正常系: `add_exchange` で統計値が更新される
- 異常系: 空リストで `next()` がパニックしない
- 異常系: 0 で `previous()` がパニックしない

---

### Step 6.3: replay サブコマンド実装

- [ ] `crates/netcap-cli/src/commands/replay.rs` にキャプチャ済みリクエストの再送を実装
- [ ] SQLite / JSONL からキャプチャデータを読み込み
- [ ] reqwest でリクエスト再送
- [ ] 結果の比較表示

**対象ファイル:** `crates/netcap-cli/src/commands/replay.rs`

**追加依存 (netcap-cli):** `reqwest = { version = "0.12", features = ["json"] }`

**テストケース:**
- 正常系: JSONL ファイルからリクエストが読み込まれ再送される
- 正常系: SQLite DB からリクエストが読み込まれ再送される
- 異常系: 存在しないファイルでエラーが返る
- 異常系: ターゲットサーバ接続失敗時のエラーハンドリング

---

### Step 6.4: Phase 6 テスト・ビルド検証

- [ ] BigQuery ストレージのモックテスト実装
- [ ] TUI の状態管理テスト実装
- [ ] replay コマンドのテスト実装
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --release --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了し、コンテナ内で `netcap capture` / `netcap-tui` が起動すること
- [ ] **skip/TODO残留チェック:** `crates/netcap-storage-bigquery/`, `crates/netcap-tui/`, `crates/netcap-cli/src/commands/replay.rs` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を検索し、残留があれば実装を完了させる
- [ ] **Phase 6 全機能検証チェックリスト:**
  - [ ] BigQuery ストレージ:
    - [ ] `BigQueryStorage::new()` で初期化が成功すること (wiremock モック)
    - [ ] `write_batch()` で Streaming Insert が送信されること (モック検証)
    - [ ] リトライが最大3回実行されること (モック検証)
    - [ ] 3回失敗後に JSONL フォールバックが動作すること
  - [ ] TUI:
    - [ ] `netcap-tui` が起動し、ターミナルUIが表示されること
    - [ ] 上下キーでリクエスト一覧が選択できること
    - [ ] Enter で詳細ビューに切り替わること
    - [ ] `q` で終了すること
  - [ ] replay コマンド:
    - [ ] `netcap replay --from ./netcap.jsonl` でリクエストが再送されること
    - [ ] `netcap replay --from ./netcap.db` で SQLite からリクエストが読み込まれること
  - [ ] 上記を実際に実行して動作確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase6_report.md` にまとめる

---

## Phase 7: モバイル FFI & クロスプラットフォームビルド

**目的:** UniFFI バインディング、Android/iOS プロジェクト、クロスコンパイル CI/CD

### Step 7.1: UniFFI インターフェース定義

- [ ] `crates/netcap-ffi/src/netcap.udl` に UDL インターフェース定義を記述
- [ ] `NetcapProxy`, `FfiProxyConfig`, `FfiCaptureStats`, `FfiError` を定義
- [ ] `Cargo.toml` に `uniffi` の `build` feature を設定

**対象ファイル:**
```
crates/netcap-ffi/src/netcap.udl
crates/netcap-ffi/Cargo.toml
```

**サンプルコード (netcap.udl):**
```
// crates/netcap-ffi/src/netcap.udl
namespace netcap {};

dictionary FfiProxyConfig {
    u16 listen_port;
    string storage_path;
    sequence<string> domain_filters;
};

dictionary FfiCaptureStats {
    u64 total_requests;
    u64 total_responses;
    u32 active_connections;
    u64 bytes_captured;
};

[Error]
enum FfiError {
    "InitFailed",
    "ProxyError",
    "AlreadyRunning",
    "NotRunning",
};

interface NetcapProxy {
    [Throws=FfiError]
    constructor(FfiProxyConfig config);

    [Throws=FfiError]
    void start();

    [Throws=FfiError]
    void stop();

    [Throws=FfiError]
    string get_ca_certificate_pem();

    [Throws=FfiError]
    FfiCaptureStats get_stats();

    [Throws=FfiError]
    string get_capture_events(u64 offset, u64 limit);
};
```

---

### Step 7.2: FFI ラッパー実装

- [ ] `crates/netcap-ffi/src/lib.rs` に UniFFI エクスポートマクロを記述
- [ ] `crates/netcap-ffi/src/proxy.rs` に `NetcapProxy` のRust実装
- [ ] `crates/netcap-ffi/src/types.rs` に FFI 型変換
- [ ] `crates/netcap-ffi/src/error.rs` に `FfiError` 定義

**対象ファイル:**
```
crates/netcap-ffi/src/lib.rs
crates/netcap-ffi/src/proxy.rs
crates/netcap-ffi/src/types.rs
crates/netcap-ffi/src/error.rs
```

**サンプルコード (proxy.rs):**
```rust
// crates/netcap-ffi/src/proxy.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use netcap_core::proxy::ProxyServer;
use crate::error::FfiError;
use crate::types::{FfiProxyConfig, FfiCaptureStats};

pub struct NetcapProxy {
    server: Arc<Mutex<Option<ProxyServer>>>,
    runtime: tokio::runtime::Runtime,
}

impl NetcapProxy {
    pub fn new(config: FfiProxyConfig) -> Result<Self, FfiError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| FfiError::InitFailed(e.to_string()))?;

        // ProxyServer を構築
        let server = runtime.block_on(async {
            // config → ProxyConfig 変換
            // ProxyServer::builder().build()
            todo!()
        });

        Ok(Self {
            server: Arc::new(Mutex::new(Some(server))),
            runtime,
        })
    }

    pub fn start(&self) -> Result<(), FfiError> {
        let server = self.server.clone();
        self.runtime.spawn(async move {
            let guard = server.lock().await;
            if let Some(ref srv) = *guard {
                let _ = srv.run().await;
            }
        });
        Ok(())
    }

    pub fn stop(&self) -> Result<(), FfiError> {
        self.runtime.block_on(async {
            let guard = self.server.lock().await;
            if let Some(ref srv) = *guard {
                srv.shutdown().map_err(|e| FfiError::ProxyError(e.to_string()))
            } else {
                Err(FfiError::NotRunning)
            }
        })
    }
}
```

**テストケース:**
- 正常系: `NetcapProxy::new` でプロキシオブジェクトが生成される
- 正常系: `start()` → `stop()` のライフサイクルが正常動作する
- 正常系: `get_stats()` で統計情報が取得される
- 異常系: 不正なポート番号で `InitFailed` が返る
- 異常系: 2重起動で `AlreadyRunning` が返る
- 異常系: 起動前の `stop()` で `NotRunning` が返る

---

### Step 7.3: Android プロジェクト基盤

- [ ] `android/` ディレクトリに Gradle プロジェクトを作成
- [ ] `scripts/build-android.sh` に cargo-ndk ビルドスクリプトを作成
- [ ] `scripts/generate-bindings.sh` に UniFFI バインディング生成スクリプトを作成
- [ ] `android/app/src/main/kotlin/com/netcap/bridge/NetcapBridge.kt` にブリッジクラスを配置

**対象ファイル:**
```
android/build.gradle.kts
android/app/build.gradle.kts
android/settings.gradle.kts
scripts/build-android.sh
scripts/generate-bindings.sh
```

---

### Step 7.4: iOS プロジェクト基盤

- [ ] `ios/` ディレクトリに Xcode プロジェクト構成を作成
- [ ] `scripts/build-ios.sh` に iOS 向けビルドスクリプトを作成
- [ ] Xcode ビルドフェーズに cargo build を組み込むスクリプト

**対象ファイル:**
```
ios/NetCap/NetCap.xcodeproj/
scripts/build-ios.sh
```

---

### Step 7.5: CI/CD ワークフロー

- [ ] `.github/workflows/ci.yml` — テスト・lint・ビルド
- [ ] `.github/workflows/release.yml` — クロスコンパイルリリースビルド
- [ ] `.github/workflows/android.yml` — Android ビルド・APK 生成
- [ ] `.github/workflows/ios.yml` — iOS ビルド (macOS ランナー)

**対象ファイル:**
```
.github/workflows/ci.yml
.github/workflows/release.yml
.github/workflows/android.yml
.github/workflows/ios.yml
```

**サンプルコード (ci.yml):**
```yaml
# .github/workflows/ci.yml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
      - name: Test
        run: cargo test --workspace
      - name: Build
        run: cargo build --release --workspace

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Coverage
        run: cargo tarpaulin --workspace --out Xml
      - name: Check coverage threshold
        run: |
          COVERAGE=$(cargo tarpaulin --workspace --print-rust-flags 2>&1 | grep -oP '\d+\.\d+%' | head -1 | tr -d '%')
          echo "Coverage: ${COVERAGE}%"
          if (( $(echo "$COVERAGE < 90" | bc -l) )); then
            echo "Coverage below 90%!"
            exit 1
          fi
```

---

### Step 7.6: Phase 7 テスト・ビルド検証

- [ ] FFI ラッパーの単体テスト実装
- [ ] UniFFI バインディング生成が正常に完了すること
- [ ] テストカバレッジ90%以上を確認。未テスト部分を特定し追加テストを実装
- [ ] `cargo build --release --workspace` が正常完了すること
- [ ] `cargo test --workspace` が全テストパスすること
- [ ] `docker build` が正常完了すること
- [ ] Android ビルド (`cargo ndk -t arm64-v8a build`) が正常完了すること (CI でのみ確認可)
- [ ] GitHub Actions の全ワークフローが通ること
- [ ] **skip/TODO残留チェック:** `crates/netcap-ffi/` 内の `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `#[ignore]` を検索し、残留があれば実装を完了させる
- [ ] **Phase 7 全機能検証チェックリスト:**
  - [ ] FFI:
    - [ ] `NetcapProxy::new(config)` で FFI 経由でプロキシオブジェクトが生成されること
    - [ ] `start()` → `stop()` のライフサイクルが正常動作すること
    - [ ] `get_stats()` で統計情報が JSON で取得されること
    - [ ] `get_capture_events(offset, limit)` でイベントが JSON で取得されること
    - [ ] `get_ca_certificate_pem()` で PEM 文字列が返ること
  - [ ] UniFFI バインディング:
    - [ ] `scripts/generate-bindings.sh` で Kotlin/Swift バインディングが生成されること
    - [ ] 生成されたバインディングにコンパイルエラーがないこと
  - [ ] Android ビルド:
    - [ ] `scripts/build-android.sh` で arm64-v8a 向け .so が生成されること
  - [ ] CI/CD:
    - [ ] `.github/workflows/ci.yml` が push / PR で実行されること
    - [ ] `.github/workflows/release.yml` がタグ push でリリースビルドされること
  - [ ] 上記を実際に実行して動作確認
  - [ ] エラーが検出された場合、エラーが出なくなるまで修正を繰り返す
  - [ ] 正常動作のエビデンスを `docs/evidence/phase7_report.md` にまとめる

---

## Phase 8: install/uninstall スクリプト & 最終検証

**目的:** kalidokit-rust の setup.sh を参考にした install/uninstall スクリプト作成と、全機能の最終動作検証

### Step 8.1: install/uninstall スクリプト作成

- [ ] `scripts/setup.sh` を作成（[kalidokit-rust/scripts/setup.sh](https://github.com/tk-aria/kalidokit-rust/blob/main/scripts/setup.sh) を参考）
- [ ] 以下の機能を実装:
  - `install`: GitHub Releases からバイナリをダウンロードしてインストール
  - `uninstall`: バイナリと関連ファイルを削除
  - `version`: インストール済みバージョンを表示
- [ ] OS/アーキテクチャ自動検出 (Linux/macOS/Windows, x86_64/aarch64)
- [ ] インストール先のカスタマイズ (`NETCAP_INSTALL_PATH` 環境変数)
- [ ] パーミッション不足時の `~/.local/bin` フォールバック
- [ ] curl パイプインストール対応 (`curl -fsSL https://.../ | sh -s -- install`)

**対象ファイル:** `scripts/setup.sh`

**サンプルコード:**
```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="tk-aria/netcap"
BINARY_NAME="netcap"
INSTALL_DIR="${NETCAP_INSTALL_PATH:-/usr/local/bin}"
VERSION="${NETCAP_VERSION:-latest}"

# --- OS/Arch 検出 ---
detect_os() {
    case "$(uname -s)" in
        Linux*)   echo "linux" ;;
        Darwin*)  echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)        echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             echo "unknown" ;;
    esac
}

# --- バージョン取得 ---
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/'
}

# --- インストール ---
cmd_install() {
    local os=$(detect_os)
    local arch=$(detect_arch)
    local version="${VERSION}"

    if [ "${version}" = "latest" ]; then
        version=$(get_latest_version)
    fi

    echo "Installing ${BINARY_NAME} v${version} (${os}/${arch})..."

    local filename="${BINARY_NAME}-v${version}-${arch}-${os}"
    [ "${os}" = "windows" ] && filename="${filename}.exe"
    local url="https://github.com/${REPO}/releases/download/v${version}/${filename}.tar.gz"

    local tmp=$(mktemp -d)
    trap "rm -rf ${tmp}" EXIT

    curl -fsSL "${url}" -o "${tmp}/archive.tar.gz"
    tar xzf "${tmp}/archive.tar.gz" -C "${tmp}"

    if install -m 755 "${tmp}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null; then
        echo "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
    else
        local fallback="${HOME}/.local/bin"
        mkdir -p "${fallback}"
        install -m 755 "${tmp}/${BINARY_NAME}" "${fallback}/${BINARY_NAME}"
        echo "Installed to ${fallback}/${BINARY_NAME}"
        echo "Add ${fallback} to your PATH if needed."
    fi

    echo "${BINARY_NAME} v${version} installed successfully."
}

# --- アンインストール ---
cmd_uninstall() {
    local targets=(
        "${INSTALL_DIR}/${BINARY_NAME}"
        "${HOME}/.local/bin/${BINARY_NAME}"
    )
    for path in "${targets[@]}"; do
        if [ -f "${path}" ]; then
            rm -f "${path}"
            echo "Removed: ${path}"
        fi
    done
    echo "${BINARY_NAME} uninstalled."
}

# --- バージョン表示 ---
cmd_version() {
    if command -v "${BINARY_NAME}" &>/dev/null; then
        ${BINARY_NAME} --version
    else
        echo "${BINARY_NAME} is not installed."
    fi
}

# --- メイン ---
case "${1:-install}" in
    install)   cmd_install ;;
    uninstall) cmd_uninstall ;;
    version)   cmd_version ;;
    *)
        echo "Usage: $0 {install|uninstall|version}"
        exit 1
        ;;
esac
```

**テストケース:**
- 正常系: `./scripts/setup.sh install` でバイナリがインストールされること
- 正常系: `./scripts/setup.sh version` でバージョンが表示されること
- 正常系: `./scripts/setup.sh uninstall` でバイナリが削除されること
- 正常系: `NETCAP_INSTALL_PATH=/tmp/test` で指定先にインストールされること
- 異常系: 存在しないバージョンを指定するとエラーメッセージが表示されること

---

### Step 8.2: 最終 skip/TODO 全体スキャン

- [ ] ワークスペース全体で以下のパターンを `grep -rn` で検索し、**残留ゼロ**にする:
  ```bash
  grep -rn 'todo!()' crates/
  grep -rn 'unimplemented!()' crates/
  grep -rn '// TODO' crates/
  grep -rn '// FIXME' crates/
  grep -rn '// HACK' crates/
  grep -rn '#\[ignore\]' crates/
  grep -rn 'skip' crates/ --include='*.rs' | grep -i 'test'
  ```
- [ ] 残留が見つかった場合は実装を完了させ、`todo!()` / `unimplemented!()` を除去する
- [ ] `#[ignore]` 付きテストは理由を確認し、可能なら `#[ignore]` を外して実行可能にする

---

### Step 8.3: 全機能 最終動作検証チェックリスト

以下の全項目を実際に実行し、動作確認を行う。エラーが検出された場合はエラーが出なくなるまで修正を繰り返す。

#### CLI コマンド
- [ ] `netcap --help`
- [ ] `netcap --version`
- [ ] `netcap capture --help`
- [ ] `netcap capture` (デフォルト起動 → Ctrl+C で停止)
- [ ] `netcap capture -l 127.0.0.1:9090 -i "*.example.com" -s sqlite -s jsonl -o /tmp/test`
- [ ] `netcap capture -e "*.ads.com,*.tracking.com"`
- [ ] `netcap capture -c config/netcap.example.toml`
- [ ] `netcap cert generate -o /tmp/ca.pem`
- [ ] `netcap cert export -o /tmp/ca_export.pem`
- [ ] `netcap replay --from /tmp/test/netcap.jsonl` (Phase 6 以降)
- [ ] `netcap-tui` (Phase 6 以降)

#### HTTP プロキシ動作
- [ ] プロキシ起動後、`curl -x http://127.0.0.1:8080 http://example.com` でHTTPキャプチャされること
- [ ] `curl -x http://127.0.0.1:8080 --proxy-cacert ca.pem https://example.com` でHTTPSキャプチャされること
- [ ] include フィルタで指定ドメインのみキャプチャされること
- [ ] exclude フィルタで除外ドメインがパススルーされること
- [ ] stdout にリアルタイムでキャプチャログが表示されること

#### ストレージ出力
- [ ] SQLite: `netcap.db` が生成され、`sqlite3 netcap.db "SELECT count(*) FROM http_requests"` でレコード数が確認できること
- [ ] JSONL: `netcap.jsonl` が生成され、各行が有効な JSON であること (`cat netcap.jsonl | jq . > /dev/null`)
- [ ] PCAP: `netcap.pcap` が生成され、ファイルサイズが0でないこと

#### ビルド・Docker
- [ ] `cargo build --release --workspace` 成功
- [ ] `cargo test --workspace` 全パス
- [ ] `cargo clippy --workspace -- -D warnings` 警告ゼロ
- [ ] `docker build -t netcap .` 成功
- [ ] `docker run --rm netcap --help` が正常出力

#### install/uninstall スクリプト
- [ ] `./scripts/setup.sh install` 成功
- [ ] `netcap --version` でバージョン表示
- [ ] `./scripts/setup.sh uninstall` でバイナリ削除

---

### Step 8.4: 最終エビデンスレポート作成

- [ ] `docs/evidence/final_report.md` に以下を記載:
  - 全 CLI コマンドの実行結果（コマンドと出力をコードブロックで記載）
  - HTTP プロキシ動作のキャプチャログサンプル
  - 各ストレージの出力サンプル（SQLite レコード数、JSONL 先頭3行、PCAP ファイルサイズ）
  - `cargo test` の全テスト結果
  - `cargo tarpaulin` のカバレッジ数値
  - `docker build` / `docker run` のログ
  - skip/TODO スキャン結果（残留ゼロの証跡）
  - install/uninstall スクリプトの動作ログ
- [ ] レポートの形式: Markdown、各セクションにコマンド実行ログをコードブロックで記載

**レポートテンプレート:**
```markdown
# netcap 最終動作検証レポート

> 検証日: YYYY-MM-DD
> バージョン: 0.1.0
> 検証者: (name)

## 1. skip/TODO スキャン結果

\`\`\`bash
$ grep -rn 'todo!()' crates/
(出力なし = 残留ゼロ)

$ grep -rn 'unimplemented!()' crates/
(出力なし = 残留ゼロ)
\`\`\`

## 2. ビルド検証

\`\`\`bash
$ cargo build --release --workspace
   Compiling netcap-core v0.1.0
   ...
   Finished release [optimized] target(s) in XXs
\`\`\`

## 3. テスト結果

\`\`\`bash
$ cargo test --workspace
   running XX tests
   test result: ok. XX passed; 0 failed; 0 ignored
\`\`\`

## 4. カバレッジ

\`\`\`bash
$ cargo tarpaulin --workspace
   XX.X% coverage, XX/XX lines covered
\`\`\`

## 5. CLI コマンド動作確認

### netcap --help
\`\`\`
$ netcap --help
(出力)
\`\`\`

### netcap capture
\`\`\`
$ netcap capture -l 127.0.0.1:8080
Proxy listening on 127.0.0.1:8080
GET example.com/ → 200 (12.3ms)
...
^C Shutting down...
\`\`\`

(以下各コマンド同様)

## 6. ストレージ出力確認

## 7. Docker 検証

## 8. install/uninstall スクリプト

## 結果サマリ

| 項目 | 結果 |
|------|------|
| ビルド | PASS |
| テスト | PASS (XX/XX) |
| カバレッジ | XX.X% (>90%) |
| CLI コマンド | 全コマンド PASS |
| ストレージ | SQLite/JSONL/PCAP PASS |
| Docker | PASS |
| skip/TODO残留 | 0件 |
| install/uninstall | PASS |
```

---

## 依存ライブラリバージョン一覧

| ライブラリ | バージョン | 用途 |
|-----------|-----------|------|
| tokio | 1.50 | 非同期ランタイム |
| async-trait | 0.1 | 非同期 trait サポート |
| hyper | 1.x | HTTP ライブラリ |
| hyper-util | 0.1 | hyper ユーティリティ |
| http | 1.x | HTTP 型定義 |
| bytes | 1.x | バイトバッファ |
| rustls | 0.23 | TLS ライブラリ |
| rcgen | 0.14 | X.509 証明書生成 |
| hudsucker | 0.21 | MITM プロキシ |
| serde | 1.0 | シリアライゼーション |
| serde_json | 1.0 | JSON |
| chrono | 0.4 | 日時処理 |
| uuid | 1.x (v7) | UUID 生成 |
| thiserror | 2.x | エラー型 derive |
| anyhow | 1.x | エラーハンドリング |
| tracing | 0.1 | 構造化ログ |
| tracing-subscriber | 0.3 | ログ出力 |
| regex | 1.x | 正規表現 |
| rusqlite | 0.32 (bundled) | SQLite |
| pcap-file | 2.x | PCAP ファイル出力 |
| gcp-bigquery-client | 0.27 | BigQuery 連携 |
| uniffi | 0.28 | FFI バインディング |
| clap | 4.5 | CLI 引数パーサー |
| ratatui | 0.29 | TUI フレームワーク |
| crossterm | 0.28 | ターミナル操作 |
| dashmap | 6.x | 並行 HashMap |
| flate2 | 1.x | gzip/deflate |
| brotli | 7.x | Brotli 解凍 |
| toml | 0.8 | TOML 設定ファイル |
| reqwest | 0.12 | HTTP クライアント (replay) |
| tokio-test | 0.4 | テスト |
| wiremock | 0.6 | HTTP モック |
| tempfile | 3.x | テスト用一時ファイル |
| cargo-tarpaulin | 最新 | カバレッジ計測 |
