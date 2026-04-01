import os
import uuid
import json
import httpx
from datetime import datetime
from typing import Optional, List, Dict, Any
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect, Query
from fastapi.responses import HTMLResponse
from pydantic import BaseModel


class Settings:
    host: str = "0.0.0.0"
    port: int = 8000


settings = Settings()
sessions: Dict[str, Dict[str, Any]] = {}


class ChatRequest(BaseModel):
    message: str
    session_id: Optional[str] = None
    model: Optional[str] = None
    provider: Optional[str] = "ollama"
    system_prompt: Optional[str] = None


class ChatResponse(BaseModel):
    session_id: str
    message: str
    model: str
    provider: str
    usage: Optional[Dict[str, int]] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    print("🤖 Uclaw Web UI starting...")
    print("💡 Default provider: Ollama (FREE, no API key needed!)")
    yield
    print("👋 Uclaw Web UI shutting down...")


app = FastAPI(
    title="Uclaw - Unified AI Platform",
    description="Free AI coding assistant - no API key needed!",
    version="0.1.0",
    lifespan=lifespan,
)


@app.get("/", response_class=HTMLResponse)
async def home():
    with open("static/index.html", "r") as f:
        return HTMLResponse(content=f.read())


@app.get("/api/sessions")
async def list_sessions():
    return [
        {
            "id": sid,
            "created_at": info["created_at"],
            "model": info["model"],
            "provider": info.get("provider", "ollama"),
            "message_count": len(info.get("messages", [])),
        }
        for sid, info in sessions.items()
    ]


@app.post("/api/sessions")
async def create_session(
    model: Optional[str] = "llama3.2", provider: Optional[str] = "ollama"
):
    session_id = str(uuid.uuid4())
    sessions[session_id] = {
        "id": session_id,
        "created_at": datetime.now().isoformat(),
        "model": model,
        "provider": provider,
        "messages": [],
        "system_prompt": None,
    }
    return {
        "id": session_id,
        "model": model,
        "provider": provider,
        "created_at": datetime.now().isoformat(),
        "message_count": 0,
    }


@app.delete("/api/sessions/{session_id}")
async def delete_session(session_id: str):
    if session_id in sessions:
        del sessions[session_id]
    return {"status": "ok"}


