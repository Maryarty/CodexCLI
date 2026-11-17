# Application state for Codex on rtwk: approaches, what each is for, what each costs (research dated 2026-11-17)

Scope: the state that lives above rtwk's widgets in the Codex TUI rebuilt on
rtwk (`codex-rs/tui-rtwk`): threads, transcripts, approvals, settings,
derived counts. rtwk owns frame loop, terminal, input, widget state,
rebuild/layout/paint, pacing, coalescing and list virtualisation; none of that
is judged here. This write-up reorganises the evidence of the earlier rounds
by **approach**, not by crate. It reuses, and cites by section:

- `state-survey.md` (landscape §2, real apps §3, scoring §4, measurement §5),
- `app-state.md` (round 1 prototype, findings A1-A5, B1, C1),
- `app-state-aopt.md` (round 3: A-opt, A-sig, dash scenario §4),
- `state-v2.md` §1, §4 (TEA/Crux), `state-management.md` §3,
- `bug-classes.md` §1-2, `desktop-gap.md` §1, §3,
- rtwk `docs/guide/state.md`, `crates/rtwk/src/app.rs`.

One new measurement (§11): how the plain model's whole-view rerun scales
with app size. Prototype copy with the new `scale` scenario:
`$TMPDIR/codexcli-research/state-approaches/proto` (`src/scale.rs`).

The acceptance criteria (binding) are abbreviated below:

- **K1** views are derived from state only, no "refresh UI" calls;
- **K2** derived values computed, never synced by hand;
- **K3** async results carry request or generation ids, stale results dropped;
- **K4** races tested;
- **K5** maintainable over phases 2-3 (Desktop features, `desktop-gap.md`).

## 1. Synthesis

**What the approaches are actually for.** The eight families answer
different questions. Only three of them answer the question Codex has —
"one ordered stream of events changes a domain model; how do 30 screens show
it, correctly and cheaply":

| Family | The problem it was built for | Codex's problem? |
|---|---|---|
| 1 Plain model + view rerun + memo | Keep one source of truth and make the view a function of it; let the framework diff | Yes |
| 2 Store + reducers + effects as data | Make every change an explicit, replayable, testable value; keep I/O out of logic | Yes (the effects part is the addition) |
| 3 Fine-grained signals | Update the exact DOM node that depends on a value, without a virtual-DOM diff | Partly: rtwk already diffs; signals would duplicate it |
| 4 Observable entities / diffs | Many views over shared mutable objects in a large app (Zed); feed diffs to foreign UIs (matrix-sdk) | Partly: a reserve for panels that must not wait on the root view |
| 5 Actors | Isolate concurrent, failure-prone services with their own mailboxes | No for UI state; yes for services (already app-server) |
| 6 ECS | Iterate many homogeneous entities per tick in a game loop | No |
| 7 Incremental computation | Recompute expensive derived results on demand after small input edits (compilers) | No as a core; maybe one heavy query |
| 8 Streams / Rx | Compose asynchronous event sources with time operators | No as a store; ingress already is a stream |

**What decides it.** Five facts, each with evidence below:

1. Codex's hard part is not propagation, it is **ordering and staleness**:
   111 of 508 historical TUI fix commits are state desync and races
   (`bug-classes.md` rows 12-13). No family removes K3 work; every real app
   read keeps ids or tickets in its apply code (`state-survey.md` §1.3).
   What a family *can* do is make the apply step deterministic and testable
   (K4), and make the view impossible to forget (K1, K2).
2. **Push vs rebuild is not the cost driver; libraries are.** Hand-written
   entity-and-notify (E) costs what the plain model costs (8.1 vs 7.3
   allocations per delta); reactive_graph costs 2.1x, reactive_stores 5.7x
   (`state-survey.md` §5).
3. **The failure modes differ in kind.** Family 1's stale-view failure is a
   forgotten revision key inside a `memo` (narrow, local); family 4's is a
   forgotten `notify` anywhere (Zed: 44 merged PRs); family 3's is an
   untracked read (silent and intermittent, measured in
   `app-state-aopt.md` §4).
