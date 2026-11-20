"""The scenario model: a list of turns, each a user prompt and the model
responses (in request order) that follow it, plus what the CODEX_HOME needs."""

from __future__ import annotations

import importlib.util
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

import harness

Json = dict[str, Any]
SCENARIO_DIR = Path(__file__).resolve().parent / "scenarios"


@dataclass(frozen=True)
class Resp:
    """One SSE response: the events and the pause after each event, in seconds.

    The rest is for shapes the plain queue cannot say:
    - `status`, `headers`, `raw`: an HTTP answer other than 200 + SSE (a 429 with a
      JSON body), or SSE with extra headers (the rate-limit headers);
    - `pick`: serve this response to the first request for which `pick(request_json, body_text)`
      is true, instead of in queue order (core's `mount_sse_once_match`; for a spawned
      agent, whose request races the parent's)."""

    events: list[Json]
    pace: float = 0.0
    status: int = 200
    headers: tuple[tuple[str, str], ...] = ()
    raw: str | None = None
    pick: Callable[[Json, str], bool] | None = None
    child: bool = False  # a spawned agent's answer: the router serves it by marker, not as a step

    def body(self) -> str:
        return self.raw if self.raw is not None else harness.sse(self.events)

    @property
    def matched(self) -> bool:
        return self.pick is not None


@dataclass(frozen=True)
class Turn:
    """What the user types, and the responses the model gives while it runs
    (one per model request: a tool call needs a following response)."""

    prompt: str
    responses: list[Resp]
    outcome: str = "completed"  # "completed" or "failed" (a fatal error ends the turn)
    esc_after: float | None = None  # Seconds after the model request before the PTY sends Escape.


@dataclass(frozen=True)
class Scenario:
    name: str
    description: str
    turns: list[Turn]
    sentinel: str  # a token the last response ends with; a run must show it
    approval_policy: str = "never"  # config.toml approval_policy
    sandbox_mode: str = "read-only"
    stream_max_retries: int = 0
    approvals: int = 0  # approval prompts a TUI run shows (try.sh tells the person)
    seed_files: dict[str, str | bytes] = field(default_factory=dict)  # files of the workdir
    notes: str = ""
    # config.toml additions: top-level keys (before the first table) and tables. `{python}`
    # and `{mcp_server}` are replaced by this tool's interpreter and mcp_server.py.
    config_top: str = ""
    config_tables: str = ""
    allow_unsupported: bool = False  # the expected tool output is an 'unsupported ...' error
    pty_only: bool = False  # needs keys or slash commands: `codex exec` cannot run it
    router: bool = False  # served by router.py (the user's message picks the response)
    # What it mirrors, for the coverage table: Rust helpers and core suite tests.
    helpers: str = ""
    suite: str = ""

    @property
    def responses(self) -> list[Resp]:
        return [r for t in self.turns for r in t.responses]


def chunks(text: str, size: int) -> list[str]:
    return [text[i : i + size] for i in range(0, len(text), size)]


def text_response(rid: str, deltas: list[str], *, pace: float = 0.0, tail: list[Json] | None = None) -> Resp:
    """created, item added, one delta per element, item done (full text), completed.
    `tail` events go between the item and `completed` (none by default)."""
    full = "".join(deltas)
    mid = f"msg-{rid}"
    events = [
        harness.ev_response_created(rid),
        harness.ev_message_item_added(mid),
        *[harness.ev_output_text_delta(d) for d in deltas],
        harness.ev_assistant_message(mid, full),
        *(tail or []),
        harness.ev_completed(rid),
    ]
    return Resp(events, pace)


def tool_response(rid: str, call: Json, *, lead_text: str | None = None, pace: float = 0.0) -> Resp:
    """An optional short message, then one tool call; the client answers it with
    the tool output, which is the next model request (the next Resp)."""
    events: list[Json] = [harness.ev_response_created(rid)]
    if lead_text:
        events.append(harness.ev_assistant_message(f"msg-{rid}", lead_text))
    events += [call, harness.ev_completed(rid)]
    return Resp(events, pace)


def names() -> list[str]:
    return sorted(p.stem for p in SCENARIO_DIR.glob("*.py") if not p.name.startswith("_"))


def load(name: str) -> Scenario:
    path = SCENARIO_DIR / f"{name}.py"
    if not path.is_file():
        raise SystemExit(f"unknown scenario {name!r}; known: {', '.join(names())}")
    spec = importlib.util.spec_from_file_location(f"scenario_{name.replace('-', '_')}", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    scenario = module.SCENARIO
    assert scenario.name == name, f"{path}: SCENARIO.name is {scenario.name!r}"
    return scenario
