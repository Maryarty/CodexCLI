<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Animation

[All widgets](../widgets.md)

## Spinner

Frames shown in turn while something goes on; spinners of the same frames step together.

API:

- `rtwk::animation::Spinner` (struct, `crates/rtwk-animation/src/spinner.rs:47`) The frames of `frames` — those of the theme's `Glyphs::spinner` unless it is given some: dots in the Unicode glyphs, a turning line in the ASCII ones — in turn, a frame each interval of the frames' clock: the frame of ...
  - public fields: `frames: Option<SpinnerFrames>`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/animation/spinner`, `crates/rtwk-gallery/src/pages/animation_progress.rs:51`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::animation::Spinner;
use rtwk::layout::{CrossAxisAlignment, MainAxisSize};
use rtwk::style::{SpinnerFrames, ThemeData};
use rtwk::text::Text;
use rtwk::{AnyWidget, column, row};

pub fn build(theme: &ThemeData) -> AnyWidget {
    let tasks = ["Resolving packages…", "Compiling…", "Running 214 tests…"];
    column![]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(1)
        .children(tasks.map(|task| {
            row![
                Spinner::new()
                    .frames(SpinnerFrames::DOTS)
                    .style(theme.primary.clone()),
                Text::new(task),
            ]
            .main_axis_size(MainAxisSize::Min)
            .gap(1)
            .into()
        }))
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| frames | dots, line, arc | dots |
| color | primary, accent, success, warning, info | primary |

### Methods

#### `Spinner`: constructors and associated functions

- `pub fn new() -> Spinner` - A spinner of the theme's frames in the theme's primary style.

#### `Spinner`: builder methods (take and return `self`)

- `pub fn frames(mut self, frames: SpinnerFrames) -> Spinner` - Of `frames`, not the theme's.
- `pub fn style(mut self, style: Style) -> Spinner` - In `style`, over the theme's text, not the theme's primary.

### Also used in the code

- `rtwk::style::SpinnerFrames` (struct, `crates/rtwk-widgets/src/style/glyphs.rs:18`) The frames of a spinner, shown in turn, each for `interval` (twk's `SpinnerFrames`, of `animation/spinner.dart`: defined here, where `Glyphs::spinner` needs it; the animation module's `Spinner` uses this type).
  - public fields: `frames: Cow<'static`, `interval: Duration`

## TickerMode

Whether the animations below tick: off, they ask for no frames and stand still, and go on when it is on again; Offstage and Visibility turn it off for what they hide.

API:

- `rtwk::animation::TickerMode` (struct, `crates/rtwk-framework/src/ticker.rs:891`) Whether the `Ticker`s below tick (twk's `TickerMode`): what is not shown asks for no frames.
  - public fields: `enabled: bool`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/animation/tickermode`, `crates/rtwk-gallery/src/pages/animation_progress.rs:69`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::animation::{Spinner, TickerMode};
use rtwk::layout::MainAxisSize;
use rtwk::style::ThemeData;
use rtwk::text::Text;
use rtwk::{AnyWidget, row};

pub fn build(theme: &ThemeData) -> AnyWidget {
    let enabled = true;
    TickerMode::new(
        enabled,
        row![
            Spinner::new().style(theme.primary.clone()),
            Text::new("Compiling…"),
        ]
        .main_axis_size(MainAxisSize::Min)
        .gap(1),
    )
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| enabled | on, off | true |

### Methods

#### `TickerMode`: constructors and associated functions

- `pub fn new(enabled: bool, child: impl Into<AnyWidget>) -> TickerMode` - Turns the tickers in `child` on or off.
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> bool` - Whether the tickers at `cx` tick: the mode of the nearest `TickerMode` above it, true where there is none.

### Also used in the code

- `rtwk::animation::Spinner` (struct, `crates/rtwk-animation/src/spinner.rs:47`) The frames of `frames` — those of the theme's `Glyphs::spinner` unless it is given some: dots in the Unicode glyphs, a turning line in the ASCII ones — in turn, a frame each interval of the frames' clock: the frame of ...
  - public fields: `frames: Option<SpinnerFrames>`, `style: Option<Style>`

## ProgressBar

A bar filled as far as its value, in eighths of a cell; a block going to and fro while the value is not known.

API:

- `rtwk::animation::ProgressBar` (struct, `crates/rtwk-animation/src/progress_bar.rs:43`) A bar filled to `value`, from 0 (empty) to 1 (full), in eighths of a cell — braille halves where block elements are two columns wide, the theme's shades where its glyphs have no partial blocks — as wide as it may be and ...
  - public fields: `value: Option<f64>`, `style: Option<Style>`, `semantics_label: Option<Str>`

### Code

The gallery's Code tab at the initial options (route `#/animation/progressbar`, `crates/rtwk-gallery/src/pages/animation_progress.rs:82`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::animation::ProgressBar;
use rtwk::layout::{MainAxisSize, SizedBox};
use rtwk::style::{Style, ThemeData};
use rtwk::text::Text;
use rtwk::{AnyWidget, row};

pub fn build(theme: &ThemeData) -> AnyWidget {
    let value: Option<u32> = Some(33);
    let style = Style {
        bg: theme.surface.bg,
        ..theme.success.clone()
    };
    let mut row = row![
        SizedBox::new(Some(40), None, None)
            .child(ProgressBar::new(value.map(|v| f64::from(v) / 100.0)).style(style),)
    ]
    .main_axis_size(MainAxisSize::Min)
    .gap(1);
    if let Some(value) = value {
        row = row.child(Text::new(format!("{value:>3}%")));
    }
    row.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| known | on, off | true |
| value | 0..100 (a slider) | 33% |
| color | primary, accent, success, warning, info | success |

### Methods

#### `ProgressBar`: constructors and associated functions

- `pub fn new(value: Option<f64>) -> ProgressBar` - A bar of `value` in the theme's primary style.

#### `ProgressBar`: builder methods (take and return `self`)

- `pub fn style(mut self, style: Style) -> ProgressBar` - In `style`, not the theme's primary.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> ProgressBar` - Called `label` to a screen reader.

## ProgressList

Many tasks at once, a row each: its mark, a bar, how much is done, its rate and the time it has left, from a ProgressTracker the work counts on.

API:

- `rtwk::animation::ProgressList` (struct, `crates/rtwk-animation/src/progress_list.rs:1080`) The tasks of a `tracker`, a row each, with the `columns` it is given: the mark of each task's `TaskItemStatus`, its label, a bar as a `ProgressBar` draws it — sweeping while the task runs without a total — and how far ...
  - public fields: `tracker: ProgressTracker`, `columns: Rc<[ProgressColumn]>`, `hide_finished: bool`, `max_visible: Option<usize>`, `spinner: Option<SpinnerFrames>`, `bar_shape: ProgressBarShape`, `max_bar_width: Option<i32>`, `byte_units: ByteUnits`

### Code

The gallery's Code tab at the initial options (route `#/animation/progresslist`, `crates/rtwk-gallery/src/pages/animation_progress.rs:109`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Drive sample downloads at 300 ms intervals on the app's frame clock.
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rtwk::animation::{
    ProgressBarShape, ProgressColumn, ProgressList, ProgressTracker, ProgressUnit,
};
use rtwk::binding::AppBinding;
use rtwk::layout::SizedBox;
use rtwk::style::SpinnerFrames;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::app::Clock;

const DOWNLOADS: [(&str, f64, f64); 3] = [
    ("assets.zip", 3_400_000.0, 170_000.0),
    ("fonts.tar.gz", 1_200_000.0, 140_000.0),
    ("docs.tar.gz", 5_600_000.0, 90_000.0),
];
const TICK_MS: u64 = 300;
const RUN: u64 = 40;
const DOCS_FAIL: u64 = 12;
const BUILD_DONE: u64 = 30;

#[derive(Debug, Default)]
struct DemoClock(AtomicU64);
impl DemoClock {
    fn set(&self, time: Duration) {
        let millis = u64::try_from(time.as_millis()).unwrap_or(u64::MAX);
        self.0.store(millis, Ordering::Relaxed);
    }
}
impl Clock for DemoClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::Relaxed))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DownloadPreview {
    pub columns: &'static [ProgressColumn],
    pub shape: ProgressBarShape,
    pub spinner: SpinnerFrames,
    pub hide_finished: bool,
    pub max_visible: Option<usize>,
    pub visible: bool,
    pub frozen: Option<Duration>,
}

