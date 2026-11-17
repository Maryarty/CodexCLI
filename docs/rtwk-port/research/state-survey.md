# App state for Codex on rtwk: the Rust landscape, judged (research dated 2026-11-17)

Scope, as the person set it last: the Rust options in the category of
application-state management (libraries and architectures), judged for the
Codex CLI TUI rebuilt on rtwk. Frame pacing, per-frame coalescing and
virtualisation are rtwk's job and are not covered. Sources: crates.io API
(fetched 2026-11-17), shallow clones under
`$TMPDIR/codexcli-research/state-survey/src/` (read, not built), the oracle
`codex-rs/tui`, rtwk, and the prototype at
`~/.cache/codexcli/research/app-state-proto` (one variant added, E).

## 1. Synthesis and recommendation

**No library in this category fits better than a plain model.** Three things
decide it.

1. **The large Rust apps keep a plain owner and move results in by message.**
   Zed, Helix, gitui, yazi and cosmic-files all do this. None of them uses a
   state library. They differ only in how a view learns of a change:
   - Zed: explicit `cx.notify()` on an entity.
   - Helix: a redraw request.
   - yazi: a `NEED_RENDER` flag.
   - iced/cosmic: the view runs again after every `update`.
2. **The cost gap comes from the library, not the push model.** Round 3
   measured the plain model C against reactive_graph (A-sig) and
   reactive_stores (A-opt). This round adds E: Zed's entity-and-notify
   pattern written by hand over `rtwk_state::Source`. E costs what C costs:
   - E: 8.1 allocations per delta, 2.5 ms scenario p50.
   - C: 7.3 allocations per delta, 2.4 ms.
   - A-sig: 15.6, 3.6 ms.
   - A-opt: 41.8, 6.6 ms.

   So the round-3 gap is reactive_graph and reactive_stores overhead, not
   push against rebuild.
3. **No library meets the acceptance criteria that matter.** Request and
   generation ids and dropping stale results live in the apply code
   everywhere:
   - yazi: tickets.
   - cosmic-files: compares the request key.
   - gitui: overwrites the next job.
   - Zed: dropping a `Task` cancels it.
   - matrix-sdk: sends `Reset` on lag.

   Race tests need one writer applying events in order. Every candidate
   reaches that only when confined to the UI thread.

**Recommendation (the person decides ADR-003):**

- **Core: the plain model C.**
  - Rust structs owned by the rtwk `AppView`, with one writer (the UI thread).
  - `apply(&mut Model, Ev)` per feature, keyed by thread.
  - Events enter through one rtwk channel. RPC replies carry the request or
    generation id and are checked in `apply`.
  - Derived values are computed in the view, kept with `ViewCx::memo` keyed
    on per-slice revisions. Counters that would cost O(n) are maintained in
    `apply`.
- **Reserve: E for panels that must rebuild without the root view.**
  - Shape: per-thread entities, `notify` inside `update` (never a separate
    call), projections filtered with `PartialEq` and followed through
    `Select`.
  - Use it only if a measurement in phase 2 shows the root view's cost. Zed's
    history shows its failure mode (§3.1).
- **Optional tool: `statig`** (or plain enums) for flow state machines such
  as approvals and onboarding. It is orthogonal to the choice above.
- **Not taken:** every signal and store library (§2). The reasons are cost
  (measured), `Send + Sync` bounds, silent untracked reads, dead or tiny
  crates, or being bound to another UI framework.

## 2. Landscape

Downloads are crates.io `recent_downloads` (90 days). "Rev" is the number of
crates.io reverse dependencies. Dates are the newest non-yanked release.

