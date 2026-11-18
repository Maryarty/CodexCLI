"""Event shapes the Python harness lacks, copied in shape from the Rust helpers.

Every builder names the Rust function it mirrors in
`codex-rs/core/tests/common/responses.rs` (line numbers at the pinned tree).
The helpers the harness has (sse, ev_response_created, ev_completed,
ev_completed_with_usage, ev_assistant_message, ev_message_item_added,
ev_output_text_delta, ev_function_call, ev_failed) come from `harness.py`, not
from here. Of responses.rs's 23 `ev_*` builders (the ones that make an event) every
one is either in the harness or below; `coverage.py` lists them.
"""

from __future__ import annotations

import base64
import json
from typing import Any

Json = dict[str, Any]


def ev_reasoning_item(item_id: str, summary: list[str], raw_content: list[str]) -> Json:
    """responses.rs:911 `ev_reasoning_item` (response.output_item.done, reasoning)."""
    event: Json = {
        "type": "response.output_item.done",
        "item": {
            "type": "reasoning",
            "id": item_id,
            "summary": [{"type": "summary_text", "text": t} for t in summary],
            "encrypted_content": base64.b64encode(
                ("b" * 550 + "".join(raw_content)).encode()
            ).decode(),
        },
    }
    if raw_content:
        event["item"]["content"] = [{"type": "reasoning_text", "text": t} for t in raw_content]
    return event


def ev_reasoning_item_added(item_id: str, summary: list[str]) -> Json:
    """responses.rs:943 `ev_reasoning_item_added`."""
    return {
        "type": "response.output_item.added",
        "item": {
            "type": "reasoning",
            "id": item_id,
            "summary": [{"type": "summary_text", "text": t} for t in summary],
        },
    }


def ev_reasoning_summary_text_delta(delta: str) -> Json:
    """responses.rs:959 `ev_reasoning_summary_text_delta`."""
    return {
        "type": "response.reasoning_summary_text.delta",
        "delta": delta,
        "summary_index": 0,
    }


def ev_reasoning_text_delta(delta: str) -> Json:
    """responses.rs:967 `ev_reasoning_text_delta`."""
    return {"type": "response.reasoning_text.delta", "delta": delta, "content_index": 0}


def ev_web_search_call_added_partial(item_id: str, status: str) -> Json:
    """responses.rs:975 `ev_web_search_call_added_partial`."""
    return {
        "type": "response.output_item.added",
        "item": {"type": "web_search_call", "id": item_id, "status": status},
    }


def ev_web_search_call_done(item_id: str, status: str, query: str) -> Json:
    """responses.rs:986 `ev_web_search_call_done`."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "web_search_call",
            "id": item_id,
            "status": status,
            "action": {"type": "search", "query": query},
        },
    }


def ev_apply_patch_custom_tool_call(call_id: str, patch: str) -> Json:
    """responses.rs:1091 `ev_apply_patch_custom_tool_call` (freeform apply_patch)."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "custom_tool_call",
            "name": "apply_patch",
            "input": patch,
            "call_id": call_id,
        },
    }


def exec_command_call(
    call_id: str,
    cmd: str,
    *,
    yield_time_ms: int | None = None,
    escalate_justification: str | None = None,
    tty: bool | None = None,
) -> Json:
    """responses.rs:1103 `ev_exec_command_call_with_args`, with the argument shape of
    core/tests/suite/approvals.rs:361 `exec_command_event` (yield_time_ms,
    sandbox_permissions = "require_escalated" + justification)."""
    args: Json = {"cmd": cmd}
    if tty is not None:
        args["tty"] = tty
    if yield_time_ms is not None:
        args["yield_time_ms"] = yield_time_ms
    if escalate_justification is not None:
        args["sandbox_permissions"] = "require_escalated"
        args["justification"] = escalate_justification
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "function_call",
            "call_id": call_id,
            "name": "exec_command",
            "arguments": json.dumps(args),
        },
    }


def ev_failed_with_code(response_id: str, code: str, message: str) -> Json:
    """responses.rs:1120 `sse_failed` (response.failed with a chosen error code); the
    harness's own `ev_failed` fixes the code to `server_error`."""
    return {
        "type": "response.failed",
        "response": {"id": response_id, "error": {"code": code, "message": message}},
    }


def add_file_patch(path: str, content_lines: list[str]) -> str:
    """approvals.rs:310 `build_add_file_patch`, for several lines."""
    body = "".join(f"+{line}\n" for line in content_lines)
    return f"*** Begin Patch\n*** Add File: {path}\n{body}*** End Patch\n"


