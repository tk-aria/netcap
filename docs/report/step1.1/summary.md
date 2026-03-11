# Step 1.1: ワークスペース初期化 - 作業報告

## 完了日時
2026-03-11

## 作業内容
- ルート `Cargo.toml` にワークスペース定義と共通依存を記述
- `rust-toolchain.toml` を作成 (stable, rustfmt, clippy)
- 8クレートのディレクトリとスケルトン `Cargo.toml` を作成:
  - netcap-core
  - netcap-storage-sqlite
  - netcap-storage-jsonl
  - netcap-storage-pcap
  - netcap-storage-bigquery
  - netcap-ffi
  - netcap-cli (binary crate)
  - netcap-tui
- 各クレートにスケルトン `lib.rs` / `main.rs` を配置
- `.cargo/config.toml` にビルド環境メモを記載

## 変更点 (features.md からの差分)
- `rustls` を `ring` バックエンドに変更 (aws-lc-sys のビルド問題回避)
  - `rustls = { version = "0.23", default-features = false, features = ["logging", "ring", "std", "tls12"] }`
- `rust-toolchain.toml` からモバイルターゲットを除外 (Phase 7 で追加予定)

## ビルド確認
```
cargo check --workspace → Finished `dev` profile [unoptimized + debuginfo]
```

## 環境構築
- Rust 1.94.0 (stable) をインストール
- GCC 12 (/tmp/gcc-root) のPATH設定
- Linux kernel headers の手動準備 (linux/limits.h, linux/falloc.h 等)
