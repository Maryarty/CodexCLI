# Codex TUI internals: runtime, state, rendering, reuse, tests

Research for tm `port-design` (2026-11-17), `codex-rs/tui` at `c6a41e32f`.
Paths are relative to `codex-rs/tui/`. Measurement scripts were scratch work
and are not kept.

## 0. Headlines

- **Pattern:** a retained, mutable component tree; one unbounded mpsc bus
  (`AppEvent`, 335 variants); one big `select!` loop in `App::run`;
  immediate-mode rendering (`render(&self, Rect, &mut Buffer)`). Not Elm: no
  pure update, no immutable view.
- **Default screen mode is the owned alt-screen**: `tui.fullscreen_transcript`
  defaults to true (`../core/src/config/mod.rs:4929`, `is_none_or`). The
  inline viewport with history inserted into scrollback is the fallback
  (`alternate_screen=never`, `--no-alt-screen`, Terminal.app over SSH). Both
  modes share `ChatWidget`.
- **Size:** `src/` 529,123 lines in 1,268 files, ~270k of them tests (~259k
  production).
- **Backend boundary:** `codex_app_server_client` (in-process, remote, cloud)
  speaking `codex_app_server_protocol`. The TUI never imports `codex_core`
  (only `codex-core-plugins`).

## 1. Runtime

- Entry: `src/main.rs:20` → `arg0_dispatch_or_else` (tokio multi-thread
  runtime, `../arg0/src/lib.rs:264`) → `run_main` (lib.rs:1115) →
  `startup_orchestration::run_main_inner` → `run_ratatui_app` (lib.rs:1157).
  Huge futures are boxed (`boxed_future!`, lib.rs:1107); `recursion_limit=256`.
- One foreground task runs `App::run` (src/app/startup.rs:248): all drawing
  and all state mutation happen there.
- Background (non-test): 119 `tokio::spawn`, 19 `spawn_blocking`, 13 std
  threads, 23 `select!`, 53 `Mutex`, 40 `RefCell`, 130 `unsafe`. Typical spawn:
  an RPC or fs job, then `app_event_tx.send(AppEvent::XxxLoaded{..})`.
- Async controller: `app/` has 320 `async fn` and 1,111 `.await`
  (`chatwidget/` 122, `bottom_pane/` 17); `App::handle_event`
  (app/event_dispatch.rs:35) is `async` and awaits RPCs inline.
- Backend events: `start_app_server` (lib.rs:504) →
  `AppServerClient::{Cloud, InProcess, Remote}`
  (`../app-server-client/src/lib.rs:378`), wrapped by `AppServerSession`
  (src/app_server_session.rs:392, ~46 async RPC methods). Events:
  `AppServerEvent::{ServerNotification, ServerRequest, Lagged, Disconnected, ..}`
  tagged with a `BackendId`; `backends.next_event()` multiplexes backends.
- Terminal input: `Tui::event_stream()` (tui.rs:1145) → `TuiEventStream` over a
  shared `EventBroker` (src/tui/event_stream.rs) wrapping crossterm's
  `EventStream`; dropped and recreated on pause so an external editor owns
  stdin; an `EventSource` trait allows a `FakeEventSource` in tests.
  `TuiEvent` (tui.rs:676): Key, Paste, Mouse, Resize, Draw, Resume,
  FocusGained, FocusLost. Draw is the frame scheduler's broadcast merged in.
- Frame scheduler: `FrameRequester` (src/tui/frame_requester.rs:30), a
  cloneable handle over an mpsc of `Instant` (`schedule_frame`,
  `schedule_frame_in`). An actor (`FrameScheduler::run`, :96) keeps the
  earliest deadline, clamps it with `FrameRateLimiter` (MIN_FRAME_INTERVAL
  8.33 ms, 120 fps, frame_rate_limiter.rs:13), emits one `broadcast<()>` per
  deadline. No fixed tick. Every Draw re-renders the whole frame;
  `custom_terminal` diffs against the previous buffer.
