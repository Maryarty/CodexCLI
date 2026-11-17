# State management for a 1:1 Codex TUI on rtwk

Research, 2026-11-17. Read-only on both repos. Numbers: crates.io API and
GitHub API pulled 2026-11-17 ("90d" = crates.io `recent_downloads`).

## 1. Synthesis and recommendation

**Primary: an Elm-style single session store, hosted outside the tree and
followed through `rtwk-state`.** One plain-Rust `Session` model; one `Msg`
enum (today's `AppEvent` plus the app-server notifications, kept verbatim);
in-place `update(&mut Session, Msg, &mut Vec<Fx>)` functions (the bodies of
today's `&mut self` handlers); effects returned as data (`Fx`) and run by a
driver that feeds results back as `Msg`. The driver drains the channel in
batches, applies them, bumps per-slice revision counters and notifies once.
Widgets follow cheap `PartialEq` projections with `Select` /
`store_provider` and dispatch by sending a `Msg`.

Why this one:

1. **It is what Codex already is.** Codex has an mpsc bus (`AppEventSender`
   over `UnboundedSender<AppEvent>`), 331 `AppEvent` variants, a central
   `handle_event` match, a ~91-arm `handle_server_notification`, and
   `&mut self` handlers that mutate one owner (ChatWidget). That is TEA with
   the model fused into the view. Separating model from view is the whole
   migration; handler bodies move nearly verbatim.
2. **1:1 behaviour needs atomic multi-field mutation in order.** Handlers
   touch transcript, stream controller, active cell, bottom pane and status
   in one call. Per-field signals or per-actor mailboxes lose that ordering;
   a single reducer keeps it.
3. **Fits rtwk's stated boundary.** ADR-019 amendment 5: external store is
   the source of truth, subscribed in `init_state`, dropped in `dispose`,
   wake from any thread. `Source<R>` is two methods; the measured adapter
   cost is 3 allocations per write and 1 wake / 1 rebuild per 100-write
   burst (docs/guide/state.md).
4. **Testable without a terminal.** `update` is a function of
   (model, msg): snapshot tests on the model or on rendered cells, no tree.
5. **Streaming is cheap if the reducer batches.** Per-token work is
   `push_str` into a buffer inside `update`; notify once per drained batch;
   rtwk's pending-flag coalescing gives one wake per frame regardless of
   token rate. Codex already separates delta intake from commit
   (`AppEvent::StartCommitAnimation`, `FrameRequester`).

**Stays local widget state (rtwk `State` / hooks), not in the store:** text
cursor and selection inside the composer's `TextEditingController`, scroll
offsets and `ScrollController`, focus nodes, hover/pressed, animation phase
(spinner, `EmptyStateAnimation`), a picker's highlighted row until the user
confirms, layout caches (`active_cell_layout`), and transient popup open/
close that no other subsystem reads. Rule: if only one widget and its
children read it and no `Msg` handler or test needs it, it is local.

**Not recommended as the primary:** signals (see runner-up), actors (the
mpsc bus we keep is already the actor-style transport; adding a framework
buys supervision Codex does not use), a third-party Redux crate (all
unmaintained or tiny; see table).

**Runner-up: fine-grained signals (`reactive_graph`).** Lost because
(a) Codex state is large, non-`Clone`, mutated in place (`Box<dyn HistoryCell>`,
`StreamController`, `RefCell` caches), which signals model badly (every
field becomes an `ArcRwSignal`, `Send + Sync` bounds propagate); (b) ordering
across fields is exactly what the 331 handlers rely on; (c) the migration
re-expresses every handler; (d) 0.3.0-rc1, and in rtwk's own measurement a
write cost 26 allocations vs 3 for a plain `Source` (docs/guide/state.md
table). It stays a valid *derived-value* tool inside the store if a view
needs a lazy computed projection (the `reactive-graph` feature of
`rtwk-state` exists for that).

## 2. What I read (local)

rtwk: `docs/guide/state.md` (Source/Select/store_provider, coalescing,
allocation table); `crates/rtwk-state/src/lib.rs` (`Source<R>`,
`Select<R>`; `follow` subscribes before the first `get`, one pending flag,
`channel_with` handler returns `Rebuild::No` on equal selection);
`docs/research/state-adapters.md` (prototype: Redux-like store, rs-store
3.0.0, reactive_graph 0.2.16 all pass six checks without core changes);
ADR-019 amendment 5 (docs/DECISIONS.md:729-751); rtwk-hooks / rtwk-provider
(`use_state`, `use_effect`, `use_stream`; `InheritedProvider`,
`provider::select`) skimmed.

Codex (`codex-rs/tui/src`):
- `app_event.rs` 2103 lines; `pub(crate) enum AppEvent` lines 356-2034,
  **331 variants** (awk count of top-level variants).
- `app.rs` 1574 lines + `app/` **167 files, 86.6k lines**; 244 top-level
  files in `src/`. `app/event_dispatch.rs:35 handle_event` is the
  central match; `app/thread_events.rs:98 ThreadEventStore` is already a
  bounded replay store (turns, buffer, active_turn_id).
- `chatwidget.rs` 2183 lines + `chatwidget/` (`protocol.rs:4
  handle_server_notification(&mut self, ServerNotification, replay)`,
  91 `ServerNotification::` references; `streaming.rs:307 on_agent_message_delta`
  pushes into a buffer / stream controller; `transcript.rs:35
  TranscriptState` with `active_cell`, `active_cell_revision`,
  streamed-message ids).
- `bottom_pane/mod.rs` 4437 lines; `view_stack: Vec<Box<dyn BottomPaneView>>`,
  97 files in `bottom_pane/`; views are trait objects with `&mut` methods.
- `history_cell/mod.rs:210 trait HistoryCell: Debug + Send + Sync + Any`
  (good: cells can be `Arc`-shared across the store boundary).
- `app_event_sender.rs`: `AppEventSender { UnboundedSender<AppEvent>, .. }`;
  `tui/frame_requester.rs`: `FrameRequester` coalesces draw requests.
Not verified: the commit-tick period and the real delta rate; measure
before fixing the batch policy.

## 3. Options surveyed

### 3.1 Elm / TEA
- **tui-realm** 4.1.0 (2026-05-02): 268k total, 45k/90d, 1.0k stars,
  pushed 2026-10-24. "Elm and React" components, messages, `update`.
  Owns the event loop and component mounting, so it competes with rtwk's
  tree. https://crates.io/crates/tuirealm https://github.com/veeso/tui-realm
- **ratatui** 0.30.2: 72.2M total, 27.4M/90d, 23.3k stars. Documents TEA,
  Component and Flux patterns, as prose and templates, not as a crate.
  https://ratatui.rs/concepts/application-patterns/the-elm-architecture/
- **iced** 0.14.0 (2025-12-07): 3.39M total, 836k/90d, 32.1k stars. The
  model-update-view loop is a GUI runtime; only the idea transfers.
  https://crates.io/crates/iced
- **teatui** 0.4.3: 883 total, 10/90d. **ratatui-elm** 2.0.1: 2.1k total,
  68/90d. **tears** 0.11.1: 2.0k total, 1.0k/90d. All too small to depend on.
- **crux_core** 0.20.0: 41k total (prior research, 2026-11-06); Elm-like
  core, effects as data, shell-driven. Same shape as the recommendation but
  built for FFI shells.
Verdict: the *pattern* is right and ~100 lines to own; no crate is both
maintained and runtime-neutral.

### 3.2 Redux / Flux store, reducers, effects
- **redux-rs** 0.3.3, last release 2022-03-13: 19k total, 1.1k/90d, 242
  stars (repo pushed 2026-09-06). Stale release.
- **rs-store** 3.0.0 (2025-11-27): 50.5k total, 1.95k/90d, 7 stars. Thread,
  reducers, middleware, thunks, backpressure. Works behind `Source`
  (prototype passes six checks) but single-maintainer, tiny.
  https://github.com/rookiecj/rs-store
- **yewdux** 0.13.0 (2026-04-07): 247k total, 27.7k/90d, 327 stars;
  Yew-bound (context + `Store` trait), idea only.
- **ratatui component/Flux** docs and template (ratatui/templates, 429
  stars, tokio events, `Action` enum, `Component` trait). It is the
  reference shape for "Action + update" in a ratatui app; the Flux page
  keeps a dispatcher and a store.
  https://ratatui.rs/concepts/application-patterns/flux-architecture/
Verdict: same model as 3.1 with a store object. Write ours (~150 lines);
the prototype's `redux.rs` is the starting point.

### 3.3 Fine-grained signals
- **reactive_graph** 0.3.0-rc1 (2026-10-31): 4.09M total, 1.83M/90d;
  Leptos 21.5k stars. Usable without Leptos' runtime (`Arc*` + immediate
  effects; prototype). Pre-1.0.
- **dioxus-signals** 0.8.0-alpha.3 (2026-10-16): 4.05M, 1.58M/90d; dioxus
  39.8k stars. Bound to dioxus' runtime and scopes.
- **sycamore-reactive** 0.10.0-beta.1 (2026-11-09): 317k, 21.6k/90d, 3.4k
  stars. **futures-signals** 0.3.34 (2024-07-26): 1.10M, 98k/90d, no
  release in 28 months.
Verdict: best for many small independent values; poor for one large
ordered state machine.

### 3.4 Actors / channels
- **tokio** 1.54.1: 1.2B total, 314M/90d (the mpsc Codex uses).
- **ractor** 0.16.6 (2026-11-06): 1.57M, 549k/90d, 2.1k stars. **kameo**
  0.22.2 (2026-07-18): 1.23M, 610k/90d, 1.4k stars. **actix** 0.13.6
  (2026-11-08): 21.4M, 2.9M/90d, 9.3k stars.
Verdict: right for isolated concurrent services (app-server session,
MCP, file watcher); wrong for UI state, where one ordered owner is wanted.
Codex's mpsc is already the transport of option 3.1/3.2.

### 3.5 Framework-local (rtwk `State`, `Obs<T>`, providers, hooks)
No dependency. `Cx::channel_with` handlers act as `update`; `Obs<T>` plus
`cx.watch` for small shared values; `provider::select` for subtree reads;
`memo` for reuse. Limit (state-adapters.md claim 8): `Obs` is not
UI-independent, has no subscribers on other threads, so the session task,
persistence and tests cannot read it. Right for local state, insufficient
as the session's source of truth.

## 4. Comparison

| | 1 TEA (own/crate) | 2 Redux store (own) | 3 Signals | 4 Actors | 5 rtwk-local |
|---|---|---|---|---|---|
| Maturity | tuirealm 268k dl; teatui/ratatui-elm negligible | rs-store 50k, redux-rs stale 2022 | reactive_graph 4.1M rc; futures-signals stale | tokio, ractor 1.6M, kameo 1.2M | in-tree |
| Threads / tokio | channel in, update on one owner | store thread or task, `Send` subscribers | `Arc*` signals are `Send+Sync`; parallel writers race on effect order | native | UI thread; `Sender` from any thread |
| Testability | pure `update`, snapshots | pure reducers, snapshots | harder: effect timing, owners | message-level, async | needs `WidgetTree` |
| Retained tree | root rebuild unless projected | `Select` per projection | node per signal; rtwk decides build time | none built in | native |
| Codex migration | low: handlers move, `AppEvent` kept | same as 1, plus store object | very high: ~331 handlers re-cut | high: ownership splits, ordering lost | medium: model stuck in widget State |
| 100-1000 ev/s | batch drain, 1 wake/frame | same; measured 100 writes = 1 wake, 1 build | 26 allocs/write (fixture) | mailbox per msg; fine | `channel_with` item alloc per send unless `()` flag |
| rtwk-state fit | `Source` over projection | `Source` + `store_provider` (prototype) | `Derived` feature | `Source` over actor snapshot | n/a (core) |

## 5. Mapping onto rtwk-state, no core change

- `Store::select(fn(&Session) -> R)` returns `Rc<dyn Source<R>>`: `get`
  reads under a read lock; `subscribe` registers a `Fn(R)` that the driver
  calls after publishing; the source filters equal consecutive values.
  `Select::new` keeps one subscription per mounted widget; `Select`'s
  pending flag coalesces a burst to one `()`.
- A wide subtree (transcript + bottom pane + status) uses `store_provider`
  with `Rc<Store>`, descendants call `rtwk::provider::select`; one
  subscription, selectors decide rebuild (prototype: 1 wake, 0 builds on
  unselected change).
- Projections must be cheap and `PartialEq`: revision tuples or
  pointer-compared `Arc` snapshots (`Snap<T>`). Never clone cells.
- Dispatch is `Store::dispatch(Msg)`: an `UnboundedSender<Msg>` clone, the
  `AppEventSender` API unchanged for callers. It is `Send`, so backend
  tasks, key handlers and timers use the same call.

## 6. Sketch

```rust
// --- model: plain data owned by the store; no widget types ---
pub struct Session {
    pub rev: Revs,
    pub transcript: Snap<Vec<Arc<dyn HistoryCell>>>, // committed, append-only (CoW)
    pub stream: Option<InFlight>,        // raw md buffer + stream controller state
    pub active_cell: Option<Arc<dyn HistoryCell>>,
    pub views: Vec<ViewSpec>,            // bottom-pane stack as DATA, not Box<dyn View>
    pub approvals: VecDeque<ApprovalRequest>,
    pub config: Arc<Config>,
}
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Revs { pub transcript: u64, pub stream: u64, pub views: u64, pub approvals: u64, pub config: u64 }
pub struct Snap<T>(pub Arc<T>);          // PartialEq = Arc::ptr_eq

pub enum Msg { App(AppEvent), Server(ServerNotification, Option<ReplayKind>) }
pub enum Fx { Op(AppCommand), Persist(ConfigEdit), Timer(Tick, Duration), Exit(ExitMode) }

// --- update: today's `&mut self` handlers, minus rendering ---
pub fn update(s: &mut Session, m: Msg, fx: &mut Vec<Fx>) {
    match m {
        Msg::Server(ServerNotification::AgentMessageDelta(d), _) => {
            s.stream.get_or_insert_default().push(&d); // no rev bump per token
        }
        Msg::App(AppEvent::CommitTick) => if let Some(st) = &mut s.stream {
            for c in st.drain_committed() { Arc::make_mut(&mut s.transcript.0).push(c); }
            s.rev.transcript += 1; s.rev.stream += 1;
        },
        Msg::App(AppEvent::ApprovalDecision { id, d }) => {
            s.approvals.retain(|a| a.id != id); s.rev.approvals += 1;
            fx.push(Fx::Op(AppCommand::approve(id, d)));
        }
        Msg::App(AppEvent::PushView(v)) => { s.views.push(v); s.rev.views += 1; }
        /* ... the other 330 variants, moved handler by handler ... */
    }
}

// --- driver: one task; batch drain, one publish per batch ---
async fn drive(store: Arc<Inner>, mut rx: UnboundedReceiver<Msg>, run: impl Fn(Vec<Fx>)) {
    while let Some(m) = rx.recv().await {
        let mut fx = vec![];
        { let mut s = store.state.write();
          update(&mut s, m, &mut fx);
          while let Ok(m) = rx.try_recv() { update(&mut s, m, &mut fx); } }
        store.notify_subscribers();      // each Source compares its projection
        run(fx);                         // spawn effects; results re-enter as Msg
    }
}

// --- widget: selects and dispatches ---
fn bottom_pane(store: Rc<Store>) -> AnyWidget {
    let src = store.select(|s| (s.rev.views, s.rev.approvals)); // kept across builds
    let st = store.clone();
    Select::new(src, move |_| {
        let (views, approvals) = st.read(|s| (s.views.clone(), s.approvals.front().cloned()));
        let d = st.dispatcher();                                 // Sender<Msg>, Clone + Send
        Column::new()
            .child(views.last().map_or_else(composer, |v| view_for(v, d.clone())))
            .child(approvals.map(|a| ApprovalOverlay::new(a, move |dec|
                d.send(Msg::App(AppEvent::ApprovalDecision { id: a.id, d: dec })))))
            .into()
    })
}
```

Notes on the sketch: the composer's text, scroll and focus are local
`State`; `ViewSpec` is an enum (`ModelPicker { .. }`, `Approval { .. }`, ...)
that a function maps to a widget, replacing `Box<dyn BottomPaneView>`
(whose `&mut` methods become `Msg` handlers or local widget state);
`Revs` stands in for hand-written `PartialEq` on large models.

## 7. Risks and costs

- **Splitting ChatWidget** (2183 lines + submodules, ~60 fields incl.
  `RefCell` animation, layout caches) into `Session` vs view is the real
  work; expect long-tail coupling where handlers call render helpers
  (`current_stream_width`) that need the terminal size. Pass layout facts
  in as a `Msg::Resize` field.
- **Bottom pane**: trait-object views with internal `&mut` state
  (97 files) either become `ViewSpec` + local widget state, or stay
  `Box<dyn ..>` inside the model behind a revision counter as a first
  step. The second keeps 1:1 faster, at the price of a model that is not
  `Clone`/testable by equality.
- **Lock held during update** blocks `get` from UI thread; keep updates
  short; stream parsing (markdown) stays in the stream controller on the
  store thread but off the render path.
- **Snapshot-by-Arc CoW** makes a transcript commit O(n) while a view
  holds the old snapshot. Fine per cell, not per token; chunk the vector
  if sessions reach 10^5 cells.
- **Replay/resume** (`ThreadEventStore`, `ReplayKind`) is already a store
  with ordering rules: it becomes a `Msg` source, unchanged.
- Measured numbers in this doc come from the rtwk fixture (debug, macOS
  arm64), not from Codex traffic.

## 8. Open questions (for the person)

1. Reducer thread: UI thread (Codex today: single loop, simplest 1:1,
   `Session` need not be `Send`) or a store task (UI never blocks on a
   token burst, `Session: Send`)? The sketch assumes a task.
2. `ViewSpec` enum from day one, or keep `Box<dyn BottomPaneView>` in the
   model first and convert view by view?
3. Own ~150-line store, adopt rs-store 3.0.0 (7 stars, one maintainer), or
   vendor the prototype's `redux.rs`? The sketch assumes own.
4. Should the composer's text live in the store (history recall, drafts,
   resume) or stay a local controller synced on submit/escape?
