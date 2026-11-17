# Codex TUI on rtwk — architecture

tm: `tui-rtwk` (root), this document is the deliverable of `port-design`.
Oracle: `codex-rs/tui` at the commit in [ORACLE_REF](ORACLE_REF). rtwk: the
checkout `/Users/admin/Documents/dev_projects/rtwk` at [RTWK_REF](RTWK_REF).
Goal for now: one to one in functionality with the oracle. New features and
redesign come after parity.

Research behind it (read by section, not whole):
[codex-tui-surface.md](research/codex-tui-surface.md) and
[codex-tui-features.csv](research/codex-tui-features.csv) (what the UI does),
[codex-tui-architecture.md](research/codex-tui-architecture.md) (how it is
built), [rtwk-for-codex.md](research/rtwk-for-codex.md) (what rtwk offers),
[state-management.md](research/state-management.md) (state options).

## 0. Decisions at a glance

| # | decision | in one line |
|---|---|---|
| ADR-001 | code organization | a framework-neutral frontend seam inside `codex-tui` (feature `frontend-api`, additive files), all rtwk code in a new crate `codex-rs/tui-rtwk` |
| ADR-002 | who owns the loop | rtwk owns the frame loop; the Codex controller runs as an async task on the UI thread (provisional until ADR-003) |
| ADR-003 | state management | **open** — chosen among Rust-world solutions by the round-2 research (tm `state-research`), not by analogy with another ecosystem |
| ADR-004 | content model | cells keep producing `ratatui-core` `Line`/`Span`/`Style` (data only); one rtwk widget paints them |
| ADR-005 | screen modes | owned alt-screen mode first (Codex's default), inline scrollback mode second |
| ADR-006 | input | rtwk decodes; keys reach the controller as crossterm `KeyEvent` values (types only) |
| ADR-007 | parity oracle | the oracle's 1,549 text snapshots must pass unchanged when rendered through rtwk; plus a cell-and-style diff harness and vt100 scenarios |
| ADR-008 | references | generated, under `docs/rtwk-port/refs/`, regenerated per pin, checked fresh |
| ADR-009 | migration | strangler: run on rtwk early through a bridge that paints legacy ratatui buffers, then replace region by region |

## 1. What we are porting (facts)

- `codex-rs/tui`: 529k lines in 1,268 files, ~259k production lines, 6,329
  tests, 1,549 plain-text insta snapshots. Biggest: `app/` 132k,
  `chatwidget/` 93k, `bottom_pane/` 85k, `transcript_view/` 20k.
- Built as a **mutable component tree + one unbounded mpsc bus** (`AppEvent`,
  335 variants) + one `select!` loop in `App::run`; rendering is
  immediate-mode (`render(&self, Rect, &mut Buffer)`), whole frame per Draw,
  diffed by `custom_terminal`, frames coalesced at ≤120 fps by
  `FrameRequester`.
- The controller is **async**: `app/` alone has 320 `async fn` and 1,111
  `.await`; handlers await RPCs inline (`App::handle_event`,
  app/event_dispatch.rs:35).
- The backend sits fully behind `codex_app_server_client` +
  `codex_app_server_protocol`: the port is a pure frontend swap.
- Two screen pipelines: **owned** (alt-screen + mouse, `TranscriptView`, the
  default: `tui.fullscreen_transcript` defaults to true) and **inline**
  (viewport + finished history written into terminal scrollback with DECSTBM,
  rebuilt from source cells on resize).
- ~45% of production lines have no ratatui/crossterm at all; ~25% more touch
  only `Line`/`Span`/`Style`/`KeyEvent`; ~30% is rendering and terminal code.
- Natural seams: `AppServerSession`, `AppEvent`/`AppCommand`,
  `HistoryCell::display_lines(width)`, `Renderable{render, desired_height}`,
  `StreamController` + chunking, the keymap runtime, `TuiEvent`.

## 2. rtwk in Flutter terms

rtwk is Flutter for the terminal, in Rust: the framework owns the frame loop,
the UI is a retained tree of widgets, elements and render objects rebuilt from
state, layout by constraints, painting into a cell surface diffed onto the
terminal.

| Flutter | rtwk | in the port |
|---|---|---|
| engine + `runApp` | `run_app_with(state, view, RunOptions{mode, ..})`, `AppHost::pump` | `codex-rtwk` main; `ScreenMode::FullScreen` (owned) or `Inline` |
| `StatelessWidget` / `StatefulWidget` + `State` | `AnyWidget`, `StatefulWidget`, `State`, `Cx` | widgets per region; local state = scroll, hover, animation phase |
| `InheritedWidget`, Provider | `rtwk::provider` (`read`, `watch`, `select`) | theme, keymap, config projections |
| (external store) | `rtwk_state::store_provider`, `Select` over a `Source<R>` | how whatever ADR-003 picks reaches the widgets |
| `Future` / `Stream` / `StreamBuilder` | `cx.spawn`, `rtwk-tokio` `subscribe`, hooks `use_stream` | backend I/O on tokio, the controller task |
| `CustomPainter` | `CustomPaint` + `CellPainter` (rtwk-widgets/layout/custom_paint.rs) | the legacy bridge (ADR-009) |
| `Navigator`, `showDialog`, `Overlay` | rtwk-overlay | bottom-pane view stack, popups, pager overlays |
| `ListView.builder`, `Scrollable` | rtwk-scroll | the owned transcript |
| widget tests, goldens | `rtwk-test` `Tester`, insta | parity tests |

## ADR-001 — Code organization and upstream sync

**Decision.** Two parts:

1. **`codex-rs/tui`, feature `frontend-api` (off by default).** Additive
   files only, plus one `mod` line per parent:
   - `src/frontend/` — the seam: a `FrontendHost` trait (event sources in,
     view models out, no rtwk types), the controller's run loop for a host
     (a sibling of `App::run`, which stays as it is), the exported
     view-model types.
   - per component, a child module `<component>/view_model.rs` declared as
     `#[cfg(feature = "frontend-api")] mod view_model;` — a child module sees
     the component's private fields, so projections need no visibility
     changes in upstream code.
   Nothing in `codex-tui` depends on rtwk; the default build is upstream's.
2. **`codex-rs/tui-rtwk` (crate `codex-tui-rtwk`, binary `codex-rtwk`).** The
   rtwk host, widgets, bridge, parity tests. Depends on `codex-tui` with
   `frontend-api` and on rtwk by path (`../../../rtwk/crates/...`), the
   commit pinned in `RTWK_REF` and checked by `tools/rtwk-port/pins.py`.

**Why.** Upstream changes `codex-rs/tui` daily. Logic stays shared and
live: an upstream merge brings controller changes to both frontends at once,
and conflicts are limited to `mod` lines. A copy of the crate would need every
upstream change forwarded by patch; editing render code in place would
conflict on every merge. The neutral seam is also a test asset: the
controller can be driven headlessly with a fake host.

**Repin** (moving `ORACLE_REF`): merge upstream, rebuild `frontend-api` (a
compile error in a `view_model.rs` names the component that changed), rerun
the parity suite, regenerate the registry (ADR-008), file the diffs as tasks.

**Alternatives.** Copy-and-own `codex-tui` into a new crate (simpler
privacy, costly repins); dual renderer inside `codex-tui` (rtwk inside the
upstream crate, feature unification turns the default binary). Rejected for
the reasons above.

## ADR-002 — rtwk owns the loop; the controller is a task

**Decision.** As in Flutter, the framework owns the main thread: rtwk's frame
loop reads input, runs timers and futures, builds, lays out, paints and
presents. Codex's controller — today's `App::run` minus the terminal — runs
as one long-lived async task on rtwk's UI thread (`cx.spawn` runs `!Send`
futures there), with a tokio context entered (`rtwk-tokio`) so its tokio
timers, channels and RPC futures work; backend I/O stays on tokio's worker
threads as today.

