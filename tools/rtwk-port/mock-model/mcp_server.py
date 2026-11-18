"""A stdio MCP server for the `mcp-tool` scenarios, stdlib only.

Codex's own test server is `codex-rs/rmcp-client/src/bin/test_stdio_server.rs`
(`cargo build -p codex-rmcp-client --bin test_stdio_server`); building codex-rs is not
allowed here, so this is a small twin with the same tools and result shapes:
`echo` (test_stdio_server.rs:225, result `{"echo": message, "env": null}` as
structured content), `cwd` (:278), `image` (:375) and `image_scenario` (:412, the
scenarios `image_only`, `text_then_image`, `image_then_text`, `text_only`,
`multiple_valid_images`, with the same built-in 1x1 PNG).

Config (what core/tests/suite/rmcp_client.rs `insert_mcp_server` sets, as TOML):

    [mcp_servers.rmcp]
    command = "python3"
    args = ["-I", "tools/rtwk-port/mock-model/mcp_server.py"]

Newline-delimited JSON-RPC 2.0 on stdin/stdout (the MCP stdio transport).
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any

Json = dict[str, Any]

# test_stdio_server.rs:61 SMALL_PNG_BASE64
SMALL_PNG_BASE64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=="

ECHO_SCHEMA = {
    "type": "object",
    "properties": {"message": {"type": "string"}, "env_var": {"type": "string"}},
    "required": ["message"],
    "additionalProperties": False,
}
ECHO_OUTPUT = {
    "type": "object",
    "properties": {"echo": {"type": "string"}, "env": {"anyOf": [{"type": "string"}, {"type": "null"}]}},
    "required": ["echo", "env"],
    "additionalProperties": False,
}
SCENARIOS = [
    "image_only",
    "text_then_image",
    "image_then_text",
    "text_only",
    "multiple_valid_images",
]
TOOLS = [
    {
        "name": "echo",
        "description": "Echo back the provided message and include environment data.",
        "inputSchema": ECHO_SCHEMA,
        "outputSchema": ECHO_OUTPUT,
        "annotations": {"readOnlyHint": True},
    },
    {
        "name": "cwd",
        "description": "Return the server's working directory.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        "annotations": {"readOnlyHint": True},
    },
    {
        "name": "image",
        "description": "Return a single image content block.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        "annotations": {"readOnlyHint": True},
    },
    {
        "name": "image_scenario",
        "description": "Return content blocks for manual testing of MCP image rendering scenarios.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "scenario": {"type": "string", "enum": SCENARIOS},
                "caption": {"type": "string"},
            },
            "required": ["scenario"],
            "additionalProperties": False,
        },
        "annotations": {"readOnlyHint": True},
    },
]


def text(value: str) -> Json:
    return {"type": "text", "text": value}


def image() -> Json:
    return {"type": "image", "data": SMALL_PNG_BASE64, "mimeType": "image/png"}


def structured(value: Json) -> Json:
    return {"content": [text(json.dumps(value))], "structuredContent": value, "isError": False}


def call_tool(name: str, args: Json) -> Json:
    if name == "echo":
        return structured({"echo": f"ECHOING: {args.get('message', '')}", "env": os.environ.get(args.get("env_var") or "")})
    if name == "cwd":
        return structured({"cwd": os.getcwd()})
    if name == "image":
        return {"content": [image()], "isError": False}
    if name == "image_scenario":
        caption = args.get("caption") or "Here is the image:"
        content = {
            "image_only": [image()],
            "text_then_image": [text(caption), image()],
            "image_then_text": [image(), text(caption)],
            "text_only": [text(caption)],
            "multiple_valid_images": [image(), image()],
        }.get(args.get("scenario"))
        if content is None:
            return {"content": [text(f"unknown scenario {args.get('scenario')!r}")], "isError": True}
        return {"content": content, "isError": False}
    return {"content": [text(f"unknown tool {name!r}")], "isError": True}


def handle(msg: Json) -> Json | None:
    method = msg.get("method")
    ident = msg.get("id")
    if ident is None:  # a notification (notifications/initialized, cancelled, ...)
        return None
    params = msg.get("params") or {}
    if method == "initialize":
        result: Json = {
            "protocolVersion": params.get("protocolVersion", "2025-06-18"),
            "capabilities": {"tools": {"listChanged": False}},
            "serverInfo": {"name": "mock-mcp", "version": "0.0.1"},
        }
    elif method == "tools/list":
        result = {"tools": TOOLS}
    elif method == "tools/call":
        result = call_tool(params.get("name", ""), params.get("arguments") or {})
    elif method in ("resources/list", "resources/templates/list", "prompts/list"):
        key = {"resources/list": "resources", "resources/templates/list": "resourceTemplates", "prompts/list": "prompts"}[method]
        result = {key: []}
    elif method == "ping":
        result = {}
    else:
        return {"jsonrpc": "2.0", "id": ident, "error": {"code": -32601, "message": f"method not found: {method}"}}
    return {"jsonrpc": "2.0", "id": ident, "result": result}


def main() -> int:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        reply = handle(msg)
        if reply is not None:
            sys.stdout.write(json.dumps(reply) + "\n")
            sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
