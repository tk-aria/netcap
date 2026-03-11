# ライブラリ選定ドキュメント

> 調査日: 2026-03-11
> プロジェクト: Rust製クロスプラットフォームHTTPキャプチャツール

---

## 1. コアライブラリ（プロキシ・ネットワーク）

### 1.1 MITMプロキシ: hudsucker

| 項目 | 内容 |
|------|------|
| クレート名 | `hudsucker` |
| 最新バージョン | 0.9.1 |
| GitHub Stars | ~309 |
| リポジトリ | https://github.com/omjadas/hudsucker |
| ライセンス | MIT / Apache-2.0 |
| Rust Edition | 2024 |
| 月間DL数 | ~998 |

**概要:**
Rust製のMITM HTTP/Sプロキシライブラリ。HTTP/HTTPSリクエスト・レスポンスの傍受・改変、WebSocketメッセージの改変が可能。

**主要Feature:**
- `rcgen-ca`（デフォルト）: rcgenベースの証明書生成
- `rustls-client`（デフォルト）: rustlsによるTLS接続
- `openssl-ca`: OpenSSLベースの証明書生成
- `native-tls-client`: ネイティブTLSコネクタ
- `http2`: HTTP/2サポート
- `decoder`: リクエスト/レスポンスのデコードヘルパー

**選定理由:**
- hyper + tokio + rustls をベースに構築されており、Rustエコシステムとの親和性が高い
- 証明書生成（rcgen）が組み込み済み
- WebSocketサポートあり
- MITMプロキシとして必要な機能が一通り揃っている

**デメリット:**
- コントリビュータが少ない（メイン開発者+3名程度）
- Star数がやや少なく、コミュニティ規模は小さい
- 大規模プロダクション事例が少ない

**代替候補との比較:**

| ライブラリ | 特徴 | 適合度 |
|------------|------|--------|
| `hudsucker` | MITMプロキシに特化、hyper/tokio/rustlsベース | **最適** |
| `http-mitm-proxy` | Burp Proxy的なバックエンド向け | 次点 |
| `mitmproxy_rs` | Python mitmproxyのRustコンポーネント | 組み込み用途には不向き |
| `product-os-proxy` | hudsucker拡張、VPNトンネル対応 | 過剰機能 |

**結論: `hudsucker` を採用。** MITMプロキシ用途に最も適しており、依存ライブラリもRust標準的なスタックで構成されている。必要に応じてフォークして拡張する戦略も取れる。

---

### 1.2 HTTPライブラリ: hyper

| 項目 | 内容 |
|------|------|
| クレート名 | `hyper` |
| 最新バージョン | 1.x系（安定版） |
| 補助クレート | `hyper-util` 0.1.20 |
| リポジトリ | https://github.com/hyperium/hyper |
| ライセンス | MIT |

**概要:**
Rustで最も広く使われるHTTPライブラリ。低レベルでありながら高性能。HTTP/1.1とHTTP/2をサポート。

**選定理由:**
- hudsucker の内部依存として必須
- Rustエコシステムのデファクトスタンダード
- 高パフォーマンス、プロダクション実績豊富

---

### 1.3 非同期ランタイム: tokio

| 項目 | 内容 |
|------|------|
| クレート名 | `tokio` |
| 最新バージョン | 1.50.0 (2026-03-03) |
| LTSバージョン | 1.47.x（2026年9月まで）|
| リポジトリ | https://github.com/tokio-rs/tokio |
| ライセンス | MIT |

**概要:**
Rustの非同期ランタイムのデファクトスタンダード。ネットワークI/O、タイマー、タスクスケジューリング等を提供。

**選定理由:**
- hudsucker, hyper の必須依存
- エコシステム全体で最も広く採用されている非同期ランタイム
- LTSリリースによる安定性保証

---

### 1.4 TLSライブラリ: rustls

| 項目 | 内容 |
|------|------|
| クレート名 | `rustls` |
| 最新バージョン | 0.23.36 (2026-01-05) |
| リポジトリ | https://github.com/rustls/rustls |
| ライセンス | MIT / Apache-2.0 / ISC |

**概要:**
Pure RustのモダンTLSライブラリ。TLS 1.2/1.3をサポート。OpenSSLへの依存なしにTLS通信が可能。

