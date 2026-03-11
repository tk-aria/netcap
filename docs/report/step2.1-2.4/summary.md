# Steps 2.1-2.4: TLS & 証明書管理 作業報告

**実行日時:** 2026-03-12 07:50 JST

## Step 2.1: CertificateProvider trait 定義
- `crates/netcap-core/src/tls/mod.rs` に既に実装済み（Phase 1で作成）
- `ServerCertificate` に `#[derive(Debug, Clone)]` を追加

## Step 2.2: CA証明書生成・管理
- `crates/netcap-core/src/tls/ca.rs` に `RcgenCaProvider` を実装
- rcgen 0.14 API に合わせた実装:
  - `generate_ca()`: CA証明書の生成
  - `load_from_files()`: ファイルからの読み込み
  - `CertificateProvider` trait 実装
- テスト5件: generate, get_or_create, export/reload, nonexistent file, issue via provider

## Step 2.3: 動的サーバー証明書発行
- `crates/netcap-core/src/tls/server_cert.rs` に `issue_server_certificate()` を実装
- rcgen 0.14 の `Issuer` API を使用してCA署名
- SAN にドメインを設定
- テスト4件: success, wildcard, invalid CA, multiple domains

## Step 2.4: 証明書キャッシュ (TTL付き)
- `crates/netcap-core/src/tls/store.rs` に `CertificateCache` を実装
- `DashMap` による並行安全なキャッシュ
- TTL ベースの期限切れ管理
- テスト6件: insert/get, nonexistent, ttl, clear, independent, overwrite

## 実行コマンド
```bash
cargo check -p netcap-core
cargo test -p netcap-core
cargo clippy -p netcap-core -- -D warnings
```

## 結果
- 64 unit tests + 26 integration tests = 90 tests 全パス
- clippy 警告ゼロ
