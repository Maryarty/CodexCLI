<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Animation

Spinners, progress, and widgets that animate as their values change.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Spinner

Frames shown in turn while something goes on; spinners of the same frames step together.

- `rtwk::animation::Spinner` (struct) in `crates/rtwk-animation/src/spinner.rs:47`: The frames of frames — those of the theme's Glyphs::spinner unless it is given some: dots in the Unicode glyphs, a turning line in the ASCII ones — in turn, a frame each interval of the frames' clock: the frame of the step of the clock the frame is in, so that spinners of the same frames show the same one and step in the same frames (twk's `Spinner`).

### Code

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
        .spacing(1)
        .children(tasks.map(|task| {
            row![
                Spinner::new()
                    .frames(SpinnerFrames::DOTS)
                    .style(theme.primary.clone()),
                Text::new(task),
            ]
            .main_axis_size(MainAxisSize::Min)
            .spacing(1)
            .into()
        }))
        .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| frames | dots, line, arc | dots |
| style | primary, accent, success, warning, info | primary |

## TickerMode

Whether the animations below tick: off, they ask for no frames and stand still, and go on when it is on again; Offstage and Visibility turn it off for what they hide.

- `rtwk::animation::TickerMode` (struct) in `crates/rtwk-framework/src/ticker.rs:891`: Whether the Tickers below tick (twk's `TickerMode`): what is not shown asks for no frames.

### Code

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
        .spacing(1),
    )
    .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| enabled | false, true | true |

## ProgressBar

A bar filled as far as its value, in eighths of a cell; a block going to and fro while the value is not known.

- `rtwk::animation::ProgressBar` (struct) in `crates/rtwk-animation/src/progress_bar.rs:43`: A bar filled to value, from 0 (empty) to 1 (full), in eighths of a cell — braille halves where block elements are two columns wide, the theme's shades where its glyphs have no partial blocks — as wide as it may be and every row it is given (twk's `ProgressBar`).

### Code

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
    .spacing(1);
    if let Some(value) = value {
        row = row.child(Text::new(format!("{value:>3}%")));
    }
    row.into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| value known | false, true | true |
| value | 0%, 1%, 2%, 3%, 4%, 5%, 6%, 7%, 8%, 9%, 10%, 11%, 12%, 13%, 14%, 15%, 16%, 17%, 18%, 19%, 20%, 21%, 22%, 23%, 24%, 25%, 26%, 27%, 28%, 29%, 30%, 31%, 32%, 33%, 34%, 35%, 36%, 37%, 38%, 39%, 40%, 41%, 42%, 43%, 44%, 45%, 46%, 47%, 48%, 49%, 50%, 51%, 52%, 53%, 54%, 55%, 56%, 57%, 58%, 59%, 60%, 61%, 62%, 63%, 64%, 65%, 66%, 67%, 68%, 69%, 70%, 71%, 72%, 73%, 74%, 75%, 76%, 77%, 78%, 79%, 80%, 81%, 82%, 83%, 84%, 85%, 86%, 87%, 88%, 89%, 90%, 91%, 92%, 93%, 94%, 95%, 96%, 97%, 98%, 99%, 100% | 33% |
| style | primary, accent, success, warning, info | success |

## ProgressList

Many tasks at once, a row each: its mark, a bar, how much is done, its rate and the time it has left, from a ProgressTracker the work counts on.

- `rtwk::animation::ProgressList` (struct) in `crates/rtwk-animation/src/progress_list.rs:1103`: The tasks of a tracker, a row each, with the columns it is given: the mark of each task's TaskItemStatus, its label, a bar as a ProgressBar draws it — sweeping while the task runs without a total — and how far it has got, how fast it goes and the time it has left (twk's `ProgressList`):

### Code

