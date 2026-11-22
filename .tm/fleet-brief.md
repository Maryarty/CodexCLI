# Rules of the CodexCLI fleet ({FORK})

## The port
- `codex-rs/tui` is the oracle (commit in `docs/rtwk-port/ORACLE_REF`);
  `codex-rs/tui-rtwk` matches its look and behaviour exactly.
- Build like a Flutter developer: pick the widget, controller,
  Focus/Shortcuts/Actions, LayoutBuilder or dialog you would use in Flutter,
  then its rtwk counterpart. Pages, modes, sheets, widgets:
  `docs/rtwk-port/architecture.md` §1.
- rtwk widgets and controllers own scroll, selection, paging, editing, tabs,
  focus, mouse, widths and ellipsis. The app picks which widget or spelling
  from a measured width (`LayoutBuilder` + `segmenter_of(cx)`).
- Rows with a cursor: `widgets/session_list.rs`; numbered options:
  `widgets/options.rs`; cursor and scroll live in `ListController`,
  `SelectionController`, `ScrollController`.
- rtwk API: `docs/rtwk-port/refs/` (grep it), the rtwk skill
  (`/Users/admin/Documents/dev_projects/CodexCLI/.claude/skills/rtwk`), the
  rtwk-gallery pages.
- What rtwk cannot do after you ran the widget against the oracle's rows: one
  line in `docs/rtwk-port/rtwk-gaps.md` (the Flutter way, the rtwk widget
  tried, what differs) and the test `pending(Gxx)`.
- The oracle's files, snapshots and tests for a screen:
  `docs/rtwk-port/refs/codex-registry.csv`. Oracle binary:
  `~/.cache/codexcli/oracle-bin/`.

## Where things are (core; look each name up with `sym`)
- A server notification enters `threads::project` / `project_unrouted`
  (core/src/threads.rs) or a screen's own `project` (e.g.
  core/src/screens/chat.rs), becomes a `ThreadEv`, and lands in
  `threads::apply` (thread state) or `app::apply_app` / `apply_thread`
  (core/src/app.rs).
- Asks going out: `Command` and `AppFx` (app.rs), `ThreadFx` (threads.rs);
  the host answers them in host/src (`effects.rs`, `facts.rs`).
- Protocol types: `sym --proto Name`. Oracle code: `sym --oracle Name`.
- A core state test to copy: core/tests/status_facts.rs (JSONL script steps:
  the header of core/src/fixtures/script.rs; `Sandbox` in fixtures/sandbox.rs).
- Oracle shard tests: tests/oracle/<shard>.rs, run by
  `python3 -I tests/oracle/tools/summary.py --run <shard>`.

## Architecture
- Core (`codex-tui-core`): models, `apply`, protocol mapping, command parsing.
- UI: widgets, `strings.rs`, debug entries; UI-only state in `UiApp.ui`.
- `shell`/host: effects and IO.
- ADR-003: `CodexApp` owns one `ThreadModel` per thread, one writer, changed
  only through `apply`; thread RPCs live in `ThreadFx`; a screen asks the app
  (`OpenThread(id)`).
- Mock and fixture state: a JSONL event script applied through core `apply`.
- The rules tests (`--test rules`) check all of this.

## Done
1. The feature's oracle tests green, or `pending(Gxx)` on an open gap row.
2. Mouse on every selectable view (tap, double tap, wheel) with rtwk_test tests.
3. Rules tests green.
4. Ran in the real binary beside the oracle; differing rows in the pack.

## Files
- Your task's area only; a Need for another file goes into your checkpoint.
- A new dependency: its own commit with `Cargo.toml` + `Cargo.lock`.
- `codex-rs/tui/**`, `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`,
  `refs/codex-registry.csv`, `parity/delta-kinds.md` belong to the
  orchestrator.

## macOS
- `timeout`: use `bgrun.sh run|wait` or the Bash tool's `timeout`.
- `sed -i ''` (BSD); prefer Edit or `python3 -I` with str.replace.
- nextest: `--cargo-quiet`.
- One cargo at a time per worktree.