- `select!` branches become sources the task awaits as today: the
  `AppEvent` channel, backend events, per-thread buffers, timers. Terminal
  input arrives as `TuiEvent`s produced by the rtwk host (ADR-006).
- A handler that awaits an RPC holds `&mut` controller state across the
  await, as today. Nobody else borrows that state: the UI reads only what
  the controller published (ADR-003). So the 1,111 awaits stay verbatim, and
  unlike today the UI keeps drawing animations and accepting input while a
  handler waits.
- `FrameRequester::schedule_frame()` becomes "publish": the controller
  updates its view models and rtwk decides when the frame is drawn.
  `handle_draw_pre_render` work (resize reflow) runs on publish with the
  width the host reported.

**Alternatives.** Codex's loop drives rtwk per Draw (max fidelity, but rtwk
reduced to a painter and no public API for an externally driven tree in
rtwk). Whether the controller keeps its async handlers or is re-cut into
synchronous updates with effects is ADR-003's question; this ADR is revisited
with it. A controller on a tokio worker
(`Send` everywhere, locks across awaits) is the fallback if the UI-thread task
fails the skeleton's spike.

## ADR-003 — State management (OPEN)

**Status.** Open. Round 1 (research/state-management.md) recommended an
Elm-style single store with effects as data, followed through rtwk-state,
but assumed handler bodies move into a synchronous `update`; the Codex
controller has 320 `async fn` and 1,111 `.await` in `app/` alone. Round 2
(tm `state-research`, research/state-v2.md) compares Rust-world solutions —
TEA with async tasks (iced, relm4, tui-realm), Crux, fine-grained signals
(reactive_graph, Dioxus signals, futures-signals, Xilem), Redux/Flux stores,
tokio actors (ractor, kameo), incremental helpers (salsa), ECS — on scored
criteria with prototypes of the finalists, and the person decides.

