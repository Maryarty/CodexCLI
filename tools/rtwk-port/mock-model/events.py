"""Event shapes the Python harness lacks, copied in shape from the Rust helpers.

Every builder names the Rust function it mirrors in
`codex-rs/core/tests/common/responses.rs` (line numbers at the pinned tree).
The helpers the harness has (sse, ev_response_created, ev_completed,
ev_assistant_message, ev_message_item_added, ev_output_text_delta,
ev_function_call, ev_failed) come from `harness.py`, not from here.
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
) -> Json:
    """responses.rs:1103 `ev_exec_command_call_with_args`, with the argument shape of
    core/tests/suite/approvals.rs:361 `exec_command_event` (yield_time_ms,
    sandbox_permissions = "require_escalated" + justification)."""
    args: Json = {"cmd": cmd}
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
