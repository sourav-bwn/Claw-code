#!/usr/bin/env python3
"""
Uclaw CLI - Unified AI Coding Assistant
Supports local models (Ollama), Anthropic, OpenAI, and Gemini APIs
"""

import os
import sys
import json
import asyncio
import argparse
from typing import Optional, List, Dict
import subprocess

try:
    import anthropic

    ANTHROPIC_AVAILABLE = True
except ImportError:
    ANTHROPIC_AVAILABLE = False

try:
    import openai

    OPENAI_AVAILABLE = True
except ImportError:
    OPENAI_AVAILABLE = False


class UclawCLI:
    def __init__(self, provider: str = "ollama", model: Optional[str] = None):
        self.provider = provider
        self.model = model or self._get_default_model()
        self.messages: List[Dict[str, str]] = []
        self.anthropic_client = None
        self.openai_client = None

        self._init_clients()

    def _get_default_model(self) -> str:
        if self.provider == "ollama":
            return "llama3.2"
        elif self.provider == "anthropic":
            return "claude-sonnet-4-6"
        elif self.provider == "openai":
            return "gpt-4o-mini"
        elif self.provider == "gemini":
            return "gemini-2.0-flash"
        return "llama3.2"

    def _init_clients(self):
        if self.provider == "anthropic":
            api_key = os.environ.get("ANTHROPIC_API_KEY")
            if api_key and ANTHROPIC_AVAILABLE:
                self.anthropic_client = anthropic.Anthropic(api_key=api_key)
            else:
                print("⚠️ Anthropic not available. Install: pip install anthropic")

        elif self.provider == "openai":
            api_key = os.environ.get("OPENAI_API_KEY")
            if api_key and OPENAI_AVAILABLE:
                self.openai_client = openai.OpenAI(api_key=api_key)
            else:
                print("⚠️ OpenAI not available. Install: pip install openai")

    def check_ollama(self) -> bool:
        """Check if Ollama is running"""
        try:
            result = subprocess.run(
                ["curl", "-s", "http://localhost:11434/api/tags"],
                capture_output=True,
                timeout=5,
            )
            return result.returncode == 0
        except:
            return False

    def install_ollama_model(self, model: str) -> bool:
        """Install an Ollama model"""
        print(f"📥 Installing Ollama model: {model}...")
        try:
            result = subprocess.run(
                ["ollama", "pull", model], capture_output=True, text=True
            )
            return result.returncode == 0
        except Exception as e:
            print(f"Error: {e}")
            return False

    async def chat(self, message: str, stream: bool = True) -> str:
        self.messages.append({"role": "user", "content": message})

        if self.provider == "ollama":
            return await self._chat_ollama(stream)
        elif self.provider == "anthropic" and self.anthropic_client:
            return await self._chat_anthropic(stream)
        elif self.provider == "openai" and self.openai_client:
            return await self._chat_openai(stream)
        elif self.provider == "gemini":
            return await self._chat_gemini(stream)
        else:
            return "Error: No provider available. Use 'ollama' (free, local) or set API keys."

    async def _chat_ollama(self, stream: bool) -> str:
        if not self.check_ollama():
            return """Ollama is not running. 

To use free local AI:
1. Install Ollama: https://ollama.ai
2. Run: ollama serve
3. Run: ollama pull llama3.2

Then restart Uclaw!"""

        import httpx

        ollama_messages = []
        for m in self.messages:
            ollama_messages.append({"role": m["role"], "content": m["content"]})

        try:
            async with httpx.AsyncClient(timeout=120.0) as client:
                response = await client.post(
                    "http://localhost:11434/api/chat",
                    json={
                        "model": self.model,
                        "messages": ollama_messages,
                        "stream": stream,
                    },
                )

                if response.status_code != 200:
                    return f"Error: {response.status_code}"

                data = response.json()
                assistant_message = data["message"]["content"]
                self.messages.append(
                    {"role": "assistant", "content": assistant_message}
                )
                return assistant_message
        except Exception as e:
            return f"Error connecting to Ollama: {e}"

    async def _chat_anthropic(self, stream: bool) -> str:
        anthropic_messages = [
            {
                "role": "user" if m["role"] == "user" else "assistant",
                "content": m["content"],
            }
            for m in self.messages
        ]

        message = self.anthropic_client.messages.create(
            model=self.model, messages=anthropic_messages, max_tokens=4096
        )

        assistant_message = message.content[0].text
        self.messages.append({"role": "assistant", "content": assistant_message})
        return assistant_message

    async def _chat_openai(self, stream: bool) -> str:
        response = self.openai_client.chat.completions.create(
            model=self.model, messages=self.messages, stream=False
        )

        assistant_message = response.choices[0].message.content
        self.messages.append({"role": "assistant", "content": assistant_message})
        return assistant_message

    async def _chat_gemini(self, stream: bool) -> str:
        import httpx

        contents = []
        for m in self.messages:
            contents.append(
                {
                    "role": "user" if m["role"] == "user" else "model",
                    "parts": [{"text": m["content"]}],
                }
            )

        api_key = os.environ.get("GEMINI_API_KEY", "")

        async with httpx.AsyncClient() as client:
            response = await client.post(
                f"https://generativelanguage.googleapis.com/v1beta/models/{self.model}:generateContent",
                params={"key": api_key},
                json={
                    "contents": contents,
                    "generationConfig": {"maxOutputTokens": 4096},
                },
            )

        if response.status_code == 200:
            data = response.json()
            assistant_message = data["candidates"][0]["content"]["parts"][0]["text"]
            self.messages.append({"role": "assistant", "content": assistant_message})
            return assistant_message
        else:
            return f"Error: {response.status_code}"

    def clear(self):
        self.messages = []

    def list_models(self):
        if self.provider == "ollama" and self.check_ollama():
            import httpx

            response = httpx.get("http://localhost:11434/api/tags")
            models = response.json().get("models", [])
            print("\n📦 Installed Ollama Models:")
            for m in models:
                print(f"  • {m.get('name', 'Unknown')}")
        else:
            print("\n📦 Available Models:")
            print("  [Ollama - Free, Local]")
            print("    • llama3.2 - General purpose")
            print("    • llama3.2:70b - Larger, more capable")
            print("    • codellama:7b - Code specialized")
            print("    • mistral - Fast & efficient")
            print("    • phi - Small but capable")
            print("  [Cloud - Requires API Key]")
            print("    • claude-sonnet-4-6 - Anthropic (set ANTHROPIC_API_KEY)")
            print("    • gpt-4o-mini - OpenAI (set OPENAI_API_KEY)")
            print("    • gemini-2.0-flash - Google (set GEMINI_API_KEY)")


