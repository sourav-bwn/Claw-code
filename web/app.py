import os
import uuid
import json
from datetime import datetime
from typing import Optional, List, Dict, Any
from contextlib import asynccontextmanager

from fastapi import (
    FastAPI,
    HTTPException,
    WebSocket,
    WebSocketDisconnect,
    Query,
    Request,
)
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel, Field
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    host: str = "0.0.0.0"
    port: int = 8000
    debug: bool = False
    secret_key: str = "uc law-secret-change-in-production"

    class Config:
        env_prefix = "UCLAW_"


settings = Settings()

sessions: Dict[str, Dict[str, Any]] = {}
connections: Dict[str, WebSocket] = {}


class Message(BaseModel):
    role: str
    content: str


class ChatRequest(BaseModel):
    message: str
    session_id: Optional[str] = None
    model: Optional[str] = None
    system_prompt: Optional[str] = None


class ChatResponse(BaseModel):
    session_id: str
    message: str
    model: str
    usage: Optional[Dict[str, int]] = None


class SessionInfo(BaseModel):
    id: str
    created_at: str
    model: str
    message_count: int


@asynccontextmanager
async def lifespan(app: FastAPI):
    print("Uclaw Web UI starting...")
    yield
    print("Uclaw Web UI shutting down...")


app = FastAPI(
    title="Uclaw - Unified AI Platform",
    description="Web interface for the Uclaw AI coding assistant",
    version="0.1.0",
    lifespan=lifespan,
)

templates = Jinja2Templates(directory="templates")


@app.get("/", response_class=HTMLResponse)
async def home():
    with open("static/index.html", "r") as f:
        return HTMLResponse(content=f.read())


@app.get("/api/sessions", response_model=List[SessionInfo])
async def list_sessions():
    return [
        SessionInfo(
            id=sid,
            created_at=info["created_at"],
            model=info["model"],
            message_count=len(info.get("messages", [])),
        )
        for sid, info in sessions.items()
    ]


@app.post("/api/sessions", response_model=SessionInfo)
async def create_session(model: Optional[str] = Query(default="claude-sonnet-4-6")):
    session_id = str(uuid.uuid4())
    sessions[session_id] = {
        "id": session_id,
        "created_at": datetime.now().isoformat(),
        "model": model,
        "messages": [],
        "system_prompt": None,
    }
    return SessionInfo(
        id=session_id,
        created_at=sessions[session_id]["created_at"],
        model=model,
        message_count=0,
    )


@app.delete("/api/sessions/{session_id}")
async def delete_session(session_id: str):
    if session_id not in sessions:
        raise HTTPException(status_code=404, detail="Session not found")
    del sessions[session_id]
    return {"status": "ok"}


@app.get("/api/sessions/{session_id}/messages")
async def get_messages(session_id: str):
    if session_id not in sessions:
        raise HTTPException(status_code=404, detail="Session not found")
    return sessions[session_id].get("messages", [])


@app.post("/api/chat", response_model=ChatResponse)
async def chat(request: ChatRequest):
    session_id = request.session_id or str(uuid.uuid4())

    if session_id not in sessions:
        sessions[session_id] = {
            "id": session_id,
            "created_at": datetime.now().isoformat(),
            "model": request.model or "claude-sonnet-4-6",
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
        model=session["model"],
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
        model=session["model"],
        usage={"total_tokens": len(response_text) // 4},
    )


@app.websocket("/ws/chat/{session_id}")
async def websocket_chat(websocket: WebSocket, session_id: str):
    await websocket.accept()
    connections[session_id] = websocket

    if session_id not in sessions:
        sessions[session_id] = {
            "id": session_id,
            "created_at": datetime.now().isoformat(),
            "model": "claude-sonnet-4-6",
            "messages": [],
            "system_prompt": None,
        }

    try:
        while True:
            data = await websocket.receive_text()
            message_data = json.loads(data)

            if message_data.get("type") == "message":
                message = message_data.get("content", "")

                session = sessions[session_id]
                session["messages"].append(
                    {
                        "role": "user",
                        "content": message,
                        "timestamp": datetime.now().isoformat(),
                    }
                )

                await websocket.send_json(
                    {"type": "status", "status": "thinking", "message": "Thinking..."}
                )

                response_text = await generate_response(
                    messages=session["messages"],
                    model=session["model"],
                    system_prompt=session.get("system_prompt"),
                )

                session["messages"].append(
                    {
                        "role": "assistant",
                        "content": response_text,
                        "timestamp": datetime.now().isoformat(),
                    }
                )

                await websocket.send_json(
                    {
                        "type": "message",
                        "content": response_text,
                        "model": session["model"],
                    }
                )

            elif message_data.get("type") == "clear":
                sessions[session_id]["messages"] = []
                await websocket.send_json(
                    {
                        "type": "status",
                        "status": "cleared",
                        "message": "Conversation cleared",
                    }
                )

    except WebSocketDisconnect:
        if session_id in connections:
            del connections[session_id]


async def generate_response(
    messages: List[Dict[str, Any]], model: str, system_prompt: Optional[str] = None
) -> str:
    api_key = os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("OPENAI_API_KEY")

    if not api_key:
        return """I'm Uclaw, your AI assistant! 

To enable full AI responses, please set one of these environment variables:
- ANTHROPIC_API_KEY (for Claude)
- OPENAI_API_KEY (for GPT-4)

For now, I can help you with:
- Viewing your conversation history
- Exploring the Uclaw platform features
- Understanding how to configure AI providers

Try setting an API key and sending another message!"""

    import httpx

    if "claude" in model.lower():
        return await generate_claude_response(messages, model, api_key, system_prompt)
    elif "gpt" in model.lower() or "openai" in model.lower():
        return await generate_openai_response(messages, model, api_key, system_prompt)
    else:
        return (
            f"Model '{model}' is not yet supported. Please use Claude or GPT-4 models."
        )


async def generate_claude_response(
    messages: List[Dict[str, Any]],
    model: str,
    api_key: str,
    system_prompt: Optional[str] = None,
) -> str:
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

        if response.status_code == 401:
            return "Invalid API key. Please check your ANTHROPIC_API_KEY."

        if response.status_code != 200:
            return f"Error: {response.status_code} - {response.text}"

        data = response.json()
        return data["content"][0]["text"]


async def generate_openai_response(
    messages: List[Dict[str, Any]],
    model: str,
    api_key: str,
    system_prompt: Optional[str] = None,
) -> str:
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

        if response.status_code == 401:
            return "Invalid API key. Please check your OPENAI_API_KEY."

        if response.status_code != 200:
            return f"Error: {response.status_code} - {response.text}"

        data = response.json()
        return data["choices"][0]["message"]["content"]


@app.get("/api/models")
async def list_models():
    return [
        {"id": "claude-opus-4-6", "name": "Claude Opus 4", "provider": "Anthropic"},
        {"id": "claude-sonnet-4-6", "name": "Claude Sonnet 4", "provider": "Anthropic"},
        {
            "id": "claude-haiku-4-5-20251213",
            "name": "Claude Haiku 4",
            "provider": "Anthropic",
        },
        {"id": "gpt-4o", "name": "GPT-4o", "provider": "OpenAI"},
        {"id": "gpt-4o-mini", "name": "GPT-4o Mini", "provider": "OpenAI"},
        {"id": "gemini-2.0-flash", "name": "Gemini 2.0 Flash", "provider": "Google"},
    ]


@app.get("/health")
async def health():
    return {"status": "healthy", "version": "0.1.0"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host=settings.host, port=settings.port)