| Crate / pattern | Model (2 lines) | Maturity | Events and async in, stale results | Derived state | Fit with rtwk | Verdict |
|---|---|---|---|---|---|---|
| **Plain model + `ViewCx::memo` (C)** | Rust structs owned by `AppView`. Handlers mutate them; the view is a function of the model; memo keeps unchanged subtrees. | rtwk API. Same shape as Helix, gitui, yazi, iced (§3) | `AppCx::channel` or rtwk-tokio. Ids are checked in `apply` | Recomputed in the view per change batch. Memo deps are compile-checked: `MemoFn` is a `fn` pointer and cannot capture (rtwk/src/app.rs:839) | Native. This is the rtwk guide's first recommendation (docs/guide/state.md:3-6) | **Shortlist #1 (recommended)** |
| **Entities + explicit notify (E, Zed's GPUI pattern by hand)** | Per-thread `Entity` with an observer list. The writer notifies; projections are filtered at the source and followed by `Select`. | Pattern from the largest Rust UI app; GPUI itself is a whole framework (gpui 0.2.2: 297k downloads, 158 rev) | Same channel. Ids in apply | Kept by observation: a `Sums` entity observes the threads (GPUI `observe`) | Uses `rtwk_state::Source`/`Select`, 306 lines here | **Shortlist #2 (reserve)** |
| reactive_graph (+ rtwk `Derived`) (A-sig) | Leptos's fine-grained signals and memos; reads are tracked. | 1.83M downloads, 0.2.16 (2026-10-31), 47 rev, 2 owners; 0.3.0-rc1 pending | Must be confined to the UI thread: cross-thread reads panic (round 1, A1) | Automatic. A forgotten tracked read fails silently | rtwk-state feature exists | **Shortlist #3** if the person wants automatic tracking: 2.1x allocations, 1.5x time |
| reactive_stores (A-opt) | Derived field-level store over reactive_graph. | 1.80M, 0.4.4 (2026-10-31), 21 rev | As above; keyed `Vec` push is O(n) (round 1, A3) | Automatic | Adapter is possible | No: 5.7x allocations, 2.7x time |
| dioxus-signals / dioxus-stores | Generational-box signals. | 1.58M / 1.21M, 0.7.10 (2026-10-16), 40 / 4 rev | Need the Dioxus reactive runtime (`dioxus_vdom::Runtime`, packages/signals/src/copy_value.rs:4) | Automatic | Duplicates rtwk's runtime | No: not standalone |
| futures-signals | Push signals as `futures` streams. | 98k, 0.3.34 (2024-07-26), 1 owner | Asynchronous delivery; adapter deadlock seen (round 1, B1) | Signal combinators | Needs a driver per subscription | No: stale, measured problems |
| eyeball / eyeball-im | Observable value and `ObservableVector` that broadcast `VectorDiff` batches (imbl vector + tokio broadcast). | 573k / 580k (via matrix-sdk), eyeball 0.8.8 (2024-06-22), eyeball-im 0.9.1 (2026-09-14) | Subscribers are async streams. A lagging subscriber gets `Reset{values}` (eyeball-im/src/vector/subscriber.rs:127, 215) | None. The consumer applies diffs | Duplicates rtwk's keyed rebuild; every push clones into a broadcast message with a state snapshot (vector.rs:108-112, 327-330) | No for the TUI. Worth it only if one core ever feeds several front-ends (matrix-sdk-ui does that for FFI apps) |
| rxRust | Rx observables and operators (`switch_map` for dropping stale results). | 14k, 0.15.0 (2026-10-11), 5 rev | Operators and a scheduler | Operator chains | A second scheduler beside rtwk's | No: tiny, foreign model |
| redux-rs, reducer, rs-store | Store, reducer and middleware crates. | 1.1k (2022-03-13), 1.0k (2022-04-12), 2.0k with 0 rev | n/a | Manual | n/a | No: dead or unused |
| yewdux, bounce | Global stores for Yew. | 28k (2026-04), 1.6k (2023-10) | Bound to Yew | n/a | n/a | No: bound to Yew |
| Crux (`crux_core`) | Headless core: `update` returns effects as data, a shell runs them. | 49k, 0.20.0 (2026-08-07), 8 rev; every minor breaks | Effects as values with typed results; ids in the model | Manual (`view` fn) | Works as a crate beside rtwk | No as a dependency. The effects-as-data idea stays optional (§4) |
| iced / libcosmic (pattern) | Elm-style `update(&mut self, Msg) -> Task`, `view(&self)`. | iced 836k, 0.14 (2025-12-07), 434 rev | `Task` results come back as messages; cosmic-files drops stale ones by key (src/app.rs:4666-4670) | `lazy` widget plus recompute | A whole framework; only the pattern applies, and C already is that pattern | Pattern only (= C) |
| relm4 | Elm-style components on GTK4, `tracker` for change tracking, factories for lists. | 833k, 0.11.0 (2026-04-08) | Commands and async components | Tracker macros | Bound to GTK | No |
| tui-realm | ratatui framework with components, `Msg` and `update`. | 45k, 4.1.0 (2026-05-02), 30 rev | Ports and subscriptions | Manual | Duplicates rtwk (a ratatui framework) | No |
| bevy_ecs standalone | ECS world with `Changed<T>` ticks and double-buffered `Messages`. | 2.77M, 0.20.1 (2026-10-28), 807 rev | Unread messages "will be dropped silently" after two updates (bevy_ecs/src/message/messages.rs doc); not lossless and ordered | Change ticks | A schedule that competes with event order | No |
| salsa | Pull-based incremental computation. Revisions, memoised queries, backdating (book/src/plumbing/fetch.md:27). | 3.39M, 0.28.5 (2026-09-24), 168 rev (rust-analyzer) | Setting an input requires `&mut db`; no subscriptions | Incremental, demand-driven | No push, so a view cannot follow it | No for app state. Option for one heavy derived query later |
| differential-dataflow, disruptor | Data-parallel incremental dataflow; LMAX ring buffer. | n/a | Throughput infrastructure | | Wrong category at about 10^3 events/s | Dropped |
| actors: ractor / kameo / actix | Per-actor state behind async mailboxes. | 549k / 610k / 2.9M (all 2026) | Mailboxes; stale handling by hand | None | State must be `Send`; the UI reads copies. off-thread-state.md found no off-thread reader | No |
| tokio `watch` / `broadcast` | Latest value / fan-out with `Lagged`. | tokio | `rtwk_tokio::follow` exists | None | Good for config and connection state | Utility, not the core |
| statig | Hierarchical state machines, sync and async. | 189k, 0.4.1 (2025-07-05), 1 owner | Events as inputs | n/a | Orthogonal, lives inside the model | Optional for flows |
| floem_reactive, sycamore-reactive, leptos_reactive, xilem | Framework-local signal runtimes or view trees. | 13k (2024-11) / 22k / 143k (0.6, 2024-09) / 3.8k | | | Bound to their frameworks or superseded | Dropped |
| **rtwk's own: `Obs<T>`, providers, `Select`, `store_provider`, `Derived`** | Local observable; scoped values with select; follow any `Source`; reactive_graph bridge. | rtwk | Wakes from any thread with pending-flag coalescing (docs/guide/state.md:41-64) | Select filters by `PartialEq`; `Derived` is lazy | The substrate for C and E | Used by C and E |

## 3. Evidence from real apps (what they do, what bit them)

### 3.1 Zed / GPUI (634k lines across the 8 crates read)

**Owner.** `App` owns every entity; an `Entity<T>` handle reads through
`cx` (crates/gpui/src/_ownership_and_data_flow.rs:1-60).

**How a change reaches a view.**

- The writer calls `cx.notify()` (app.rs:3274-3299).
- `notify` invalidates the windows that read the entity during their last
  render. Reads are tracked per entity (`detect_accessed_entities`,
  app.rs:1466-1501; entity_map.rs:154).
- The dirty view and its ancestors are marked (window.rs:2522-2533).
- A `cached` view reuses its prepaint and paint unless it is dirty
  (view.rs:544-559).

So tracking is automatic and per entity, but the change signal is manual.

**Async.**

- `cx.spawn` with a `WeakEntity`; the result is applied in
  `entity.update(cx, ..)`.
- Dropping a `Task` cancels it (executor.rs:149). The usual stale-result
  guard is a `_task: Task<()>` field that is replaced.

**Streaming.** The agent thread (acp_thread) emits
`NewEntry` / `EntryUpdated(idx)` (acp_thread.rs:3948-3952, 5673-5830). The
view remeasures one list item (agent_ui/src/conversation_view.rs:2010-2017).

**What bit them.**

- A forgotten notify means a stale view. 44 merged PRs mention `cx.notify`,
  for example #47600 "agent_ui: Add missing `cx.notify()` calls for
  real-time UI updates", #53126 "profile selector not repainting", #60552
  "MCP server toggle not updating in UI", #59299 "Re-render Git Panel when
  language model registry updates", #66599 "testing example not notifying
  after async task".
- Derived view state is synced by hand after events (`sync_entry`,
  `sync_generating_indicator`, `sync_elicitation_state_for_entry` in
  conversation_view.rs:2010-2026).

That is exactly the bug class the acceptance criteria forbid.

### 3.2 Helix (108k)

- **Owner.** One `Editor` and a `Compositor` of layers.
- **Async.** Async work returns closures run on the main loop with
  `&mut Editor, &mut Compositor` (helix-term/src/job.rs:12-30). Redraw is
  requested explicitly (helix-event/src/redraw.rs:28).

The single owner and message-in shape is C's.

### 3.3 gitui (50k)

- **Async jobs.** `AsyncSingleJob` keeps only the newest pending job and
  overwrites `next` (asyncgit/src/asyncjob/mod.rs:66-72, 114-117). That is
  stale-drop by replacement.
- **Ingress.** A select over six channels: input, git, app, ticker, notify,
  spinner (src/main.rs:296-314).

### 3.4 yazi (83k)

- **Ingress.** Two unbounded channels, `preempt` (high) and `emit` (normal)
  (yazi-shared/src/event/event.rs:18-24). The loop drains everything pending
  (yazi-fm/src/app/app.rs:62-71).
- **Change signal.** A `NEED_RENDER` flag with a render throttle of about
  10 ms (app.rs:74-86).
- **Stale results.** Results carry tickets; a stale ticket is dropped
  (yazi-fs/src/entries.rs:49-53; yazi-core/src/tab/folder.rs:67-79).
- **Revisions.** A revision counter on the entries (entries.rs:42).

Tickets plus revisions is C plus ids.

### 3.5 cosmic-files on libcosmic/iced (27k)

- **Shape.** Elm-style `update -> Task`. The rescan result carries its
  location and is applied only if the tab still shows that location
  (src/app.rs:4666-4670).
- **Cost.** One `update` in a 7,249-line app.rs. The feature split that
  round 2 asked for (sub-models with `map`) is the remedy, and Codex needs it.

### 3.6 matrix-rust-sdk timeline (29k in `matrix-sdk-ui/src/timeline`)

- **State.** Behind `Arc<RwLock<TimelineState>>`. The items are an
  `ObservableVector<Arc<TimelineItem>>`, mutated in transactions
  (controller/state.rs:44-50; controller/observable_items.rs:70-78, 221).
- **Subscribers.** They get batched `VectorDiff` streams (subscriber.rs:57-100).
- **Settled items.** They are `Arc` and replaced whole by `set`
  (observable_items.rs:196).

It is the closest domain match, a streaming transcript. It needs diffs
because its consumers are foreign UIs across FFI. rtwk's keyed rebuild
already does that diffing.

### 3.7 The oracle's own ingress (codex-rs/tui and app-server-client)

**What arrives.**

- **About 84 notification kinds** (app-server-protocol/src/protocol/common.rs:2055-2200).
- **Hot ones:** agent message, plan, reasoning and command-output deltas
  (common.rs:2104-2136). Each carries `thread_id`, `turn_id` and `item_id`,
  but no sequence number (v2/item.rs:1575-1580, 1644-1649).

**How it gets to the TUI.**

- **Consumer queue.** app-server-client keeps it unbounded so unread
  notifications cannot block RPC replies. Delivery is ordered and lossless,
  and a `Lagged` marker is surfaced (app-server-client/src/lib.rs:14-21,
  398-399, 120).
- **On `Lagged`.** The TUI resets the derived overview state
  (app/app_server_events.rs:73-95).
- **Per-thread channels.** They are bounded at 32,768 (app.rs:333). A replay
  store merges deltas up to 4 KiB and caps them at 256 KiB
  (app/thread_event_buffer.rs:7-8, 11-36).
- **Stale guards.** Generation ids already exist (app_event.rs: `generation`
  appears 9 times, e.g. `ConnectorScopeGeneration`).

The new app keeps all of that in its `apply` layer.

**What bit it.** State desync and races are 87+24 fixes. Examples:
b83105710 (stale transcript on thread switch) and e43a2e297 (stale
background poll events) (bug-classes.md:62-63).

## 4. How the shortlist scores (acceptance criteria, then phases 2-3)

| Criterion | C plain model | E entities + notify | A-sig signals |
|---|---|---|---|
| Views from state only | Yes: view = f(model) | Yes: `Select` over projections | Yes |
| Derived values computed, never synced | Yes: computed in view, memo by value or revision | Partly: derived rows kept by observers. A new input with no observer is stale (Zed's class) | Yes: automatic; an untracked read is stale silently |
| Request / generation ids, stale dropped | In `apply` (as yazi, cosmic, gitui) | Same | Same |
| Races tested | Deterministic: events in, model out, no UI | Same, plus observer order | Only with UI-thread confinement (round 1, A1) |
| Cost (same harness, release, 21 runs) | 7.3 allocs/delta, 2.4 ms | 8.1, 2.5 ms | 15.6, 3.6 ms |
| Per-change root view run | Yes, once per batch; derived recomputed whether shown or not | No: only subscribed widgets rebuild | No |
| Boilerplate per feature | A sub-model, `apply` arm, memo keys | Entity, projections, observers, notify | Signals per field, memos |
| Standard in Rust | Helix, gitui, yazi, iced/cosmic | Zed | Leptos ecosystem |
| Dependencies | none | none | reactive_graph pinned `=0.2.16`, rc pending |

**Risks of C over phases 2-3, each with its mitigation:**

1. **The root view runs once per change batch.** All derived values are
   recomputed then; one O(n) projection (for example a token sum over 10^4
   items) costs every frame. Mitigation: running totals are maintained in
   `apply`; panel subtrees are memoised on slice revisions.
2. **A forgotten revision bump means a stale subtree.** Mitigation: bump in
   one helper per slice; memo deps are the values themselves where cheap.
3. **One big `update` grows** (cosmic-files has 7.2k lines). Mitigation:
   feature sub-modules keyed by thread; the parent only routes.

If (1) shows in a phase-2 profile, move that panel to E with notify inside
`update`. It runs on the same channel and the same model types.

## 5. Measurement (variant E added to the round-3 harness)

**Scenario (unchanged):**

- 2 s at 1 ms steps.
- Thread 0 streams 1,000 deltas/s with a commit every 10.
- An approval at 800 ms, answered at 1,100 ms.
- Thread 1 notified twice while unfocused.
- 1,000 seed entries.

**Build.** Release, `--features count` for allocations, plain release for
times, 21 runs.

| Variant | write allocs/delta | flush allocs/frame | total allocs/delta | UI pump us/frame | scenario ms p50 | frame step p95 ms |
|---|---|---|---|---|---|---|
| C plain model | 1.1 | 102 | 7.3 | 17.4 | 2.4 | 0.019 |
| **E entities + notify** | 1.1 | 116 | **8.1** | 17.9 | **2.5** | 0.020 |
| A-sig reactive_graph | 1.1 | 239 | 15.6 | 26.7 | 3.6 | 0.032 |
| A-opt reactive_stores | 1.1 | 673 | 41.8 | 51.0 | 6.6 | 0.055 |

**Behaviour.** All 9 tests pass with E included: identical settled screens,
rebuild-isolation bounds, and 8 of 8 cross-thread trials with 1 and 2 tokio
writers.

**Widget builds.** E builds the same widgets as A-sig (live 120, entry 200,
status 121). It evaluates 129 summary rows against A-sig's 127 memo
evaluations.

**Reproduce:**

- `cd ~/.cache/codexcli/research/app-state-proto`
- `acargo test -q --release`
- `acargo build -q --release --features count && ./target/release/proto 1`
- `acargo build -q --release && ./target/release/proto 21`

The code is in `src/e.rs`. The harness registers `V::E`, and tests.rs gives E
the same mid-stream exemption as A-opt and A-sig, for the second wake inside
a flush.

## 6. Audit of the prior rounds

**Right:**

- A single writer on the UI thread.
- Events through one channel.
- Effects' results re-enter as events.
- Local widget state stays in rtwk.
- reactive_stores is too costly on hot data.
- The cross-thread panic in reactive_graph (measured).
- "No published snapshot needed" (off-thread-state.md).

**Never looked at:**

- Zed's entity-and-notify model, the largest Rust UI app. Now measured as E.
- eyeball / eyeball-im and the matrix timeline, the closest domain match.
- rxRust, yewdux, tui-realm, relm4's tracker.
- statig and state machines for flows.
- How real apps drop stale results (tickets, key compare, task-drop
  cancellation).

salsa, bevy_ecs, actors, Crux, iced, Dioxus and futures-signals were covered.

**Claims without evidence:**

- **Round 2's "TEA is the most widely documented Rust pattern" as an
  argument.** It is documented, but the large Rust apps read here (Zed,
  Helix, gitui, yazi) use a single mutable owner with messages, not effects
  as data. The benefit of effects as data at Codex's size is unshown.
- **The cost of "reactive" in round 3.** The C-vs-A gap was taken to mean
  reactivity costs. E shows the push model costs the same as C. The gap is
  library overhead.
- **Maintainability judged on a 69 vs 95-line toy (round 3 §4).** The real
  evidence is Zed's notify PR history (§3.1) and cosmic's 7k-line `update`
  (§3.5).
- **rtwk's 26-vs-3 allocation figure** (debug fixture, docs/guide/state.md)
  was reused in every round as if it were app evidence.

## 7. For the person

1. ADR-003: C (recommended), with E kept as the documented reserve for panels
   the root view should not rebuild. Or A-sig, if automatic tracking is
   worth 2.1x allocations and silent untracked reads.
2. Whether flows (approvals, onboarding, login) use `statig` or plain enums
   inside the model.
3. Whether app-server notifications should gain a per-thread sequence number,
   so replay and reconnect can deduplicate without content comparison. Today
   only `thread_id/turn_id/item_id` exist. This is protocol work, outside
   the TUI.
