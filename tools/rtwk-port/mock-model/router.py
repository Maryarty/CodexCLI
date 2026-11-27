"""Router mode: one session, the user's message picks the response.

    try.sh router rtwk|oracle        # then type: exec, patch, plan, image, web, mcp, ... or all

The router is stateless: from the request body alone it finds
- the user's last real message (the last `user` message that is not injected context:
  not starting with `<` such as `<environment_context>`, not an AGENTS.md block),
- the step, the number of tool outputs that came after it (a tool call needs the next
  model request to answer it, so step k of a type is the k-th response of its scenario).
A keyword picks a scenario of `scenarios/` and serves its first turn's responses by step;
`all` serves the showcase, the responses of many scenarios merged so that each ends in a
tool call except the last. Any other text gets a short answer listing the keywords.
The thread-title request is answered by server.py, not here.

Three requests the router knows apart from keywords:
- the compaction request (the config sets `compact_prompt` to a marker): a summary;
- a spawned agent's request (its prompt carries `[mock-child]`): a one-line answer;
- the `compact` keyword answers with token usage over the auto-compaction limit, so the
  next message compacts first (manual `/compact` also works).
"""

from __future__ import annotations

import json
import re
from typing import Any

import harness
import scenario as scenarios
from configs import COMPACT_PROMPT, COMPACT_TOP, MCP_TABLES
from events import ev_completed_with_tokens
from scenario import Resp, Scenario, Turn, text_response

Json = dict[str, Any]

CHILD_MARKER = "[mock-child]"

# (scenario, keywords, what it shows). The first keyword is the one the help lists.
ROUTES: list[tuple[str, tuple[str, ...], str]] = [
    ("text-100", ("text", "stream"), "600 streamed words"),
    ("markdown", ("markdown", "md"), "heading, lists, code block, table, quote"),
    ("reasoning", ("reasoning", "think"), "reasoning summary, then the answer"),
    ("exec-command", ("exec", "shell", "command"), "two commands, no approval (one fails)"),
    ("exec-approval", ("approve", "approval", "escalate"), "a command that asks for approval"),
    ("patch-approval", ("patch", "diff"), "apply_patch (update + add) that asks for approval"),
    ("patch-heredoc", ("heredoc",), "apply_patch typed into a shell heredoc"),
    ("plan-tool", ("plan", "todo"), "update_plan three times"),
    ("proposed-plan", ("proposal", "proposed"), "a <proposed_plan> block"),
    ("view-image", ("image", "view"), "view_image of a PNG"),
    ("image-generation", ("imagegen", "generate", "draw"), "an image_generation_call"),
    ("web-search", ("web", "search"), "a web search item, then the answer"),
    ("mcp-tool", ("mcp",), "MCP echo and image tool calls"),
    ("unified-exec", ("unified", "stdin", "pty"), "a kept process fed through write_stdin"),
    ("background-terminal", ("background", "sleep"), "a command left running after the turn"),
    ("request-user-input", ("input", "question", "ask"), "request_user_input, a question to answer"),
    ("multi-agent", ("agent", "spawn"), "spawn_agent, the child answers"),
    ("code-mode", ("code", "js"), "the code-mode exec custom tool"),
    ("tool-search", ("toolsearch", "discover"), "a tool_search_call"),
    ("custom-tool-unknown", ("custom",), "a custom tool call Codex refuses"),
    ("model-verification", ("verify", "verification"), "response.metadata with a verification recommendation"),
    ("token-usage", ("tokens", "usage"), "an answer with 12k tokens used (cached, reasoning parts)"),
    ("rate-limits", ("limits", "ratelimit"), "an answer with rate-limit headers"),
    ("usage-limit", ("quota", "limit429"), "HTTP 429 usage_limit_reached"),
    ("error-fatal", ("error", "fatal"), "insufficient_quota: the turn ends in an error"),
    ("interrupted", ("interrupt", "slow"), "a 30 s stream to cut with Esc"),
]
# Not routable: error-retry and error-stream-closed (the response depends on how many times
# it was asked, which a stateless router cannot see); error-context-window (whether Codex
# then compacts depends on the history, so the number of requests is not fixed); the
# multi-prompt scenarios compact-manual, compact-auto, multi-turn, long-transcript.
SPECIAL = {
    "compact": "token usage over the compaction limit; the next message compacts first",
    "all": "the showcase: reasoning, web, plan, exec, patch, image, MCP, unified exec, a question, an agent",
}
# The showcase order. Types needing approval or an answer are in it on purpose: the person
# answers them. Types that end the turn early (errors, interrupt) are not.
SHOWCASE = [
    "reasoning",
    "web-search",
    "plan-tool",
    "exec-command",
    "exec-approval",
    "patch-approval",
    "view-image",
    "image-generation",
    "mcp-tool",
    "unified-exec",
    "request-user-input",
    "multi-agent",
]


