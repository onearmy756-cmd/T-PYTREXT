# Fundi Agent (Rust) — Development Guide

Instructions for AI coding assistants and developers working on the fundi-agent-rust codebase.

> **Parity status**: 13/13 scoped items completed. See `README.md` for the full tracker.

---

## Build & Test

```bash
cargo build --release                    # Binary at target/release/fundi
cargo test --workspace                   # 1,428 tests, all crates
cargo test -p fundi-parity-tests        # Parity fixture tests only
cargo clippy --workspace --all-targets   # Lint (target: -D warnings)
cargo fmt --all --check                  # Format check

# Eval framework with real agent loop
cargo build -p fundi-eval --features agent-loop
```

### Artifact 锁：`Blocking waiting for file lock on artifact directory`

终端里 **`cargo run`** 与编辑器里的 **rust-analyzer**（`cargo check`）会争用同一 **`target/`** artifact 锁，表现为长时间阻塞。

**仓库内缓解（已提交）：**

1. **`.vscode/settings.json`**：`"rust-analyzer.cargo.targetDir": true` —— 让 rust-analyzer 使用 `target` 下**独立子目录**做检查，与命令行 `cargo build` / `cargo run` 分离（**改完后请重载窗口 / 重启 rust-analyzer**）。
2. **`scripts/fundi`**：若已存在 `target/debug/fundi`，则**直接执行**该二进制，避免每次 `cargo run` 抢锁；需要强制走 cargo 时：`FUNDI_USE_CARGO=1 ./scripts/fundi …`。首次可 `chmod +x scripts/fundi`，或始终 `sh scripts/fundi …`。

若仍卡住：确认没有其它终端在长时间 `cargo build`，必要时结束残留 `cargo` 进程后再试。

---

## Project Structure

```
fundi-agent-rust/
├── Cargo.toml                # Workspace root — 16 crates, pinned deps
├── AGENTS.md                 # This file (loaded by Fundi at runtime)
├── PARITY_PLAN.md            # 8-week parity roadmap
├── Dockerfile                # Multi-stage release build
├── scripts/
│   ├── fundi                # 优先 target/debug/fundi，减轻与 RA 的 artifact 锁竞争
│   ├── install.sh           # installer
│   └── ci/                  # E2E scripts
└── crates/
    ├── fundi-core/          # Shared types, traits, error hierarchy
    ├── fundi-agent/         # Agent loop, LLM providers, memory plugins, context
    ├── fundi-tools/         # Tool registry, dispatch, 30 tool backends
    ├── fundi-gateway/       # Message gateway, 17 platform adapters
    ├── fundi-cli/           # CLI/TUI binary, slash commands, app state
    ├── fundi-config/        # Config loading, YAML/JSON/env merging, validation
    ├── fundi-intelligence/  # Smart routing, prompt builder, usage pricing, display
    ├── fundi-skills/        # Skill management, file store, hub client, versioning
    ├── fundi-environments/  # Terminal backends (Local/Docker/SSH/Daytona/Modal/Singularity)
    ├── fundi-cron/          # Cron scheduling and persistence
    ├── fundi-mcp/           # Model Context Protocol client/server
    ├── fundi-acp/           # Agent Communication Protocol
    ├── fundi-eval/          # Evaluation framework (Runner, BenchmarkAdapter, Verifier)
    ├── fundi-server/        # HTTP/WebSocket API server
    ├── fundi-telemetry/     # OpenTelemetry + Prometheus metrics
    └── fundi-parity-tests/  # Golden fixture tests for behavioral parity
```

**User config:** `~/.fundi/config.yaml` (settings), `~/.fundi/.env` (API keys)

## Crate Dependency Chain

