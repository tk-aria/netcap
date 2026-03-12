# Step 3.5: ProxyServer.run() の完全実装 - 作業レポート

## 完了日時
2026-03-12 15:30 JST

## 実装内容

### ProxyServer.run() メソッド (proxy/mod.rs)
- `CertificateProvider::get_or_create_ca()` で CA 証明書を取得
- PEM → DER 変換 (`RcgenCaProvider::pem_to_der()`)
- `hudsucker::certificate_authority::RcgenAuthority` 構築
- `CaptureBuffer::new()` でイベントチャネル作成
- `StorageDispatcher` をバックグラウンドタスクとして起動
- `NetcapHandler` を HttpHandler として登録
- `hudsucker::Proxy::builder()` チェーンでプロキシ構築
- `broadcast::channel` による Graceful Shutdown 実装

### BufferSender.into_inner() 追加 (storage/buffer.rs)
- `BufferSender` の内部 `mpsc::Sender` を公開する `into_inner()` メソッド追加
- handler が `mpsc::Sender<CapturedExchange>` を期待するため必要

### TLS プロバイダ統合 (tls/ca.rs)
- `pem_to_der()` を `pub` に変更（proxy/mod.rs から呼び出すため）
- `ca_cert_pem()`, `ca_key_pem()` アクセサメソッド追加

## テスト結果
- 95 unit tests: PASS
- 26 integration tests: PASS
- `run_and_shutdown` テスト: プロキシ起動 → shutdown 信号 → 正常停止を確認

## 変更ファイル
- `crates/netcap-core/src/proxy/mod.rs` - run() 完全実装、run_and_shutdown テスト
- `crates/netcap-core/src/proxy/handler.rs` - NetcapHandler (変更なし、mpsc::Sender のまま)
- `crates/netcap-core/src/storage/buffer.rs` - into_inner() 追加
- `crates/netcap-core/src/tls/ca.rs` - pem_to_der() pub化、アクセサ追加
