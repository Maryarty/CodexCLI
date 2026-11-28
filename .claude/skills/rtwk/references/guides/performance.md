<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Performance

Most apps need nothing on this page. A frame builds only what a change marked,
lays out only what changed, paints only what changed and writes only the cells
that differ from the screen. Measure before you change anything: the sections
below say what rtwk does by itself, what you can narrow, and the one option
that is a trade.

<!-- not rtwk: Criterion -->

Where a figure is quoted it is rtwk's own, from the performance
baselines: the core's frame benchmark
(`bench/`, `rtwk-bench`, 200 x 60 cells, release build, M1 Max, recorded in
`bench/baselines/macos-arm64-m1-max/foundation.json` at 1-minute load 7.0 to
6.2) and the widget layer's Criterion benchmarks, each with the load of its
run. They are the time one frame costs, not what an app feels like; a 60 Hz
frame is 16 600 microseconds. A machine that is busy moves them by tens of
percent.

## What costs nothing

- **A still app runs no frame.** A frame is requested by a state change, a
  watched observable, a ticker or an input event. When nothing asks, nothing
  runs; the benchmark's `idle` frame, when one does run, writes no bytes,
  allocates nothing and takes 102 ns (p50; p95 104 ns).
- **At most `max_fps` frames a second, 60 by default.** Frames asked for sooner
  than that gap after the last one wait for it and are one frame; several
  changes between two frames make one build. the `max_fps` of `RunOptions` takes fewer
  (`0` is no cap). Changing a state a thousand times a second
  therefore costs a thousand cheap changes and sixty frames.
- **Tickers stop when idle.** An animation (`Spinner`, an animation
  controller) has the app run frames while it moves and not after. A `Spinner`
  repaints its own cells and builds nothing.
- **Lazy lists are lazy.** `ListView::builder(count, item)` builds the rows in
  view: a list of 100 000 items costs what the rows on the screen cost. The
  benchmark's `scroll_log` (58 rows scrolled by a recorded move) takes 4.5 us
  and writes 173 bytes a frame, with no allocation. A `Column` of children is still all built, so for a long list use
  the builder.
- **The terminal sees only what changed.** The presenter compares the frame
  with the screen and writes the difference; the benchmark's `typing` frame (a
  character typed mid-line) writes 17 bytes and takes 0.79 us, and a changed
  cell 4 bytes in 0.14 us. You do not need a double buffer or a dirty flag for
  that.

## Say what changed

A build runs when its state is marked, and rebuilds every widget it returns
that is not equal to the one before. Two things keep that small.

A widget that implements equality (`impl_widget!(stateful eq Button)`; rtwk's
`Button` and `TextField` do) and is equal to the one it replaces is not
updated, and its subtree is not visited. Put the state low: a change high in
the tree builds everything under it that is not equal to what it built before.
The application root costs what its components cost: in the facade's budgets
(`tests/facade_budgets.rs`) a local change of the benchmark dashboard is 2
allocations and one build, the same as for a component with a state of its own.

The second is a part that reads what it needs itself. `ViewCx::memo(deps,
build)` keeps a subtree while its `deps` compare equal, and the function it is
given can `watch` an observable: the node, and nothing above it, is built again
when the value is written.

<!-- file: crates/rtwk/examples/guides/perf_watch.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::{AppView, MemoCx, RunOptions, ViewCx};
use rtwk::layout::CrossAxisAlignment;
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    run_app_view(
        AppView::new(Dashboard::default(), view)
            .on_start(|app, cx| app.count = Some(cx.observable(0))),
        RunOptions::default(),
    )
}

#[derive(Default)]
pub struct Dashboard {
    pub count: Option<Obs<u32>>,
}

pub fn view(app: &Dashboard, cx: &mut ViewCx<Dashboard>) -> AnyWidget {
    let plus = cx.on(|app, cx| {
        // the application's own state is not changed: the view has nothing to run again for
        cx.unchanged();
        if let Some(count) = app.count {
            cx.write(count, |n| {
                *n += 1;
                true
            });
        }
    });
    Focus::new(
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            // the memo node reads the observable itself: a write builds it, and not this view
            .child(cx.memo(app.count, counter))
            .child(Text::new("+ counts, q quits")),
    )
    .autofocus()
    .bind('+', plus)
    .bind('q', cx.on(|_, cx| cx.quit()))
    .into()
}