struct DownloadState {
    clock: Arc<DemoClock>,
    tracker: ProgressTracker,
    born: Option<Duration>,
    tick: Option<u64>,
    wake: Option<(AppBinding, u64)>,
}

impl StatefulWidget for DownloadPreview {
    type State = DownloadState;
    fn create_state(&self) -> Self::State {
        let clock = Arc::new(DemoClock::default());
        DownloadState {
            tracker: ProgressTracker::with_clock(clock.clone()),
            clock,
            born: None,
            tick: None,
            wake: None,
        }
    }
}
impl_widget!(stateful eq DownloadPreview);

impl DownloadState {
    fn cancel_wake(&mut self) {
        if let Some((binding, id)) = self.wake.take() {
            binding.cancel_frame_callback(id);
        }
    }

    fn start(&self) {
        for task in self.tracker.tasks() {
            self.tracker.remove(&task);
        }
        for (name, size, _) in DOWNLOADS {
            self.tracker
                .add_with(name, Some(size), ProgressUnit::Bytes)
                .start();
        }
        self.tracker.add("Building binary").start();
    }

    fn step(&self, tick: u64) {
        let tasks = self.tracker.tasks();
        for (task, (_, size, bytes)) in tasks.iter().zip(DOWNLOADS) {
            task.advance(bytes);
            if task.completed() >= size {
                task.done();
            }
        }
        if tick == DOCS_FAIL {
            tasks[2].fail_with("connection reset");
        }
        if tick == BUILD_DONE {
            tasks[3].done();
        }
    }

    fn refresh(&mut self, w: &DownloadPreview, cx: &mut Cx<'_, Self>) {
        self.cancel_wake();
        if !w.visible {
            return;
        }
        let binding = AppBinding::maybe_of(cx);
        let now = binding
            .as_ref()
            .map_or(Duration::ZERO, AppBinding::frame_time);
        let born = *self.born.get_or_insert(now);
        let elapsed = w.frozen.unwrap_or_else(|| now.saturating_sub(born));
        let due = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX) / TICK_MS;
        let run_start = due - due % RUN;

        // All tasks are recreated at each run boundary. Replay only the current
        // run if a late frame crossed a boundary, including a frozen route.
        let previous = if let Some(tick) = self.tick.filter(|tick| (run_start..=due).contains(tick))
        {
            tick
        } else {
            self.clock.set(Duration::from_millis(run_start * TICK_MS));
            self.start();
            run_start
        };
        for tick in (previous + 1)..=due {
            self.clock.set(Duration::from_millis(tick * TICK_MS));
            self.step(tick % RUN);
        }
        self.clock.set(elapsed);
        self.tick = Some(due);

        if w.frozen.is_none()
            && let Some(binding) = binding
        {
            let next =
                born.saturating_add(Duration::from_millis((due + 1).saturating_mul(TICK_MS)));
            let id = binding.schedule_frame_callback(cx.map(|_: Duration| ()), Some(next));
            self.wake = Some((binding, id));
        }
    }
}

impl State for DownloadState {
    type Widget = DownloadPreview;
    type Msg = ();

    fn init_state(&mut self, w: &DownloadPreview, cx: &mut Cx<'_, Self>) {
        self.refresh(w, cx);
    }

    fn did_update_widget(
        &mut self,
        old: &DownloadPreview,
        w: &DownloadPreview,
        cx: &mut Cx<'_, Self>,
    ) {
        if !old.visible && w.visible {
            // Re-entering the preview starts a new download run, as a remount does.
            self.born = None;
            self.tick = None;
        }
        if old.visible != w.visible || old.frozen != w.frozen {
            self.refresh(w, cx);
        }
    }

    fn update(&mut self, (): (), w: &DownloadPreview, cx: &mut Cx<'_, Self>) -> Rebuild {
        self.refresh(w, cx);
        Rebuild::Yes
    }

    fn dispose(&mut self) {
        self.cancel_wake();
        self.tracker.dispose();
    }

    fn build(&self, w: &DownloadPreview, _: &mut Cx<'_, Self>) -> AnyWidget {
        let mut list = ProgressList::new(self.tracker.clone())
            .columns(w.columns)
            .bar_shape(w.shape)
            .spinner(w.spinner.clone())
            .hide_finished(w.hide_finished);
        if let Some(rows) = w.max_visible {
            list = list.max_visible(rows);
        }
        SizedBox::new(Some(60), None, None).child(list).into()
    }
}


