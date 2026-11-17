# App state round 3: reactive_graph optimized (A-opt) against the plain model (C), cost and maintainability (research dated 2026-11-17)

Follows `app-state.md` (round 1). The person asked for A judged with
optimizations and on maintainability. Prototype:
`~/.cache/codexcli/research/app-state-proto` (rtwk `675a98d7`,
`reactive_graph =0.2.16`, `reactive_stores 0.4.4`, rustc 1.99.0, macOS arm64,
10 cores). Round 1's variants and harness are unchanged and still give the
same counts. New files: `src/a_opt.rs`, `src/a_sig.rs`, `src/dash_common.rs`,
`src/dash_c.rs`, `src/dash_a.rs`. Tests and modes are new too
(`same_behaviour_all_variants` now covers 5 variants, `dash_c_vs_aopt`,
`forgotten_dependency_a`, `dash_derivations_without_ui`,
`frame_phase_after_idle`, `main dash`).

## 1. Summary

- **Optimized, reactive_graph gets close to C on cost. reactive_stores stays
  well behind.** Total allocations per delta: C 7.3; A-sig (reactive_graph
  signals per field) 15.6; A-opt with a reactive_stores store 41.8 (28.0 with
  the three writes merged into one, which emulates a working `batch`); A
  naive 170.6. Whole scenario, release, p50 of 21 interleaved runs: C 2.5 ms,
  A-sig 3.7 ms, A-opt 8.9 ms (5.1 ms merged), A naive 26.3 ms. The gap goes
  from 23x allocations and 10x time down to 2.1x and 1.5x (A-sig), or 5.7x and
  3.6x (store).
- **Confining the store to the UI thread removes the cross-thread panic.**
  A-opt and A-sig pass 8 of 8 trials with one and with two tokio writers,
  against 8 of 8 trials failing for A naive.
- **Maintainability (a scenario shaped like phases 2-3: 40 threads, 4
  derived panels).**
  - Code size: C 69 lines of state, derivation and view; A-opt 95.
  - Adding one derived value and one panel: C 3 places / 10 lines, A-opt 5
    places / 5 lines. Neither touches the event path.
  - Forgotten dependencies: in C, the build of a kept subtree can only see its
    deps, so a capture does not compile (E0308, checked). In A, a forgotten
    dependency is an untracked read. It fails silently: the panel is stale
    until some other tracked input changes (test).
  - Cost: identical widget builds. C 14.5 us and 17 allocations per change,
    A-opt 22.5 us and 178.
- **Recommendation:** C stays the recommendation, now on maintainability
  rather than on cost. If the person wants automatic tracking, the A to take
  is A-sig: reactive_graph signals per field, everything on the UI thread,
  coalesced per frame, no reactive_stores on hot data. That costs about 2x C's
  allocations and adds 6 rules that fail silently (section 5). The person
  decides ADR-003.

## 2. What A-opt and A-sig do (both pass every behavioural test of round 1)

- **One thread.** The producer sends `Ev`s into `AppCx::channel`. The store is
  read and written only by the channel handler and by key handlers. Each
  handler calls `cx.unchanged()`, because the widgets follow the store and the
  root view is not rebuilt.
- **One store write per frame.** Deltas and commits go into a plain
  per-thread buffer. The first one in a flush sends a `Flush` item (`None`)
  into the same channel. rtwk handles it in the next round of the same flush
  (`READY_ROUNDS` = 64, `rtwk-framework/src/owner.rs:104`). That item writes
  `entries` (one `extend`), `tokens` and `live` once each. Rare events (turn
  start, approval, notify, focus) flush what is pending, then write directly.
- **Transcript.** The transcript is an unkeyed `Vec<Str>` field. A committed
  entry never changes, so its widget follows a constant `Source` and needs no
  per-entry memo. This avoids round 1's 9 entry-memo re-evaluations per
  commit and the O(n) keyed push.
- **Memos.** Memos are used only for derived values: summaries feeding
  status, tabs and panel; len; live; approval.
- **No `effect::batch`.** It still loses the notification through
  `rtwk_state::reactive::Derived` (round 1 A2). Coalescing makes it
  unnecessary on the hot path. `AOPT_ONE_WRITE=1` measures what a working
  batch would buy (section 6).
- **The two variants differ only in representation.** A-opt keeps
  `#[derive(Store)]` with keyed threads and `update_keys()` after seeding.
  A-sig uses one `ArcRwSignal` per field.
- **What C gets.** C already coalesces per frame: its handler mutates plain
  data (a `push_str` and a counter) and rtwk runs one view per flush
  (ADR-017). A separate "C + coalesce" variant would do the same work, so the
  table shows one C.

