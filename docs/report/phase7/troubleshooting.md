# Phase 7 トラブルシューティング

## 問題1: CapturedExchange のパス
**症状:** `netcap_core::capture::CapturedExchange` がprivate
**原因:** capture/mod.rs で `use crate::capture::exchange::CapturedExchange` だが re-export していない
**解決:** `netcap_core::capture::exchange::CapturedExchange` で直接参照

## 問題2: DomainPattern のパス
**症状:** `netcap_core::filter::DomainPattern` がprivate
**原因:** filter/mod.rs で `use pattern::DomainPattern` だが pub で re-export していない
**解決:** `netcap_core::filter::pattern::DomainPattern` で直接参照

## 問題3: async_trait が依存関係にない
**症状:** `#[async_trait::async_trait]` でunresolved module
**解決:** Cargo.toml に `async-trait = { workspace = true }` 追加

## 問題4: Bytes/HeaderMap の型アクセス
**症状:** body, headers のフィールドアクセスで型エラー
**原因:** CapturedRequest.body は `Bytes` (not Option), headers は `HeaderMap`
**解決:** `.len()` 直接呼び出し、headers は `.to_string()` + `.to_str()` で変換

## 問題5: FfiCaptureStats に Debug がない
**症状:** ExchangeCollector の #[derive(Debug)] がFfiCaptureStatsにDebugを要求
**解決:** FfiCaptureStats に `#[derive(Debug)]` 追加