def interactive_mode(cli: UclawCLI):
    print("=" * 60)
    print("🔓 Uclaw CLI - Unified AI Platform")
    print(f"   Provider: {cli.provider} | Model: {cli.model}")
    print("=" * 60)

    if cli.provider == "ollama":
        print("\n💡 Using FREE local AI (Ollama)")
        print("   No API key needed!")
        if not cli.check_ollama():
            print("\n⚠️  Ollama not running!")
            print("   Install: curl -fsSL https://ollama.ai | sh")
            print("   Start:   ollama serve")
            print("   Models:  ollama pull llama3.2")

    print("\nType 'exit' to quit, 'help' for commands")
    print()

    while True:
        try:
            user_input = input("\nYou: ").strip()

            if not user_input:
                continue

            cmd = user_input.lower()

            if cmd in ["exit", "quit", "q"]:
                print("Goodbye!")
                break

            elif cmd == "clear":
                cli.clear()
                print("Conversation cleared.")
                continue

            elif cmd == "models":
                cli.list_models()
                continue

            elif cmd == "help":
                print("""
Commands:
  exit, quit    - Exit
  clear         - Clear conversation
  models        - List available models
  provider <p>  - Switch provider (ollama/anthropic/openai/gemini)
  model <m>     - Switch model
  help          - Show this help
                """)
                continue

            elif cmd.startswith("provider "):
                new_provider = user_input[9:].strip()
                cli.provider = new_provider
                cli._init_clients()
                cli.model = cli._get_default_model()
                print(f"Provider: {cli.provider}, Model: {cli.model}")
                continue

            elif cmd.startswith("model "):
                cli.model = user_input[6:].strip()
                print(f"Model: {cli.model}")
                continue

            print("\nUclaw: ", end="", flush=True)
            response = asyncio.run(cli.chat(user_input))
            print(response)

        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"\nError: {e}")


def main():
    parser = argparse.ArgumentParser(description="Uclaw - Unified AI Platform")
    parser.add_argument(
        "--provider",
        "-p",
        default="ollama",
        choices=["ollama", "anthropic", "openai", "gemini"],
        help="AI provider (default: ollama - free!)",
    )
    parser.add_argument("--model", "-m", help="Model to use")
    parser.add_argument("prompt", nargs="*", help="Prompt (if not interactive)")
    args = parser.parse_args()

    cli = UclawCLI(provider=args.provider, model=args.model)

    if args.prompt:
        prompt = " ".join(args.prompt)
        print(f"You: {prompt}\n")
        response = asyncio.run(cli.chat(prompt, stream=False))
        print(f"Uclaw: {response}")
    else:
        interactive_mode(cli)


if __name__ == "__main__":
    main()