4. **rtwk is already an update-view framework.** `AppView` owns the model;
   handlers are `update` arms; `AppCx` is "the effects an A+Msg update has"
   (rtwk/src/app.rs:937); `ViewCx::memo` keeps subtrees with
   compile-checked dependencies (`MemoFn` is a `fn` pointer, app.rs:839).
   Families 1 and 2 use it as is; 3, 4 and 8 add a second propagation
   system beside it.
5. **The whole-view rerun scales at Codex's size**, but it has a growth law
   to respect (§11): about 10-20 µs per frame at 1,000 threads and 30
   panels, rising to 0.65 ms when 1,000 subtrees are memoised in one view.
   The cost is quadratic in memo calls (rtwk's memo cache is a linear
   search, app.rs:812-819), not in model size.

**Recommendation (the person decides ADR-003):** family 1 (plain model,
whole-view rerun, `ViewCx::memo`) as the core, with **family 2's "effects as
data" at the domain boundary**: `apply(&mut Model, Ev) -> Effects`, run by a
thin shell. Family 4 (entities + notify inside update) is the documented
reserve for a panel that a profile shows must not wait on the root view.
State machines (`statig` or plain enums) and `tokio::watch` are complements.
Reasons in §10; the chosen approach's own weaknesses and their mitigations
in §12.

## 2. The Codex cases used for every approach

- **(a)** 1,000 deltas/s into the active thread.
- **(b)** 5 background threads streaming at once.
- **(c)** thread switch, today with replay of buffered events. The oracle
  keeps a `ThreadEventStore` of raw events per inactive thread and replays
  them through the chat widget on switch, with eviction state such as
  `evicted_active_turn` and `replay_may_have_output`
  (codex-rs/tui/src/app/thread_events.rs:98-121). Bug b83105710 (stale
  transcript on switch) lives there.
- **(d)** an approval request arriving while another dialog is open.
- **(e)** derived values across panels: per-thread unread and running,
  token totals, sidebar filter, agents-center counts.
- **(f)** a settings change affecting many screens (theme, keymap).
- **(g)** a late RPC reply after the user moved on.
- **(h)** phase 2, the most state-heavy Desktop feature: **the Activity view
  over N concurrent threads** (`desktop-gap.md` §1 row 1, with §3
  requirements 1, 8, 9): unread, running and needs-input per thread, an
  attention inbox, and approvals that another client may resolve first.

---

## 3. Family 1: plain model, whole-view rerun, memoised subtrees

**Why it exists.** The view should be a pure function of one model, so
there is nothing to keep in sync; the framework diffs the output. This is
the update-view loop of iced (and libcosmic), and the "keep ordinary
application state in a Rust model" first rule of rtwk's guide
(rtwk docs/guide/state.md:3-6). Helix, gitui and yazi are the same shape
with a manual redraw flag (`state-survey.md` §3.2-3.4).

**Mechanism.** The model is plain structs owned by `AppView`; events arrive
through one `AppCx::channel` and mutate it in `apply`. After a batch, rtwk
reruns `view(&Model)` once per frame. `cx.memo(deps, fn)` returns the kept
subtree when `deps` compare equal.

**What it solves well.**
- K1, K2 by construction: a value computed in `view` cannot be stale;
  inside a kept subtree, a capture does not compile (E0308, checked in
  `app-state-aopt.md` §4).
- Determinism (K4): `apply` is a plain method; events in, model out, no
  runtime (`dash_derivations_without_ui`, round 3).
- Lowest cost measured: 7.3 allocations per delta, 2.4 ms scenario p50
  (`state-survey.md` §5); dash 14.5 µs and 17 allocations per change
  (`app-state-aopt.md` §4).
- No `Send + Sync` on the model; no dependency.

**What it brings.**
- **The root view runs once per frame with a change**, and every
  derivation in it runs whether shown or not (round 1, C1). Cost grows with
  the number of memo calls, quadratically (§11).
- **Proxy keys can be forgotten.** When a memo key is a revision counter
  instead of the value, a forgotten bump is a stale subtree that only a test
  sees (`app-state.md` §7).
- **`cx.unchanged()` misuse** silences a needed rebuild (rtwk app.rs:986-991
  documents it).
- **One big `update`.** cosmic-files' `update` sits in a 7,249-line app.rs
  (`state-survey.md` §3.5); the oracle has 335+ `AppEvent` variants
  (`bug-classes.md` row 13).
