# CodexCLI — agent map

A fork of openai/codex (`upstream`). The work here is the tui-rtwk port: the
Codex terminal UI `codex-rs/tui` (ratatui, the oracle — not edited) carried one
to one onto rtwk (`/Users/admin/Documents/dev_projects/rtwk`) as
`codex-rs/tui-rtwk`, built as a native rtwk app the way a Flutter developer
would build it. Design: `docs/rtwk-port/architecture.md`; research:
`docs/rtwk-port/research/`; rtwk API: `docs/rtwk-port/refs/`
(`rtwk-in-one-screen.md` first) and the rtwk skill. The architecture rules are
enforced by `cargo nextest run -p codex-tui-rtwk --test rules`.

- Orchestrator (main session): `docs/rtwk-port/orchestrator.md`, injected at
  session start by `.claude/hooks/orchestrator_context.py`.
- Fleet agents: their agent file (`.claude/agents/<type>.md`) and the fleet
  brief `.tm/fleet-brief.md` (returned by `task_start`). They do not read this
  file (`omitClaudeMd: true`).

<!-- tm -->
## Work is tracked in tm

Tasks, the plan and its state live in tm — not in markdown and not in the
harness's todo list. This checkout's project is the one `.tm/project.yaml`
names; `tm` finds its store through the machine's registry, `~/.tm/tm.yaml`.

- Where work stands and where the last session stopped: `project_status`
  first, not the git log.
- A task is claimed before the first edit (`task_start`) and handed over with
  `task_done`; what may be taken next is `get_active_front`.
- Where the tm MCP server only reads (under OpenCode, agy and pi), the
  tracker is changed in the shell, whose commands carry the session:
  `tm task start <alias|id>`, `tm task done <alias|id>`.
- How any of it is done: the tm skills — `tm-implement`, `tm-plan`,
  `tm-review`, `tm-memory`, and `tm` where none of them fits.

Written by `tm setup` and rewritten whole by its next run; the repository's
own rules and checks belong outside these markers.
<!-- /tm -->