pub fn build() -> AnyWidget {
    DownloadPreview {
        columns: &ProgressList::DEFAULT_COLUMNS,
        shape: ProgressBarShape::Blocks,
        spinner: SpinnerFrames::DOTS,
        hide_finished: false,
        max_visible: None,
        visible: true,
        frozen: None,
    }
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| columns | default, count, elapsed | default |
| bar | blocks, line | blocks |
| spinner | dots, line, arc | dots |
| hide | on, off | false |
| max | null, 2 | null |

### Methods

#### `ProgressList`: constructors and associated functions

- `pub fn new(tracker: ProgressTracker) -> ProgressList` - Shows the tasks of `tracker` in the default columns.

#### `ProgressList`: builder methods (take and return `self`)

- `pub fn columns(mut self, columns: impl Into<Rc<[ProgressColumn]>>) -> ProgressList` - In `columns`.
- `pub fn hide_finished(mut self, on: bool) -> ProgressList` - The tasks done or skipped left out where `on`.
- `pub fn max_visible(mut self, rows: usize) -> ProgressList` - At most `rows` rows of tasks.
- `pub fn spinner(mut self, frames: SpinnerFrames) -> ProgressList` - A running task's mark showing `frames`.
- `pub fn bar_shape(mut self, shape: ProgressBarShape) -> ProgressList` - Its bars drawn as `shape`.
- `pub fn max_bar_width(mut self, cells: Option<i32>) -> ProgressList` - Its bars `cells` wide at most.
- `pub fn byte_units(mut self, units: ByteUnits) -> ProgressList` - Bytes counted as `units` say.

### Also used in the code

- `rtwk::animation::ProgressTracker` (struct, `crates/rtwk-animation/src/progress_list.rs:293`) Tasks going on at once, added and removed as they come, each counting how far it has got: what a `ProgressList` shows, and what a program that is not an app reads to write a line per task (twk's `ProgressTracker`).
- `rtwk::animation::ProgressUnit` (enum, `crates/rtwk-animation/src/progress_list.rs:149`) What a `ProgressTask` counts, which says how its amounts and its rate are written.
  - variants: `Count`, `Bytes`
- `rtwk::binding::AppBinding` (struct, `crates/rtwk-widgets/src/binding/app_binding.rs:327`) The app a widget runs in (twk's `AppBinding`): one per running app, found from a build by `maybe_of`.
- `rtwk::animation::ProgressBarShape` (enum, `crates/rtwk-animation/src/progress_list.rs:159`) How a `ProgressList` draws the bar of a task.
  - variants: `Blocks`, `Line`
- `rtwk::style::SpinnerFrames` (struct, `crates/rtwk-widgets/src/style/glyphs.rs:18`) The frames of a spinner, shown in turn, each for `interval` (twk's `SpinnerFrames`, of `animation/spinner.dart`: defined here, where `Glyphs::spinner` needs it; the animation module's `Spinner` uses this type).
  - public fields: `frames: Cow<'static`, `interval: Duration`

## Skeleton, Skeletonize

Placeholders while content loads: blocks where it will be, or a subtree drawn as blocks in the layout it has, a shimmer sweeping over them as a SkeletonTheme says.

API:

- `rtwk::animation::Skeleton` (struct, `crates/rtwk-animation/src/skeleton.rs:280`) A block of placeholder cells where content will be: the `SkeletonTheme`'s glyph in its muted color, shimmering as a `Skeletonize` does (twk's `Skeleton`; twk also has it read as `loading`, which waits for the semantics ...
  - public fields: `width: Option<i32>`, `height: i32`, `lines: Option<i32>`, `last_line: f64`, `style: Option<Style>`
- `rtwk::animation::Skeletonize` (struct, `crates/rtwk-animation/src/skeleton.rs:206`) Shows `child` as a skeleton while `enabled`: laid out as it is, so the layout is the real one, each cell of it that shows something becoming the `SkeletonTheme`'s glyph in its muted color, the blanks left as they are ...
  - public fields: `enabled: bool`, `child: AnyWidget`, `shimmer: bool`

### Code

The gallery's Code tab at the initial options (route `#/animation/skeleton-skeletonize`, `crates/rtwk-gallery/src/pages/animation_progress.rs:157`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::animation::{Skeleton, SkeletonTheme, Skeletonize};
use rtwk::layout::{
    Border, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, MainAxisSize, SizedBox,
};
use rtwk::style::ThemeData;
use rtwk::text::Text;
use rtwk::{AnyWidget, column, row};

pub fn build(theme: &ThemeData) -> AnyWidget {
    let loading = true;
    let shimmer = true;
    SkeletonTheme::new(
        column![
            SizedBox::new(Some(40), None, None).child(
                BoxWidget::new()
                    .child(
                        Skeletonize::new(
                            loading,
                            column![
                                Text::new("user@twk").style(theme.primary.clone()),
                                Text::new("Maintains twk and its gallery."),
                                Text::new("Joined 19 October 2026").style(theme.muted.clone()),
                            ]
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Start),
                        )
                        .shimmer(shimmer),
                    )
                    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                    .padding(EdgeInsets::x(1)),
            ),
            SizedBox::new(Some(40), None, None).child(Skeleton::text().lines(3)),
            row![Skeleton::new().width(8), Skeleton::new().width(12)]
                .main_axis_size(MainAxisSize::Min)
                .gap(1),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(1),
    )
    .glyph("░")
    .band(8)
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| loading | on, off | true |
| shimmer | on, off | true |
| glyph | ░, ▒, ▓ | ░ |
| band | 8, 4, 16 | 8 |

### Methods

#### `Skeleton`: constructors and associated functions

- `pub fn new() -> Skeleton` - A block as wide as it may be, a row tall.
- `pub fn text() -> Skeleton` - A line of text as wide as it may be; more with `lines`, the last of them `last_line` (0.6 by default) of the width.

#### `Skeleton`: builder methods (take and return `self`)

- `pub fn width(mut self, width: i32) -> Skeleton` - This block `width` cells wide.
- `pub fn height(mut self, height: i32) -> Skeleton` - This block `height` rows tall.
- `pub fn lines(mut self, lines: i32) -> Skeleton` - This text of `lines` lines.
- `pub fn last_line(mut self, last_line: f64) -> Skeleton` - The last line of this text `last_line` of the width.
- `pub fn style(mut self, style: Style) -> Skeleton` - The glyph in `style` over the theme's.

#### `Skeletonize`: constructors and associated functions

- `pub fn new(enabled: bool, child: impl Into<AnyWidget>) -> Skeletonize` - Shows `child` as a skeleton while `enabled`, shimmering.

#### `Skeletonize`: builder methods (take and return `self`)

- `pub fn shimmer(mut self, shimmer: bool) -> Skeletonize` - Whether the skeleton shimmers.

### Also used in the code

- `rtwk::animation::SkeletonTheme` (struct, `crates/rtwk-animation/src/skeleton.rs:62`) How the `Skeleton`s and `Skeletonize`s below it look: the `glyph` their cells show, in `style`, and the shimmer that sweeps over them, a band `band` cells wide in `highlight` crossing them from left to right each ...
  - public fields: `period: Duration`, `glyph: Option<Str>`, `style: Option<Style>`, `highlight: Option<Style>`, `band: i32`, `child: AnyWidget`

## AnimationController

An animation of the page's own: a controller from 0 to 1 and back, a frame at a time, through a curve and a tween.

API:

- `rtwk::animation::AnimationController` (struct, `crates/rtwk-animation/src/controller.rs:154`) A value from `lower_bound` to `upper_bound` (0 and 1 by default) that runs over a `duration` once set going — `forward`, `reverse`, `animate_to`, `repeat` — a step each frame of its `Ticker`, and tells its listeners ...

### Code

The gallery's Code tab at the initial options (route `#/animation/animationcontroller`, `crates/rtwk-gallery/src/pages/animation_timeline.rs:85`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use std::time::Duration;

use rtwk::animation::{AnimationController, Curve, Curves};
use rtwk::controls::Button;
use rtwk::data::LineGauge;
use rtwk::layout::{Border, BorderType, BoxWidget, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::Theme;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget, row};

const DURATION: u64 = 2;
const REVERSE: bool = true;
const CURVE: Curve = Curves::EASE_IN_OUT;

#[derive(Debug, PartialEq, Eq)]
struct ControllerDemo;

#[derive(Default)]
struct ControllerState {
    controller: Option<AnimationController>,
}

impl StatefulWidget for ControllerDemo {
    type State = ControllerState;

    fn create_state(&self) -> ControllerState {
        ControllerState::default()
    }
}
impl_widget!(stateful eq ControllerDemo);

impl State for ControllerState {
    type Widget = ControllerDemo;
    type Msg = ();

    fn init_state(&mut self, _: &ControllerDemo, cx: &mut Cx<'_, Self>) {
        let controller = AnimationController::new(cx).with_duration(Duration::from_secs(DURATION));
        controller.listen(cx.id());
        controller.repeat(REVERSE, None, cx);
        self.controller = Some(controller);
    }

    fn update(&mut self, (): (), _: &ControllerDemo, cx: &mut Cx<'_, Self>) -> Rebuild {
        let controller = self.controller.as_ref().expect("mounted controller");
        if controller.is_animating() {
            controller.stop();
        } else {
            controller.repeat(REVERSE, None, cx);
        }
        Rebuild::Yes
    }

    fn dispose(&mut self) {
        if let Some(controller) = &self.controller {
            controller.dispose();
        }
    }

    fn build(&self, _: &ControllerDemo, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let controller = self.controller.as_ref().expect("mounted controller");
        let value = controller.value();
        let curved = CURVE.transform(value);
        let cells = (curved * 39.0).round().clamp(0.0, 39.0) as usize;
        let gauge = |value, label| {
            LineGauge::new(value)
                .label(label)
                .kind(BorderType::Heavy)
                .filled_style(theme.primary.clone())
                .unfilled_style(theme.border.clone())
        };
        SizedBox::new(Some(42), None, None)
            .child(
                column![
                    BoxWidget::new()
                        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                        .child(
                            Text::new(format!("{}●", " ".repeat(cells)))
                                .style(theme.primary.clone())
                        ),
                    column![
                        gauge(value, format!("value  {value:.2}")),
                        gauge(curved, format!("curved {curved:.2}")),
                    ]
                    .main_axis_size(MainAxisSize::Min),
                    row![
                        Button::new(if controller.is_animating() {
                            "Stop"
                        } else {
                            "Run"
                        })
                        .on_pressed(cx.send(())),
                        Text::new(controller.status().name()).style(theme.muted.clone()),
                    ]
                    .spacing(2),
                ]
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .spacing(1),
            )
            .into()
    }
}

pub fn build() -> AnyWidget {
    ControllerDemo.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| reverse | on, off | true |
| duration | 1, 2, 4 | 2 |

### Methods

#### `AnimationController`: constructors and associated functions

- `pub fn new<S: State>(cx: &mut Cx<'_, S>) -> AnimationController` - A controller of the state of `cx`, at its lower bound, 0 to 1, with no duration (give one with `with_duration` before running it).
- `pub fn with_bounds<S: State>(cx: &mut Cx<'_, S>, lower: f64, upper: f64) -> AnimationController` - A controller from `lower` to `upper`, at `lower`.

#### `AnimationController`: builder methods (take and return `self`)

- `pub fn with_duration(self, duration: Duration) -> AnimationController` - With `duration` as how long a run from one bound to the other takes.
- `pub fn with_value(self, value: f64) -> AnimationController` - With `value` as where it starts (clamped to the bounds), its status that of the value.

#### `AnimationController`: methods

- `pub fn duration(&self) -> Option<Duration>` - How long a run from one bound to the other takes; `None` until set.
- `pub fn set_duration(&self, d: Option<Duration>)` - Sets how long a run from one bound to the other takes (for the runs set going after).
- `pub fn reverse_duration(&self) -> Option<Duration>` - How long a run back takes (`reverse`); `None`: the `duration`.
- `pub fn set_reverse_duration(&self, d: Option<Duration>)` - Sets how long a run back takes.
- `pub fn lower_bound(&self) -> f64` - The least value.
- `pub fn upper_bound(&self) -> f64` - The greatest value.
- `pub fn value(&self) -> f64` - The value now.
- `pub fn status(&self) -> AnimationStatus` - Where it is.
- `pub fn is_animating(&self) -> bool` - Whether it runs: its ticker is active.
- `pub fn is_dismissed(&self) -> bool` - Whether it is at the lower bound: its status is `Dismissed`.
- `pub fn is_completed(&self) -> bool` - Whether it is at the upper bound: its status is `Completed`.
- `pub fn has_rebuilt_listener(&self, cx: &mut impl AnimationCx) -> bool` - Whether a listener already rebuilt in the current build scope.
- `pub fn ticker(&self) -> &Ticker` - Its ticker (whether it is muted, bound).
- `pub fn set_value(&self, v: f64, cx: &mut impl AnimationCx)` - Stops the run and puts the value at `v`, clamped to the bounds; the status that of the value (twk's `value` setter).
- `pub fn forward(&self, from: Option<f64>, cx: &mut impl AnimationCx)` - Runs to the upper bound in the part of the duration the way left is of the whole, from `from` when given.
- `pub fn reverse(&self, from: Option<f64>, cx: &mut impl AnimationCx)` - Runs back to the lower bound in the part of the reverse duration (else the duration) the way left is of the whole, from `from` when given.
- `pub fn toggle(&self, from: Option<f64>, cx: &mut impl AnimationCx)` - Runs back to the lower bound if it is at the upper one or running towards it (`reverse`), else forward to the upper one (`forward`), from `from` when given.
- `pub fn animate_to(&self, target: f64, duration: Option<Duration>, curve: Curve, cx: &mut impl AnimationCx)` - Runs to `target` (clamped) along `curve`: in `duration`, or in the part of the controller's duration the way is of the whole.
- `pub fn repeat(&self, reverse: bool, period: Option<Duration>, cx: &mut impl AnimationCx)` - Runs from the lower bound to the upper one, again and again, each round in `period` (else the duration) — and back again in as long when `reverse` — from the share of the way it is at; until stopped.
- `pub fn stop(&self)` - Stops the run where it is; the status stays as it was.
- `pub fn reset(&self, cx: &mut impl AnimationCx)` - Puts it back at the lower bound, stopped.
- `pub fn listen(&self, element: Eid)` - Rebuilds `element` whenever the value changes, until `unlisten`ed or unmounted: once however often asked.
- `pub fn unlisten(&self, element: Eid)` - No longer rebuilds `element`.
- `pub fn add_listener(&self, h: Handler<()>)` - Calls `h` whenever the value changes (twk's `addListener`).
- `pub fn remove_listener(&self, h: &Handler<()>)` - Calls `h` no more (one of the handlers equal to it).
- `pub fn add_status_listener(&self, h: AnimationStatusListener)` - Calls `h` with each new status (twk's `addStatusListener`).
- `pub fn remove_status_listener(&self, h: &AnimationStatusListener)` - Calls `h` no more (one of the handlers equal to it).
- `pub fn listeners(&self) -> usize` - How many listeners it has (elements and handlers).
- `pub fn dispose(&self)` - Stops it for good: its ticker disposed, its listeners let go.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::data::LineGauge` (struct, `crates/rtwk-data/src/gauge.rs:215`) `ratio` as a line after `label` (or with `show_percentage` the ratio as a percentage) and a blank: the part from the left filled in `filled_style`, the rest in `unfilled_style`, to the nearest half cell where `kind` has ...
  - public fields: `ratio: f64`, `label: Option<Str>`, `show_percentage: bool`, `kind: Option<BorderType>`, `style: Style`, `filled_style: Style`, `unfilled_style: Style`, `segmented: bool`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## AnimatedBuilder

A builder run again at each tick of an animation, the part that does not change built once and passed in as its child.

API:

- `rtwk::animation::AnimatedBuilder` (struct, `crates/rtwk-animation/src/builder.rs:31`) Shows what `builder` returns, built again whenever `animation`'s value changes — as an `AnimationController` does each frame it runs — and nothing else of its parent (twk's `AnimatedBuilder`, a `ListenableBuilder` of ...
  - public fields: `animation: AnimationController`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedbuilder`, `crates/rtwk-gallery/src/pages/animation_timeline.rs:105`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const DURATION: u64 = 2;
const REVERSE: bool = true;
const CURVE: Curve = Curves::EASE_IN_OUT;
// Store and dispose the controller with the owning State.
let controller = AnimationController::new(cx).with_duration(Duration::from_secs(DURATION));
controller.repeat(REVERSE, None, cx);
let animated = AnimatedBuilder::new(controller, |controller, child| {
    let x = (CURVE.transform(controller.value()) * 35.0).round() as i32;
    row![
        SizedBox::new(Some(x.max(0)), None, None),
        child.expect("the stable child").clone(),
    ]
    .into()
})
.child(
    BoxWidget::new()
        .style(theme.selection.clone())
        .padding(EdgeInsets::x(1))
        .child(Text::new("twk")),
);
SizedBox::new(Some(42), None, None).child(
    BoxWidget::new()
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .child(animated),
)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| reverse | on, off | true |
| duration | 1, 2, 4 | 2 |

### Methods

#### `AnimatedBuilder`: constructors and associated functions

- `pub fn new(animation: AnimationController, builder: impl Fn(&AnimationController, Option<&AnyWidget>) -> AnyWidget + 'static) -> AnimatedBuilder` - Builds `builder` of `animation`, again whenever it moves.

#### `AnimatedBuilder`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> AnimatedBuilder` - With `child` handed to the builder.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::animation::AnimationController` (struct, `crates/rtwk-animation/src/controller.rs:154`) A value from `lower_bound` to `upper_bound` (0 and 1 by default) that runs over a `duration` once set going — `forward`, `reverse`, `animate_to`, `repeat` — a step each frame of its `Ticker`, and tells its listeners ...

## AnimatedAlign, AnimatedPadding

A child that runs to the place its alignment or its padding gives it, a cell at a time.

API:

- `rtwk::animation::AnimatedAlign` (struct, `crates/rtwk-animation/src/implicit.rs:349`) Puts its child at `alignment` of the room it gets, moving it across to a new one a cell at a time (twk's `AnimatedAlign`): as big as it may be.
  - public fields: `alignment: Alignment`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`, `child: Option<AnyWidget>`
- `rtwk::animation::AnimatedPadding` (struct, `crates/rtwk-animation/src/implicit.rs:288`) Insets its child by `padding`, running each side to a new one a cell at a time (twk's `AnimatedPadding`).
  - public fields: `padding: EdgeInsets`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedalign-animatedpadding`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:62`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 4) % 4;
let curve = Curves::EASE_IN_OUT;

{
    let alignment = [
        Alignment::CENTER_LEFT,
        Alignment::CENTER,
        Alignment::CENTER_RIGHT,
        Alignment::BOTTOM_RIGHT,
    ][selected];
    SizedBox::fixed(34, 7)
        .child(
            BoxWidget::new()
                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                .child(
                    column![
                        Expanded::new(
                            1,
                            AnimatedAlign::new(
                                alignment,
                                duration,
                                Text::new("◆ aligned").style(theme.primary.clone()),
                            )
                            .curve(curve.clone()),
                        ),
                        AnimatedPadding::new(
                            EdgeInsets::only(alignment.x * 10 + 10, 0, 0, 0),
                            duration,
                            Text::new("◇ padded").style(theme.accent.clone()),
                        )
                        .curve(curve),
                    ]
                    .cross_axis_alignment(CrossAxisAlignment::Start),
                ),
        )
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| alignment | centerLeft, center, centerRight, bottomRight | centerLeft |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| cycle | on, off | true |

### Methods

#### `AnimatedAlign`: constructors and associated functions

- `pub fn new(alignment: Alignment, duration: Duration, child: impl Into<AnyWidget>) -> AnimatedAlign` - `child` at `alignment`, a change run over `duration`.

#### `AnimatedPadding`: constructors and associated functions

- `pub fn new(padding: EdgeInsets, duration: Duration, child: impl Into<AnyWidget>) -> AnimatedPadding` - `child` inset by `padding`, a change run over `duration`.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).

## AnimatedPositioned

A child pinned in a stack that runs to the edges it is pinned to.

API:

- `rtwk::animation::AnimatedPositioned` (struct, `crates/rtwk-animation/src/implicit.rs:737`) Pins `child` to the edges of the `Stack` it is in as `Positioned` does, each edge and length running to its new place a cell at a time; one given where there was none, or taken away, at once.
  - public fields: `child: AnyWidget`, `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedpositioned`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:78`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 5) % 5;
let curve = Curves::EASE_IN_OUT;

{
    let (left, top, width, height) = [
        (1, 1, 12, 1),
        (1, 5, 12, 1),
        (21, 5, 12, 1),
        (21, 1, 12, 1),
        (1, 1, 32, 5),
    ][selected];
    let pinned = AnimatedPositioned::new(
        BoxWidget::new()
            .style(theme.selection.clone())
            .child(Center::new(Text::new("▣ pinned"))),
        duration,
    )
    .left(left)
    .top(top)
    .width(width)
    .height(height)
    .curve(curve);
    SizedBox::fixed(34, 7)
        .child(
            Stack::new()
                .fit(StackFit::Expand)
                .child(
                    BoxWidget::new()
                        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded)),
                )
                .child(pinned),
        )
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| pin | topLeft, bottomLeft, bottomRight, topRight, fill | topLeft |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| cycle | on, off | true |

### Methods

#### `AnimatedPositioned`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>, duration: Duration) -> AnimatedPositioned` - `child` pinned nowhere yet (give it edges and lengths), a change run over `duration`.

#### `AnimatedPositioned`: builder methods (take and return `self`)

- `pub fn left(mut self, v: i32) -> Self` - Columns from the stack's left edge.
- `pub fn top(mut self, v: i32) -> Self` - Rows from the stack's top edge.
- `pub fn right(mut self, v: i32) -> Self` - Columns from the stack's right edge.
- `pub fn bottom(mut self, v: i32) -> Self` - Rows from the stack's bottom edge.
- `pub fn width(mut self, v: i32) -> Self` - The child's width.
- `pub fn height(mut self, v: i32) -> Self` - The child's height.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::StackFit` (enum, `crates/rtwk-render/src/objects/stack.rs:23`) How a `RenderStack` lays out the children it places by its alignment.
  - variants: `Loose`, `Expand`, `Passthrough`

## AnimatedBox

A box whose size, padding, background and border colors run to the new ones; a border of another type switches halfway.

API:

- `rtwk::animation::AnimatedBox` (struct, `crates/rtwk-animation/src/implicit.rs:1057`) A `BoxWidget` around `child` made `width` wide and `height` tall where they are given (`SizedBox`), its `style`, the styles of its `border`, `padding` and size running to the new ones: colors as `StyleTween` has them, ...
  - public fields: `style: Option<Style>`, `border: Option<Border>`, `titles: Vec<BorderTitle>`, `padding: EdgeInsets`, `width: Option<i32>`, `height: Option<i32>`, `child: Option<AnyWidget>`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedbox`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:94`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 2) % 2;
let curve = Curves::EASE_IN_OUT;

{
    let big = selected != 0;
    let (width, height, fill, border, edge, padding) = if big {
        (
            34,
            7,
            theme.selection.clone(),
            BorderType::Double,
            theme.accent.clone(),
            EdgeInsets::only(4, 1, 0, 0),
        )
    } else {
        (
            16,
            3,
            theme.surface.clone(),
            BorderType::Rounded,
            theme.border.clone(),
            EdgeInsets::default(),
        )
    };
    let animated = AnimatedBox::new(duration)
        .width(width)
        .height(height)
        .style(fill)
        .border(Border::all(edge).kind(border))
        .padding(padding)
        .curve(curve.clone())
        .child(Text::new("a box"));
    let (room_width, room_height) = if curve == Curves::ELASTIC_OUT {
        (39, 8)
    } else {
        (34, 7)
    };
    SizedBox::fixed(room_width, room_height)
        .child(Align::new(Alignment::TOP_LEFT, animated))
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| big | on, off | false |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| cycle | on, off | true |

### Methods

#### `AnimatedBox`: constructors and associated functions

- `pub fn new(duration: Duration) -> AnimatedBox` - An empty box that fills nothing, a change run over `duration`.

#### `AnimatedBox`: builder methods (take and return `self`)

- `pub fn style(mut self, style: Style) -> Self` - Filled with `style`.
- `pub fn border(mut self, border: Border) -> Self` - Lined by `border`.
- `pub fn titles(mut self, titles: impl IntoIterator<Item = BorderTitle>) -> Self` - With `titles` on its edges.
- `pub fn padding(mut self, padding: EdgeInsets) -> Self` - `padding` cells between the border and the child.
- `pub fn width(mut self, width: i32) -> Self` - `width` cells wide.
- `pub fn height(mut self, height: i32) -> Self` - `height` cells tall.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - With `child` inside.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::layout::Align` (struct, `crates/rtwk-widgets/src/layout/basic.rs:103`) Places its child inside itself as `alignment` says (twk's `Align`, over a `RenderPositionedBox`).
  - public fields: `alignment: Alignment`, `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`

## AnimatedOpacity, Opacity

A child shown at an opacity, mixed cell by cell with what is under it; the one on the left runs to the opacity chosen.

API:

- `rtwk::animation::AnimatedOpacity` (struct, `crates/rtwk-animation/src/implicit.rs:893`) Shows `child` at `opacity` (`Opacity`), running to the new one (twk's `AnimatedOpacity`).
  - public fields: `opacity: f64`, `child: Option<AnyWidget>`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`
- `rtwk::animation::Opacity` (struct, `crates/rtwk-animation/src/switcher.rs:675`) Shows `child` at `opacity`, from 0 (not at all: what is under it shows) to 1 (as it paints), mixed with what is under it cell by cell: the child's background over the one under it as by alpha (a pen's `bg_alpha`), then ...
  - public fields: `opacity: f64`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedopacity-opacity`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:101`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (2 + step % 5) % 5;

{
    let opacity = [0.0, 0.25, 0.5, 0.75, 1.0][selected];
    let over = |child: AnyWidget, name| {
        column![
            BoxWidget::new()
                .style(theme.surface.clone())
                .padding(EdgeInsets::all(1))
                .child(
                    Stack::new()
                        .child(Text::new("under it, under it").style(theme.muted.clone()))
                        .child(child),
                ),
            Text::new(name).style(theme.muted.clone()),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
    };
    let shown = BoxWidget::new()
        .style(theme.selection.clone())
        .padding(EdgeInsets::x(1))
        .child(Text::new("the child").style(theme.text.clone()));
    row![
        over(
            AnimatedOpacity::new(opacity, duration)
                .child(shown.clone())
                .into(),
            "AnimatedOpacity",
        ),
        over(Opacity::new(opacity).child(shown).into(), "Opacity"),
    ]
    .main_axis_size(MainAxisSize::Min)
    .spacing(2)
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| opacity | 0, 0.25, 0.5, 0.75, 1 | 0.5 |
| cycle | on, off | true |

### Methods

#### `AnimatedOpacity`: constructors and associated functions

- `pub fn new(opacity: f64, duration: Duration) -> AnimatedOpacity` - No child yet, shown at `opacity`, a change run over `duration`.

#### `AnimatedOpacity`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - With `child`.

#### `Opacity`: constructors and associated functions

- `pub fn new(opacity: f64) -> Opacity` - No child yet, shown at `opacity`.

#### `Opacity`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Opacity` - With `child`.

### Also used in the code

- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`

## AnimatedDefaultTextStyle

The style of the text below it, its colors running to the new ones.

API:

- `rtwk::animation::AnimatedDefaultTextStyle` (struct, `crates/rtwk-animation/src/implicit.rs:974`) The style the `Text`s below it are drawn in (`DefaultTextStyle`), its colors running to the new ones (`StyleTween`) (twk's `AnimatedDefaultTextStyle`).
  - public fields: `style: Style`, `child: AnyWidget`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animateddefaulttextstyle`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:108`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 3) % 3;
let bold = false;

{
    let mut style = [&theme.primary, &theme.error, &theme.success][selected].clone();
    if bold {
        style = style.merge(&Style::new().attrs(Attrs::BOLD));
    }
    AnimatedDefaultTextStyle::new(
        style,
        duration,
        column![
            Text::new("Text in the style above it,"),
            Text::new("and a second Text, in step."),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| style | primary, error, success | primary |
| bold | on, off | false |
| cycle | on, off | true |

### Methods

#### `AnimatedDefaultTextStyle`: constructors and associated functions

- `pub fn new(style: Style, duration: Duration, child: impl Into<AnyWidget>) -> AnimatedDefaultTextStyle` - Draws the texts of `child` in `style`, running to a new one over `duration`.

## AnimatedSize

As big as its child, running to the new size when the child grows or shrinks.

API:

- `rtwk::animation::AnimatedSize` (struct, `crates/rtwk-animation/src/implicit.rs:1230`) As big as `child`, and when the child's size changes, running to the new size from the one it shows in `duration` along `curve`, a cell at a time: the child laid out at its new size, placed by `alignment` and cut off ...
  - public fields: `duration: Duration`, `curve: Curve`, `alignment: Alignment`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedsize`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:119`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 3) % 3;
let curve = Curves::EASE_IN_OUT;
let alignment = Alignment::TOP_LEFT;

{
    let lines = [
        "one line",
        "a second, longer line",
        "a third",
        "a fourth, the longest of them",
        "and a fifth",
    ];
    let count = [1, 3, 5][selected];
    let animated = AnimatedSize::new(duration)
        .alignment(alignment)
        .curve(curve.clone())
        .child(
            column![]
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children(
                    lines[..count]
                        .iter()
                        .copied()
                        .map(|line| Text::new(line).into()),
                ),
        );
    let (width, height) = if curve == Curves::ELASTIC_OUT {
        (36, 8)
    } else {
        (31, 7)
    };
    SizedBox::fixed(width, height)
        .child(Align::new(
            Alignment::TOP_LEFT,
            BoxWidget::new()
                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                .child(animated),
        ))
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| lines | 1, 3, 5 | 1 |
| alignment | topLeft, center, bottomRight | topLeft |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| cycle | on, off | true |

### Methods

#### `AnimatedSize`: constructors and associated functions

- `pub fn new(duration: Duration) -> AnimatedSize` - No child yet, following its size over `duration`.

#### `AnimatedSize`: builder methods (take and return `self`)

- `pub fn curve(mut self, curve: Curve) -> Self` - Along `curve`.
- `pub fn alignment(mut self, alignment: Alignment) -> Self` - The child placed at `alignment` while it runs.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - With `child`.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::layout::Align` (struct, `crates/rtwk-widgets/src/layout/basic.rs:103`) Places its child inside itself as `alignment` says (twk's `Align`, over a `RenderPositionedBox`).
  - public fields: `alignment: Alignment`, `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## TweenAnimationBuilder

Builds with a value that runs to the one it is given, through the tweens it makes.

API:

- `rtwk::animation::TweenAnimationBuilder` (struct, `crates/rtwk-animation/src/implicit.rs:515`) Builds `builder` of a value that runs to `value` whenever it changes — from `from` at first when given — over its duration, along its curve (twk's `TweenAnimationBuilder`).
  - public fields: `value: T`, `from: Option<T>`, `duration: Duration`, `curve: Curve`, `on_end: Option<Handler<()>>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/tweenanimationbuilder`, `crates/rtwk-gallery/src/pages/animation_timeline.rs:113`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const TARGET: f64 = 0.5;
const CURVE: Curve = Curves::EASE_IN_OUT;
const CYCLE: bool = true;
// With cycle enabled, update the target option every 1,200 ms.
let target = if CYCLE { cycle_target } else { TARGET };
let style = theme.accent.merge(&Style { bg: theme.surface.bg, ..Style::new() });
TweenAnimationBuilder::new(target, Duration::from_millis(600), move |value, _| {
    let bar = ProgressBar::new(Some(*value)).style(style.clone());
    let text = Text::new(format!("{:.0}%", (value * 100.0).round()));
    row![
        SizedBox::new(Some(30), None, Some(bar.into())),
        SizedBox::new(Some(4), None, Some(text.into())),
    ]
    .main_axis_size(MainAxisSize::Min)
    .spacing(1)
    .into()
})
.from(0.0)
.curve(CURVE)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| value | 0, 0.25, 0.5, 0.75, 1 | 0.5 |
| curve | linear, ease, easeIn, easeOut, easeInOut, fastOutSlowIn, decelerate, bounceOut, elasticOut | easeInOut |
| cycle | on, off | true |

### Methods

#### `TweenAnimationBuilder`: constructors and associated functions

- `pub fn new(value: T, duration: Duration, builder: impl Fn(&T, Option<&AnyWidget>) -> AnyWidget + 'static) -> TweenAnimationBuilder<T>` - Builds `builder` of a value running to `value` over `duration`.

#### `TweenAnimationBuilder`: builder methods (take and return `self`)

- `pub fn from(mut self, from: T) -> Self` - Its first run from `from`.
- `pub fn curve(mut self, curve: Curve) -> Self` - Along `curve`.
- `pub fn on_end(mut self, h: Handler<()>) -> Self` - Calls `h` once a run has ended.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - With `child` handed to the builder.

### Also used in the code

- `rtwk::animation::Curves` (struct, `crates/rtwk-animation/src/curves.rs:388`) The usual curves (twk's `Curves`).
- `rtwk::animation::ProgressBar` (struct, `crates/rtwk-animation/src/progress_bar.rs:43`) A bar filled to `value`, from 0 (empty) to 1 (full), in eighths of a cell — braille halves where block elements are two columns wide, the theme's shades where its glyphs have no partial blocks — as wide as it may be and ...
  - public fields: `value: Option<f64>`, `style: Option<Style>`, `semantics_label: Option<Str>`

## AnimatedSwitcher

A child that gives way to another as the transition shows them: faded, slid or dissolved.

API:

- `rtwk::animation::AnimatedSwitcher` (struct, `crates/rtwk-animation/src/switcher.rs:82`) Shows `child`, and when given one that does not update it (of another type, or with another key: `Text::new(n.to_string()).into().with_key(Key::value(n))`), has the new one come in as the old one goes out, as ...
  - public fields: `child: Option<AnyWidget>`, `duration: Duration`, `reverse_duration: Option<Duration>`, `switch_in_curve: Curve`, `switch_out_curve: Curve`, `transition: SwitcherTransition`, `alignment: Alignment`

### Code

The gallery's Code tab at the initial options (route `#/animation/animatedswitcher`, `crates/rtwk-gallery/src/pages/animation_implicit.rs:136`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let duration = Duration::from_millis(600);
// Rebuild at 1.2 s intervals from when cycling starts.
let step = usize::try_from(elapsed_ms / 1_200).unwrap_or(0);
let selected = (0 + step % 3) % 3;
let transition = SwitcherTransition::FADE;

{
    let text = ["the first child", "and now the second one", "a third"][selected];
    let child = AnyWidget::from(SizedBox::fixed(32, 3).child(Center::new(Text::new(text))))
        .with_key(Key::value(text));
    SizedBox::fixed(34, 5)
        .child(
            BoxWidget::new()
                .style(theme.surface.clone())
                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                .child(
                    AnimatedSwitcher::new(duration)
                        .transition(transition)
                        .child(child),
                ),
        )
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| child | first, second, third | first |
| transition | fade, slide, slide right, slide up, slide down, dissolve | fade |
| cycle | on, off | true |

### Methods

#### `AnimatedSwitcher`: constructors and associated functions

- `pub fn new(duration: Duration) -> AnimatedSwitcher` - No child yet, a new one coming in over `duration` with a fade.

#### `AnimatedSwitcher`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> AnimatedSwitcher` - Showing `child`.
- `pub fn reverse_duration(mut self, d: Duration) -> AnimatedSwitcher` - An old child going out over `d`.
- `pub fn switch_in_curve(mut self, c: Curve) -> AnimatedSwitcher` - A new child coming in along `c`.
- `pub fn switch_out_curve(mut self, c: Curve) -> AnimatedSwitcher` - An old child going out along `c`.
- `pub fn transition(mut self, t: SwitcherTransition) -> AnimatedSwitcher` - The children coming in and going out with `t`.
- `pub fn alignment(mut self, a: Alignment) -> AnimatedSwitcher` - A child smaller than the biggest at `a`.

### Also used in the code

- `rtwk::animation::SwitcherTransition` (enum, `crates/rtwk-animation/src/switcher.rs:253`) How a child of an `AnimatedSwitcher` comes in and goes out: `build` wraps it in what shows as much of it as `shown` says.
  - variants: `Fade`, `Dissolve`, `Slide`, `Custom`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## Dissolve, FractionalTranslation

What the transitions are made of: a share of the cells of a child, and a child moved a share of its size.

API:

- `rtwk::animation::Dissolve` (struct, `crates/rtwk-animation/src/switcher.rs:918`) Shows `shown` of the cells of `child`, from 0 (none: what is under it shows) to 1 (all), taken in a scattered order of the screen's cells, the same for every `Dissolve`: one with `inverse` takes them from the end of ...
  - public fields: `shown: f64`, `inverse: bool`, `child: Option<AnyWidget>`
- `rtwk::animation::FractionalTranslation` (struct, `crates/rtwk-animation/src/switcher.rs:1099`) Shows `child` moved `x` times its width to the right and `y` times its height down (left and up where negative), to the nearest cell, and cut off at the edges of its own place: as big as the child, which takes the mouse ...
  - public fields: `x: f64`, `y: f64`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/dissolve-fractionaltranslation`, `crates/rtwk-gallery/src/pages/animation_timeline.rs:128`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const SHARE: f64 = 0.5;
const RUN: bool = true;
const INVERSE: bool = false;
// In a running preview, value comes from a repeating controller.
let share = if RUN { controller.value() } else { SHARE };
let shown = BoxWidget::new()
    .style(theme.selection.clone())
    .padding(EdgeInsets::x(1))
    .child(Text::new("a child, its cells shown or moved"));
let shifted = share - 1.0;
let qualifier = if INVERSE { ", inverse: true" } else { "" };
column![
    Text::new(format!("Dissolve(shown: {share:.2}{qualifier})")).style(theme.muted.clone()),
    Dissolve::new(share).inverse(INVERSE).child(shown.clone()),
    SizedBox::new(None, Some(1), None),
    Text::new(format!("FractionalTranslation(x: {shifted:.2})")).style(theme.muted.clone()),
    FractionalTranslation::new(shifted, 0.0).child(shown),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| share | 0, 0.25, 0.5, 0.75, 1 | 0.5 |
| run | on, off | true |
| inverse | on, off | false |

### Methods

#### `Dissolve`: constructors and associated functions

- `pub fn new(shown: f64) -> Dissolve` - No child yet, `shown` of its cells shown from the start of their order.

#### `Dissolve`: builder methods (take and return `self`)

- `pub fn inverse(mut self, inverse: bool) -> Dissolve` - Taking the cells from the end of their order.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Dissolve` - With `child`.

#### `FractionalTranslation`: constructors and associated functions

- `pub fn new(x: f64, y: f64) -> FractionalTranslation` - No child yet, moved by `x` times its width and `y` times its height.

#### `FractionalTranslation`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> FractionalTranslation` - With `child`.

## Effect

A child shown through an effect over its cells, run on the app's clock: recolored, swept, dissolved, typed or glitched in, while the child repaints underneath.

API:

- `rtwk::animation::Effect` (struct, `crates/rtwk-animation/src/effect.rs:76`) Shows `child` through `effect`, a function over the cells of the child's rectangle: each shows what the child painted there, what was under it, or what the effect makes of them.
  - public fields: `effect: Option<CellEffect>`, `on_end: Option<Handler<()>>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/animation/effect`, `crates/rtwk-gallery/src/pages/animation_timeline.rs:155`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const CYCLE: bool = true;
let selected = CellEffect::sweep().with_duration(Duration::from_millis(600));
// With cycle enabled, the caller supplies the next effect every 1,200 ms.
let effect = if CYCLE { cycle_effect } else { selected };
let subtree = SizedBox::new(Some(32), Some(3), None).child(column![
    Text::new("Effects over a subtree").style(theme.primary.clone()),
    row![Spinner::new().style(theme.accent.clone()), Text::new("repainting underneath")]
        .spacing(1),
    Text::new("the cells it paints, filtered").style(theme.muted.clone()),
]
.cross_axis_alignment(CrossAxisAlignment::Start));
let shown = AnyWidget::from(Effect::new(effect).child(subtree)).with_key(runs);
column![
    BoxWidget::new()
        .style(theme.surface.clone())
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .child(shown),
    row![
        Button::new("Again").on_pressed(on_again),
        Text::new("a new key runs it from its start").style(theme.muted.clone()),
    ]
    .main_axis_size(MainAxisSize::Min)
    .spacing(2),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| effect | sweep, dissolve, typewriter, glitch, dim, desaturate, tint, scanlines, grade, sweep down, glitch | sweep |
| cycle | on, off | true |

### Methods

#### `Effect`: constructors and associated functions

- `pub fn new(effect: impl Into<Option<CellEffect>>) -> Effect` - Shows its child through `effect`, run once from its start.

#### `Effect`: builder methods (take and return `self`)

- `pub fn on_end(mut self, h: Handler<()>) -> Effect` - Calls `h` when a run of the effect ends.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Effect` - Shows `child` through the effect.

### Also used in the code

- `rtwk::animation::CellEffect` (struct, `crates/rtwk-animation/src/effect.rs:652`) An effect over the cells of the child of an `Effect`: what it does to them at each point of its run, from its start to its end in `duration` (`at`) (twk's `CellEffect`).
- `rtwk::animation::Spinner` (struct, `crates/rtwk-animation/src/spinner.rs:47`) The frames of `frames` — those of the theme's `Glyphs::spinner` unless it is given some: dots in the Unicode glyphs, a turning line in the ASCII ones — in turn, a frame each interval of the frames' clock: the frame of ...
  - public fields: `frames: Option<SpinnerFrames>`, `style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
