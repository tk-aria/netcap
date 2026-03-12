# Step 4.4: PCAP ストレージ実装 - 作業レポート

## 完了日時
2026-03-12 17:00 JST

## 実装内容

### converter.rs (HTTP → PCAP パケット変換)
- `build_request_packet()`: CapturedExchange → 擬似 TCP/IP パケット (リクエスト)
- `build_response_packet()`: CapturedExchange → 擬似 TCP/IP パケット (レスポンス)
- Ethernet (14B) + IPv4 (20B) + TCP (20B) + HTTP ペイロード
- TLS通信: port 443, 非TLS: port 80
- クライアント 127.0.0.1:50000 → サーバー 127.0.0.2:80/443

### lib.rs (PcapStorage + StorageBackend)
- `PcapStorageConfig { output_path, snaplen }`
- `PcapStorage::new()`: ファイル作成、PcapHeader 書き込み
- `write()`: リクエスト + レスポンス(あれば) のパケットを書き込み
- `write_batch()`: 複数 exchange を順次書き込み
- snaplen によるパケットトランケーション対応

## テスト結果
- converter: 6テスト通過
- lib: 7テスト通過
- 全ワークスペース: 177テスト通過

## 変更ファイル
- `crates/netcap-storage-pcap/src/converter.rs` (新規)
- `crates/netcap-storage-pcap/src/lib.rs` (全面書き換え)
- `crates/netcap-storage-pcap/Cargo.toml` (依存追加済み)
