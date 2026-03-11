# ディレクトリ構成

## 1. プロジェクトルート全体構成

```
netcap/
├── Cargo.toml                          # ワークスペースルート定義
├── Cargo.lock                          # 依存関係ロックファイル
├── rust-toolchain.toml                 # Rustツールチェイン指定
├── .gitignore
├── LICENSE
├── README.md
│
├── crates/                             # Rustクレート群
│   ├── netcap-core/                    # コアライブラリ
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # クレートルート、pub mod 宣言
│   │       ├── proxy/
│   │       │   ├── mod.rs              # ProxyServer, ProxyServerBuilder
│   │       │   ├── handler.rs          # hudsucker HttpHandler 実装
│   │       │   └── connection.rs       # コネクション管理
│   │       ├── tls/
│   │       │   ├── mod.rs              # CertificateProvider trait
│   │       │   ├── ca.rs               # CA証明書の生成・管理
│   │       │   ├── server_cert.rs      # 動的サーバー証明書発行
│   │       │   └── store.rs            # 証明書キャッシュ
│   │       ├── capture/
│   │       │   ├── mod.rs              # CaptureHandler trait
│   │       │   ├── exchange.rs         # CapturedRequest, CapturedResponse, CapturedExchange
│   │       │   └── body.rs             # ボディ処理（圧縮解凍、サイズ制限）
│   │       ├── filter/
│   │       │   ├── mod.rs              # DomainMatcher trait, DomainFilter 実装
│   │       │   └── pattern.rs          # DomainPattern, ワイルドカード・正規表現マッチ
│   │       ├── storage/
│   │       │   └── mod.rs              # StorageBackend trait, StorageConfig trait, FanoutWriter
│   │       ├── config.rs               # ProxyConfig 等の設定構造体
│   │       └── error.rs                # CaptureError, ProxyError, StorageError, CertError
│   │
│   ├── netcap-storage-sqlite/          # SQLiteストレージ実装
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # SqliteStorage, SqliteStorageConfig
│   │       ├── schema.rs              # テーブル定義・マイグレーション
│   │       └── queries.rs              # SQL クエリ定数
│   │
│   ├── netcap-storage-jsonl/           # JSONLファイル出力実装
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # JsonlStorage, JsonlStorageConfig
│   │       ├── serializer.rs           # CapturedExchange → JSON 変換
│   │       └── rotation.rs             # ファイルローテーション
│   │
│   ├── netcap-storage-pcap/            # PCAP出力実装
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # PcapStorage, PcapStorageConfig
│   │       └── converter.rs            # HTTP → PCAPパケット変換
│   │
│   ├── netcap-storage-bigquery/        # BigQuery連携実装
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # BigQueryStorage, BigQueryStorageConfig
│   │       ├── schema.rs              # BigQueryテーブルスキーマ定義
│   │       └── batch.rs               # バッチ挿入・リトライロジック
│   │
│   ├── netcap-ffi/                     # FFIバインディング（Android/iOS向け）
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                  # UniFFI エクスポート
│   │   │   ├── proxy.rs               # NetcapProxy FFIラッパー
│   │   │   ├── types.rs               # FfiProxyConfig, FfiCaptureStats 等
│   │   │   └── error.rs               # FfiError
│   │   ├── src/netcap.udl             # UniFFI インターフェース定義
│   │   └── uniffi-bindgen.rs          # バインディング生成スクリプト
│   │
│   ├── netcap-cli/                     # CLIアプリケーション
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # エントリポイント
│   │       ├── args.rs                 # clap による引数定義
│   │       ├── commands/
│   │       │   ├── mod.rs              # サブコマンドルーティング
│   │       │   ├── capture.rs          # capture サブコマンド（メイン機能）
│   │       │   ├── cert.rs             # cert サブコマンド（証明書管理）
│   │       │   └── replay.rs           # replay サブコマンド（キャプチャ再生）
│   │       └── output.rs              # コンソール出力フォーマッタ
│   │
│   └── netcap-tui/                     # TUIアプリケーション（オプション）
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                 # エントリポイント
│           ├── app.rs                  # アプリケーション状態管理
│           ├── ui/
│           │   ├── mod.rs              # UI レイアウト
│           │   ├── request_list.rs     # リクエスト一覧パネル
│           │   ├── detail_view.rs      # リクエスト/レスポンス詳細パネル
│           │   └── status_bar.rs       # ステータスバー
│           └── event.rs               # キーイベントハンドラ
│
├── tests/                              # ワークスペースレベルの統合テスト
│   ├── integration/
│   │   ├── proxy_test.rs              # プロキシ起動・HTTP キャプチャE2E
│   │   ├── tls_test.rs               # HTTPS キャプチャE2E
│   │   ├── filter_test.rs            # ドメインフィルタE2E
│   │   └── storage_test.rs           # 各ストレージバックエンドE2E
│   └── fixtures/
│       ├── certs/                      # テスト用証明書
│       ├── requests/                   # テスト用HTTPリクエスト/レスポンス
│       └── configs/                    # テスト用設定ファイル
│
├── examples/                           # 使用例
│   ├── basic_capture.rs               # 最小限のキャプチャ例
│   ├── filtered_capture.rs            # ドメインフィルタ付きキャプチャ
│   ├── multi_storage.rs              # 複数ストレージ同時出力
│   └── custom_handler.rs             # カスタムCaptureHandler実装
│
├── android/                            # Androidプロジェクト
│   ├── app/
│   │   ├── build.gradle.kts
│   │   └── src/
│   │       └── main/
│   │           ├── AndroidManifest.xml
│   │           ├── kotlin/
│   │           │   └── com/netcap/
│   │           │       ├── MainActivity.kt
│   │           │       ├── service/
│   │           │       │   └── CaptureVpnService.kt   # VPN Service（ローカルプロキシ経由）
│   │           │       ├── ui/
│   │           │       │   ├── MainScreen.kt
│   │           │       │   └── CaptureDetailScreen.kt
│   │           │       └── bridge/
│   │           │           └── NetcapBridge.kt         # UniFFI生成バインディングの利用
│   │           └── res/
│   ├── gradle/
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   └── gradle.properties
│
├── ios/                                # iOSプロジェクト
│   ├── NetCap/
│   │   ├── NetCap.xcodeproj/
│   │   ├── NetCap/
│   │   │   ├── App/
│   │   │   │   ├── NetCapApp.swift
│   │   │   │   └── ContentView.swift
│   │   │   ├── Service/
│   │   │   │   └── PacketTunnelProvider.swift  # Network Extension（ローカルプロキシ経由）
│   │   │   ├── Bridge/
│   │   │   │   └── NetcapBridge.swift          # UniFFI生成バインディングの利用
│   │   │   └── Views/
│   │   │       ├── CaptureListView.swift
│   │   │       └── CaptureDetailView.swift
│   │   └── PacketTunnel/
│   │       ├── Info.plist
│   │       └── PacketTunnelProvider.swift
│   └── NetCap.xcworkspace/
│
├── docs/                               # ドキュメント
│   ├── 01_requirements.md             # 要件定義
│   ├── 02_system_overview.md          # システム全体設計
│   ├── 03_platform_design.md          # プラットフォーム別設計
│   ├── 04_module_architecture.md      # モジュール構成（本ドキュメントと対）
│   ├── 05_directory_structure.md      # ディレクトリ構成（本ドキュメント）
│   ├── 06_api_reference.md            # API リファレンス
│   └── 07_deployment.md               # デプロイ・配布手順
│
├── .github/                            # GitHub Actions CI/CD
│   ├── workflows/
│   │   ├── ci.yml                     # テスト・lint・ビルド
│   │   ├── release.yml                # リリースビルド（クロスコンパイル）
│   │   ├── android.yml                # Androidビルド・APK生成
│   │   └── ios.yml                    # iOSビルド（macOSランナー）
│   └── dependabot.yml                 # 依存関係自動更新
│
├── scripts/                            # ビルド・ユーティリティスクリプト
│   ├── build-android.sh               # Android向けクロスコンパイル
│   ├── build-ios.sh                   # iOS向けクロスコンパイル
│   ├── generate-bindings.sh           # UniFFIバインディング生成
│   └── setup-dev.sh                   # 開発環境セットアップ
│
└── config/                             # 設定ファイルテンプレート
    ├── netcap.example.toml            # 設定ファイル例
    └── filters.example.txt            # ドメインフィルタ例
```