def chain_of(name: str) -> list[Resp]:
    """The responses of a scenario's first turn, in step order (a spawned agent's
    matched child response is not a step of the parent)."""
    sc = scenarios.load(name)
    return [r for r in sc.turns[0].responses if not r.child]


def merge(chains: list[list[Resp]], tail: Resp) -> list[Resp]:
    """Concatenates chains: the last (plain) response of one is joined to the first of the
    next, so each response but the last ends in a tool call."""
    out: list[Resp] = []
    pending: Resp | None = None  # the plain final response of the chain so far
    for chain in chains + [[tail]]:
        for i, resp in enumerate(chain):
            if i == 0 and pending is not None:
                resp = join(pending, resp)
                pending = None
            if i == len(chain) - 1 and not ends_in_call(resp):
                pending = resp
            else:
                out.append(resp)
    if pending is not None:
        out.append(pending)
    return out


def ends_in_call(resp: Resp) -> bool:
    return any(
        e.get("type") == "response.output_item.done"
        and e.get("item", {}).get("type") in ("function_call", "custom_tool_call", "tool_search_call")
        for e in resp.events
    )


def join(a: Resp, b: Resp) -> Resp:
    """a's events without its `completed`, then b's without its `created`."""
    head = [e for e in a.events if e.get("type") != "response.completed"]
    rest = [e for e in b.events if e.get("type") != "response.created"]
    return Resp(head + rest, max(a.pace, b.pace), headers=a.headers + b.headers)


def over_limit() -> Resp:
    text = "Context usage is now over the limit. Send any message: Codex compacts first, then answers."
    return Resp(
        [
            harness.ev_response_created("rt-compact"),
            harness.ev_assistant_message("msg-rt-compact", text),
            ev_completed_with_tokens("rt-compact", 330_000),
        ]
    )


def summary() -> Resp:
    return text_response("rt-summary", ["Summary: a router session; the user tried several response types."])


def child() -> Resp:
    return text_response("rt-child", ["child done"])


def helptext() -> Resp:
    keys = ", ".join(sorted({r[1][0] for r in ROUTES} | set(SPECIAL)))
    return text_response(
        "rt-help",
        [f"Mock model router. Type one keyword to get that response type: {keys}. ", "`all` plays every type in one turn."],
    )


class Router:
    def __init__(self) -> None:
        self.aliases: dict[str, str] = {}
        self.chains: dict[str, list[Resp]] = {}
        for name, words, _ in ROUTES:
            for w in words:
                self.aliases[w] = name
            self.chains[name] = chain_of(name)
        self.aliases["compact"] = "compact"
        self.aliases["all"] = "all"
        self.chains["compact"] = [over_limit()]
        self.chains["all"] = showcase()

    def respond(self, body: bytes) -> Resp:
        text, step = analyse(body)
        if COMPACT_PROMPT in text:
            return summary()
        if last_item_mentions(body, CHILD_MARKER):
            return child()
        key = pick(text, self.aliases)
        if key is None:
            return helptext()
        chain = self.chains[key]
        if step < len(chain):
            return chain[step]
        return text_response("rt-end", [f"({key}: no more steps) "])

    def pick_name(self, text: str) -> str | None:
        return pick(text, self.aliases)


