# Application state and reactivity for the Codex CLI on rtwk (research dated 2026-11-17)

Scope: where the app's domain state lives, how it changes, how changes
propagate, how tokio I/O feeds it, how rtwk widgets follow it through
`rtwk-state`. Not widget state, not TUI patterns. Prototype (outside any
checkout): `$TMPDIR/codexcli-research/app-state/proto` (`cargo test`, `cargo run
--release -- 15`, `cargo run --features count -- 1`, `-- micro`).

## 1. Recommendation

**Phase 1: C, a plain Rust model owned by the rtwk app, one writer (the UI
thread), events in through an rtwk channel, unchanged subtrees kept with
`ViewCx::memo` on revision counters.** It is the only candidate that passed
every measurement without a caveat: lowest CPU (about 8x less than A per
delta), the lowest allocation counts, no `Send + Sync` bounds, no cross-thread
failure mode, and a model you test with `Model::apply` and no UI.

Its price is manual dependency tracking: a forgotten revision bump is a stale
screen, and derived values (status line, per-thread flags) are recomputed in
the view function each frame. That is cheap now (a loop over the threads) and
is the part to revisit when phase 2 adds panels.

**Do not take A (reactive_graph + reactive_stores) as the cross-thread store.**
Read access to it panics under contention (section 5). If derived state later
outgrows C, A is the upgrade path **on one thread only** (events through the
same channel, all reads and writes on the UI thread). That mode removes the
panics; I measured its cost, not its behaviour under phase-2 load.

**Do not take B (futures-signals).** It needs a driver for every subscription,
delivers asynchronously (tests need a flush hook), has no batching, and the
crate's last release is 2024-07-26.

**No D.** Dioxus signals depend on `dioxus-core`'s runtime (not standalone);
salsa is a pull-based incremental compiler database with no subscriptions
(section 2). `tokio::sync::watch` already has an rtwk bridge (`rtwk_tokio::follow`)
and suits single values (config, connection state), not the transcript.

What the prototype does not cover: scrolling or virtualisation (the transcript
shows the last 8 entries; 1,000 are in state, not rendered), real tokio
streaming under `rtwk-tokio`, persistence, multi-process.

## 2. Desk research (crates.io API and GitHub, fetched 2026-11-17)

| Crate | Latest (date) | Downloads total / 90 days | Maintainers | Notes |
|---|---|---|---|---|
| reactive_graph | 0.3.0-rc1 (2026-10-31); stable 0.2.16 (2026-10-14) | 4.09M / 1.83M | benwis, gbj (Leptos) | rtwk-state pins `=0.2.16`; 0.3 is in RC, so the pin must move |
| reactive_stores | 0.5.0-rc1 (2026-10-31); stable 0.4.4 (2026-09-26) | 4.01M / 1.80M | benwis, gbj | 0.4.4 needs reactive_graph `^0.2.15`; Leptos repo 21.5k stars, pushed 2026-11-12 |
| futures-signals | 0.3.34 (2024-07-26) | 1.10M / 98k | Pauan (one) | repo 792 stars, last push 2026-04-07, no release in 16 months |
| dioxus-signals | 0.8.0-alpha.3 (2026-10-16) | 4.05M / 1.58M | Dioxus Labs | depends on `dioxus-core`, `generational-box`: needs the Dioxus runtime context, not usable standalone |
| salsa | 0.28.5 (2026-09-24) | 10.8M / 3.39M | salsa-rs | pull-based memoised queries over inputs, built for compilers; no change subscription, so nothing to feed a widget; an input write invalidates queries and cancels in-flight ones |

API fit (my reading of the sources, plus what the prototype proved):

