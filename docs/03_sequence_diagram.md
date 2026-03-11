# シーケンス図: netcap-core 処理フロー

## 概要

netcap-core の主要な処理フローをシーケンス図で記載する。

---

## シーケンス1: HTTPSキャプチャの全体フロー

クライアントからのHTTPSリクエストをMITMプロキシがインターセプトし、
TLS終端・再暗号化を行いながらキャプチャする一連のフロー。

```mermaid
sequenceDiagram
    autonumber
    participant Client as Client<br/>(Browser / App)
    participant Proxy as netcap-core<br/>Proxy
    participant CA as CA Certificate<br/>Manager
    participant CertCache as Certificate<br/>Cache
    participant Filter as Domain<br/>Filter
    participant Logger as Capture<br/>Logger
    participant Target as Target<br/>Server

    Note over Client, Target: Phase 1: CONNECT トンネル確立
    Client->>Proxy: CONNECT example.com:443 HTTP/1.1
    Proxy->>Filter: ドメインフィルタチェック (example.com)
    Filter-->>Proxy: キャプチャ対象 (include マッチ)

    Proxy-->>Client: HTTP/1.1 200 Connection Established

    Note over Client, Target: Phase 2: TLS終端 (MITM)
    Proxy->>CertCache: example.com の証明書をキャッシュ検索
    alt キャッシュヒット
        CertCache-->>Proxy: キャッシュ済み証明書を返却
    else キャッシュミス
        Proxy->>CA: example.com 用のサーバ証明書を動的生成
        CA->>CA: CA秘密鍵で署名
        CA-->>Proxy: 署名済みサーバ証明書 + 秘密鍵
        Proxy->>CertCache: 証明書をキャッシュに保存 (TTL付き)
    end

    Client->>Proxy: TLS ClientHello (SNI: example.com)
    Proxy-->>Client: TLS ServerHello + 動的生成証明書
    Client->>Proxy: TLS Finished (クライアント側TLSハンドシェイク完了)

    Note over Proxy, Target: Phase 3: ターゲットサーバへのTLS接続
    Proxy->>Target: TLS ClientHello (SNI: example.com)
    Target-->>Proxy: TLS ServerHello + サーバ証明書
    Proxy->>Proxy: サーバ証明書の検証
    Proxy->>Target: TLS Finished (サーバ側TLSハンドシェイク完了)

    Note over Proxy: Connection レコード作成

    Proxy->>Logger: Connection情報を記録<br/>(client_ip, server_ip, tls_version, sni, cipher_suite)

    Note over Client, Target: Phase 4: HTTPリクエスト/レスポンスのキャプチャ
    Client->>Proxy: GET /api/data HTTP/1.1<br/>Host: example.com<br/>Authorization: Bearer xxx
    Proxy->>Proxy: リクエストを復号・パース

    Proxy->>Logger: HttpRequest を記録<br/>(method, url, headers, body, timestamp)

    Proxy->>Target: GET /api/data HTTP/1.1<br/>Host: example.com<br/>Authorization: Bearer xxx

    Target-->>Proxy: HTTP/1.1 200 OK<br/>Content-Type: application/json<br/>{"data": "..."}
    Proxy->>Proxy: レスポンスを復号・パース

    Proxy->>Logger: HttpResponse を記録<br/>(status_code, headers, body, latency)

    Proxy-->>Client: HTTP/1.1 200 OK<br/>Content-Type: application/json<br/>{"data": "..."}

    Note over Logger: 非同期でストレージバックエンドへ永続化
```

---

## シーケンス2: フィルタリング処理

リクエスト受信時のドメインフィルタリング判定フロー。
include/exclude ルールの優先度に基づいて、キャプチャ対象かパススルーかを判定する。