- Main loop `select!` (startup.rs ~1500–1850), priority by guards, not
  `biased`: daemon restart, cloud env picker, the AppEvent bus
  (`saved_openings.next_event(&mut app_event_rx)`), `active_thread_rx`
  (per-thread bounded mpsc of `ThreadBufferedEvent` + replay store,
  app/thread_events.rs:650), `tui_events.next()` (gated off while app events
  are pending during startup or replay), `backends.next_event` (~1700),
  reconnect, resize-telemetry deadline, rate-limit poll, goal-status tick,
  `commit_animation.tick()`.
- Commit animation: `commit_animation: Option<tokio::time::Interval>`
  (app.rs:625), period TARGET_FRAME_INTERVAL; started by
  `AppEvent::StartCommitAnimation` (event_dispatch.rs:1283), stopped by
  `StopCommitAnimation`; drives `chat_widget.on_commit_tick()`; suppressed
  while app events are pending.
- Key routing: `App::handle_tui_event` (app.rs:886) — ~500 lines of
  pre-filters (sidebar press, link hover, chord matcher, overlay, owned
  transcript, voice toggle) → `ChatWidget::handle_key_event`
  (chatwidget/interaction.rs:125) → `BottomPane::handle_key_event`
  (bottom_pane/mod.rs:901) → the top `BottomPaneView` or the `ChatComposer`.
  Draw: `handle_draw_pre_render` (app/resize_reflow.rs:428) →
  `render_chat_widget_frame` (app.rs:1498).
- src/tui/: job control (SIGTSTP), stderr guard, panic restore (tui.rs:399).

## 2. State management as it is

```
App (app.rs:562-747, ~119 fields)
 |- chat_widget: ChatWidget (chatwidget.rs:536-825, impl over ~100 files)
 |    |- bottom_pane: BottomPane (bottom_pane/mod.rs:305)
 |    |    |- composer: ChatComposer (+ command/file/skill/mention popups)
 |    |    |- view_stack: Vec<Box<dyn BottomPaneView>>  (modals, pickers, approvals)
 |    |    `- status indicator, footer, pending_input_preview ...
 |    |- transcript: TranscriptState (live cell, stream controllers)
 |    `- config copy, model/permission/collab state, token and rate-limit info
 |- transcript_cells: Vec<Arc<dyn HistoryCell>>  (finalized history, source of truth)
 |- transcript_view: TranscriptView (owned-screen scroll, selection, layout cache)
 |- overlay: Option<Overlay>, sidebars, agents_overview, backtrack
 |- keymap: RuntimeKeymap, key_chord_matcher
 `- config, local_settings, backend_registry, thread event channels, commit_animation
