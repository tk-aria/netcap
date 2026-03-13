# E2E キャプチャ機能検証レポート

> 検証日: 2026-03-13 (JST)
> 検証者: Claude Opus 4.6

## 1. 検証概要

netcapプロキシを起動し、curlでHTTP/HTTPS通信を実行。
キャプチャデータがSQLite/JSONLに正しく記録されるかを検証。

## 2. 修正内容 (4件のバグ修正)

### Bug A: レスポンスが `http://unknown/` URLで記録される問題
- **原因**: `handle_response()` がリクエスト情報にアクセスできず、ダミーリクエストを生成
- **修正**: `NetcapHandler` に `pending_request` フィールドを追加。`handle_request()` でリクエスト情報を保存し、`handle_response()` で紐付け
- **ファイル**: `crates/netcap-core/src/proxy/handler.rs`

### Bug B: JSONL ファイルが生成されない問題
- **原因**: `capture.rs` が `storage_types.first()` で最初のストレージのみ初期化
- **修正**: 全指定ストレージバックエンドをループで初期化し、`storages()` メソッドで一括登録
- **ファイル**: `crates/netcap-cli/src/commands/capture.rs`, `crates/netcap-core/src/proxy/mod.rs`

### Bug C: latency_us / ttfb_us が常に0
- **原因**: レスポンス生成時にリクエストのタイムスタンプが不明
- **修正**: `PendingRequestInfo` に `request_instant: std::time::Instant` を追加し、レスポンス受信時に差分を計算
- **ファイル**: `crates/netcap-core/src/proxy/handler.rs`

### Bug D: CA証明書の永続化とリロード
- **原因**: `capture` コマンドが毎回新しいCA証明書を生成。`cert generate` とは別のCA
- **修正**: CA証明書を `netcap-ca/ca.pem` と `netcap-ca/ca.key.pem` にディスク保存。既存ファイルがあればリロード
- **ファイル**: `crates/netcap-cli/src/commands/capture.rs`

## 3. テスト結果

### 3.1 ユニットテスト
```
全238テスト合格 (0 failed)
```

### 3.2 E2Eテスト

| # | テスト | プロトコル | メソッド | URL | 結果 |
|---|--------|-----------|---------|-----|------|
| 1 | HTTP GET | HTTP | GET | http://example.com | 200 ✅ |
| 2 | HTTP GET | HTTP | GET | http://httpbin.org/get | 200 ✅ |
| 3 | HTTP POST | HTTP | POST | http://httpbin.org/post | 200 ✅ |
| 4 | HTTPS GET (--cacert) | HTTPS | GET | https://example.com | 200 ✅ |
| 5 | HTTPS GET (-k) | HTTPS | GET | https://httpbin.org/get | 200 ✅ |
| 6 | HTTPS POST (-k) | HTTPS | POST | https://httpbin.org/post | 200 ✅ |
| 7 | HTTPS GET (--cacert) | HTTPS | GET | https://httpbin.org/get | 200 ✅ |
| 8 | HTTPS POST (--cacert) | HTTPS | POST | https://httpbin.org/post | 200 ✅ |

### 3.3 SQLite データ検証

```
Requests: 8, Responses: 8
"http://unknown" entries: 0
Requests without response: 0

1. GET http://example.com/ → 200 (latency: 95301µs)
2. GET http://httpbin.org/get → 200 (latency: 919675µs)
3. POST http://httpbin.org/post → 200 (latency: 630679µs)
4. GET https://httpbin.org:443/get → 200 (latency: 934568µs)
5. POST https://httpbin.org:443/post → 200 (latency: 229982µs)
6. GET https://example.com:443/ → 200 (latency: 261643µs)
7. GET https://httpbin.org:443/get → 200 (latency: 210233µs)
8. POST https://httpbin.org:443/post → 200 (latency: 396768µs)
```

### 3.4 JSONL データ検証

```
8 lines (8 exchanges)
各エントリに正しいURI、メソッド、ヘッダー、ボディ、レスポンスが記録
```

## 4. 検証項目チェック

| 検証項目 | 結果 |
|---------|------|
| HTTP GET キャプチャ | ✅ PASS |
| HTTP POST キャプチャ (ボディ含む) | ✅ PASS |
| HTTPS MITM キャプチャ (-k) | ✅ PASS |
| HTTPS CA証明書検証 (--cacert) | ✅ PASS |
| リクエスト-レスポンス紐付け | ✅ PASS |
| レイテンシ計測 | ✅ PASS |
| SQLite ストレージ | ✅ PASS |
| JSONL ストレージ | ✅ PASS |
| 複数ストレージ同時使用 | ✅ PASS |
| CA証明書永続化・リロード | ✅ PASS |

## 5. 総合判定

**E2E キャプチャ機能: PASS**
