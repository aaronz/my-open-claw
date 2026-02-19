# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-19

## OVERVIEW

Rust port of [OpenClaw](https://github.com/openclaw/openclaw) — personal AI assistant with WebSocket gateway control plane, multi-channel inbox, and CLI. Currently: core types + gateway skeleton + CLI commands. No real AI provider integration yet (agent responses are stubs).

## STRUCTURE

```
my-open-claw/
├── Cargo.toml                      # Workspace root (resolver v2, 3 members)
├── crates/
│   ├── openclaw-core/              # Shared types: config, session, message protocol, channel trait, errors
│   ├── openclaw-gateway/           # axum WebSocket + HTTP server (the control plane)
│   └── openclaw-cli/               # Binary `openclaw` — clap derive CLI
│       └── src/commands/           # One file per subcommand (gateway, onboard, message, agent, doctor)
└── target/                         # Build artifacts (gitignored)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new WsMessage variant | `core/src/message.rs` | Tagged enum, `#[serde(tag = "type", rename_all = "snake_case")]` |
| Add channel adapter | `core/src/channel.rs` | Implement `Channel` trait (async_trait) |
| Configure channels | `core/src/config.rs` | `ChannelsConfig` — add field + `ChannelInstanceConfig` |
| Handle new WS message type | `gateway/src/ws.rs` → `handle_text_message` | Match arm in the dispatch |
| Add HTTP API endpoint | `gateway/src/routes.rs` → `api_router()` | Add `.route()` call, handler takes `State<Arc<AppState>>` |
| Add CLI subcommand | `cli/src/commands/` + `cli/src/main.rs` | Create file, add to `mod.rs`, add variant to `Commands` enum |
| Change default port | `core/src/config.rs` | `default_port()` → currently 18789 |
| Change default model | `core/src/config.rs` | `default_model()` → currently claude-sonnet-4-20250514 |
| Shared state (sessions, ws_clients) | `gateway/src/state.rs` | `AppState` — DashMap-based, wrapped in `Arc` |
| Config file path | `core/src/config.rs` | `AppConfig::default_path()` → `~/Library/Application Support/ai.openclaw.openclaw/config.yaml` (macOS) |

## CONVENTIONS

- **Workspace deps**: ALL dependency versions declared in root `Cargo.toml` `[workspace.dependencies]`. Crates use `{ workspace = true }`.
- **Error handling**: `openclaw_core::OpenClawError` (thiserror) + `Result<T>` alias in core. CLI uses `anyhow::Result`.
- **Serde**: All protocol types use `#[serde(rename_all = "snake_case")]`. Config uses YAML. WS protocol uses JSON.
- **Concurrency**: `DashMap` for thread-safe maps (sessions, ws_clients). `tokio::sync::broadcast` for WS fan-out.
- **State sharing**: `Arc<AppState>` passed via axum `State` extractor. Never `Mutex<AppState>`.
- **CLI pattern**: Each command is `pub struct XxxArgs` + `pub async fn run(args, config)` in its own file. Commands registered via `#[derive(Subcommand)]` in `main.rs`.
- **Module exports**: `lib.rs` re-exports key types. Consumers use `openclaw_core::AppConfig`, not `openclaw_core::config::AppConfig`.

## ANTI-PATTERNS (THIS PROJECT)

- **DO NOT** add `as any` / type suppressions — this is Rust, use proper types
- **DO NOT** put business logic in `main.rs` — it's a thin dispatcher only
- **DO NOT** add deps to crate `Cargo.toml` without also declaring in workspace root
- **DO NOT** use `Mutex` for `AppState` — use `DashMap` for concurrent access, `Arc` for sharing
- **DO NOT** skip secret redaction — see `redact_secrets()` in routes.rs; any new endpoint exposing config must redact `password`, `token`, `api_key`

## UNIQUE STYLES

- **WsMessage protocol**: Internally tagged JSON (`"type": "send_message"`). Add variants to `WsMessage` enum, NOT separate message structs.
- **Session lookup**: Dual-key — by UUID and by `(ChannelKind, peer_id)` via `peer_index` DashMap.
- **Agent stub pattern**: `SendMessage` handler spawns a tokio task that: broadcasts `AgentThinking`, sleeps 500ms, broadcasts `AgentResponse`. Replace this with real provider calls.
- **Config defaults**: Heavy use of serde `#[serde(default)]` + `Default` impls. Missing YAML fields get sensible defaults — config file is optional.
- **CLI interactive**: `dialoguer` for interactive prompts (onboard wizard). `colored` for terminal output. Non-interactive commands print status and exit.

## COMMANDS

```bash
# Build (requires: source "$HOME/.cargo/env")
cargo build                          # Debug build
cargo build --release                # Release build

# Run CLI
cargo run -p openclaw-cli -- --help
cargo run -p openclaw-cli -- gateway           # Start WS gateway on :18789
cargo run -p openclaw-cli -- onboard           # Interactive setup wizard
cargo run -p openclaw-cli -- doctor            # Run diagnostics
cargo run -p openclaw-cli -- agent --message "hello"
cargo run -p openclaw-cli -- message send --to user --message "hi"

# Test (no tests yet)
cargo test
```

## NOTES

- **No git repo**: Not initialized yet. Run `git init` when ready.
- **No tests**: Zero test coverage. Priority when adding features.
- **Stub responses**: Agent/message commands print placeholders. Gateway WS echoes messages back after 500ms delay. Real AI integration requires implementing provider API calls.
- **WS clients map**: `ws_clients` DashMap stores `broadcast::Sender` per client UUID. Broadcasting sends to ALL connected clients — no per-session routing yet.
- **Config optional**: If config file doesn't exist, `AppConfig::load()` returns defaults. CLI works without `onboard` first.
- **Rust toolchain**: Installed at `$HOME/.cargo/env`. Run `source "$HOME/.cargo/env"` if `cargo` not found.
- **Port 18789**: Matches original TypeScript OpenClaw gateway default.
- **Upstream reference**: Original TypeScript codebase at https://github.com/openclaw/openclaw. Key dirs: gateway/, cli/, channels/, sessions/, config/.
