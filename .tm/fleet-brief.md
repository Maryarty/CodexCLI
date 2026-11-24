# Rules of the CodexCLI fleet ({FORK})

Shared by every agent type. Your process (start, checks, handover) is in your
agent file, not here.

## The port
- `codex-rs/tui` is the oracle (commit in `docs/rtwk-port/ORACLE_REF`);
  `codex-rs/tui-rtwk` matches its look and behaviour exactly. Design:
  `docs/rtwk-port/architecture.md`.
- Build like a Flutter developer: pick the widget, controller,
  Focus/Shortcuts/Actions, LayoutBuilder or dialog you would use in Flutter,
  then its rtwk counterpart. Pages, modes, sheets, widgets: architecture.md §1.
- rtwk widgets and controllers own scroll, selection, paging, editing, tabs,
  focus, mouse, widths and ellipsis. The app picks which widget or spelling
  from a measured width (`LayoutBuilder` + `segmenter_of(cx)`).
- Rows with a cursor: `widgets/session_list.rs`; numbered options:
  `widgets/options.rs`; cursor and scroll live in `ListController`,
  `SelectionController`, `ScrollController`.
- rtwk API: `docs/rtwk-port/refs/` (`rtwk-in-one-screen.md`), the rtwk skill
  (`/Users/admin/Documents/dev_projects/CodexCLI/.claude/skills/rtwk`), the
  rtwk-gallery pages.
- What rtwk cannot do: one line in `docs/rtwk-port/rtwk-gaps.md` (the Flutter
  way, the rtwk widget tried, what differs) and the test `pending(Gxx)`.
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

## Architecture
- Core (`codex-tui-core`): models, `apply`, protocol mapping, command parsing.
  UI: widgets, `strings.rs`, debug entries. `shell`/host: effects and IO.
- ADR-003: `CodexApp` owns one `ThreadModel` per thread, changed only through
  `apply`; thread RPCs live in `ThreadFx`; a screen asks the app (`OpenThread(id)`).
- Mock and fixture state: a JSONL event script applied through core `apply`.
- ADR-013 (architecture.md "ADR-013", full text
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
  routes of a nested `Navigator` in `IntrinsicHeight` (no own page stack).
  Code you touch in an old family keeps its shape unless your task is its
  migration step.
- Test roots `tests/{screens,pages,oracle,shell}.rs` are generated from the files
  (`test-mods/build.rs`): a file of `tests/{pages,sheets,app}/` runs under
  `--test screens` unless it starts with `// test root: pages`; no hand `mod` lines.
- The rules tests (`--test rules`) check all of this.

## Done
1. The feature works in the real binary beside the oracle (verified in the
   batch; you name the pty scenario). No visual deltas accepted.
2. Its key oracle tests (the feature's main forms, a handful) are green in the
   batch. The rest stay `pending` with a reason and are named in the
   deliverable as "sweep: <shard>::<test>..."; a filler batch un-pends them.
3. Mouse on every selectable view (tap, double tap, wheel) with rtwk_test tests.
4. No new framework doors (AppCx/Feature hooks, kit options) unless the
   feature cannot be built without one: then hold and ask the orchestrator.

## Files
- Your task's area only; a Need for another file goes into your checkpoint.
- A new dependency: its own commit with `Cargo.toml` + `Cargo.lock`.
- `codex-rs/tui/**`, `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`,
  `refs/codex-registry.csv`, `parity/delta-kinds.md` belong to the
  orchestrator.

## macOS
- No `timeout`: use `~/.cache/rtwk/bgrun.sh run|wait` or the Bash tool's timeout.
- `sed -i ''` (BSD); prefer Edit or `python3 -I` with str.replace.
- One cargo at a time per worktree.
