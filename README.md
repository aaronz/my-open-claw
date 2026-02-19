# OpenClaw 🦞

Rust port of [OpenClaw](https://github.com/openclaw/openclaw) — a personal AI assistant with WebSocket gateway, multi-channel inbox, and CLI.

## Architecture

```
crates/
├── openclaw-core/       # Shared types: config, session, message protocol, channel trait, provider trait, errors
├── openclaw-gateway/    # axum WebSocket + HTTP server (control plane) with AI provider streaming
└── openclaw-cli/        # Binary `openclaw` — clap-based CLI with WS client
```

## Quick Start

```bash
# Build
cargo build

# Interactive setup (provider, API key, model)
cargo run -p openclaw-cli -- onboard

# Start the gateway
cargo run -p openclaw-cli -- gateway

# Send a message (requires running gateway)
cargo run -p openclaw-cli -- agent --message "hello"

# Run diagnostics
cargo run -p openclaw-cli -- doctor

# Run tests
cargo test
```

## Features

- **AI Provider Integration** — Anthropic (Claude) and OpenAI (GPT) with SSE streaming
- **WebSocket Gateway** — Real-time message protocol with JSON-tagged enums
- **Per-Session Routing** — Clients subscribe to sessions; messages route only to subscribers
- **Auth Middleware** — Optional password or token auth on WS + HTTP endpoints
- **Multi-Channel** — Telegram, Discord, Slack, WhatsApp, Signal, WebChat channel trait
- **CLI** — Interactive onboard wizard, gateway management, agent chat, diagnostics
- **YAML Config** — Sensible defaults, optional config file, secret redaction on API

## Configuration

Config file location: `~/Library/Application Support/ai.openclaw.openclaw/config.yaml` (macOS)

```yaml
gateway:
  port: 18789
  bind: loopback
  auth:
    mode: none  # none | password | token
models:
  default_model: claude-sonnet-4-20250514
  providers:
    - name: anthropic
      model: claude-sonnet-4-20250514
      api_key: sk-...
agent:
  system_prompt: "You are a helpful assistant."
  thinking: medium
```

## WebSocket Protocol

All messages are JSON with `"type"` tag:

```json
{"type": "send_message", "content": "hello", "session_id": null, "channel": null, "peer_id": "cli"}
{"type": "agent_thinking", "session_id": "..."}
{"type": "agent_response", "session_id": "...", "content": "token", "done": false}
{"type": "agent_response", "session_id": "...", "content": "full response", "done": true}
```

## HTTP API

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check with uptime |
| `GET /api/sessions` | List all sessions |
| `GET /api/config` | Config (secrets redacted) |
| `GET /api/status` | Server status |

## License

MIT
