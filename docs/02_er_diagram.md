# E-R図: netcap-core データモデル

## 概要

netcap-core が扱うデータモデルのEntity-Relationship図。
キャプチャセッション、HTTP通信、接続情報、フィルタ設定、CA証明書を中心に構成される。

## E-R図

```mermaid
erDiagram
    CaptureSession ||--o{ HttpRequest : "contains"
    CaptureSession }o--o{ DomainFilter : "applies"
    CaptureSession ||--o| CertificateAuthority : "uses"
    HttpRequest ||--o| HttpResponse : "receives"
    HttpRequest }o--|| Connection : "over"
    CaptureSession ||--o{ Connection : "establishes"

    CaptureSession {
        TEXT id PK "UUIDv7 (時系列ソート可能)"
        TEXT name "セッション名 (ユーザ定義, NULLABLE)"
        TEXT status "enum: running / paused / stopped"
        TEXT created_at "ISO8601 タイムスタンプ"
        TEXT stopped_at "ISO8601 タイムスタンプ (NULLABLE)"
        TEXT proxy_listen_addr "プロキシリッスンアドレス e.g. 127.0.0.1:8080"
        INTEGER proxy_port "リッスンポート"
        TEXT platform "enum: windows / macos / linux / android / ios"
        INTEGER capture_request_body "リクエストボディ記録フラグ (0/1)"
        INTEGER capture_response_body "レスポンスボディ記録フラグ (0/1)"
        INTEGER max_body_size_bytes "ボディ保存上限バイト数 (default: 10MB)"
        TEXT storage_backends "JSON配列: sqlite / jsonl / pcap / bigquery"
        TEXT ca_id FK "使用するCA証明書ID"
        TEXT metadata "JSON: 任意の追加メタデータ"
    }

    HttpRequest {
        TEXT id PK "UUIDv7"
        TEXT session_id FK "CaptureSession.id"
        TEXT connection_id FK "Connection.id"
        INTEGER sequence_number "セッション内の連番"
        TEXT method "HTTP method: GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS / CONNECT"
        TEXT url "完全なリクエストURL"
        TEXT scheme "enum: http / https"
        TEXT host "ホスト名"
        INTEGER port "ポート番号"
        TEXT path "パス部分"
        TEXT query_string "クエリ文字列 (NULLABLE)"
        TEXT fragment "フラグメント (NULLABLE)"
        TEXT http_version "HTTP/1.0 / HTTP/1.1 / HTTP/2 / HTTP/3"
        TEXT headers "JSON: ヘッダーの配列 [{name, value}]"
        INTEGER content_length "Content-Length (NULLABLE)"
        TEXT content_type "Content-Type (NULLABLE)"
        BLOB body "リクエストボディ (NULLABLE)"
        INTEGER body_truncated "ボディが切り詰められたか (0/1)"
        TEXT body_encoding "identity / gzip / br / deflate (NULLABLE)"
        TEXT timestamp "ISO8601 リクエスト受信時刻"
        INTEGER timestamp_unix_us "Unixマイクロ秒 (高精度ソート用)"
        TEXT matched_filter_id FK "マッチしたDomainFilter.id (NULLABLE)"
        TEXT tags "JSON配列: ユーザ定義タグ (NULLABLE)"
    }

    HttpResponse {
        TEXT id PK "UUIDv7"
        TEXT request_id FK "HttpRequest.id (UNIQUE)"
        INTEGER status_code "HTTPステータスコード"
        TEXT status_text "ステータステキスト e.g. OK, Not Found"
        TEXT http_version "レスポンスのHTTPバージョン"
        TEXT headers "JSON: ヘッダーの配列 [{name, value}]"
        INTEGER content_length "Content-Length (NULLABLE)"
        TEXT content_type "Content-Type (NULLABLE)"
        BLOB body "レスポンスボディ (NULLABLE)"
        INTEGER body_truncated "ボディが切り詰められたか (0/1)"
        TEXT body_encoding "identity / gzip / br / deflate (NULLABLE)"
        TEXT timestamp "ISO8601 レスポンス受信完了時刻"
        INTEGER timestamp_unix_us "Unixマイクロ秒"
        INTEGER latency_us "リクエスト送信〜レスポンス受信の遅延 (マイクロ秒)"
        INTEGER ttfb_us "Time To First Byte (マイクロ秒)"
        INTEGER transfer_duration_us "ボディ転送時間 (マイクロ秒)"
        INTEGER total_size_bytes "ヘッダー + ボディの合計サイズ"
    }

    Connection {
        TEXT id PK "UUIDv7"
        TEXT session_id FK "CaptureSession.id"
        TEXT client_ip "クライアントIPアドレス"
        INTEGER client_port "クライアントポート"
        TEXT server_ip "接続先サーバIPアドレス"
        TEXT server_hostname "接続先ホスト名 (DNS解決前)"
        INTEGER server_port "接続先ポート"
        TEXT transport "enum: tcp / udp (QUIC)"
        INTEGER is_tls "TLS接続か (0/1)"
        TEXT tls_version "TLS 1.0 / TLS 1.1 / TLS 1.2 / TLS 1.3 (NULLABLE)"
        TEXT tls_cipher_suite "暗号スイート名 (NULLABLE)"
        TEXT sni "Server Name Indication (NULLABLE)"
        TEXT alpn "Application-Layer Protocol Negotiation e.g. h2, http/1.1 (NULLABLE)"
        TEXT client_cert_subject "クライアント証明書のSubject (NULLABLE)"
        TEXT established_at "ISO8601 接続確立時刻"
        TEXT closed_at "ISO8601 接続終了時刻 (NULLABLE)"
        TEXT close_reason "enum: client_close / server_close / timeout / error (NULLABLE)"
        INTEGER request_count "この接続上のリクエスト数"
    }

    DomainFilter {
        TEXT id PK "UUIDv7"
        TEXT name "フィルタ名"
        TEXT filter_type "enum: include / exclude"
        TEXT pattern "マッチパターン (ワイルドカード対応 e.g. *.example.com)"
        TEXT pattern_type "enum: exact / wildcard / regex"
        INTEGER priority "適用優先度 (数値が大きいほど優先)"
        INTEGER enabled "有効/無効 (0/1)"
        TEXT created_at "ISO8601"
        TEXT updated_at "ISO8601"
    }

    CertificateAuthority {
        TEXT id PK "UUIDv7"
        TEXT common_name "CA証明書のCommon Name"
        TEXT organization "組織名"
        BLOB certificate_pem "CA証明書 (PEM形式)"
        BLOB private_key_pem "CA秘密鍵 (PEM形式, 暗号化済み)"
        TEXT key_algorithm "enum: rsa2048 / rsa4096 / ecdsa_p256 / ecdsa_p384"
        TEXT not_before "ISO8601 有効期間開始"
        TEXT not_after "ISO8601 有効期間終了"
        TEXT fingerprint_sha256 "SHA-256フィンガープリント"
        INTEGER is_default "デフォルトCAか (0/1)"
        TEXT created_at "ISO8601"
        TEXT storage_path "証明書ファイルの保存パス (NULLABLE)"
    }

    SessionDomainFilter {
        TEXT session_id FK "CaptureSession.id"
        TEXT filter_id FK "DomainFilter.id"
        INTEGER order "適用順序"
        TEXT added_at "ISO8601"
    }
```

