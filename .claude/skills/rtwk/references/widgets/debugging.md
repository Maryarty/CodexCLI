<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Debugging

[All widgets](../widgets.md)

## Console

Log records listed over the app, newest last; the app has one of its own, which Ctrl+G then c opens.

API:

- `rtwk::debug::Console` (struct, `crates/rtwk/src/debug/console.rs:1359`) Shows `child` and, while `controller` is open, its log console over the bottom `rows` rows of `child`, or over all of it when it is shorter.
  - public fields: `controller: ConsoleController`, `child: AnyWidget`, `rows: i32`

### Code

The gallery's Code tab at the initial options (route `#/debugging/console`, `crates/rtwk-gallery/src/pages/semantics_debug.rs:79`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::debug::{Console, ConsoleController};
use rtwk::text::Text;
use rtwk_core::log::{LogLevel, LogSink};

pub fn build(sink: &LogSink) -> Console {
    let controller = ConsoleController::new(sink);
    controller.set_level(LogLevel::Debug);
    controller.set_open_on_error(false);
    controller.open();
    Console::new(controller, Text::new("The app, under its console.")).rows(6)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| level | debug, info, warning, error | debug |
| rows | 4, 6, 8 | 6 |
| live | on, off | true |
| openOnError | on, off | false |

### Methods

#### `Console`: constructors and associated functions

- `pub fn new(controller: ConsoleController, child: impl Into<AnyWidget>) -> Console` - Shows `child`, and `controller`'s console over it while it is open, 10 rows tall.

#### `Console`: builder methods (take and return `self`)

- `pub fn rows(mut self, rows: i32) -> Console` - The console `rows` rows tall.

### Also used in the code

- `rtwk::debug::ConsoleController` (struct, `crates/rtwk/src/debug/console.rs:151`) What a log console shows of a `LogSink` and what is done to it: whether it is open, the records listed — those at `level` and above, of the ones the sink keeps, which it takes as they are logged — the window onto them, ...
- `rtwk_core::log::LogLevel` (enum, `crates/rtwk-core/src/log.rs:28`) How much a `LogRecord` matters, from the least to the most.
  - variants: `Debug`, `Info`, `Warning`, `Error`

## FrameOverlay

Each frame's numbers over the app: the time its build, layout, paint and write took and the bytes it wrote; Ctrl+G then o.

API:

- `rtwk::debug::FrameOverlay` (struct, `crates/rtwk/src/debug/frame_overlay.rs:223`) The frame overlay: the meter of the frames, what the panel shows of it, and when it wants a frame to show new numbers.

### Code

The gallery's Code tab at the initial options (route `#/debugging/frameoverlay`, `crates/rtwk-gallery/src/pages/semantics_debug.rs:114`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| overlay | on, off | true |

### Methods

#### `FrameOverlay`: constructors and associated functions

- `pub fn new() -> FrameOverlay` - An overlay that has seen no frame.

#### `FrameOverlay`: methods

- `pub fn frames(&self) -> usize` - How many frames it counted.
- `pub fn is_dirty(&self) -> bool` - Whether the panel is marked to show new numbers in the next frame.
- `pub fn is_refreshing(&self) -> bool` - Whether the mark of the panel is in the repaints of the frame being presented, which the overlay leaves out.
- `pub fn refreshes(&self) -> (usize, usize)` - How many times it showed numbers (the first, in `start`, too), and in how many of the frames that showed them the numbers were the ones already shown, so that the frame wrote nothing.
- `pub fn next_refresh(&self) -> Option<u64>` - The time (microseconds of the app's clock) a refresh waits for, none when it waits for nothing.
- `pub fn start(&mut self, now: u64, last: &FrameSample, rebuilt: u64)` - Takes the last frame as the first and shows it at once: the overlay turned on shows the numbers of the frame before it.
- `pub fn refresh(&mut self, now: u64)` - Shows the numbers kept: the frames a second, then each number's last and median; the panel is marked.
- `pub fn tick(&mut self, now: u64)` - The time has come for the refresh that waited.
- `pub fn after_frame(&mut self, now: u64, s: &FrameSample, rebuilt: u64, own: PanelShare)` - Takes the frame just presented: counts it when the app did anything in it (less the panel's own share), and schedules the refresh of what is shown.
- `pub fn dispose(&mut self)` - Stops what waits.
- `pub fn paint(&mut self, canvas: &mut Canvas<'_>, theme: &ThemeData, width: i32, height: i32)` - Paints the panel `width` × `height` at the canvas origin, and notes the time it took as its own share of the frame.
- `pub fn take_share(&mut self) -> PanelShare` - The panel's share of the frame since the last call, and whether it was painted: taken, it starts again.
- `pub fn add_layout_micros(&mut self, micros: u64)` - Notes `micros` of layout as the panel's own.
- `pub fn present_alone(&mut self, spec: &AloneSpec, theme: &ThemeData) -> AloneStats` - Paints the panel alone on a screen of the app's terminal, at its place, and presents it: what that wrote is the panel's own share of the frame's bytes and cells.

### Also used in the code

- `rtwk::debug::PanelShare` (struct, `crates/rtwk/src/debug/frame_overlay.rs:91`) The panel's own share of a frame, which the overlay leaves out of what it shows: the host measured it.
  - public fields: `layout_micros: u64`, `paint_micros: u64`, `painted: bool`, `alone: AloneStats`

## Repaint rainbow

Each repaint boundary tinted a new colour when it paints, and relayouts marked, to see what a change redraws; Ctrl+G then r, and l.

API:


### Code

The gallery's Code tab at the initial options (route `#/debugging/repaint-rainbow`, `crates/rtwk-gallery/src/pages/semantics_debug.rs:138`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk_render::RenderTree;

pub fn configure(tree: &mut RenderTree) {
    tree.set_debug_repaint_rainbow(true);
    tree.set_debug_relayout_highlight(false);
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| rainbow | on, off | true |
| relayout | on, off | false |

## RepaintBoundary

A subtree painted on its own layer: what changes in it is painted again without what is around it, and the other way round.

API:

- `rtwk::layout::RepaintBoundary` (struct, `crates/rtwk-widgets/src/layout/repaint_boundary.rs:45`) Makes its child a part that repaints on its own: what changes inside it paints again only its own cells, over what it painted before, not the parent and not the siblings (twk's `RepaintBoundary`, a `RenderEffect` with ...
  - public fields: `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/debugging/repaintboundary`, `crates/rtwk-gallery/src/pages/semantics_debug.rs:165`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| boundary | on, off | true |
| every | 300, 1000 | 300 |

### Methods

#### `RepaintBoundary`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> RepaintBoundary` - `child` as a part that repaints on its own.