**選定理由:**
- Pure Rustのためクロスコンパイルが容易（Android/iOS対応に有利）
- OpenSSL依存を排除できる
- hudsucker のデフォルトTLSバックエンド
- セキュリティ重視の設計（メモリ安全性保証）

**注意:**
- TLS 1.2はデフォルトで無効（互換性が必要な場合は `rustls-tls-12` featureで有効化）

---

### 1.5 証明書生成: rcgen

| 項目 | 内容 |
|------|------|
| クレート名 | `rcgen` |
| 最新バージョン | 0.14.6 (2025-12-13) |
| リポジトリ | https://github.com/rustls/rcgen |
| ライセンス | MIT / Apache-2.0 |

**概要:**
X.509証明書・CSRを生成するPure Rustライブラリ。MITMプロキシではクライアント向けの動的証明書生成に使用。

**選定理由:**
- hudsucker の `rcgen-ca` featureで統合済み
- Pure Rustのためクロスプラットフォーム対応が容易
- WASMでも動作する移植性
- rustlsプロジェクト傘下で信頼性が高い

---

### 1.6 PCAPファイル出力: pcap-file

| 項目 | 内容 |
|------|------|
| クレート名 | `pcap-file` |
| 最新バージョン | 2.1.x |
| リポジトリ | https://github.com/courvoif/pcap-file |
| ライセンス | MIT / Apache-2.0 |

**概要:**
PcapおよびPcapNgファイルの読み書きを行うライブラリ。パーサー、リーダー、ライターを提供。

**選定理由:**
- Pure Rustでネイティブライブラリ不要
- PcapNg形式もサポート
- シンプルなAPIで書き出しが容易

**代替候補:**

| ライブラリ | 特徴 | 判定 |
|------------|------|------|
| `pcap-file` | Pure Rust、読み書き両対応 | **採用** |
| `pcap` (rust-pcap) | libpcapラッパー、キャプチャ機能含む | ネイティブ依存あり、過剰 |
| `pcap-parser` | パース特化 | 書き出し不可 |

**注意:** 本プロジェクトではアプリケーション層（HTTP）のデータをPCAP形式で出力するため、パケットヘッダの構築は自前で行う必要がある。

---

## 2. クロスプラットフォーム対応

### 2.1 推奨: UniFFI (Mozilla)

| 項目 | 内容 |
|------|------|
| クレート名 | `uniffi` |
| 最新バージョン | 0.30.0 |
| リポジトリ | https://github.com/mozilla/uniffi-rs |
| ライセンス | MPL-2.0 |
| 対応言語 | Kotlin, Swift, Python, Ruby, C#, Go |

**概要:**
Mozillaが開発するクロスプラットフォームFFIバインディング生成ツール。Rustで書いたコアロジックから、Kotlin（Android）やSwift（iOS）向けのバインディングを自動生成。

**選定理由:**
- **Android/iOS両方を単一のツールでカバー**できる唯一の選択肢
- Firefoxモバイル版で実プロダクション使用実績
- UDL（Interface Definition Language）またはproc-macroでインターフェースを定義
- React Native対応も進行中
- 型安全なバインディング生成

**デメリット:**
- 1.0未リリース、内部API変更の可能性あり
- 学習コストがやや高い
- 一部の複雑な型マッピングに制約あり

**結論: Android/iOSのFFI層には `uniffi` をメイン採用。**

---

### 2.2 Android向け: jni クレート（補助）

| 項目 | 内容 |
|------|------|
| クレート名 | `jni` |
| 最新バージョン | 0.22.0 |
| リポジトリ | https://github.com/jni-rs/jni-rs |
| ライセンス | MIT / Apache-2.0 |
| 累計DL数 | 90M+ |

**概要:**
RustからJava/Android APIを直接呼び出すためのJNIバインディング。UniFFIでカバーできない低レベル操作（VPNService APIの直接呼び出し等）が必要な場合の補助として使用。

**UniFFIとの使い分け:**
- 通常のデータ受け渡し・API呼び出し → UniFFI
- Android固有の低レベルAPI呼び出し → jni

---

### 2.3 iOS向け: cbindgen / swift-bridge（補助）

