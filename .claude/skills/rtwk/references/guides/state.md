<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. Copied from docs/guide/state.md. -->
# State in rtwk

Keep ordinary application state in a Rust model. With `AppView`, the model
lives outside individual widgets and a handler can call its methods through
`AppCx`. Test the model's transitions without building a widget tree.

- Use `Obs<T>` when a few widgets need to track a changing value without
  rebuilding the whole view. A widget calls `cx.watch(obs)`. Write through
  the context and return whether the value changed.
- Use `rtwk::provider` to provide a value to a subtree. `read` looks up a
  value without subscribing, `watch` follows the whole value, and `select`
  rebuilds its consumer when the projection changes. An inner provider can
  override the same type for its subtree.
- Use `ViewCx::memo` to reuse a subtree across view builds when its inputs
  compare equal. A widget within the subtree can still rebuild when an
  observable or provider it reads changes, and so can the subtree's own
  `build` when it reads one through its `MemoCx` (`depend`, `watch`).
  Key your rows (`memo_keyed` with the row's id, an integer if you can):
  unkeyed rows are matched by place, so one inserted or removed rebuilds
  every row after it.

## Follow a store that already exists

Add `rtwk-state` alongside `rtwk` when the state belongs to an external
store. The store keeps its state, update rules, threads and other
subscribers. The widgets keep only the projections they display.

Implement `Source<R>` for a handle to a store projection. `get` reads the
current selection. `subscribe` registers a callback and returns an owned
guard that unregisters it on drop. Publish the new state before calling
subscribers and filter equal selections at the source. `Select::new` takes
an `Rc<dyn Source<R>>` and builds its child from `&R`. Keep that handle
across parent builds so the mounted widget can reuse the subscription.

Use `store_provider` when multiple widgets should share one subscription.
Give it an `Rc<T>` and a function that registers a change callback on the
store. Attach the child with `.child(...)`. Descendants can use
`rtwk::provider::select` to pick a part of `T`. A store change can wake the
provider even when none of its consumers needs to rebuild.

The app uses its terminal's waker by default. Store callbacks can run on a
writer's thread. The callback queues work and the waker interrupts the
terminal read so the application can draw it without a keypress.

### Coalescing changes

A `channel_with` with a nonzero-sized item allocates a box for every
payload. In the allocation test, after warming both queue buffers, 100
`u64` sends allocate 100 times and 100 unit sends allocate zero times on
the sender. Buffer growth and the work that applies queued items are
separate costs.

When each change must be processed, send its value. When only the latest
snapshot matters, use a shared pending flag and send `()` only when it
becomes pending. The UI clears the flag before reading the current
snapshot. If a producer delivers values without a readable source, store
the latest value in a `Mutex<Option<T>>` and send one unit when the slot
changes from empty to full. Take the value from the slot on the UI thread.
The same patterns apply to `Cx::channel_with` and `AppCx::channel`.

The adapter tests also record the exact allocation counts for one store
write, the following UI flush and a flush after a burst of 100 writes.
These are debug counts from the fixture on macOS arm64 with rustc 1.99.1.

| Fixture | Store write | Flush after one write | Flush after 100 writes |
| --- | ---: | ---: | ---: |
| `Select` with two projected subscribers | 3 | 5 | 2 |
| `store_provider` with two selecting descendants | 1 | 7 | 5 |
| `Select` with two reactive graph projections | 26 | 7 | 4 |

The write column includes the store or graph's own work. In each case
the burst wakes the UI once and rebuilds only the changed projection.
Use `RTWK_PERF_REPORT=1 cargo nextest run -p rtwk-state --all-features`
to compare incidental counts on this fixture.

Relative to the original adapter `2d4e48e1`, current RTWK saves one
128-byte allocation in all three burst flushes and in the provider's
one-write flush. Allocator traces identify the removed allocation as
growth of the owner's `Vec<(Eid, Payload)>` in `Executor::run_round`.
The owner has reused its small message buffers since `8bc72f44`.
The integration uses that existing behavior and keeps exact build and
wake assertions.

In a separate diagnostic checkout of this integration, use the three
original expectations from `2d4e48e1` and replace the
`if q.capacity() <= QUEUE_KEPT` block in `Owner::drain_queue` with
`drop(q)`. The same three tests reproduce all original counts with
`RTWK_PERF_REPORT=1`. Restoring the retention block restores the
current counts.

## Follow a reactive graph

Enable `rtwk-state`'s `reactive-graph` feature for
`rtwk_state::reactive::Derived`. `Derived::new` computes a lazy memo from
the signals its closure reads. `Derived::from_memo` follows an existing
`ArcMemo`. The memo decides whether the projection changed, and rtwk
decides when the UI builds. The adapter subscribes to the memo with
reference-counted signals and needs no reactive_graph owner or executor.
Write the graph on the UI thread ([below](#the-graph-is-the-ui-threads)). A
subscription hears one notification per change of the memo's value, and one
per `reactive_graph::effect::batch`: the last value of the batch, even when
it nets to no change (`Select` then builds nothing, as it compares what it
shows). A `batch` is process-wide: one on any thread defers every effect in
the process until it ends. The
[batch tests](../../crates/rtwk-state/tests/reactive_batch.rs) run in a
separate test binary and take a mutex, because of that one global. They
also pin the allocation cost of the adapter: subscribing allocates 15
times, a write that changes the memo's value 16, a write the memo filters
8, a batch of two writes 28 and a write after the subscription is dropped
3 (debug counts, macOS arm64; dropping allocates nothing).

### The graph is the UI thread's

Write the graph on the UI thread, as twk keeps its state on the UI isolate
and posts values to it. A worker thread, or a task on another runtime,
sends its result through an rtwk channel (`AppCx::channel`,
`Cx::channel_with`), and the channel's function, which runs on the UI
thread in a flush, writes the signal:

```rust,ignore
let tokens = cx.channel(|app: &mut Chat, token: String, cx| {
    app.reply.update(|text| text.push_str(&token));
    cx.unchanged(); // the Select over the reply rebuilds by itself
});
std::thread::spawn(move || {
    for token in stream {
        let _ = tokens.send(token);
    }
});
```

A burst of items is applied in one flush and builds once. When only the
newest value matters, use the pending flag or latest-value slot of
[Coalescing changes](#coalescing-changes): the producer then allocates
nothing. The
[frame-loop tests](../../crates/rtwk-state/tests/support/frame_loop.rs)
`reactive_projection_draws_a_cross_thread_update_without_terminal_input`
and
`producers_post_through_the_app_channel_and_the_ui_thread_writes_the_graph`
run this route, the second with three producer threads.

reactive_graph 0.2 is not safe when two threads use one graph at once, and
no adapter can make it so, because your own code reads the same graph.
Two things go wrong:

- A read panics. A signal, a memo or a store is read with a non-blocking
  lock. A read that meets a write on another thread panics "already been
  disposed", or finds the memo's value taken out for a recompute. The
  panic comes in your memo's closure and in `Derived`'s `get`. In a real
  app loop, one producer writing the signal directly ended the app at
  10,000 writes a second, and at 2,000 a second it survived three rounds
  of 4 s.
- The memo keeps a stale value, and nothing panics. Two threads recompute
  one memo together, the slower writes back last, and the memo is marked
  clean with the older value. After the writers stop, 2 to 4 % of the
  contended runs of the research showed the wrong value until the next
  write: a stream's last token that never appears.

reactive_graph also prints "ImmediateEffect recursed more than once." with
`eprintln!` under such contention, 50,000 lines a second over the
terminal. `rtwk-state` enables reactive_graph's `tracing` feature, which
in 0.2.16 drops those warnings (they reach no `tracing` subscriber
either), so they do not draw over the screen.

A `Derived` reports a write that reaches one of its subscriptions from a
thread other than the one that subscribed (`Select` subscribes on the UI
thread), once per `Derived`, as a warning in the app's log, at its next
read on the UI thread. It is made in release builds as well as in debug,
and costs one thread-id comparison and no allocation per notification. It
sees the writes that reach a followed `Derived`, not every misuse of the
graph, and it cannot prevent the panic or the stale value that the same
write may cause. The decision and its evidence are in ADR-026 of
[DECISIONS.md](../DECISIONS.md) and
[the research](../research/state-cross-thread.md).

### Keyed stores

With `reactive_stores`, call `update_keys()` on a keyed field once after
building the store, and again after you push a new item through the
parent store (`store.update(|s| s.items.push(..))`). reactive_stores 0.4.4
takes the write lock of an `at_key` write before it resolves the key. When
that write comes before the field's first read, the key list it caches is
empty, the write is lost, and later `at_key` reads panic. A push through
the parent never refreshes the keys, so `at_key` on the new item panics
until `update_keys()` runs. A push through the keyed field itself
registers its key. The
[keyed-store tests](../../crates/rtwk-state/tests/keyed_store.rs) pin each
of these.

Every push through a keyed field rebuilds its whole key map, in time and
allocations proportional to the length: one allocation per key, so a push
at 1,000 items allocates about 1,000 times and at 100 about 100, where a
push to an unkeyed `Vec` allocates 4 times at either length (debug,
macOS arm64). For an append-only list, such as a transcript, use an
unkeyed `Vec` field and read it with `at_unkeyed`.

The [counter example](../../crates/rtwk-state/examples/state.rs) keeps an
`ArcRwSignal` and a derived projection outside the widget tree. Press Up
to increment the store and q to exit.

```sh
cargo run -p rtwk-state --features reactive-graph --example state
```

## Bloc and Riverpod equivalents

The replacement exercises the same state boundary through the Rust APIs.
The [adapter tests](../../crates/rtwk-state/tests/adapters.rs) cover
selection and rebuild isolation, queued changes, replacing the source,
releasing subscriptions, external listeners, lazy provider subscriptions,
scope overrides, dynamic dependencies and explicit invalidation. The
[frame-loop tests](../../crates/rtwk-state/tests/support/frame_loop.rs)
wait for a blocked terminal read and show a writer's update in one frame
without terminal input, and run producer threads through an app channel.

| In twk | In rtwk |
| --- | --- |
| `BlocSelector`, `BlocBuilder` with `buildWhen` | `Select` follows a projection. Put a state-dependent filter in the store's subscription. |
| `BlocProvider`, `context.select` | `store_provider`, `rtwk::provider::select` |
| `BlocListener` | An independent store subscription whose guard controls its lifetime |
| `ProviderScope`, overrides, read and watch | `rtwk::provider` scopes and projections, and `Obs<T>` for local UI state |
| Derived providers and invalidation | An external memo graph via `Derived`. A reactive_graph `ArcTrigger` can invalidate a computation that depends on external data. |
