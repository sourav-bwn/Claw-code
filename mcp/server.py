#!/usr/bin/env python3
"""
Uclaw MCP Server - Model Context Protocol implementation
Allows Uclaw to connect to external tools and services
"""

import json
import sys
import asyncio
from typing import Any, Dict, List, Optional
from dataclasses import dataclass, asdict
from abc import ABC, abstractmethod

VERSION = "1.0.0"


@dataclass
class MCPRequest:
    jsonrpc: str = "2.0"
    id: Optional[Any] = None
    method: str = ""
    params: Optional[Dict] = None


@dataclass
class MCPResponse:
    jsonrpc: str = "2.0"
    id: Optional[Any] = None
    result: Optional[Any] = None
    error: Optional[Dict] = None


@dataclass
class ToolDefinition:
    name: str
    description: str
    input_schema: Dict


@dataclass
class Resource:
    uri: str
    name: str
    description: Optional[str] = None
    mime_type: Optional[str] = None


class MCPTool(ABC):
    @abstractmethod
    def get_definition(self) -> ToolDefinition:
        pass

    @abstractmethod
    async def execute(self, params: Dict) -> Dict:
        pass


class MCPResourceProvider(ABC):
    @abstractmethod
    def get_resources(self) -> List[Resource]:
        pass

    @abstractmethod
    async def read_resource(self, uri: str) -> str:
        pass


class UclawMCPServer:
    def __init__(self):
        self.tools: Dict[str, MCPTool] = {}
        self.resource_providers: List[MCPResourceProvider] = []
        self.capabilities = {
            "tools": True,
            "resources": True,
            "prompts": True,
        }

    def register_tool(self, tool: MCPTool):
        self.tools[tool.get_definition().name] = tool

    def register_resource_provider(self, provider: MCPResourceProvider):
        self.resource_providers.append(provider)

    async def handle_request(self, request: MCPRequest) -> MCPResponse:
        try:
            if request.method == "initialize":
                return MCPResponse(
                    id=request.id,
                    result={
                        "protocolVersion": "2024-11-05",
                        "capabilities": self.capabilities,
                        "serverInfo": {"name": "uc law-mcp", "version": VERSION},
                    },
                )

            elif request.method == "tools/list":
                tools = [tool.get_definition() for tool in self.tools.values()]
                return MCPResponse(
                    id=request.id, result={"tools": [asdict(t) for t in tools]}
                )

            elif request.method == "tools/call":
                tool_name = request.params.get("name")
                params = request.params.get("arguments", {})

                if tool_name not in self.tools:
                    return MCPResponse(
                        id=request.id,
                        error={
                            "code": -32602,
                            "message": f"Tool not found: {tool_name}",
                        },
                    )

                result = await self.tools[tool_name].execute(params)
                return MCPResponse(
                    id=request.id,
                    result={"content": [{"type": "text", "text": json.dumps(result)}]},
                )

            elif request.method == "resources/list":
                resources = []
                for provider in self.resource_providers:
                    resources.extend(provider.get_resources())
                return MCPResponse(
                    id=request.id, result={"resources": [asdict(r) for r in resources]}
                )

            elif request.method == "resources/read":
                uri = request.params.get("uri")
                for provider in self.resource_providers:
                    for resource in provider.get_resources():
                        if resource.uri == uri:
                            content = await provider.read_resource(uri)
                            return MCPResponse(
                                id=request.id,
                                result={
                                    "contents": [
                                        {
                                            "uri": uri,
                                            "mimeType": resource.mime_type,
                                            "text": content,
                                        }
                                    ]
                                },
                            )
                return MCPResponse(
                    id=request.id,
                    error={"code": -32602, "message": f"Resource not found: {uri}"},
                )

            elif request.method == "prompts/list":
                return MCPResponse(id=request.id, result={"prompts": []})

            elif request.method == "ping":
                return MCPResponse(id=request.id, result={})

            else:
                return MCPResponse(
                    id=request.id,
                    error={
                        "code": -32601,
                        "message": f"Method not found: {request.method}",
                    },
                )

        except Exception as e:
            return MCPResponse(id=request.id, error={"code": -32603, "message": str(e)})

    async def read_message(self) -> Optional[MCPRequest]:
        line = sys.stdin.readline()
        if not line:
            return None

        try:
            data = json.loads(line.strip())
            return MCPRequest(
                jsonrpc=data.get("jsonrpc", "2.0"),
                id=data.get("id"),
                method=data.get("method", ""),
                params=data.get("params"),
            )
        except json.JSONDecodeError:
            return None

    async def write_message(self, response: MCPResponse):
        print(json.dumps(asdict(response)), flush=True)

    async def run(self):
        initialized = False

        while True:
            request = await self.read_message()
            if request is None:
                break

            if not initialized and request.method != "initialize":
                continue

            response = await self.handle_request(request)
            await self.write_message(response)

            if request.method == "initialize":
                initialized = True