```

- Only the foreground task mutates state (`&mut App`, `&mut ChatWidget`).
  Cross-thread state: `Arc<Mutex>` (thread event stores) or atomics.
  `RefCell` for render-time caches. App reaches into `chat_widget.*` freely.
- Bus: `AppEventSender` (src/app_event_sender.rs:26) over the unbounded
  sender; helpers `interrupt()`, `compact()`, `exec_approval()` build
  `AppEvent::CodexOp(AppCommand)`; filters events for parked voice owners
  (:41).
- `AppEvent` (src/app_event.rs:356, 335 variants), heuristic groups: UI
  intents (open/show/confirm/select/start-flow) ~109; async results
  (*Loaded/Finished/Created/...) ~66; persist/apply settings ~39;
  refresh/fetch ~22; history and stream (InsertHistoryCell,
  ConsolidateAgentMessage, Begin/EndInitialHistoryReplayBuffer,
  Start/StopCommitAnimation, ResetTranscriptForThreadSwitch) ~5; backend op
  submission 2 (`CodexOp`, `SubmitThreadOp`); remainder ~92. `AppCommand`
  (app_command.rs:111) is the typed backend-op enum. `AppEvent` embeds UI
  types (`Box<dyn HistoryCell>`, `Line`, `StatusIndicatorWidget`, ~:340).
- Dispatch: `App::handle_event` (app/event_dispatch.rs:35), one match. Ops:
  `CodexOp` → `submit_active_thread_op` (app/thread_routing.rs:572) → typed
  RPC. Notifications: `handle_app_server_event` (app/app_server_events.rs:67)
  → per-thread buffered channel → `handle_thread_event_now/replay`
  (thread_routing.rs:2093/2160) → `ChatWidget::handle_server_notification`
  (chatwidget/protocol.rs:4, ~91 arms) or `handle_server_request`
  (protocol_requests.rs:9).
- Request/response: no correlation framework in the UI; results come back as
  AppEvents carrying an id (e.g. `model_popup_request_id`) so stale ones are
  dropped; `AppServerSession` numbers JSON-RPC ids (:398).
- Popup results: `SelectionItem.actions: SelectionAction =
  Box<dyn Fn(&AppEventSender) + Send + Sync>` (bottom_pane/list_selection_view.rs:125);
  the view reports `is_complete()` / `completion()` and BottomPane pops it.
- Config: `AppEvent::PersistX` → `ConfigEdit`s (src/config_update.rs) →
  app-server `ConfigBatchWrite` → App updates its copy → ChatWidget setters →
  BottomPane. Config is duplicated in App, ChatWidget, local_settings and the
  per-thread server snapshot. The keymap is rebuilt into `RuntimeKeymap`.

```
crossterm EventStream -> EventBroker -> TuiEventStream --\
FrameScheduler (actor, 120fps cap) --broadcast Draw------+--> tui_events.next()
AppServerClient.next_event -> backends.next_event -------+          |
per-thread mpsc<ThreadBufferedEvent> -----------------------+       v
mpsc<AppEvent> (views, tasks, timers) -------------------+--> select! in App::run
commit_animation Interval ------------------------------/           |
   ^                                       handle_tui_event / handle_event / handle_server_*
   |  AppEventSender.send                  mutate App/ChatWidget/BottomPane; spawn RPC tasks
   +------ views, tasks, ChatWidget <----- frame_requester.schedule_frame()
                                                    v
     on Draw: pre_render -> desired_height(w) -> Tui::draw(h, |f| widget.render(area, f.buffer))
     -> custom_terminal diff -> crossterm backend inside a synchronized update
```

Label: "retained mutable component tree, immediate-mode Buffer rendering,
actor-ish message bus" — effects go through messages, state is not
centralised.

## 3. Rendering pipeline

- ratatui 0.30 `Buffer`, `Rect`, `Line`, `Span`, `Style`; features
  scrolling-regions, unstable-backend-writer, unstable-rendered-line-info,
  unstable-widget-ref.
- `Renderable` (src/render/renderable.rs:16): `render`, `desired_height(width)`,
  `render_scrolled`, `cursor_pos`, `cursor_style`; combinators
  `ColumnRenderable` (:186), `FlexRenderable` (:272), `InsetRenderable`
  (:407), `RenderableItem` — measure by width, then render into a Rect.
- Height: inline mode passes `chat_widget.as_renderable().desired_height(width)`
  (app.rs:1559) to `Tui::draw(height, ..)` (tui.rs:1449), which resizes the
  viewport; owned mode lays out against the full screen in
  `render_owned_transcript` (app/owned_transcript.rs:247).
- `custom_terminal.rs` (1,635 lines, from `ratatui::Terminal`): double-buffered
  diff with a movable `viewport_area`; cursor, image placements
  (kitty/iTerm), hyperlink cells; `draw_with_size` :383. `Tui::draw` wraps it
  in a synchronized update (DEC 2026), grows the viewport by scrolling,
  flushes pending history first.
- Inline history: finished cells are not drawn in the viewport.
  `insert_history_lines` (src/insert_history.rs:63) sets a scroll region above
  the viewport, prints wrapped lines, resets (SetScrollRegion :346).
  Strategies Standard, Zellij, FullScreen (src/tui/scrollback.rs:17).
  Hyperlinks via `HyperlinkLine`. On width change app/resize_reflow.rs clears
  Codex-owned scrollback and re-emits from `transcript_cells` under a row cap.
- Owned mode: `TranscriptMode::{Owned, Terminal}` (src/transcript_mode.rs);
  `TranscriptView` (src/transcript_view/) lays out the retained cells per
  frame: anchors that survive prepend and rewrap, `LayoutCache`
  (layout.rs:114), selection, search, copy mode, follow-latest; sidebars,
  side panes and mouse capture on top. Overlays (pager, diff, analytics) use
  `enter_alt_screen` (tui.rs:1165).
- `HistoryCell` (src/history_cell/mod.rs:210, `Arc<dyn HistoryCell>`):
  `display_lines(width) -> Vec<Line<'static>>`, plus `raw_lines`,
  `transcript_lines`, `desired_height`, hyperlink variants,
  `display_lines_for_mode`, `is_stream_continuation`,
  `transcript_animation_tick`, `copy_source`. The best seam for a port.
