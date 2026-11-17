# Rules for every agent of the CodexCLI fleet ({FORK})

You are `{FORK}`. The tm hook supplies your session and subagent identity.
Your prompt carries only your task; the working rules are in your agent file
(`.claude/agents/codex-porter.md` or `codex-reviewer.md`) and here.

## What the port is
- `codex-rs/tui` (ratatui) is the oracle at the commit in
  `docs/rtwk-port/ORACLE_REF`; `codex-rs/tui-rtwk` is its rtwk twin, one to
  one in behaviour. Architecture and decisions: `docs/rtwk-port/architecture.md`
  (read the sections your task names, by heading).
- rtwk API: `docs/rtwk-port/refs/` first (generated; grep it), rtwk source in
  ranges only for what the references do not answer. rtwk is read-only here;
  a gap goes into your handover as "rtwk needs: …".
- Never edit `codex-rs/tui/**` unless your task's area names the file.

## Checks: scoped while you work, full once at the end
- While iterating: `~/.cache/rtwk/acargo nextest run -p codex-tui-rtwk
  --status-level fail --final-status-level fail --hide-progress-bar` (add
  `-E 'test(<filter>)'`), `acargo clippy -q -p codex-tui-rtwk --all-targets
  -- -D warnings`.
- Before `task_done`, once: `acargo fmt -p codex-tui-rtwk -- --check`, the
  crate's whole nextest run, clippy as above. A long one through
  `~/.cache/rtwk/bgrun.sh`.
- Name anything that was red when you arrived, separately from your results.

## Where your work lives
- Stay inside your task's area; widen it with `task_update` (listing the
  existing entries) only for files no live claim covers. Shared registration
  files (`codex-rs/tui-rtwk/src/lib.rs`, a parent `mod.rs`,
  `codex-rs/Cargo.toml`) belong to the task that names them.
- Commit on your worktree branch, staging by path. Never merge into main,
  never merge another branch into yours.
- Read the `notes` tm hands you on each call before your next edit.

## Handover
`task_done` with deliverables: the commits, a result summary (what was done vs
the task, codex-tui behaviours covered and the ones left with reasons, parity
tests and their bites, rtwk gaps found, risks, what others must change).
