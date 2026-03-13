# Phase 6 作業サマリー

## 概要
Phase 6: 拡張ストレージ & ユーティリティの全機能を実装完了。

## 実装内容

### Step 6.1: BigQuery ストレージバックエンド
- `BigQueryStorage` に `StorageBackend` trait を実装
- `exchange_to_row` で CapturedExchange → BigQueryRow 変換
- `retry_with_backoff` で最大3回リトライ (指数バックオフ)
- BigQuery 送信失敗時に JSONL ファイルへ自動フォールバック
- 12テスト追加

### Step 6.2: TUI リアルタイムモニター
- ratatui 0.29 + crossterm 0.28 ベースのターミナルUI
- App 状態管理 (リクエスト一覧、統計情報、タブ切り替え)
- キーボードナビゲーション (j/k/arrows, Tab, Enter, q/Esc)
- request_list, detail_view, status_bar ウィジェット
- 15テスト追加

### Step 6.3: リプレイコマンド
- JSONL / SQLite からリクエスト読み込み
- reqwest 0.12 (rustls-tls) で HTTP リクエスト再送信
- CLI サブコマンド `netcap replay --from <path>` 追加
- 4テスト追加

### Step 6.4: テスト・ビルド検証
- 全219テスト合格
- TODO/FIXME残留なし
- エビデンス: docs/evidence/phase6_report.md

## ファイル変更
- 新規: 9ファイル (batch.rs, schema.rs, app.rs, event.rs, ui/*.rs, replay.rs)
- 変更: 10ファイル (各 Cargo.toml, args.rs, main.rs, mod.rs, features.md)
- 合計: +1,308行

## テスト結果
- 全219テスト合格 (0 failed)
- Phase 6 追加分: 31テスト (BigQuery 12 + TUI 15 + Replay 4)