def ev_completed_with_tokens(response_id: str, total_tokens: int) -> Json:
    """responses.rs:849 `ev_completed_with_tokens` (all tokens counted as input)."""
    return {
        "type": "response.completed",
        "response": {
            "id": response_id,
            "usage": {
                "input_tokens": total_tokens,
                "input_tokens_details": None,
                "output_tokens": 0,
                "output_tokens_details": None,
                "total_tokens": total_tokens,
            },
        },
    }


def ev_model_verification_metadata(response_id: str, verifications: list[str]) -> Json:
    """responses.rs:838 `ev_model_verification_metadata`."""
    return {
        "type": "response.metadata",
        "sequence_number": 1,
        "response_id": response_id,
        "metadata": {"openai_verification_recommendation": verifications},
    }


def ev_image_generation_call(item_id: str, status: str, revised_prompt: str, result: str) -> Json:
    """responses.rs:998 `ev_image_generation_call` (`result` is base64 image data)."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "image_generation_call",
            "id": item_id,
            "status": status,
            "revised_prompt": revised_prompt,
            "result": result,
        },
    }


def ev_function_call_with_namespace(call_id: str, namespace: str, name: str, arguments: str) -> Json:
    """responses.rs:1028 `ev_function_call_with_namespace` (an MCP tool is
    namespace `mcp__<server>`, name = the tool)."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "function_call",
            "call_id": call_id,
            "namespace": namespace,
            "name": name,
            "arguments": arguments,
        },
    }


def ev_tool_search_call(call_id: str, arguments: Json) -> Json:
    """responses.rs:1046 `ev_tool_search_call` (client-executed tool search)."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "tool_search_call",
            "call_id": call_id,
            "execution": "client",
            "arguments": arguments,
        },
    }


def ev_custom_tool_call(call_id: str, name: str, input_text: str) -> Json:
    """responses.rs:1058 `ev_custom_tool_call`."""
    return {
        "type": "response.output_item.done",
        "item": {"type": "custom_tool_call", "call_id": call_id, "name": name, "input": input_text},
    }


def ev_custom_tool_call_with_namespace(call_id: str, namespace: str, name: str, input_text: str) -> Json:
    """responses.rs:1070 `ev_custom_tool_call_with_namespace`."""
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "custom_tool_call",
            "call_id": call_id,
            "namespace": namespace,
            "name": name,
            "input": input_text,
        },
    }


def ev_apply_patch_exec_command_call_via_heredoc(call_id: str, patch: str) -> Json:
    """responses.rs:1113 `ev_apply_patch_exec_command_call_via_heredoc` (apply_patch typed
    into exec_command; core intercepts it, unified_exec.rs:630)."""
    return exec_command_call(call_id, f"apply_patch <<'EOF'\n{patch}\nEOF\n")


def ev_rate_limits(used_percent: int = 42, window_minutes: int = 60, reset_at: int = 1700000000) -> Json:
    """request_user_input.rs:218 `ev_rate_limits` (a `codex.rate_limits` event; Codex reads it
    on the WebSocket transport, the HTTP transport reads headers instead: `rate_limit_headers`)."""
    return {
        "type": "codex.rate_limits",
        "plan_type": "plus",
        "rate_limits": {
            "allowed": True,
            "limit_reached": False,
            "primary": {"used_percent": used_percent, "window_minutes": window_minutes, "reset_at": reset_at},
            "secondary": None,
        },
        "code_review_rate_limits": None,
        "credits": None,
        "promo": None,
    }


def rate_limit_headers(
    primary: float, secondary: float, primary_minutes: int = 10, secondary_minutes: int = 60
) -> tuple[tuple[str, str], ...]:
    """client.rs:3767 `token_count_includes_rate_limits_snapshot`: the HTTP transport's rate
    limits are response headers."""
    return (
        ("x-codex-primary-used-percent", str(primary)),
        ("x-codex-secondary-used-percent", str(secondary)),
        ("x-codex-primary-window-minutes", str(primary_minutes)),
        ("x-codex-secondary-window-minutes", str(secondary_minutes)),
        ("x-codex-primary-reset-at", "1704069000"),
        ("x-codex-secondary-reset-at", "1704074400"),
    )


def png_bytes(width: int, height: int, rgb: tuple[int, int, int]) -> bytes:
    """A solid-colour PNG (view_image.rs:157 `png_bytes` makes one the same way)."""
    import struct
    import zlib

    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    row = b"\x00" + bytes(rgb) * width
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(row * height))
        + chunk(b"IEND", b"")
    )