The person's criteria (2026-11-17): Codex's state handling today is
home-grown and mixes state logic with UI; the goal is the best standard Rust
solution with a clean separation of state/logic from UI that fits rtwk.
Restructuring the controller is acceptable — the port may carry Codex's
behaviour rather than its code. ADR-001 (logic shared in codex-tui behind a
seam) and ADR-002 are therefore revisited together with ADR-003.

What holds whatever wins: rtwk imposes no state manager and an external
store reaches widgets through `rtwk_state::Source` / `Select` /
`store_provider` and `Obs` without core changes (rtwk docs/guide/state.md).
The draft below was written before round 2 and is kept only as one
candidate's shape (an async controller publishing immutable view models);
it is not the decision.

**Draft (superseded by round 2).** The Codex controller is the store (one
owner, a message enum, effects as async work). Concretely:

- **Events** in: `AppEvent` (kept verbatim, 335 variants), backend
  notifications, `TuiEvent`s. Widgets dispatch with the existing
  `AppEventSender` (`Clone`, any thread) — Codex's popups already return
  results by `Fn(&AppEventSender)` closures, which is the Bloc `add`.
- **State** out: immutable view models, published by the controller at its
  frame points. Each component's `view_model.rs` projects its private state
  into plain data (`Arc` snapshots, revision counters, `PartialEq` cheap):
  `TranscriptModel { rev, cells: Arc<[Arc<dyn HistoryCell>]> }`,
  `ComposerModel`, `BottomPaneModel { views: Vec<ViewModel> }`,
  `StatusModel`, ... A component recomputes its model only when its revision
  moved.
- **Follow**: the published models live in a small store (`ModelStore`,
  `Rc`, UI thread) that implements `rtwk_state::Source<R>` per slice; regions
  use `Select` (BlocSelector) and `store_provider` (BlocProvider) so a token
  burst rebuilds only the live tail. Hot values with one reader (spinner
  phase, elapsed timer) use `Obs<T>`.
- **Local widget state** (rtwk `State`): only what Codex itself treats as
  presentation and no handler reads — hover, animation phase, layout caches.
  Everything Codex's handlers read stays in the controller: composer text
  and cursor, picker selection, transcript scroll and selection, copy mode.
  Moving those into rtwk widgets is post-parity work.

Measured in rtwk for any external store: `Select` / `store_provider` give 1
wake and 1 rebuild per 100-write burst (docs/guide/state.md).

## ADR-004 — Content model: ratatui-core text as data

**Decision.** History cells, markdown, diffs, exec output, status lines keep
producing `ratatui-core` `Line`/`Span`/`Style` through their existing code
(`display_lines(width)`, `markdown_render`, `diff_render`, `wrapping`,
syntect highlighting). `tui-rtwk` paints them with one widget, `LinesView`
(style mapping `ratatui-core` → `rtwk::style`, OSC 8 links preserved), sized
by `desired_height(width)` from a `LayoutBuilder`.

**Why.** Byte-for-byte equal lines are what makes 1:1 cheap: ~51 cell kinds
and the markdown renderer port as one widget instead of fifty. `ratatui-core`
is a small data crate; the ratatui renderer, `Terminal` and crossterm I/O
leave the rtwk build. rtwk-rich's `MarkdownView` / `DiffView` would render
differently from Codex; adopting them is a post-parity design choice.

## ADR-005 — Screen modes

**Decision.** Owned (alt-screen, `ScreenMode::FullScreen`, mouse) first: it
is Codex's default and fits a retained tree (the transcript is an rtwk
scroll view over `LinesView` items, with Codex's anchors, selection, search
and copy mode kept in the controller). Inline mode second: rtwk's
`ScreenMode::Inline` with finished cells committed through `Static` /
`Screen::commit`.

