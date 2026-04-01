#!/usr/bin/env python3
"""
Uclaw CLI - Unified AI Coding Assistant
Supports Anthropic, OpenAI, and Gemini APIs
"""

import os
import sys
import json
import asyncio
import argparse
from typing import Optional, List, Dict
import anthropic
import openai


class UclawCLI:
    def __init__(self, provider: str = "anthropic", model: Optional[str] = None):
        self.provider = provider
        self.model = model
        self.messages: List[Dict[str, str]] = []
        self.anthropic_client = None
        self.openai_client = None

        self._init_clients()

    def _init_clients(self):
        if self.provider == "anthropic":
            api_key = os.environ.get("ANTHROPIC_API_KEY")
            if api_key:
                self.anthropic_client = anthropic.Anthropic(api_key=api_key)
                self.model = self.model or "claude-sonnet-4-6"

        elif self.provider == "openai":
            api_key = os.environ.get("OPENAI_API_KEY")
            if api_key:
                self.openai_client = openai.OpenAI(api_key=api_key)
                self.model = self.model or "gpt-4o"

        elif self.provider == "gemini":
            api_key = os.environ.get("GEMINI_API_KEY")
            if api_key:
                # Gemini uses a different client
                self.gemini_api_key = api_key
                self.model = self.model or "gemini-2.0-flash"

    def _print_stream(self, text: str):
        print(text, end="", flush=True)

    async def chat(self, message: str, stream: bool = True) -> str:
        self.messages.append({"role": "user", "content": message})

        if self.anthropic_client:
            return await self._chat_anthropic(stream)
        elif self.openai_client:
            return await self._chat_openai(stream)
        elif hasattr(self, "gemini_api_key"):
            return await self._chat_gemini(stream)
        else:
            return "Error: No API client initialized. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or GEMINI_API_KEY"

    async def _chat_anthropic(self, stream: bool) -> str:
        anthropic_messages = [
            {
                "role": "user" if m["role"] == "user" else "assistant",
                "content": m["content"],
            }
            for m in self.messages
        ]

        with self.anthropic_client.messages.stream(
            model=self.model, messages=anthropic_messages, max_tokens=4096
        ) as stream:
            response = stream.get_full_message()

        assistant_message = response.content[0].text
        self.messages.append({"role": "assistant", "content": assistant_message})
        return assistant_message

    async def _chat_openai(self, stream: bool) -> str:
        response = self.openai_client.chat.completions.create(
            model=self.model, messages=self.messages, stream=stream
        )

        if stream:
            full_response = ""
            for chunk in response:
                if chunk.choices[0].delta.content:
                    self._print_stream(chunk.choices[0].delta.content)
                    full_response += chunk.choices[0].delta.content
            print()
            self.messages.append({"role": "assistant", "content": full_response})
            return full_response
        else:
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

        async with httpx.AsyncClient() as client:
            response = await client.post(
                f"https://generativelanguage.googleapis.com/v1beta/models/{self.model}:generateContent",
                params={"key": self.gemini_api_key},
                json={
                    "contents": contents,
                    "generationConfig": {"maxOutputTokens": 4096, "temperature": 0.9},
                },
                headers={"Content-Type": "application/json"},
            )

        if response.status_code == 200:
            data = response.json()
            assistant_message = data["candidates"][0]["content"]["parts"][0]["text"]
            self.messages.append({"role": "assistant", "content": assistant_message})
            return assistant_message
        else:
            return f"Error: {response.status_code} - {response.text}"

    def clear(self):
        self.messages = []

    def history(self):
        return self.messages

    def export(self, filename: str):
        with open(filename, "w") as f:
            json.dump(
                {
                    "provider": self.provider,
                    "model": self.model,
                    "messages": self.messages,
                },
                f,
                indent=2,
            )

    def import_history(self, filename: str):
        with open(filename, "r") as f:
            data = json.load(f)
            self.provider = data.get("provider", self.provider)
            self.model = data.get("model", self.model)
            self.messages = data.get("messages", [])
            self._init_clients()


def interactive_mode(cli: UclawCLI):
    print("=" * 60)
    print("Uclaw CLI - Unified AI Coding Assistant")
    print(f"Provider: {cli.provider} | Model: {cli.model}")
    print("Type 'exit' to quit, 'clear' to reset, 'help' for commands")
    print("=" * 60)
    print()

    while True:
        try:
            user_input = input("\nYou: ").strip()

            if not user_input:
                continue

            if user_input.lower() in ["exit", "quit", "q"]:
                print("Goodbye!")
                break

            elif user_input.lower() in ["clear", "reset"]:
                cli.clear()
                print("Conversation cleared.")
                continue

            elif user_input.lower() in ["help", "h", "?"]:
                print("""
Commands:
  exit, quit, q  - Exit the program
  clear, reset   - Clear conversation history
  history, hist   - Show conversation history
  export <file>   - Export conversation to JSON
  import <file>   - Import conversation from JSON
  model <name>    - Switch model
  provider <name> - Switch provider (anthropic/openai/gemini)
  help            - Show this help message
                """)
                continue

            elif user_input.lower().startswith("export "):
                filename = user_input[7:].strip()
                cli.export(filename)
                print(f"Exported to {filename}")
                continue

            elif user_input.lower().startswith("import "):
                filename = user_input[7:].strip()
                cli.import_history(filename)
                print(f"Imported from {filename}")
                continue

            elif user_input.lower().startswith("model "):
                cli.model = user_input[6:].strip()
                print(f"Model set to {cli.model}")
                continue

            elif user_input.lower().startswith("provider "):
                provider = user_input[9:].strip()
                cli.provider = provider
                cli._init_clients()
                print(f"Provider set to {cli.provider}")
                continue

            elif user_input.lower() in ["history", "hist"]:
                print("\nConversation History:")
                for i, m in enumerate(cli.history()):
                    print(f"  [{i}] {m['role']}: {m['content'][:100]}...")
                continue

            # Send message
            print("\nUclaw: ", end="", flush=True)
            response = asyncio.run(cli.chat(user_input))
            print(response)

        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"\nError: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="Uclaw - Unified AI Coding Assistant",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  uc law                           # Start interactive mode
  uc law --provider openai         # Use OpenAI
  uc law --model gpt-4o            # Use GPT-4o
  uc law "Explain this code"       # Single prompt mode
        """,
    )

    parser.add_argument("prompt", nargs="*", help="Prompt to send (if not interactive)")
    parser.add_argument(
        "--provider",
        "-p",
        choices=["anthropic", "openai", "gemini"],
        default=os.environ.get("UCLAW_PROVIDER", "anthropic"),
        help="AI provider to use",
    )
    parser.add_argument("--model", "-m", default=None, help="Model to use")
    parser.add_argument("--no-stream", action="store_true", help="Disable streaming")
    parser.add_argument("--export", "-e", help="Export conversation to file")
    parser.add_argument(
        "--import", "-i", dest="import_file", help="Import conversation from file"
    )

    args = parser.parse_args()

    cli = UclawCLI(provider=args.provider, model=args.model)

    if args.import_file:
        cli.import_history(args.import_file)
        print(f"Imported conversation from {args.import_file}")

    if args.prompt:
        # Single prompt mode
        prompt = " ".join(args.prompt)
        print(f"Prompt: {prompt}\n")
        response = asyncio.run(cli.chat(prompt, stream=not args.no_stream))
        if not args.no_stream:
            print(response)

        if args.export:
            cli.export(args.export)
            print(f"\nExported to {args.export}")
    else:
        # Interactive mode
        interactive_mode(cli)


if __name__ == "__main__":
    main()