```
fundi-cli (binary entry point)
├── fundi-agent (core loop)
│   ├── fundi-core (types, traits, errors)
│   ├── fundi-intelligence (routing, prompts, pricing)
│   ├── fundi-config (config loading)
├── fundi-tools (30 tool backends)
├── fundi-gateway (17 platform adapters)
│   └── fundi-core
├── fundi-server (REST/WS server)
├── fundi-cron (scheduling)
├── fundi-skills (skill management)
├── fundi-mcp (MCP client/server)
├── fundi-environments (terminal backends)
└── fundi-telemetry (metrics)

fundi-eval (benchmarking, optional)
├── fundi-agent (via feature `agent-loop`)
└── fundi-environments
```

---

## Core Traits (fundi-core)

All cross-crate abstractions live in `fundi-core::traits`:

```rust
// LLM provider — Anthropic, OpenAI, OpenRouter, Generic, Nous, Qwen, Kimi, MiniMax, Copilot, Codex
trait LlmProvider: Send + Sync {
    async fn chat_completion(&self, messages, tools, max_tokens, temperature, model, extra_body) -> Result<LlmResponse, AgentError>;
    fn chat_completion_stream(&self, ...) -> BoxStream<Result<StreamChunk, AgentError>>;
}

// Tool execution — 30 backends implement this
trait ToolHandler: Send + Sync {
    async fn execute(&self, params: Value) -> Result<String, ToolError>;
    fn schema(&self) -> ToolSchema;
}

// Platform messaging — 17 adapters implement this
trait PlatformAdapter: Send + Sync {
    async fn start(&self) -> Result<(), GatewayError>;
    async fn stop(&self) -> Result<(), GatewayError>;
    async fn send_message(&self, chat_id, text, parse_mode) -> Result<(), GatewayError>;
    async fn edit_message(&self, chat_id, message_id, text) -> Result<(), GatewayError>;
    async fn send_file(&self, chat_id, file_path, caption) -> Result<(), GatewayError>;
    fn platform_name(&self) -> &str;
}

// Terminal / shell — Local, Docker, SSH, Daytona, Modal, Singularity
trait TerminalBackend: Send + Sync {
    async fn execute_command(&self, command, timeout, workdir, background, pty) -> Result<CommandOutput, AgentError>;
    async fn read_file(&self, path, offset, limit) -> Result<String, AgentError>;
    async fn write_file(&self, path, content) -> Result<(), AgentError>;
    async fn file_exists(&self, path) -> Result<bool, AgentError>;
}

// Memory — 8 external plugins + built-in file/SQLite
trait MemoryProvider: Send + Sync { ... }

// Skills — file store + hub
trait SkillProvider: Send + Sync { ... }
```

## Error Hierarchy

```
AgentError (top-level, fundi-core)
├── LlmApi(String)
├── ToolExecution(String)      ← auto From<ToolError>
├── Gateway(String)            ← auto From<GatewayError>
├── Config(String)             ← auto From<ConfigError>
├── RateLimited { retry_after_secs }
├── Interrupted { message }
├── AuthFailed(String)
├── ContextTooLong
├── MaxTurnsExceeded
├── InvalidToolCall(String)
├── Timeout(String)
└── Io(String)

ToolError: ExecutionFailed | InvalidParams | NotFound | Timeout | SchemaViolation
GatewayError: ConnectionFailed | SendFailed | Platform | Auth | SessionExpired
ConfigError: ParseError | NotFound | ValidationError | IoError
```

Use `thiserror` for all error types. The compiler enforces every error path via `From` traits.

---

## Agent Loop (fundi-agent)

`AgentLoop` in `crates/fundi-agent/src/agent_loop.rs` is the core engine:

```rust
pub struct AgentLoop {
    config: AgentConfig,
    provider: Arc<dyn LlmProvider>,
    tool_registry: ToolRegistry,
    context_manager: ContextManager,
    memory_manager: MemoryManager,
    plugin_manager: PluginManager,
    skill_orchestrator: SkillOrchestrator,
    interrupt_controller: InterruptController,
    // ...
}
```

### Main Loop

