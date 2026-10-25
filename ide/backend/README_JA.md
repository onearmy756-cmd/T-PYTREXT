<div align="center">

# ⚡ Fundi Agent

**自己進化する AI エージェント。バイナリひとつで、あらゆるプラットフォームに対応。**

[Lumio Research](https://github.com/Lumio-Research) の [Fundi Agent](https://github.com/Lumio-Research/hermes-agent-rs) を Rust で書き直したプロダクション版。

`110,000行超の Rust` · `1,428 テスト` · `17 クレート` · `~16MB バイナリ`

**[English](./README.md)** · **[中文](./README_ZH.md)** · **[日本語](./README_JA.md)** · **[한국어](./README_KO.md)**

</div>

---

## なぜ Fundi？

🚀 **依存関係ゼロ** — 単一の静的バイナリ。Python も pip も Docker も不要。Raspberry Pi、月額 $3 の VPS、エアギャップサーバーにコピーして、そのまま実行。

🧠 **自己進化エンジン** — マルチアームドバンディットによるモデル選択、長期タスク計画、プロンプトとメモリの自動最適化。使うほどエージェントが賢くなる。

🔌 **17 プラットフォーム · 30 以上のツール · 8 つのメモリバックエンド** — Telegram、Discord、Slack、WhatsApp、Signal、Matrix ほか 11 プラットフォーム。ファイル操作、ブラウザ、コード実行、画像認識、音声、Web 検索、Home Assistant など。

⚡ **真の並行処理** — Rust の tokio ランタイムがツール呼び出しを OS スレッドに分散。30 秒のブラウザスクレイプが 50ms のファイル読み取りをブロックしない。GIL なし。

## クイックスタート

```bash
# インストール
cargo install --path server/agent/crates/fundi-cli

# API キーを設定
echo "ANTHROPIC_API_KEY=sk-..." >> ~/.fundi/.env

# 起動
fundi
```

これだけ。ツール、メモリ、ストリーミング出力が使えるインタラクティブセッションが始まる。

## 何ができる？

**あらゆる LLM と対話** — 会話中にモデルを切り替え：
```
fundi
> /model gpt-4o
> このリポジトリを分析して、セキュリティの問題を見つけて
```

**コマンドラインからワンショット実行**：
```bash
fundi chat --query "auth.rs を新しいエラー型にリファクタリングして"
```

**マルチプラットフォームゲートウェイ** — Telegram、Discord、Slack などを同時接続：
```bash
fundi gateway start
```

**どこでも実行** — Docker、SSH、リモートサンドボックス：
```yaml
# ~/.fundi/config.yaml
terminal:
  backend: docker
  image: ubuntu:24.04
```

**MCP + ACP** — 外部ツールサーバーに接続、または Fundi をツールサーバーとして公開：
```yaml
mcp:
  servers:
    - name: my-tools
      command: npx my-mcp-server
```

**ボイスモード** — VAD + STT + TTS パイプラインでハンズフリー操作。

## アーキテクチャ

```
fundi-cli                    # バイナリエントリポイント、TUI、スラッシュコマンド
├── fundi-agent              # エージェントループ、LLM プロバイダ、メモリプラグイン
│   ├── fundi-core           # 共有型、trait、エラー階層
│   ├── fundi-intelligence   # モデルルーティング、プロンプト構築、自己進化
│   └── fundi-config         # 設定読み込み、YAML/環境変数マージ
├── fundi-tools              # 30 以上のツールバックエンド、承認エンジン
├── fundi-gateway            # 17 プラットフォームアダプタ、セッション管理
├── fundi-environments       # ターミナル: Local/Docker/SSH/Daytona/Modal/Singularity
├── fundi-mcp                # Model Context Protocol クライアント/サーバー
├── fundi-acp                # Agent Communication Protocol
├── fundi-skills             # スキル管理と Hub
├── fundi-cron               # Cron スケジューリング
├── fundi-server             # HTTP/WebSocket API サーバー
├── fundi-eval               # SWE-bench、Terminal-Bench、YC Bench
└── fundi-telemetry          # OpenTelemetry + Prometheus
```

**主要 trait：** `LlmProvider`（10 プロバイダ）· `ToolHandler`（30 以上のバックエンド）· `PlatformAdapter`（17 プラットフォーム）· `TerminalBackend`（6 バックエンド）· `MemoryProvider`（8 プラグイン）

**ツールコールパーサー：** Fundi、Anthropic、OpenAI、Qwen、Llama、DeepSeek、Auto

## インストール

**ワンライナー**（OS と CPU を自動検出）：
```bash
cargo install --path server/agent/crates/fundi-cli
```

**ソースから：**
```bash
cargo install --path server/agent/crates/fundi-cli --locked
```

**手動ダウンロード：** [Releases](https://github.com/Lumio-Research/hermes-agent-rs/releases) からプラットフォーム別バイナリを取得。

**Docker：**
```bash
docker run --rm -it -v ~/.fundi:/root/.fundi ghcr.io/lumio-research/fundi-agent-rs
```

## コントリビュート

コントリビュート歓迎。提出前にテストを実行してください：

```bash
cargo test --workspace        # 1,428 テスト
cargo clippy --workspace      # リント
cargo fmt --all --check       # フォーマット
```

アーキテクチャの詳細とコーディング規約は [AGENTS.md](AGENTS.md) を参照。

## ライセンス

MIT — [LICENSE](LICENSE) を参照。

[Lumio Research](https://github.com/Lumio-Research) の [Fundi Agent](https://github.com/Lumio-Research/hermes-agent-rs) に基づく。