```rust
// Drive sample downloads at 300 ms intervals on the app's frame clock.
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rtwk::animation::{
    ProgressBarShape, ProgressColumn, ProgressList, ProgressTracker, ProgressUnit,
};
use rtwk::binding::AppBinding;
use rtwk::clock::Clock;
use rtwk::layout::SizedBox;
use rtwk::style::SpinnerFrames;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

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

| option | values | starts at |
| --- | --- | --- |
| columns | default, count, elapsed | default |
| bar | blocks, line | blocks |
| spinner | dots, line, arc | dots |
| hide finished | false, true | false |
| at most | all, 2 | all |

## Skeleton, Skeletonize

Placeholders while content loads: blocks where it will be, or a subtree drawn as blocks in the layout it has, a shimmer sweeping over them as a SkeletonTheme says.

- `rtwk::animation::Skeleton` (struct) in `crates/rtwk-animation/src/skeleton.rs:280`: A block of placeholder cells where content will be: the SkeletonTheme's glyph in its muted color, shimmering as a Skeletonize does (twk's `Skeleton`; twk also has it read as `loading`, which waits for the semantics module).
- `rtwk::animation::Skeletonize` (struct) in `crates/rtwk-animation/src/skeleton.rs:206`: Shows child as a skeleton while enabled: laid out as it is, so the layout is the real one, each cell of it that shows something becoming the SkeletonTheme's glyph in its muted color, the blanks left as they are (twk's `Skeletonize`).

### Code

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
                                Text::new("user@rtwk").style(theme.primary.clone()),
                                Text::new("Maintains rtwk and its gallery."),
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
                .spacing(1),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1),
    )
    .glyph("░")
    .band(8)
    .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| loading | false, true | true |
| shimmer | false, true | true |
| glyph | ░, ▒, ▓ | ░ |
| band | 8, 4, 16 | 8 |

## AnimationController

An animation of the page's own: a controller from 0 to 1 and back, a frame at a time, through a curve and a tween.

- `rtwk::animation::AnimationController` (struct) in `crates/rtwk-animation/src/controller.rs:154`: A value from lower_bound to upper_bound (0 and 1 by default) that runs over a duration once set going — forward, reverse, animate_to, repeat — a step each frame of its Ticker, and tells its listeners (twk's `AnimationController`).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| reverse | false, true | true |
| duration | 1 s, 2 s, 4 s | 2 s |

## AnimatedBuilder

A builder run again at each tick of an animation, the part that does not change built once and passed in as its child.

- `rtwk::animation::AnimatedBuilder` (struct) in `crates/rtwk-animation/src/builder.rs:31`: Shows what `builder` returns, built again whenever `animation`'s value changes — as an AnimationController does each frame it runs — and nothing else of its parent (twk's `AnimatedBuilder`, a `ListenableBuilder` of the animation).

### Code

```rust
fn pulse<S: State>(cx: &mut Cx<'_, S>, theme: &ThemeData) -> SizedBox {
    const DURATION: u64 = 2;
    const REVERSE: bool = true;
    const CURVE: Curve = Curves::EASE_IN_OUT;
    // Store and dispose the controller with the owning State.
    let controller = AnimationController::new(cx).with_duration(Duration::from_secs(DURATION));
    controller.repeat(REVERSE, None, cx);
    let animated = AnimatedBuilder::new(controller, |controller, child| {
        let x = (CURVE.transform(controller.value()) * 34.0).round() as i32;
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
            .child(Text::new("rtwk")),
    );
    SizedBox::new(Some(42), None, None).child(
        BoxWidget::new()
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .child(animated),
    )
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| reverse | false, true | true |
| duration | 1 s, 2 s, 4 s | 2 s |

## AnimatedAlign, AnimatedPadding

A child that runs to the place its alignment or its padding gives it, a cell at a time.

- `rtwk::animation::AnimatedAlign` (struct) in `crates/rtwk-animation/src/implicit.rs:349`: Puts its child at alignment of the room it gets, moving it across to a new one a cell at a time (twk's `AnimatedAlign`): as big as it may be.
- `rtwk::animation::AnimatedPadding` (struct) in `crates/rtwk-animation/src/implicit.rs:288`: Insets its child by padding, running each side to a new one a cell at a time (twk's `AnimatedPadding`).

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment | CENTER_LEFT, CENTER, CENTER_RIGHT, BOTTOM_RIGHT | CENTER_LEFT |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| cycle | false, true | true |

## AnimatedPositioned

A child pinned in a stack that runs to the edges it is pinned to.

- `rtwk::animation::AnimatedPositioned` (struct) in `crates/rtwk-animation/src/implicit.rs:737`: Pins child to the edges of the Stack it is in as Positioned does, each edge and length running to its new place a cell at a time; one given where there was none, or taken away, at once.

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| pinned | TOP_LEFT, BOTTOM_LEFT, BOTTOM_RIGHT, TOP_RIGHT, fill | TOP_LEFT |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| cycle | false, true | true |

## AnimatedBox

A box whose size, padding, background and border colors run to the new ones; a border of another type switches halfway.

- `rtwk::animation::AnimatedBox` (struct) in `crates/rtwk-animation/src/implicit.rs:1057`: A BoxWidget around child made width wide and height tall where they are given (SizedBox), its style, the styles of its border, padding and size running to the new ones: colors as StyleTween has them, lengths a cell at a time.

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| big | false, true | false |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| cycle | false, true | true |

## AnimatedOpacity, Opacity

A child shown at an opacity, mixed cell by cell with what is under it; the one on the left runs to the opacity chosen.

- `rtwk::animation::AnimatedOpacity` (struct) in `crates/rtwk-animation/src/implicit.rs:893`: Shows child at opacity (Opacity), running to the new one (twk's `AnimatedOpacity`).
- `rtwk::animation::Opacity` (struct) in `crates/rtwk-animation/src/switcher.rs:675`: Shows child at opacity, from 0 (not at all: what is under it shows) to 1 (as it paints), mixed with what is under it cell by cell: the child's background over the one under it as by alpha (a pen's `bg_alpha`), then the child's text, its color mixed with that background as the alpha says, or where the child has none, or both have text and the opacity is below half, the text under it, its color mixed with the child's background.

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| opacity | 0, ¼, ½, ¾, 1 | ½ |
| cycle | false, true | true |

## AnimatedDefaultTextStyle

The style of the text below it, its colors running to the new ones.

- `rtwk::animation::AnimatedDefaultTextStyle` (struct) in `crates/rtwk-animation/src/implicit.rs:974`: The style the Texts below it are drawn in (DefaultTextStyle), its colors running to the new ones (StyleTween) (twk's `AnimatedDefaultTextStyle`).

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| style | primary, error, success | primary |
| bold | false, true | false |
| cycle | false, true | true |

## AnimatedSize

As big as its child, running to the new size when the child grows or shrinks.

- `rtwk::animation::AnimatedSize` (struct) in `crates/rtwk-animation/src/implicit.rs:1230`: As big as child, and when the child's size changes, running to the new size from the one it shows in duration along curve, a cell at a time: the child laid out at its new size, placed by alignment and cut off where it is bigger than this.

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| lines | 1, 3, 5 | 1 |
| alignment | TOP_LEFT, CENTER, BOTTOM_RIGHT | TOP_LEFT |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| cycle | false, true | true |

## TweenAnimationBuilder

Builds with a value that runs to the one it is given, through the tweens it makes.

- `rtwk::animation::TweenAnimationBuilder` (struct) in `crates/rtwk-animation/src/implicit.rs:515`: Builds builder of a value that runs to value whenever it changes — from from at first when given — over its duration, along its curve (twk's `TweenAnimationBuilder`).

### Code

```rust
fn tween(cycle_target: f64, theme: &ThemeData) -> TweenAnimationBuilder<f64> {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| value | 0, ¼, ½, ¾, 1 | ½ |
| curve | LINEAR, EASE, EASE_IN, EASE_OUT, EASE_IN_OUT, FAST_OUT_SLOW_IN, DECELERATE, BOUNCE_OUT, ELASTIC_OUT | EASE_IN_OUT |
| cycle | false, true | true |

## AnimatedSwitcher

A child that gives way to another as the transition shows them: faded, slid or dissolved.

- `rtwk::animation::AnimatedSwitcher` (struct) in `crates/rtwk-animation/src/switcher.rs:82`: Shows child, and when given one that does not update it (of another type, or with another key: `Text::new(n.to_string()).into().with_key(Key::value(n))`), has the new one come in as the old one goes out, as transition shows them: the new one in duration along switch_in_curve, the old one in reverse_duration (the duration when none) along switch_out_curve, from where it was when a child that had not come in all the way is replaced.

### Code

```rust
fn cycling(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| child | first, second, third | first |
| transition | fade, slide, slide right, slide up, slide down, dissolve | fade |
| cycle | false, true | true |

## Dissolve, FractionalTranslation

What the transitions are made of: a share of the cells of a child, and a child moved a share of its size.

- `rtwk::animation::Dissolve` (struct) in `crates/rtwk-animation/src/switcher.rs:918`: Shows shown of the cells of child, from 0 (none: what is under it shows) to 1 (all), taken in a scattered order of the screen's cells, the same for every `Dissolve`: one with inverse takes them from the end of that order, so that at `1 - t` it shows just the cells one without leaves out at `t`.
- `rtwk::animation::FractionalTranslation` (struct) in `crates/rtwk-animation/src/switcher.rs:1099`: Shows child moved x times its width to the right and y times its height down (left and up where negative), to the nearest cell, and cut off at the edges of its own place: as big as the child, which takes the mouse where it shows (twk's `FractionalTranslation`).

### Code

```rust
fn dissolve(controller: &AnimationController, theme: &ThemeData) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| share | 0, ¼, ½, ¾, 1 | ½ |
| run | false, true | true |
| inverse | false, true | false |

## Effect

A child shown through an effect over its cells, run on the app's clock: recolored, swept, dissolved, typed or glitched in, while the child repaints underneath.

- `rtwk::animation::Effect` (struct) in `crates/rtwk-animation/src/effect.rs:76`: Shows child through effect, a function over the cells of the child's rectangle: each shows what the child painted there, what was under it, or what the effect makes of them.

### Code

```rust
fn effect(cycle_effect: CellEffect, theme: &ThemeData, runs: u64, on_again: Handler<()>) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| effect | sweep, dissolve, typewriter, glitch, dim, desaturate, tint, scanlines, grade, sweep down, glitch | sweep |
| cycle | false, true | true |
