# .gitignore 確認・更新

## 作業内容
- `.gitignore` が既に存在し内容が入っていることを確認
- features.md の内容を精査し、不足している ignore パターンを追加:
  - `*.db`, `*.db-journal`, `*.db-wal`, `*.db-shm` (SQLite)
  - `*.jsonl`, `*.pcap`, `*.pcapng` (キャプチャ出力)
  - `coverage/`, `tarpaulin-report.html` (カバレッジ)
  - `node_modules/`

## 実行コマンド
```bash
# .gitignore を読み取り確認
cat .gitignore

# 不足パターンを追加 (Edit tool)
# git add & commit
git add .; git commit -m "Update .gitignore: add SQLite, capture output, coverage patterns"
```