- Streaming: deltas → streaming/controller.rs (`StreamController`,
  `PlanStreamController`) → queue of stable lines → commit ticks
  (commit_tick.rs). chunking.rs `AdaptiveChunkingPolicy`: Smooth drains one
  line per tick, CatchUp drains all, with hysteresis. Output as
  `AppEvent::InsertHistoryCell`; on completion `ConsolidateAgentMessage`
  swaps in a source-backed `AgentMarkdownCell`. markdown_stream.rs parses
  pulldown-cmark incrementally; markdown_render/ produces Lines; syntect
  highlighting in render/highlight.rs.
- Wrapping: src/wrapping.rs (2,097 lines) `RtOptions`, `word_wrap_line(s)`,
  URL-preserving `adaptive_wrap_line(s)` (textwrap, unicode-width,
  segmentation, linebreak, bidi); line_truncation.rs, live_wrap.rs.
- Styles (styles.md): bold headers, default fg, `secondary_text_style()`,
  `accent_style()`, `status_style(StatusTone)`; ANSI green/red/magenta only;
  clippy.toml bans `Color::Rgb`, `Color::Indexed`, `Stylize::white`.
- `ui_tree.rs` (1,273 lines): a declarative JSON display-element tree (Text,
  Link, Row, ...), used by the statusline picker and inline visualizations.

## 4. Boundaries and reuse

"prod" excludes `*_tests.rs`, tests dirs, `*_test_support.rs` and trailing
`#[cfg(test)] mod tests`; "free" = the file mentions neither `ratatui` nor
`crossterm`. File-level, so coupling is over-counted: in coupled files only
5.8% of lines (8,148 of 141,489) match UI-type tokens.