- **Nothing off the UI thread can read the model**; notifications to other
  consumers must be emitted by `apply` (`app-state.md` §7).
- **Nested memos do not compose like nested caches** (by reading app.rs:
  805-830 and 386-388): when an outer memo hits, the inner entries it would
  have touched are not marked used and are released at the end of the run;
  the next miss of the outer memo rebuilds every inner subtree. Not
  measured; see §12.

**What it demands from 30 screens written by Sonnet agents.** One rule set,
the same for every screen: a feature sub-model with `apply`; derived values
are pure functions of `&Model`; memo keys are values (or a revision owned by
the slice's mutator); never `unchanged()` after a change the view reads.
The rules are checkable in review and two of them by the compiler.

**Codex cases.**
- (a) Deltas are coalesced by rtwk to one view run per frame; measured 17
  deltas plus 1 thread change per frame at 18.6 µs view time with 1,000
  threads (§11).
- (b) Background threads' events are applied to their own sub-model; the
  root view reruns per frame (not per delta). Cost as (a).
- (c) **Replay disappears**: every thread's model is kept current whether
  shown or not, so a switch changes `active` and the view renders that
  thread. Reconnect still needs a backfill from the server.
- (d) A modal queue in the model (`VecDeque<Approval>` per thread, plus a
  focus rule); the view shows the head over rtwk's dialog barrier. Order is
  the event order.
- (e) Computed in `view` from the model; 10 O(T) scans over 1,000 threads
  add about 9 µs per frame (§11). No hand sync.
- (f) Settings live in the model and are provided once at the root through
  an rtwk provider; kept subtrees that `depend` on it still rebuild (memo
  doc, app.rs:770-778).
- (g) The model holds `pending: Option<(ReqId, TaskId)>`; the reply applies
  only if the id matches; a new request cancels the old task
  (`AppCx::spawn` and `cancel`, app.rs:1001, 1042). Same as yazi tickets and
  gitui's overwrite.
- (h) N thread sub-models in a `BTreeMap`; unread, running, needs-input are
  functions of each; the inbox is a filtered view; an approval resolved
  elsewhere arrives as an event that removes it from the queue.

**Verdict: fits.** It is the shape rtwk is built for, it meets K1, K2, K4
by construction and is the cheapest measured. Its weakness is growth of the
view run, which is bounded and has mitigations (§12).

## 4. Family 2: store with actions and reducers, effects as data

**Why it exists.** Make every change an explicit value (an action), apply
it in a pure reducer, and return side effects as data so logic can be
tested and replayed without I/O. Crux: "Cross-platform app development in
Rust" (crates.io), a headless core whose `update` returns commands a shell
executes (`state-v2.md` §1, §4); redux-rs: "A Rust implementation of
Redux". Round 2 scored the TEA core 89/100 (`state-v2.md` §4).

**Mechanism.** `update(&mut Model, Event) -> Cmd<Effect, Event>`; the core
never does I/O. The shell runs each effect and feeds its result back as an
event. Views read projections of the model.

**What it solves well.**
- K3, K4 at the strongest: a late reply is an event like any other, and a
  race test is a list of events and expected effects; time is an input
  (`Event::Tick { now }`) instead of `Instant::now()` inside a handler
  (the oracle reads it at chatwidget/streaming.rs:663, `state-v2.md` §1).
- Separation by construction: a core crate cannot name a widget or a
  future (`state-v2.md` §1, "Why TEA wins" 1).

**What it brings.**
- Boilerplate: an event variant per result, an effect variant per I/O, a
  mapping layer per feature (`Task::map`, `Command::map_event`).
- Indirection: following "what happens after a click" crosses event, update,
  effect, shell, event. The large apps read (Zed, Helix, gitui, yazi) do not
  use effects as data (`state-survey.md` §6, "Claims without evidence").
- The crates are dead (redux-rs 2022) or churning (crux_core 0.16 to 0.20 in
  13 months, every minor breaking; `Event: Send + 'static`, serde required;
  `state-v2.md` §4 notes).
- Migration: each of the oracle's awaiting handler arms must be cut into
  request event + result event (`state-v2.md` §1.2: ~1,100 awaits).

**What it demands from the team.** Writing every async step as two events
and an effect, which is mechanical and suits agents, but doubles the event
enum and needs a convention for ids on every effect.

**Codex cases.** (a, b, e, f, h) as family 1: this family says where logic
lives, not how views follow it, and on rtwk its views are family 1's.
(c) as family 1. (d) as family 1, with the dialog order an explicit
reducer rule. (g) best of all: the reply is an event carrying the effect id;
the test is pure.

**Verdict: partial — merge its effect boundary into family 1.** As a
separate architecture with a runtime crate it adds churn and indirection
for no view benefit. As a signature, `apply(&mut Model, Ev) -> Effects`
executed by a small shell, it buys K3 and K4 testability at the cost of one
enum per feature.

## 5. Family 3: fine-grained signals with automatic dependency tracking

**Why it exists.** "Fine-grained reactivity is an approach to modeling the
flow of data through an interactive application by composing together
three categories of reactive primitives: signals, computations, effects"
(reactive_graph src/lib.rs:1-9). It was built to update the exact DOM node
that depends on a value without a virtual-DOM diff (Leptos, Dioxus,
Sycamore). Dioxus signals add `Copy` handles: "makes it easy to move any
data into futures or children" (dioxus packages/signals/README.md:3-7).
futures-signals expose signals as `futures` streams.

**Mechanism.** Reading a signal inside a computation records a dependency;
writing it marks dependents dirty; memos recompute lazily and stop
propagation on equal values. rtwk follows a projection through
`rtwk_state::reactive::Derived` and `Select`.

**What it solves well.**
- K2 automatically: no dependency list to write.
- Only subscribed widgets rebuild; the root view does not run (A-sig builds
  the same widgets as E, `state-survey.md` §5).

**What it brings.**
- **Silent untracked reads**: `*_untracked`, a value captured at creation,
  or a plain cache leaves a panel stale until another input moves
  (`forgotten_dependency_a`, `app-state-aopt.md` §4).
- **Cross-thread reads panic**: 8 of 8 trials with tokio writers; 12,524 of
  153,618 reads (8%) in a stand-alone stress (`app-state.md` A1). The fix is
  UI-thread confinement, which removes the reason for `Send + Sync`.
- **`batch` lost a notification** through `Derived` (A2, not understood);
  keyed `Vec` push is O(n) (A3); write before first keyed read breaks reads
  (A4); version pin `=0.2.16` with 0.3.0-rc1 pending (A5).
- **Six rules that fail silently** (keep `Source` handles and `Select`
  widgets across builds, `unchanged()` in every handler, no `batch`, ...;
  `app-state-aopt.md` §4).
- **futures-signals are lossy**: "*all* other `SignalExt` methods are
  *lossy*: they might skip changes" (futures-signals src/lib.rs:275-281);
  polling inside `wake` deadlocked (B1); last release 2024-07-26.
- **dioxus-signals need the Dioxus runtime** (`state-survey.md` §2).
- Cost: A-sig 15.6 allocations per delta (2.1x), A-opt 41.8 (5.7x).

**What it demands from the team.** Knowing which reads track and which do
not, keeping handles across builds, never reading off-thread. An agent's
mistake is not a compile error and often not a test failure.

**Codex cases.** (a) per-delta signal write; acceptable at A-sig cost,
not at reactive_stores' (A3 at 10,000 entries: 7.8 ms per commit, debug).
(b) as (a), on the UI thread only. (c) no replay if every thread's signals
are kept. (d) the dialog stack is a signal of a `Vec`; ordering is the
writer's. (e) automatic, best of all families, until an untracked read.
(f) one settings signal read everywhere: every dependent re-evaluates;
fine. (g) ids by hand, as everywhere. (h) memos per thread and per inbox;
works, with the silent-read risk multiplied by N panels.

**Verdict: does not fit as the core.** It solves propagation, which rtwk
already does, at 2x cost with silent failure modes; A-sig is the only
acceptable form if automatic tracking is wanted.

## 6. Family 4: observable entities or collections, explicit notify or diffs

**Why it exists.** Zed: "every model or view in the application is
actually owned by a single top-level object called the `App`" with entity
handles (gpui src/_ownership_and_data_flow.rs:1-20); a view re-renders when
an entity it read calls `cx.notify()`. eyeball: "Add observability to your
Rust types", with `ObservableVector` broadcasting `VectorDiff` batches
(eyeball README) so foreign UIs (matrix-sdk FFI apps) can apply diffs.
rtwk's own `Obs<T>`, `Select`, `store_provider` are this family's
substrate (docs/guide/state.md:7-40).

**Mechanism.** State lives in entities with an observer list; the writer
mutates and notifies; observers (views or derived entities) re-read.
Collections emit diffs instead of whole values. Projections filter by
`PartialEq` at the source.

**What it solves well.**
- Large apps with many independent panels: only views that read a changed
  entity rebuild (Zed `cached` views, view.rs:544-559).
- Cost equal to family 1 when written by hand (E: 8.1 allocations per delta,
  2.5 ms; `state-survey.md` §5).
- Diffs suit a transcript consumed by several front-ends (matrix-sdk
  timeline, `state-survey.md` §3.6).

**What it brings.**
- **A forgotten notify is a stale view**: 44 merged Zed PRs, e.g. #47600
  "agent_ui: Add missing `cx.notify()` calls for real-time UI updates",
  #60552 "MCP server toggle not updating in UI" (`state-survey.md` §3.1).
- **Derived state kept by observers**: Zed's `sync_entry`,
  `sync_generating_indicator` (conversation_view.rs:2010-2026) — the hand
  sync that K2 forbids. In E, a new input without an observer is stale
  (`state-survey.md` §4).
- Observer order becomes part of behaviour (E's race tests add it).
- eyeball: every push clones into a broadcast message; a lagging subscriber
  gets `Reset{values}` (eyeball-im subscriber.rs:127, 215) — it duplicates
  rtwk's keyed rebuild.

**What it demands from the team.** A notify on every write path (best
placed inside the entity's own `update`, never separate), observers for
every derived value, `Select` handles kept across builds. Errors are silent.

**Codex cases.** (a) one entity per thread, notify per frame-coalesced batch;
only the transcript panel rebuilds. (b) background entities notify the
sidebar's summary projection only. (c) no replay, as family 1. (d) a dialog
queue entity; fine. (e) a `Sums` entity observing all threads; correct only
while every new field gets an observer. (f) a settings entity provided at
the root. (g) ids by hand; Zed's `_task` replacement cancels the old.
(h) good fit structurally (entities per thread), with the notify risk per
thread kind.

**Verdict: partial — the reserve.** Same cost as family 1 and better
isolation, but its failure is the one bug class K1/K2 exist to remove.
Use it only for a panel a profile shows must rebuild without the root view,
with `notify` inside the entity's `update`.

## 7. Family 5: actors per session or thread

**Why it exists.** Isolate concurrent, failure-prone services; each owns
its state and processes messages one at a time. kameo: "Fault-tolerant
Async Actors Built on Tokio"; ractor, actix: actor frameworks (crates.io
descriptions). kameo handlers are "processed sequentially one at a time"
(`state-v2.md` §4 notes). The oracle's `App::run` is already a hand-rolled
tokio actor: one task, an mpsc of `AppEvent`, `select!` (`state-v2.md` §4).

**Mechanism.** One actor per thread or service with a mailbox; replies by
message or future; the UI holds handles and receives copies.

**What it solves well.** Isolation of services that block or crash (the
app-server session, MCP, file watchers); supervision; natural per-thread
serialisation.

**What it brings.**
- The UI cannot read actor state; it needs published snapshots (`watch`,
  `arc-swap`) and every handler forces `Send` (`state-v2.md` §1, runner-up D).
- Ordering across actors is not defined; a cross-thread derived value (e)
  needs another actor or a gather step.
- Awaiting inside a handler blocks the mailbox (relm4 book on
  `AsyncComponent`, `state-v2.md` §4 notes): Codex's current semantics,
  and the source of its races (`bug-classes.md` row 13: 320 async fns, 1,111
  awaits in app/).
- `off-thread-state.md` found no off-thread reader that needs it
  (`state-survey.md` §2).

**What it demands from the team.** Message protocols per actor, snapshot
types per view, `Send` everywhere, async tests with fakes.

**Codex cases.** (a) fine in the actor; the UI gets a snapshot per frame.
(b) natural. (c) switch asks the actor for a snapshot: latency and a race
with in-flight deltas. (d) approvals from several actors meet in the UI:
order across mailboxes is undefined. (e) gather step needed. (f) broadcast
to all actors. (g) the actor drops by id; the UI may still show the old
snapshot. (h) fits the ownership, not the derived Activity counts.

**Verdict: does not fit for UI state.** Right for services, which
app-server already is.

## 8. Family 6: ECS as a state store (bevy_ecs)

**Why it exists.** "Entities are unique things that are assigned groups of
Components, which are then processed using Systems"; table storage is "fast
and cache friendly iteration" (bevy_ecs README:17, 231). Built for game
loops over many homogeneous entities per tick.

**Mechanism.** A `World` of entities and components; systems run in a
schedule; `Changed<T>` ticks detect changes; messages are double-buffered.

**What it solves well.** Throughput over 10^4-10^6 entities; change
detection per component.

**What it brings.** Schedule order instead of event order; unread messages
"will be dropped silently" after two updates (bevy_ecs message docs,
`state-survey.md` §2) — not lossless, against Codex's ordered lossless
ingress; queries instead of plain field access.

**What it demands from the team.** Learning the ECS idiom for what is a
small number of heterogeneous objects (threads, dialogs, settings).

**Codex cases.** (a, b) fine as throughput; (d, g) ordering is the
schedule's, not the arrival's; (c, e, f, h) possible but every derived value
is a system and its run order matters.

**Verdict: does not fit.** Wrong scale and wrong ordering model.

## 9. Families 7 and 8, and the complements

### Incremental computation (salsa)

**Why it exists.** "The goal of Salsa is to support efficient incremental
recomputation. Salsa is used in rust-analyzer" (salsa book/src/overview.md).
**Mechanism.** Inputs with revisions; memoised queries pulled on demand;
backdating stops recomputation when a result is equal. **Solves:** heavy
derived results after small edits. **Brings:** no push, so a view cannot
follow it; setting an input needs `&mut db` and cancels in-flight queries
by panicking ("Cancellation is implemented via panicking", book/src/
tuning.md:62-66). **Codex:** (a) a new revision per delta, pointless; (e)
correct but pull-only. **Verdict: does not fit as the core;** a candidate
for one heavy derived query later (a phase-2 diff/hunk index), as would be
comemo.

### Streams and Rx pipelines (rxRust, futures streams)

**Why it exists.** Compose asynchronous sources with operators over time;
`switch_map` drops stale inner results. **Mechanism.** Observables or
`Stream`s chained through operators, driven by a scheduler. **Solves:** (g)
elegantly; ingress (Codex's app-server notifications already arrive as an
ordered stream, `state-survey.md` §3.7). **Brings:** a second scheduler
beside rtwk's; state hidden inside operator closures (not inspectable,
hard to test); combining several sources can emit intermediate inconsistent
combinations; rxRust is small (14k downloads in 90 days). **Verdict: does
not fit as a store;** streams stay what they are today, the ingress.

### Complements

- **State machines (`statig` or plain enums)** for flows with phases:
  approval, onboarding, login. They live inside the model in any family;
  (d) becomes a transition table.
- **`tokio::sync::watch`** for single latest values fed off-thread (config
  file, connection state); `rtwk_tokio::follow` bridges it
  (`app-state.md` §1).

## 10. Matrix and the choice

Scores: ++ strong, + adequate, ~ conditional, - weak, -- fails. Evidence in
the sections above.

| | K1 views from state | K2 derived computed | K3 ids, stale dropped | K4 races tested | Cost (measured) | 30+ screens | Testability | Rust standard | rtwk fit |
|---|---|---|---|---|---|---|---|---|---|
| 1 Plain model + memo | ++ | ++ (compile-checked in memo) | + (in apply) | ++ (pure apply) | ++ 7.3 allocs/delta | + (grows a view run; §11, §12) | ++ | ++ Helix, gitui, yazi, iced | ++ native |
| 2 Reducers + effects as data | ++ (views are 1's) | ++ | ++ (replies are events) | ++ | + (one more enum hop) | + (boilerplate) | ++ | ~ documented, not used by the large apps read | + (as a signature) |
| 3 Signals | ++ | ~ (silent untracked read) | + | ~ (UI-thread only) | - 2.1x to 5.7x | ~ (6 silent rules) | + | + Leptos ecosystem | ~ via `Derived` |
| 4 Entities + notify | ~ (forgotten notify) | - (observers sync) | + | + (plus observer order) | ++ 8.1 | + | + | + Zed | + `Source`/`Select` |
| 5 Actors | - (snapshots) | - | + | - (cross-mailbox order) | ~ | - | - async fakes | + services | - |
| 6 ECS | ~ | + (change ticks) | -- (messages dropped) | - (schedule order) | ~ | - | + | + games | - |
| 7 salsa | -- (no push) | ++ | n/a | + | ~ | - | + | + compilers | - |
| 8 Streams / Rx | ~ | ~ | ++ (`switch_map`) | - (scheduler) | ~ | - | - | ~ | - |

**The choice: family 1, with family 2's effect boundary; family 4 as the
reserve.** Three reasons.

1. **It removes the bug class that matters by construction, with the
   narrowest failure mode.** State desync and races are about 22 points of
   the oracle's fix history (`bug-classes.md` §1). With the view a function
   of the model, "forgot to clear the row" cannot happen. The only staleness
   left is a wrong memo key, local to one `memo` call; a capture is a
   compile error. Family 4's forgotten notify (Zed's 44 PRs) and family 3's
   untracked read are both silent and both global.
2. **It makes K3 and K4 the cheapest to meet.** One writer, events in order
   through one channel, `apply` a plain function returning effects as data:
   a race test is a list of events with expected model and effects, no
   runtime (`app-state-aopt.md` §4, `state-v2.md` §1). Families 3, 5, 6, 8
   reach determinism only by confining themselves to the same shape.
3. **It is rtwk's own model, the cheapest measured, and what the large Rust
   TUIs do.** `AppView` + `memo` + channel needs no adapter, no `Send`, no
   pinned dependency; 7.3 allocations per delta against 15.6 to 41.8 for
   signal libraries (`state-survey.md` §5); Helix, gitui, yazi and iced keep
   a plain owner (`state-survey.md` §3). The view-run cost at Codex's size
   is measured below, not assumed.

## 11. New measurement: how the whole-view rerun scales

**Scenario** (`src/scale.rs` in the prototype copy). Per frame: 17 deltas
into a live tail (1,000 deltas/s at 60 fps) and one change to one thread
(tokens, unread or running). The view computes P panel values, S of them by
an O(T) scan over all threads (S = 0: counters maintained in `apply`),
memoises P panels, R thread rows and the live tail. 300 frames, release,
7 runs, p50; rtwk-test fake clock, fullscreen 120x60. Load average about
21 on 10 cores during the runs, so absolute times are inflated; ratios
hold.

| T threads | R memo rows | P panels | S scans | frame µs | view fn µs per run |
|---|---|---|---|---|---|
| 40 | 40 | 10 | 4 | 37.3 | 6.6 |
| 200 | 50 | 30 | 10 | 47.7 | 12.1 |
| 1,000 | 50 | 30 | 10 | 54.2 | 18.6 |
| 1,000 | 50 | 30 | 0 | 44.5 | 9.5 |
| 5,000 | 50 | 30 | 30 | 186.3 | 149.4 |
| 1,000 | 300 | 30 | 10 | 166.5 | 97.1 |
| 1,000 | 1,000 | 30 | 10 | 763.9 | 648.5 |
| 1,000 | 50 | 300 | 0 | 516.8 | 295.6 |

Views per frame: 1.00 in every row (rtwk coalesces the 18 events).

**What it shows.**
- At realistic size (hundreds of threads, a virtualised sidebar of about
  50 visible rows, 30 memoised panels, 10 O(T) derived values) the view
  costs 12-19 µs per frame: about 0.1% of a core at 60 fps.
- O(T) derived scans cost little: 10 scans over 1,000 threads add 9 µs;
  30 scans over 5,000 threads add about 140 µs. Maintained counters are not
  needed below several thousand threads.
- **The growth law is the number of memo calls, quadratically**: 81 memo
  calls 9.5 µs, 331 calls 97 µs, 1,031 calls 648 µs, 351 calls (300 panels)
  296 µs. rtwk's memo cache is a `Vec` searched linearly per call
  (app.rs:812-819), so k calls cost about k²/2 comparisons. Even the worst
  row (1,000 rows unvirtualised) is about 4% of one core at 60 fps, but it
  grows with every memo a screen adds.

**Reproduce:** `cd $TMPDIR/codexcli-research/state-approaches/proto &&
CARGO_TARGET_DIR=~/.cache/codexcli/research/app-state-proto/target-scale
~/.cache/rtwk/acargo build -q --release`, then
`.../target-scale/release/proto scale T R P S 7`.

## 12. The chosen approach's own weaknesses, and their mitigations

1. **The root view reruns per frame with a change, and its cost grows with
   memo calls (k²).** Mitigations: (i) lists that can be long (sidebar,
   transcript in pane mode, agents center) are `ListView::builder`, which
   builds only visible items (rtwk-scroll list_view.rs:310); since the item
   builder is `'static`, the view hands it an `Rc`/`Arc` snapshot of the
   rows, and the `ListView` itself sits under one memo keyed on the
   slice's revision; (ii) only mounted screens are in the view: 30 screens
   do not mean 30 screens' memos per frame; (iii) a budget test per release:
   the `scale` scenario at the target size must stay under a fixed µs per
   frame; (iv) a request to rtwk to key the memo cache (by position or
   hash) would turn k² into k — rtwk's call, not done here.
2. **O(n) derived values could tempt hand-maintained counters (against
   K2).** Mitigation: compute them in `view` (measured cheap, §11). Where a
   profile demands incremental maintenance, it lives in one type whose only
   mutators update the aggregate (a collection wrapper with private
   fields), with a debug assertion or property test that recomputes from
   scratch and compares. No handler maintains a counter.
3. **A forgotten revision bump means a stale kept subtree.** Mitigation:
   memo keys are values where cheap (`Arc` clones, small tuples); where a
   revision is used, it is bumped by the slice's own accessor (a
   `Versioned<T>` whose `get_mut` bumps), so a write without a bump does not
   compile outside the module; a screen test asserts the rendered screen
   after each event (the harness pattern of rounds 1 and 3).