```mermaid
sequenceDiagram
    autonumber
    participant Client as Client
    participant Proxy as netcap-core<br/>Proxy
    participant FilterEngine as Filter<br/>Engine
    participant FilterDB as Filter<br/>Rules DB
    participant Logger as Capture<br/>Logger
    participant Target as Target<br/>Server

    Client->>Proxy: CONNECT api.example.com:443
    Proxy->>FilterEngine: evaluate("api.example.com")

    FilterEngine->>FilterDB: セッションに紐づくフィルタルール一覧を取得
    FilterDB-->>FilterEngine: フィルタルール一覧<br/>(priority順にソート済み)

    Note over FilterEngine: 優先度の高い順にルールを評価

    loop 各フィルタルールを順に評価
        FilterEngine->>FilterEngine: pattern マッチ判定<br/>(exact / wildcard / regex)
        alt マッチした
            Note over FilterEngine: マッチしたルールの filter_type を確認
        end
    end

    alt include ルールにマッチ → キャプチャ
        FilterEngine-->>Proxy: CaptureDecision::Capture(filter_id)
        Proxy-->>Client: 200 Connection Established
        Note over Proxy: MITM TLS終端を実施
        Client->>Proxy: HTTPリクエスト (暗号化解除)
        Proxy->>Logger: リクエスト/レスポンスを記録<br/>(matched_filter_id を付与)
        Proxy->>Target: リクエスト転送
        Target-->>Proxy: レスポンス
        Proxy->>Logger: レスポンスを記録
        Proxy-->>Client: レスポンス返却

    else exclude ルールにマッチ → パススルー
        FilterEngine-->>Proxy: CaptureDecision::Passthrough
        Proxy-->>Client: 200 Connection Established
        Note over Proxy, Target: TCPレベルの透過プロキシ<br/>(TLS終端しない、バイトストリームをそのまま転送)
        Client->>Proxy: TLS暗号化データ (そのまま)
        Proxy->>Target: TLS暗号化データ (そのまま転送)
        Target-->>Proxy: TLS暗号化データ
        Proxy-->>Client: TLS暗号化データ

    else どのルールにもマッチしない → デフォルト動作
        FilterEngine-->>Proxy: CaptureDecision::Default
        Note over Proxy: セッション設定のデフォルト動作に従う<br/>(default_action: capture / passthrough)
        alt デフォルト = capture
            Proxy->>Proxy: MITM キャプチャフローへ
        else デフォルト = passthrough
            Proxy->>Proxy: パススルーフローへ
        end
    end
```

---

## シーケンス3: ログ永続化フロー

キャプチャしたHTTP通信データを複数のストレージバックエンドへ並行書き出しするフロー。
バッファリングとバッチ処理でI/O負荷を抑える。

```mermaid
sequenceDiagram
    autonumber
    participant Proxy as netcap-core<br/>Proxy
    participant Buffer as Ring Buffer<br/>(lock-free)
    participant Dispatcher as Storage<br/>Dispatcher
    participant SQLite as SQLite<br/>Writer
    participant JSONL as JSONL<br/>Writer
    participant PCAP as PCAP<br/>Writer
    participant BQ as BigQuery<br/>Writer
    participant SQLiteDB as SQLite DB<br/>(WAL mode)
    participant JSONLFile as JSONL File
    participant PCAPFile as PCAP File
    participant BQAPI as BigQuery<br/>Streaming API

    Note over Proxy, Buffer: Phase 1: キャプチャデータのバッファリング
    Proxy->>Buffer: CaptureEvent を push<br/>(HttpRequest + HttpResponse + Connection)
    Note over Buffer: lock-free ring buffer で<br/>プロキシスレッドをブロックしない

    Note over Buffer, Dispatcher: Phase 2: バッチ取り出し
    loop バッチ間隔 (100ms) またはバッファ閾値到達
        Buffer->>Dispatcher: バッチ取り出し<br/>(最大 N 件ずつ drain)
    end

    Note over Dispatcher, BQAPI: Phase 3: 並行書き出し (各バックエンドは独立)

    par SQLite 書き出し
        Dispatcher->>SQLite: CaptureEvent バッチを送信
        SQLite->>SQLite: トランザクション開始
        SQLite->>SQLiteDB: INSERT INTO http_requests ...
        SQLite->>SQLiteDB: INSERT INTO http_responses ...
        SQLite->>SQLiteDB: UPDATE connections SET request_count = ...
        SQLite->>SQLite: トランザクションCOMMIT
        SQLite-->>Dispatcher: 書き出し完了 (件数)

    and JSONL 書き出し
        Dispatcher->>JSONL: CaptureEvent バッチを送信
        loop 各 CaptureEvent
            JSONL->>JSONL: JSON シリアライズ
            JSONL->>JSONLFile: 1行追記 (append)
        end
        JSONL->>JSONLFile: fsync (durability)
        JSONL-->>Dispatcher: 書き出し完了 (件数)

    and PCAP 書き出し
        Dispatcher->>PCAP: CaptureEvent バッチを送信
        loop 各 CaptureEvent
            PCAP->>PCAP: PCAP パケット形式に変換<br/>(TCP再構築)
            PCAP->>PCAPFile: パケットレコード追記
        end
        PCAP->>PCAPFile: flush
        PCAP-->>Dispatcher: 書き出し完了 (件数)

    and BigQuery 書き出し
        Dispatcher->>BQ: CaptureEvent バッチを送信
        BQ->>BQ: BigQuery行形式に変換
        BQ->>BQAPI: tabledata.insertAll<br/>(Streaming Insert)
        alt 成功
            BQAPI-->>BQ: 200 OK
            BQ-->>Dispatcher: 書き出し完了 (件数)
        else エラー (レート制限 / ネットワーク)
            BQAPI-->>BQ: 429 / 5xx エラー
            BQ->>BQ: エクスポネンシャルバックオフで再試行キューへ
            Note over BQ: 最大3回リトライ後、<br/>ローカルフォールバック (JSONL) に退避
        end
    end

    Note over Dispatcher: 各バックエンドの書き出し結果を集約<br/>メトリクス更新 (成功件数, 失敗件数, レイテンシ)
```

