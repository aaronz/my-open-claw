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
    -   **WhatsApp**: Support via Cloud API.
    -   **Signal**: Bridge support.
    -   **Matrix**: Decentralized messaging.
    -   **BlueBubbles**: iMessage integration.
    -   **Mattermost**: Enterprise chat.
    -   **Web/CLI**: Real-time WebSocket interface with Artifact support.
-   **🛠️ Tools & Capabilities**:
    -   **Browser Control**: Headless Chrome for web interaction and data extraction.
    -   **Canvas**: Persistent visual workspace for code, documents, and charts (with Mermaid diagram support).
    -   **YouTube Search**: Find and display video information directly.
    -   **Web Search**: Real-time information via Tavily.
    -   **Code Interpreter**: Safe(ish) Python execution for calculations and logic.
    -   **Shell**: Local command execution (guarded).
    -   **Cron**: Reminders and recurring tasks.
-   **🧩 Skills System**:
    -   **GitHub**: Manage issues and get repository information (authenticated).
    -   **Obsidian**: Read and update your local vault notes.
    -   **Spotify**: Control playback and search music.
    -   **Linear**: Issue and project management.
    -   **Google**: Calendar and Sheets integration.
    -   **Todoist**: Task management.
    -   **1Password**: Secret retrieval simulation.
    -   **Custom Skills**: Easily extensible via the Skill trait.
-   **💾 Long-term Memory**:
    -   **Vector RAG**: Stores every interaction in Qdrant (local Docker) or In-memory.
    -   **SQLite Persistence**: Robust local session and message history.
    -   **Context Compaction**: Automatically summarizes long conversations to save tokens.
-   **🛡️ Reliability**:
    -   **Model Failover**: Automatically rotates through providers on failure.
    -   **Presence**: Real-time online/offline status updates.
    -   **OAuth**: Secure skill authentication infrastructure.
-   **🤖 Headless Autonomy**: Webhook endpoints triggers agent logic autonomously.
-   **🐳 Production Ready**: Docker & Docker Compose support.

## 🚀 Quick Start

1.  **Dev Mode (Minimal Dependencies)**:
    ```bash
    cargo run -p openclaw-cli -- dev
    ```
    Starts the gateway with Mock AI and In-memory storage. No keys required.

2.  **Production (Wizard)**:
    ```bash
    cargo run -p openclaw-cli -- onboard
    ```
    Guides you through setting up OpenAI/Claude and Qdrant.

3.  **Start Gateway**:
    ```bash
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