fn counter(count: &Option<Obs<u32>>, cx: &mut MemoCx<Dashboard>) -> AnyWidget {
    let n = count.map_or(0, |count| cx.watch(count));
    Text::new(format!("count: {n}")).into()
}
```

Pressing `+` builds the `Text` of the count; `view` is not run again, because
the handler says `cx.unchanged()` (it changed none of the application's own
fields) and the observable's only reader is the memo node. Without that line the
view runs too, and keeps the memo node for the cost of one comparison. The
[state guide](state.md) has the other ways to hold state, and they build the
same way: the nodes that read a value are the ones that rebuild.

A build is cheap, though. In the widget layer's benchmark
(`cargo bench -p rtwk --bench framework`, Criterion, medians; the machine was
shared with other work, 1-minute load 5.3 to 13.3) a local change of the
dashboard, from the event through the frame, takes 1.41 us and a full rebuild
of its 1 113 widgets 1.10 ms; most of the cost of a frame is painting and
writing, which only the rows that changed pay. Narrow a rebuild when a profile
shows builds, not by habit. A flat application that keeps no subtree runs its
whole view for a local change (1.3 ms for the same dashboard, 118 us with
`memo` around each group, in the facade's own benchmark at load 13.8 to 41.5):
the cost is in proportion to the view, so `memo` or a component is for the
part that is large and rarely changes. Key the rows of a list
(`memo_keyed`), so that one inserted or removed does not rebuild the ones after
it.

## A part that changes often

A `RepaintBoundary` makes its child repaint alone, over what was under it, and
keeps what is around it from being painted again with it. A clock, a counter or
a log under one does not repaint the screen with it.

A `Spinner` is one already. Before you add one, run with the `rainbow` debug
aid (`rtwk run --debug rainbow`; [Getting started](getting-started.md) has the
debug aids): it tints what repaints, and a region that flashes when it should
not is where one goes.

## A long text edited one row at a time

A `Text` of eight rows or more, edited a few times without changing its size (a
checklist, a log you edit, a rendered document), keeps what it painted by itself
and from then on repaints only the rows that differ. There is nothing to set,
and a text that lies over another object's text stays as it was, as does a short
or a still one: it reads the cells under it before it turns, and turns back if a
glyph comes there.

It is a small gain. In `docs/research/text-repaint.md` (`cargo bench -p
rtwk-render --bench keeps_paint`, load 9.1 to 12.0, the ratios of alternating
runs) a word typed into a 200 x 60 paragraph takes 40.6 us a frame plain and
26.6 us kept, and damages 856 cells where the plain one damages 10 459, against
a frame budget of 16 600 us. A render object of your own has
`RenderParagraph::keeps_paint` to force it, and its contract in the doc comment;
a text that moves most of its rows each frame is slower with it (15 % in the
research's reflow case), which is why it is not set for you.

## Streams and logs

- A `MarkdownView` with `appended` set to the piece added checks the length and the end of the
  text instead of comparing all of it, so a streamed answer costs the piece, not
  the answer. Without `appended` the whole text is compared. `MarkdownStream`
  keeps the blocks that are settled. [Markdown](markdown.md) has the view.
- A `LogView` holds at most `max_lines` of a `LogController` (10 000 by
  default). At its cap each new line moves every index, so the window repaints
  each frame. A smaller cap is cheaper, and a log nobody reads past the last
  screen does not need 10 000.

## Measure

- The `overlay` debug aid shows each frame's times and bytes in the app (`rtwk
  run --debug overlay`).
- For a change to rtwk itself, `cargo xtask bench-check` runs the benchmarks
  against the baseline of the machine and fails on more bytes or allocations or
  a slowdown; `cargo xtask perf-report` lists the allocation counts the tests
  keep. Both are in the repository, not in the packages. A benchmark's times
  count only when the machine is quiet; a busy one moves them by tens of
  percent, as the figures above did, and the baseline file records the load it
  was taken at.

## Habits from other toolkits that do not carry over

- **A deep widget tree is not the worry.** A build is a few microseconds, and an
  element that is equal to what it had is not visited at all.
- **The allocator is not where the time goes.** A frame that changes nothing
  allocates nothing, and the budgets above count the allocations of the rest.
- **There is no 60 fps to hit by hand.** The frame rate is a cap (`max_fps`), not
  a target: an app that changes nothing runs no frames.

[Coming from Flutter](coming-from-flutter.md) lists the rest of what differs.
