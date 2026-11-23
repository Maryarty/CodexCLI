# Fleet start context: what was cut, why it went wrong, what keeps it from recurring

Date 2026-11-23. Source: the Opus start-context review (side-session note
`start-context-review.md`) and the person's decisions on it. Claude Code 2.1.330.

## Measured before / after (probe agents: one `echo probe` Bash call)

First-call input tokens (input + cache read + cache write). "Fresh" = a new
headless session of the same binary; "before" ran in a temporary worktree at
`90c720a12` with the old `.claude/settings.json`, "after" in this checkout.
The desktop session caches agent definitions at its start, so desktop numbers
change only in sessions started after this commit.

| agent | desktop before | fresh before | fresh after | saved |
|---|---|---|---|---|
| codex-porter | 14.4k | 13.8k | **8.1k** | −5.7k |
| codex-reviewer | 13.9k | 13.3k | **7.5k** | −5.8k |
| codex-light-reviewer | 13.8k | 13.2k | **7.4k** | −5.8k |
| codex-waiter | 11.0k | 10.4k | **4.4k** | −6.0k |
| codex-filler | 12.3k | 11.7k | **5.8k** | −6.0k |

What the fresh runs show: instruction files before `User, Project, AutoMem`,
after none; the tm SubagentStart text before in all five, after only in the
three with tm tools; tools unchanged (`Bash, Read, Edit, Write, ToolSearch`).
The orchestrator SessionStart text reached the main session and no subagent.

Desktop sessions add ~0.6k at call 1 (16 side-session dirs) and ~2.3k at
call 2 (computer-use MCP instructions) that the headless runs do not have.

Porter at its first useful read (desktop, estimate from the blocks):
24–25k before → ~17k after (the brief 7.2k → 4.7k chars, no second ToolSearch
round for `get_task`) → ~15k once computer-use is off for these sessions.

## What changed

- All five agent files: `omitClaudeMd: true` (a Claude Code agent field: the
  subagent runs without the user, project and local CLAUDE.md files; that also
  drops AGENTS.md, which CLAUDE.md imports, and the auto-memory index).
- AGENTS.md: a ~0.7k map. "rtwk in one screen" moved to
  `refs/rtwk-in-one-screen.md`. "What we build", "Architecture is enforced"
  and "Agents and tokens" (dated quotes included, sccache fact corrected) moved
  to `docs/rtwk-port/orchestrator.md`, printed into the main session by the
  local SessionStart hook `.claude/hooks/orchestrator_context.py`, which exits
  silently when its input has `agent_type`. The tm block between the tm markers
  stays where `tm setup` writes it; subagents no longer read AGENTS.md at all,
  so it needs no other handling.
- `codex-porter.md` is the one source of porter process; `.tm/fleet-brief.md`
  keeps only the rules all types share (Done 4, Scripts first, Test commands
  and the oracle shard run left it). A two-line batched-read example and the
  `read:` habit sit at the top of porter.md.
- Reviewers: the batched-read line, the shard path; filler and waiter: the
  `cd` line. Their bodies otherwise stay (the review said KEEP).
- `agent_models.py` refusal text points to porter.md "Checks", not the brief.
- Local `.claude/settings.json`: tm's SubagentStart hook matcher
  `^(?!codex-(waiter|filler)$).*` (those two have no tm tools); the
  orchestrator SessionStart hook added.

Contradictions of the review: C1 (task_start and get_task together), C2 (cd:
see root cause 3), C3 (no Grep/Glob promised), C4 (no build in the first turn),
C5 (one rule, `PORTER_RUN_OK=1`), C6 (one spelling of check), C7 (no binary run
in the handover), C8 (shard run only in the reviewers), C9 (kache), C10
(follow the base the prompt names), C11 (global CLAUDE.md no longer reaches
agents; bgrun is the means), C13 (report file only where the prompt names one)
are fixed in the files. C12 (the SubagentStart list names `active_claims` and
reaches agents without tm) is tm's text: narrowed by the matcher for waiter and
filler; the list itself is for the tm owner.

