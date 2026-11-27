# Codex TUI on rtwk — architecture

tm: `tui-rtwk` (root); this document is the deliverable of `port-design`.
Oracle: `codex-rs/tui` at [ORACLE_REF](ORACLE_REF), never edited. rtwk: the
checkout `/Users/admin/Documents/dev_projects/rtwk` at [RTWK_REF](RTWK_REF).
The target architecture drawn: [design.md](design.md) /
[design.html](design.html) (regenerate with `python3 -I docs/rtwk-port/design_html.py`).
Phase 1: one to one in functionality with the oracle. Phase 2: Codex Desktop
features ([desktop-gap.md](research/desktop-gap.md)). Phase 3: our own.

Research (read by section): what the UI does —
[codex-tui-surface.md](research/codex-tui-surface.md),
[codex-tui-features.csv](research/codex-tui-features.csv) (312 rows); how it is
built — [codex-tui-architecture.md](research/codex-tui-architecture.md); what
rtwk offers — [rtwk-for-codex.md](research/rtwk-for-codex.md),
[reuse-map.md](research/reuse-map.md); app state —
[state-survey.md](research/state-survey.md), [app-state.md](research/app-state.md),
[app-state-aopt.md](research/app-state-aopt.md),
[off-thread-state.md](research/off-thread-state.md);
bugs that go away — [bug-classes.md](research/bug-classes.md); tests —
[test-infra.md](research/test-infra.md); performance —
[perf-compare.md](research/perf-compare.md).

## 0. Direction and decisions

**Screen first.** We build the screens of the screen map (§1) as a native
rtwk app, the way a Flutter app is built. codex-tui's reusable code is
carried over (copied) into `tui-rtwk`: the rows that
[reuse-map.md](research/reuse-map.md) marks REUSE (protocol seam
`app_server_session`/`app_command`, keymap, clipboard OS side,
markdown_copy, chunking, analytics data, thread/update/session helpers) and
the non-UI part of ADAPT rows, adjusted to the app model — minus the content
renderers (history_cell, markdown_render, diff_render, wrapping, textarea),
which rtwk widgets replace (ADR-004, ADR-006). A carried file brings its
dependency closure: each module it imports is carried too or replaced by an
app/rtwk counterpart (e.g. `markdown_copy` → `clipboard_copy` and
`terminal_hyperlinks` are replaced; `analytics/data.rs` loses
`FrameRequester`; `keymap` needs `key_hint`, which is re-expressed over rtwk
keys, not crossterm). reuse-map Top-10 #1, #4, #5, #7, #8, #9 are superseded
by this document (arch-review M7). Its UI code is not
carried over, wrapped or bridged: no seam in codex-tui, no ratatui painter;
screens are built of rtwk widgets. codex-tui itself stays untouched as the
oracle: behaviour spec, tests to re-express, snapshots as the visual
reference. The backend is reused as it is (`codex-app-server-client`,
`codex-app-server-protocol`).

