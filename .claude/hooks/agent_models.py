#!/usr/bin/env python3
"""The model of a fleet agent comes from its agent type, never from the call.

PreToolUse hook on Agent/Task (.claude/settings.json). The fleet has four
types, each with its model fixed in its frontmatter (the person, 2026-11-21):
the implementer (`*-porter`) always Sonnet, the reviewer (`*-reviewer`)
always Opus, the light reviewer (`*-light-reviewer`) Sonnet, the helper
(`*-waiter`) Haiku; CodexCLI also the batch filler (`codex-filler`, Haiku pilot,
the person 2026-11-22).

- a `model` passed on the call is refused: the type decides it;
- general-purpose, claude and Plan are refused: they inherit the
  orchestrator's Opus and its whole tool list (Explore stays allowed for a
  read-only search);
- `effort` stays free (xhigh for complex slices).
Same script in rtwk and CodexCLI.
"""
import json
import sys
from pathlib import Path

AGENTS = Path(__file__).resolve().parent.parent / "agents"
PLAIN = {"general-purpose", "claude", "Plan", ""}

# Launch template (person, 2026-11-22): a porter's prompt is `task_start <alias>` + its worktree; the tm task
# holds the assignment. Measured: launch prompts ordering rebase + full gate first cost 78 of 108 calls
# (research/porter-steps, rampup-report) and the full gate per step 31 % of porter time.
import re
GATE = re.compile(r"clippy\s+--workspace|whole nextest|full (gate|checks?)|after each (green )?step|"
                  r"\brebase\b|reset --hard|run the probe|shards\.py --check", re.I)


def porter_prompt_problem(prompt: str) -> str | None:
    landing = prompt.lstrip().lower().startswith("landing job")
    if not landing and GATE.search(prompt):
        return ("codex-porter launch prompt orders a rebase / full gate / probe: forbidden (person 2026-11-22). "
                "Use the launch template (process.md §A4.2): `tm task <alias>; worktree <path>; task_start` - "
                "the tm task description is the assignment; checks are in .claude/agents/codex-porter.md 'Checks'.")
    if not landing and len(prompt) > 700:
        return ("codex-porter launch prompt over 700 chars: put the assignment into the tm task description "
                "(task_update) and launch with the template `tm task <alias>; worktree <path>; task_start`.")
    return None


def decide(call: dict) -> str | None:
    if call.get("tool_name") not in ("Agent", "Task"):
        return None
    inp = call.get("tool_input") or {}
    kind = inp.get("subagent_type") or ""
    if kind == "fork":
        return None
    types = ", ".join(sorted(p.stem for p in AGENTS.glob("*.md")))
    if inp.get("model"):
        return (f"No `model` on an Agent call: the agent type fixes it (implementer Sonnet, reviewer Opus, "
                f"light reviewer Sonnet, waiter and filler Haiku). Drop `model` and pick the type: {types}. `effort` stays free.")
    if kind == "codex-porter":
        why = porter_prompt_problem(inp.get("prompt") or "")
        if why:
            return why
    if kind in PLAIN:
        return (f"`{kind or 'general-purpose'}` inherits the orchestrator's Opus. Use a fleet type ({types}) "
                f"or Explore for a read-only search.")
    return None


def main() -> None:
    reason = decide(json.load(sys.stdin))
    if reason:
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }}))


if __name__ == "__main__":
    main()
