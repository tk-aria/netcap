# Phase 3: プロキシエンジン実装 - 機能検証レポート

## 検証日時
2026-03-12 15:45 JST

## テスト結果サマリ

- **Unit Tests**: 95 passed, 0 failed
- **Integration Tests**: 32 passed, 0 failed
- **Total**: 127 tests, all passing
- **`cargo build --workspace`**: SUCCESS
- **`cargo test --workspace`**: SUCCESS

## TODO/FIXME 残留チェック

```
$ grep -rE 'todo!\(\)|unimplemented!\(\)|// TODO|// FIXME|#\[ignore\]' crates/netcap-core/src/proxy/ crates/netcap-core/src/storage/
(no matches found)
```

## Phase 3 機能検証チェックリスト

### ProxyServerBuilder (proxy/mod.rs)
- [x] `ProxyServerBuilder` で全フィールドを設定して `build()` が成功する
  - テスト: `builder_success`, `builder_with_custom_config`, `proxy_builder_all_fields_build_success`
- [x] cert_provider 未設定で `build()` がエラーを返す
  - テスト: `builder_missing_cert_provider`
- [x] storage 未設定で `build()` がエラーを返す
  - テスト: `builder_missing_storage`

### ProxyServer::run() (proxy/mod.rs)
- [x] `ProxyServer::run()` でプロキシが起動する
  - テスト: `run_and_shutdown`, `proxy_run_and_graceful_shutdown`
- [x] hudsucker ProxyBuilder でプロキシを構築
- [x] RcgenAuthority を使った TLS 証明書統合
- [x] CaptureBuffer + StorageDispatcher によるイベントパイプライン

### NetcapHandler (proxy/handler.rs)
- [x] リクエストをキャプチャし `CapturedExchange` を生成する
  - テスト: `capture_request_creates_exchange`
- [x] レスポンスをキャプチャし `CapturedExchange` を生成する
  - テスト: `capture_response_creates_exchange`
- [x] ボディトランケーションが正しく動作する
  - テスト: `truncate_body_no_truncation`, `truncate_body_with_truncation`, `capture_response_truncation`
- [x] ホスト抽出が URI とヘッダから正しく動作する
  - テスト: `extract_host_from_uri`, `extract_host_from_header`, `extract_host_empty`

### ドメインフィルタ連携
- [x] Include ルールで Capture が返される
  - テスト: `filter_include_evaluates`
- [x] Exclude ルールで Passthrough が返される
  - テスト: `filter_exclude_evaluates`
- [x] Include/Exclude/Default の統合テスト
  - テスト: `domain_filter_include_exclude_default_integration`

### ConnectionTracker (proxy/connection.rs)
- [x] 接続を追跡・取得できる
  - テスト: `track_and_get`, `connection_tracker_tracks_and_counts`
- [x] リクエストカウントをインクリメントできる
  - テスト: `increment_request_count`
- [x] 接続クローズが記録される
  - テスト: `close_connection`
- [x] アクティブ接続数が正しくカウントされる
  - テスト: `active_count`

### BufferSender / BufferReceiver (storage/buffer.rs)
- [x] イベントの送受信が正しく動作する
  - テスト: `send_and_recv`, `buffer_sender_receiver_event_flow`
- [x] バッチ受信が max_size まで取得する
  - テスト: `recv_batch_up_to_max`
- [x] バッファフル時に try_send がエラーを返す
  - テスト: `try_send_full_buffer`
- [x] Sender ドロップ時に空バッチが返る
  - テスト: `sender_drop_returns_empty`

### StorageDispatcher (storage/dispatcher.rs)
- [x] 複数バックエンドへ並行書き出しする
  - テスト: `dispatch_to_multiple_backends`, `dispatcher_writes_to_multiple_backends`
- [x] 失敗バックエンドが他に影響しない
  - テスト: `failing_backend_does_not_affect_others`
- [x] Sender ドロップ時にディスパッチャが終了する
  - テスト: `empty_on_sender_drop`

### Graceful Shutdown
- [x] `shutdown()` で broadcast 信号が送信される
  - テスト: `shutdown_sends_signal`
- [x] run() → shutdown() → 正常終了のフルフロー
  - テスト: `run_and_shutdown`, `proxy_run_and_graceful_shutdown`

## http バージョン変換
hudsucker 0.21 は hyper 0.14 (http 0.2) を使用するが、ワークスペースは http 1.x を使用。
`handler.rs` 内の `convert` モジュールで以下の変換を実装:
- Method, Uri, Version, StatusCode, HeaderMap
- テスト: `convert_method`, `convert_status`, `convert_version`

## 変更ファイル一覧

| ファイル | 内容 |
|---------|------|
| `crates/netcap-core/src/proxy/mod.rs` | ProxyServer, ProxyServerBuilder, run(), shutdown() |
| `crates/netcap-core/src/proxy/handler.rs` | NetcapHandler, HttpHandler 実装, convert モジュール |
| `crates/netcap-core/src/proxy/connection.rs` | ConnectionInfo, ConnectionTracker |
| `crates/netcap-core/src/storage/mod.rs` | StorageBackend trait, FanoutWriter, buffer/dispatcher モジュール |
| `crates/netcap-core/src/storage/buffer.rs` | CaptureBuffer, BufferSender, BufferReceiver |
| `crates/netcap-core/src/storage/dispatcher.rs` | StorageDispatcher |
| `crates/netcap-core/src/tls/ca.rs` | pem_to_der() pub化, アクセサ追加 |
| `crates/netcap-core/tests/integration_test.rs` | Phase 3 統合テスト追加 |
