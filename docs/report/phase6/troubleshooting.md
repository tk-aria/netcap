# Phase 6 トラブルシューティング

## 問題1: コンテナ再起動によるビルド環境消失
**症状:** Rust/GCC が消失し `cargo build` が実行不能
**原因:** コンテナ再起動で /home/node/ が初期化される
**解決:** 13パッケージの GCC インストールレシピを確立（gcc-12, cpp-12, libgcc-12-dev, binutils, binutils-common, libbinutils, libctf0, libjansson4, libc6-dev, linux-libc-dev, libisl23, libmpfr6, libmpc3）+ symlinks + libc.so パッチ

## 問題2: DomainFilter::add_rule が見つからない
**症状:** `capture.rs` でコンパイルエラー
**原因:** `add_rule` は `DomainMatcher` trait のメソッドで、直接 `DomainFilter` にはない
**解決:** `use netcap_core::filter::DomainMatcher;` を追加

## 問題3: JsonlStorage::new が async
**症状:** `create_storage` 関数内で async 関数を同期的に呼び出してエラー
**解決:** `create_storage` を `async fn` に変更
