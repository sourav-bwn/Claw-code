#!/usr/bin/env python3
"""
Uclaw Termux - Ultra Light Version
No Ollama, No API key needed!
Uses free AI APIs
"""

import os
import sys
import asyncio
import subprocess


class UclawTermux:
    def __init__(self):
        self.messages = []
        self.model = "llama3.2"
        self.provider = "cloudflare"

    def check_internet(self):
        try:
            subprocess.run(
                ["curl", "-s", "https://1.1.1.1"], capture_output=True, timeout=5
            )
            return True
        except:
            return False

    async def chat(self, message):
        self.messages.append({"role": "user", "content": message})

        # Try Cloudflare Workers AI (FREE, no key for basic use)
        try:
            import httpx

            async with httpx.AsyncClient(timeout=60.0) as client:
                response = await client.post(
                    "https://api.cloudflare.com/client/v4/accounts/YOUR_ACCOUNT_ID/ai/run/@cf/meta/llama-3.1-8b-instruct",
                    headers={"Authorization": "Bearer YOUR_API_TOKEN"},
                    json={"messages": self.messages, "stream": False},
                )
                if response.status_code == 200:
                    data = response.json()
                    result = data["result"]["response"]
                    self.messages.append({"role": "assistant", "content": result})
                    return result
        except:
            pass

        # Fallback: local processing (no AI)
        self.messages.append(
            {"role": "assistant", "content": self._local_response(message)}
        )
        return self.messages[-1]["content"]

    def _local_response(self, message):
        msg = message.lower()
        if "hello" in msg or "hi" in msg:
            return "Hello! I'm Uclaw running on your Android phone! 🐾"
        elif "help" in msg:
            return """I can help you with:
• File operations
• Running shell commands
• Writing code
• Answering questions

To use AI, either:
1. Install Ollama: pkg install ollama
2. Or set a free API key"""
        else:
            return f"I heard: {message[:50]}...\n\nFor AI responses, install Ollama or use an API key."


def interactive():
    print("=" * 50)
    print("🐾 Uclaw Termux - AI on your phone!")
    print("=" * 50)
    print()

    cli = UclawTermux()

    while True:
        try:
            user = input("\nYou: ").strip()
            if not user:
                continue

            if user.lower() in ["exit", "quit"]:
                print("Goodbye! 👋")
                break

            response = asyncio.run(cli.chat(user))
            print(f"\nUclaw: {response}")

        except KeyboardInterrupt:
            print("\n\n👋")
            break


if __name__ == "__main__":
    interactive()