4. **`cx.unchanged()` can hide a rebuild.** Mitigation: banned in feature
   code except for handlers that touch only a controller (rtwk's documented
   use, app.rs:967-991); a grep in CI.
5. **Nested memos release inner entries when the outer one hits** (by
   reading, §3). Mitigation: memoise at one level per region (a panel, or
   its rows, not both) until measured otherwise; report it to rtwk as a
   question.
6. **One large `apply`.** Mitigation: one sub-model and `apply` per feature,
   keyed by thread; the parent routes by event kind (cosmic-files' 7k-line
   `update` is the counter-example, `state-survey.md` §3.5).
7. **Only the UI thread reads the model.** Mitigation: off-thread consumers
   (OSC notifications, a future daemon mirror) are fed by effects from
   `apply` or by a published `watch` snapshot of the one value they need.
8. **Memory for keeping every thread current** (replaces the oracle's
   256 KiB-capped replay buffer, thread_event_buffer.rs:8-11, per `state-survey.md` §3.7): settled
   transcript items as `Arc`, a cap on kept history per inactive thread
   with backfill from app-server on reopen. Parity note: the 1:1 port must
   decide whether replay semantics are observable behaviour to keep.

**What would make us switch to the reserve (family 4) for a panel.** A
phase-2 profile at the target size where the root view exceeds about 1 ms
per frame after mitigations 1 (i)-(ii), or where a panel must update while
the root view is deliberately not rerun (for example a live tail updating
at a higher rate than the rest). The switch is local: same channel, same
model types, `notify` inside the entity's `update`.

**What would make us reconsider family 3.** Evidence that hand-written memo
keys cause stale-view bugs at a rate comparable to Zed's notify history
across 30 screens, and a reactive_graph release that fixes A2 (lost batch
notification) and A4 with a measured cost under 1.5x.

## 13. For the person

1. ADR-003: family 1 with family 2's effect boundary
   (`apply -> Effects`), family 4 as the reserve. Or family 1 without the
   effect boundary (handlers call `AppCx::spawn` directly; simpler, race
   tests then need rtwk's fake clock).
2. Whether to raise the memo-cache k² growth and the nested-memo release
   with rtwk.
3. Whether the replay-on-switch behaviour of the oracle is part of the 1:1
   parity, or replaced by keeping every thread's model current.
4. Flows: `statig` or plain enums inside the model.
