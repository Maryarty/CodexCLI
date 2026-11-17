# State management for Codex TUI on rtwk, round 2 (Rust-native options)

Research for ADR-003, 2026-11-17. Both repos were only read.

- **Oracle:** `codex-rs/tui` at `ebb14b169`. Paths are relative to `codex-rs/tui/src/`.
- **Crate numbers:** crates.io and GitHub APIs, pulled 2026-11-17. "90d" is crates.io's `recent_downloads`.
- **Weighting** (from the person's clarifications):
  - first: clean state/UI separation, a standard Rust approach, fit with rtwk, streaming and testability, and phases 2–3 (Desktop features, then new features);
  - second: the migration cost of Codex's async handlers (C1) and upstream-merge friction (C7). The port may carry Codex's behaviour, not its code.

## 1. Recommendation

**Primary: the Elm Architecture (TEA) as a headless core.** It is the most widely documented Rust pattern for application state:

- ratatui's docs present it as an application pattern;
- iced is built on it, and relm4 is built on it;
- Crux packages it as a headless library.

Concretely:

- **Core crate `codex-tui-core`.** No rtwk, no terminal, no tokio.
  - It holds a plain-Rust `Model` and an `Event` enum: input, backend notifications, server requests and effect results.
  - `update(&mut Model, Event) -> Cmd<Effect, Event>` is **synchronous**.
  - The core never does I/O. An `Effect` is a value ("write config", "update thread settings", "submit op"), and its result comes back as an `Event`.
- **Composed by feature, keyed by thread.**
  - `workspace.threads: BTreeMap<ThreadId, thread::Model>` holds each thread's transcript, stream, approvals and status.
  - Beside it sit `composer`, `pickers`, `config` and `account`.
  - Each feature module has its own `Model`, `Event`, `update` and projections. The parent maps them, as iced's `Task::map` and Crux's `Command::map_event` / `map_effect` do.
- **View models out.** Per-slice projections (`TranscriptVm`, `LiveTailVm`, `PaneVm`, `StatusVm`, ...).
  - Each carries a revision, with `PartialEq` on the revision and `Arc` snapshots inside.
  - A slice is recomputed only when its revision moved.
- **rtwk shell `codex-tui-rtwk`.** It owns the loop, as ADR-002 already says.
  - Input: keys, pastes, resizes and backend notifications become `Event`s. Notifications arrive through `rtwk_tokio::subscribe`, which is ordered and bounded.
  - Effects: each runs as a `Send` future on tokio (`rtwk_tokio::spawn_tokio`) against `AppServerSession`, and its result re-enters as an `Event`.
  - Store: a UI-thread `Store` implements `rtwk_state::Source<R>` per slice. Regions follow slices with `Select` / `store_provider`; hot single-reader values use `Obs<T>`.
  - rtwk core needs no change.

**Which TEA runtime** is the person's call; prototype F1 measures both:

- **A thin in-house `Cmd`**, about 200 lines, shaped like Crux's API. My pick, weighted score 89.
- **`crux_core` 0.20**, score 85.
  - It gives: a documented Command runtime (async blocks that await shell requests; `then`, `all`, `map_event`; abort handles; `expect_*` test helpers).
  - It costs: pre-1.0 churn (0.16 in Jul 2025 to 0.20 in Aug 2026, every minor breaking), a small user base (48.9k downloads in 90d), and a model kept behind its `Core`.

The core's code is nearly the same either way, so switching later is mechanical.

**Why TEA wins** (scores in §4):

1. **Separation by construction.** The core crate cannot name a widget, a `Rect`, a terminal or an I/O future. Today's coupling (§2) is a compile error there, not a review rule.
2. **rtwk fit.** It is what rtwk's guide asks for: a Rust model, an external store followed through `Source` / `Select`, and local UI state in `State` (docs/guide/state.md). My prototype (§1.3) measured it with no rtwk change.
3. **Streaming.** A delta touches one thread's buffer and one revision. The commit tick moves lines into the transcript. `Select`'s pending flag bounds rebuilds per frame. Measured.
4. **Tests.** `update` maps (model, event) to data, so tests need no tokio, terminal or tree. Time becomes an input (`Event::Tick { now }`). Today the commit tick reads `Instant::now()` itself (chatwidget/streaming.rs:663).
5. **Phases 2–3.**
   - Several threads are a keyed map of sub-models, each with its own slices. Panels showing different threads rebuild independently and release their subscriptions on unmount (measured).
   - A feature is one sub-module plus one parent variant.
   - Cross-feature derived state is a projection memoized by the revisions it reads.
   - A feature test is events in, effects out.

**Runner-up: async owner with published snapshots** (F2). This is the tokio actor pattern with relm4's `AsyncComponent` semantics, i.e. ADR-003 as written, minus the borrowed name.

- It keeps Codex's `async fn(&mut self)` handlers and the oracle's exact serialization.
- It loses on separation: logic still awaits I/O inline, and state is borrowed across awaits, so the UI can only see copies.
- It loses on testing (async tests with fakes) and on phases 2–3 (features accrete in one owner).
- It wins only on C1, C2 and C7.

**Not primary:**

- **Signals** (reactive_graph, reactive_stores, dioxus-signals, futures-signals, Floem): they propagate changes, which rtwk already does, and say nothing about where logic lives. A write cost 26 allocations against 3 in rtwk's fixture. Keep `rtwk_state::reactive::Derived` for derived state if the graph grows.
- **Actor frameworks** (kameo, ractor, actix): one owner is enough, and they force `Send` on all state.
- **Redux crates:** stale or tiny.
- **Xilem:** a competing view tree.
- **bevy_ecs:** schedule order, not event order.
- **rtwk's `AppView` as the store:** the core stops being headless.

### 1.1 What stays component-local (rtwk `State`, hooks, `Obs`)

Rule: if no `update` arm reads it and no test asserts it, it is local.

| Local (rtwk) | Core |
|---|---|
| hover, pressed, focus ring | focus *target* where logic routes keys (composer vs pane) |
| scroll offset, viewport, wrap/layout caches, `rendered_selection` | "follow the tail" (`FollowTranscript`), a copied selection |
| spinner/shimmer phase, `EmptyStateAnimation`, elapsed-time text | `started_at`, status text, status-row visibility |
| panel open state nobody else reads; split ratios | panels/layouts that logic or persistence reads (Desktop workspaces) |
| post-parity: composer editing in rtwk-editing, synced on submit | at parity: composer text, cursor, history, slash/mention popups |

### 1.2 The ~1,100 awaits

They live in `app/` (1,100 `.await` in non-test files).

**The hot paths are already synchronous.** These are plain `fn`s:

- `ChatWidget::handle_server_notification` (chatwidget/protocol.rs:4);
- the delta and commit-tick paths (chatwidget/streaming.rs:307, :643);
- `handle_server_request` (protocol_requests.rs:9).

Their ~91 arms move into `thread::update` almost verbatim, minus the render calls.

**The awaits sit in `App::handle_event`** (app/event_dispatch.rs:35). There, 124 of ~340 top-level arms reach an `.await` (counted with awk over the match arms). Each await falls into one of four classes:

1. **Fire and report.** One RPC, then report it (`persist_default_model_selection`, app/model_defaults.rs:51). This becomes one `Effect` plus one result `Event`. Most awaits are this.
2. **A chain with no model access between steps.** It becomes one async function in the shell's effect executor, with one result event.
3. **Model reads or writes between awaits** (`select_session_model`, model_defaults.rs:89; `UpdateModel`, event_dispatch.rs:2523).
   - Each such await becomes a continuation event.
   - Where the oracle relied on blocking the loop, the core gets an explicit in-flight guard. Codex already has some: `fork_in_progress` (event_dispatch.rs:41-53) and request ids (`model_popup_request_is_current`, :2722-2741).
4. **Test awaits** (4,915 in app/ test files). Not ported; core tests are synchronous.

Expect roughly 124–250 new result or continuation events.

**The one behaviour change to accept:** today a slow inline RPC freezes input and drawing (the `select!` arm awaits `handle_event`). In the port, input keeps flowing. Parity scenarios must script RPC latency where it matters.

### 1.3 Prototype run (cheapest, mine): TEA core + rtwk-state slices

**Setup.**

- `$TMPDIR/codexcli-research/proto-tea/`: a scratch crate with path deps on `rtwk` and `rtwk-state`. Neither repo was touched.
- Core:
  - stream + commit tick, chunking-like: one line per tick, or all of them once more than 8 lines are queued;
  - model picker: fetch, drop a stale id, select, persist, info cell;
  - approval: decision cell + `SubmitApproval`.
- View: four `Select` slices over a 1,000-cell transcript.
- `cargo test --release`: 4/4 pass. macOS arm64, rustc 1.99.0, load average 53.6 (a shared machine), so times are orders of magnitude only.

**Results** for 1,000 deltas over 120 frames (≈8 deltas, 1 tick, 1 publish, 1 flush per frame):

- **Builds:**
  - transcript 115, exactly the 115 ticks that committed (0 per delta);
  - live tail 120 (1 per frame);
  - status row 1; pane 0.
  - Wakes: ≈2 per frame, one per changed slice, merged into one frame.
- **Allocations:**
  - `update`: 0.01 per delta.
  - Commit tick: 5.67 per tick. That includes an O(n) `Arc::make_mut` copy of the 1,000-cell vector while the view holds the old snapshot. Chunk it, or use a persistent vector, past ~10⁴ cells.
  - Publish + flush: 6.92 per frame.
- **Time:** update + tick 3.3 µs per frame; publish + flush 3.8 µs per frame (Text leaves, no layout or paint).
- **Per-event publish** (no batching): 100 deltas give 1 wake and 1 live-tail build, but 2.3 allocations per event. So publish once per drained batch.
- **Picker + approval interleaved with the stream:**
  - pure data with no runtime; effects compared by equality; a stale `ModelsLoaded` dropped.
  - Found one behaviour to port: cells inserted mid-stream land before the uncommitted lines. Codex orders these with its interrupt queue (`flush_interrupt_queue`, chatwidget/streaming.rs:689).
- **Unmount:** all 4 subscriptions released.

**Not covered here** (F1 covers them): a real frame loop, key latency, tokio effects, Crux.

### 1.4 What it implies for other ADRs (recommendations; the person decides)

- **ADR-001, code organization.**
  - Instead of a seam inside codex-tui: `codex-tui-core` (headless, behaviour ported) plus `codex-tui-rtwk` (shell).
  - `codex-tui` stays untouched as the **oracle**, as twk is for rtwk. Upstream merges cause zero conflicts.
  - The cost moves to behaviour drift. At each repin, the oracle's snapshot and scenario diff between the old and new `ORACLE_REF` is the work list.
  - Pure leaves (markdown_render, wrapping, diff_render, chunking, `display_lines`, all data per ADR-004) are `pub(crate)` today. Vendor them with provenance and re-diff at each repin, or split them out upstream.
- **ADR-002, the loop.**
  - rtwk keeps the loop. The long-lived `!Send` controller task disappears, and gap G3 with it.
  - The shell drives the synchronous core on the UI thread. I/O is plain `Send` tokio work through rtwk-tokio.
  - Fallback: run the core on its own thread behind the same `Source` API.
- **ADR-003.** "TEA headless core, followed through rtwk-state"; drop the Bloc wording.
- **ADR-007, parity.**
  - The `.snap` comparison stays; fixtures become **event scripts**. The oracle's chatwidget tests already drive `handle_server_notification`, so one script feeds both sides.
  - The cell diff compares the same scenario's screens, not the same view model.
  - Oracle controller tests stay; the core gets ported tests.
- **ADR-009, migration.**
  - The bridge painting the live legacy ChatWidget cannot be the migration path any more; it stays as a reference viewer.
  - Migration goes feature by feature into the core, each gated by its oracle snapshots and scenarios.

## 2. Where Codex mixes state and UI, and how TEA separates it

1. **Views own state, rendering and behaviour.**
   - `ListSelectionView` (bottom_pane/list_selection_view.rs:345-370) mixes styled `Line`s (`footer_note`), layout knobs (`max_visible_rows`, `col_width_mode`), interaction state (`search_query`, `ScrollState`, `active_tab_idx`) and an `AppEventSender`.
   - Its items carry closures: `SelectionAction = Box<dyn Fn(&AppEventSender) + Send + Sync>` (:125).
   - Every pane view is `BottomPaneView: Renderable` (bottom_pane_view.rs:30, 37 methods).
   - **TEA:** `picker::Model { items: Vec<Item { label, on_accept: Event }>, query, selected, request_id }` plus `picker::update`. The widget paints `PickerVm`; a choice is a comparable `Event`, not an opaque closure.
2. **Handlers drive the terminal and style UI.**
   - `AppEvent::DiffResult` (app/event_dispatch.rs:1655-1678) clears pane state, calls `tui.enter_alt_screen()`, styles lines (`"No changes detected.".italic()`), installs an `Overlay` and schedules a frame.
   - The dispatcher touches `tui.` 60 times. Chatwidget production code calls `request_redraw` / `schedule_frame` 177 times.
   - **TEA:** `m.overlay = Some(Overlay::Diff { text })`. The shell enters the alt screen when that slice is `Some`; styling is the view's; publishing replaces every redraw call.
3. **Render writes state that logic reads.**
   - `last_rendered_width: Cell<Option<u16>>` (chatwidget.rs:751) is set during render (:1769) and read by `current_stream_width` (:1705) when logic builds stream controllers (chatwidget/streaming.rs:326, :743).
   - Render caches sit in `RefCell`s on the model: `rendered_selection` (:626), `EmptyStateAnimation` (:538).
   - **TEA:** width arrives as `Event::Resized { width }`, ordered with everything else. Caches and animations live in rtwk widgets.
4. **Components reach into each other.**
   - The commit tick (chatwidget/streaming.rs:643-685) updates chunking, both stream controllers and the history, then sends `StopCommitAnimation` back to App.
   - It also calls `self.bottom_pane.hide_status_indicator()`; that call has 6 sites (streaming.rs:128, 572, 672, 721, 789, 818).
   - Counts: 489 `self.bottom_pane.` calls in chatwidget; 1,121 `self.chat_widget.` calls in app/.
   - **TEA:** one `thread::update` arm sets `stream`, `transcript` and `status.visible`. Views derive what to show.
5. **Events carry UI types.**
   - `InsertHistoryCell(Box<dyn HistoryCell>)` (app_event.rs:1385).
   - `SidePanePressAfterReplay { area: Rect, press: crossterm MouseEvent }` (:803-809).
   - `CapturedSideWork.row: Box<StatusIndicatorWidget>` (:333).
   - The approval *view* builds a history cell and sends both it and the op (bottom_pane/approval_overlay.rs:389-427).
   - **TEA:** events carry data only; cells are display data per ADR-004. The core builds the decision cell and emits `Effect::SubmitThreadOp`.

## 3. Before and after: two real handlers

**Model selection persist** (app/model_defaults.rs:51-87, abridged):

```rust
// Codex today: async, &mut self held across the RPC, writes into a widget
async fn persist_default_model_selection(&mut self, rh: AppServerRequestHandle,
                                          model: String, effort: Option<ReasoningEffort>) {
    self.app_event_tx.send(AppEvent::FollowTranscript);
    match self.persist_model_defaults(rh, build_model_selection_edits(&model, effort.as_ref()),
                                      "default model and reasoning effort").await {
        Ok(()) => self.chat_widget.add_info_message(model_changed_message(&model, effort.as_ref()), None),
        Err(e) => self.chat_widget.add_error_message(format!("Failed to save default model: {}",
                                                             format_config_error(&e))),
    }
}
// TEA core: two arms, no await, no widget
Event::PersistModelSelection { model, effort } => {
    m.follow_tail = true;
    let edits = build_model_selection_edits(&model, effort.as_ref());
    Cmd::effect(Effect::ConfigWrite(edits))
        .then(move |r| Event::ModelSelectionPersisted { model, effort, result: r })
}
Event::ModelSelectionPersisted { model, effort, result } => {
    let t = m.active_thread_mut();
    match result {
        Ok(()) => t.push_info(model_changed_message(&model, effort.as_ref())),
        Err(e) => t.push_error(format!("Failed to save default model: {}", format_config_error(&e))),
    }
    Cmd::none()
}
```

- 14 lines in, 16 out; the logic is unchanged.
- `select_session_model` (:89-~230, three awaits with reads between them) becomes 3 arms plus 2 continuation events.
- With crux_core it can stay one `Command::new(|ctx| async move { .. ctx.send_event(..) })` block, with the values it reads captured up front.

**Exec approval decision.** Today the view builds the cell and sends two events (bottom_pane/approval_overlay.rs:389-427), then App awaits `submit_thread_op` (event_dispatch.rs:1650). In the core:

```rust
Event::Thread(id, thread::Event::ApprovalDecided { call_id, decision }) => {
    let t = m.thread_mut(id);
    let req = t.approvals.take(&call_id)?;           // the queue is model, not view
    t.transcript.push(approval_decision_cell(&req, &decision)); // same cell builder
    Cmd::effect(Effect::SubmitThreadOp { thread: id, op: AppCommand::exec_approval(call_id, None, decision) })
        .then(move |r| Event::Thread(id, thread::Event::OpSubmitted(r)))
}
```

## 4. Scored comparison

Scores run 1–5, 5 best. The weights in the header sum to 20, so totals are out of 100. † marks a score measured by my prototype; the rest is desk research. How each column was judged:

- **Sep:** what the state layer can name, by construction.
- **Std:** adoption, docs, idiom in Rust UI code.
- **Fit:** the rtwk-state mechanism used.
- **Strm:** builds and allocations per event.
- **Test:** what a feature test needs.
- **P2-3:** keyed multi-session state, panels mounting and unmounting subscriptions, derived state, the cost of a feature test. C10 is folded in here.
- **Ord:** atomic per event; order across events.
- **Mig:** share of the 124 awaiting arms re-cut.
- **Mrg:** codex-tui edits.
- **Mat:** release cadence and stability.
- **Thr:** `Send` bounds forced on state.
- **C6:** frontend neutrality.

| Candidate | Sep ×3 | Std ×2 | Fit C4 ×2 | Strm C3 ×2 | Test C5 ×2 | P2-3 ×3 | Ord C2 | Mig C1 | Mrg C7 | Mat C8 | Thr C9 | C6 | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **A TEA core, own `Cmd`** | 5 | 4 | 5† | 5† | 5† | 4 | 4 | 2 | 5 | 4 | 5 | 4 | **89** |
| **B TEA core on crux_core 0.20** | 5 | 4 | 4 | 3 | 5 | 5 | 4 | 3 | 5 | 2 | 4 | 5 | **85** |
| **D async owner + snapshots (tokio actor)** | 3 | 3 | 4 | 4 | 3 | 3 | 5 | 5 | 3 | 5 | 3 | 4 | **71** |
| C TEA frameworks as deps (iced, relm4, tui-realm) | 4 | 5 | 1 | 3 | 4 | 4 | 4 | 2 | 5 | 5 | 3 | 1 | 70 |
| H Redux crates (redux-rs, rs-store) | 4 | 1 | 4 | 4 | 4 | 3 | 4 | 2 | 5 | 1 | 3 | 4 | 66 |
| J rtwk `AppView` (D-over-A) as the store | 3 | 2 | 5 | 4 | 3 | 3 | 4 | 2 | 5 | 3 | 4 | 1 | 65 |
| K bevy_ecs World | 4 | 2 | 2 | 3 | 4 | 4 | 3 | 1 | 5 | 3 | 3 | 3 | 64 |
| E actor frameworks (kameo, ractor, actix) | 3 | 3 | 3 | 3 | 3 | 3 | 4 | 4 | 3 | 4 | 3 | 3 | 63 |
| F reactive_graph / reactive_stores | 3 | 4 | 3 | 2 | 3 | 4 | 3 | 1 | 5 | 3 | 3 | 3 | 63 |
| G dioxus-signals, futures-signals, Floem | 3 | 3 | 2 | 2 | 3 | 4 | 3 | 1 | 5 | 2 | 2 | 2 | 56 |
| I Xilem / xilem_core | 3 | 2 | 1 | 3 | 3 | 3 | 4 | 2 | 5 | 2 | 3 | 1 | 53 |
| salsa / comemo | complement only (memoized derived projections) | | | | | | | | | | | | — |

**Maturity** (C8; crates.io and GitHub, 2026-11-17):

| crate | downloads all / 90d | latest release | stars |
|---|---|---|---|
| crux_core | 389k / 48.9k | 0.20.0, 2026-08-07 (0.16→0.20 in 13 months) | 2,850 |
| iced | 3.39M / 836k | 0.14.0, 2025-12-07 | 32,061 |
| relm4 | 1.43M / 833k | 0.11.0, 2026-04-08 | 2,002 |
| tuirealm | 268k / 45.1k | 4.1.0, 2026-05-02 | 1,018 |
| ratatui (TEA/Component/Flux docs) | 72.2M / 27.4M | 0.30.2, 2026-06-19 | 23,337 |
| reactive_graph; reactive_stores | 4.09M / 1.83M; 4.01M / 1.80M | 0.2.16 / 0.4.4 stable; rc1 2026-10-31 | leptos 21,540 |
| dioxus-signals | 4.05M / 1.58M | 0.7.10; 0.8.0-alpha.3, 2026-10-16 | dioxus 39,794 |
| futures-signals; floem | 1.10M / 98k; 46k / 13k | 0.3.34, 2024-07-26; 0.2.0, 2024-11-14 | — |
| xilem; xilem_core | 13.7k / 3.8k; 12.6k / 4.5k | 0.4.0, 2025-10-29 | 5,613 |
| redux-rs; rs-store | 19k / 1.1k; 50.5k / 2.0k | 0.3.3, 2022-03-13; 3.0.0, 2025-11-27 | — |
| kameo; ractor; actix | 1.23M / 610k; 1.57M / 549k; 21.4M / 2.90M | 0.22.2; 0.16.6; 0.13.6 (all 2026) | 1,404; 2,144; 9,271 |
| salsa; comemo | 10.8M / 3.39M; 4.62M / 2.16M | 0.28.5, 2026-09-24; 0.5.1, 2026-01-29 | 3,030; — |
| bevy_ecs | 10.8M / 2.77M | 0.20.1, 2026-10-28 | — |
| arc-swap (for F2 on a worker thread) | 415M / 119M | 1.9.5, 2026-11-09 | — |

**Notes:**

- **A/B.** `Model` needs no `Send`; Crux requires `Event: Send + 'static` and `Effect: Send`.
  - Crux's `Command` offers `new` (async with `ctx`), `request_from_shell`, `stream_from_shell`, `then`, `all`, `map_event`, `map_effect`, `abort_handle` and `expect_*`.
  - Its README: "pre-1.0 … production-ready, but occasional breaking changes … can be expected". It shows no Rust TUI shell; "Rust shells can import the core directly". `serde` is not optional.
  - Unverified, measured in F1: whether `Operation` needs `Serialize`, and what `process_event` costs per event.
- **C.**
  - iced: `Task` needs iced's runtime and `MaybeSend` futures.
  - relm4: GTK. Its `AsyncComponent` has `async fn update(&mut self, ..)`, and its book says "awaiting slow futures will block the processing of further messages". That is Codex's exact semantics, and D's precedent.
  - tui-realm renders through ratatui.
  - These are patterns to follow, not dependencies usable under rtwk.
- **D.**
  - `App::run` is already a hand-rolled tokio actor: one task, an mpsc of `AppEvent`, and `select!`.
  - Publishing would use `Rc` slices, or `watch` / `arc-swap` from a worker (`rtwk_tokio::follow` takes a `watch`).
  - It needs gap G3, or `Send` everywhere.
- **E.** kameo: `handle(&mut self, msg, ctx) -> impl Future + Send`; "processed sequentially one at a time". ractor passes `&mut State` to async handlers. Both force `Send`, and neither has a view-projection story.
- **F/G.** rtwk's fixture: 26 allocations per write through `Derived`, vs 3 for a plain `Source`. reactive_graph "tracks the last effect run to start under parallel writes" (docs/guide/state.md).

## 5. Finalists and prototype specs (for a Sonnet agent)

The finalists:

- **F1a:** TEA core with its own `Cmd` (A).
- **F1b:** the same core on `crux_core` 0.20 (B).
- **F2:** async owner with published snapshots (D).

**Setup.** Scratch workspaces `$TMPDIR/codexcli-proto/{f1a,f1b,f2}`, outside both repos. Path deps on `rtwk`, `rtwk-state` and `rtwk-tokio`, plus `tokio`. Start from `$TMPDIR/codexcli-research/proto-tea`. Never build codex-rs.

**Common Codex-shaped slice:**

- **FakeServer:** `rpc(Req) -> Result<Resp>` with 0, 50 or 500 ms latency (`tokio::time`).
- **Notifications:**
  - `AgentMessageDelta` at 1,000/s for 10 s; 3–8 chars each, ~20% ending a line.
  - `TurnStarted` / `TurnCompleted`.
  - a `CommandExecutionRequestApproval` at t=3 s.
- **Commit tick** every 8.33 ms. Paced, with catch-up when more than 8 lines are queued or the oldest is older than 120 ms (streaming/chunking.rs:85-116).
- **Model picker:** open, `FetchModels` RPC, drop a stale id, move, accept, `PersistModel` RPC, then the "Model changed to X" cell.
  - Add a two-RPC variant that reads the model between the RPCs, shaped like `select_session_model`.
- **Approval:** the overlay, key `y`, the decision cell, then a `SubmitThreadOp` RPC.
- **View:** a 1,000-cell transcript, the live tail, the status row and the pane, each behind `Select`. Run them in `rtwk::app::run` on an in-memory terminal (rtwk-state tests/support/frame_loop.rs).

**Measure** (release build; report `uptime`):

- builds per slice per frame, and wakes;
- allocations per delta, per tick, and per publish + flush (rtwk's counting_alloc.rs, plus `RTWK_PERF_REPORT=1`);
- key-to-presented-frame p50/p95/p99 over at least 2,000 keys, one key every 50 ms during the stream, at 0 and 500 ms RPC latency;
- 200 seeded runs with jittered latency, asserting the order invariants:
  - decision cell before the op result;
  - info cell after the persist;
  - no half-applied tick in any frame;
- diff stats against the Codex originals (model_defaults.rs:51-230, approval_overlay.rs:389-427, streaming.rs:643-685);
- test lines, and whether tests need a runtime.

**Pass/fail** (every finalist):

- transcript: 0 builds per delta, at most 1 per commit;
- live tail: at most 1 build per frame; pane: 0 builds while streaming;
- `update`: at most 1 allocation per delta (amortized, delta excluded);
- key-to-frame: p95 ≤ 8.3 ms and p99 ≤ 16.7 ms at 1,000 deltas/s with 0 ms RPC;
- zero ordering violations;
- unmounting releases its subscriptions.

**Per finalist:**

- **F1a:** the flow tests run without tokio.
- **F1b:** everything F1a passes, plus:
  - `process_event` overhead is at most +5 allocations and +5 µs per delta over F1a; otherwise deltas bypass Crux, or F1a wins;
  - the two-RPC flow is a single `Command::new` block;
  - record whether `Operation` needs `Serialize`.
- **F2:**
  - the controller is a `!Send` task on rtwk's executor with `rtwk_tokio::enter`, and its handlers are `async fn(&mut self)`;
  - it publishes per drained batch;
  - record (not pass/fail) key latency during a 500 ms handler, where keys queue as in the oracle;
  - report whether the G3 spike works.

**Rule:** adopt F1a or F1b if both pass. Prefer F1b only if its overhead passes and the churn is accepted. Fall back to F2 only if F1 fails on latency or ordering.

## 6. Open questions for the person

1. **Code organization:** a new behaviour-ported core crate with codex-tui as an untouched oracle (recommended), or ADR-001's seam with shared logic (pairs with F2)?
2. **TEA runtime:** own `Cmd` (recommended) or `crux_core` (a standard crate, but pre-1.0 churn)? F1 gives the numbers.
3. **Interleaving:** may input keep flowing while an RPC is in flight, where the oracle blocks? This is the one deliberate behaviour difference.
4. **Pure leaves** (markdown_render, wrapping, diff_render, chunking, `display_lines`): vendor them with provenance, or split them upstream?
5. **ADR-009:** demote the live-ChatWidget bridge to a reference viewer?
6. **Composer:** in the core at parity, moving to rtwk-editing after?
7. **Derived state in phases 2–3:** revision-keyed memos first, adding `Derived` or comemo only where the graph grows?

## 7. Sources

- **crates.io** API `https://crates.io/api/v1/crates/{crux_core,iced,relm4,tuirealm,ratatui,reactive_graph,reactive_stores,dioxus-signals,futures-signals,floem,xilem,xilem_core,redux-rs,rs-store,ractor,kameo,actix,salsa,comemo,bevy_ecs,arc-swap}`, pulled 2026-11-17.
- **GitHub** API `https://api.github.com/repos/{redbadger/crux,iced-rs/iced,Relm4/Relm4,veeso/tui-realm,leptos-rs/leptos,DioxusLabs/dioxus,tqwewe/kameo,slawlor/ractor,actix/actix,linebender/xilem,salsa-rs/salsa,ratatui/ratatui}`.
- **Crux:** https://github.com/redbadger/crux; https://docs.rs/crux_core/0.20.0/crux_core/trait.App.html; https://docs.rs/crux_core/0.20.0/crux_core/command/index.html; https://docs.rs/crux_core/0.20.0/crux_core/command/struct.Command.html.
- **iced** `Task`: https://docs.rs/iced/0.14.0/iced/struct.Task.html.
- **relm4** async components: https://relm4.org/book/stable/threads_and_async/async.html.
- **kameo:** https://docs.rs/kameo/latest/kameo/message/trait.Message.html.
- **ratatui** application patterns: https://ratatui.rs/concepts/application-patterns/.
- **Tokio actor pattern:** https://ryhl.io/blog/actors-with-tokio/.
- **Local, CodexCLI repo:** docs/rtwk-port/research/state-management.md (round 1); research/codex-tui-architecture.md §1–2, §4; architecture.md ADR-001–004, 007, 009.
- **Local, rtwk:** docs/guide/state.md; crates/rtwk-state/src/{lib,provider,reactive}.rs; crates/rtwk-tokio/src/lib.rs; docs/design/framework-api.md (D-over-A, `memo`, ADR-017, budgets); docs/research/state-adapters.md
- **Oracle** (codex-rs/tui/src): app/event_dispatch.rs:35-80, 1650-1678, 2515-2560, 2700-2760, 3040-3110; app/model_defaults.rs:40-230; app_event_sender.rs:20-125; app_event.rs:319-345, 800-812, 1385; chatwidget.rs:538, 626, 751, 1705-1769; chatwidget/protocol.rs:4; chatwidget/protocol_requests.rs:1-60; chatwidget/streaming.rs:307-350, 643-700; streaming/chunking.rs:85-116; bottom_pane/approval_overlay.rs:385-440; bottom_pane/list_selection_view.rs:125, 345-370; bottom_pane/bottom_pane_view.rs:30