```
while turn < max_turns && budget.remaining > 0 && !interrupted:
    1. Build messages (system prompt + history + memory snapshots)
    2. Apply prompt caching markers (Anthropic: persistent system + ephemeral recent turns)
    3. Send to LLM via provider.chat_completion_stream()
    4. If response has tool_calls:
         - Execute tools in parallel via tokio::task::JoinSet
         - Append tool results to history
         - Fire memory/skill lifecycle hooks
         - Continue loop
    5. Else: return final response
```

### Key Methods

- `run(messages, system_prompt) -> AgentResult` — synchronous (waits for completion)
- `run_stream(messages, system_prompt) -> Stream<StreamChunk>` — streaming output
- `with_sub_agent_orchestrator(orch)` — attach in-process sub-agent spawner

### AgentConfig

```rust
pub struct AgentConfig {
    pub model: String,
    pub max_turns: u32,
    pub budget: BudgetConfig,           // max_usd, max_input_tokens, max_output_tokens
    pub retry: RetryConfig,             // max_retries, base_delay_ms, fallback_model
    pub smart_routing: SmartModelRoutingConfig,
    pub personality: Option<String>,
    pub skip_context_files: bool,
    pub skip_memory: bool,
    pub background_review_enabled: bool,
    // ...
}
```

---

## Tool System (fundi-tools)

### Registry

`ToolRegistry` in `crates/fundi-tools/src/registry.rs` — thread-safe (`Arc<Mutex<...>>`):

```rust
registry.register(
    name,           // "web_search"
    toolset,        // "web"
    schema,         // ToolSchema (OpenAI function calling format)
    handler,        // Arc<dyn ToolHandler>
    check_fn,       // Arc<dyn Fn() -> bool> — availability check
    env_deps,       // vec!["EXA_API_KEY"]
    is_async,       // true
    description,    // human-readable
    emoji,          // "🔍"
    max_result_size_chars,  // Option<usize>
);
```

### Built-in Registration

`register_builtin_tools()` in `crates/fundi-tools/src/register_builtins.rs` — equivalent of `_discover_tools()` in the original codebase. Takes a `TerminalBackend` and `SkillProvider`, instantiates all handlers.

### Tool Backends (30)

```
crates/fundi-tools/src/
├── tools/           # Tool handler implementations
│   ├── file.rs          # read_file, write_file, patch, search_files
│   ├── terminal.rs      # terminal, process management
│   ├── web.rs           # web_search, web_extract
│   ├── browser.rs       # browser automation (CDP/Firecrawl/BrowserBase)
│   ├── code_execution.rs # execute_code (sandboxed)
│   ├── memory.rs        # memory add/replace/remove
│   ├── session_search.rs # session_search (FTS5 + LLM summaries)
│   ├── delegation.rs    # delegate_task (sub-agent)
│   ├── skills.rs        # skill management
│   ├── vision.rs        # image analysis
│   ├── image_gen.rs     # DALL-E, Midjourney
│   ├── tts.rs           # text-to-speech
│   ├── transcription.rs # speech-to-text
│   ├── cronjob.rs       # cron scheduling
│   ├── messaging.rs     # send_message (real delivery)
│   ├── homeassistant.rs # Home Assistant control
│   ├── todo.rs          # todo management
│   ├── clarify.rs       # ask user for clarification
│   ├── osv_check.rs     # OSV vulnerability scanning
│   ├── url_safety.rs    # URL safety checking
│   └── ...
├── backends/        # Backend trait implementations
├── approval.rs      # Dangerous command detection (deny/confirm/approve)
├── credential_guard.rs  # Secret file protection
├── toolset_distributions.rs  # 17 named toolset profiles
└── v4a_patch.rs     # V4A patch format parser (codex/cline compat)
```

### Adding a New Tool

1. Create `crates/fundi-tools/src/tools/your_tool.rs`:
```rust
pub struct YourToolHandler { backend: Arc<dyn YourBackend> }

#[async_trait]
impl ToolHandler for YourToolHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> { ... }
    fn schema(&self) -> ToolSchema {
        tool_schema("your_tool", "Description", JsonSchema::object(props, required))
    }
}
```