#### cbindgen

| 項目 | 内容 |
|------|------|
| クレート名 | `cbindgen` |
| 最新バージョン | 0.29.2 |
| リポジトリ | https://github.com/mozilla/cbindgen |
| 累計DL数 | 63.6M+ |

**概要:**
RustコードからC/C++ヘッダーを自動生成。UniFFIで対応できない場合のフォールバック。

#### swift-bridge

| 項目 | 内容 |
|------|------|
| クレート名 | `swift-bridge` |
| 最新バージョン | 0.1.59 (2026-01-06) |
| リポジトリ | https://github.com/chinedufn/swift-bridge |
| ライセンス | MIT / Apache-2.0 |

**概要:**
RustとSwift間の直接的なFFI生成。`String`, `Option<T>`, `Result<T,E>`, async関数等の高レベル型をブリッジ可能。オブジェクトシリアライゼーション不要でオーバーヘッドが小さい。

**FFI戦略まとめ:**

| レイヤー | ツール | 用途 |
|----------|--------|------|
| メインFFI | UniFFI | Android (Kotlin) / iOS (Swift) 共通 |
| Android低レベル | jni | VPNService等のシステムAPI直接呼び出し |
| iOS低レベル | cbindgen + swift-bridge | UniFFIで対応不可な場合のフォールバック |

---

## 3. データ永続化・ログ

### 3.1 シリアライゼーション: serde + serde_json

| 項目 | 内容 |
|------|------|
| クレート名 | `serde` / `serde_json` |
| 最新バージョン | serde ~1.0.220 / serde_json 1.0.149 |
| リポジトリ | https://github.com/serde-rs/serde |
| ライセンス | MIT / Apache-2.0 |

**概要:**
Rustのシリアライゼーション/デシリアライゼーションのデファクトスタンダード。JSONL出力にはserde_jsonを使用。

**選定理由:**
- 議論の余地なくRustエコシステムの標準
- derive マクロによる自動実装
- 高性能、ゼロコスト抽象化
- JSONL出力は `serde_json::to_string()` + 改行で簡単に実現

---

### 3.2 ローカルDB: rusqlite vs sqlx

| 項目 | rusqlite | sqlx |
|------|----------|------|
| 最新バージョン | 0.38.0 | 0.8.6 |
| 非同期サポート | なし（同期のみ） | あり（async/await） |
| コンパイル時クエリチェック | なし | あり |
| 対応DB | SQLiteのみ | PostgreSQL, MySQL, SQLite |
| SQLite機能カバレッジ | フル（FTS, JSON拡張等） | 基本的な操作 |
| 純Rust | bundledで可能 | 純Rust SQLiteドライバ |

**選定: `rusqlite`**

**理由:**
- プロキシ処理はtokioの非同期コンテキストで動くが、DB書き込みは `tokio::task::spawn_blocking` で分離可能
- SQLiteのフル機能セット（FTS、JSON拡張）にアクセス可能
- SQLiteのみ使用するため、マルチDB対応は不要
- `bundled` featureでSQLiteをリンクすればクロスコンパイルが容易
- シンプルで軽量

**補足:** 将来的にasync化が必要になった場合は `sqlx` への移行も検討可能。

---

### 3.3 BigQuery連携

| 方式 | クレート | 特徴 |
|------|----------|------|
| REST API | `gcp-bigquery-client` 0.27.0 | 専用クライアント、非同期、認証サポート |
| gRPC | `tonic` (将来的に `grpc`) | 汎用gRPCクライアント |
| REST直接 | `reqwest` + 手動実装 | 柔軟だが実装コスト高 |

**選定: `gcp-bigquery-client`**

| 項目 | 内容 |
|------|------|
| クレート名 | `gcp-bigquery-client` |
| 最新バージョン | 0.27.0 |
| リポジトリ | https://github.com/lquerel/gcp-bigquery-client |
| ライセンス | MIT |

**選定理由:**
- BigQuery REST APIのエルゴノミックなラッパー
- Service Account Key認証、Workload Identity等の認証方式サポート
- ストリーミングInsert API対応（ログの逐次送信に最適）
- 非同期（tokio互換）

