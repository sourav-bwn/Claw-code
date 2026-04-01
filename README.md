# 🐾 Uclaw - 100% Free AI Assistant

> Your personal AI that works in any browser - **NO API KEY NEEDED!**

[![License: MIT](https://img.shields.io/badge/License-MIT-purple.svg)](https://opensource.org/licenses/MIT)

## ✨ Features

- 💬 **Chat** - Ask anything, get instant answers
- ✍️ **Write** - Essays, emails, code, stories
- 🔍 **Research** - Understand complex topics
- 💡 **Brainstorm** - Generate creative ideas
- 🔧 **Code** - Write, debug, and explain code

## 🚀 Quick Start (2 Minutes)

### Step 1: Install Ollama (Free AI)

Go to **[ollama.com](https://ollama.com)** and click **Download**

### Step 2: Open Terminal & Run

```bash
ollama pull llama3.2
ollama serve
```

### Step 3: Start Chatting!

Open the `index.html` file in your browser and start chatting!

---

## 📱 Use Anywhere

| Platform | How to Use |
|----------|------------|
| **Desktop (Mac/Windows/Linux)** | Open `index.html` in browser |
| **Android (Termux)** | Install Ollama in Termux |
| **Server** | Run `python web/app.py` |
| **Host Online** | Deploy to Railway/Render |

## 🎯 Quick Links

- **Live Demo**: [Open index.html](index.html)
- **Web UI**: Run `cd web && python app.py` then visit http://localhost:8000
- **CLI**: Run `python python/cli.py`

## 🖥️ Web UI Preview

![Uclaw Web UI](https://img.shields.io/badge/Web-UI-blue)

The web interface includes:
- Beautiful dark theme
- Real-time chat
- Session history
- Multiple AI providers support
- Mobile responsive

## 💻 CLI Usage

```bash
# Interactive chat
python python/cli.py

# Single question
python python/cli.py "Hello, how are you?"

# Use a different model
python python/cli.py --model codellama
```

## 🌐 Available AI Providers

| Provider | Cost | API Key Required |
|----------|------|------------------|
| **Ollama** (Default) | FREE | ❌ No |
| Claude | $ | ✅ Yes |
| GPT-4 | $ | ✅ Yes |
| Gemini | Free tier | ✅ Optional |

## 🛠️ Development

### Local Setup

```bash
# Clone the repo
git clone https://github.com/KING-SS-OFFICIAL/Claw-code.git
cd Claw-code

# Run web UI
cd web
pip install -r requirements.txt
python app.py

# Or run CLI
cd python
pip install httpx
python cli.py
```

### Docker

```bash
cd docker
docker-compose up
```

## 📂 Project Structure

```
uc law/
├── index.html          # Simple HTML chat (open in browser!)
├── web/               # Web UI (FastAPI)
│   ├── app.py         # Server
│   └── static/        # Frontend
├── python/            # Python CLI
│   └── cli.py         # Command line interface
├── rust/              # Rust implementation
│   └── crates/        # Modular components
├── mcp/               # MCP Server (plugins)
├── vscode/            # VS Code extension
├── docker/            # Docker configs
└── github-actions/    # CI/CD workflows
```

## 🤝 Contributing

1. Fork the repo
2. Create your branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing`)
5. Open a Pull Request

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

Based on [claw-code](https://github.com/instructkr/claw-code) by @sigridjineth

## 🙏 Credits

- Original Claude Code: Anthropic
- Llama models: Meta AI
- Ollama: Local AI runtime
- Contributors: Open source community

---

<p align="center">
  Made with ❤️ | 100% Free | Open Source
</p>