---

## シーケンス4: モバイルアプリ連携 (Android / iOS)

ネイティブモバイルアプリから VPN/ローカルプロキシ経由で Rust コアライブラリ (FFI) を呼び出し、
HTTP通信をキャプチャする連携フロー。

```mermaid
sequenceDiagram
    autonumber
    participant User as User
    participant NativeApp as Native App<br/>(Android/iOS)
    participant VPNService as VPN Service<br/>(Android: VpnService<br/>iOS: NEPacketTunnelProvider)
    participant FFI as FFI Bridge<br/>(JNI / C-ABI)
    participant Core as netcap-core<br/>(Rust Library)
    participant ProxyLoop as Proxy<br/>Event Loop
    participant Logger as Capture<br/>Logger
    participant TargetApp as Target App<br/>Traffic
    participant Internet as Internet

    Note over User, Internet: Phase 1: 初期化とVPN確立
    User->>NativeApp: キャプチャ開始ボタンタップ
    NativeApp->>NativeApp: VPN権限チェック / リクエスト

    alt Android
        NativeApp->>VPNService: VpnService.establish()<br/>tun デバイス作成
        Note over VPNService: tun fd を取得<br/>DNS設定, ルーティング設定
    else iOS
        NativeApp->>VPNService: NEPacketTunnelProvider<br/>startTunnel(options:)
        Note over VPNService: utun デバイス作成<br/>NEPacketTunnelNetworkSettings 適用
    end

    VPNService-->>NativeApp: VPN確立完了 (tun fd)

    Note over NativeApp, Core: Phase 2: Rustコアライブラリ初期化
    NativeApp->>FFI: netcap_init(config)
    FFI->>Core: NetcapConfig を Rust構造体に変換
    Core->>Core: CaptureSession 作成
    Core->>Core: CA証明書ロード / 生成
    Core->>Core: DomainFilter ルール設定
    Core->>Core: ストレージバックエンド初期化
    Core-->>FFI: SessionHandle (opaque pointer)
    FFI-->>NativeApp: session_handle

    Note over NativeApp, Internet: Phase 3: プロキシ起動とトラフィック処理
    NativeApp->>FFI: netcap_start_proxy(session_handle, tun_fd)
    FFI->>Core: tun fd を受け取り
    Core->>ProxyLoop: tokio ランタイム起動<br/>ローカルプロキシ起動 (127.0.0.1:PORT)

    Note over VPNService: 全トラフィックを tun 経由で<br/>ローカルプロキシにルーティング

    loop トラフィックキャプチャ (セッション中継続)
        TargetApp->>VPNService: HTTPSリクエスト (アプリの通信)
        VPNService->>ProxyLoop: tun デバイス経由でパケット受信
        ProxyLoop->>ProxyLoop: IP/TCPパケットをパース<br/>HTTP CONNECT を検出
        ProxyLoop->>Core: ドメインフィルタ判定
        Core-->>ProxyLoop: CaptureDecision

        alt キャプチャ対象
            ProxyLoop->>ProxyLoop: MITM TLS終端
            ProxyLoop->>Internet: ターゲットサーバへリクエスト転送
            Internet-->>ProxyLoop: レスポンス受信
            ProxyLoop->>Logger: Request + Response を記録
            ProxyLoop->>VPNService: レスポンスパケットを tun に書き戻し
        else パススルー
            ProxyLoop->>Internet: パケットをそのまま転送
            Internet-->>ProxyLoop: レスポンス
            ProxyLoop->>VPNService: そのまま返却
        end

        VPNService-->>TargetApp: レスポンス返却
    end

    Note over NativeApp, Core: Phase 4: キャプチャ結果の取得
    NativeApp->>FFI: netcap_get_capture_events(session_handle, offset, limit)
    FFI->>Core: キャプチャイベント取得
    Core-->>FFI: Vec<CaptureEvent> → JSON / FlatBuffers
    FFI-->>NativeApp: キャプチャイベント一覧

    NativeApp->>NativeApp: UI表示<br/>(リクエスト一覧, 詳細, フィルタ)

    Note over User, Internet: Phase 5: セッション終了
    User->>NativeApp: キャプチャ停止ボタンタップ
    NativeApp->>FFI: netcap_stop_proxy(session_handle)
    FFI->>Core: セッション停止
    Core->>ProxyLoop: graceful shutdown
    ProxyLoop->>Logger: 残りバッファをフラッシュ
    Logger-->>Core: フラッシュ完了
    Core->>Core: CaptureSession.stopped_at を記録
    Core-->>FFI: 停止完了
    FFI-->>NativeApp: OK

    NativeApp->>FFI: netcap_free(session_handle)
    FFI->>Core: メモリ解放
    NativeApp->>VPNService: VPN切断
    VPNService-->>NativeApp: 切断完了
    NativeApp-->>User: キャプチャ停止完了
```