def showcase() -> list[Resp]:
    tail = text_response("rt-all-end", ["Showcase finished. ", "END-router-all"], pace=0.01)
    return merge([chain_of(n) for n in SHOWCASE], tail)


def pick(text: str, aliases: dict[str, str]) -> str | None:
    for token in re.findall(r"[a-z0-9]+", text.lower()):
        if token in aliases:
            return aliases[token]
    return None


def analyse(body: bytes) -> tuple[str, int]:
    """(the user's last real message, the tool outputs after it)."""
    try:
        items = json.loads(body.decode("utf-8", "replace")).get("input") or []
    except ValueError:
        return "", 0
    if not isinstance(items, list):
        return "", 0
    last, text = -1, ""
    for i in range(len(items) - 1, -1, -1):
        item = items[i]
        if not isinstance(item, dict) or item.get("role") != "user" or item.get("type", "message") != "message":
            continue
        t = message_text(item)
        if t and not t.lstrip().startswith(("<", "# AGENTS.md")):
            last, text = i, t
            break
    step = sum(
        1
        for item in items[last + 1 :]
        if isinstance(item, dict) and str(item.get("type", "")).endswith("_output")
    )
    return text, step


def last_item_mentions(body: bytes, marker: str) -> bool:
    """A spawned agent's request ends with the message that started it (any item type)."""
    try:
        items = json.loads(body.decode("utf-8", "replace")).get("input") or []
    except ValueError:
        return False
    return bool(items) and marker in json.dumps(items[-1])


def message_text(item: Json) -> str:
    content = item.get("content")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(p.get("text", "") for p in content if isinstance(p, dict) and p.get("type") == "input_text")
    return ""


def build_scenario() -> Scenario:
    """The router as a scenario: one turn per keyword (check.py runs them, in order)."""
    r = Router()
    turns: list[Turn] = []
    for name, words, _ in ROUTES:
        responses = r.chains[name]
        if name == "multi-agent":
            # The server handles the child's request separately from the parent's steps.
            responses = [*responses, child()]
        turns.append(
            Turn(
                words[0],
                responses,
                outcome="failed" if name in FAILS else "completed",
                esc_after=4.0 if name == "interrupted" else None,
            )
        )
    turns.append(Turn("compact", r.chains["compact"]))
    # the next message compacts first: one more request (the summary) before the help text
    turns.append(Turn("nothing special", [summary(), helptext()]))
    turns.append(Turn("all", [*r.chains["all"], child()]))
    # The router is stateless, so any keyword may start a session: `check.py --jobs N` runs four
    # pieces side by side (~40 s each, 29 turns of ~4 s were one 151 s session). Never cut
    # between `compact` and `nothing special`: the second one needs the first one's token usage.
    cut_before = ("proposal", "code", "compact")
    cuts = tuple(i for i, t in enumerate(turns) if t.prompt in cut_before)
    assert len(cuts) == len(cut_before), "router.py: a cut keyword is not a turn any more"
    return Scenario(
        name="router",
        description="one session: the keyword in the user's message picks the response type; `all` plays the showcase",
        turns=turns,
        sentinel="END-router-all",
        approval_policy="on-request",
        sandbox_mode="read-only",
        router=True,
        cuts=cuts,
        pty_only=True,
        allow_unsupported=True,  # the `custom` keyword is a call Codex refuses
        config_top=COMPACT_TOP,
        config_tables=MCP_TABLES
        + "\n[features]\ndefault_mode_request_user_input = true\ncode_mode = true\n"
        + "\n[tools.update_plan]\nenabled = true\n",
        seed_files=seeds(),
        helpers="every scenario; routing by the request body (resize_reflow.rs `mount_resize_response` matches bodies the same way)",
        suite="n/a: Codex tests each type separately; the router is a convenience of this tool",
    )


def seeds() -> dict[str, str | bytes]:
    out: dict[str, str | bytes] = {}
    for name, _, _ in ROUTES:
        out.update(scenarios.load(name).seed_files)
    return out


FAILS = {"usage-limit", "error-fatal"}