## 2. Cargo.toml ワークスペース設定

### ルート Cargo.toml

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
repository = "https://github.com/example/netcap"

[workspace.dependencies]
# 非同期ランタイム
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# HTTP / ネットワーク
hyper = { version = "1", features = ["full"] }
hyper-util = "0.1"
http = "1"
bytes = "1"

# TLS
rustls = "0.23"
rcgen = "0.13"

# MITM プロキシ
hudsucker = "0.21"

# シリアライズ
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 日時・UUID
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }

# エラー
thiserror = "2"
anyhow = "1"

# ログ
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 正規表現
regex = "1"

# ストレージ: SQLite
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"

# ストレージ: PCAP
pcap-file = "2"

# ストレージ: BigQuery
gcp-bigquery-client = "0.22"

# FFI
uniffi = "0.28"

# CLI
clap = { version = "4", features = ["derive"] }

# TUI
ratatui = "0.29"
crossterm = "0.28"

# テスト
tokio-test = "0.4"
wiremock = "0.6"
tempfile = "3"
```

### crates/netcap-core/Cargo.toml

```toml
[package]
name = "netcap-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
tokio = { workspace = true }
async-trait = { workspace = true }
hyper = { workspace = true }
http = { workspace = true }
bytes = { workspace = true }
rustls = { workspace = true }
rcgen = { workspace = true }
hudsucker = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
regex = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
tempfile = { workspace = true }
```

### crates/netcap-storage-sqlite/Cargo.toml

```toml
[package]
name = "netcap-storage-sqlite"
version.workspace = true
edition.workspace = true