class FileSystemTool(MCPTool):
    def get_definition(self) -> ToolDefinition:
        return ToolDefinition(
            name="filesystem",
            description="Read or write files in the workspace",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["read", "write", "list"]},
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                },
                "required": ["action", "path"],
            },
        )

    async def execute(self, params: Dict) -> Dict:
        action = params.get("action")
        path = params.get("path")

        if action == "read":
            try:
                with open(path, "r") as f:
                    return {"success": True, "content": f.read()}
            except Exception as e:
                return {"success": False, "error": str(e)}

        elif action == "write":
            try:
                with open(path, "w") as f:
                    f.write(params.get("content", ""))
                return {"success": True}
            except Exception as e:
                return {"success": False, "error": str(e)}

        elif action == "list":
            import os

            try:
                items = os.listdir(path)
                return {"success": True, "files": items}
            except Exception as e:
                return {"success": False, "error": str(e)}

        return {"success": False, "error": "Invalid action"}


class GitTool(MCPTool):
    def get_definition(self) -> ToolDefinition:
        return ToolDefinition(
            name="git",
            description="Git operations",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "diff", "log", "branch"],
                    },
                },
                "required": ["action"],
            },
        )

    async def execute(self, params: Dict) -> Dict:
        import subprocess

        action = params.get("action")

        commands = {
            "status": ["git", "status", "--porcelain"],
            "diff": ["git", "diff", "--stat"],
            "log": ["git", "log", "--oneline", "-10"],
            "branch": ["git", "branch", "-a"],
        }

        if action not in commands:
            return {"success": False, "error": "Invalid action"}

        try:
            result = subprocess.run(commands[action], capture_output=True, text=True)
            return {
                "success": True,
                "output": result.stdout,
                "error": result.stderr if result.returncode != 0 else None,
            }
        except Exception as e:
            return {"success": False, "error": str(e)}


class SearchTool(MCPTool):
    def get_definition(self) -> ToolDefinition:
        return ToolDefinition(
            name="search",
            description="Search for patterns in code",
            input_schema={
                "type": "object",
                "properties": {
                    "pattern": {"type": "string"},
                    "path": {"type": "string", "default": "."},
                    "file_pattern": {"type": "string"},
                },
                "required": ["pattern"],
            },
        )

    async def execute(self, params: Dict) -> Dict:
        import subprocess

        pattern = params.get("pattern")
        path = params.get("path", ".")

        cmd = ["grep", "-r", "-n", pattern, path]

        if params.get("file_pattern"):
            cmd.extend(["--include=" + params["file_pattern"]])

        try:
            result = subprocess.run(cmd, capture_output=True, text=True)
            lines = result.stdout.strip().split("\n") if result.stdout.strip() else []
            return {"success": True, "matches": lines, "count": len(lines)}
        except Exception as e:
            return {"success": False, "error": str(e)}


async def main():
    server = UclawMCPServer()

    server.register_tool(FileSystemTool())
    server.register_tool(GitTool())
    server.register_tool(SearchTool())

    await server.run()


if __name__ == "__main__":
    asyncio.run(main())