2. Register in `crates/fundi-tools/src/register_builtins.rs`:
```rust
reg(registry, "your_toolset", Arc::new(YourToolHandler::new(backend)), "🔧", vec![]);
```

3. Add `pub mod your_tool;` to `crates/fundi-tools/src/tools/mod.rs`.

---

## CLI Architecture (fundi-cli)

### Entry Point

`crates/fundi-cli/src/main.rs` → `#[tokio::main] async fn main()`:
1. Parse CLI args via `clap` (`Cli::parse()`)
2. Initialize tracing
3. Dispatch to subcommand handler

### Subcommands (cli.rs)

Defined via `clap::Subcommand` enum `CliCommand`:
- `Fundi` — interactive session (default)
- `Chat { query, preload_skill, yolo }` — one-shot mode
- `Model`, `Tools`, `Config`, `Gateway`, `Setup`, `Doctor`, `Update`, `Status`, `Logs`, `Profile`, `Auth`, `Cron`, `Webhook`

### App State (app.rs)

```rust
pub struct App {
    pub config: Arc<GatewayConfig>,
    pub agent: Arc<AgentLoop>,
    pub tool_registry: Arc<ToolRegistry>,
    pub messages: Vec<Message>,
    pub session_id: String,
    pub current_model: String,
    pub current_personality: Option<String>,
    pub interrupt_controller: InterruptController,
    pub stream_handle: Option<StreamHandle>,
    // ...
}
```

### Slash Commands (commands.rs)

Centralized in `SLASH_COMMANDS` array:
```rust
pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/new", "Start a new session"),
    ("/reset", "Reset the current session"),
    ("/retry", "Retry the last user message"),
    ("/model", "Show or switch the current model"),
    ("/personality", "Show or switch personality"),
    ("/skills", "List available skills"),
    ("/tools", "List registered tools"),
    ("/compress", "Trigger context compression"),
    ("/usage", "Show token usage statistics"),
    ("/yolo", "Toggle auto-approve mode"),
    // ... 22 total
];
```

Dispatch: `handle_slash_command(app, cmd, args) -> CommandResult { Handled | NeedsAgent | Quit }`

### Adding a Slash Command

1. Add entry to `SLASH_COMMANDS` in `commands.rs`
2. Add match arm in `handle_slash_command()`
3. If gateway-visible, add handler in `fundi-gateway/src/commands.rs`

---

## Configuration (fundi-config)

### Loading Order

1. `~/.fundi/.env` — loaded via `load_dotenv()` (env vars win over file)
2. `~/.fundi/config.yaml` — parsed into `GatewayConfig`
3. Environment variable overrides
4. CLI flag overrides

### GatewayConfig (config.rs)

```rust
pub struct GatewayConfig {
    pub model: Option<String>,
    pub personality: Option<String>,
    pub max_turns: u32,                          // default: 90
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
    pub budget: BudgetConfig,
    pub platforms: HashMap<String, PlatformConfig>,
    pub session: SessionConfig,
    pub streaming: StreamingConfig,
    pub terminal: TerminalConfig,
    pub web: ToolCapabilityConfig,
    pub image_gen: ToolCapabilityConfig,
    pub tts: ToolCapabilityConfig,
    pub browser: ToolCapabilityConfig,
    pub llm_providers: HashMap<String, LlmProviderConfig>,
    pub smart_model_routing: SmartModelRoutingConfig,
    pub proxy: Option<ProxyConfig>,
    pub approval: ApprovalConfig,
    pub skills: SkillsSettings,
    pub tools_config: ToolsSettings,
    pub mcp: McpSettings,
    // ...
}
```

### Adding Configuration

1. Add field to `GatewayConfig` in `crates/fundi-config/src/config.rs` with `#[serde(default)]`
2. If it needs validation, add check in `validate_config()`
3. For env vars, document in `~/.fundi/.env` format

---

## Gateway & Platform Adapters (fundi-gateway)

### Architecture