[dependencies]
netcap-core = { path = "../netcap-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
rusqlite = { workspace = true }
r2d2 = { workspace = true }
r2d2_sqlite = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
tempfile = { workspace = true }
```

### crates/netcap-storage-jsonl/Cargo.toml

```toml
[package]
name = "netcap-storage-jsonl"
version.workspace = true
edition.workspace = true

[dependencies]
netcap-core = { path = "../netcap-core" }
tokio = { workspace = true, features = ["fs", "io-util"] }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
flate2 = "1"

[dev-dependencies]
tokio-test = { workspace = true }
tempfile = { workspace = true }
```

### crates/netcap-storage-pcap/Cargo.toml

```toml
[package]
name = "netcap-storage-pcap"
version.workspace = true
edition.workspace = true

[dependencies]
netcap-core = { path = "../netcap-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
pcap-file = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
tempfile = { workspace = true }
```

### crates/netcap-storage-bigquery/Cargo.toml

```toml
[package]
name = "netcap-storage-bigquery"
version.workspace = true
edition.workspace = true

[dependencies]
netcap-core = { path = "../netcap-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
gcp-bigquery-client = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
wiremock = { workspace = true }
```

### crates/netcap-ffi/Cargo.toml

```toml
[package]
name = "netcap-ffi"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "staticlib"]
name = "netcap_ffi"

[dependencies]
netcap-core = { path = "../netcap-core" }
netcap-storage-sqlite = { path = "../netcap-storage-sqlite" }
netcap-storage-jsonl = { path = "../netcap-storage-jsonl" }
netcap-storage-pcap = { path = "../netcap-storage-pcap" }
tokio = { workspace = true }
uniffi = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[build-dependencies]
uniffi = { workspace = true, features = ["build"] }
```

### crates/netcap-cli/Cargo.toml

```toml
[package]
name = "netcap-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "netcap"
path = "src/main.rs"

[dependencies]
netcap-core = { path = "../netcap-core" }
netcap-storage-sqlite = { path = "../netcap-storage-sqlite" }
netcap-storage-jsonl = { path = "../netcap-storage-jsonl" }
netcap-storage-pcap = { path = "../netcap-storage-pcap" }
netcap-storage-bigquery = { path = "../netcap-storage-bigquery" }
tokio = { workspace = true }
clap = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
serde = { workspace = true }
toml = "0.8"
```

### crates/netcap-tui/Cargo.toml

```toml
[package]
name = "netcap-tui"
version.workspace = true
edition.workspace = true

[[bin]]
name = "netcap-tui"
path = "src/main.rs"

[dependencies]
netcap-core = { path = "../netcap-core" }
netcap-storage-sqlite = { path = "../netcap-storage-sqlite" }
netcap-storage-jsonl = { path = "../netcap-storage-jsonl" }
netcap-storage-pcap = { path = "../netcap-storage-pcap" }
tokio = { workspace = true }
ratatui = { workspace = true }
crossterm = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
```

## 3. rust-toolchain.toml

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