**代替案:**
- `tonic` + BigQuery gRPC: 公式gRPCのRustサポートは発展途上。現時点ではREST APIクライアントの方が安定。
- 将来的に公式 `grpc` クレート（tonic後継）がリリースされた場合は移行を検討。

---

## 4. TUI/CLI

### 4.1 CLI引数パーサー: clap

| 項目 | 内容 |
|------|------|
| クレート名 | `clap` |
| 最新バージョン | 4.5.x |
| リポジトリ | https://github.com/clap-rs/clap |
| ライセンス | MIT / Apache-2.0 |

**概要:**
Rustで最も広く使われるCLI引数パーサー。derive マクロによる宣言的定義とbuilderパターンの両方をサポート。

**選定理由:**
- デファクトスタンダード、豊富なドキュメント
- derive マクロで構造体からCLI定義を自動生成
- サブコマンド、引数バリデーション、ヘルプ生成等が充実
- 代替候補なし（事実上の一択）

---

### 4.2 TUIフレームワーク: ratatui

| 項目 | 内容 |
|------|------|
| クレート名 | `ratatui` |
| 最新バージョン | 0.30.0 |
| リポジトリ | https://github.com/ratatui/ratatui |
| ライセンス | MIT |

**概要:**
tui-rsからフォークされたTUIフレームワーク。即時モードレンダリングで高性能なターミナルUIを構築。

**選定理由:**
- Rust TUIエコシステムの現在のデファクトスタンダード
- チャート、テーブル、リスト、ゲージ等のウィジェット内蔵
- サブミリ秒レンダリング
- 0.30.0でno_stdサポート追加（組み込み対象にも展開可能）
- アクティブなコミュニティと活発な開発

**用途:**
- リアルタイムのHTTPキャプチャログ表示
- ドメインフィルタ設定のインタラクティブ操作
- 統計ダッシュボード

**注意:** TUI機能はデスクトップ版（Windows/macOS/Linux）限定。初期リリースではCLI（clap）のみでも十分であり、TUIは後続フェーズでの実装を推奨。

---

## 5. テスト・CI / クロスコンパイル

### 5.1 クロスコンパイル: cross

| 項目 | 内容 |
|------|------|
| クレート名 | `cross` |
| 最新バージョン | 0.2.5 |
| リポジトリ | https://github.com/cross-rs/cross |
| ライセンス | MIT / Apache-2.0 |

**概要:**
Docker/Podmanを使用した「ゼロセットアップ」クロスコンパイル・テストツール。cargoと同じCLIで、異なるターゲットへのビルドが可能。

**選定理由:**
- ホスト環境を汚さずにクロスコンパイル可能
- Linux (GNU/musl), Windows, macOS, FreeBSD, Android, iOS対応
- CI/CDパイプラインとの親和性が高い
- GitHub Actions用のアクション (`houseabsolute/actions-rust-cross`) も利用可能

---

### 5.2 Android向けビルド: cargo-ndk

| 項目 | 内容 |
|------|------|
| クレート名 | `cargo-ndk` |
| 最新バージョン | 4.1.2 |
| リポジトリ | https://github.com/bbqsrc/cargo-ndk |
| ライセンス | MIT / Apache-2.0 |

**概要:**
Android NDKを使用したRustライブラリのビルドを簡素化するCargoサブコマンド。jniLibsディレクトリ構造の自動生成をサポート。

**主要コマンド:**
- `cargo ndk` - ビルド
- `cargo ndk-test` - テスト実行
- `cargo ndk-env` - 環境設定
- `cargo ndk-runner` - Androidデバイスでのバイナリ実行

**選定理由:**
- Android NDKの環境変数設定を自動化
- ABI別のライブラリ生成（arm64-v8a, armeabi-v7a, x86, x86_64）
- UniFFIとの組み合わせでAndroid向けライブラリ生成が効率化

---

### 5.3 iOS向けビルド: cargo-lipo

| 項目 | 内容 |
|------|------|
| クレート名 | `cargo-lipo` |
| 最新バージョン | 3.3.1 |
| リポジトリ | https://github.com/TimNN/cargo-lipo |
| ライセンス | MIT / Apache-2.0 |
| ステータス | **非推奨 / パッシブメンテナンス** |

