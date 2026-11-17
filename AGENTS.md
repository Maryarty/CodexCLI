# CodexCLI — agent map

A fork of openai/codex (`upstream`). The work here is the tui-rtwk port: the
Codex terminal UI `codex-rs/tui` (ratatui, the oracle — not edited) carried one
to one onto rtwk (`/Users/admin/Documents/dev_projects/rtwk`) as
`codex-rs/tui-rtwk`. Design: `docs/rtwk-port/architecture.md`; research:
`docs/rtwk-port/research/`; rtwk API references: `docs/rtwk-port/refs/`.

## Agents and tokens
- Implementation: agent type `codex-porter` (Sonnet; a hook refuses a model
  override). Reviews and research: `codex-reviewer` (Sonnet; `model: opus`
  only for architecture and risky reviews). Long waits: `codex-waiter`
  (Haiku). Shared fleet rules: `.tm/fleet-brief.md`.
- Briefs carry only the task: name sections and `file:line` ranges, grep
  every name before writing it into a brief.
- Cargo through `~/.cache/rtwk/acargo`, quiet output (`-q`, nextest
  `--status-level fail --final-status-level fail --hide-progress-bar`), no
  call over ~4 min (`~/.cache/rtwk/bgrun.sh`). Read big files in ranges.
- An agent hands off at ~100 turns / ~250k context through
  `task_checkpoint`; the orchestrator at the end of a wave or ~230k (never
  past 350k) with a ≤40-line `## Now` handoff.

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