| # | decision | in one line |
|---|---|---|
| ADR-001 | crate and pins | `codex-rs/tui-rtwk` (crate `codex-tui-rtwk`), its own Cargo workspace passing codex-rs manifest policy; codex-rs `[patch]` copied, lockfile seeded from codex-rs; rtwk by path at `RTWK_REF` with a pin check; codex-rs workspace, lockfile and Bazel untouched |
| ADR-002 | app structure | a Flutter-style rtwk app in the manner of `rtwk-weather`: model + events + `view(&App, &mut ViewCx)` + `cx.on`; screens via `Navigator` + `PageRoute`, inline views as the chat page's bottom-pane slot (amendment 2026-11-18's core view stack withdrawn by ADR-013: nested `Navigator` in `IntrinsicHeight`, docked `SheetHost`), centered views and dialogs via `show_dialog`, live data through `rtwk::provider` projections of the model — as in twk; rtwk widgets |
| ADR-003 | app state | plain model owned by the rtwk app: one writer, `apply(&mut ThreadModel, ThreadEv, &mut ThreadFx)` per thread and `apply_app(&mut CodexApp, AppEv, &mut AppFx)` for app-scoped state, tickets on replies, derived values via `ViewCx::memo`; per-thread UI controllers in `ThreadUi`, kept in the UI model (`UiApp.ui`), never in the core model; entities+notify as reserve; chosen on per-approach trade-offs |
| ADR-004 | screens and content | 24 screen-map rows (§1) as pushed routes, dialogs and popovers of rtwk widgets; rtwk-rich Markdown/Diff/Code; rtwk-data charts, rtwk-rich Image |
| ADR-005 | screen modes | owned full screen first (the oracle's default), inline last |
| ADR-006 | input | rtwk `Focus` / `Shortcuts` / `KeyBindings`; Codex's keymap config read into them; input never frozen by an RPC |
| ADR-007 | parity | functional (screen tests + pty scenarios against the oracle) and visual (oracle snapshots and captures as the reference, deltas listed per screen) |
| ADR-008 | performance | the pty A/B harness of perf-compare.md, baseline of the original first, report.html + side-by-side casts + milestone trend |
| ADR-009 | migration | foundation → main chat screen → screen slices in parallel → inline mode and rtwk gaps |
| ADR-010 | localization | the app's strings are typed `const` tables (rtwk-l10n's form), one `strings.rs` per screen directory plus `src/l10n/`; English is the oracle's text byte for byte; the locale is negotiated once and reaches the tree as rtwk's `TwkLocalizations`; a rules test bans user-visible literals in widget code |
| ADR-011 | mouse | mouse support is added though codex-tui has none (the person, 2026-11-18): capture on, wheel scrolls, click selects, click on the selected row or double click accepts, tabs and buttons clickable; tested with rtwk_test mouse events |
| ADR-012 | core feature slices | a feature of the core that is a flow, not a screen (`/goal`, `/export`, `/rename`, `/plugins`) is a slice (thread lifecycle state, the queue and the turn starts, stays the thread's own): `core/src/features/<f>/` and `src/features/<f>/`, registered by one line in `feature_list!`, which generates the root enums and their dispatch as `screen_list!` does for screens; it reads the app through `AppView` / `ThreadView` and acts through an `AppCx` outbox, never names `CodexApp` or `ThreadModel`; hooks run in list order; the hub enums only shrink (ratchet) |
| ADR-013 | core/UI boundary | accepted 2026-11-23 (option C): a value lives in core only if IO needs it — server/host facts (Д), request records and op status by domain key (З), lifecycle and one-shot domain signals (Ж), interest counters (И); everything drawn (cursor, tab, sheet page stack, expanded, density, open, field text) is UI state (Н) in `State`/`ThreadUi`; `Feature` loses `Menu/Pick/Prompt`; supersedes ADR-002 amendment 2026-11-18 (core view stack) |

## 1. What we build (facts and the screen map)

- The oracle: ~259k production lines, 6,329 tests, 1,549 text snapshots
  (`app/` 130k, `chatwidget/` 90k, `bottom_pane/` 85k). It is a mutable
  component tree, one `AppEvent` bus and one `select!` loop; one foreground
  task mutates all state, background tasks report over the bus.
- The backend sits fully behind `codex_app_server_client` (InProcess, Remote,
  Cloud) and `codex_app_server_protocol`: the frontend is a pure swap.
- Default screen mode is owned full screen: `tui.fullscreen_transcript`
  is true when unset (`codex-rs/core/src/config/mod.rs:4932`).

Screen map (sources: codex-tui-surface.md §1–§13; ids are the plan's):

| id | screen | kind | oracle |
|---|---|---|---|
| S01 | update prompt | route before main | update_prompt.rs |
| S02 | onboarding: welcome, auth (browser, device code, API key), GovCloud guidance | routes | onboarding/welcome.rs, auth.rs, auth/ |
| S03 | onboarding: provider, gateway, Bedrock, GCP setup | routes | onboarding/gateway*.rs, bedrock.rs, gcp.rs, provider_setup.rs |
| S04 | directory trust | route | onboarding/trust_directory.rs |
| S05 | startup prompts: cwd, model migration, startup hooks, unarchive, OSS provider, daemon recovery, startup error, Windows sandbox | dialogs | cwd_prompt.rs, model_migration.rs, startup_hooks_review.rs, unarchive_prompt.rs, oss_selection.rs, daemon_recovery.rs, startup_error.rs |
| S06 | import from other agents (`/import`) | route | external_agent_config_migration/ |
| S10 | **main chat**: transcript, banners, pending input, working indicator, composer (usable as a startup draft before the session is ready), footer, status line (incl. `ui_tree` elements) | home route | chatwidget/, bottom_pane/, transcript_view/, app/owned_transcript.rs |
| S11 | streaming and history cells (~48 kinds), markdown, diff, exec output | transcript items | history_cell/, streaming/, markdown_render/, diff_render.rs |
| S12 | composer popovers: `/` commands, `@` files, mentions, `$` skills, history search | popovers | command_popup.rs, file_search_popup.rs, mentions_v2/, skill_popup.rs, chat_composer/history_search*.rs |
| S13 | approvals: exec, permissions, patch, MCP elicitation, request-user-input, async questions | dialogs | approval_overlay.rs, mcp elicitation, request_user_input |
| S14 | generic selection dialog (~30 parameterisations: model, permissions, review, worktrees, ...) | dialog | list_selection_view.rs |
| S15 | settings dialogs: `/config`, status line, title, theme, keymap setup/capture/debug, experimental, memories, skills, hooks, provider editor | dialogs | config_panel/, status_line_setup, keymap_setup/, ... |
| S16 | transcript pager (Ctrl+T), static pager (`/diff`), search | route | pager_overlay.rs |
| S17 | backtrack (Esc Esc) | mode of S10 | app_backtrack.rs |
| S18 | resume / fork picker | route | resume_picker/ |
| S19 | `/status` card | dialog | status/card.rs |
| S20 | `/usage` analytics dashboard | route | analytics/ |
| S21 | agents command center, subagent switcher | route | app/agents_overview*.rs, multi_agents.rs |
| S22 | sidebar and side conversation panes | parts of S10 | app/sidebar*.rs, app/side*.rs |
| S23 | small dialogs: shortcut overlay, warnings, daemon menu, app link, custom prompt, user verification, feedback (+ note, consent) | dialogs | bottom_pane/* |
| S24 | pets | overlay | pets/ |
| S25 | voice / realtime strip and settings | part of S10 + dialogs | voice_strip.rs, chatwidget/realtime*.rs |
| S26 | `ui_tree` status-line elements (part of the S10 footer) and inline visualization (an S11 content kind) — not a route | parts of S10/S11 | ui_tree/, inline_visualization.rs |
| S27 | inline mode (screen mode, not a screen) | mode | tui.rs, insert_history.rs, app/resize_reflow.rs |

Behaviour without a screen of its own is carried by the screen that needs
it: slash commands (69, slash_command.rs) by S10/S12, IDE context
(ide_context/) and notifications by S10, keymap runtime by ADR-006.

## ADR-001 — Crate layout and pins

**Decision.** `codex-rs/tui-rtwk` is its own Cargo workspace (`[workspace]`
in its Cargo.toml, its own Cargo.lock), producing the binary `codex-rtwk`.
It depends on codex crates by path (`../app-server-client`,
`../app-server-protocol`, `../core` for config, `../arg0`, test support as
dev-deps) and on rtwk crates by path pinned to `RTWK_REF`. It does not depend
on `codex-tui`.

**Layout inside the crate (skeleton-layers).** The workspace has three
members: `core/` (package `codex-tui-core`: ids, RPC tickets, the app and
thread models with `apply`, the protocol mapping, each screen's model, events,
intents and RPCs, the fixture scripts; it depends on `codex-app-server-protocol`
and serde only: no rtwk and no direct runtime or IO dependency; the protocol
crate itself pulls tokio and others in transitively, so core's sources name
none of them and the lints ban the IO calls), the UI package
`codex-tui-rtwk` (the rtwk app: widgets, the shell that runs effects, the
composition root) and `host/` (the binary's backend, `codex-rtwk`). The
compiler holds the first rule, that core cannot name a widget; the other
boundaries are `tests/rules/layers.rs`, see "Layers" below.

**Why.** The codex-rs workspace, its Cargo.lock, Bazel (`crate_universe`,
the PR gate) and `rust-ci-full` stay as upstream has them, so merges from
`upstream` never conflict with the port and no CI breaks on a path that
leaves the repository. Joining the codex-rs workspace (and a `--frontend
rtwk` switch in `codex`) is decided at the end of phase 1, when rtwk can be
referenced by git or registry.

**What the layout needs to resolve** (arch-review M6, checked with
`cargo metadata` on a scratch workspace):
- codex-rs's `[patch.crates-io]` (codex-rs/Cargo.toml:684-700: crossterm,
  sqlx, tungstenite, v8, …) is not applied to another workspace, so
  tui-rtwk's Cargo.toml copies it verbatim; a check diffs the two.
- A fresh lockfile hits a yanked `spin 0.9.8`, so tui-rtwk's Cargo.lock is
  seeded from codex-rs/Cargo.lock and refreshed at each upstream merge. With
  both, it resolves: tokio stays single, rtwk adds no crossterm.
- `.github/scripts/verify_cargo_workspace_manifests.py` (repo-checks CI)
  walks every Cargo.toml under codex-rs. The crate stays at
  `codex-rs/tui-rtwk` and satisfies it: package name `codex-tui-rtwk`, its
  own `[workspace.package]` (version, edition, license) and
  `[workspace.lints]`, `[lints] workspace = true`, no `[features]` or
  feature toggles on codex crates. The skeleton runs the script.
- The pin is checked, not a convention: a test (or build script) compares
  `git -C <rtwk> rev-parse HEAD` with `RTWK_REF`.
- tui-rtwk has its own `clippy.toml` (otherwise codex-rs/clippy.toml's
  ratatui entries apply) carrying the R-rule lints.

**Check in the skeleton.** The above resolves and builds cold within
`bgrun`; worktrees reach rtwk (`.claude/worktrees/rtwk` symlink).

## ADR-002 — App structure: a Flutter-style rtwk app

Template: `/Users/admin/Documents/dev_projects/rtwk-weather` (5k lines):
`model.rs` (domain types), `app.rs` (the app struct, `Msg` from background
work), `ui/view.rs` (`view(&App, &mut ViewCx<App>) -> AnyWidget`, callbacks
`cx.on(|app, cx| ..)` and `cx.on_with`; its dialog is drawn in the view as
`Stack` + `ModalBarrier` + `Dialog`, view.rs:18-20, :132, :429), `theme.rs`,
`tests/screens.rs` (`rtwk_test::Tester` over
`overlay::App::new(AppView::new(app, view))` on made-up data). Background
results arrive as `Msg` values carrying an id (`Slot.id`), and stale ones are
dropped. Unlike rtwk-weather, state changes go through `apply` (ADR-003), not
methods that also start work.

```
codex-rtwk main ─ arg0_dispatch_or_else → multi-thread tokio runtime → app-server client
 └ run_app_with on the block_on thread, runtime entered (rtwk-tokio)
    └ overlay::App → AppView(UiApp { core: CodexApp, ui: UiState }, view)
       └ Theme · Localizations · providers (config projection, keymap)
          └ MultiProvider: projections of the model (Rc slices) + app Handlers
             └ Navigator (home: chat) — PageRoutes pushed: pager · resume · usage · agents · import · onboarding/*
                └ screens: rtwk widgets reading projections with select / watch
                └ request presenter: show_dialog for the head of the model's request queue
                └ show_dialog from the asking widget; popovers via the overlay, toasts
```

**Routes and dialogs: as in twk** (the person, 2026-11-17, after the
examples were compared; arch-review B1).

twk, the Dart original that rtwk is ported from, does it this way
(`packages/twk/example`):
- The app root is a `Navigator` (files.dart:98). Screens are pushed as
  `PageRoute<T>`, and the result is awaited on pop (files.dart:813, the
  settings screen; guides/flutter_list.dart).
- A dialog is `await showDialog<T>` with `closeDialog(context, value)`
  (settings.dart:95, recipes.dart:18).
- Screens read shared state through a provider above the app:
  `ChangeNotifierProvider` at the root, `context.select` / `read` below it
  (provider/counter.dart).

rtwk has all three under the same names: `Navigator`, `PageRoute`,
`NavigatorState::push -> DialogResult<T>` (rtwk navigation/navigator.rs:714,
:1223, :1637); `show_dialog` / `close_dialog` (rtwk-overlay/src/dialog.rs:301);
`rtwk::provider::{Provider, MultiProvider, read, watch, select}` (the port of
twk's provider, rtwk-provider/src/lib.rs).

Codex does the same, on top of the plain model of ADR-003:
- **Shape.** The root view is a `MultiProvider` of read-only projections of
  the model (thread slices as `Rc<T>`, approvals, settings, session) and of
  the app's `Handler`s. Its child is a `Navigator` whose home is the chat
  screen.
- **Screens.** The pager, resume, usage, agents, import and onboarding
  steps are pushed as `PageRoute`s by the widget that opens them
  (`Navigator::of(cx).push`). A route reads its live data with `select` /
  `watch`, so the pager follows the transcript while it is open.
- **Dialogs.** Selection, settings, confirms, prompts, `/status` and the
  small dialogs open with `show_dialog` from the widget that asks, as twk
  and rtwk triage do (examples/triage/delete.rs:50).
- **Arguments are ids, not data (R14).** A route or dialog gets ids (thread,
  request, item) and reads data through the providers. Only a dialog that
  edits a value copies it at open, as its draft, and returns it on close.
  This is twk's settings screen.
- **Results return to the model as events.** A pop result or dialog answer
  goes through a `Handler` as an intent carrying its ticket or request id.
  The model drops a stale one (R1), as triage's `finish_delete` checks
  `pending_delete` (app.rs:105-118).
- **Backend-initiated dialogs** (approvals, elicitations, request-user-input,
  startup prompts) open through a presenter widget under the `Navigator`.
  This is twk's listener pattern, `BlocListener` in Flutter terms. The
  presenter `select`s the head of the model's request queue and calls
  `show_dialog` when a new id appears. The dialog `select`s its request and
  closes itself when the model no longer holds it, e.g. when another client
  resolved it. So the model's queue stays the only source, and the
  rtwk overlay holds only the id (the oracle bug bc969b651 was an overlay
  that kept its own queue).
- **Navigation state is rtwk's.** It is widget state (ADR-002) and is not
  mirrored into the model. Where a rule needs to know the open screen, a
  `NavigatorObserver` reports route changes as events.
- **Verified by a prototype** ([research/nav-proto.md](research/nav-proto.md),
  14 tests; until then rtwk had no `Navigator` under an `AppView` with
  providers above it). It checked:
  - pushed routes follow model changes (from handlers and channel items)
    after the view reruns;
  - the stack and route-local `State` survive the rerun, and home gets new
    props while covered;
  - open-on-event, close-on-resolve and the ticketed answer work, and focus
    returns to the screen;
  - `push::<T>` returns its pop result;
  - `select` isolates rebuilds.

  Conditions, binding:
  1. Screen and dialog widgets are `impl_widget!(stateful eq …)`; without
     `eq` a route rebuilds on every view run.
  2. The request presenter and every `show_dialog` caller sit under the
     `Navigator`. Above it the dialog falls back to the `overlay::App`
     overlay above the providers, and its content cannot find them.
  3. The provider list is fixed and the `Navigator` carries a `GlobalKey`.
     A changed provider list remounts it and silently drops the stack.
  4. The handlers provider has `.update_should_notify(|_, _| false)`
     (handlers are never `==` across view runs). A widget never decides from
     a `read` in the same flush as a handler change, because the projection
     lags the model until the view reruns. The presenter never reopens the
     last id it showed. Backend request ids are unique (R13).

| need | rtwk |
|---|---|
| transcript | `ListView` with an item builder over the model's cells (rtwk mounts only visible items: virtualization is the default, not a task); `SelectionArea` for selection and copy |
| message content | rtwk-rich `MarkdownView`, `DiffView`, `CodeView` |
| composer | `TextArea` over a per-thread `TextEditingController` (`TextField` for single-line inputs); `Autocomplete` for caret-anchored popovers; what TextArea lacks is gap G12 |
| lists, pickers, commands | `SelectionList<K>`, `OptionList`, `CommandPalette`, `Table<T>`, `TreeView<T>`, `TabBar` |
| screens | `Navigator` + `PageRoute`, pushed by the opening widget; data through `rtwk::provider` `select` / `watch` |
| dialogs, approvals | `show_dialog` + `Dialog` (from the asking widget, or the request presenter for backend requests); answers to the model through `Handler`s with their ticket |
| progress, working | `Spinner`, `ProgressBar`, rtwk-animation |
| analytics | rtwk-data `Chart`, `BarChart`, `Sparkline` |
| pets, images | rtwk-rich `Image` |
| keys | `Focus`, `Shortcuts`, `KeyBindings` |

- Backend events: the shell owns the app-server client
  (`AppServerRequestHandle`, app-server-client/src/lib.rs:366-372) and runs
  it through `TokioApp::spawn_tokio` / `subscribe` (rtwk-tokio/src/host.rs:28,
  :43); `AppCx::spawn` polls a `!Send` future without tokio's reactor and is
  not used for RPCs. Notifications, server requests, RPC replies and user
  intents enter one ordered ingress stream (R10) and reach `apply` on the UI
  thread; handlers never apply directly. Replies carry their ticket (R1).
- rtwk owns widget state (scroll, focus, text editing, selection, hover,
  animation); the app model holds only application state (§ADR-003).
  Per-thread widget state that must survive a thread switch (composer text,
  scroll and selection controllers) lives in `ThreadUi`, see below.
- Standalone pre-app screens of the oracle (onboarding, trust, prompts)
  become routes pushed over the chat screen at start; the composer is
  usable as a startup draft while `Session` is `Starting` (submit waits for
  `Ready` through R10's ordering).

**Per-thread widget state (arch-review M1).** rtwk's `TextArea` keeps its
text in its state or a shared `TextEditingController`
(rtwk-editing/src/text_area.rs:174-327), `SelectionList<K>` in a
`SelectionController<K>`. Per-thread UI controllers (`TextEditingController`,
scroll and selection controllers) live in a `ThreadUi` map keyed by
`ThreadId`: a field of the UI model (`UiApp.ui`, `UiState.thread_ui`; rtwk
triage keeps its controllers in the app struct too, app.rs:58-63), beside the
core model `UiApp.core` (`CodexApp`) and never inside it: core has no rtwk, so
a controller cannot be there. They are made only where Flutter makes them (R14: in a widget's
`State`, or in this per-thread UI holder, disposed with it) and outside `ThreadModel`, which is what `apply` sees. On a
switch the subtree is keyed by ThreadId and reuses that thread's controllers,
so A→B→A keeps the draft and kill buffer (SC2b). Text the model needs
(submit, queue) enters `apply` as an intent on submit; the model never mirrors
the draft. The composer is rtwk `TextArea`; what it lacks for Codex (atomic
elements such as `[Image #1]`, kill ring and yank, vim mode, paste-burst
detection, history recall; Ctrl+Y is redo in rtwk) is rtwk gap G12, built in
rtwk. Codex's pure composer logic (paste-burst detector, mention codec,
history store) is carried as ADAPT. Until G12 lands, S10 lists the missing
behaviours as deltas.

### Amendment 2026-11-19: pages, modes, slot views and widgets

The person: "there are not many pages here; distinguish, as in Flutter, what
is a page and what is a widget." Research `docs/rtwk-port/research/pages-vs-widgets.md`
classified every entry of the map against the oracle. Classes (Flutter
analogue): **Page** (a `Navigator` route with its own layout), **Mode** (a
state of one page or widget, switched by its controller), **Slot view** (a
panel in the chat page's bottom slot), **Dialog** (centered, `show_dialog`),
**Widget** (part of a page). Result, by id:

- Pages (~6): S10 home chat; S16 pager (transcript, static `/diff`) with S20
  `/usage` as the same alt-screen overlay family; S18 resume picker; S02-S04
  onboarding; S01/S05 (cwd, model migration, OSS, unarchive) and S06 import as
  pre-chat or own-loop screens (three S05 entries unverified).
- Modes: S17 backtrack (a mode of the pager, not a screen); subagent switcher
  (active thread of the chat page); S27 inline.
> **Amendment 2026-11-19b (the person): lay the app out as one would in Flutter, not as the oracle is built inside.**
> The oracle is the spec of what the screen shows; its internal structure (bottom_pane views, overlays) is not a model.
> - **Pages** (Navigator routes, each its own scaffold): StartupPage (onboarding as one page with steps: sign-in,
>   trust, model migration, import — a step controller), SessionPage (transcript ListView + composer), AgentsPage
>   (the command center), ResumePage (pick a past session to continue; a separate page — the same list/row widgets
>   as AgentsPage may be reused, shown differently), UsagePage (/usage).
> - **Modes of SessionPage**, switched by a controller, never a new page or a copied widget: expanded transcript
>   (pager: Ctrl+T, /diff, backtrack), the active thread (subagent switch).
> - **Modal sheets/dialogs over a page** (Flutter showModalBottomSheet / showDialog; rtwk show_dialog with bottom
>   alignment / Dialog::bare or centred): approvals, model and other pickers, settings, pets, confirmations.
> - **Widgets** of SessionPage: transcript cells (/status is a cell), composer with its autocomplete popups, status
>   line, footer hints, ambient pet; shared: SelectionList-based lists and pickers, the theme with its palette.
> This replaces the oracle-based classification below wherever they differ.

- Pages also: S21 the Agent command center (the person, 2026-11-19: a page, as VS1 built it — not the oracle's bottom-pane view).
- Slot views: S13, S14, S15, S23, the pets picker, voice settings (S21 is a page, above)
  the agents command center (the oracle shows it as a 24-row `BottomPaneView`,
  `app/agent_center/render.rs:88`, not a full screen).
- Widgets: S11, S12, S19 (`/status` is a history cell), S22, S24 ambient, S25
  strip, S26.

The rule: **a mode of one widget is driven by its controller, never a second
copy** (no parallel screen with its own list, scroll or selection for what is
one widget in another mode). The "screen" column of the map above reads
"entry"; a row is a page only where this amendment says so.

### Amendment 2026-11-18: the bottom pane is a slot of the chat page

The person, after research `parity-routes` (`docs/rtwk-port/research/parity-routes.md`)
and rtwk's answers (`docs/rtwk-port/refs/rtwk-answers-2026-11-18.md`, G28):
codex-tui draws its inline views (approvals, selection lists, the composer's
popups, settings panes) *in place of the composer*, with the transcript tail
still visible. In tui-rtwk these are **a slot of the chat page**, one widget
under `Expanded(transcript)` in the chat `Column`, rendering the top of a
**core view stack** (ids only, changed by `apply`; no rtwk in core). Their rows
are drawn by one shared `Picker` (the twin of `ListSelectionView`). Only the
oracle's `Centered` views and real dialogs use `show_dialog`, with an unframed
builder (barrier alpha 0, width `min(w-4, 72)`), under the Navigator as before.
For inline views nothing reopens and no close watcher is needed (G14/G16 do
not apply to them). The binding conditions below hold for the dialogs that
remain.

## ADR-003 — App state

**Scope.** Application state and how views follow it; widget state is
rtwk's.

**Measured** ([app-state.md](research/app-state.md), prototype
`~/.cache/codexcli/research/app-state-proto`): C, a plain model owned by the
rtwk app (one writer on the UI thread, events via `AppCx::channel`,
`ViewCx::memo` on revisions): 1.1 allocs/delta, 7.4 ms per scenario. A,
reactive_graph + reactive_stores used naively (a signal write per delta
through an Arc store across threads, keyed transcript): 160 allocs/delta,
58.6 ms; cross-thread store reads panic, contrary to rtwk's
`docs/guide/state.md:94` (reported:
`~/.cache/codexcli/rtwk-issues/rtwk-state-reactive-graph.md`). B
futures-signals: rejected (deadlock-prone wake, no batching, unmaintained
since 2024-07). Rebuild isolation was the same for all three.

**Round 3, optimized** ([app-state-aopt.md](research/app-state-aopt.md);
the person's criterion: cost *with* optimizations, and maintainability of
derived state across panels). Same harness, release; times as ratios (load
was high):

| variant | allocs/delta | scenario | derived-panel change |
|---|---|---|---|
| C plain model | 7.3 | 1.0x | 14.5 µs, 17 allocs |
| A-sig: reactive_graph signal per field, UI thread, per-frame coalescing | 15.6 | 1.5x | — |
| A-opt: reactive_stores store, UI thread, coalesced | 41.8 (28.0 with one write per frame) | 3.6x (2.0x) | 22.5 µs, 178 allocs |
| A naive (round 1) | 170.6 | 10.5x | — |

Maintainability (40 threads, 4 derived panels): C 69 lines vs A-opt 95;
adding a derived value and a panel: C 3 places / 10 lines, A 5 places / 5
lines. In C a cached subtree can use only what it is keyed on — anything
else fails to compile. In A a forgotten dependency is an untracked read: the
panel stays stale until something else changes, caught only by a test; A
also carries six rules that fail silently (UI thread only, keep widgets
across rebuilds, no `batch`, ...). UI-thread confinement removes the
cross-thread panic; the rtwk fixes in the issue file do not change A-opt's
cost (it is reactive_stores' own per-write work). New rtwk finding: a
`Select` notified inside a frame wakes the loop again (+13 ms first frame
after idle; cause inferred).

**Landscape of Rust state libraries** ([state-survey.md](research/state-survey.md)):
reactive_graph/reactive_stores, dioxus-signals, futures-signals,
eyeball/eyeball-im, rxRust, redux-style crates, yewdux, relm4, tui-realm,
iced, Crux, bevy_ecs, salsa, actor crates, tokio `watch`, statig. None
handles request ids and stale results — that is app code in Zed, Helix,
gitui, yazi and cosmic-files, which all keep a plain owner and bring async
results in by message. Variant E (an entity + notify pattern written by
hand) measured 8.1 allocs/delta, 2.5 ms against C's 7.3 and 2.4 ms; its
risk is a forgotten notify (a stale view), a recurring bug class in Zed.

**Trade-offs of the approach families** ([state-approaches.md](research/state-approaches.md):
for each — why it exists, what it solves, what it brings, eight Codex cases,
verdict; matrix in §10):

| family | solves | brings | verdict |
|---|---|---|---|
| 1 plain model, whole-view rerun, `ViewCx::memo` | views and derived values cannot go stale (a kept subtree can use only its keys — otherwise no compile); `apply` is a plain function, deterministic tests; cheapest (7.3 allocs/delta) | root view reruns each changed frame; a wrong revision key or a misused `unchanged()` gives a stale subtree; `update` grows large | **core** |
| 2 reducers + effects as data (redux-style, Crux) | late replies are events; race tests need no runtime | boilerplate, indirection; crates dead or breaking each minor | **partial**: take its `apply -> Effects` boundary |
| 3 fine-grained signals (reactive_graph, dioxus-signals, futures-signals) | automatic dependency tracking; only subscribers rebuild | untracked reads go stale silently; cross-thread read panic; lost batch notification; six silent rules; 2.1–5.7x cost | does not fit |
| 4 entities + notify / diffs (Zed, eyeball, rtwk `Select`) | panel isolation at family 1's cost (8.1) | a forgotten notify is a stale view (44 Zed PRs); derived state synced by observers | **reserve** |
| 5 actors (ractor, kameo, actix) | isolating services | UI sees only snapshots; `Send` everywhere; no order across mailboxes | not for UI state |
| 6 ECS (bevy_ecs) | iterating many entities per tick | schedule decides order; unread messages dropped silently | does not fit |
| 7 incremental computation (salsa) | heavy derived results on demand | nothing pushes, a view cannot follow it; cancellation by panic | not now; maybe one heavy query later |
| 8 streams / Rx | `switch_map`-style dropping of stale results | a second scheduler beside rtwk's; state hidden in closures | ingress only, not a store |

**Decision (2026-11-17, accepted by the person): family 1 with family 2's
effect boundary; family 4 as the reserve; rules R1–R14 binding.** Reasons: (1) with the rules R1–R14 below it removes state bugs
as a class on the oracle's real history (state-bug-classes.md) with the
narrowest failure mode — many mistakes do not compile; (2) request ids and
race tests are cheapest with one writer and a pure `apply`; (3) it is rtwk's
own model, the cheapest measured, and what Helix, gitui, yazi and iced do.

- Rust structs owned by the rtwk `AppView`; one writer, the UI thread.
- Per thread `apply(&mut ThreadModel, ThreadEv, &mut ThreadFx)`; app-scoped
  state (account, rate limits, app-level MCP startup, settings)
  `apply_app(&mut CodexApp, AppEv, &mut AppFx)`. Backend events, replies and
  intents enter one ordered ingress stream (one `AppCx::channel` sender;
  handlers send their intents into it too, so an intent never overtakes an
  event already received, R10). Effects (RPCs, config writes) are values the
  shell runs through rtwk-tokio (`TokioApp::spawn_tokio` / `subscribe`);
  their results come back as events carrying their ticket, and `apply` drops
  stale ones. Tests: events in, model and effects out, no UI.
- Two kinds of pending request, kept apart: `outgoing: (ThreadId, Ticket) ->
  Ctx` for our RPCs, and `server_requests: (ConnEpoch, ThreadId,
  ServerRequestId) -> Approval` for requests the user answers (approvals,
  elicitations, user input). Request ids are unique only per connection
  (6f47fcb0b), so the connection epoch is part of the key; an approval reply
  goes through the originating connection's handle; entries of an old epoch
  are dropped when the epoch changes.
- A thread/read or resume snapshot replaces the affected turns wholesale
  (snapshot wins); it is never merged by id with live entries, because the
  backend synthesises ids when it rebuilds history (`item-N`, `rollout-N`,
  app-server-protocol/src/protocol/thread_history.rs:1600-1606, :1724-1728).
- Every thread's model is kept current as its events arrive, so switching
  threads needs no replay in the UI; the user-visible behaviour of the
  oracle's replay-on-switch (what a switched-to thread shows) is the parity
  target. Reconnect/resume still replays from the backend.
- Derived values are pure functions computed in the view (measured cheap:
  10 derived values scanning 1,000 threads add 9 µs per frame) and kept by
  `ViewCx::memo`. Memo deps are moved into the cache and its build is a `fn`
  pointer seeing only `&D` (rtwk app.rs:805-839), so the data travels with
  the key: deps are cheap per-row values or `(rev, Rc<T>)` over structurally
  shared items. `Versioned<T>` (an app type, not rtwk's) bumps `rev` on
  `get_mut`. A counter kept in `apply` is allowed only inside one collection
  type, with a test that recomputes it.
- Rules: `unchanged()` is banned in feature code (CI grep); long lists use
  `ListView` builders; only mounted screens render; memoise one level per
  region; a per-release budget test on the scale scenario.
- Flows (approvals, onboarding) are plain enums; `statig` only if a flow
  outgrows them. Config and connection changes enter as events (the shell
  `subscribe`s to the `watch`); no `watch` is read in a view or followed into
  an `Obs`.
- Switch a panel to the reserve (entities + notify inside `update`,
  followed through `Select`) only if a phase-2 profile shows the root view
  above ~1 ms per frame after these rules.
- Measured scaling (state-approaches.md §11): 12–19 µs per frame at 1,000
  threads / 30 panels; the cost grows with the square of memo calls (81 →
  9.5 µs, 331 → 97 µs, 1,031 → 648 µs) because rtwk's memo cache is a
  linearly searched `Vec` (rtwk crates/rtwk/src/app.rs:812-819) — reported
  to rtwk with the nested-memo release (an outer memo hit releases the inner
  entries).
- Later, not phase 1: per-thread sequence numbers in app-server
  notifications would let reconnect deduplicate without comparing content.

**Off-thread readers: none need a snapshot**
([off-thread-state.md](research/off-thread-state.md)). In the oracle,
notifications, the status line and the title are decided on the UI thread;
background work reports over channels. Four places touch UI state
off-thread (onboarding sign-in state, the `voice_only` send filter, the
interrupt retry task, the dynamic-tools MCP handle); in the rtwk app each
becomes a message handled on the UI thread, or a `watch` of the connection
handle. Work started off-thread gets config and theme cloned at spawn. So
the model has one owner and no published copy.

**Acceptance criteria (every screen and the state layer).**
1. Views are derived from state only: no "refresh the UI" calls, no widget
   holding a copy of application state.
2. Derived values are computed (pure functions or memos), never kept in sync
   by hand.
3. Every async result carries a generation or request id; a stale one is
   dropped.
4. Races are tested: late RPC replies, thread switches mid-stream, resize
   mid-stream, approvals arriving during a dialog.

**Checked on the real bug history** ([state-bug-classes.md](research/state-bug-classes.md):
159 commits and 42 issues read, root cause per item). The design *without*
rules removes no sub-class as a class (it only removes the oracle's
mechanism for 15 commits). Of the 508 bug-flavoured fix commits, ~11% are
state-mechanism bugs (the earlier "~22–32%" counted tests, build, backend and
layout fixes too). With R1–R14, as corrected by arch-review M2/M4 (SC6a
moved to caught: R14 is a lint over known escape routes, not a type
guarantee; SC4a's live+replay case 6107b9886 moved to caught: snapshots
replace, ids do not merge): 20 (3.9% of 508) impossible by construction, 33
(6.5%) caught by a mandatory test or lint, 2 still possible; of 34 state
issues, 8 impossible, 21 caught, 5 possible (SC6a issues and #2886 moved). A hand-written single
reducer (every change an Action) removes no more than this mix — the oracle
already has a 335-variant `AppEvent` dispatcher and still had every class,
because state lived in widget copies — so only intents that touch shared or
cross-thread state are values (R10).

**Rules (binding; state-bug-classes.md §4 has enforcement details):**

| rule | what | enforced by |
|---|---|---|
| R1 | requests return `Reply<T>` readable only through `pending.resolve(reply)` (ticket check); effects run only in `shell`: every spawn path (`AppCx::spawn`, `Cx::spawn`, `TokioApp::{spawn_tokio,subscribe,subscribe_with,follow}`, `rtwk_tokio::{spawn_tokio,subscribe,follow}`, `std::thread::spawn`, `tokio::spawn`, `AppCx::channel`) is banned elsewhere; each subscription captures its attach epoch when created and stamps every event with that constant; one ingress router drops older epochs | module privacy + `clippy::disallowed_methods` (crate-wide deny, `#[allow]` only on `shell`, CI grep rejects other allows) |
| R2 | per-thread data only in `ThreadModel`, per-thread UI controllers only in `ThreadUi`; per-thread views keyed by ThreadId through one `thread_scope` helper | switch round-trip property test |
| R3 | `apply(&mut ThreadModel, ThreadEv, &mut ThreadFx)`; thread RPCs only via `ThreadFx` (stamps its id); `outgoing` keyed `(ThreadId, Ticket)`, `server_requests` keyed `(ConnEpoch, ThreadId, ServerRequestId)`; intents carry their target ThreadId | types |
| R4 | transcript = ordered map keyed `(TurnId, ItemId)` with upsert, one projection from protocol events; a resume/read snapshot replaces the affected turns wholesale, never merged by id | type API (no push) + resume/reconnect-mid-turn test (transcript equals the snapshot) |
| R5 | `TurnState { Idle, Running(RunningTurn{..}) }`: transient state inside the variant, the end transition consumes it | types + end-path matrix test |
| R6 | `Remote<T> { Loading, Loaded{value, fetched_at, generation}, Failed }` | types |
| R7 | derived values are functions, never stored; any cache is a memo on `Versioned<T>` or checked by `Model::assert_consistent()` after every event in tests | test harness |
| R8 | lifecycles as enums, predicates as functions of them, outcomes as enums not bools | lifecycle × intent table test |
| R9 | entities created/upserted from whichever of response or notification comes first; local mutations wait for confirmation | event-permutation property test |
| R10 | intents touching backend-shared or cross-thread state are values sent into the same ingress sender as backend events; apply order = arrival order | permutation test |
| R11 | lists use rtwk's keyed `SelectionList<K>`; the clock is a model input | lint on `Instant::now`/`SystemTime::now` in views |
| R12 | no IO in `apply` or views; IO only in effects run by the shell | `disallowed_methods` lint |
| R13 | a newtype per id kind; composite keys where an id is scoped | types |
| R14 | app data never enters rtwk local state; controllers (`ScrollController`, `ListController`, `SelectionController`, `TextEditingController`, `TabController`, `ExpansionController`, `AnimationController`, `FocusNode`) are made as Flutter makes them: in a widget's `State` (`create_state`/`init_state`/its fields' constructors) or the per-thread `ThreadUi` holder, disposed with it, never inside a `build`/`view`/`*_view`/`render*` function or a builder closure, never in core; screens take app data only as `&CodexApp`, memo deps or `rtwk::provider` projections of the model; routes and dialogs take ids, not data (a dialog that edits a value copies it as its draft) | `disallowed_types` `Obs`; `disallowed_methods` `use_state`, `TokioApp::follow`; `rules.rs`: `r14_controllers_are_made_in_a_state_never_in_build_or_a_closure`, `r14_core_has_no_controller` (the constructor ban and its file allowlist are gone, 2026-11-19); a check that `State` fields of screens and dialogs hold ids, drafts and controllers, never model types — a lint, so SC6a is caught, not impossible |

**Queued command continuations (R8, R9, R10 and ADR-012).** The thread queue issues each
command with its original thread and generation. Only a report for the issued generation
can consume it. For a modal continuation, the queue retains the opening and open phases.
The route callback carries the operation it received when it opened. Cancellation or
refusal settles that generation once, even if its writer detached. A selection can
change settings only while its original thread is active and still holds the operation.
An accepted selection retains the operation through the confirmation of the write that
affects the next turn. Picker failures and write responses report in its original conversation. Lag,
reconnect, feature settlement and page-front events can
reconsider admission through the same queue gate. Admission still waits for recovery.
The private phases and
addressed events apply the [Rust Book's state types](https://doc.rust-lang.org/book/ch18-03-oo-design-patterns.html#encoding-states-and-behavior-as-types),
[SCXML run-to-completion](https://www.w3.org/TR/scxml/) and
[Flutter's route-owned dialog result](https://api.flutter.dev/flutter/material/showDialog.html).

The model picker uses this protocol. Rename retains its feature-owned operation.
Verify the open and close routes for permissions, apps, review, presets and other modal
commands before migrating their page-front continuations.

**ADR-003 thread ownership: how it is enforced (2026-11-19, adr003-enforce).**
By type (the compiler refuses a violation): `ThreadModel` has a private
constructor and a private `seal` field, so it is made only by
`ThreadSet::open(id, Origin)` in `core/src/threads.rs`; `CodexApp.threads` is a
private `ThreadSet`, so a screen or the UI model cannot insert, replace or
drop a model; a thread RPC is the data `threads::ThreadAsk`, whose wire form
(`ThreadAsk::wire`) exists in `threads.rs` only; `Ticket` and `Reply` are as
before (R1, R3). By rules test (`tests/rules/layers.rs`):
`thread_model_is_made_only_in_threads_rs`, `codex_app_holds_its_threads_privately`,
`thread_rpcs_only_in_threads` (no `ClientRequest::Thread*`/`Turn*` outside
`threads.rs` and `protocol/mod.rs`'s two app-minted commands),
`the_shown_thread_is_set_only_in_apply_app` (`session.active` is written in
`core/src/app.rs` alone). By behaviour tests (`core/tests/adr003.rs`): R2
switch round-trip and "A keeps receiving while B is shown", R4 resume
mid-turn, R5 end-path matrix, R7 `CodexApp::assert_consistent` after every
event, R8 lifecycle x intent, R9/R10 interleavings. `CodexApp` and `ThreadModel` fields are private. Readers receive immutable
projections. The command center and resume picker both open sessions through
`Intent::OpenThread(id)`. The app displays a held writer, waits for an
in-flight resume, or issues one resume. Esc cancels the navigation while a
successful late reply can still establish the single held writer. The
command center derives the current row from the app session, request previews
from the app queue, and activity from the typed thread catalog in `ThreadSet`.
Catalog previews also cover threads that have no local writer. Detail reads
track one phase per thread so reverts and gaps can invalidate an old turns
reply. Pin rank comes from the backend pin lists, and the backend-cycle
control comes from the active conversation and connected backends.
The app sequences worktree creation, thread start, bind and compensation.
The shell performs filesystem effects and sends RPCs; the view renders a
typed retained-checkout notice.

Clippy's `disallowed_*` lists are crate-wide: each rule that says "outside
X" is a crate-wide deny with `#[allow(clippy::disallowed_…)]` on the
allowlisted modules, plus a CI grep that rejects such allows anywhere else.

## ADR-004 — Screens and content

- Each screen of §1 is a slice: its route or dialog, the state it reads, the
  behaviour and the reusable code carried over from the oracle (named by
  oracle `file:line` and test names in the slice's task; copied code keeps a
  comment naming its oracle file), its screen tests.
- Content is rendered by rtwk-rich (`MarkdownView`, `DiffView`, `CodeView`),
  not by re-implementing Codex's line renderers. Where the result differs
  visually (wrapping, table borders, highlight palette), the difference is
  listed in the slice and accepted by the person or fixed.
- Charts (`/usage`) use rtwk-data; pets and images use rtwk-rich `Image`.
  Visual deltas from the oracle's braille plots and image protocol code are
  accepted (the person, 2026-11-17).
- The streaming tail is one live item at the end of the transcript list,
  updated at most once per frame; settled messages become ordinary items.

## ADR-005 — Screen modes

Owned full screen first (the default; alt screen, mouse, the transcript as an
rtwk `ListView`). Inline mode last: `ScreenMode::Inline` with settled items
committed through `Static`, after the rtwk gaps it needs (G1, G2, G5, G7).

## ADR-006 — Input and keys

rtwk decodes the terminal (kitty flags, bracketed paste, mouse, focus,
resize). Keys are handled by rtwk `Focus`, `Shortcuts` and `KeyBindings`.
Codex's keymap configuration (`tui.keymap.*`: 9 contexts, chords, a leader
key, vim mode — the editing side of vim is G12) is read into those bindings, so user configs keep working;
the oracle's keymap tests are the spec. Input is never frozen while an RPC
is in flight (the person, 2026-11-17). Ctrl+Z suspends; the external editor
suspends the app. Typeahead quarantine before security prompts needs G8.

## ADR-007 — Parity: functional and visual

1. **Screen tests** in the rtwk-weather manner (`tests/screens.rs`): a
   `Tester` of the oracle's size over the app with a fake backend, keys and
   server notifications scripted, the screen asserted. Each slice re-expresses
   the oracle tests that define its behaviour.
2. **Scenarios, both binaries.** `codex` (oracle) and `codex-rtwk` driven in a
   pty (rtwk-term `TerminalSession`) against the same mocked model
   (`core_test_support::responses`, wiremock SSE), the same `CODEX_HOME` and
   key script; screens compared at named checkpoints, timers and paths
   normalized ([test-infra.md](research/test-infra.md) §D3).
3. **Visual reference.** The oracle's `.snap` files and pty captures are the
   reference a slice is reviewed against, side by side. Byte equality is not
   required; every difference is listed in the slice (ADR-004) and accepted
   or fixed.
4. **Registry** (created in W1, tm `port-refs`). `docs/rtwk-port/refs/codex-registry.csv`: screen id →
   oracle files, snapshots, tests → status (`todo` / `built` /
   `parity-accepted`).

## ADR-008 — Performance, shown visually

[perf-compare.md](research/perf-compare.md): `perf/drive.py` runs a binary in
a pty at fixed sizes against `perf/fake_model.py`, 11 scenarios (stream
100/s and 1000/s, typing while streaming, long transcript scroll, resize
storm, startup, idle, ...), interleaved A/B runs N=15, load gate, raw JSONL.
`perf/report.py` writes one `report.html` (distributions, ratio table with
bootstrap intervals, side-by-side asciicasts); `perf/trend.py` keeps the
milestone trend against the original's baseline. The baseline of the
original (S1, S2, S5, S9, S10 at 120x40) is taken first, before any port
code. No speed claim from a microbenchmark alone.

While a turn only waits on a background terminal, the status row and the
window title keep their activity marks still. The elapsed display updates
once per second. New output renders when it arrives, and active reply or
plan output restores animation. This is the accepted performance
departure tracked by `fix-status-idle-redraw`.

## ADR-009 — Migration order

Wave labels match design.md §10.

0. **W0** research, architecture, its verification (arch-review), process
   design (a separate session). No code.
1. **W1 foundation.** Skeleton (below); session core: app-server client in
   the shell, thread start/resume, streaming into the model, approvals round
   trip, slash command dispatch; perf baseline of the original; scenario
   harness; refs registry.
2. **W2 main chat screen** S10 + S11 + S12 + S14 (the generic selection
   dialog the chat screen opens) — critical path.
3. **W3 screen slices in parallel** (4–5 `codex-porter` agents in worktrees,
   rtwk-weather manner): 18 slices — S13, S15–S25, pre-app routes S01–S06.
4. **W4 inline mode** S27 and the rtwk gaps it needs; parity sweep over the
   registry; performance report.

## ADR-010 — Localization

**Decision.** The app's user-visible text lives in typed `const` tables, never
as literals in widgets, in the form rtwk-l10n has for its own widget strings
(rtwk `docs/design/l10n.md`):

- one `src/screens/<dir>/strings.rs` per screen directory: a struct of
  `&'static str` fields, `Msg<N>` fields behind typed methods for messages
  with arguments, `PluralMsg<N>` where a count changes the wording (the
  locale's plural rule picks the category); `EN` is the oracle's text byte
  for byte, each group of strings with a comment naming the oracle file it
  comes from;
- shared text (the chat frame, the status line, the presenter's request
  dialog, the debug menu) in `src/l10n/strings.rs`; `src/l10n/mod.rs` has
  `Lang`, `Tables<T>` (English required, other languages optional, a missing
  one shows English) and `lang(cx)`;
- a screen reads its table by reference through one accessor,
  `strings::of(cx)`: no lookup by key, no allocation, and the screen rebuilds
  when the locale changes;
- the locale is negotiated once at start (`Cli::lang`: `--locale`, else
  `LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE` through rtwk-l10n's `Locale`;
  English as the fallback) and stored in `UiApp.ui.lang` (a UI fact, not in the core model); it reaches the tree
  as `TwkLocalizations` at the root (`ui::run::localized`), so rtwk's
  built-in widget strings follow the same locale;
- numbers and dates go through rtwk-l10n's formatters where the oracle formats
  them (the clock's 12/24h detection of `clock_format.rs` stays);
- enforcement: `tests/rules.rs` (`adr010_no_user_visible_literal_outside_the_string_tables`)
  fails on `Text::new("`, `Text::styled("`, `Span::styled("`, `Text::rich("`,
  `.title("`, `.placeholder("`, `Button::new("`, their `format!(` forms, a
  `const TITLE`/`LABEL`/`HINT` and any `format!` literal that holds a word,
  outside `strings.rs` and `src/l10n/`. A screen directory's title (debug menu
  row, slot dialog) is `Strings.title` of its own table, read as
  `strings::title(lang)` where there is no tree.

**Adding a language** is one `const` per table and one `with_xx(&XX)` on its
`TABLES`, plus a variant of `Lang`; no screen is edited. A slice edits only
its own `strings.rs`. The skeleton ships a three-string Russian table in
`src/l10n/strings.rs` (the ready word, the working word, the composer
placeholder) as the proof of the path (`tests/l10n.rs`); everything else
stays English.

**Why.** The oracle has no localization: about 3,000 user-visible literals sit
in its widgets. Moving them into tables is cheap while each screen is written
and costly once they are scattered; English tables are parity, since they
hold the oracle's strings byte for byte, so the port loses nothing and gains
the structure. Following rtwk-l10n's form means no new mechanism: the same
table shape, the same plural rules, the same `TwkLocalizations` carrier.

## ADR-011 — Mouse

The person (2026-11-18): tui-rtwk adds mouse support, which codex-tui does not
have. It is a decided addition, not a delta: the keyboard behaviour stays the
oracle's, and with the mouse unused every screen equals the oracle's. Mouse
capture is turned on through rtwk; the wheel scrolls lists, the transcript and
the pager; a click selects a row (Picker, lists, the command center), a click
on the selected row or a double click accepts it; tabs, buttons and the
composer take clicks (focus). Terminal text selection and copy must keep
working (rtwk-gaps G9, G11). Every view a slice builds wires the mouse and
tests it with rtwk_test mouse events; the `mouse` slice of VS1 closes what is
left across views and adds a pty scenario.

## ADR-012 — Core feature slices

Research: [research/core-slices/report.md](research/core-slices/report.md) (the core hub
measured: 14 core-pass features changed a median of 7 shared files each, 49% of a week's core
commits touched one of 8 hub files). The porter's recipe is the last part of this ADR
("Recipe: a feature as a slice").

**Decision.** A feature of the core is a slice: `core/src/features/<f>/` (its intents, thread
intents, RPCs, notes, app state and per-thread part, and the hooks it needs) and
`src/features/<f>/` (its words and views). It is registered once in `feature_list!`
(`core/src/features/mod.rs`), which generates the root enums and their dispatch, as
`screen_list!` does for screens. A slice reads the app through `AppView` / `ThreadView` and
changes it only through the outbox of `AppCx` (and the effects of `ThreadCx`). A slice never
names `CodexApp` or `ThreadModel`. Its per-thread part is a field of `ThreadModel`
(`ThreadModel { core, slots, features }`), written only by `threads::apply` under the
thread's writer (ADR-003 unchanged). Its RPCs go out through `AppCx::ask` and
`ThreadCx::request` (the one door `ThreadFx::request_feature`). The thread lifecycle RPCs
(resume, fork, start, read, turns, subscribe, the turn calls, `review/start`) stay in `ThreadAsk`, built in
`threads.rs` and `protocol/mod.rs` only. The generic menu, prompt and notes carry
`FeatureMenu` / `FeaturePrompt` / `FeatureNote`, never a per-feature variant of their own.

**What is not a slice (amendment 2026-11-23, after the early review
[research/core-slices/review-early.md](research/core-slices/review-early.md), option B).**
Thread lifecycle state is the thread's own, not a feature's: the queue (`threads/queue.rs`: the
lines, the command due, the held flag, `send_next`), the steers, `last_submit`, the turn starts.
It is a pipeline of about twenty sites interlocked with `recovered`, the steers, `provider_error`
and the interrupt, the same kind of thing as `turn/start` itself, and a slice for it would need a
hook on every feature per interlock (`hold_submit`, `on_start_refused`, ...) and a write into
`ThreadCore` that `ThreadCx`'s shared view forbids. The queue's five intents left `apply_ev` for
`threads::queue::apply` (the views' door stays `chat::ThreadIntent`; `apply` gives back, as
`Err`, an intent that is not the queue's, so none is dropped there); `AppCx::settled`
is the generic "input released" event, and the app turns it into the queue's resume
(`app/features.rs`, `Out::Settled`) beside every feature's `on_settled`. A feature that must
start a turn or give a message back asks through a `ThreadCx` effect, each one a decision.
**A hook is added to `Feature` only for a thread or app lifecycle fact** (a turn ended, a
thread resumed, the input released), never for one producer and one consumer; the count of the
trait's methods, of `Out` and of the generated root's non-feature variants is a ratchet
(`tests/rules/hub_budget.txt`, `the_feature_trait_has_the_hooks_the_budget_says`), raised only
with a paragraph here.

**A feature that is half a slice (framework-2, 2026-11-23).** A flow that writes `ThreadModel`
fields or sits in `threads::apply_ev` is split: the menu half is a slice (the menu, its rows, its
picks, the words), the thread half stays in `core/src/threads/` as thread lifecycle state. The
slice asks the thread through the generic door (`cx.thread(&id, intent)`) or the app reaches the
thread through `ThreadAsk` as before; it never writes the field. `/review` is the example
(`Menu::Review` and its picks move, `ThreadAsk::Review` and `review/start` stay: a lifecycle
RPC, `LIFECYCLE_RPCS`), likewise `/permissions` args (the menu moves, `HoldQueue` is the queue's)
and collaboration modes (the menu moves, `SetMask`, `CycleMode`, `switch_mode` and
`ThreadFx::modes` are turn settings); provider error is thread state, not a slice at all
(`provider_error` is in `has_writer` and `send_next`). The template step is section 8 of
[refs/feature-slices.md](refs/feature-slices.md).

**The counterpart.** In Flutter this is feature-first folders (`features/goal/` with its data,
domain and presentation, not `models/`, `screens/`, `blocs/` across the app). The generated
root is Redux's `combineReducers` and the elm-spa generated `Main` (one `Model`, one `Msg`,
one `update` per module composed by a list), and `AppCx` is Elm's `Cmd` / `Effect`: a step
says what it wants, the app does it after the call, in the order it was said, and the step
holds neither the app nor a thread. `AppView` / `ThreadView` are what `BuildContext` and a
selector are: read-only facts. A feature's `Feature` trait is the module's `init / update /
subscriptions` with default no-ops.

**Shape.**
- `trait Feature` (marker type per feature): associated types `State`, `ThreadPart`, `Intent`,
  `ThreadIntent`, `Command`, `Reply`, `Ctx`, `Menu`, `Pick`, `Prompt`, `Note`, `Fact`; steps
  `intent`, `answered`, `effect_done`, `choose`, `picked`, `prompt_submitted`, `note_tone`,
  `on_fact`, `on_resumed`,
  `thread_intent`, `thread_answered`, `on_turn_end`, `on_interrupted`, `on_settled`, `wire`,
  `decode`; every step has a default that does nothing but `wire` / `decode`. A component a feature does not have is
  `Never<Self>`; the feature's own types are its own (no two features share a type, so the
  root enums' `From` impls are unique).
- The hub carries one generic variant per component: `Intent::Feature`,
  `ThreadEv::FeatureIntent`, `Command::Feature`, `AppCtx::Feature`, `ThreadCtx::Feature`,
  `Answer::Feature`, `Menu::Feature`, `Picked::Feature`, `PromptKind::Feature`,
  `NoteKey::Feature`, `AppEv::Feature` (a fact the host sends unasked). The UI routes `FeatureNote` to `src/features/<f>/strings.rs` and
  `FeatureMenu` to the feature's `FeatureView::menu_list`: the opener owns the continuation,
  as Flutter's `showDialog<T>` does.
- **Hook order is the order of `feature_list!`, which is by directory name.** Every hook that runs
  over all features (`on_resumed`, `on_turn_end`, `on_interrupted`) runs them in that order; a
  feature must not depend on it (a feature that needs another to go on says the event, `cx.settled`). What a hook sees is the thread as the step left it:
  the turn start the thread's own `send_next` said before the hooks counts
  (`ThreadView::busy`), the ticket being the model's only after the whole `apply`.
  `core/src/features/framework.rs` tests it with two probe features registered in a list of
  their own.
- **The host door** (early review R2). A feature asks the host for one of its capabilities
  with `AppCx::effect(ask, ctx)`; `effects::Ask` / `Done` stay the hub's list of what the host
  can do (copy, write a file, suggest a title, read the clipboard...), never a feature's: a new
  capability is a variant there, counted in the budget. The app chooses the request number
  (`FEATURE_REQUEST_BIT | n`, so it never meets a screen's stamp), keeps the feature's `ctx`
  against it and sends the same `Command::Effect` a screen's `Intent::Effect` makes, so the
  host does not know who asked; the answer goes to `Feature::effect_done` and is kept for no
  screen; a copy answers twice (`Pending`, then how it ended: `Done::is_final`), the last
  answer ends the ask. Only `AppCx` has it: a host effect is the app's, not a thread's. The
  hook is an app lifecycle fact (the host answered), which is why it is a method of `Feature`
  and not one producer's nudge; it raised `Out` and the trait by one each in the budget.
- **Facts the host sends unasked** (framework-2, M1). The host tells a feature what it read (the
  config feed of the plugins, the presets): `AppEv::Feature(FeatureFact)`, built with the
  feature's own `Fact` type and `From` (`FeatureFact::from(presets::Fact::Slots(..))`; the
  sender, `host/src/in_process.rs`, is the one file besides a feature's own that may name
  `features::<f>::Fact`, `FACT_SENDERS` in the rules), reaches
  `Feature::on_fact(state, fact, cx)`; the feature keeps it in its state and its open menu shows
  it (below). One generic hub variant (`AppEv` 17 -> 18, budgeted) takes the place of a variant
  per feature (`AppEv::Presets`, `AppEv::PluginConfig` shrink when those features move), and the
  hook is an app lifecycle fact (the host spoke), which is why it is a method of `Feature`
  (`Feature` 15 -> 16).
- **A live menu** (framework-2, M2). A feature's `Menu` says which page is open; its rows are
  `FeatureView::menu_list(menu, &State, words)`, read from the feature's state at every build.
  The app provides `FeatureStates` to the views as it provides `ScreenStates`
  (`CodexApp::features()`), the select dialog watches it, and a change of the state (a fact, a
  reply, a pick) rebuilds the rows under the same picker: the picker's key is the menu's depth,
  so the cursor, tab and query stay. A feature never reopens a menu to refresh it (that
  replaces the stack and loses the cursor).
- **A thread step has a context for every thread, and its effects are the writer's**
  (ADR-003, framework-2 K1): `ThreadCx::new` always gives one, so a reply and a lifecycle fact
  (`thread_answered`, `on_turn_end`, `on_interrupted`, `on_settled`) reach the feature's part
  whether or not this process still writes the thread (a spinner or a pending flag does not stay
  stuck because a provider refusal took the writer); `ThreadCx::request` is `None` without the
  writer, and a feature's ask (`thread_intent`) is not delivered there.
- **Live thread facts and acknowledgement.** The thread delivers live item completions through
  `on_live_thread_event`, including items buffered while the owner was attaching. A terminal
  fact reaches the same hook after the thread applies its lifecycle and returns queued input.
  Loaded history can reconcile the same turn independently. A feature can retain a terminal fact
  until the view acknowledges delivery,
  including across a thread switch. The `thread_fact` handler can apply that acknowledgement
  after writer loss. It receives only the feature's thread part and the intent. Intents it
  returns proceed through the existing writer check before the effect-capable handler.
- **One turn start per step** (framework-2 K2): `ThreadFx::start_turn` returns
  `Err(message)` when a start was already said in the step (`start_said`), giving the message
  back, so two features' hooks cannot double-start. Whether the model is busy is the caller's
  to ask (it holds the model; for a feature `ThreadView::busy`).
- **The slash door** (fw-hubs-a, P1; `Feature` 14 -> 15). A slice owns the slash commands it
  runs, with their arguments, as it owns the one that opens its sheet: `const RUNS:
  &[SlashCommand]` names them and `fn slash(state, Slash { command, args, source }, cx)` runs one
  (`args` is the text after the command's name, `source` the thread it was typed in). The one list
  line generates `runs_of(command)` and `apply_slash(states, slash, cx)`; the dispatcher
  (`commands/dispatch.rs`) turns every command some feature runs into the one `SessionIntent::Feature
  { args }` and the app routes that to `apply_slash` (`app/session.rs`), so no hub names a slice's
  verb: `/rename <name>`, `/model <name>`, `/mcp verbose`, `/recap`, `/copy`, `/goal ...` are their
  slices' own, and a command nobody runs is answered "not available", as the oracle's stubs are.
  `SessionIntent` 41 -> 25 (`Side` stays until fw-hubs-b moves the side lifecycle). A command
  has one owner: `one_owner_per_command` (`RUNS` and `OPENS` of two slices never overlap, and
  `SLASH_ROUTERS` is empty); `core/tests/slash_door.rs` proves the routes and that every command the
  dispatcher hands to a feature has an owner.
- **Selectors are the slice's** (fw-hubs-a, P3). `AppView` and `ThreadView` are handles, not
  a menu of facts: `ThreadView::core()`, `slots()` and `parts()` give the thread's own domain facts
  read-only, `AppView::facts()` gives `AppFacts` (the moment, the `[tui]` settings, what the status
  screen holds of the account and the catalog, what the host knows of the account's access
  programs). `ThreadCx` carries the same `AppFacts` (`cx.facts`), so a selector written over them
  (`fn plan(facts: AppFacts<'_>) -> Option<PlanType>`) works in an app step and in a thread step. A
  fact one slice wants is that slice's function over the handles (`processes::terminals`,
  `copy::response_sources`, `recap::conversation`, `rename::name`, `small_cmds::daybreak_of`),
  never a method of the view: `view_methods_budget` counts the `pub fn`s of `AppView` (9) and
  `ThreadView` (7) and only lets them go down, and `framework_names_no_slice` makes `cx.rs`,
  `surface.rs` and `remote.rs` name no slice (the exemption `cx.rs` had is gone).
  `app_names_no_feature_state` counts the lines of the app that reach into a slice's state
  (`app.features.side`, `parts.small_cmds`: six, all fw-hubs-b's) and only lets them go down.
  *Not done:* a slice does not read another slice's state. The state a step holds `&mut` cannot be
  lent too, and a snapshot of it before the step would deep-copy the stepped feature's state every
  time (copy on write, `Shared`); no slice needs it today, so a cross-slice read stays a fact or
  an interest, and the first one that needs more adds a handle here with its own decision.
- **Time is told, and a wake is declared** (fw-hubs-a, P2; `Feature` 15 -> 17). Nothing in the
  core reads a clock or ticks. The shell reads the wall clock and tells the app, `AppEv::Clock(Now)`
  (`Now` is milliseconds and the zone's offset: `core/src/app/clock.rs`); the app keeps it
  (`CodexApp::now`, `AppView::now`, `AppFacts::now`) and mirrors it into the screens that word an
  age (`ScreenStates::set_now`), so `Agents::Clock`, `Status::Clock`, `Resume::Clock` and the chat's
  clock event are gone. What wants to be woken says when: `Feature::wake_at(&state) -> Option<(Now,
  Intent)>` and `thread_wake_at(&part)` (the part only where this process writes the thread), and the
  screens' own: `agents::wake_at` (the next minute while the center lists tasks) and `chat::wake_at`
  (the workspace headline's next read). After every step the app
  gives the nearest moment as `AppFx::wake` (a `Wake`, never less than a second ahead, so a wake
  the intent did not clear cannot spin the shell); the shell keeps ONE timer for it
  (`services/shell.rs`: armed when the moment changes, so a stream of keys does not push it back; no
  wake, no timer), tells the clock when it fires, and the app delivers the intents that are due as
  the feature's own, through `intent` / `thread_intent` (Elm's subscription: the time is a message).
  The shell reads the time from a `Clock` (`WallClock`; `FixedClock` for a test, `UiApp::with_clock`),
  a script says it with the `clock` step (`{"step":"clock","secs":N}`, `millis`, `utc_offset`; the
  old per-screen `{"clock":N}` is read as it), and the harness's ingress barrier is a clock event.
  `no_clock_loops` counts what still ticks (a `Clock` variant in a screen's or a slice's enum,
  `AppEv::Clock` named outside the app's arm, the script and the timer, a periodic timer): 0.
  The thread-usage state machine of fw1-c7 (`screens/usage/thread.rs`: `State::next_due` is the
  moment, `tick` the intent) waits for its slice: it declares `thread_wake_at` over `next_due`.

**Enforced** (`tests/rules/layers.rs`, each with a test that bites on a synthetic source):
`hub_enums_only_shrink` (ratchet of the hub enums' variants, `tests/rules/hub_budget.txt`),
`threads_apply_matches_no_screen_or_feature_intent` (the arms of `threads::apply_ev` that name
a screen's thread intent by name: 9 now, to 0 as they move behind `chat::apply_thread_intent`
or into `threads/queue.rs`; a feature's: always 0), `the_feature_trait_has_the_hooks_the_budget_says`
(the methods of `Feature`, the variants of `Out` and of the generated `FeatureThreadIntent`
that are no feature's: a ratchet raised only with a paragraph here),
`view_methods_budget`, `framework_names_no_slice`, `app_names_no_feature_state` and
`no_clock_loops` (the three doors above, each ratchet a line of `hub_budget.txt`),
`features_are_named_only_through_the_list` (no `features::<f>` outside the feature's two
folders and the two lists), `features_hold_no_app`
(no `CodexApp` or `ThreadModel` in `core/src/features`), `feature_slices_ask_only_through_the_outbox`
(a slice names none of `AppFx`, `ThreadFx`, `ThreadAsk`, `ThreadSet`, `apply_app`, `AppEv`,
`Intent::Effect`, `Command::Effect`; `ThreadCx` keeps its `fx` private), `thread_rpcs_only_in_threads` (the lifecycle list everywhere; the
feature RPCs `compact`, `rename`, `shell`, `revert` also inside a slice), and the existing
layer rules, which cover `core/src/features` (core pure) and `src/features` (a view folder:
no protocol, no model code, a known top-level folder).

**Departures: none.** Step 0 adds the framework and no behaviour (core's 1509 tests, the
rules and the oracle shards are unchanged; the real list is empty). The existing features
(`/goal`, the queue, `/export`, ...) migrate when convenient, and the budget records how far
that has gone; a feature that has not moved is still in the hub and counted there.

**Edges known at step 0** (decide in the first slice that meets them): a feature RPC on a
thread must be listed with `Command::Screen` where the host routes by thread
(`host/src/in_process.rs` `asks_thread`, `host/src/cloud.rs`; `Command::Feature` is, since the
`/goal` move); a feature that changes a `ThreadCore` field it does not own gets a `ThreadCx`
effect for it, in `cx.rs` (an outbox the thread carries out, one decision each; the queue does
not need one: it is not a slice); the words of a note see the session strings only
(`FeatureView::note_words`), a feature that needs another table adds its own; a host effect
(`/export`'s file write is `effects::Ask::Export`, no `ClientRequest`) has a door since the
early review's R2: `AppCx::effect(ask, ctx)` and `Feature::effect_done(state, ctx, done, cx)`,
below.

**The stream: a typed `Event` per feature (fw-hubs-b, P4).** A feature that follows the server's
event stream says what it makes of a notification in its own type: `Feature::Event`,
`project(&ServerNotification) -> Option<Event>` and `project_request(&ServerRequest) ->
Option<Event>` (pure, no state, no thread; the notification still goes to the threads as before:
the event is the feature's own copy of the part it needs), delivered to `on_event(state, event,
cx)` with an `AppCx`, in list order, once per notification or request (`features::observe`,
`observe_request`). A server request is observe-only: the feature never answers it (the app asks
the user as before). A feature with no event says `Never<Self>`. The side slice is the first user
(the parent's status: `StatusChange::for_notification`, `status_for_request`); the app's hand-called
`side::observe`/`sync` are gone. There is no `ThreadEv::FeatureEvent` (a thread-scoped event for a
`ThreadPart`): no slice has a consumer for one, and a hook with no consumer is the kind this ADR
refuses; the first thread-scoped follower adds it. Mcp startup and `McpLagged` still go through
`threads::project` (they are a turn-pipeline interlock of the thread, `McpStartup` in `send_next`,
not a feature's reading of the stream), and so do Goal, compaction and processes; moving them
needs the chat screen's own part of the interlock to move too (a Need for the chat tasks).

**Lifecycle doors (fw-hubs-b, P5; ADR-003 stays).** The app owns every `ThreadModel`; a feature
that wants a thread opened, made or dropped says so through `AppCx` and the app runs it through
its own steps (`app/lifecycle.rs`): `open_thread(&id, ctx)` (a thread this process holds or lists
is shown, one it does not know is read first, so the server's "no such session" is answered
rather than opened), `spawn_thread(ThreadSpawn::{Start | Fork { parent, ephemeral }}, ctx)` (the
fork of `/side` is `Fork` with developer instructions: ephemeral, empty on the screen; named
`spawn_thread` because rule R1 bans `cx.spawn(`), `discard(&id)` (the turn is interrupted, the
server stops holding the thread, the app forgets it, never the thread in use), `submit(&id,
message)` (the thread's own input pipeline) and `give_back(&id, text)` (text that could not go
returns to the composer). Open and spawn are answered in `Feature::thread_opened(state, ctx,
Result<ThreadId, Refusal>, cx)` under the feature's own `FeatureCtx`, before the thread is shown
(`Refusal::Busy`: one conversation is made at a time; `Failed(RpcError)`). The app tells every
feature when the thread in use changed (`on_shown`), and what the chat reads of it that a feature
knows (a side conversation's flag and footer) comes back as `AppCx::chat_facts`, which the app
writes to the chat (`Out::Chat`); the key label the footer names is read through
`AppFacts::key_label`. `AppCtx` names no feature (`app_ctx_names_no_feature`: the generic
`FeatureCtx` rides in it). Inside a thread step a feature has `ThreadCx::start_turn(message)`
(one start per step; the thread must be idle, `ThreadView::busy`) and `ThreadCx::interrupt()`.
`/resume <id>` is `open_thread` (the settings slice no longer reads the thread itself, which
turned `thread_rpcs_only_in_threads` green).

**Structured turn (fix-recap-title; ADR-003 stays).** An isolated request to the model, the
oracle's `temporary_structured_request`, is a lifecycle door: `AppCx::structured_turn(StructuredTurn
{ kind, source, model, effort, prompt, schema }, ctx)`. The app runs it (`app/structured.rs`), as an
ephemeral thread that never becomes a `ThreadModel`, in the thread RPCs of `threads::ThreadAsk`
(`McpServers`: `config/read` for the servers to disable, `StartTemporary`: `thread/start`,
`StructuredTurn`: `turn/start` with the output schema, then `Unsubscribe`; the wire requests are made
in `threads.rs`, from the data of `threads/structured.rs`). The thread starts read-only with no tools,
no environments and every MCP server of the config disabled (a title by the built-in read-only
profile, `never` approval; a recap by the active profile when it is a named one), and one that did
not start as asked is let go before any turn runs. The last assistant message of the turn comes from
the server's stream (`structured::observe`, which keeps what the stream said before `turn/start`
answered), and is told to the asking slice as `Feature::structured_done(state, ctx, Result<String,
StructuredFailure>, cx)` under the feature's own `FeatureCtx`, whatever happened (`Refused`,
`NotReadOnly`, `Ended`, `Empty`, `TooLong`, `TimedOut`, `Lost`); the temporary thread is
unsubscribed after every end (and the turn interrupted when it was the clock that ended it). A run
is a record of the app (`app.structured`) with its own number: the ticket of its RPC in flight says
which run it answers (`AppCtx::Structured`), the feature's `FeatureCtx` rides in the run, and a
phase that takes longer than 30 seconds ends it by the clock (`wake_at`, `expire`; core reads no
clock). A lost connection ends a run that waits for its stream. `/recap` and the name prompt's title suggestion are the first users
(the title asks `gpt-5.6-luna` with a low effort on a ChatGPT account whose catalog has it; no
suggestion for a cloud conversation or one with no message to name it by); `Ask::StructuredTurn`,
`Ask::SuggestTitle` and their `Done`s are gone. Not built: the automatic title of the first message
(`ThreadTitleDestination::Automatic`) and the cloud refusal of `/recap`.

**Connection and stream loss (fw-hubs-b, P8).** `Feature::on_connection(state, Connection, cx)`
tells every feature that a connection came up (`Up`: the first or a reconnect), was lost (`Lost`:
what waited for it will not come) or that the host is quitting (`ShuttingDown`: what a feature
holds on the server is let go; the host's word is `AppEv::ShuttingDown`). A thread's event stream
can be lost while the connection stands (`BackendEv::StreamLost{thread, error}` /
`StreamRestored(thread)`, owner-checked by backend; host/src/cloud.rs forwards them, the in-process
backend has no per-thread stream and shows the disconnect as an `Error` line): an idle displayed
thread asks the host to reconnect (`Command::ReconnectIdleThread`, the error line only if that
fails), a thread with a user turn shows the `Error` line; opening a thread whose stream was lost or
whose history may be incomplete resumes it again;
`ThreadEv::StreamLost` sets `ThreadCore::stream_lost` (`ThreadView::stream_lost()`), and the queue
interlock follows the oracle's `thread_stream_disconnected` (`chatwidget/input_flow.rs`): while it
stands `Submit` queues instead of sending or steering and `send_next` does not send;
`StreamRestored` clears it and the queue goes on if no turn runs.

**Doors of the second round (doors-r2).** Four hooks of `Feature`, each with a default that does
nothing: `structured_wanted(state, &ctx, view)` asks the feature that asked for a structured run
whether its answer is still wanted just before `turn/start` (a recap of a thread that moved on says
no, and the run ends without a turn); `on_user_message(state, thread, text, cx)` tells every
feature each user message of a thread as the thread takes it (the automatic title waits for the
first); `on_rate_limits_read(state, backend, &RateRead, cx)` tells every feature an account read of
the limits, after the model took it (`AppFacts::rate_limits()` and `AppFacts::backend_banner()` read
what the app holds); `fault_words(state, thread, message, view)` lets the first feature, in list
order, word the error of a turn of its own thread. `PublicVerb::NotifyOwner(Nudge)` is the one
path of the add-credits e-mail (the banner's action goes through it too). `AppCx::quiet_interrupts`
(`Lifecycle::QuietInterrupts`, `ThreadEv::QuietInterrupts`) makes one thread say no interrupted-turn
notice: a side conversation's. Two selectors keep the hub from naming a slice: the app asks
`features::setup_before_pick` (the Windows sandbox's enable prompt before an Auto pick, an
`Intent::Prompt` that lets the pick go when the setup is done: `PublicVerb::PickPermissions`) and
the presenter asks `FeatureView::in_place(wish, thread_ui)` (the copy mode that `/copy` begins on
a transcript the chat owns, instead of the picker).

**The host's doors (`host_capability_list!`, fw-hubs-c P6 and C2 F5).** The app emits
`Command::Effect` with a named `effects::Ask`. Features use `AppCx::effect` and receive
`AppEv::Effect` under their request number. The capability catalog in `core/src/app/effects.rs`
generates `Ask`, `Done`, `Ask::thread`, `HostDoors`, and the dispatcher. A field marked
`by <field>` selects the owning connection for a thread effect. A capability with its own
continuation keeps that continuation's typed event or ticket. An executor can handle a command
without emitting an event, which also prevents the dispatcher from trying another executor.
The local services, the connection, and the fake each implement `HostDoors`.

Startup and import continue to register their typed requests before dispatch. An ask using the
app-server handle carries its physical transport epoch. Connector-owned setup can replace that
handle and pass the restart acknowledgement to its original continuation. Popovers carry the
conversation captured at submission. The browser door reports either its feature result or an
error line. The account's reset-credit mutation uses the ordinary RPC queue with the picker's
backend, the original idempotency key on retry, and the existing timeout. The tests switch
backends and replace the transport before accepting the current receipt.

The status line uses a read-only `GitSummary` capability for branch and pull request metadata.
The app captures the shown thread, cwd and physical address when it registers the request. It
accepts the reply only for that pending workspace. The connection runs bounded probes through its workspace
executor. The hub budget allows 10 `Command` variants and 30 entries in the capability catalog.

**The host's feeds: `host_feed_list!` (fw-hubs-c, P7).** What the host tells a slice unasked (the
config's facts at start and after a write, the account's, the notices the server holds) is the slice's
own host half: `host/src/feeds/<f>.rs` has one function, `feed(event, cx, out)`, and a line in
`host_feed_list!` (`host/src/feeds.rs`). `in_process.rs` names no slice: it says what happened
(`feeds::Event::{Startup, Reconnected, ConfigWritten, Notification}`) and delivers what comes back
(a `Feed::Fact` now, a `Feed::Job`, a read of the server, when its answer is there). The rule
`host_half_names_one_feature` makes a feed name its own slice and no other, and
`features_are_named_only_through_the_list` reads every other host file as one that names none (the
`FACT_SENDERS` exception is gone; `host/src/settings.rs`, which named `features::settings`, is the
settings feed, and the write of the config file stays in `settings.rs`).

**Public verbs (fw-hubs-c, P9).** What a feature asks of an owner outside it and does not merely want
shown is `AppCx::verb(PublicVerb)` (`Out::Verb`): a short whitelist (`core/src/features/verb.rs`),
verbs and never gestures, carried out by the app in the owner's own step (`app/verbs.rs`). Today:
`StatusOverride` (`/title` and `/statusline` with items: the chat screen's session-only override) and
`PickModel` (`/model <name>`: the app's model pick, as the sheet's own pick is carried out). A
`Wish` is for showing something once (`KeymapConflict`); a verb changes something, so the UI no
longer relays the slice's words to the app. `OpenThread` was already `AppCx::open_thread`.

**Reachability by path (fw-hubs-c).** The adr013 draw-state scan resolves a field's type by its path
when it has one (`safety::Ev`, `crate::threads::safety::Ev`) and by the owner file's `use` when it
has not (`use crate::threads::safety::Ev` then `Ev`): the definitions of that name are narrowed to the
files under the module the path names, and a path no file fits stays whole (more than the truth, never
less). Two types of one name in two modules are therefore no longer one node, and a type is not
renamed (`SafetyEv`, `CompactionEv`) to keep the scan apart. Test:
`the_reachability_resolves_a_name_by_its_path`.

**Registration by file (fw-hubs-c5).** No list is a hand-written line: `build.rs` (core's, and the UI
crate's) writes every registration list from the files, with one scanner, `test-mods/registry.rs`
(std only; the rules call the same functions, so a rule reads what the compiler got). A slice is a
directory `core/src/features/<f>/` with a `mod.rs` that holds `impl Feature for <Marker>`: the `pub mod`
line, `feature_list!` (by name: no hook of the slices depends on the order, the two that look at it,
`on_resumed` and `on_shown`, are one slice each), the UI's `src/features/<f>` module line and
`sheet_list!` (the slices whose `const OPENS` is `Some`) follow. A key action is a file of
`key_actions/` (`impl KeyAction for <Marker>`); a transcript mode is a file of `transcript/` with
`impl TranscriptMode for Mode`; a cell is a file (or directory) of `cells/` whose head holds the
line `// cell: <Item pattern>`, e.g. `// cell: Item::Error { .. } | Item::HighLoad`: that line is
the `cell_list!` entry, the file name its module. `core/tests/all.rs` is `core_all!` (every file and
`mod.rs` directory of `core/tests`, as the UI roots are). Two parallel packages that each add a slice,
action, mode, cell or test file touch no common line. rustfmt does not follow modules a macro
declares: `tools/rtwk-port/registry/fmt.sh` formats them. A slice is written with `feature!`
(`core/src/features/mod.rs`): `impl Feature for <F> { const NAME..; type X = ..; { fn .. } }` with
only what the slice has; an absent associated type is `Never<Self>` (`()` for the two states),
`wire`/`decode` of a slice without `type Command` are `command.absurd()`, every other method the
trait's default. Constants come first (`NAME`, `SEED`, `OPENS`, `RUNS`), then the types in the
order of the trait, then the methods in one braces group; rustfmt leaves the inside of the call as
written. Test: `the_lists_are_generated_from_the_files`, `the_feature_scan_bites`,
`every_cell_view_has_its_head_line`, `core_tests_are_listed_by_their_files`.

**The wire is generated (`rpc_list!`).** A slice's RPCs are one `crate::rpc_list! { pub enum Command => Reply [, request thread: &ThreadId] [, reply ctx: &T]; ... }` in the slice (core/src/protocol/rpc_list.rs). Each line is one RPC: the ask's variant and the pattern binding its fields, `=> Method(params)` (the `ClientRequest::Method`'s params, an expression over the bindings and the request binder), `-> Response, |answer| reply` (the protocol's response type and what the slice reads of it). The macro writes the ask enum, `request` and `reply` (a result that does not parse is `RpcError::Malformed`), so the slice's `Feature::wire` and `decode` are one call each and nothing else matches an ask to a request. The `Reply` stays the slice's own value (what it reads; several asks may share a variant), so `FeatureReply` keeps its `Eq`. The protocol crates get no per-method trait; the list is ours, and a new RPC is one line (about 6 to 10 lines, not 40). A protocol data type a slice reads as it is, is re-exported through `core::data::wire` instead of mirrored by an enum of ours. No `features/*/wire.rs` and no hand-made `ClientRequest` in `core/src/features/` (rule `wire_is_generated`; a ratchet names the three slices still to move). A host-sent RPC that is no slice's (the usage reset's consume) is a list of its own in core (`screens::usage::reset::Consume`), which the host reads for request and reply.

### Recipe: a feature as a slice

For a flow that is not a screen: a slash command that opens a menu or a prompt, asks the
server and says what happened in the transcript (`/goal`, `/export`, `/rename`). Thread
lifecycle state (the queue, the steers, the turn starts) is not one (see "What is not a slice").
A screen is still a `screen_list!` directory.

**Files.** You edit only these (no list line: the directory registers the slice):

```
core/src/features/<f>/mod.rs     the marker type and `impl Feature for <F>`; model.rs, menu.rs, wire.rs, note.rs as it grows
src/features/<f>/mod.rs          `impl FeatureView for <F>` (the same marker, from codex_tui_core)
src/features/<f>/strings.rs      the words of its notes and menu rows (a typed const table, ADR-010)
core/tests/<f>.rs                tests through `CodexApp`, as the oracle's tests are
```

**The list** (`feature_list!`, written by `core/build.rs` from the directories as `goal => Goal;`:
the directory => the marker type of its `impl Feature`). It generates `FeatureStates`,
`FeatureThreadParts`, the root enums `FeatureIntent`, `FeatureThreadIntent`, `FeatureCommand`,
`FeatureReply`, `FeatureCtx`, `FeatureMenu`, `FeaturePick`, `FeaturePrompt`, `FeatureNote`,
their `From` impls and the dispatch, in core and in the UI. No other hub file changes for the
feature. A variant added to `Menu`, `NoteKey`, `PromptKind`, `ThreadEv`, `Command`... fails
`hub_enums_only_shrink`: the case is a variant of your own type.

**The trait.** `impl Feature for Goal`: `const NAME`; types `State` (app-scoped, a field of
`CodexApp`, yours alone), `ThreadPart` (per thread, a field of `ThreadModel`; `()` if none),
`Intent` (from the slash router or a view), `ThreadIntent` (to this thread's part, applied by
`threads::apply`), `Command` / `Reply` / `Ctx` (your RPCs as data, and what one was for),
`Menu` / `Pick` (the generic select dialog and what it settled), `Prompt`, `Note` (a
transcript line; its words are `src/features/<f>/strings.rs`), `Fact` (what the host sends
unasked, `AppEv::Feature`).
- Every component type is the feature's own, so the root enums' `From` impls do not overlap
  (two features sharing `String` as a note would not compile). A component you do not have is
  `Never<Self>`; the two states are `()`. Every step but `wire` / `decode` has a default that
  does nothing; a feature without RPCs says `command.absurd()` (`Never::absurd`) in those two.
- The outbox, `AppCx`: `ask(thread_or_none, command, ctx)`, `open(menu)`,
  `prompt(prompt, initial)`, `note(thread_or_none, note)`, `thread(&id, thread_intent)`,
  `settled(&id)`; the
  app carries them out after your step, in the order you said them. `cx.view()` is the app
  read-only: `active()`, `thread(id)`, `active_thread()`, `tui()`, `screens()`, `modes()`;
  a `ThreadView` has `core` and `slots` (read-only), `id()`, `is_running()`, `busy()`,
  `has_writer()`.
- On a thread (`ThreadCx`, in `thread_intent`, `thread_answered`, `on_turn_end`,
  `on_interrupted`): `cx.view` is the thread read-only, `&mut` is your `ThreadPart` only,
  `cx.request(ctx, command)` is the RPC (`None` where this process does not write the thread;
  replies and lifecycle facts reach you there too, an ask does not). More effects (start a turn, give a message back) are methods you add to `ThreadCx`
  in `cx.rs`, each one a decision (a start must refuse while `cx.view.busy()`: the thread
  arbitrates starts, so no hook order can double-start).
- A menu: `choose(menu, row) -> Choice::{Stay, Dismiss, Pick(p), Replace(m)}` says what a row
  did in your terms (`Pick` closes the menu and `picked(state, pick, cx)` acts); the rows are
  the UI's (`FeatureView::menu_list`).
- A slash command stays in the oracle's list (`SessionIntent`); the router
  (`core/src/app/session.rs`, the one file besides your own that may name
  `features::<f>::Intent`) turns it into `Intent::Feature(<f>::Intent::..)`.

**Hooks.** `on_fact(state, fact, cx)` (the host sent a fact unasked: keep it, the open menu reads
it), `on_resumed(state, thread, cx)` (a resumed thread is applied and in use),
`on_turn_end(part, end, cx)` (completed, failed or interrupted; `busy()` is as the hub left
it), `on_interrupted(part, cx)` (right after the `on_turn_end` of an interrupt),
`on_settled(part, cx)` (what held the thread's input back is over: another feature said
`cx.settled(&thread)`, the goal was set; the queue, the thread's own, resumes beside them). They run over
all features in the order of `feature_list!` (by directory name; a feature does not depend on it). A feature that
needs another to go on never names it: it says the event (`cx.settled`), the hooks of every
feature answer in their own words (`on_settled` was the one hook the `/goal` move added,
because the goal-to-queue nudge was a `QueueResume` that named both). A new hook is for a
lifecycle fact only (see "What is not a slice").

**The view half.** `impl FeatureView for Goal { fn note_words(note, &SessionStrings) -> String;
fn menu_list(menu, &State, &SelectStrings) -> List; fn prompt_words(prompt, &DialogStrings) ->
PromptWords; }` in `src/features/<f>/mod.rs` (the prompt dialog words a `PromptKind::Feature`
through `features::prompt_words`: its title and placeholder). Views only: no
protocol, no model code, no `apply` (`views_never_name_protocol_shell_tokio_or_io`,
`screen_dirs_hold_view_files_only` cover `src/features/`). A feature with a page, a sheet or a
widget of its own builds it in `pages/`, `sheets/` or `widgets/` as any screen; a menu row or a
note leads to it by an intent.

**Moving an existing feature in.** The step list with the `/goal` move worked in full is
[refs/feature-slices.md](refs/feature-slices.md) and its mechanical half
`refs/slice-codemod.py` (`new`, `move`, `hub`, `budget`). The tests stay as they are. The model and steps go to
`core/src/features/<f>/`; the variants it had in `Menu`, `Picked`, `PromptKind`, `NoteKey`,
`Command` / `ThreadAsk`, `ThreadEv`, `chat::ThreadIntent` become your own types; its arms in
`apply_choice`, `picks`, `answered`, `prompt_submitted`, `threads::apply_ev` go with it; the
wording goes to `src/features/<f>/strings.rs`. Then lower the line in
`tests/rules/hub_budget.txt`: the ratchet fails until the saving is written down. The
feature RPCs `ThreadCompactStart`, `ThreadSetName`, `ThreadShellCommand`, `ThreadRevert` may be
built in your `wire`; the lifecycle RPCs never (`thread_rpcs_only_in_threads`).

**Tests.** `core/src/features/framework.rs` shows the pattern with two probes in a list of
their own: dispatch to the feature's own state, an answer of another shape is
`RpcError::Mismatch`, hooks in list order. For your feature, go through `CodexApp` as the
oracle's tests do: apply the intent, read the `AppFx` commands and the outbox's result (a
menu on the select screen, a prompt in the dialogs, a note in the thread), feed the reply,
check the state. `acargo nextest run -p codex-tui-core`, the `rules` root, your oracle shard.

## ADR-013 — The core/UI boundary by the IO test

Accepted by the person 2026-11-23 ("по рекомендациям, максимально архитектурно правильно").
The full document is the normative text:
[research/arch-boundary/full/architecture.md](research/arch-boundary/full/architecture.md)
(17 sections; diagrams d1-d15 in `full/diagrams.md`; adversarial review F1-F17 in
`full/review.md`, blockers F1-F4 resolved in the document). This section is its summary;
where they differ, the full document wins.

**Decision (option C).** Where a value lives is decided by the IO test (full §1.3), first
"yes" wins: (1) a server/host fact or derived from one → core, domain (Д); (2) a request
parameter or op status a reply attaches to → core, record (З), kept as a cache by key
(`Map<Key, Remote<T>>`), never "the current X"; (3) a lifecycle step or a one-shot signal the
domain sends a surface (`SurfaceWish`, `Directive<T>` with `seq`) → core (Ж); (4) whether
data is wanted now → `Interest` counter in core, the open fact in UI (И); (5) otherwise →
drawn state (Н) in the UI: `State` of the widget or `ThreadUi`. Field text is always Н; core
gets it only in a verb (`Submit`, `Search{query}`). "Who writes it" decides, not "who reads".

**`core::data`** — plain domain values shared by kit, screens and widgets; depends on std/protocol/ids (`adr013_data_names_only_ids`). **`core::kit`** names no screen or feature (`adr013_kit_names_no_screen`); hardwired keys are named actions of `core::keymap::fixed` read by `pressed` like any action (`adr006_keycode_only_in_keymap`).

**Recommendations taken (full §16).** O1 sheet pages are routes of a nested `Navigator`
wrapped in `IntrinsicHeight` inside the sheet's max-height cap (re-decided by the person
2026-11-23 after the rtwk orchestrator showed this is twk's supported way: rtwk's Navigator
reports intrinsic height from onstage routes only, navigation/navigator.rs `most`; each page a
route with its own `State`, Esc pops; `SheetStack` of full §6.2 is dropped; a mismatch with
the oracle goes to rtwk as a repro); O2 bottom-pane views stand in a docked `SheetHost` slot of the session
page (check the oracle snapshot first; until then the modal routes stay); O3 pure helpers in
`core::kit`; O4 ops keyed by a domain key (+ `gen`), never a UI correlation id; O5 the rules
first (`adr013-rules`), then the `/plugins` pilot; O6 the 9 doors of WIP `6cafa8de1` are not
merged, its domain goes into the pilot; O7 `Rc`/`Arc` left to `session-core`.
`95826e191` (/rename, /export) lands as is. Transcript text selection is drawn state: it lives in the session page's
UI (`SelectionArea` + its controller), no `FeatureView` hook; only the copy verb and its
`Done::Copy` answer cross into core (the person, 2026-11-23). Until rtwk's `clipboard-provider`
lands, `SelectionArea`'s own OSC 52 write is a known diff against the oracle.

**Review coverage.** review.md F1-F4 (blockers) are resolved: F1 by O1 (nested `Navigator` in
`IntrinsicHeight`), F2 `SurfaceWish` (full §4.4), F3 `OpKey` + `gen` (§4.5), F4 `Interest`
(§4.7). F5-F12 are resolved in the full text (§6.3-6.4 thread switch and `alive`, §5.3
reconnect + rule, §4.6 directives, §13 kit rule, §12 fixtures `ui` step, §11 + watch ratchet,
O2 `SheetHost`, §7 + `KeyCode` rule); F13-F17 by §1.3 clarifications, §14 and §15.

**What changes in earlier ADRs** (full §14): ADR-001 core holds the domain of screens and
features, `kit` is a core module; ADR-002 amendment 2026-11-18 ("core view stack") is
withdrawn; ADR-003 gets the IO test, the five kinds, `SurfaceWish`/`Directive`/`Interest`/
`OpKey`; ADR-006 adds `KeyCode` only in the keymap; ADR-010 feature menu/prompt words move to
`src/features/<f>/strings.rs`; ADR-012 `Feature` has no UI forms (`Menu`, `Pick`, `Prompt`,
`choose`, `picked`, `prompt_submitted`, `AppCx::open/prompt` go), gains `Wish`, `Interest`,
`on_interest`; the visible half of a feature is `open(cx, args)` and its own sheet.

**Enforced by** the rules of full §13 (`adr013_*`, ratchets where migration goes in steps),
written by tm `adr013-rules` before the pilot. Until then and for any family not yet moved:
no NEW `Menu/Pick/Prompt`, gesture intents (`Choose`, `Toggle`, `Cursor`, `Expand`, ...) or
`FeatureView` hooks; new drawn state goes to the UI.

**Departures while migrating** (process.md A4.1): the `select::Menu` families, `settings`,
`dialogs`, `resume` (density, expanded), `pets` (highlight) keep Н in core until their
migration step (full §15 steps 2-4); the semantic half of the IO test (key vs cursor) is a
review checklist item and a debt task.

## Skeleton (tm `port-skeleton`, layered by `skeleton-layers`)

```
codex-rs/tui-rtwk/          own workspace (members: core, host), package codex-tui-rtwk, binary codex-rtwk
  Cargo.toml                codex crates by path, rtwk by path; codex-rs [patch] copied;
                            [workspace.package]/[workspace.lints]; passes the manifest script
  Cargo.lock                seeded from codex-rs/Cargo.lock
  clippy.toml               R1/R11/R12/R14 disallowed lists (UI crate; core/clippy.toml: the same without rtwk paths)
  core/                     codex-tui-core: no rtwk, no direct runtime or IO dependency
    src/app.rs              CodexApp (model), AppEv, Command, apply_app, ingest (epoch drop)
    src/threads.rs          ThreadModel, Transcript, apply, ThreadFx, project (protocol -> ThreadEv), Failure
    src/ids.rs, rpc.rs      ThreadId, Ticket, RequestKey, ConnEpoch; Reply, Answer, RpcError
    src/wire.rs             protocol requests and replies the host sends (client_request, reply, dismissal)
    src/screens/<dir>.rs    22 files, one per screen dir: Model, Ev, Intent, ThreadPart, Command, Reply, Ctx, apply, wire, decode
    src/fixtures/           builders (protocol payloads), script (JSONL event scripts: seed/notify/request/key/resize)
    tests/model.rs          apply on the plain model; tests/script.rs: the fixture loader
  src/app.rs                UiApp { core: CodexApp, ui: UiState { nav_key, thread_ui, lang, debug }, link: shell::Link }, ingest, send
  src/run.rs                composition root: run_app_with, passes the shell's dialog watcher into the view
  src/shell.rs, shell/      the only effect runner and spawner (client, tickets, epoch stamping, fake backend); names no widget
  src/theme.rs              style tokens from the rtwk ThemeData at the root (no literal colours)
  src/l10n/                 ADR-010: Lang, Tables<T>, the shared strings.rs (EN + a RU proof); each screen dir has its own strings.rs
  src/ui/view.rs            root: Theme, MultiProvider (model projections, handlers), Navigator (home: chat), request presenter
  src/ui/{presenter,thread_ui,debug_menu,seeded}.rs   request presenter; ThreadUi controllers (R14: per-thread holder); debug menu; a seeded model for an entry
  src/screens/<dir>/        22 view dirs: widgets, strings.rs, debug entries, fixtures/*.jsonl; no model, no protocol
  host/                     the binary's backend (in-process app-server) and main
  tests/screens.rs          Tester over the app on a fake backend
  tests/pin.rs              rtwk HEAD == RTWK_REF; [patch] equals codex-rs's
  tests/rules/layers.rs, rules.rs layer and R-rule scans (one test per rule)
  tests/fixtures.rs         debug entries over fixture scripts
```

Done when: `codex-rtwk` starts against the in-process app-server, shows the
empty chat screen, sends one prompt to the mocked model and shows the
streamed answer as plain text, quits; `tests/screens.rs` and `tests/pin.rs`
green; clippy with the R-rule lists clean;
`verify_cargo_workspace_manifests.py` passes; the codex-rs workspace and
lockfile untouched.

## Layers (tm `skeleton-layers`)

Four layers, one direction of knowledge. `core` (no rtwk, no direct runtime or IO
dependency) holds the models, `apply`, the protocol mapping and every screen's model,
events, intents and RPCs; failures are typed values and the words are the UI's
string tables'. The views (`src/screens/<dir>/`, `src/ui/`, `src/l10n/`,
`src/theme.rs`) are widgets that read the model through providers and send
intents; they name no protocol, no shell, no tokio, no IO and never call
`apply`. The `shell` is the only effect runner and spawner and names no
widget. `run.rs` and `host/` are the composition: they name all of them. The UI model
is `UiApp { core: CodexApp, ui: UiState { nav_key, thread_ui, lang, debug, presenter }, link: shell::Link }`:
what only a widget keeps is in `ui`, never in `core`; `link` is the app's end
of the effect runner, the one field that names the shell, which is why `app.rs`
is the model root and not a view.

| the layer | may name | may not name | enforced by |
|---|---|---|---|
| core | protocol crate, serde, std (no IO, no clock, no threads) | rtwk, tokio, futures, UI types, IO | the compiler (no rtwk in `core/Cargo.toml`) and an allowlist of its direct dependencies (the protocol crate pulls tokio in transitively, so it is the sources that name none); `core/clippy.toml` (R1/R11/R12 without rtwk paths, plus `disallowed_types` for `File`, `TcpStream`, `Command`, `Stdout` and the protocol's generators); `layers.rs`: `core_manifest_has_no_ui_or_runtime_dependency`, `core_sources_name_no_ui_runtime_or_io`, `core_sources_hold_no_user_visible_prose`, `the_ui_model_is_the_core_model_plus_ui_state` |
| view | core, rtwk widgets, its own strings | protocol, `core::wire`, shell, tokio, IO, `apply` | `views_never_name_protocol_shell_tokio_or_io`, `views_never_apply_the_model`, `screen_dirs_hold_view_files_only`, `ui_crate_has_only_the_known_folders`; R14 and ADR-010 in `rules.rs` |
| shell | core, the protocol, tokio, rtwk's `Cx` and `DialogResult` for spawning | widgets, views, theme | `shell_names_no_widget`; R1 in `rules.rs` (`r1_spawn_paths_are_the_shells`) |
| run, host | everything | - | the composition root; `r1_the_host_neither_spawns_nor_allows` |

A screen directory is two halves of one name: `core/src/screens/<dir>.rs`
(model, events, intents, RPCs, protocol wiring) and `src/screens/<dir>/`
(view files only). One macro list (`screen_list!`) makes both, so a directory
is registered once, and a slice edits only its own two halves.

A core feature that is a flow, not a screen, is a slice of the same shape in `features/`
(ADR-012): `core/src/features/<f>/` (model, steps, RPCs, notes, hooks) and
`src/features/<f>/` (words and views), registered once in `feature_list!`. The layer rules
cover both folders as they cover the screen directories.

**Fixtures.** A screen's mock state is a JSONL event script in its view
directory (`fixtures/*.jsonl`): `seed` (a ready session), `screen` (an
event of one screen directory, read by that directory's own `seed` in
`core/src/screens/<dir>.rs`, so a slice never edits the loader),
`notify` and `request` (the protocol's own JSON, taken from the oracle's tests),
`key` and `resize` (inputs for a tester). Core's `fixtures::script` plays it
through `CodexApp::ingest`, the live path, so a script cannot build a state the
app cannot reach. A `DebugEntry` carries its script; the debug menu opens a
`SeededScope` (`ui/seeded.rs`) that plays it into a core `Sandbox`, provides
that model and its own handlers, and shows the real screen. What the user does
on it is applied to the sandbox, which drops the effects: nothing reaches the
live app. Worked examples: the chat's streamed answer, an exec approval
request, a resume list from fake rollouts. Details and the oracle map:
[parity/skeleton-layers.md](parity/skeleton-layers.md).

**The core as an upstream candidate** (tm `core-upstream-ready`, decided with
the person 2026-11-18). The path to upstream is to offer the UI-free core, not a
framework switch, so the core is named for what it is, `codex-tui-core`, a
frontend-neutral crate that any Codex terminal frontend could sit on. It holds
what no frontend should own twice: ids and RPC tickets, the app and thread
models with `apply` (events in, a new model and typed effects out), the mapping
from `codex-app-server-protocol`, every screen's model, events, intents and
RPCs, command parsing and the fixture scripts. What makes it acceptable to
offer: it names no rtwk (the compiler holds that), no direct runtime or IO
dependency (only the protocol crate, serde and test helpers; its sources name no
tokio, clock, thread or file call, and `core/clippy.toml` bans them), no user
visible prose (strings are the frontend's), and it meets codex-rs's own
conventions: `tools/rtwk-port/core-upstream-check/core-upstream-check` copies
codex-rs into a temp dir, adds the core as a member (its path
dependencies become `workspace = true`; its package name is checked against
codex-rs's `codex-<dir>` rule for `codex-rs/tui-core`), and runs codex-rs's manifest policy
script, `cargo fmt --check`, `cargo check` and `cargo clippy -D warnings` with
codex-rs's `clippy.toml`, `rustfmt.toml` and `[workspace.lints]` plus
`print_stdout` and `print_stderr` denied, and the core's tests, with no
rule it cannot meet (it passed unchanged on 2026-11-18); the real
`codex-rs/Cargo.toml` and `Cargo.lock` are never touched. One question stays
open and `session-core` decides it: the model shares slices as `Rc<T>`, which
keeps the core `!Send`; codex-rs's runtimes and an upstream session layer would
want `Arc<T>` (`Send + Sync`), at the cost of atomic counts on every
slice share (ADR-003's measurements of the model's cost are the place to start). Until then the core is a candidate, not an
offer.

## rtwk gaps (work in the rtwk repository, its own tm)

| gap | needed by |
|---|---|
| G1 enter / leave the alternate screen at runtime from an inline app | inline overlays (pager, diff) |
| G2 re-emit the app's own committed scrollback on resize | inline resize reflow |
| G4 image placements inside a region, if rtwk-rich `Image` falls short | pets, `view_image` |
| G5 Zellij and Windows Terminal scrollback strategies | inline on those terminals |
| G7 committed rows keep OSC 8 links and images | inline history |
| G8 discard pending input | typeahead quarantine before approvals |
| G9 selection, search and copy-source over a virtualized transcript | S10, S16 |
| G10 tmux probes (mouse option, pane-local graphics) | S10, S24 |
| G11 clipboard breadth (native, WSL, X11 PRIMARY beyond OSC 52) | copy |
| G12 `TextArea`: atomic elements, kill ring and yank, vim mode, paste-burst, history recall (arch-review M1) | S10 composer |
| G13 `show_dialog` without a Navigator falls back to the `overlay::App` overlay above the app's providers (nav-proto gap 1) | fixed in rtwk `9f28b7fb`: the Navigator home is a memo node whose handle the presenter shows requests from (`show_dialog_from`) |
| G14 a provider projection lags the model until the view reruns; no same-flush read (nav-proto gap 2) | presenters; workaround: never decide from a same-flush read |
| G15 changing the provider list silently drops an unkeyed Navigator's stack (nav-proto gap 3) | root; workaround: fixed list + `GlobalKey` |
| G16 `show_dialog` has no on-closed message: the caller awaits the `DialogResult` future, so a spawned watcher is needed, and spawning is the shell's (R1); hence `shell::dialog_closed` and the injected `CloseWatcher` | fixed in rtwk `9f28b7fb`: `AppCx::show_dialog(builder, options, closed)` / `show_dialog_from`; the request presenter (`ui::presenter`) decides on the model in `closed`; `shell::dialog_closed` and `CloseWatcher` removed |
| rtwk-state: reactive_graph cross-thread panic, `batch` loss (the issue file) | rtwk quality; family 3, not used by the port |
| rtwk-state: `Select` notified inside a frame wakes the loop again (app-state-aopt.md §5) | ADR-003 reserve (family 4) |

G3 (a `!Send` controller task) and G6 (shared width model with Codex's
renderers) no longer apply: there is no carried-over controller and rtwk
lays out all text.

## Risks

1. rtwk-rich renderers differ from Codex's (markdown, diff, highlight); many
   small visual deltas to review. Mitigation: list per slice, decide once
   per kind.
2. Behaviour is spread across the oracle's 130k-line `app/` and 90k-line
   `chatwidget/`; a slice can miss a rule. Mitigation: each slice names the
   oracle tests it re-expresses; pty scenarios catch the rest.
3. Transcript performance at 10³–10⁴ items and 1,000 deltas/s; keep the
   live tail one item, settle into ordinary items, measure (ADR-008).
4. Inline mode depends on rtwk gaps landing.
5. Fork-specific areas (agents center, sidebar, analytics, `ui_tree`) are
   large and lightly documented.
6. Windows: rtwk's evidence is partial.

## Decided after the architecture review (2026-11-17, for the person to confirm)

[reviews/arch-review.md](reviews/arch-review.md) (independent Opus review of
a2dbf130c) found what had to be decided before the skeleton. Decided on its
evidence, in the text above:

1. Routes and dialogs (B1) — the person: "as in twk": `Navigator` +
   `PageRoute`, `show_dialog`, live data through `rtwk::provider`
   projections of the model; ids as arguments; results back as ticketed
   events; a presenter for backend requests. Verified by a prototype
   (`research/nav-proto.md`, 14 tests) with four binding conditions
   (`eq` widgets, presenter under the Navigator, fixed providers + keyed
   Navigator, no same-flush decisions); rtwk gaps G13–G15, none blocking.
2. Per-thread widget state and the composer (M1): `ThreadUi` controllers;
   composer = rtwk `TextArea` + gap G12 built in rtwk.
3. Effect runner and runtime (M3, m10): the shell, through rtwk-tokio;
   `run_app_with` on the `arg0` runtime's block_on thread.
4. Crate location and resolution (M6): stays `codex-rs/tui-rtwk`, passes
   manifest policy, copies `[patch]`, seeds its lockfile, checks the pin.
5. Carried code (M7): REUSE minus content renderers, with dependency closure.

## Open decisions for the person

1. When the port joins the codex-rs workspace and how `codex` exposes it
   (`--frontend rtwk` or `codex-rtwk`) — end of phase 1.
2. Whether the rtwk gaps go into rtwk's own tm now or when a slice needs
   them.