**概要:**
iOS向けユニバーサルライブラリ（.a）を自動生成するCargoサブコマンド。aarch64-apple-iosとx86_64-apple-ios向けのバイナリをlipo結合。

**注意:**
- メンテナが非推奨を表明
- Xcodeのアーキテクチャ・OS固有環境変数を使用した方法が推奨されている

**代替案:**

| 方式 | 説明 | 推奨度 |
|------|------|--------|
| `cargo-lipo` | 従来の方法、パッシブメンテナンス | 既存プロジェクトのみ |
| Xcode環境変数 + スクリプト | `ARCHS`, `PLATFORM_NAME` を使用したビルドスクリプト | **推奨** |
| `cargo-xcode` | Xcodeプロジェクト統合 | 検討可能 |

**結論:** iOS向けビルドは `cargo-lipo` ではなく、**Xcodeのビルドスクリプト + rustup target** による直接的なアプローチを採用。具体的には:
1. `rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios`
2. Xcodeのビルドフェーズスクリプトで `cargo build --target` を呼び出し
3. 必要に応じて `lipo` コマンドで手動結合

---

## 6. 依存関係サマリー

### Cargo.toml 想定依存（コアライブラリ）

```toml
[dependencies]
# MITMプロキシ
hudsucker = { version = "0.9", features = ["rcgen-ca", "rustls-client", "http2", "decoder"] }

# 非同期ランタイム
tokio = { version = "1.47", features = ["full"] }

# シリアライゼーション
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# ローカルDB
rusqlite = { version = "0.38", features = ["bundled"] }

# PCAP出力
pcap-file = "2"

# CLI
clap = { version = "4.5", features = ["derive"] }

# TUI（後続フェーズ）
# ratatui = "0.30"

# BigQuery連携（feature flag制御）
gcp-bigquery-client = { version = "0.27", optional = true }

# ログ
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 開発・ビルド依存

```toml
[build-dependencies]
# iOS向けFFI（必要に応じて）
cbindgen = "0.29"

[dev-dependencies]
# テスト
tokio-test = "0.4"
```

### FFI層（別クレート）

```toml
# uniffi用
uniffi = "0.30"

# Android低レベル（必要に応じて）
jni = "0.22"
```

---

## 7. 選定方針まとめ

| カテゴリ | 選定ライブラリ | 理由 |
|----------|---------------|------|
| MITMプロキシ | hudsucker | Rust製MITMプロキシの最適解 |
| HTTP | hyper (hudsucker経由) | デファクトスタンダード |
| 非同期ランタイム | tokio | デファクトスタンダード |
| TLS | rustls | Pure Rust、クロスコンパイル容易 |
| 証明書生成 | rcgen (hudsucker経由) | Pure Rust、hudsucker統合済み |
| PCAP出力 | pcap-file | Pure Rust、シンプルAPI |
| FFI (Android/iOS) | uniffi | 両プラットフォーム一元管理 |
| FFI (Android補助) | jni | 低レベルAPI呼び出し用 |
| FFI (iOS補助) | cbindgen + swift-bridge | フォールバック用 |
| シリアライゼーション | serde + serde_json | デファクトスタンダード |
| ローカルDB | rusqlite | SQLiteフル機能、シンプル |
| BigQuery | gcp-bigquery-client | エルゴノミックな専用クライアント |
| CLI | clap | デファクトスタンダード |
| TUI | ratatui | 後続フェーズで導入 |
| クロスコンパイル | cross | ゼロセットアップ |
| Androidビルド | cargo-ndk | NDK統合の標準ツール |
| iOSビルド | Xcodeスクリプト | cargo-lipoは非推奨のため |

---

## 8. リスクと対策

| リスク | 対策 |
|--------|------|
| hudsuckerの開発停滞 | フォークして自社メンテナンス、または http-mitm-proxy に切り替え |
| UniFFI破壊的変更 | バージョン固定、必要に応じてcbindgen+jniにフォールバック |
| gcp-bigquery-clientの非互換 | REST API直接呼び出し（reqwest）にフォールバック |
| rustls互換性問題 | native-tls featureでOpenSSLベースに切り替え可能 |
| cargo-lipoの完全廃止 | 既にXcodeスクリプト方式を採用しているため影響なし |
