# OpenClaw 🦞

A powerful, multi-channel, autonomous AI agent gateway written in Rust. Port of the original [OpenClaw](https://github.com/openclaw/openclaw).

## ✨ Features

-   **🧠 Multi-LLM Support**: OpenAI, Anthropic (Claude), Google Gemini.
-   **🗣️ Multi-Modal**:
    -   **Voice**: Automatic STT (Whisper) and TTS (OpenAI Audio). Speak to your bot, it speaks back.
    -   **Vision**: Send images to the agent (Discord/URL).
-   **🔌 Multi-Channel**:
    -   **Telegram**: Full support (Text, Voice, Groups).
    -   **Discord**: Full support (Text, Voice, Images, Threads).
    -   **Slack**: Inbound/Outbound via Socket Mode.
    -   **Web/CLI**: Real-time WebSocket interface.
-   **🛠️ Tools & Capabilities**:
    -   **Web Search**: Real-time information via Tavily.
    -   **Code Interpreter**: Safe(ish) Python execution for calculations and logic.
    -   **Weather**: Built-in weather lookup.
-   **💾 Long-term Memory**:
    -   **Vector RAG**: Stores every interaction in Qdrant (local Docker).
    -   **Context Compaction**: Automatically summarizes long conversations to save tokens.
    -   **Persistence**: Session state saved to disk.
-   **🤖 Headless Autonomy**: Webhook endpoint (`/api/webhook`) triggers agent logic autonomously.
-   **🐳 Production Ready**: Docker & Docker Compose support.

## 🚀 Quick Start (Docker)

1.  **Configure**: Edit `docker-compose.yml` or `config/config.yaml` (see Configuration below).
2.  **Run**:
    ```bash
    docker-compose up -d
    ```
3.  **Chat**: Use the CLI or connect your Telegram/Discord bot.

## 🛠️ Manual Installation

Requirements: Rust 1.75+, `libclang` (for some crates), Docker (for Qdrant).

```bash
# 1. Start Vector DB
docker run -p 6333:6333 qdrant/qdrant

# 2. Configure (Wizard)
cargo run -p openclaw-cli -- onboard

# 3. Start Gateway
cargo run -p openclaw-cli -- gateway
```

## ⚙️ Configuration

Location: `config.yaml` (in `~/.config/openclaw` or local `config/`).

```yaml
gateway:
  port: 18789
  auth: { mode: none } # or 'token'

models:
  default_model: gpt-4o
  providers:
    - name: openai
      model: gpt-4o
      api_key: sk-...
    - name: anthropic
      model: claude-3-5-sonnet-20240620
      api_key: sk-ant-...
    - name: gemini
      model: gemini-1.5-pro
      api_key: ...

channels:
  telegram:
    enabled: true
    token: "YOUR_BOT_TOKEN"
  discord:
    enabled: true
    token: "YOUR_BOT_TOKEN"
  slack:
    enabled: true
    token: "xoxb-..."
    app_token: "xapp-..."

agent:
  system_prompt: "You are OpenClaw."
  tavily_api_key: "tvly-..." # Enable Web Search

audio:
  enabled: true # Enable Voice (STT/TTS)
  openai_api_key: "sk-..."

memory:
  enabled: true
  qdrant_url: "http://localhost:6333"
```

## 📚 Tools

The agent can use these tools automatically:
-   `web_search(query)`: Search the internet.
-   `python_interpreter(code)`: Execute Python code.
-   `get_weather(location)`: Check weather.

## 🧠 Memory & RAG

OpenClaw uses **Qdrant** for vector memory. It embeds all conversations locally using **FastEmbed** (no API cost for embeddings) and retrieves relevant context before answering.

## 📡 API Endpoints

-   `WS  /ws`: Real-time chat protocol.
-   `POST /api/webhook`: Trigger agent from external sources.
-   `GET /api/status`: System status.

## License

MIT
