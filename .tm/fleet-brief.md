# Rules for every agent of the CodexCLI fleet ({FORK})

You are `{FORK}`. The tm hook supplies your session and subagent identity.
Your prompt carries only your task; the brief is the task's description in
tm, the working rules are in your agent file and here.

**How a screen is translated** — `docs/rtwk-port/process.md` Part 1: §2 (the
codex-tui → rtwk concept map) and §3 (the eight steps), with the worked
example in §4. Read them once before your first edit; a porter and a reviewer
both work by them. Appendix A (the fleet's process) is for the orchestrator;
read only the subsection this file points to.

## What the port is
- `codex-rs/tui` (ratatui) is the oracle at the commit in
  `docs/rtwk-port/ORACLE_REF`; `codex-rs/tui-rtwk` is its rtwk twin, one to
  one in behaviour. Architecture and decisions: `docs/rtwk-port/architecture.md`
  (read the sections your task names, by heading).
- rtwk API: `docs/rtwk-port/refs/` first (generated; grep it), rtwk source in
  ranges only for what the references do not answer. rtwk is read-only here.
- Behaviour, not mechanics (the person, 2026-11-18): match what the oracle
  shows and does, never how it does it. Scroll, selection, paging, editing,
  tabs, focus, mouse, widths and ellipsis belong to rtwk widgets and
  controllers (`.claude/skills/rtwk`, `guides/widgets-by-need.md`); no app-side
  shadow of their state, no carried oracle helper that duplicates them. If
  your brief or a research note names such a mechanism, use rtwk instead and
  say so in the handover. Process.md §5.
- The oracle's look and behaviour are the spec; no visual delta is accepted
  (the person, 2026-11-18). Never propose "accept" and never settle for the
  nearest ready widget that looks different (`Dialog`, `Autocomplete`):
  compose rtwk primitives until rows and styles equal the oracle's. Only what
  rtwk truly cannot do goes into the handover as "rtwk needs: …" with the
  route you tried (widgets, file:line) and a `docs/rtwk-port/rtwk-gaps.md`
  row; stop and report it rather than writing it off as a delta.
- The oracle's files, snapshots and tests for a screen: its rows in
  `docs/rtwk-port/refs/codex-registry.csv`. The oracle binary for pty
  scenarios: `~/.cache/codexcli/oracle-bin/` (never build codex-tui yourself).
- Never edit `codex-rs/tui/**`, `codex-rs/Cargo.toml` or `codex-rs/Cargo.lock`.

## A slice is done when (process.md §3, §A4)
1. **Tests first**: your first commit is `<alias>: parity tests (red)` —
   screen tests at the oracle's size with expected values from the oracle
   (its `.snap` text, its assertions), state tests (events in, model and
   effects out). Stubs with `todo!()` are fine in it. Then the code.
2. **Every oracle test** of your oracle files is mapped: ported (named after
   it) / n/a with a reason / delta.
3. **Parity pack** `docs/rtwk-port/parity/<alias>.md` (format: process.md
   §A4.3): oracle-test map, snapshots, visual deltas (kind from
   `parity/delta-kinds.md` or NEW, cause, the differing rows of both sides,
   proposal), pty scenarios `<alias>-<name>` and their result (or "n/a until
   scenario-harness"), rules extended, rtwk needs.
4. **ADR-002 conditions** hold: screen/dialog widgets
   `impl_widget!(stateful eq …)`; `show_dialog` callers and the request
   presenter under the `Navigator`; providers fixed, `Navigator` keyed; no
   decision from a same-flush `read`; routes and dialogs take ids, not data.
5. **ADR-003 R1–R14** hold: no `#[allow(clippy::disallowed_…)]` outside the
   allowlisted modules; new events, intents or lifecycle states extend the
   mandatory tests (R2 switch round-trip, R4 snapshot, R5 end-path matrix, R7
   `assert_consistent`, R8 lifecycle × intent, R9/R10 permutations).
6. Checks green (below); copied oracle code keeps a comment naming its file.

## Checks: scoped while you work, full once at the end
All from `codex-rs/tui-rtwk` (its own workspace), through `~/.cache/rtwk/acargo`.
- While iterating: `acargo nextest run --status-level fail
  --final-status-level fail --hide-progress-bar -E 'test(<filter>)'`,
  `acargo clippy -q --all-targets -- -D warnings`.
- Before `task_done`, once: `acargo fmt -- --check`, the crate's whole nextest
  run, clippy as above, your own pty scenarios. Anything long through
  `~/.cache/rtwk/bgrun.sh`.
- nextest `FLAKY`: name the test and its log in the handover; investigate
  when it covers what you changed.
- Name anything that was red when you arrived, separately from your results.

## Layers (process.md §A4.4)
- Core (`codex-tui-core`, no rtwk): models, `apply`, tickets, protocol
  mapping, command parsing. UI: view files only (widgets, `strings.rs`, debug
  entries) that never name the protocol, `shell`, tokio or std IO; UI-only
  state lives in `UiApp.ui`, never in the core model. `shell` runs effects and
  names no widget.
- Mock and fixture state is a JSONL event script applied through core
  `apply`, built from the oracle's test data; never data handed to a widget.
- If the brief seems to ask you to cross a layer, stop and say so in the
  handover; do not build it.

## Where your work lives
- Stay inside your task's area; widen it with `task_update` (listing the
  existing entries) only for files no live claim covers. Never edit `lib.rs`,
  a parent `mod.rs` or a root event/intent enum: add variants to your
  screen's own sub-enum; a change needed elsewhere goes into the handover.
- A new dependency: a separate commit with `Cargo.toml` + `Cargo.lock`, named
  in the handover.
- Do not commit `docs/rtwk-port/refs/codex-registry.csv` or
  `parity/delta-kinds.md`: the orchestrator rebuilds them.
- Commit on your worktree branch, staging by path, messages `<alias>: …`.
  Never merge (not the integration branch, not a sibling's branch); rebase
  onto the integration tip only when the orchestrator asks. Never push.
- Never accept or reject in tm, your own work included.
- Read the `notes` tm hands you on each call before your next edit.

## Handover
`task_done` with deliverables: the commits, the parity pack path, and a
result summary (what was done vs the task, oracle behaviours covered and the
ones left with reasons, tests and their bites, deltas and NEW kinds, rtwk
gaps, risks, what others must change). A fix round answers each review
finding by `path:line` and commit.
