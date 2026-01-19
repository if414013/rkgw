<div align="center">

# 🦀 rkgw — Rust Kiro Gateway

**High-performance proxy gateway for Kiro API (AWS CodeWhisperer)**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

*A Rust rewrite of [kiro-gateway](https://github.com/jwadow/kiro-gateway) — Use Claude models through any OpenAI or Anthropic compatible tool*

[Features](#-features) • [Quick Start](#-quick-start) • [Configuration](#%EF%B8%8F-configuration) • [API Reference](#-api-reference)

</div>

---

## 🙏 Attribution

This project is a Rust rewrite of the original [kiro-gateway](https://github.com/jwadow/kiro-gateway) by [@Jwadow](https://github.com/jwadow). The original project is written in Python using FastAPI.

**Why Rust?**
- ⚡ Faster startup time and lower memory footprint
- 🔒 Memory safety without garbage collection
- 📦 Single binary deployment — no runtime dependencies

---

## 🤖 Supported Models

| Model | Description |
|-------|-------------|
| 🧠 **Claude Opus 4.5** | Most powerful. Complex reasoning, deep analysis, research |
| 🚀 **Claude Sonnet 4.5** | Balanced. Coding, writing, general-purpose |
| ⚡ **Claude Haiku 4.5** | Lightning fast. Quick responses, simple tasks |
| 📦 **Claude Sonnet 4** | Previous generation. Reliable for most use cases |
| 📦 **Claude 3.7 Sonnet** | Legacy model. Backward compatibility |

> 💡 **Smart Model Resolution:** Use any model name format — `claude-sonnet-4-5`, `claude-sonnet-4.5`, or versioned names like `claude-sonnet-4-5-20250929`. The gateway normalizes them automatically.

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🔌 **OpenAI-compatible API** | Works with any OpenAI-compatible tool |
| 🔌 **Anthropic-compatible API** | Native `/v1/messages` endpoint |
| 🧠 **Extended Thinking** | Reasoning support |
| 👁️ **Vision Support** | Send images to model |
| 🛠️ **Tool Calling** | Function calling support |
| 💬 **Full message history** | Complete conversation context |
| 📡 **Streaming** | Full SSE streaming support |
| 🔄 **Retry Logic** | Automatic retries on errors |
| 🔐 **Smart token management** | Automatic refresh before expiration |

---

## 🏗️ Architecture

<details>
<summary>View architecture documentation</summary>

For detailed architecture documentation including component diagrams, data flows, and implementation details, see **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**

</details>

---

## 💡 API Usage Examples

### OpenAI API

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "Authorization: Bearer my-super-secret-password-123" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'
```

### Anthropic API

```bash
curl http://localhost:8000/v1/messages \
  -H "x-api-key: my-super-secret-password-123" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

---

## 🖥️ OpenCode Setup

To use this gateway with [OpenCode](https://opencode.ai), add the following provider configuration to your global config file at `~/.config/opencode/opencode.json`. This makes the Kiro provider available across all your projects.

For more details on OpenCode configuration, see the [OpenCode Config Documentation](https://opencode.ai/docs/config/).

https://github.com/user-attachments/assets/7a3ab9ba-15b4-4b96-95df-158602ed08b0

```json
{
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    "kiro": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Kiro Proxy",
      "options": {
        "baseURL": "http://127.0.0.1:9000/v1",
        "apiKey": "your-proxy-api-key"
      },
      "models": {
        "auto": {
          "name": "Auto"
        },
        "claude-haiku-4.5": {
          "name": "Claude Haiku 4.5"
        },
        "claude-opus-4.5": {
          "name": "Claude Opus 4.5"
        },
        "claude-sonnet-4": {
          "name": "Claude Sonnet 4"
        },
        "claude-sonnet-4.5": {
          "name": "Claude Sonnet 4.5"
        }
      }
    }
  }
}
```

> **Note:** Replace `your-proxy-api-key` with the value of your `PROXY_API_KEY` environment variable, and adjust the `baseURL` port if you're running the gateway on a different port.

---

## 🔧 Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

---

## 📜 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

This means:
- ✅ You can use, modify, and distribute this software
- ✅ You can use it for commercial purposes
- ⚠️ **You must disclose source code** when you distribute the software
- ⚠️ **Network use is distribution** — if you run a modified version on a server, you must make the source code available
- ⚠️ Modifications must be released under the same license

See the [LICENSE](LICENSE) file for the full license text.

### Contributor License Agreement (CLA)

By submitting a contribution to this project, you agree to the terms of our [Contributor License Agreement (CLA)](CLA.md).

---

## ⚠️ Disclaimer

This project is not affiliated with, endorsed by, or sponsored by Amazon Web Services (AWS), Anthropic, or Kiro IDE. Use at your own risk and in compliance with the terms of service of the underlying APIs.

---

<div align="center">

**[⬆ Back to Top](#-rkgw--rust-kiro-gateway)**

</div>