Inline needs three things rtwk lacks today (§ rtwk gaps): entering the
alternate screen from an inline app for overlays (Ctrl+T pager, diff), re-
emitting the app's own scrollback on resize (Codex rebuilds it from source
cells, app/resize_reflow.rs), and Zellij / Windows Terminal scrollback
strategies (tui/scrollback.rs). Standalone pre-App screens (onboarding,
update prompt, cwd prompt, model migration, OSS selection) run as their own
small rtwk apps before the main one, as they run their own draw loops today.

## ADR-006 — Input and keys

**Decision.** rtwk-input decodes the terminal (kitty keyboard flags,
bracketed paste, mouse 1000/1002/1003/1006, focus, resize, colour replies).
The host converts each event into Codex's `TuiEvent` with crossterm
`KeyEvent` / `MouseEvent` values (crossterm used for its types; no crossterm
I/O). Codex's routing — the ~500 lines of pre-filters in
`App::handle_tui_event`, chord and leader keymaps, vim mode, Windows key
repair — runs unchanged. rtwk's own `Focus`/`KeyBindings` are used only for
widgets that own their keys (none in the parity phase except mouse
hover/scroll on presentation-only regions). Ctrl+Z stays reserved; the host
uses `App::suspend_to_shell`. External editors use `App::suspend`.

## ADR-007 — Parity oracle and tests

1. **Snapshots unchanged.** Codex's snapshot helpers (`snapshot_buffer`,
   `render_bottom_popup`, `buffer_to_text`, `snapshot_lines`, ...) get an rtwk
   twin in `tui-rtwk`: same fixture → view model → rtwk widget rendered
   headless by `rtwk-test` at the same size → text. The `.snap` file of the
   oracle is the expected value, read from `codex-rs/tui/src/**/snapshots/`
   at `ORACLE_REF`. A region is ported when its oracle snapshots pass
   through rtwk.
2. **Cell diff.** For every ported component, a test renders the same view
   model through the oracle's ratatui `render` and through rtwk and compares
   every cell: symbol, fg, bg, modifiers, link. Snapshots are text-only;
   this catches style drift.
3. **Scenarios.** Codex's vt100 suite (`tests/suite/`: history, live commit,
   resize reflow, reconnect, status indicator, ...) ported as rtwk-e2e /
   pty scenarios driven by a scripted backend (recorded app-server
   notifications) and a key script, comparing screen and scrollback.
4. **Controller tests** stay in `codex-tui` and keep running: the logic is
   shared, so they cover both frontends.
5. **Performance**: a Codex-shaped benchmark (token stream at 100–1,000
   deltas/s into a 1,000-cell transcript, a 10k-line exec output, resize
   storm) with p50/p95/p99 and `uptime` load, rtwk vs the oracle.

## ADR-008 — Generated references

Porting agents read references, not framework source. All live in
`docs/rtwk-port/refs/`, are written by `tools/rtwk-port/*.py`, regenerate
when a pin moves and fail a `check` when stale.

| reference | source of truth | generator | size target |
|---|---|---|---|
| `rtwk-api/INDEX.md` + `rtwk-api/<crate>.txt`: one line per public item, `path · signature · first doc sentence · file:line` | rustdoc JSON of rtwk at `RTWK_REF` | extend rtwk's `tools/parity/rust_api.py` walk with signatures and doc lines | INDEX ≤150 lines, ≤4k lines total for the 12 crates the port uses |
| `ratatui-to-rtwk.md`: Frame, Buffer, Rect, Layout, Paragraph, Block, List, Scrollbar, Style, Line, Span, crossterm events, `Terminal::insert_before` → rtwk items | curated TOML | checker validates every rtwk path against the index | ~150 lines |
| `codex-registry.csv`: every component, `Renderable`, `BottomPaneView`, `HistoryCell`, overlay, onboarding step, slash command, `AppEvent` variant and snapshot of the oracle, with `file:line`, size, snapshot count and port status (`bridged` / `native` / `n/a`, read from markers in `tui-rtwk`) | oracle source at `ORACLE_REF` + `tui-rtwk` | `inventory.py` (seeded by the research generator) | one row per item |
| `feature-map.md`: Codex need → rtwk item → example → gap | capability matrix (research/rtwk-for-codex.md §2) | curated TOML + index check | ~120 lines |
| `recipes.md`: inline `Static` commits, live tail, `Select` over a store, `CustomPaint`, tokio feed, suspend, headless test | rtwk examples and doctests | excerpt script | ~200 lines |

