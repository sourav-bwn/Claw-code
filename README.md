# Uclaw - Unified AI Platform

> Better Harness Tools, Make Real Things Done

A unified AI platform combining CLI, Web UI, VS Code extension, and GitHub Actions into one powerful AI coding assistant.

## Features

- **Multi-Provider Support**: Anthropic Claude, OpenAI GPT, Google Gemini, and more
- **CLI Interface**: Terminal-based AI coding assistant
- **Web UI**: Browser-based chat and code analysis
- **VS Code Extension**: Inline AI suggestions and chat panel
- **GitHub Actions**: Automated PR reviews and code analysis
- **MCP Server**: Plugin ecosystem for extending functionality
- **Cloud-Ready**: Deploy locally or on any cloud platform

## Quick Start

```bash
# Install CLI
cargo install --path rust/crates/uc law-cli

# Or use Docker
docker run -it uc law/uc law:latest

# Start web UI
cd web && uvicorn app:app --reload
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Uclaw Platform                          │
├─────────────────────────────────────────────────────────────┤
│  CLI          │  Web UI      │  VS Code    │  GitHub Action │
├───────────────┼──────────────┼─────────────┼────────────────┤
│               │              │             │                │
│  ┌────────────┴──────────────┴─────────────┴────────────┐  │
│  │              UNIFIED AGENT CORE                        │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐          │  │
│  │  │ Tools  │ │Session │ │ Config │ │  MCP   │          │  │
│  │  │ Engine │ │Manager │ │ System │ │ Layer  │          │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘          │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│  ┌──────────────────────────┴──────────────────────────┐  │
│  │              MULTI-PROVIDER API LAYER                │  │
│  │  Claude │ OpenAI │ Gemini │ Ollama │ Custom...       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Supported Providers

| Provider | Models | Status |
|----------|--------|--------|
| Anthropic | claude-opus, claude-sonnet, claude-haiku | ✅ |
| OpenAI | gpt-4, gpt-4-turbo, gpt-3.5-turbo | ✅ |
| Google | gemini-pro, gemini-ultra | ✅ |
| Ollama | Any local model | ✅ |
| Custom | OpenAI-compatible APIs | ✅ |

## License

MIT - See [LICENSE](LICENSE) for details. Based on [claw-code](https://github.com/instructkr/claw-code).