| Need | A reactive_graph + stores | B futures-signals | C plain model |
|---|---|---|---|
| Nested / keyed collections | `#[derive(Store)]`, `#[store(key)]` keyed Vec, `at_key`, `at_unkeyed`; keyed Vec costs O(n) per push (section 5) | `MutableVec<Arc<Cell>>`, `SignalVec` diffs (rtwk `Select` reads snapshots, so diffs are unused) | `Vec<Th>`, `Vec<Str>` |
| Batch several writes into one notification | `effect::batch` exists but lost the notification in my repro | none | one `apply` per event; one flush per frame (rtwk coalesces) |
| Derived values | `ArcMemo` (shared, deduped, lazy); `rtwk_state::reactive::Derived` | `map_ref!`, `map_signal`, `to_signal_cloned`, `dedupe_cloned`; one instance per consumer | recompute in the view, memo on the result |
| tokio integration | writer calls from tasks (unsafe, section 5) | signals are polled by an executor; wake happens under the writer's lock | `AppCx::channel` Sender, coalesced per flush; rtwk-tokio `subscribe` for streams |
| Send/Sync | all values `Send + Sync`; `Arc<str>`, not `Rc` | `Send` for polled signals | none |
| Cross-thread writes | panics under contention | works (8 of 8 trials clean, 1 and 2 writers) | via Sender, works |
| Testable without UI | yes (store plus memos) | yes, but needs the driver flushed | yes (`Model::apply`) |

## 3. Prototype