```
Gateway (orchestrator)
├── SessionManager — conversation persistence
├── StreamManager — progressive message updates
├── MediaCache — image/audio/document caching
├── DmManager — unauthorized user handling
├── HookRegistry — event hooks
├── ChannelDirectory — persistent channel mapping
├── DeliveryRouter — message routing + retry
└── PlatformAdapter implementations (17)
```

### 17 Platform Adapters

Telegram, Discord, Slack, WhatsApp, Signal, Matrix, Mattermost, DingTalk, Feishu, WeCom, Weixin, Email, SMS, BlueBubbles, Home Assistant, Webhook, API Server.

Each implements `PlatformAdapter` trait. Located in `crates/fundi-gateway/src/platforms/`.

### Adding a Platform Adapter

1. Create `crates/fundi-gateway/src/platforms/your_platform.rs`
2. Implement `PlatformAdapter` trait
3. Add feature flag in `crates/fundi-gateway/Cargo.toml`
4. Wire into `Gateway` startup in `crates/fundi-cli/src/main.rs`

---

## Intelligence & Routing (fundi-intelligence)

```
fundi-intelligence/src/
├── router.rs           # SmartModelRouter — per-turn cheap/strong routing
├── anthropic_adapter.rs # Anthropic message format + cache markers
├── prompt/             # PromptBuilder — system prompt assembly
├── context_engine.rs   # Token estimation, message compression
├── credential_pool.rs  # Key rotation across multiple API keys
├── error_classifier.rs # Error categorization + retry strategy
├── model_metadata.rs   # Context lengths, capabilities, pricing
├── usage.rs            # Token counting, cost calculation
├── usage_pricing.rs    # Per-model pricing tables
├── display.rs          # Tool call formatting, progress bars
├── title.rs            # Session title generation
├── redact.rs           # PII/secret redaction
├── session_insights/   # Nudge counters, background review
├── auxiliary/          # Auxiliary LLM calls (summaries, classification)
└── models_dev.rs       # models.dev registry integration
```

---

## Memory System (fundi-agent)

### MemoryManager

`crates/fundi-agent/src/memory_manager.rs` — orchestrates built-in + ONE external plugin:

- **Built-in**: `~/.fundi/memories/MEMORY.md` + `USER.md` (file-based, always active)
- **External**: one of 8 plugins (Mem0, Honcho, Holographic, Hindsight, ByteRover, OpenViking, RetainDB, Supermemory)

### Memory Tool Semantics

- `action=add|replace|remove`, `target=memory|user`
- `old_text` substring matching for replace/remove
- Store limits: `memory` ≈ 2200 chars, `user` ≈ 1375 chars

### Lifecycle Hooks

```
on_memory_write    — after any memory mutation
queue_prefetch     — background recall for next turn
on_pre_compress    — before context compression
on_session_end     — session teardown
on_delegation      — sub-agent delegation lineage
```

---

## Security

### ApprovalManager (fundi-tools/src/approval.rs)

Three-tier command classification:
- **Denied**: `rm -rf /`, `mkfs`, `dd of=/dev/`, `chmod 777` — blocked outright
- **RequiresConfirmation**: `sudo`, `kill -9`, `docker rm`, `git push --force`, `curl | sh` — auto-approved in agent mode with warning log
- **Approved**: everything else

### CredentialGuard (fundi-tools/src/credential_guard.rs)

- Blocks read/write to protected paths (`.env`, `credentials.json`, SSH keys, etc.)
- Scans file content for secret patterns before write

### Context File Security (fundi-agent/src/context_files.rs)

- Prompt injection detection (invisible Unicode, threat patterns)
- Per-file 20K char limit with head/tail truncation
- Blocked files get `[BLOCKED: ...]` placeholder

---

## MCP Integration (fundi-mcp)

- **McpClient** / **McpManager**: connect to external MCP servers, discover tools at runtime
- **McpServer**: expose Fundi tools to external MCP clients
- **McpCapabilityPolicy**: deny-by-default gating (`allow_tool_invoke`, `allow_prompt_read`, `allow_resource_read`)
- **Transport**: stdio and HTTP/SSE
- **Auth**: OAuth and bearer token for remote servers

