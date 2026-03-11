# Step 1.6: lib.rs と mod 宣言 - 作業報告

## 完了日時
2026-03-11

## 作業内容
- `lib.rs` に全モジュールの pub mod 宣言:
  - capture, config, error, filter, proxy, storage, tls
- `storage/mod.rs` に `StorageBackend` trait と `FanoutWriter` を実装:
  - `StorageBackend` trait (async: initialize, write, write_batch, flush, close)
  - `FanoutWriter` - 複数バックエンドへの並行書き出し
- `tls/mod.rs` に `CertificateProvider` trait と証明書構造体を定義:
  - `CaCertificate`, `ServerCertificate` 構造体
  - `CertificateProvider` trait (get_or_create_ca, issue_server_cert, export_ca_pem)
- proxy/mod.rs, tls/ca.rs, tls/server_cert.rs, tls/store.rs はスケルトン

## ビルド確認
- cargo check --workspace → OK
- cargo clippy -p netcap-core -- -D warnings → 0 warnings