## ADR-009 — Migration: strangler with a bridge

1. **Run on rtwk first.** The host (ADR-002/006) plus a **bridge**: a
   `CustomPaint` whose `CellPainter` paints a ratatui `Buffer` produced by
   the oracle's `render` of a region at the size rtwk laid out. With the
   whole frame bridged, Codex runs on rtwk's terminal, loop and input with
   every screen intact (milestone M1).
2. **Replace region by region**, each with its snapshots and cell diff
   green: history cells and transcript (one `LinesView`), the
   `Renderable` combinators (`ColumnRenderable`, `FlexRenderable`,
   `InsetRenderable`), the generic `ListSelectionView` (211 popups use it),
   the composer and its popups, footer and status line, approvals, the
   remaining `BottomPaneView`s, overlays, onboarding and pickers, then the
   large fork-specific areas (analytics, agents overview, sidebar and side
   panes, pets and images, voice, `ui_tree`).
3. **Inline mode** after the owned mode reaches parity and the rtwk gaps
   land.
4. **Remove the bridge**: no `ratatui::Buffer` in `tui-rtwk`; parity sweep
   over the registry; performance gate; the `codex` binary gets a
   `--frontend rtwk` switch (or ships `codex-rtwk`) — decided then.

## Skeleton (tm `port-skeleton`)

```
codex-rs/tui/src/frontend/         feature frontend-api (no rtwk dependency)
  mod.rs        FrontendHost trait, TuiEvent source, publish sink, run_with_host (stub)
  model.rs      the first view models (TranscriptModel over HistoryCell)
codex-rs/tui-rtwk/
  Cargo.toml    codex-tui {features = [frontend-api]}, rtwk crates by path, ratatui-core
  src/lib.rs
  src/host.rs   rtwk app: screen mode, input → TuiEvent, frame on publish
  (state integration: after ADR-003 is decided, not in the skeleton)
  src/convert.rs ratatui-core Style/Line/Span → rtwk; rtwk key → crossterm KeyEvent
  src/bridge.rs  CellPainter over a ratatui Buffer
  src/views/lines.rs LinesView
  src/bin/codex-rtwk.rs  demo: an rtwk app showing bridged and native cells
  tests/parity.rs  one oracle snapshot passing through rtwk; one cell diff
```

The skeleton also answers ADR-002's spike: a long-lived `!Send` controller
task on rtwk's UI thread awaiting a tokio channel and a tokio interval while
the UI keeps drawing.

## rtwk gaps (work in the rtwk repository, tracked here)

| gap | needed by | first wave it blocks |
|---|---|---|
| G1 enter / leave the alternate screen at runtime from an inline app | inline overlays (pager, diff) | inline |
| G2 re-emit the app's own committed scrollback (clear + reprint, row cap) | inline resize reflow | inline |
| G3 a public, tested way to host a long-lived `!Send` task with a tokio context on the UI thread | ADR-002 | M1 (spike in the skeleton first) |
| G4 image placements (kitty, sixel, iTerm2) inside a region, if rtwk-rich's image widget does not cover Codex's | pets, `view_image`, visualizations | late |
| G5 Zellij and Windows Terminal scrollback strategies | inline on those terminals | inline |

## Risks

1. ADR-002's UI-thread controller task is unproven in rtwk (G3). Fallback:
   controller on a tokio worker, store behind `Arc`, same `Source` API.
2. View-model extraction touches every component's private state; upstream
   refactors break it at merge time (loud, but frequent).
3. Owned-mode transcript performance with 10³–10⁴ cells in a retained tree;
   the 4.7 ms/token live-table case in rtwk is the known trap — keep live
   regions small, commit settled cells.
4. Inline mode depends on G1/G2/G5 landing in rtwk.
5. Fork-specific areas (sidebar, agents overview, ui_tree, analytics) are
   large and lightly documented upstream.
6. Windows: rtwk's Windows evidence is partial.

## Open decisions for the person

0. ADR-003: the state-management solution, after the round-2 research.
1. ADR-001: seam in `codex-tui` + separate crate (recommended) vs a full
   copy of `codex-tui`.
2. ADR-005: owned mode first (recommended) vs inline first.
3. ADR-009 end state: `--frontend rtwk` switch in `codex` vs a separate
   `codex-rtwk` binary.
4. Whether the rtwk gaps G1–G5 go into rtwk's own tm plan now.
