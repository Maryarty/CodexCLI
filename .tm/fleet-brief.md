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
- ADR-013 (accepted 2026-11-23; architecture.md "ADR-013", full text
  docs/rtwk-port/research/arch-boundary/full/architecture.md §1.3, §2, §6):
  a value lives in core only if IO needs it — server/host facts, request
  records and op status by a DOMAIN key, lifecycle steps and one-shot domain
  signals (`SurfaceWish`, `Directive`), interest counters. Everything drawn —
  cursor, selection, scroll, tab, sheet page stack, expanded, density, open,
  field text before submit — is UI state: `State` of the widget, a rtwk
  controller, or `ThreadUi`. Core intents are verbs, never gestures (no new
  `Choose`/`Toggle`/`Cursor`/`Expand`...). No new `Menu`/`Pick`/`Prompt`,
  `FeatureView` or `AppCx` hooks: a feature's visible half is
  `src/features/<f>/` with `open(cx, args)` and its own sheet; sheet pages are
  a `SheetStack` in `State`. Code you touch in an old family keeps its shape
  unless your task is its migration step.
- The rules tests (`--test rules`) check all of this.

## Done (the person, 2026-11-23: function first, the full test sweep after)
1. The feature works in the real binary beside the oracle (pty scenario or
   check.py run); differing rows in the pack. No visual deltas accepted.
2. Its KEY oracle tests green: the ones that show the feature's main forms
   (a handful, not the whole family). The rest of its oracle tests stay
   `pending` with a reason and are named in the deliverable as
   "sweep: <shard>::<test>..." — a filler batch un-pends them later. Do not
   spend a round chasing a snapshot of a secondary form.
3. Mouse on every selectable view (tap, double tap, wheel) with rtwk_test tests.
4. Rules tests green. Run scoped checks only (your crate's tests, your
   oracle shard, `--test rules`, fmt, clippy on what you touched): the full
   land-check is run ONCE per landing by the orchestrator on the merge
   candidate (the person, 2026-11-23). Do not wait for it; hand in.
5. No new framework doors (AppCx/Feature hooks, kit options) unless the
   feature cannot be built without one: then hold and ask the orchestrator.

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