@app.post("/api/chat")
async def chat(request: ChatRequest):
    session_id = request.session_id or str(uuid.uuid4())
    provider = request.provider or "ollama"
    model = request.model or "llama3.2"

    if session_id not in sessions:
        sessions[session_id] = {
            "id": session_id,
            "created_at": datetime.now().isoformat(),
            "model": model,
            "provider": provider,
            "messages": [],
            "system_prompt": request.system_prompt,
        }

    session = sessions[session_id]
    session["messages"].append(
        {
            "role": "user",
            "content": request.message,
            "timestamp": datetime.now().isoformat(),
        }
    )

    response_text = await generate_response(
        messages=session["messages"],
        model=model,
        provider=provider,
        system_prompt=session.get("system_prompt"),
    )

    session["messages"].append(
        {
            "role": "assistant",
            "content": response_text,
            "timestamp": datetime.now().isoformat(),
        }
    )

    return ChatResponse(
        session_id=session_id,
        message=response_text,
        model=model,
        provider=provider,
        usage={"total_tokens": len(response_text) // 4},
    )


async def generate_response(
    messages: List[Dict[str, Any]],
    model: str,
    provider: str = "ollama",
    system_prompt: Optional[str] = None,
) -> str:
    if provider == "ollama" or provider == "local":
        return await generate_ollama_response(messages, model)
    elif provider == "anthropic":
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if api_key:
            return await generate_claude_response(
                messages, model, api_key, system_prompt
            )
        return "⚠️ Set ANTHROPIC_API_KEY to use Claude"
    elif provider == "openai":
        api_key = os.environ.get("OPENAI_API_KEY")
        if api_key:
            return await generate_openai_response(
                messages, model, api_key, system_prompt
            )
        return "⚠️ Set OPENAI_API_KEY to use GPT-4"
    elif provider == "gemini":
        api_key = os.environ.get("GEMINI_API_KEY")
        if api_key:
            return await generate_gemini_response(messages, model, api_key)
        return "⚠️ Set GEMINI_API_KEY to use Gemini"
    else:
        return await generate_ollama_response(messages, model)


async def generate_ollama_response(messages: List[Dict[str, Any]], model: str) -> str:
    try:
        async with httpx.AsyncClient(timeout=120.0) as client:
            ollama_messages = []
            for msg in messages:
                if msg["role"] == "system":
                    continue
                ollama_messages.append({"role": msg["role"], "content": msg["content"]})

            response = await client.post(
                "http://localhost:11434/api/chat",
                json={"model": model, "messages": ollama_messages, "stream": False},
            )

            if response.status_code == 200:
                data = response.json()
                return data.get("message", {}).get("content", "No response")
            elif response.status_code == 404:
                return f"""⚠️ Ollama model '{model}' not found!

To install models:
1. Install Ollama: curl -fsSL https://ollama.ai | sh
2. Pull a model: ollama pull llama3.2
3. Restart Uclaw

Available free models:
• llama3.2 - General purpose
• llama3.2:70b - Larger model
• codellama - Code specialized
• mistral - Fast & efficient
• phi - Small but capable"""
            else:
                return f"⚠️ Ollama error: {response.status_code}"

    except httpx.ConnectError:
        return """🤖 Ollama is not running!

To use FREE AI (no API key needed):

1. Install Ollama:
   curl -fsSL https://ollama.ai | sh

2. Pull a model:
   ollama pull llama3.2

3. Start Ollama:
   ollama serve

4. Refresh this page!

After that, you can chat for FREE with no limits!"""


async def generate_claude_response(messages, model, api_key, system_prompt=None):
    async with httpx.AsyncClient(timeout=60.0) as client:
        anthropic_messages = []
        for msg in messages:
            if msg["role"] == "system":
                continue
            anthropic_messages.append(
                {
                    "role": "user" if msg["role"] == "user" else "assistant",
                    "content": msg["content"],
                }
            )

        payload = {"model": model, "messages": anthropic_messages, "max_tokens": 4096}
        if system_prompt:
            payload["system"] = system_prompt

        response = await client.post(
            "https://api.anthropic.com/v1/messages",
            headers={
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
                "content-type": "application/json",
            },
            json=payload,
        )

        if response.status_code == 200:
            data = response.json()
            return data["content"][0]["text"]
        return f"⚠️ Claude error: {response.status_code}"


async def generate_openai_response(messages, model, api_key, system_prompt=None):
    async with httpx.AsyncClient(timeout=60.0) as client:
        openai_messages = []
        if system_prompt:
            openai_messages.append({"role": "system", "content": system_prompt})

        for msg in messages:
            if msg["role"] == "system":
                continue
            openai_messages.append({"role": msg["role"], "content": msg["content"]})

        payload = {"model": model, "messages": openai_messages, "max_tokens": 4096}

        response = await client.post(
            "https://api.openai.com/v1/chat/completions",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            json=payload,
        )

        if response.status_code == 200:
            data = response.json()
            return data["choices"][0]["message"]["content"]
        return f"⚠️ OpenAI error: {response.status_code}"


async def generate_gemini_response(messages, model, api_key):
    async with httpx.AsyncClient(timeout=60.0) as client:
        contents = []
        for msg in messages:
            contents.append(
                {
                    "role": "user" if msg["role"] == "user" else "model",
                    "parts": [{"text": msg["content"]}],
                }
            )

        response = await client.post(
            f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent",
            params={"key": api_key},
            json={"contents": contents, "generationConfig": {"maxOutputTokens": 4096}},
        )

        if response.status_code == 200:
            data = response.json()
            return data["candidates"][0]["content"]["parts"][0]["text"]
        return f"⚠️ Gemini error: {response.status_code}"


@app.get("/api/models")
async def list_models():
    return [
        {
            "id": "llama3.2",
            "name": "Llama 3.2",
            "provider": "Ollama (FREE)",
            "requires_key": False,
        },
        {
            "id": "llama3.2:70b",
            "name": "Llama 3.2 70B",
            "provider": "Ollama (FREE)",
            "requires_key": False,
        },
        {
            "id": "codellama",
            "name": "Code Llama",
            "provider": "Ollama (FREE)",
            "requires_key": False,
        },
        {
            "id": "mistral",
            "name": "Mistral",
            "provider": "Ollama (FREE)",
            "requires_key": False,
        },
        {
            "id": "claude-sonnet-4-6",
            "name": "Claude Sonnet 4",
            "provider": "Anthropic",
            "requires_key": True,
        },
        {
            "id": "gpt-4o-mini",
            "name": "GPT-4o Mini",
            "provider": "OpenAI",
            "requires_key": True,
        },
        {
            "id": "gemini-2.0-flash",
            "name": "Gemini 2.0 Flash",
            "provider": "Google",
            "requires_key": True,
        },
    ]


@app.get("/health")
async def health():
    return {"status": "healthy", "version": "0.1.0", "default": "ollama (FREE)"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host=settings.host, port=settings.port)