## Root causes

1. **Grep and Glob never reached any session.** The native (standalone)
   Claude Code build removes both tools and aliases `grep`/`find` in the Bash
   tool to its embedded ugrep and bfs. It keeps them only when the session is
   launched with `--allowedTools` or `--tools` naming them, or under the
   `local-agent` entrypoint (in the binary: `dR()`/`b6()` and
   `searchToolsOptIn`). The desktop app launches without them, so the main
   session has none, and an agent's `tools:` list can only narrow what the
   session has. Settings `permissions.allow` does not opt in. The agent file
   was written from the documented tool set and never checked against a run;
   the porter then got a rule it could not follow, and the harness's own error
   ("search with `grep` via Bash instead") overrode it.
   The tools cannot be given to desktop agents from this repository; the
   embedded grep/find in Bash are the same engines, so the agent files now
   say so and drop the two names.
2. **The rules contradicted each other** because each was restated where it
   was needed at the time (porter.md, the brief, the launch prompt, AGENTS.md,
   a hook) and later edits changed one copy: the check got three spellings, the
   exception two forms, the first-turn build survived the no-cargo rule. Facts
   written once and never re-read (sccache, "task_start returns the same as
   get_task") went stale. Nothing compared the copies.
3. **The per-call "needs no `cd`" text** comes from tm's PreToolUse hook when
   the session's cwd is inside a git worktree and the command starts with `cd`
   (reproduced: same payload, cwd in a worktree → text; cwd at the checkout
   root → silent). A subagent inherits the orchestrator's cwd. The orchestrator
   had `cd`'d into a landing worktree, and porter.md told porters to start
   every call with `cd`: 39 of 39 such calls in one porter, 177 of 177 in
   another, 0 of 24 in a porter launched from a non-worktree cwd.
   `orchestrator.md` now says: launch agents from the checkout root.
4. **Orchestrator material reached every agent** because CLAUDE.md (→ AGENTS.md)
   and the auto-memory index are loaded into every subagent by default, and
   AGENTS.md grew as the orchestrator's notebook.

## Guards

- `tools/rtwk-port/agent-ctx-check/check.py` (run after editing an agent file,
  the brief, AGENTS.md or the hooks; `--measure` prints the newest first-call
  tokens per type): each agent's listed tools against the tools the newest
  transcript of that type actually got; a body naming a tool the agent lacks;
  `omitClaudeMd` on every agent; porter-process markers in the brief; sccache
  without kache; the two local hooks still registered (tm setup may rewrite
  `.claude/settings.json`). Run against the old agent files it reports the
  Grep/Glob gap and the five missing `omitClaudeMd`.
- `orchestrator.md` "One source per rule" and the task description shape with
  `read:` ranges.
- Probe agents: `~/.cache/codexcli/agent-ctx <agentId>` and
  `~/.cache/codexcli/agent-ctx-session <session dir>` read the transcripts.

## Not changeable from this repository

- Computer-use MCP instructions (~2.3k per desktop agent): sent to every
  subagent even when its tools are denied (easel is denied in
  `settings.local.json` and its instructions still arrive). Off only by the
  desktop's computer-use setting or by disabling the server for this project.
- The 16 side-session directories and the orchestrator's gitStatus in every
  agent's environment block: the desktop app.
- tm (the tm owner): `task_start` returning the description; the cd hook
  firing only when the cd target is not the cwd's own worktree; the
  SubagentStart tool list following the agent's tools (no `active_claims`,
  nothing for agents without tm); SessionStart:compact status not printed into
  subagents; agent answers without scores and timestamps; a shorter
  `task_start` schema.
- Grep/Glob for desktop sessions: the desktop app would have to launch
  Claude Code with `--allowedTools Grep,Glob`.