5. Is `AppEvent` kept as `Msg` verbatim (mechanical, 331 variants) or
   split into intents (`Cmd`) vs facts (`Evt`) during the port? Verbatim
   is cheaper for 1:1 parity checks.
6. Real delta rate and commit-tick period in Codex: measure before
   choosing the batch cap (e.g. at most N msgs per publish).
7. Is a derived/lazy projection graph needed anywhere (status line,
   token usage)? If yes, enable `rtwk-state`'s `reactive-graph` feature
   there only.

## 9. Sources

- https://crates.io/api/v1/crates/{ratatui,tuirealm,iced,redux-rs,rs-store,yewdux,reactive_graph,futures-signals,dioxus-signals,sycamore-reactive,ractor,kameo,actix,tokio,teatui,ratatui-elm,tears} (2026-11-17)
- https://api.github.com/repos/{ratatui/ratatui,ratatui/templates,veeso/tui-realm,iced-rs/iced,leptos-rs/leptos,DioxusLabs/dioxus,sycamore-rs/sycamore,yewdux/yewdux (renamed intendednull/yewdux),rookiecj/rs-store,redux-rs/redux-rs,slawlor/ractor,tqwewe/kameo,actix/actix}
- https://ratatui.rs/concepts/application-patterns/ (TEA, Component, Flux pages)
- https://github.com/ratatui/templates (component template)
- https://github.com/veeso/tui-realm README; https://github.com/rookiecj/rs-store README
- Local: rtwk docs/guide/state.md, docs/research/state-adapters.md,
  docs/DECISIONS.md:729-751, crates/rtwk-state/src/lib.rs; Codex
  codex-rs/tui/src/{app_event.rs,app_event_sender.rs,app/event_dispatch.rs,
  app/thread_events.rs,chatwidget/{protocol,streaming,transcript}.rs,
  bottom_pane/mod.rs,history_cell/mod.rs,tui/frame_requester.rs}
