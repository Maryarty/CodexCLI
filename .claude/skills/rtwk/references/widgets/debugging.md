<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Debugging

The log console, the frame overlay, the repaint rainbow and repaint boundaries.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Console

Log records listed over the app, newest last; the app has one of its own, which Ctrl+G then c opens.

- `rtwk::debug::Console` (struct) in `crates/rtwk/src/debug/console.rs:1359`: Shows `child` and, while `controller` is open, its log console over the bottom `rows` rows of `child`, or over all of it when it is shorter.

### Code

```rust
use rtwk::debug::{Console, ConsoleController};
use rtwk::log::{LogLevel, LogSink};
use rtwk::text::Text;

pub fn build(sink: &LogSink) -> Console {
    let controller = ConsoleController::new(sink);
    controller.set_level(LogLevel::Debug);
    controller.set_open_on_error(false);
    controller.open();
    Console::new(controller, Text::new("The app, under its console.")).rows(6)
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| level | debug, info, warning, error | debug |
| rows | 4, 6, 8 | 6 |
| traffic | false, true | true |
| open on error | false, true | false |

## FrameOverlay

Each frame's numbers over the app: the time its build, layout, paint and write took and the bytes it wrote; Ctrl+G then o.

- `rtwk::debug::FrameOverlay` (struct) in `crates/rtwk/src/debug/frame_overlay.rs:223`: The frame overlay: the meter of the frames, what the panel shows of it, and when it wants a frame to show new numbers.

### Code

```rust
use rtwk::debug::{FrameOverlay, FrameSample, PanelShare};

pub fn record(sample: &FrameSample, now_micros: u64) -> FrameOverlay {
    let enabled = true;
    let mut overlay = FrameOverlay::new();
    if enabled {
        overlay.start(now_micros, sample, 0);
        overlay.after_frame(now_micros, sample, 0, PanelShare::default());
        overlay.refresh(now_micros);
    }
    overlay
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| overlay | false, true | true |

## Repaint rainbow

Each repaint boundary tinted a new colour when it paints, and relayouts marked, to see what a change redraws; Ctrl+G then r, and l.

- `rtwk_render::RenderTree` (struct) in `crates/rtwk-render/src/tree.rs:436`: A retained tree of RenderObjects and the pipeline that lays it out and paints it from its root, a frame at a time: what was marked since the last frame, and nothing else (twk's `PipelineOwner` and the tree it owns).

### Code

```rust
use rtwk::WidgetTree;

pub fn configure(tree: &mut WidgetTree) {
    tree.render_tree_mut().set_debug_repaint_rainbow(true);
    tree.render_tree_mut().set_debug_relayout_highlight(false);
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| rainbow | false, true | true |
| relayout highlight | false, true | false |

## RepaintBoundary

A subtree painted on its own layer: what changes in it is painted again without what is around it, and the other way round.

- `rtwk::layout::RepaintBoundary` (struct) in `crates/rtwk-widgets/src/layout/repaint_boundary.rs:45`: Makes its child a part that repaints on its own: what changes inside it paints again only its own cells, over what it painted before, not the parent and not the siblings (twk's `RepaintBoundary`, a `RenderEffect` with no effect).

### Code

```rust
use rtwk::AnyWidget;
use rtwk::layout::RepaintBoundary;
use std::time::Duration;

pub fn every() -> Duration {
    Duration::from_millis(300)
}

pub fn build(marker: AnyWidget) -> AnyWidget {
    let boundary = true;
    if boundary {
        RepaintBoundary::new(marker).into()
    } else {
        marker
    }
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| RepaintBoundary | false, true | true |
| a step every | 300 ms, 1 s | 300 ms |