## 3. Measurements: the round 1 scenario, same harness, release

Allocations come from the `count` build and are whole-process. Flush counts
include rtwk's layout and paint. The total per delta is write + flush ×
frames / deltas (122 frames, 2,000 deltas). Times come from a build without
counting, as the p50 of 21 interleaved runs. **The load average was 19 to 34
on 10 cores, so the times are inflated; use them for ratios.**

| variant | write allocs/delta | flush allocs/frame | **total allocs/delta** | state-layer evals | UI-thread pump us/frame p50 | **scenario ms p50 (min)** |
|---|---|---|---|---|---|---|
| C | 1.1 | 102 | **7.3** | 123 views | 18.1 | **2.5 (2.4)** |
| A naive (round 1) | 160 | 174 | **170.6** | 7,828 | 31.3 (+11.1 us/delta on the writer) | **26.3 (20.9)** |
| A-opt, store, not coalesced | 1.1 | 2,342 | 142.8 | 6,228 | | 22.2 (17.5) |
| **A-opt, store, coalesced** | 1.1 | 673 | **41.8** | 1,462 | 70.0 | **8.9 (6.5)** |
| A-opt, store, coalesced, one write per frame | 1.1 | 444 | 28.0 | 1,220 | | 5.1 (5.0) |
| A-sig, signals, not coalesced | 1.1 | 594 | 37.0 | 3,622 | | 6.1 (5.9) |
| **A-sig, signals, coalesced** | 1.1 | 239 | **15.6** | 1,085 | 27.4 | **3.7 (3.5)** |
| B (round 1, for reference) | 2.3 | 106 | 8.8 | 1,186 | 61.3 | 49.3 (includes the test's handoff wait) |

**Rebuild isolation.** Widget builds are identical for every variant: status
121, live about 121 (one per frame), committed entries 200 (each built once,
when it first shows), tabs 4, dialog 2, panel 2. The bounds of
`same_behaviour_all_variants` hold for all five.

**Where A-opt's extra cost goes.**
- **reactive_stores overhead.** Each store write builds a `StorePath` and a
  `Vec` of triggers for the field and every ancestor
  (`reactive_stores-0.4.4/src/store_field.rs:69-95`). It also notifies the
  ancestors' `children` triggers. Because of that, `len`, `live` and
  `summaries` are each evaluated twice per frame in A-opt and once in A-sig
  (named evals: 246/250/254 against 123/125/127). Same coalescing, same UI:
  the store alone costs 434 allocations per frame (673 against 239).
- **Select-based UI.** A naive's flush (store written outside the flush) is
  174 allocations per frame against C's 102. About 70 allocations per frame
  belong to `Select` + `Derived` against `ViewCx::memo`, and every A variant
  pays them.

**Cost at real rates.** At 1,000 deltas/s per streaming thread, A-sig is
about 0.19 percent of one core, C 0.13, A-opt 0.45 (from the scenario times,
over 2 s). Cost alone no longer decides this, unless reactive_stores holds
the hot data.

**New finding (rtwk side).** A `Select` notified inside a flush sends into
its own cross-thread channel (`rtwk-state/src/lib.rs:86-98`). That send wakes
the frame loop for work the same flush has already consumed. The visible
effect: after a quiet period, A-opt and A-sig show the first delta 13 ms
later than A and C. Frames come at i = 13, 29, 46 instead of 0, 16, 33
(`frame_phase_after_idle`). This happens with or without the coalescing
self-send, so `Select`'s own send causes it. The cause is inferred, not
traced. It lands frames on a different 60 fps phase, so mid-stream
checkpoints of A-opt and A-sig are checked by their wanted texts, while the
settled screens (start, end, focus1) are compared equal to A. Fix options for
rtwk: when notified on the UI thread inside a flush, `Select` marks itself
dirty directly; or the executor does not ask for a frame for items a running
flush will take.

## 4. Maintainability: derived state over several panels (phases 2-3)

**The scenario** (`dash_common.rs`): 40 threads with title, running, unread
and tokens, plus a sidebar filter (text, running-only). The panels are:
- a thread list, filtered, one row per thread;
- a status bar: thread count, running count, total tokens;
- a sidebar: shown count and running count among the shown threads;
- an inbox panel (the extension): unread total and the number of threads with
  unread messages.

Eleven steps (single-thread changes, a change to a hidden thread, filter
changes, a burst of three) give identical screens in C and A-opt
(`dash_c_vs_aopt`).

| | C (`dash_c.rs`) | A-opt (`dash_a.rs`) |
|---|---|---|
| state + `apply` | 20 lines | 15 store/struct + 11 `apply` |
| derivation | 12 (pure fns) | 30 (memos, row cache) |
| view | 37 | 39 |
| **total** (no imports) | **69** | **95** (+14 import lines against 5) |
| add one derived value + one panel | 3 places in 1 file, 10 lines (6 with one shared text-widget fn): a function, a call, a `cx.memo` child | 5 places in 1 file, 5 lines: struct field, memo, `Derived`, struct literal, `Select` child |
| event path touched by the extension | no | no |
| forgotten dependency | **does not compile** inside a kept subtree: a `MemoFn` is a `fn` pointer, and a capture is E0308 "closures can only be coerced to `fn` types if they do not capture" (checked). A derived value computed by each view cannot be stale. Staleness needs a proxy key (a revision counter whose bump is forgotten) or a wrong `cx.unchanged()`, and then the view is stale and only a test sees it. | tracking is automatic, so forgetting is rarer. When it happens (`*_untracked`, a value captured at creation, a plain-data cache such as A-opt's coalescing buffer or its constant entry sources) it is **silent and intermittent**: the panel is stale until some other tracked input changes (`forgotten_dependency_a`). Only a test sees it. |
| testability without a UI | derivations are functions of a plain struct, and `apply` is a plain method (`dash_derivations_without_ui`) | memos run without owner or executor (same test). Notification behaviour (batch, immediate effects, frame phase) can only be tested in an rtwk integration test. |
| per change (600 changes, one frame each, p50 of 15) | 14.5 us, 17 allocations, 3 derivations (all of them, every view) | 22.5 us, 178 allocations, 7.4 memo evaluations (one per write; a burst of 3 = 24) |
| widget builds per change | row 0.70, status 0.37, sidebar 0.03, inbox 0.33 | identical |

**Excerpts (41 lines).** C, derivation and view (`dash_c.rs`):

```rust
pub fn visible(m: &Dash) -> Vec<&Thread> { m.th.iter().filter(|t| passes(&m.filter, t)).collect() }
pub fn totals(m: &Dash) -> (usize, u64) {
    (m.th.iter().filter(|t| t.running).count(), m.th.iter().map(|t| t.tokens).sum())
}
pub fn unread_total(m: &Dash) -> (u32, usize) { // EXT
    (m.th.iter().map(|t| t.unread).sum(), m.th.iter().filter(|t| t.unread > 0).count()) // EXT
} // EXT
pub fn view(m: &Dash, cx: &mut ViewCx<'_, '_, Dash>) -> AnyWidget {
    let vis = visible(m);
    let (running, tokens) = totals(m);
    let (unread, unread_threads) = unread_total(m); // EXT
    let mut list = col();
    for t in &vis { list = list.child(cx.memo((*t).clone(), row_w)); }
    let shown_running = vis.iter().filter(|t| t.running).count();
    col().child(cx.memo(status_text(m.th.len(), running, tokens), status_w))
        .child(cx.memo(sidebar_text(&m.filter, vis.len(), shown_running), sidebar_w))
        .child(cx.memo(inbox_text(unread, unread_threads), inbox_w)) // EXT
        .child(list).into()
}
```

A-opt, derivation and view (`dash_a.rs`):

```rust
let visible = memo(move || s1.clone().filter().with(|f| s1.clone().threads()
    .with(|v| v.iter().filter(|t| passes(f, t)).map(|t| t.id).collect::<Vec<_>>())));
let totals = memo(move || s2.clone().threads().with(|v|
    (v.len(), v.iter().filter(|t| t.running).count(), v.iter().map(|t| t.tokens).sum::<u64>())));
let unread = memo(move || s3.clone().threads().with(|v| (v.iter().map(|t| t.unread).sum::<u32>(),
    v.iter().filter(|t| t.unread > 0).count()))); // EXT
let status = Rc::new(Derived::from_memo(memo(move || totals.with(|&(n, r, tok)| Str::from(status_text(n, r, tok))))));
let sidebar = Rc::new(Derived::from_memo(memo(move || {
    let shown_running = vis.with(|ids| s4.clone().threads().with(|v| ids.iter().filter(|&&i| v[i].running).count()));
    s4.clone().filter().with(|f| Str::from(sidebar_text(f, vis.with(|v| v.len()), shown_running)))
})));
let inbox = Rc::new(Derived::from_memo(memo(move || unread.with(|&(u, n)| Str::from(inbox_text(u, n)))))); // EXT
// view: a Select per panel; the row Selects kept in a map so a list rebuild reuses them
    .child(text(&d.inbox, dk::INBOX)) // EXT
```

A-opt, per-frame coalescing (`a_opt.rs`, the channel handler and the flush):

```rust
Some(Ev::Delta(t)) | Some(Ev::Commit(t)) => {         // buffer, then one Flush item per flush
    /* push into pend[t].tail or move tail into pend[t].commits */
    if !self.scheduled.replace(true) { let _ = self.tx.borrow().as_ref().unwrap().send(None); }
}
Some(ev) => self.apply_now(ev),                       // rare events: flush, then write directly
None => { self.scheduled.set(false); self.flush(); }  // next round of the same flush
// flush: th.entries().write().extend(commits); th.tokens().update(..); th.live().set(tail)
```

**Rules A carries that C does not.** A program breaks each one silently
(stale or slow); none is a compile error.
1. Read and write the store only on the UI thread. Round 1 A1: a read under a
   writer panics.
2. Keep each `Source` handle and each `Select` widget across parent builds.
   Otherwise every window entry rebuilds (round 1: 968 builds instead of
   200).
3. Call `cx.unchanged()` in every channel handler. Otherwise the root view
   rebuilds and makes new `Select`s.
4. Never use `batch` through `Derived` (round 1 A2).
5. Call `update_keys()` after seeding keyed fields (round 1 A4).
6. Keep hot data out of reactive_stores, or pay double evaluation through
   parent triggers (above).

C's one rule of the same kind: never `cx.unchanged()` after changing what the
view reads. rtwk documents it at `rtwk/src/app.rs:986-991`.

**Maintainability verdict.** In phases 2-3, the case for automatic dependency
tracking is weak:
- rtwk's `ViewCx::memo` already makes C's dependencies explicit by
  construction.
- Recomputing every derivation per view costs about 3 scans of 40 threads,
  14.5 us per change.
- C is about 30 percent less state code, and its failure modes are compile
  errors or a single documented misuse.
- A-opt's extension is 5 lines against 10, but it is spread over more places.
- A's forgotten dependency is rarer but silent and intermittent.
- A adds six silent rules.

On these grounds C is better to maintain; A-sig is the acceptable form of A.

## 5. Would the rtwk-side fixes change A's numbers?

The fix ideas for the issues in
`~/.cache/codexcli/rtwk-issues/rtwk-state-reactive-graph.md` were (a)
`Derived` reads with `try_get` and keeps the last value instead of
panicking, (b) docs that state the one-thread contract, (c) an upstream
blocking read, (d) the 0.3 line.

- **(a) to (c): no.** These fix correctness off-thread (A naive's panics).
  Once reads and writes are confined to the UI thread, which A-opt and A-sig
  do, the panics are already gone and the numbers do not move. A naive's 160
  allocations per delta are reactive_stores' path and trigger work per write,
  which is upstream, not rtwk.
- **A working `batch`: modestly, and only for the store.** Emulated by
  merging the three field writes into one write of the thread node, A-opt
  goes from 41.8 to 28.0 allocations per delta and from 8.9 to 5.1 ms. It
  then re-evaluates `approval` every frame, because a parent write notifies
  all subfields. A-sig already evaluates each memo once per frame, so batch
  would save little there (not measured).
- **The new `Select`-inside-a-flush issue (section 3): latency and part of
  the Select-UI overhead.** A fix would remove the 13 ms first-frame delay
  after a quiet period. It would also remove one channel item per
  notification, part of the roughly 70 allocations per frame between
  `Select` and `memo` UIs (not separated).
- **(d) The 0.3 line** (`reactive_graph` 0.3.0-rc1, `reactive_stores` 0.5) is
  not measured.

## 6. Remaining gap and what the person decides

| | allocs/delta | scenario ms | dash us/change |
|---|---|---|---|
| C | 7.3 | 2.5 | 14.5 |
| A-sig (best A) | 15.6 (2.1x) | 3.7 (1.5x) | |
| A-opt store | 41.8 (5.7x), 28.0 with one write | 8.9 (3.6x), 5.1 with one write | 22.5 (1.6x) |
| A naive | 170.6 (23x) | 26.3 (10.5x) | |

The person decides:
1. ADR-003: C (plain model, `ViewCx::memo`, channel) or A-sig (signals,
   UI-thread confinement, per-frame coalescing).
2. Whether reactive_stores is allowed on hot data. On these numbers it should
   not be.
3. Whether to file the `Select`-inside-a-flush wake with rtwk, which matters
   for any A.

Reproduce:
- `~/.cache/rtwk/acargo test -q --release`: 9 tests.
- `acargo run -q --release --features count -- 1`: counts.
- `acargo run -q --release -- 21`: times.
- `acargo run -q --release [--features count] -- dash 15`: the dash scenario.
- `AOPT_NO_COALESCE=1`, `ASIG_NO_COALESCE=1` and `AOPT_ONE_WRITE=1` select
  the variants in the table.