| module | total | prod | prod coupled | prod free | ratatui/crossterm hits |
|---|---|---|---|---|---|
| app/ | 131,838 | 50,087 | 18,451 | 31,636 | 141 |
| chatwidget/ | 92,638 | 40,980 | 9,233 | 31,747 | 70 |
| bottom_pane/ | 84,838 | 44,546 | 39,416 | 5,130 | 532 |
| transcript_view/ | 20,455 | 9,878 | 7,905 | 1,973 | 63 |
| history_cell/ | 13,594 | 7,536 | 2,296 | 5,240 | 24 |
| analytics/ | 13,148 | 7,177 | 4,422 | 2,755 | 82 |
| app_server_session/ | 10,701 | 6,178 | 531 | 5,647 | 1 |
| resume_picker/ | 10,472 | 4,990 | 4,487 | 503 | 23 |
| tui/ | 8,275 | 3,151 | 2,337 | 814 | 100 |
| keymap/ | 7,807 | 5,060 | 4,419 | 641 | 11 |
| markdown_render/ | 7,777 | 5,415 | 3,338 | 2,077 | 17 |
| onboarding/ | 7,063 | 4,836 | 4,473 | 363 | 108 |
| streaming/ | 6,028 | 2,544 | 1,645 | 899 | 5 |
| status/ | 5,270 | 2,329 | 1,468 | 861 | 6 |
| keymap_setup/ | 4,086 | 2,194 | 2,194 | 0 | 31 |
| pets/ | 4,084 | 2,714 | 1,287 | 1,427 | 18 |
| clipboard_copy/ | 3,137 | 1,413 | 0 | 1,413 | 0 |
| diff_render.rs | 2,746 | 1,364 | 1,364 | 0 | 10 |
| render/ | 2,721 | 1,566 | 1,537 | 29 | 28 |
| exec_cell/ | 2,398 | 1,645 | 1,095 | 550 | 9 |
| pager_overlay/ | 2,349 | 1,160 | 1,160 | 0 | 18 |
| lib.rs | 2,611 | 2,611 | 2,611 | 0 | 4 |
| app_event.rs | 2,104 | 2,104 | 2,104 | 0 | 2 |
| wrapping.rs | 2,097 | 1,401 | 1,401 | 0 | 7 |
| custom_terminal/ | 1,943 | 1,710 | 1,710 | 0 | 57 |
| terminal_hyperlinks/ | 1,821 | 1,336 | 1,336 | 0 | 35 |
| ide_context, dynamic_tools, markdown_copy, clipboard_paste, statusline_provider | ~7,600 | ~5,000 | 0 | ~5,000 | 0 |
| **total** | **529,123** | **~259,480** | **141,489 (54.5%)** | **117,991 (45.5%)** | |

