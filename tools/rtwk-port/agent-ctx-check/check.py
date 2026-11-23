#!/usr/bin/env python3
"""Guard for the fleet agents' start context (2026-11-23 start-context review).

Run after editing `.claude/agents/*.md`, `.tm/fleet-brief.md`, AGENTS.md or the hooks:
    python3 -I tools/rtwk-port/agent-ctx-check/check.py [--measure] [--agents-dir DIR]

Checks
1. Tools: every tool an agent file lists is one the harness really gives that agent type.
   The truth is the newest subagent transcript of the type (`*.prefix.json` `toolNames`;
   tm tools are deferred and not listed there, so only non-MCP tools are compared).
   This is what failed before: Grep/Glob were listed but the native build drops them
   (Bash's grep/find are its embedded ugrep/bfs) unless the session is launched with
   `--allowedTools Grep,Glob`, which the desktop app does not do.
2. Prompt vs tools: an agent body must not tell the agent to use a tool it does not have.
3. omitClaudeMd: every codex-* agent sets it, so AGENTS.md, CLAUDE.md and MEMORY.md stay
   out of subagents.
4. One source: porter process markers must not appear in the shared brief.
5. Stale facts: `sccache` named without `kache`.
6. Hooks: the orchestrator SessionStart hook is registered; tm's SubagentStart hook skips
   the agents without tm tools (`tm setup` may rewrite .claude/settings.json).
--measure: first-call input tokens of the newest transcript per agent type.
Exit 1 on any ERROR.
"""
import glob
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
AGENTS = Path(sys.argv[sys.argv.index("--agents-dir") + 1]) if "--agents-dir" in sys.argv else ROOT / ".claude" / "agents"
PROJ = Path.home() / ".claude" / "projects" / ("-" + str(ROOT).strip("/").replace("/", "-").replace("_", "-"))
HARNESS_ONLY = {"SubagentHandback"}
errors, warns = [], []


def frontmatter(path):
    s = path.read_text()
    m = re.match(r"---\n(.*?)\n---\n(.*)", s, re.S)
    fm = {}
    for line in m.group(1).splitlines():
        k, _, v = line.partition(":")
        fm[k.strip()] = v.strip()
    return fm, m.group(2)


def newest_transcripts():
    """agentType -> (mtime, prefix.json path, jsonl path) of the newest subagent run."""
    out = {}
    for meta in glob.glob(str(PROJ / "*" / "subagents" / "agent-*.meta.json")):
        try:
            typ = json.load(open(meta)).get("agentType")
        except Exception:
            continue
        pre = meta.replace(".meta.json", ".prefix.json")
        if not typ or not os.path.exists(pre):
            continue
        t = os.path.getmtime(pre)
        if typ not in out or t > out[typ][0]:
            out[typ] = (t, pre, meta.replace(".meta.json", ".jsonl"))
    return out


def first_call_tokens(jsonl):
    for line in open(jsonl):
        d = json.loads(line)
        u = (d.get("message") or {}).get("usage") if d.get("type") == "assistant" else None
        if u:
            return u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0) + u.get("cache_creation_input_tokens", 0)


runs = newest_transcripts()
agents = sorted(AGENTS.glob("codex-*.md"))
for path in agents:
    fm, body = frontmatter(path)
    name = fm.get("name", path.stem)
    listed = [t.strip() for t in fm.get("tools", "").split(",") if t.strip()]
    plain = {t for t in listed if not t.startswith("mcp__")}
    if fm.get("omitClaudeMd") != "true":
        errors.append(f"{name}: no `omitClaudeMd: true` (AGENTS.md, CLAUDE.md, MEMORY.md would reach it)")
    run = runs.get(name)
    if run:
        given = set(json.load(open(run[1]))["state"]["toolNames"]) - HARNESS_ONLY
        missing = plain - given
        if missing:
            errors.append(f"{name}: lists {sorted(missing)} but the harness gave it {sorted(given)} "
                          f"(newest run {Path(run[1]).name})")
    else:
        warns.append(f"{name}: no transcript yet; launch a probe agent of this type to compare tools")
    for tool in ("Grep", "Glob", "Monitor", "WebFetch", "WebSearch", "Agent"):
        if tool in plain:
            continue
        for ln in body.splitlines():
            if re.search(rf"\b{tool}\b", ln) and not re.search(r"\bno\b|\bnot\b|There is no", ln, re.I):
                errors.append(f"{name}: body tells it to use {tool}, which it does not have: {ln.strip()[:90]}")
    print(f"{name:22} body {len(body):5} chars ~{len(body) / 2.8 / 1000:.1f}k tok; tools {len(listed)}")

brief = (ROOT / ".tm" / "fleet-brief.md").read_text()
for marker in ("PORTER_RUN_OK", "nextest run", "acargo check", "task_checkpoint", "Scripts first", "Test commands"):
    if marker in brief:
        errors.append(f"fleet-brief: porter process marker {marker!r} (one source: codex-porter.md)")
print(f"{'fleet-brief.md':22} {len(brief):5} chars ~{len(brief) / 2.8 / 1000:.1f}k tok")
am = (ROOT / "AGENTS.md").read_text()
print(f"{'AGENTS.md':22} {len(am):5} chars ~{len(am) / 2.8 / 1000:.1f}k tok (orchestrator only)")

for p in [ROOT / "AGENTS.md", ROOT / ".tm" / "fleet-brief.md", *agents]:
    t = p.read_text()
    if "sccache" in t and "kache" not in t.replace("sccache", ""):
        errors.append(f"{p.name}: names sccache without kache (acargo picks kache for new checkouts)")

settings = ROOT / ".claude" / "settings.json"
if settings.exists():
    hooks = json.load(open(settings)).get("hooks", {})
    if "orchestrator_context.py" not in json.dumps(hooks.get("SessionStart", [])):
        errors.append("settings.json: SessionStart orchestrator_context.py hook missing (tm setup rewrote it?)")
    for e in hooks.get("SubagentStart", []):
        if "tm hook" in json.dumps(e) and e.get("matcher") in (".*", "*", ""):
            warns.append("settings.json: tm SubagentStart hook runs for codex-waiter/filler too (matcher reset?)")
else:
    errors.append(".claude/settings.json missing")

if "--measure" in sys.argv:
    print("\nnewest run per type: first-call input tokens")
    for typ, (_, _, jl) in sorted(runs.items()):
        if typ.startswith("codex-") and os.path.exists(jl):
            print(f"  {typ:22} {first_call_tokens(jl)}  ({Path(jl).name})")

for w in warns:
    print("WARN ", w)
for e in errors:
    print("ERROR", e)
sys.exit(1 if errors else 0)