Same scenario for A, B, C, same UI (identical screens at every checkpoint,
asserted by one test): two threads (1,000 and 50 seeded entries); thread 0
streams 1,000 deltas/s for 2 s at 1 ms steps, one commit per 10 deltas (200
commits, 1,200 entries at the end); approval at 800 ms shown in a dialog and
answered with `y` at 1,100 ms; thread 1 notified at 500 and 1,500 ms while
unfocused; derived status line (`run 1 | input 0 | tok 2400`), tab line with
per-thread flags, a third panel (`needs input: T0`). Run in `rtwk-test`'s
`Tester` (fake clock, 60 fps, 122 frames). Widgets follow state with
`rtwk_state::Select` (A, B) or `ViewCx::memo` (C, rtwk-weather's manner).

Behavioural tests (all pass for A, B, C): screens equal at start, after
notification, with the approval shown, after the answer, at the end (tokens
2,400, unread 2, window 1192..1199), after switching focus (unread cleared);
rebuild bounds (below); writes from two tokio tasks and from one.

## 4. Measurements

Widget builds (calls of a build closure; identical for A, B, C):

| | per run | per delta | per frame |
|---|---|---|---|
| committed entry builds | 200 | 0.10 | 1.64 |
| live entry builds | 123 | 0.06 | 1.01 |
| status line builds | 121 | | 1.0 (tokens change per commit) |
| tabs / dialog / panel | 4 / 2 / 2 | | |

Only the live entry rebuilds per delta; a committed entry builds once, when it
first shows. Caveat found on the way: with `Select`, a parent that rebuilds
creates new `Select` widgets and every window entry rebuilds (968 entry
builds instead of 200). The parent must keep the entry widgets and reuse the
same `AnyWidget` (rtwk skips an identical widget). C gets this from `memo`.

Allocations (debug, counting allocator, whole process; the flush includes
rtwk's own layout and paint, the same for all three):

| | write allocs / delta | write bytes / delta | flush allocs / frame | flush bytes / frame |
|---|---|---|---|---|
| A | 160 | 9,462 | 174 | 15,041 |
| B | 2.3 (driver thread included) | 95 | 106 | 8,432 |
| C | 1.1 (channel send) | 28 | 102 | 8,563 |

State-layer work: A ran 7,828 derived evaluations (3.9 per delta: live 4,400,
entry 1,800, len 400, summaries 408, tabs/panel/status 272 each). A push
onto the Vec re-evaluates every subscribed entry memo (9 per commit); the memos
dedupe, so no widget rebuilds. B ran 1,186 evaluations. C ran its view
function 123 times (memo hits 1,158, misses 467).

Wall time (release, 15 interleaved runs of A, B, C; **load average 43 to 48
on 10 cores, so absolute numbers are inflated and noisy; ratios and p95 are
indicative only**):

| | writer side, us / delta | UI-thread pump, us / frame p50 (p95) | whole scenario p50 ms |
|---|---|---|---|
| A | 24.5 | 53 (137) | 58.6 |
| B | 2.7, plus 19.6 handoff wait | 63 (168) | 64.9 (includes the wait) |
| C | 0.1 (the apply runs in the flush, inside the pump) | 42 (70) | 7.4 |

The handoff wait is an artefact: the test waits for B's driver thread to
drain before pumping so the fake-clock run is deterministic; a live app does
not wait. At 1,000 deltas/s per streaming thread A costs about 2.5 percent of
one core on the writer side, B and C under 0.3 percent. Performance does not
decide this; failure modes do.

Code shape (non-blank lines, rustfmt-like, my files):

| | state layer | notes |
|---|---|---|
| A | 189 (`a.rs`) | store structs, `apply`, 8 derived sources; plus `ui.rs` 80 shared with B |
| B | 287 (`b.rs`), of which 129 are a reusable `Source`-over-signal adapter with a driver thread | adapter would be about 40 lines on a tokio runtime, but then delivery depends on the runtime's scheduling |
| C | 60 for the model and `apply`; the view with memos is another 60 | no adapter, no sources |

Adding the third panel (a derived `needs input: T0`): A one field, one derived
closure, one trait method, one widget (about 14 lines). B the same, with a
signal chain instead of a memo (about 12). C one `panel_line(&sums)` call
under one `memo` and a 4-line build function (about 6). None touches the
event path.

## 5. Findings that decide it (each reproduced in the prototype)

**A1. Cross-thread reads panic.** `ArcStore` and `ArcRwSignal` reads use a
non-blocking `try_read` (`Plain::try_new`, guardian `try_take`); a read that
meets a writer holding the lock answers `None` and `.get()`/`.with()` panics
with "already been disposed". Prototype, store plus tokio writers feeding the
UI: **8 of 8 trials failed with one writer task and 8 of 8 with two**
(panic on the UI thread and on the writer). Stand-alone: one `ArcRwSignal` plus
`Derived`, one hot writer thread, UI thread reading in a loop: **12,524 of
153,618 reads panicked (8 percent)**. The whole `ArcStore` is one lock, so
two writers on different threads also collide with each other.
`rtwk-state`'s docs say an app can write from another thread; with a hot
writer that does not hold. Fix by design: confine reads and writes to one
thread (then A needs no `Send + Sync` and the cost buys little).

**A2. `effect::batch` lost a notification.** With `Derived` (ArcMemo plus
`ImmediateEffect::new_isomorphic`) subscribed, `batch(|| { a.set(2); b.set(2) })`
produced no callback and the next plain write worked; same for a batch of one
write. Repro: test `batch_repro` (`proto/src/tests.rs`). Without batch, each
signal write notifies (rtwk coalesces per flush, so only writer-side work
grows). Not understood; it needs an upstream look before anyone relies on it.

**A3. Keyed Vec push is O(n).** Each commit through `#[store(key)]` rebuilds
the key map: per commit 239 / 1,139 / 10,139 allocations and 140 / 804 /
7,798 us (debug, store alone) for 100 / 1,000 / 10,000 entries. The same
push on an unkeyed Vec (`at_unkeyed(i)`) is 97 allocations and 37 to 80 us flat.
The prototype uses unkeyed for the transcript and keyed for threads.

**A4. Write before first read of a keyed field breaks reads.** A store with a
keyed field written before any read answered `None` for `at_key(k)` reads (same
"disposed" panic); calling `update_keys()` at construction fixes it. The app
must register keys after seeding and after any structural change it does
outside the store's own writers.

**A5. Version coupling.** `rtwk-state` pins `reactive_graph =0.2.16`;
upstream's next line is 0.3.0-rc1 (2026-10-31) with `reactive_stores` 0.5. Any
store derive written now moves with that line.

**B1. Polling in `wake` deadlocks.** futures-signals calls the waker while the
writer still holds the `Mutable`'s lock; an adapter that polls inside `wake`
(the obvious `Source` implementation) hung the test run (`sample` showed
`MutableLockState::notify` waiting). Working design: wakes go to a driver
thread (or tokio task) that polls. Consequences: delivery is asynchronous,
every test needs a flush hook (`quiesce()`), and a subscriber costs a queue
push per write.

**B2. Maintenance.** One maintainer, last release 2024-07-26, 98k downloads in
90 days against 1.8M for reactive_graph. No batching; a commit is three
writes and three wakes (coalesced by `Select`'s pending flag).

**C1. What C costs.** The view function runs per change batch (123 times
here, each building 8 memo keys); entry memo deps clone an `Arc<str>`; the
memo cache is a linear search (fine at about 15 live entries, not at 1,000).
Dependencies are written by hand (`live_rev`, the `Summary` values); the
failure is a stale widget, not a crash. Derived values are recomputed
whether or not a panel is shown.

## 6. Excerpts (state layer and a subscribing widget; 52 lines)

A, store and writes (`a.rs`):

```rust
#[derive(Store, Clone)] pub struct Th { id: usize, running: bool, unread: u32, tokens: u64,
    approval: Option<Approval>, live: String, entries: Vec<Entry> } // unkeyed Vec: see A3
fn apply(s: &ArcStore<S>, ev: Ev) { match ev {
    Ev::Delta(t) => th!(s, t).live().update(|l| l.push_str(TOK)),
    Ev::Commit(t) => { let th = th!(s, t);
        let text = th.clone().live().with_untracked(|l| Str::from(l.as_str()));
        let id = th.clone().entries().read_untracked().len();
        th.clone().entries().write().push(Entry { id, text });
        th.clone().live().write().clear();
        th.clone().tokens().update(|x| *x += COMMIT_TOKENS); } // no batch: see A2
```

C, model and view (`c.rs`):

```rust
Ev::Delta(t) => { self.th[t].live.push_str(TOK); self.th[t].live_rev += 1; }
pub fn start(c: &mut C, cx: &mut AppCx<'_, '_, C>) {
    c.tx = Some(cx.channel(|c: &mut C, ev: Ev, _| c.m.apply(ev))); } // any thread may send
// in view(): the live entry rebuilds when its revision moves; entries once
.child(cx.memo((th.live_rev, Str::from(th.live.as_str())), live_w))
list = list.child(cx.memo((i, th.entries[i].clone()), entry_w));
```

A and B, a widget following a projection (`ui.rs`, shared; the entry widget
is kept across builds, see section 4):

```rust
let e = Select::new(bb.entry(t, i), move |s: &Str| {
    bump(k::ENTRY); Text::new(format!("{i}: {s}")).into() });
AnyWidget::from(e).with_key(Key::value(i))
```

B, the adapter (`b.rs`; wakes are queued to a driver, never polled in `wake`):

```rust
fn wake(self: Arc<Self>) { if !self.queued.swap(true, Relaxed) { driver().push(self) } }
// Sub::poll(): drain poll_change under its own Mutex; first value is not a change;
// later values call `changed(v)`, which sends one `()` into the Select.
```

## 7. Risks

- C: stale UI from a missed revision bump; view cost grows with panels (each
  panel adds a derivation and memo call per frame); memo's linear cache.
  Mitigation: a test per feature that asserts the screen after each event
  (the harness here does that), and a rule that derived values are plain
  functions of the model.
- C: nothing outside the UI thread can read the model. Notifications to
  other consumers (OSC 9, daemon) must be emitted by `apply` as effects.
- A on one thread: the A1 to A5 findings still apply (A2, A3, A4); the pin
  moves with Leptos's 0.3 line. `batch` cannot be used until understood.
- B: unmaintained upstream; async delivery makes every behavioural test
  timing-aware.
- Measurement: load average 43 to 48; allocations are exact, times are not.
  The scenario has no tokio runtime under the fake clock, so tokio
  scheduling latency is untested beyond the cross-thread test.

## 8. Open questions for the person

1. Accept C's manual dependency tracking in phase 1, and revisit when a
   third derived panel exists (the section 4 numbers say the cost is small
   now)? Or take A on the UI thread from the start, accepting A2 to A5?
2. Must any non-UI thread read domain state (a notification bus, a daemon
   mirror)? If yes, C needs a published snapshot (`Arc` swap or `watch`), and
   A is out unless reads are marshalled too.
3. Is the A1 panic worth reporting to rtwk (docs say cross-thread writes work)
   and to Leptos (blocking read instead of `try_read`)? Not done here.
4. Is rtwk willing to unpin `reactive_graph` to follow 0.3, or should phase 1
   stay off `rtwk-state`'s `reactive-graph` feature entirely?
5. Should the transcript be virtualised in phase 1? The measurements assume
   an 8-entry window; a full-history view changes the memo cache (C1) and the
   keyed-versus-unkeyed choice (A3).
