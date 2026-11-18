# CodexCLI — agent map

A fork of openai/codex (`upstream`). The work here is the tui-rtwk port: the
Codex terminal UI `codex-rs/tui` (ratatui, the oracle — not edited) carried one
to one onto rtwk (`/Users/admin/Documents/dev_projects/rtwk`) as
`codex-rs/tui-rtwk`. Design: `docs/rtwk-port/architecture.md`; research:
`docs/rtwk-port/research/`; rtwk API references: `docs/rtwk-port/refs/`.

## What we build
Screen first: the screens of the screen map (architecture.md §1) built as a
native rtwk app in the manner of `/Users/admin/Documents/dev_projects/rtwk-weather`.
codex-tui's reusable code (reuse-map.md REUSE/ADAPT rows: protocol and
session helpers, keymap, chunking, clipboard, markdown_copy, analytics data,
...) is carried over into `tui-rtwk`; its UI code is not — screens are built
of rtwk widgets. codex-tui itself is never edited. Backend crates are used
as they are.

## rtwk in one screen
- The user's framework for terminal apps in Rust, built the way Flutter is:
  immutable widgets describe the UI; the framework inflates them into
  elements (which keep `State` across rebuilds) and render objects (layout
  by constraints, paint into a cell surface); a change of state rebuilds
  only the affected subtree, and the frame's dirty cells are diffed onto
  the terminal. Runs full screen (alt screen) or inline. ~340k lines, 25
  crates, facade `rtwk::…`. Docs: rtwk `README.md` (crate table),
  `docs/ARCHITECTURE.md`, `docs/guide/state.md`; a whole app to copy the
  manner from: `/Users/admin/Documents/dev_projects/rtwk-weather`.
- Layers: `rtwk-unicode` → `rtwk-input` → `rtwk-core` (cells, styles,
  damage spans, presenter for full screen / inline / fixed viewport, frame
  loop, terminal I/O) → `rtwk-render` (retained render tree: constraints
  layout, paint, hit test) → `rtwk-framework` (widgets, elements, `State`
  with Flutter's lifecycle, keys, inherited widgets, `Obs<T>`, focus) →
  widget crates `rtwk-widgets`, `-scroll` (ListView, virtualized),
  `-editing` (TextField, TextArea), `-overlay` (dialogs, popovers,
  CommandPalette), `-controls`, `-data` (charts, tables, trees),
  `-animation`, `-provider`, `-hooks`, `-cli`; extras `rtwk-rich` (streamed
  Markdown, highlight, diff, images), `rtwk-term` (VT emulator + pty),
  `rtwk-tokio`, `rtwk-state` (adapters to external stores), `rtwk-test`
  (the app on an in-memory terminal under a fake clock), `rtwk-e2e`.
- rtwk owns the frame loop, terminal, input, widget state (scroll, focus,
  editing, selection, animation), rebuild/layout/paint, frame pacing,
  coalescing of work from other threads, list virtualization. The app never
  re-implements any of it.
- An app (`crates/rtwk/src/app.rs`): `run_app_with(state, view, RunOptions)`;
  the model is a plain struct `A`; `view(&A, &mut ViewCx<A>) -> AnyWidget`
  builds the tree; `cx.on(|a, cx| ..)` / `cx.on_with` handlers change `A` and
  rerun the view; `ViewCx::memo(deps, build)` reuses a subtree while its deps
  compare equal; `AppCx::spawn(future, then)` runs work and applies its
  result; `AppCx::channel(f)` takes items from any thread, applied in order
  at the next flush with one view for all. Navigation `rtwk::navigation`,
  dialogs `overlay::show_dialog`. Tests: `rtwk_test::Tester` over
  `overlay::App::new(AppView::new(app, view))`.
- App state above this (ADR-003) is the Rust counterpart of what
  bloc/redux/riverpod are in Flutter. rtwk's own tools for it: the plain
  model + `ViewCx::memo`, `Obs<T>`, `rtwk::provider` (read/watch/select),
  and `rtwk-state` (`Source`/`Select`/`store_provider` over an external
  store, optional reactive_graph).

## Agents and tokens
- Implementation: agent type `codex-porter` (Sonnet by default; a model override
  is allowed — the person removed the guard hook 2026-11-18). Reviews and research: `codex-reviewer` (Sonnet; `model: opus`
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
