# rtwk in one screen

Moved from AGENTS.md on 2026-11-23 (agents read it on demand; the rtwk skill
`.claude/skills/rtwk` and `docs/rtwk-port/refs/` hold the API).

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