- (a) Framework-free as-is (~45%): app_server_session/*, most of
  chatwidget/* (protocol handling, input queue, goal, status, rate limits,
  model and permission state), most of app/* (thread routing, config
  persistence, event buffers, agents-overview data, history pagination),
  config_update, local_settings, ide_context, dynamic_tools, clipboard OS
  side, markdown_copy, mention_codec, statusline_provider, git helpers,
  diff_model, the chunking policy.
- (b) Entangled through `Line`/`Span`/`Style`/`KeyEvent`/`Rect`, separable
  (~25%): history_cell/*, markdown_render, wrapping, line_truncation,
  diff_render, exec_cell, status/, streaming/controller, keymap (crossterm
  `KeyEvent` is the key type everywhere), composer editing logic, key_hint,
  `AppEvent` itself.
- (c) Rendering and terminal (~30%): custom_terminal, insert_history, tui/,
  render/, bottom-pane view render impls (88% coupled), transcript_view
  layout and selection, pager_overlay, onboarding, resume_picker,
  keymap_setup, pets image rendering, terminal_hyperlinks, terminal_probe and
  palette, ui_tree render.
- State is not layered (ChatWidget owns BottomPane and calls its setters), so
  reuse needs an extraction step, not just a path dependency.
- codex-* crates: 58 path deps; most used: app_server_protocol 1,692 use
  lines, protocol 644, config 294, app_server_client 218,
  utils_absolute_path 74, terminal_detection 53.
- External: tokio family, futures; ratatui + macros, crossterm, textwrap,
  unicode-width/segmentation/linebreak/bidi, pulldown-cmark, syntect +
  two-face, vtparse, image, arboard, diffy; clap, serde, toml(_edit), chrono,
  uuid, url, strum, color-eyre, anyhow, tracing; rmcp, axum, supports-color,
  webbrowser, sha2, regex-lite; libc, core-foundation/objc2 (macOS), zbus
  (linux), windows-sys; dev: insta, vt100, wiremock, pretty_assertions,
  serial_test.
- Seams to keep as the port contract: `AppServerSession` + protocol types;
  the `AppCommand`/`AppEvent` split; `HistoryCell::display_lines(width)`;
  `Renderable{render, desired_height}`; `StreamController` + chunking; the
  keymap runtime + action ids; the `TuiEvent` enum.

## 5. Tests

- 6,329 `#[test]`/`#[tokio::test]`; 1,450 snapshot assertions in 329 files;
  1,549 `.snap` (chatwidget 441, bottom_pane 363, app 360); 123 `TestBackend`
  references; 35 files use `VT100Backend`; tests/all.rs aggregates 18 files in
  tests/suite/ (3.3k lines): vt100_history, vt100_live_commit, resize_reflow,
  screen_reader, reconnect, status_indicator, focus_palette,
  daemon_compatibility, ...
- Snapshots are plain text, no styles: a widget is rendered into a `Buffer`
  and flattened by per-test helpers (`snapshot_buffer` bottom_pane/mod.rs:2771,
  `render_bottom_popup`, `buffer_to_text`, `snapshot_lines`
  diff_render.rs:1477), then `insta::assert_snapshot!`. Style checks are
  separate unit tests.
- `VT100Backend` (src/test_backend.rs:21): a crossterm backend writing into a
  `vt100::Parser` with scrollback; with `custom_terminal::Terminal` it tests
  insert_history, scroll regions, live commit and reflow at the escape level.
- Fixtures: `FakeEventSource`, `FrameRequester::test_dummy`,
  `make_chatwidget_manual` (chatwidget/tests/helpers.rs:213),
  app/test_support.rs, paused tokio time.
- As an oracle: text snapshots are directly diffable once rtwk renders a
  component at width W to text; vt100 scenarios give screen and scrollback
  text for inline mode; styles need a per-cell dump; tests use private items,
  so the harness needs a seam (architecture ADR-001).

## 6. Flags, platforms, performance

- No cargo `[features]` in the tui crate; behaviour from config
  (`codex_features`, `tui.*`: alternate_screen auto/always/never,
  fullscreen_transcript, sidebar_width_percent, mouse_scroll_speed,
  copy_on_select, raw_output_mode, collapse_turn_details, effects,
  animations) and probes (terminal_probe/, terminal_detection: kitty
  graphics, enhanced keys, OSC palette, tmux, zellij, herdr, iTerm).
- Extension points: slash commands (bottom_pane/slash_commands.rs,
  chatwidget/slash_dispatch.rs), `BottomPaneView` (bottom_pane/bottom_pane_view.rs:31,
  ~30 defaulted hooks), HistoryCells, AppEvent arms, keymap contexts,
  statusline items, dynamic tools, MCP elicitation, plugins/hooks browsers.
- `cfg` counts: windows 252, unix 133, macos 69, linux 87; WSL mentions 201,
  tmux 272, zellij 38. Terminal.app over SSH disables the alt screen; Windows
  Terminal and Zellij have their own scrollback strategies; kitty keyboard
  negotiation (tui/keyboard_modes.rs, 608 lines), `activation_key_guard`,
  leader chords, vim-escape recovery.
- Performance-sensitive: full re-render per Draw at ≤120 fps (layout caches
  `ActiveCellLayoutCache`, `LayoutCache`); streaming deltas coalesced to
  ≤4 KiB per buffered event, 256 KiB replay cap
  (app/thread_event_buffer.rs:9-10); adaptive chunking; resize rebuilds
  scrollback under a row cap, debounced; pagination of large transcripts
  (`RequestOlderScrollbackHistory`); syntect + markdown re-parse per delta;
  image and pet placement bookkeeping in the buffer diff.

## 7. Constraints for a port

1. Pick the mode first: owned alt-screen suits a retained tree; inline needs
   "emit rows above a movable viewport" plus resize re-emission.
2. rtwk needs: width→height measuring, hyperlinked rich text per cell,
   per-widget cursor position and style, image-placement cells, synchronized
   update, a diffed double buffer.
3. The bus (AppEvent, AppCommand, AppServerSession) can be kept with rtwk
   replacing the terminal side of `App::run`; `AppEvent` embeds UI types.
4. Popup completion by closure-over-sender maps to rtwk callbacks; the
   BottomPaneView stack maps to a Navigator-like stack, but views also
   mutate each other through BottomPane (paste burst, key guards).
5. crossterm `KeyEvent` is the key vocabulary everywhere: rtwk input must
   convert to it.