---

## 補足: FFI関数一覧 (C-ABI)

モバイルアプリ連携で使用する主要なFFI関数。

| 関数名 | 引数 | 戻り値 | 説明 |
|--------|------|--------|------|
| `netcap_init` | `config: *const c_char` (JSON) | `*mut SessionHandle` | セッション初期化 |
| `netcap_start_proxy` | `handle: *mut SessionHandle, tun_fd: c_int` | `c_int` (0=成功) | プロキシ起動 |
| `netcap_stop_proxy` | `handle: *mut SessionHandle` | `c_int` | プロキシ停止 |
| `netcap_get_capture_events` | `handle: *mut SessionHandle, offset: u64, limit: u64` | `*mut c_char` (JSON) | キャプチャイベント取得 |
| `netcap_get_stats` | `handle: *mut SessionHandle` | `*mut c_char` (JSON) | 統計情報取得 |
| `netcap_update_filters` | `handle: *mut SessionHandle, filters: *const c_char` | `c_int` | フィルタ動的更新 |
| `netcap_export_session` | `handle: *mut SessionHandle, format: *const c_char, path: *const c_char` | `c_int` | セッションエクスポート |
| `netcap_free` | `handle: *mut SessionHandle` | `void` | メモリ解放 |

---

## 補足: エラーハンドリング方針

各シーケンスにおける障害時の動作方針。

| 障害箇所 | 挙動 | リカバリ |
|----------|------|----------|
| CA証明書生成失敗 | セッション開始を拒否 | ユーザにCA再生成を促す |
| ターゲットサーバ接続失敗 | 502 Bad Gateway をクライアントに返却 | Connection.close_reason = "error" を記録 |
| ストレージ書き出し失敗 (SQLite) | エラーログ出力、リトライ | 3回失敗で該当バックエンドを一時無効化 |
| ストレージ書き出し失敗 (BigQuery) | ローカル JSONL にフォールバック | 接続回復後にリトライキューから再送 |
| バッファオーバーフロー | 古いイベントを破棄 (ring buffer) | メトリクスで drop count を記録 |
| TLSハンドシェイク失敗 | パススルーにフォールバック | TLS固定 (certificate pinning) の可能性をログ記録 |
| VPN切断 (モバイル) | セッションを自動停止 | バッファ内データをフラッシュしてから停止 |
