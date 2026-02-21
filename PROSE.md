# OpenProse Specification

OpenProse is a "Living Prompt" system used by OpenClaw to inject dynamic, structured context into the agent's system prompt.

## 📝 File Formats
- **.prose**: Native prose format for behavioral descriptions.
- **.md**: Markdown format for hierarchical knowledge bases.

## 🔄 Loading Logic
The OpenClaw Gateway automatically scans the workspace directory for the following files:
1. `SOUL.md`: Core personality and identity.
2. `AGENTS.md`: Task-specific agent roles.
3. `TOOLS.md`: Documentation for available tools.
4. `CORE.prose`: Low-level system instructions.

## 🚀 Usage
To update the agent's behavior, simply edit the `.prose` or `.md` files in your workspace. The changes are picked up on the next agent cycle.