---

## Context Files

Fundi auto-loads context files at startup:

| File | Location | Purpose |
|------|----------|---------|
| `AGENTS.md` | Working directory (walks up to git root) | Project-level instructions |
| `agents.md` | Working directory | Same (case-insensitive fallback) |
| `.fundi.md` | Working directory | Alternative name |
| `~/.fundi/context/*.md` | User home | Global context files |
| `~/.fundi/memories/MEMORY.md` | User home | Persistent memory snapshot |
| `~/.fundi/memories/USER.md` | User home | User profile snapshot |

Files are scanned for prompt injection, truncated at 20K chars, and injected into the system prompt.

---

## Parity Testing

### Framework

`crates/fundi-parity-tests/` — golden JSON fixtures that validate behavioral correctness, replayed against Rust.

### Active Modules (registry.json)

| Module | Source | Rust Target |
|--------|--------|-------------|
| `anthropic_adapter` | Anthropic message formatting | `fundi_intelligence::anthropic_adapter` |
| `fundi_core_tool_format` | XML round-trip | `fundi_core::tool_call_parser` |
| `checkpoint_manager` | Git shadow snapshots | `fundi_parity_tests::harness` |

### Recording Fixtures

```bash
# Fixtures were captured once from the legacy Python reference; MTECHOS runs Rust only
```

### Adding a Parity Test

1. Record fixture JSON from the reference implementation
2. Place in `crates/fundi-parity-tests/fixtures/<module>/`
3. Add module entry to `fixtures/registry.json`
4. Implement Rust-side assertion in `crates/fundi-parity-tests/src/harness.rs`

---

## Coding Conventions

### Must

1. **Read the reference implementation first** before porting any module.
2. **API naming**: keep `snake_case` consistent with the original.
3. **Errors**: use existing `AgentError` / `ToolError` / `GatewayError` — don't create parallel hierarchies.
4. **Logging**: `tracing::{debug,info,warn,error}` — no `println!` (except CLI user output).
5. **Async**: tokio only, no async-std.
6. **Tests**: golden fixtures in `crates/fundi-parity-tests/fixtures/<module>/*.json`.
7. **Commits**: `parity(<module>): <description>`

### Must Not

- Don't modify workspace member structure without clear motivation (update root `Cargo.toml` if you do).
- Don't add top-level dependencies that conflict with existing version pins.
- Don't leave new `clippy` warnings (target: `-D warnings`).
- Don't hardcode `~/.fundi` paths — use `fundi_config::fundi_home()`.
- Don't break prompt caching — don't alter past context, change toolsets, or rebuild system prompts mid-conversation.
- Don't hardcode cross-tool references in schema descriptions (tools may be unavailable).

---

## Known Pitfalls

### Two ToolRegistry types exist

`fundi_agent::agent_loop::ToolRegistry` (minimal, for standalone agent tests) and `fundi_tools::ToolRegistry` (full-featured, used in production). The CLI bridges them via `bridge_tool_registry()` in `app.rs`. Don't confuse the two.

### Prompt caching is fragile

Anthropic cache markers are applied in `messages_for_api_call`. Any change that alters the system prompt or early message history mid-conversation will invalidate the cache and spike costs.

### `unsafe { std::env::set_var() }` in config loading

`load_dotenv()` uses unsafe env mutation at startup. This is called once before multi-threading starts. Don't call it later.

### Platform adapters are feature-gated

Most gateway platforms are behind Cargo feature flags. Check `crates/fundi-gateway/Cargo.toml` before assuming an adapter is compiled in.

### Sub-agent depth is capped

`SubAgentOrchestrator` enforces `max_depth` (default 4). Child agents run in their own `tokio::spawn` task with wall-clock timeout and cooperative cancellation via `InterruptController`.

---

## Reference

Architecture details (CLI architecture, TUI process model, slash command registry,
skin engine, profile system, testing conventions) are documented in
[README.md](README.md) and in the crate sections above.