## エンティティ詳細

### CaptureSession
キャプチャの論理的な単位。ユーザがキャプチャを開始してから停止するまでの期間を表す。
- プラットフォーム情報、プロキシ設定、ストレージバックエンド設定を保持
- 1つのCAと紐づく (MITM用)

### HttpRequest
プロキシが受信したHTTPリクエストの全情報。
- URL はパース済みの各パート (scheme, host, port, path, query_string) も個別カラムに保持し、高速なフィルタ・検索を可能にする
- `sequence_number` でセッション内の時系列順序を保証
- `body` は設定に応じて保存/非保存を切り替え可能

### HttpResponse
リクエストに対応するレスポンス。1:1対応。
- レスポンスが得られなかった場合 (接続断等) は HttpResponse レコードが存在しない (0..1)
- `latency_us`, `ttfb_us`, `transfer_duration_us` でパフォーマンス分析が可能

### Connection
TCP/UDP接続レベルの情報。1つの接続上で複数のHTTPリクエストが流れうる (HTTP/1.1 Keep-Alive, HTTP/2 Multiplexing)。
- TLS関連情報 (バージョン、暗号スイート、SNI、ALPN) を保持
- `request_count` で接続の多重化度を把握可能

### DomainFilter
キャプチャ対象/除外を制御するドメインフィルタ。
- `include` / `exclude` タイプで包含・除外を指定
- ワイルドカード (`*.example.com`) または正規表現でパターンマッチ
- `priority` による優先度制御

### CertificateAuthority
MITM用のCA証明書情報。
- RSA / ECDSA の鍵アルゴリズム対応
- 秘密鍵は暗号化された状態で保存

### SessionDomainFilter (中間テーブル)
CaptureSession と DomainFilter の多対多リレーションを実現する中間テーブル。
- `order` で同一セッション内のフィルタ適用順序を制御

## インデックス戦略

| テーブル | カラム | 種別 | 目的 |
|---------|--------|------|------|
| HttpRequest | session_id, timestamp_unix_us | 複合INDEX | セッション内の時系列クエリ |
| HttpRequest | host | INDEX | ホスト名での検索 |
| HttpRequest | method, status (JOIN) | INDEX | メソッド別集計 |
| HttpRequest | connection_id | INDEX | 接続ごとのリクエスト一覧 |
| HttpResponse | request_id | UNIQUE INDEX | リクエストとの1:1対応 |
| HttpResponse | status_code | INDEX | ステータスコード別検索 |
| Connection | session_id | INDEX | セッション内の接続一覧 |
| Connection | server_hostname | INDEX | ホスト名での接続検索 |
| DomainFilter | pattern | INDEX | パターンマッチ検索 |
| SessionDomainFilter | session_id, filter_id | 複合UNIQUE INDEX | 重複防止 |

## SQLiteスキーマ補足

- すべてのIDは UUIDv7 を TEXT 型で保存 (時系列ソート可能)
- タイムスタンプは ISO8601 の TEXT 型 + Unixマイクロ秒の INTEGER 型を併用
- ボディは BLOB 型で保存し、大きなボディは `max_body_size_bytes` で切り詰め
- ヘッダーは JSON 配列として TEXT 型に保存 (`[{"name": "Content-Type", "value": "application/json"}]`)
- WAL モードで並行読み書きに対応
