<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Data

[All widgets](../widgets.md)

## Sparkline

A series of values as bars an eighth of a cell fine, the newest on the right.

API:

- `rtwk::data::Sparkline` (struct, `crates/rtwk-data/src/sparkline.rs:48`) `data` as bars a column wide that grow upwards in eighths of a cell, oldest to the left: the last values that fit, the newest at the right edge once they fill it.
  - public fields: `data: Rc<[f64]>`, `min: f64`, `max: Option<f64>`, `style: Style`, `from_right: bool`

### Code

The gallery's Code tab at the initial options (route `#/data/sparkline`, `crates/rtwk-gallery/src/pages/data_charts.rs:154`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock every 150 ms.
let step: i64 = elapsed_ms / 150;
let data: Vec<_> = (step..step + 48).map(|i| 4.0 + 3.0 * (i as f64 / 4.0).sin()).collect();
let spark = Sparkline::new(data)
    .style(Theme::of(cx).info.clone());

SizedBox::new(Some(48), Some(4), Some(spark.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| data | wave, ramp, steps, noise | wave |
| rows | 1, 2, 4 | 4 |
| max | null, 10 | null |
| live | on, off | true |
| color | info, primary, accent, success, warning | info |

### Methods

#### `Sparkline`: constructors and associated functions

- `pub fn new(data: impl Into<Rc<[f64]>>) -> Sparkline` - Shows `data` as bars from 0 to the largest value.

#### `Sparkline`: builder methods (take and return `self`)

- `pub fn min(mut self, min: f64) -> Sparkline` - An empty bar at `min`.
- `pub fn max(mut self, max: f64) -> Sparkline` - A full bar at `max`.
- `pub fn style(mut self, style: Style) -> Sparkline` - In `style`.
- `pub fn from_right(mut self, from_right: bool) -> Sparkline` - Whether fewer values than fit end at the right edge.

## Gauge, LineGauge

How much of something there is: a block filled to a ratio, or a line.

API:

- `rtwk::data::Gauge` (struct, `crates/rtwk-data/src/gauge.rs:60`) `ratio` of its width filled from the left, in eighths of a cell, on every row, with `label` in the middle of the middle row, or with `show_percentage` the ratio as a percentage.
  - public fields: `ratio: f64`, `label: Option<Str>`, `show_percentage: bool`, `style: Style`, `label_style: Style`
- `rtwk::data::LineGauge` (struct, `crates/rtwk-data/src/gauge.rs:215`) `ratio` as a line after `label` (or with `show_percentage` the ratio as a percentage) and a blank: the part from the left filled in `filled_style`, the rest in `unfilled_style`, to the nearest half cell where `kind` has ...
  - public fields: `ratio: f64`, `label: Option<Str>`, `show_percentage: bool`, `kind: Option<BorderType>`, `style: Style`, `filled_style: Style`, `unfilled_style: Style`, `segmented: bool`

### Code

The gallery's Code tab at the initial options (route `#/data/gauge-linegauge`, `crates/rtwk-gallery/src/pages/data_charts.rs:167`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock every 25 ms.
let ratio = ((elapsed_ms / 25) * 25).rem_euclid(9_000).min(7_000) as f64 / 7_000.0;
let percent = format!("{:.0}%", (ratio * 100.0).round());
let theme = Theme::of(cx);
let style = theme.primary.clone().merge(&Style::new().bg(
    theme.surface.bg.unwrap_or(Color::DEFAULT),
));
let gauge = Gauge::new(ratio)
    .style(style)
    .label_style(Style::new().attrs(Attrs::BOLD))
    .label(percent.clone());
let line = LineGauge::new(ratio)
    .kind(BorderType::Heavy)
    .filled_style(if ratio > 0.9 {
        theme.error.clone()
    } else if ratio > 0.75 {
        theme.warning.clone()
    } else {
        theme.success.clone()
    })
    .unfilled_style(theme.border.clone())
    .label(format!("used {percent}"));

let gauges = column![
    SizedBox::new(None, Some(3), Some(gauge.into())),
    line,
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Stretch)
.spacing(1);
SizedBox::new(Some(40), None, Some(gauges.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| live | on, off | true |
| ratio | 0..100 (a slider) | 75% |
| line | single, heavy, double, ascii | heavy |
| label | own, percent, none | own |
| color | primary, accent, success, warning, info | primary |
| segmented | on, off | false |

### Methods

#### `Gauge`: constructors and associated functions

- `pub fn new(ratio: f64) -> Gauge` - Fills `ratio` of its width.

#### `Gauge`: builder methods (take and return `self`)

- `pub fn label(mut self, label: impl Into<Str>) -> Gauge` - With `label` over it.
- `pub fn show_percentage(mut self) -> Gauge` - With the percentage over it.
- `pub fn style(mut self, style: Style) -> Gauge` - In `style`.
- `pub fn label_style(mut self, style: Style) -> Gauge` - The label in `style`.

#### `LineGauge`: constructors and associated functions

- `pub fn new(ratio: f64) -> LineGauge` - Shows `ratio` as a line.

#### `LineGauge`: builder methods (take and return `self`)

- `pub fn label(mut self, label: impl Into<Str>) -> LineGauge` - After `label`.
- `pub fn show_percentage(mut self) -> LineGauge` - After the percentage.
- `pub fn kind(mut self, kind: BorderType) -> LineGauge` - Drawn with the lines of `kind`.
- `pub fn style(mut self, style: Style) -> LineGauge` - In `style`.
- `pub fn filled_style(mut self, style: Style) -> LineGauge` - The filled part in `style`.
- `pub fn unfilled_style(mut self, style: Style) -> LineGauge` - The rest in `style`.
- `pub fn segmented(mut self) -> LineGauge` - A column a cell.

## BarChart

Bars in groups, up or across, their values on them and their labels under them.

API:

- `rtwk::data::BarChart` (struct, `crates/rtwk-data/src/bar_chart.rs:177`) The bars of `groups` side by side, `bar_width` cells thick and `bar_gap` apart, the groups `group_gap` apart; up from the bottom, or with a `direction` of `Axis::Horizontal` from the left, in eighths of a cell.
  - public fields: `groups: Rc<[BarGroup]>`, `direction: Axis`, `bar_width: i32`, `bar_gap: i32`, `group_gap: i32`, `max: Option<f64>`, `style: Style`, `bar_style: Style`, `value_style: Style`, `label_style: Style`

### Code

The gallery's Code tab at the initial options (route `#/data/barchart`, `crates/rtwk-gallery/src/pages/data_charts.rs:187`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let base: [f64; 6] = [4.0, 7.0, 6.0, 9.0, 3.0, 5.0];
// Redraw from the application clock every 100 ms.
let t = (elapsed_ms / 100) as f64 * 0.1;
let values = std::array::from_fn::<_, 6, _>(|i| base[i] + (t * 1.3 + i as f64 * 1.7).sin());

LayoutBuilder::new(move |cx, room| {
    let theme = Theme::of(cx);
    let bw = 3;
    let (w, h) = (3 * (2 * bw + 1) + 4, room.max_height.clamp(5, 10));
    let bar = |i: usize, label, style| {
        let value = values[i];
        Bar::new(value).label(label).text(format!("{:.0}", value.round())).style(style)
    };
    let groups = ["Q1", "Q2", "Q3"].into_iter().enumerate()
        .map(|(i, quarter)| BarGroup::new([
            bar(i * 2, "a", Style::new()),
            bar(i * 2 + 1, "b", theme.accent.clone()),
        ]).label(quarter)).collect::<Vec<_>>();
    let chart = BarChart::new(groups)
        .direction(Axis::Vertical)
        .bar_width(bw)
        .bar_gap(1)
        .group_gap(2)
        .max(10.0)
        .bar_style(theme.primary.clone())
        .value_style(Style::new().attrs(Attrs::BOLD))
        .label_style(theme.muted.clone());
    SizedBox::new(Some(w), Some(h), Some(chart.into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| direction | up, across | up |
| barWidth | 1, 2, 3 | 3 |
| groups | grouped, single | grouped |
| live | on, off | true |

### Methods

#### `BarChart`: constructors and associated functions

- `pub fn new(groups: impl Into<Rc<[BarGroup]>>) -> BarChart` - Shows `groups` of bars, up, a cell thick and a cell apart.

#### `BarChart`: builder methods (take and return `self`)

- `pub fn direction(mut self, direction: Axis) -> BarChart` - Growing in `direction`.
- `pub fn bar_width(mut self, width: i32) -> BarChart` - `width` cells thick.
- `pub fn bar_gap(mut self, gap: i32) -> BarChart` - `gap` cells between two bars of a group.
- `pub fn group_gap(mut self, gap: i32) -> BarChart` - `gap` cells between two groups.
- `pub fn max(mut self, max: f64) -> BarChart` - A bar of `max` as long as the chart allows.
- `pub fn style(mut self, style: Style) -> BarChart` - In `style`.
- `pub fn bar_style(mut self, style: Style) -> BarChart` - The bars in `style`.
- `pub fn value_style(mut self, style: Style) -> BarChart` - The values in `style`.
- `pub fn label_style(mut self, style: Style) -> BarChart` - The labels in `style`.

### Also used in the code

- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::data::Bar` (struct, `crates/rtwk-data/src/bar_chart.rs:43`) One bar of a `BarChart`: its `value`, the `label` beside it and the `text` its value is shown as.
  - public fields: `value: f64`, `label: Option<Str>`, `text: Option<Str>`, `style: Style`, `value_style: Style`
- `rtwk::data::BarGroup` (struct, `crates/rtwk-data/src/bar_chart.rs:114`) Bars of a `BarChart` side by side, with a `label` under them (after them, across).
  - public fields: `bars: Rc<[Bar]>`, `label: Option<Str>`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`

## Chart

Datasets of points on two axes, as lines or dots, with a legend.

API:

- `rtwk::data::Chart` (struct, `crates/rtwk-data/src/chart.rs:220`) `datasets` plotted against `x_axis` and `y_axis`, with a `marker` finer than the cells (see `DotCanvas`), each dataset covering those before it cell by cell.
  - public fields: `datasets: Rc<[Dataset]>`, `x_axis: ChartAxis`, `y_axis: ChartAxis`, `marker: Marker`, `legend_position: Option<LegendPosition>`, `style: Style`

### Code

The gallery's Code tab at the initial options (route `#/data/chart`, `crates/rtwk-gallery/src/pages/data_charts.rs:204`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock every 50 ms.
let phase = (elapsed_ms / 50) as f64 * 0.1;
let wave = move |f: fn(f64) -> f64| (0..=60).map(|i| {
    let x = f64::from(i) * std::f64::consts::PI / 30.0;
    (x, f(x - phase))
}).collect::<Vec<_>>();

LayoutBuilder::new(move |cx, room| {
    let chart = Chart::new(
        [
            Dataset::new(wave(f64::sin)).name("sin").kind(GraphType::Line).color(Color::CYAN),
            Dataset::new(wave(f64::cos)).name("cos").kind(GraphType::Line).color(Color::YELLOW),
        ],
        ChartAxis::new((0.0, std::f64::consts::TAU)).title("x").with_labels(["0", "π", "2π"]),
        ChartAxis::new((-1.0, 1.0)).with_labels(["-1", "0", "1"]),
    )
    .marker(Marker::BRAILLE)
    .legend_position(Some(LegendPosition::TopRight))
    .style(Theme::of(cx).muted.clone());
    SizedBox::new(Some(50), Some(room.max_height.clamp(6, 12)), Some(chart.into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| graph | scatter, line | line |
| marker | braille, halfBlock, quadrant, sextant, octant, block, dot, bar, custom | braille |
| legend | topLeft, top, topRight, left, right, bottomLeft, bottom, bottomRight, none | topRight |
| live | on, off | true |

### Methods

#### `Chart`: constructors and associated functions

- `pub fn new(datasets: impl Into<Rc<[Dataset]>>, x_axis: ChartAxis, y_axis: ChartAxis) -> Chart` - Plots `datasets` against `x_axis` and `y_axis`.

#### `Chart`: builder methods (take and return `self`)

- `pub fn marker(mut self, marker: Marker) -> Chart` - With `marker`.
- `pub fn legend_position(mut self, position: Option<LegendPosition>) -> Chart` - With the legend at `position`; none for no legend.
- `pub fn style(mut self, style: Style) -> Chart` - In `style`.

### Also used in the code

- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::data::GraphType` (enum, `crates/rtwk-data/src/chart.rs:38`) How a `Dataset` is drawn.
  - variants: `Scatter`, `Line`
- `rtwk::data::ChartAxis` (struct, `crates/rtwk-data/src/chart.rs:131`) An axis of a `Chart`: the values it spans (`bounds`), its `labels` and `title`.
  - public fields: `bounds: (f64`, `title: Option<Str>`, `labels: Option<Rc<[Str]>>`, `style: Style`
- `rtwk::data::Marker` (struct, `crates/rtwk-data/src/marker.rs:20`) How a `DotCanvas` shows its dots: how many a cell holds across and down, and the glyph each pattern of them is drawn as.
  - public fields: `dots_across: i32`, `dots_down: i32`
- `rtwk::data::LegendPosition` (enum, `crates/rtwk-data/src/chart.rs:48`) Where a `Chart` puts its legend in its plot.
  - variants: `TopLeft`, `Top`, `TopRight`, `Left`, `Right`, `BottomLeft`, `Bottom`, `BottomRight`

## DotCanvas

Shapes drawn finer than the cells, in data coordinates, with labels over them.

API:

- `rtwk::data::DotCanvas` [path inferred] (struct, `crates/rtwk-data/src/dot_canvas.rs:79`) Shapes and labels drawn finer than the cells: `draw` sets dots at points of a data space (`x_bounds` across, `y_bounds` upwards), which `marker` shows several to a cell.
  - public fields: `draw: DotDrawer`, `x_bounds: (f64`, `y_bounds: (f64`, `marker: Marker`, `background: Option<Color>`

### Code

The gallery's Code tab at the initial options (route `#/data/dotcanvas`, `crates/rtwk-gallery/src/pages/data_charts.rs:216`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock every 50 ms.
let t = (elapsed_ms / 50) as f64 * 0.05;
let hand = std::f64::consts::PI / 2.0 - std::f64::consts::TAU * t / 6.0;
let wave = Shape::polyline((0..=24).map(|i| {
    let i = f64::from(i);
    (24.5 + i / 2.0, 10.5 + 5.0 * (i / 3.0 - 4.0 * t).sin())
}).collect::<Vec<_>>()).color(Color::GREEN);
let draw = drawer(move |dots| {
    dots.draw(&Shape::circle(12.0, 12.0, 9.0).color(Color::CYAN));
    dots.draw(&Shape::line(12.0, 12.0, 12.0 + 8.0 * hand.cos(), 12.0 + 8.0 * hand.sin())
        .color(Color::MAGENTA));
    dots.draw(&Shape::rectangle(24.0, 3.0, 13.0, 15.0).color(Color::YELLOW));
    dots.draw(&wave);
    dots.layer();
    dots.label(11.5, 22.6, "12");
    dots.label(28.0, 20.0, "dots");
});

LayoutBuilder::new(move |_, room| {
    let canvas = DotCanvas::new(draw.clone(), (0.0, 40.0), (0.0, 24.0))
        .marker(Marker::BRAILLE);
    SizedBox::new(Some(40), Some(room.max_height.clamp(6, 12)), Some(canvas.into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| marker | braille, halfBlock, quadrant, sextant, octant, block, dot, bar, custom | braille |
| live | on, off | true |

### Methods

#### `DotCanvas`: constructors and associated functions

- `pub fn new(draw: DotDrawer, x_bounds: (f64, f64), y_bounds: (f64, f64)) -> DotCanvas` - Draws with `draw` on the space between `x_bounds` and `y_bounds`.

#### `DotCanvas`: builder methods (take and return `self`)

- `pub fn marker(mut self, marker: Marker) -> DotCanvas` - With `marker`.
- `pub fn background(mut self, background: Color) -> DotCanvas` - With every cell's background `background`.

### Also used in the code

- `rtwk::data::Shape` (enum, `crates/rtwk-data/src/dot_painter.rs:286`) Something drawn on a `DotPainter` in its data coordinates.
  - variants: `Points`, `Line`, `Rectangle`, `Circle`, `Polyline`, `Custom`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::data::Marker` (struct, `crates/rtwk-data/src/marker.rs:20`) How a `DotCanvas` shows its dots: how many a cell holds across and down, and the glyph each pattern of them is drawn as.
  - public fields: `dots_across: i32`, `dots_down: i32`

## Calendar

A month to pick a day of: the arrows move the cursor, PageUp and PageDown a month, Enter or a click picks the day.

API:

- `rtwk::data::Calendar` (struct, `crates/rtwk-data/src/calendar.rs:422`) The days of a month in weeks, a row each from `first_weekday` — six rows, so that every month is as tall — under a row of the weekdays' names and, with `show_header`, the month's name and year, centered: the month of ...
  - public fields: `controller: Option<CalendarController>`, `on_selected: Option<Rc<dyn Fn(DateTime)>>`, `today: Option<DateTime>`, `first_weekday: u8`, `show_header: bool`, `show_week_numbers: bool`, `show_outside_days: bool`, `day_style: Option<DayStyle>`, `month_names: Rc<[String]>`, `weekday_names: Rc<[String]>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `key_hints: bool`, `style: Option<Style>`, `header_style: Option<Style>`, `weekday_style: Option<Style>`, `outside_style: Option<Style>`, `today_style: Option<Style>`, `selected_style: Option<Style>`
- `rtwk::data::CalendarController` (struct, `crates/rtwk-data/src/calendar.rs:130`) Where a `Calendar` is: its cursor, the date its keys move, whose month it shows, and its selected date, which Enter and Space set.

### Code

The gallery's Code tab at the initial options (route `#/data/calendar`, `crates/rtwk-gallery/src/pages/data_charts.rs:226`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn date_label(date: DateTime) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

// Keep a CalendarController initialized at the demo date and a
// Rc<ValueNotifier<Option<String>>> named selected in your state.
let choose = selected.clone();
let widget = Calendar::new()
    .controller(controller.clone())
    .today(DateTime::new(2026, 10, 19))
    .first_weekday(DateTime::MONDAY)
    .show_week_numbers(false)
    .show_outside_days(false)
    .on_selected(move |date| choose.set_value(Some(date_label(date))));

let readout = selected.clone();
let selection = ListenableBuilder::new(selected.clone(), move |cx, _| {
    let theme = Theme::of(cx);
    readout.with(|value| Text::rich([
        Span::styled("selected: ", theme.muted.clone()),
        match value {
            Some(value) => Span::new(value.clone()),
            None => Span::styled("nothing", theme.muted.clone()),
        },
    ]).into())
});

column![widget, selection]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| firstWeekday | 1, 7 | 1 |
| weeks | on, off | false |
| outside | on, off | false |
| marked | on, off | false |

### Methods

#### `Calendar`: constructors and associated functions

- `pub fn new() -> Calendar` - A calendar of a controller of its own, the weeks from Monday, the header shown, in English.

#### `Calendar`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: CalendarController) -> Calendar` - Showing the month of `controller`'s cursor.
- `pub fn on_selected(mut self, f: impl Fn(DateTime) + 'static) -> Calendar` - Calling `f` with the date selected.
- `pub fn today(mut self, date: DateTime) -> Calendar` - With `date` as today.
- `pub fn first_weekday(mut self, weekday: u8) -> Calendar` - Weeks from `weekday`.
- `pub fn show_header(mut self, on: bool) -> Calendar` - Whether the header shows.
- `pub fn show_week_numbers(mut self, on: bool) -> Calendar` - Whether the week numbers show.
- `pub fn show_outside_days(mut self, on: bool) -> Calendar` - Whether the days around the month show.
- `pub fn day_style(mut self, f: DayStyle) -> Calendar` - A style of its own for the days `f` says one of.
- `pub fn month_names<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Calendar` - In the months' `names`, January first.
- `pub fn weekday_names<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Calendar` - In the weekdays' `names`, Monday first.
- `pub fn focus_node(mut self, node: FocusNode) -> Calendar` - Focused by `node`.
- `pub fn autofocus(mut self) -> Calendar` - Focused when it is built.
- `pub fn can_request_focus(mut self, on: bool) -> Calendar` - Whether it takes the focus, and clicks.
- `pub fn key_hints(mut self, on: bool) -> Calendar` - Whether its keys carry their hints.

#### `CalendarController`: constructors and associated functions

- `pub fn new() -> CalendarController` - A controller with its cursor on today, no date selected and no bounds.
- `pub fn at(cursor: DateTime) -> CalendarController` - A controller with its cursor on `cursor`.

#### `CalendarController`: builder methods (take and return `self`)

- `pub fn with_selected(self, date: DateTime) -> CalendarController` - With `date` selected, and the cursor on it unless it was given by `at` (twk's constructor: the cursor on `cursor`, else on `selected`, else on today).
- `pub fn with_bounds(self, first: Option<DateTime>, last: Option<DateTime>) -> CalendarController` - Bounded by `first` and `last`, either none for no bound; the cursor moved within them.

#### `CalendarController`: methods

- `pub fn dispose(&self)` - Stops notifying and drops the listeners.
- `pub fn selected(&self) -> Option<DateTime>` - The date chosen; none for none.
- `pub fn set_selected(&self, date: Option<DateTime>)` - Selects `date` (none for none).
- `pub fn cursor(&self) -> DateTime` - The date the keys move from, marked while the calendar has the focus; its month is the one shown.
- `pub fn set_cursor(&self, date: DateTime)` - Moves the cursor to `date`, within the bounds.
- `pub fn month(&self) -> DateTime` - The first day of the month shown: the cursor's.
- `pub fn set_month(&self, date: DateTime)` - Shows the month of `date`: the cursor moves to that month, as `move_cursor` does.
- `pub fn move_cursor(&self, days: i64, months: i64)` - Moves the cursor `months` months, to the same day of the month or the last day of a shorter one, then `days` days; within the bounds.
- `pub fn first_date(&self) -> Option<DateTime>` - The earliest date the cursor may be on; none for no bound.
- `pub fn set_first_date(&self, date: Option<DateTime>)` - Bounds the cursor below by `date`.
- `pub fn last_date(&self) -> Option<DateTime>` - The latest date the cursor may be on; none for no bound.
- `pub fn set_last_date(&self, date: Option<DateTime>)` - Bounds the cursor above by `date`.

### Also used in the code

- `rtwk::data::DateTime` (struct, `crates/rtwk-data/src/date_time.rs:35`) A civil local date and time to the microsecond, the constructors normalising as Dart's `DateTime(year, month, day)` does: a month past 12 carries into the year, a day past the month's length (or below 1) into the months ...
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## Heatmap, ActivityHeatmap

Values over a grid as shades or colors, its rows and columns labelled; counts by day over weeks, a column a week. The arrows move a cursor, its tooltip saying what is there.

API:

- `rtwk::data::Heatmap` (struct, `crates/rtwk-data/src/heatmap.rs:220`) `rows` × `columns` cells, each the `value` there on the `scale`: from `min`, by default the smallest value, to `max`, by default the largest; those beyond them as they are.
  - public fields: `rows: usize`, `columns: usize`, `value: HeatmapValue`, `min: Option<f64>`, `max: Option<f64>`, `scale: HeatmapScale`, `row_labels: Option<Rc<[String]>>`, `column_labels: Option<Rc<[String]>>`, `cell_width: i32`, `show_legend: bool`, `legend_labels: (String`, `tooltip: Option<Rc<dyn Fn(usize`, `on_selected: Option<Rc<dyn Fn(Cell2)>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `key_hints: bool`, `style: Option<Style>`, `label_style: Option<Style>`, `cursor_style: Option<Style>`
- `rtwk::data::ActivityHeatmap` (struct, `crates/rtwk-data/src/heatmap.rs:1836`) Counts by day as a `Heatmap`, a column a week from `first_weekday` and a row a weekday, the last column the week of `end`: `weeks` weeks, the oldest at the left.
  - public fields: `counts: Rc<[(DateTime`, `end: Option<DateTime>`, `weeks: usize`, `first_weekday: u8`, `tooltip: Option<DayTooltip>`, `scale: Option<HeatmapScale>`, `on_selected: Option<Rc<dyn Fn(DateTime)>>`, `show_legend: bool`, `legend_labels: (String`, `cell_width: i32`, `weekday_names: Rc<[String]>`, `month_names: Rc<[String]>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `label_style: Option<Style>`, `cursor_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/data/heatmap-activityheatmap`, `crates/rtwk-gallery/src/pages/data_charts.rs:238`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn noise(i: i64) -> f64 {
    let s = (i as f64 * 91.345 + 3.1).sin() * 43_758.545_3;
    s - s.floor()
}

fn date_label(date: DateTime) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

let end = DateTime::new(2026, 10, 19);
let counts = (0..140).map(|i| {
    let day = end.add_days(-i);
    let max = if day.weekday() > DateTime::FRIDAY { 3.0 } else { 9.0 };
    (day, (noise(i) * max).floor())
});
// Keep a Rc<ValueNotifier<Option<String>>> named selected in your state.
let choose = selected.clone();
let widget = ActivityHeatmap::new(counts)
    .end(end)
    .weeks(20)
    .first_weekday(DateTime::MONDAY)
    .show_legend(true)
    .cell_width(2)
    .on_selected(move |day| choose.set_value(Some(date_label(day))));

let readout = selected.clone();
let selection = ListenableBuilder::new(selected.clone(), move |cx, _| {
    let theme = Theme::of(cx);
    readout.with(|value| Text::rich([
        Span::styled("selected: ", theme.muted.clone()),
        match value {
            Some(value) => Span::new(value.clone()),
            None => Span::styled("nothing", theme.muted.clone()),
        },
    ]).into())
});

column![widget, selection]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| kind | activity, grid | activity |
| firstWeekday | 1, 7 | 1 |
| scale | shades, colors | shades |
| legend | on, off | true |
| cell | 2, 1 | 2 |

### Methods

#### `Heatmap`: constructors and associated functions

- `pub fn new(rows: usize, columns: usize, value: impl Fn(usize, usize) -> Option<f64> + 'static) -> Heatmap` - Shows `value` of each of `rows` × `columns` cells on the default shades.

#### `Heatmap`: builder methods (take and return `self`)

- `pub fn min(mut self, min: f64) -> Heatmap` - The value of the first shade or color.
- `pub fn max(mut self, max: f64) -> Heatmap` - The value of the last shade or color.
- `pub fn scale(mut self, scale: HeatmapScale) -> Heatmap` - The shades or the colors.
- `pub fn row_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Heatmap` - A label for each row; panics unless there are as many as `rows`.
- `pub fn column_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Heatmap` - A label for each column; panics unless there are as many as `columns`.
- `pub fn cell_width(mut self, width: i32) -> Heatmap` - The columns of a cell; panics for less than one.
- `pub fn show_legend(mut self, show: bool) -> Heatmap` - Whether a row below the cells shows the scale.
- `pub fn legend_labels(mut self, less: impl Into<String>, more: impl Into<String>) -> Heatmap` - The words either side of the legend.
- `pub fn tooltip(mut self, tooltip: impl Fn(usize, usize) -> String + 'static) -> Heatmap` - What a cell says on the mouse or marked.
- `pub fn on_selected(mut self, f: impl Fn(Cell2) + 'static) -> Heatmap` - Called with the cell Enter, Space or a click selected.
- `pub fn focus_node(mut self, node: FocusNode) -> Heatmap` - The focus node it listens to and takes keys on.
- `pub fn autofocus(mut self, on: bool) -> Heatmap` - Whether it takes the focus as it is built.
- `pub fn key_hints(mut self, on: bool) -> Heatmap` - Whether the keys it takes carry their hints.

#### `ActivityHeatmap`: constructors and associated functions

- `pub fn new(counts: impl IntoIterator<Item = (DateTime, f64)>) -> ActivityHeatmap` - Shows `counts` over the last 26 weeks.
- `pub fn default_scale() -> HeatmapScale` - `·░▒▓█`: a dot for no count, so that the weeks show.
- `pub fn default_tooltip(day: DateTime, count: f64) -> String` - `count` on `day`, as `3 on 2026-10-21`.

#### `ActivityHeatmap`: builder methods (take and return `self`)

- `pub fn end(mut self, end: DateTime) -> ActivityHeatmap` - The last day shown.
- `pub fn weeks(mut self, weeks: usize) -> ActivityHeatmap` - How many weeks; panics for none.
- `pub fn first_weekday(mut self, weekday: u8) -> ActivityHeatmap` - The weekday of the first row; panics for another number than 1 to 7.
- `pub fn tooltip(mut self, f: impl Fn(DateTime, f64) -> String + 'static) -> ActivityHeatmap` - What a day says on the mouse or marked.
- `pub fn scale(mut self, scale: HeatmapScale) -> ActivityHeatmap` - The shades or the colors.
- `pub fn on_selected(mut self, f: impl Fn(DateTime) + 'static) -> ActivityHeatmap` - Called with the day selected.
- `pub fn show_legend(mut self, show: bool) -> ActivityHeatmap` - Whether a row below the cells shows the scale.
- `pub fn legend_labels(mut self, less: impl Into<String>, more: impl Into<String>) -> ActivityHeatmap` - The words either side of the legend.
- `pub fn cell_width(mut self, width: i32) -> ActivityHeatmap` - The columns of a cell; panics for less than one.
- `pub fn weekday_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> ActivityHeatmap` - The weekdays' names, Monday first; panics unless there are 7.
- `pub fn month_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> ActivityHeatmap` - The months' names, January first; panics unless there are 12.
- `pub fn focus_node(mut self, node: FocusNode) -> ActivityHeatmap` - The focus node.
- `pub fn autofocus(mut self, on: bool) -> ActivityHeatmap` - Whether it takes the focus as it is built.
- `pub fn style(mut self, style: Style) -> ActivityHeatmap` - Every cell.
- `pub fn label_style(mut self, style: Style) -> ActivityHeatmap` - The labels and the words of the legend.
- `pub fn cursor_style(mut self, style: Style) -> ActivityHeatmap` - The marked cell.

### Also used in the code

- `rtwk::data::DateTime` (struct, `crates/rtwk-data/src/date_time.rs:35`) A civil local date and time to the microsecond, the constructors normalising as Dart's `DateTime(year, month, day)` does: a month past 12 carries into the year, a day past the month's length (or below 1) into the months ...
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## Table

Rows in columns that size by their content or a share, sorted by a click on a title; the arrows select a row or a cell.

API:

- `rtwk::data::Table` (struct, `crates/rtwk-data/src/table.rs:693`) A table of rows in `columns` under a header of their titles, which stays while the rows scroll.
  - public fields: `rows: Option<Rc<[T]>>`, `source: Option<Rc<dyn TableSource<T>>>`, `columns: Rc<[TableColumn<T>]>`, `row_key: Option<Rc<dyn Fn(&T) -> Key>>`, `controller: Option<TableController>`, `selection: TableSelection`, `marks: Option<SelectionController<Key>>`, `row_height: Option<i32>`, `column_gap: i32`, `padding: Option<EdgeInsets>`, `border: BorderType`, `show_header: bool`, `sample_rows: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `header_style: Option<Style>`, `border_style: Option<Style>`, `selected_style: Option<Style>`
- `rtwk::data::TableColumn` (struct, `crates/rtwk-data/src/table.rs:212`) A column of a `Table` whose rows are `T`s: its `title`, what it shows of a row (`cell`), how wide it is (`width`), where its text sits (`align`) and how text too wide for it is cut off (`overflow`), and how it orders ...
  - public fields: `title: Str`, `cell: Rc<dyn Fn(&T) -> TableCell>`, `width: GridTrack`, `align: TextAlign`, `overflow: TextOverflow`, `compare: Option<Rc<dyn Fn(&T`
- `rtwk::data::TableController` (struct, `crates/rtwk-data/src/table.rs:385`) Where a `Table` is: the row selected, by its key (`selected_key`), the column of the cell selected (`selected_column`), the column the rows are sorted by (`sort_column`, `sort_ascending`), and the widths the columns ...

### Code

The gallery's Code tab at the initial options (route `#/data/table`, `crates/rtwk-gallery/src/pages/data_lists.rs:44`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let columns = [
        TableColumn::new("name", |row: &FileRow| TableCell::new(row.name.clone()))
            .compare(|a, b| a.name.cmp(&b.name)),
        TableColumn::new("lines", |row: &FileRow| {
            TableCell::rich([Span::new(row.lines.to_string()).bold()])
        })
        .align(TextAlign::Right)
        .compare(|a, b| a.lines.cmp(&b.lines)),
        TableColumn::new("covered", |row: &FileRow| TableCell::widget(Covered(row.coverage)))
            .width(GridTrack::fr(1)),
    ];
// Retain a controller initialized with Key::value("box.dart".to_owned()).
let files = [
    ("box.dart", 412, 0.9),
    ("text.dart", 230, 0.75),
    ("flex.dart", 388, 0.6),
    ("grid.dart", 520, 0.4),
    ("stack.dart", 145, 1.0),
].map(|(name, lines, coverage)| FileRow { name: name.to_owned(), lines, coverage });
let table = Table::new(files, columns).row_key(|row| Key::value(row.name.clone()));
let table = table
    .controller(controller)
    .selection(TableSelection::Row)
    .border(BorderType::Single)
    .show_header(true)
    .column_gap(3)
    .on_activated(on_activated);

LayoutBuilder::new(move |_, _| {
    SizedBox::new(Some(50), Some(7), Some(table.clone().into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| selection | none, row, cell | row |
| border | none, single, rounded, double, heavy, ascii | single |
| header | on, off | true |
| rows | 5, 10000 | 5 |

### Methods

#### `Table`: constructors and associated functions

- `pub fn new(rows: impl Into<Rc<[T]>>, columns: impl Into<Rc<[TableColumn<T>]>>) -> Table<T>` - A table of `rows`, a list it is given whole, in `columns`.
- `pub fn source(source: Rc<impl TableSource<T> + 'static>, columns: impl Into<Rc<[TableColumn<T>]>>) -> Table<T>` - A table of the rows of `source`, read one at a time, for more of them than a list would hold.

#### `Table`: builder methods (take and return `self`)

- `pub fn row_key(mut self, key: impl Fn(&T) -> Key + 'static) -> Table<T>` - Keyed by `key`.
- `pub fn controller(mut self, controller: TableController) -> Table<T>` - Held by `controller`.
- `pub fn selection(mut self, selection: TableSelection) -> Table<T>` - Selecting as `selection` says.
- `pub fn marks(mut self, marks: impl Into<SelectionController<Key>>) -> Self` - Show a leading checkbox column backed by a multiple selection controller.
- `pub fn markable(mut self, predicate: impl Fn(&Key) -> bool + 'static) -> Self` - Which row keys can be marked by input or display a checkbox.
- `pub fn on_activated(mut self, handler: Handler<T>) -> Table<T> where T: Clone,` - Gives `handler` a clone of the selected row when Enter is pressed.
- `pub fn row_menu(mut self, menu: impl Fn(&T) -> Vec<MenuEntry> + 'static) -> Table<T>` - The entries of the menu of a row (`show_context_menu`), made of the row when the menu opens.
- `pub fn row_height(mut self, rows: impl Into<Option<i32>>) -> Table<T>` - Rows `rows` tall, none for as many as the tallest cell.
- `pub fn column_gap(mut self, gap: i32) -> Table<T>` - `gap` cells between the columns.
- `pub fn padding(mut self, padding: EdgeInsets) -> Table<T>` - With `padding` around the rows, which scrolls with them (the header stays above it).
- `pub fn border(mut self, border: BorderType) -> Table<T>` - Lines of `border` between the columns and under the header.
- `pub fn show_header(mut self, on: bool) -> Table<T>` - Whether the titles show.
- `pub fn sample_rows(mut self, rows: usize) -> Table<T>` - Measured by `rows` besides those in view.
- `pub fn focus_node(mut self, node: FocusNode) -> Table<T>` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> Table<T>` - Taking the focus once mounted.
- `pub fn can_request_focus(mut self, on: bool) -> Table<T>` - Whether it can take the focus.
- `pub fn style(mut self, style: Style) -> Table<T>` - Every cell in `style`, over the theme's.
- `pub fn header_style(mut self, style: Style) -> Table<T>` - The header in `style`.
- `pub fn border_style(mut self, style: Style) -> Table<T>` - The lines in `style`.
- `pub fn selected_style(mut self, style: Style) -> Table<T>` - The row selected, with the focus, in `style`.
- `pub fn unfocused_selected_style(mut self, style: Style) -> Table<T>` - The row selected, without the focus, in `style`.
- `pub fn cursor_style(mut self, style: Style) -> Table<T>` - The cell selected in `style`.
- `pub fn sort_keys(mut self, on: bool) -> Table<T>` - Whether `s`, `<` and `>` sort.

#### `TableColumn`: constructors and associated functions

- `pub fn new(title: impl Into<Str>, cell: impl Fn(&T) -> TableCell + 'static) -> TableColumn<T>` - A column titled `title` that shows `cell` of each row: as wide as its title and its text want by default (`width`), its text at the left (`align`) and cut off with `…` (`overflow`); with `compare`, one that sorts the ...

#### `TableColumn`: builder methods (take and return `self`)

- `pub fn width(mut self, width: GridTrack) -> TableColumn<T>` - This wide.
- `pub fn align(mut self, align: TextAlign) -> TableColumn<T>` - Its text placed so.
- `pub fn overflow(mut self, overflow: TextOverflow) -> TableColumn<T>` - Its text cut off so.
- `pub fn compare(mut self, compare: impl Fn(&T, &T) -> Ordering + 'static) -> TableColumn<T>` - Sorting the rows by `compare`.

#### `TableController`: constructors and associated functions

- `pub fn new() -> TableController` - A controller that selects no row, the first column, sorts nothing and resizes no column.

#### `TableController`: builder methods (take and return `self`)

- `pub fn with_selected_key(self, key: Key) -> TableController` - Selecting the row whose key is `key` (none by default).
- `pub fn with_selected_column(self, column: usize) -> TableController` - Selecting the cell in `column` (the first by default).
- `pub fn with_sort_column(self, column: usize) -> TableController` - Sorting the rows by `column`, in the order of its `TableColumn::compare` unless `with_sort_ascending` says false.
- `pub fn with_sort_ascending(self, ascending: bool) -> TableController` - Sorting in the order of the column's compare, or the other way.
- `pub fn with_column_widths(self, widths: impl IntoIterator<Item = (usize, i32)>) -> TableController` - Giving the columns in `widths` those widths, in cells, by the column.

#### `TableController`: methods

- `pub fn selected_key(&self) -> Option<Key>` - The key of the row selected (`Table::row_key`); none for none, and a key no row has selects none.
- `pub fn set_selected_key(&self, key: Option<Key>)` - Selects the row whose key is `key`; none for none.
- `pub fn selected_column(&self) -> usize` - The column of the cell selected, from 0, in a table that selects cells.
- `pub fn set_selected_column(&self, column: usize)` - Selects the cell in `column`.
- `pub fn sort_column(&self) -> Option<usize>` - The column the rows are sorted by, from 0; none, or a column that does not sort, for the order of `Table::rows`.
- `pub fn sort_ascending(&self) -> bool` - Whether the rows are sorted in the order of the column's `TableColumn::compare`, or the other way.
- `pub fn sort_by(&self, column: Option<usize>, ascending: bool)` - Sorts the rows by `column`, `ascending` or not; none for the order of `Table::rows`.
- `pub fn column_widths(&self) -> BTreeMap<usize, i32>` - The width given to each column resized (`resize_column`), in cells, by the column, from 0; a column not in it is as wide as its `TableColumn::width` makes it.
- `pub fn resize_column(&self, column: usize, width: Option<i32>)` - Makes `column` `width` cells wide, within its bounds (see `Table`), as a drag of its edge and the keys do; none gives it back the width its `TableColumn::width` makes.
- `pub fn has_listeners(&self) -> bool` - Whether anything listens.
- `pub fn dispose(&self)` - Lets go of its listeners.

### Also used in the code

- `rtwk::data::TableCell` (struct, `crates/rtwk-data/src/table.rs:114`) What a cell of a `Table` shows: text in one style (`TableCell::new`), text in spans (`TableCell::rich`), or a widget (`TableCell::widget`).
  - public fields: `text: Option<Str>`, `spans: Option<Rc<[Span]>>`, `style: Style`, `child: Option<AnyWidget>`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`
- `rtwk::layout::GridTrack` (struct, `crates/rtwk-render/src/objects/grid.rs:54`) The length of a column or a row of a `RenderGrid`, in whole cells: fixed, a fraction of the cells the other tracks leave (CSS `fr`), or as long as the children in it ask for (CSS `auto`).
  - public fields: `kind: GridTrackKind`, `fraction: i32`, `min: i32`, `max: i32`
- `rtwk::data::TableSelection` (enum, `crates/rtwk-data/src/table.rs:301`) What the keys and the mouse select in a `Table`.
  - variants: `None`, `Row`, `Cell`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## DescriptionList

Terms and their values, the values in a column of their own, aligned group by group; a long value wraps under itself, and on a narrow screen each goes under its term.

API:

- `rtwk::data::DescriptionList` (struct, `crates/rtwk-data/src/description_list.rs:171`) Terms and their values, a row each, the values in a column of their own: what `docker inspect`, `kubectl describe` or the header of `htop` prints.
  - public fields: `items: Rc<[DescriptionItem]>`, `separator: Str`, `spacing: i32`, `term_align: TextAlign`, `term_width: Option<i32>`, `max_term_fraction: f64`, `stack_below_width: i32`, `term_style: Option<Style>`, `value_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/data/descriptionlist`, `crates/rtwk-gallery/src/pages/data_lists.rs:62`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let description = DescriptionList::new([
    DescriptionItem::new("Status", Text::rich([
        Span::styled("● ", theme.success.clone()),
        Span::new("Running (healthy)"),
    ])),
    DescriptionItem::text("IP Address", "192.168.1.42"),
    DescriptionItem::text("Uptime", "3 days, 4 hours"),
    DescriptionItem::text("Image", "twk/server:0.1.0"),
    DescriptionItem::group("Limits"),
    DescriptionItem::text("CPU", "2 cores"),
    DescriptionItem::new("Memory", ProgressBar::new(Some(0.25))),
    DescriptionItem::text("Restart policy", "unless stopped, 5 times at most in a minute"),
])
.term_align(TextAlign::Left)
.separator(":");

SizedBox::new(Some(44), None, Some(description.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 20..60 (a slider) | 44 cells |
| align | left, center, right | left |
| separator | colon, none | colon |
| groups | on, off | true |

### Methods

#### `DescriptionList`: constructors and associated functions

- `pub fn new(items: impl Into<Rc<[DescriptionItem]>>) -> DescriptionList` - Shows `items`, a row each.

#### `DescriptionList`: builder methods (take and return `self`)

- `pub fn separator(mut self, separator: impl Into<Str>) -> DescriptionList` - Terms followed by `separator`.
- `pub fn spacing(mut self, spacing: i32) -> DescriptionList` - `spacing` columns between the term column and the values.
- `pub fn term_align(mut self, align: TextAlign) -> DescriptionList` - Terms placed as `align` says in their column.
- `pub fn term_width(mut self, width: i32) -> DescriptionList` - A term column `width` wide, for every group.
- `pub fn max_term_fraction(mut self, fraction: f64) -> DescriptionList` - A term column of at most `fraction` of the width.
- `pub fn stack_below_width(mut self, width: i32) -> DescriptionList` - Values under their terms below `width` columns.
- `pub fn term_style(mut self, style: Style) -> DescriptionList` - Terms in `style` over the theme's muted style.
- `pub fn value_style(mut self, style: Style) -> DescriptionList` - Values in `style` over the theme's text.

### Also used in the code

- `rtwk::data::DescriptionItem` (struct, `crates/rtwk-data/src/description_list.rs:40`) A row of a `DescriptionList`: a `term` and its `value`, or the heading of a group of rows (`DescriptionItem::group`).
  - public fields: `term: Str`, `value: Option<AnyWidget>`, `semantics_value: Option<Str>`
- `rtwk::animation::ProgressBar` (struct, `crates/rtwk-animation/src/progress_bar.rs:43`) A bar filled to `value`, from 0 (empty) to 1 (full), in eighths of a cell — braille halves where block elements are two columns wide, the theme's shades where its glyphs have no partial blocks — as wide as it may be and ...
  - public fields: `value: Option<f64>`, `style: Option<Style>`, `semantics_label: Option<Str>`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`

## TreeView

Nodes under their parents: the arrows move and expand, Enter or a click on the expander opens one; test/ loads a while.

API:

- `rtwk::data::TreeView` (struct, `crates/rtwk-data/src/tree.rs:1195`) `roots` and their descendants, each node a row below its parent and indented a level more, those whose parent `controller` expands; a node with children shows `▸` before its label, `▾` while expanded.
  - public fields: `roots: Rc<[T]>`, `children: Rc<dyn Fn(&T) -> NodeChildren<T>>`, `label: TreeLabel<T>`, `has_children: Option<Rc<dyn Fn(&T) -> bool>>`, `node_key: Rc<dyn Fn(&T) -> Key>`, `controller: Option<TreeController>`, `show_guides: bool`, `separate_roots: bool`, `max_indent: Option<i32>`, `gutter: Option<TreeLabel<T>>`, `gutter_width: i32`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `style: Option<Style>`, `guide_style: Option<Style>`, `selected_style: Option<Style>`, `unfocused_selected_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/data/treeview`, `crates/rtwk-gallery/src/pages/data_lists.rs:80`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const ROOTS: [&str; 4] = ["lib", "test", "README.md", "pubspec.yaml"];

fn children(path: &str) -> &'static [&'static str] {
    match path {
        "lib" => &["lib/src", "lib/twk.dart"],
        "lib/src" => &["lib/src/box.dart", "lib/src/flex.dart", "lib/src/table.dart", "lib/src/text.dart"],
        "test" => &["test/box_test.dart", "test/table_test.dart"],
        _ => &[],
    }
}

// Retain the controller in your state, initialized as:
// TreeController::new()
//     .with_expanded([Key::value("lib"), Key::value("lib/src")])
//     .with_selected_key(Key::value("lib/src/table.dart")).

// load_test creates a cancellable future that completes after one second.
let tree = TreeView::new(
    ROOTS,
    move |path| {
        if *path == "test" {
            load_test(Ok(children(path).into()))
        } else {
            NodeChildren::list(children(path))
        }
    },
    |_, entry| Text::new(entry.node.rsplit('/').next().unwrap_or(entry.node)).into(),
)
.has_children(|path| !path.contains('.'))
.controller(controller)
.show_guides(true)
.separate_roots(false);

LayoutBuilder::new(move |_, room| {
    SizedBox::new(Some(40), Some(room.max_height.clamp(4, 12)), Some(tree.clone().into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| guides | on, off | true |
| separate | on, off | false |
| indent | none, 2 | none |
| load | loads, fails | loads |

### Methods

#### `TreeView`: constructors and associated functions

- `pub fn new(roots: impl Into<Rc<[T]>>, children: impl Fn(&T) -> NodeChildren<T> + 'static, label: impl for<'a, 'b> Fn(&mut BuildCx<'b>, &TreeEntry<'a, T>) -> AnyWidget + 'static) -> TreeView<T>` - Shows `roots` and the children of those the controller expands, each labelled by `label`; a node is its own key (see `node_key`).
- `pub fn keyed(roots: impl Into<Rc<[T]>>, children: impl Fn(&T) -> NodeChildren<T> + 'static, label: impl for<'a, 'b> Fn(&mut BuildCx<'b>, &TreeEntry<'a, T>) -> AnyWidget + 'static, node_key: impl Fn(&T) -> Key + 'static) -> TreeView<T>` - As `TreeView::new`, for nodes that are no values: each keyed by `node_key`.

#### `TreeView`: builder methods (take and return `self`)

- `pub fn has_children(mut self, has_children: impl Fn(&T) -> bool + 'static) -> TreeView<T>` - Asking `has_children`, not `children`, whether a node has children.
- `pub fn node_key(mut self, node_key: impl Fn(&T) -> Key + 'static) -> TreeView<T>` - Keying each node by `node_key`.
- `pub fn controller(mut self, controller: TreeController) -> TreeView<T>` - Held by `controller`.
- `pub fn on_activated(mut self, handler: Handler<T>) -> TreeView<T> where T: Clone,` - Gives `handler` a clone of the selected node when Enter is pressed.
- `pub fn show_guides(mut self, on: bool) -> TreeView<T>` - Whether lines join the nodes to their parents.
- `pub fn separate_roots(mut self, on: bool) -> TreeView<T>` - Whether an empty row comes between two roots.
- `pub fn max_indent(mut self, cells: i32) -> TreeView<T>` - The indent grows to `cells` at most.
- `pub fn gutter(mut self, gutter: impl for<'a, 'b> Fn(&mut BuildCx<'b>, &TreeEntry<'a, T>) -> AnyWidget + 'static) -> TreeView<T>` - `gutter` in the first cells of each row.
- `pub fn gutter_width(mut self, cells: i32) -> TreeView<T>` - The gutter `cells` wide.
- `pub fn focus_node(mut self, node: FocusNode) -> TreeView<T>` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> TreeView<T>` - Taking the focus once mounted.
- `pub fn can_request_focus(mut self, on: bool) -> TreeView<T>` - Whether it can take the focus.
- `pub fn physics(mut self, physics: Rc<dyn ScrollPhysics>) -> TreeView<T>` - Scrolled by `physics`.
- `pub fn style(mut self, style: Style) -> TreeView<T>` - Every row in `style`, over the theme's.
- `pub fn guide_style(mut self, style: Style) -> TreeView<T>` - The guides in `style`.
- `pub fn selected_style(mut self, style: Style) -> TreeView<T>` - The row selected, with the focus, in `style`.
- `pub fn unfocused_selected_style(mut self, style: Style) -> TreeView<T>` - The row selected, without the focus, in `style`.

### Also used in the code

- `rtwk::data::TreeController` (struct, `crates/rtwk-data/src/tree.rs:148`) What a `TreeView` shows: the nodes expanded, by key (`is_expanded`), and the node selected (`selected_key`).
- `rtwk::data::NodeChildren` (enum, `crates/rtwk-data/src/tree.rs:336`) What `TreeView::children` gives for a node: its children, or a future of them (a spinner row shows below the node until it ends, then its list, which is kept by the node's key until `TreeController::reload`, or a row ...
  - variants: `List`, `Future`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## TreeTable

A tree in the first column of a table: rows that expand and collapse, sorted within their parents, selected one or many at a time.

API:

- `rtwk::data::TreeTable` (struct, `crates/rtwk-data/src/tree_table.rs:134`) `roots` and their descendants as the rows of a `Table` in `columns`: a tree whose nodes show their columns, as a process viewer, a file manager with sizes and dates, or a browser of JSON or of dependencies shows them.
  - public fields: `roots: Rc<[T]>`, `children: Rc<dyn Fn(&T) -> NodeChildren<T>>`, `columns: Rc<[TableColumn<T>]>`, `has_children: Option<Rc<dyn Fn(&T) -> bool>>`, `node_key: Rc<dyn Fn(&T) -> Key>`, `controller: Option<TreeController>`, `table_controller: Option<TableController>`, `selection: TableSelection`, `marks: Option<crate::SelectionController<Key>>`, `show_guides: bool`, `column_gap: i32`, `border: BorderType`, `show_header: bool`, `sample_rows: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `header_style: Option<Style>`, `border_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/data/treetable`, `crates/rtwk-gallery/src/pages/data_lists.rs:92`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const ROOTS: [&str; 4] = ["lib", "test", "README.md", "pubspec.yaml"];

fn children(path: &str) -> &'static [&'static str] {
    match path {
        "lib" => &["lib/src", "lib/twk.dart"],
        "lib/src" => &["lib/src/box.dart", "lib/src/flex.dart", "lib/src/table.dart", "lib/src/text.dart"],
        "test" => &["test/box_test.dart", "test/table_test.dart"],
        _ => &[],
    }
}

// Retain the controller in your state, initialized as:
// TreeController::new()
//     .with_expanded([Key::value("lib"), Key::value("lib/src")])
//     .with_selected_key(Key::value("lib/src/table.dart")).

fn lines(path: &str) -> usize {
    match path {
        "lib/src/box.dart" => 412,
        "lib/src/flex.dart" => 388,
        "lib/src/table.dart" => 960,
        "lib/src/text.dart" => 230,
        "lib/twk.dart" => 48,
        "test/box_test.dart" => 310,
        "test/table_test.dart" => 655,
        "README.md" => 120,
        "pubspec.yaml" => 22,
        _ => children(path).iter().map(|child| lines(child)).sum(),
    }
}

let columns = [
    TableColumn::new("name", |path: &&str| TableCell::new(
        path.rsplit('/').next().unwrap_or(path).to_owned(),
    ))
    .width(GridTrack::fr(1))
    .compare(Ord::cmp),
    TableColumn::new("lines", |path: &&str| TableCell::new(lines(path).to_string()))
        .align(TextAlign::Right)
        .compare(|a, b| lines(a).cmp(&lines(b))),
];
let tree = TreeTable::new(ROOTS, |path| NodeChildren::list(children(path)), columns)
    .has_children(|path| !path.contains('.'))
    .controller(controller)
    .table_controller(table_controller)
    .selection(TableSelection::Row)
    .border(BorderType::Single)
    .column_gap(3)
    .show_guides(true);

LayoutBuilder::new(move |_, room| {
    SizedBox::new(Some(40), Some(room.max_height.clamp(4, 12)), Some(tree.clone().into())).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| selection | none, row, cell | row |
| border | none, single, rounded, double, heavy, ascii | single |
| guides | on, off | true |

### Methods

#### `TreeTable`: constructors and associated functions

- `pub fn new(roots: impl Into<Rc<[T]>>, children: impl Fn(&T) -> NodeChildren<T> + 'static, columns: impl Into<Rc<[TableColumn<T>]>>) -> TreeTable<T>` - Shows `roots` and the children of those the controller expands as the rows of a table in `columns`; a node is its own key (see `node_key`).
- `pub fn keyed(roots: impl Into<Rc<[T]>>, children: impl Fn(&T) -> NodeChildren<T> + 'static, columns: impl Into<Rc<[TableColumn<T>]>>, node_key: impl Fn(&T) -> Key + 'static) -> TreeTable<T>` - As `TreeTable::new`, for nodes that are no values: each keyed by `node_key`.

#### `TreeTable`: builder methods (take and return `self`)

- `pub fn has_children(mut self, has_children: impl Fn(&T) -> bool + 'static) -> TreeTable<T>` - Asking `has_children`, not `children`, whether a node has children.
- `pub fn node_key(mut self, node_key: impl Fn(&T) -> Key + 'static) -> TreeTable<T>` - Keying each node by `node_key`.
- `pub fn controller(mut self, controller: TreeController) -> TreeTable<T>` - Expanding and selecting by `controller`.
- `pub fn table_controller(mut self, controller: TableController) -> TreeTable<T>` - Sorting, resizing and selecting a cell by `controller`.
- `pub fn selection(mut self, selection: TableSelection) -> TreeTable<T>` - Selecting as `selection` says.
- `pub fn marks(mut self, marks: impl Into<crate::SelectionController<Key>>) -> Self` - Show a leading checkbox column using a multiple selection controller keyed by `Self::node_key`.
- `pub fn on_activated(mut self, handler: Handler<T>) -> TreeTable<T> where T: Clone,` - Gives `handler` a clone of the selected node when Enter is pressed.
- `pub fn show_guides(mut self, on: bool) -> TreeTable<T>` - Whether lines join the nodes to their parents.
- `pub fn column_gap(mut self, gap: i32) -> TreeTable<T>` - `gap` cells between the columns.
- `pub fn border(mut self, border: BorderType) -> TreeTable<T>` - Lines of `border` between the columns and under the header.
- `pub fn show_header(mut self, on: bool) -> TreeTable<T>` - Whether the titles show.
- `pub fn sample_rows(mut self, rows: usize) -> TreeTable<T>` - Measured by `rows` besides those in view.
- `pub fn focus_node(mut self, node: FocusNode) -> TreeTable<T>` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> TreeTable<T>` - Taking the focus once mounted.
- `pub fn can_request_focus(mut self, on: bool) -> TreeTable<T>` - Whether it can take the focus.
- `pub fn style(mut self, style: Style) -> TreeTable<T>` - Every cell in `style`, over the theme's.
- `pub fn header_style(mut self, style: Style) -> TreeTable<T>` - The header in `style`.
- `pub fn border_style(mut self, style: Style) -> TreeTable<T>` - The lines in `style`.
- `pub fn guide_style(mut self, style: Style) -> TreeTable<T>` - The guides in `style`.
- `pub fn selected_style(mut self, style: Style) -> TreeTable<T>` - The row selected, with the focus, in `style`.
- `pub fn unfocused_selected_style(mut self, style: Style) -> TreeTable<T>` - The row selected, without the focus, in `style`.
- `pub fn cursor_style(mut self, style: Style) -> TreeTable<T>` - The cell selected in `style`.

### Also used in the code

- `rtwk::data::TreeController` (struct, `crates/rtwk-data/src/tree.rs:148`) What a `TreeView` shows: the nodes expanded, by key (`is_expanded`), and the node selected (`selected_key`).
- `rtwk::data::TableColumn` (struct, `crates/rtwk-data/src/table.rs:212`) A column of a `Table` whose rows are `T`s: its `title`, what it shows of a row (`cell`), how wide it is (`width`), where its text sits (`align`) and how text too wide for it is cut off (`overflow`), and how it orders ...
  - public fields: `title: Str`, `cell: Rc<dyn Fn(&T) -> TableCell>`, `width: GridTrack`, `align: TextAlign`, `overflow: TextOverflow`, `compare: Option<Rc<dyn Fn(&T`
- `rtwk::data::TableCell` (struct, `crates/rtwk-data/src/table.rs:114`) What a cell of a `Table` shows: text in one style (`TableCell::new`), text in spans (`TableCell::rich`), or a widget (`TableCell::widget`).
  - public fields: `text: Option<Str>`, `spans: Option<Rc<[Span]>>`, `style: Style`, `child: Option<AnyWidget>`
- `rtwk::layout::GridTrack` (struct, `crates/rtwk-render/src/objects/grid.rs:54`) The length of a column or a row of a `RenderGrid`, in whole cells: fixed, a fraction of the cells the other tracks leave (CSS `fr`), or as long as the children in it ask for (CSS `auto`).
  - public fields: `kind: GridTrackKind`, `fraction: i32`, `min: i32`, `max: i32`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`
- `rtwk::data::NodeChildren` (enum, `crates/rtwk-data/src/tree.rs:336`) What `TreeView::children` gives for a node: its children, or a future of them (a spinner row shows below the node until it ends, then its list, which is kept by the node's key until `TreeController::reload`, or a row ...
  - variants: `List`, `Future`
- `rtwk::data::TableSelection` (enum, `crates/rtwk-data/src/table.rs:301`) What the keys and the mouse select in a `Table`.
  - variants: `None`, `Row`, `Cell`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## DirectoryTree

The files under a directory as a tree for a side panel: each directory listed as it is opened and again as it changes, what .gitignore ignores left out.

API:

- `rtwk::data::DirectoryTree` (struct, `crates/rtwk-data/src/directory_tree.rs:963`) The files and directories under `root` as a tree, for a side panel: each directory listed as it is first expanded, and listed again while it shows as it changes on disk.
  - public fields: `root: String`, `file_system: SharedFileSystem`, `controller: Option<DirectoryTreeController>`, `on_selected: Option<rtwk_framework::Handler<String>>`, `on_opened: Option<rtwk_framework::Handler<String>>`, `show_hidden: bool`, `gitignore: bool`, `watch: bool`, `show_sizes: bool`, `byte_units: ByteUnits`, `filter: Option<FileFilter>`, `icon: Option<Rc<dyn Fn(&DirectoryEntry) -> Option<String>>>`, `show_guides: bool`, `focus_node: Option<FocusNode>`, `autofocus: bool`

### Code

The gallery's Code tab at the initial options (route `#/data/directorytree`, `crates/rtwk-gallery/src/pages/data_lists.rs:103`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const ROOT: &str = "/home/user/twk";

// Retain a DirectoryTreeController with ROOT/lib and ROOT/lib/src expanded.
// fixture_fs is an InMemoryFileSystem with project files and .gitignore.
// on_opened saves the path relative to ROOT as opened.
let tree = DirectoryTree::new(ROOT)
    .file_system(fixture_fs)
    .controller(controller)
    .show_hidden(false)
    .gitignore(true)
    .show_sizes(false)
    .show_guides(false)
    .on_opened(on_opened);

LayoutBuilder::new(move |cx, room| {
    let muted = Theme::of(cx).muted.clone();
    column![
        SizedBox::new(Some(36), Some((room.max_height - 2).clamp(2, 12)), Some(tree.clone().into())),
        Text::rich([
            Span::styled("opened: ", muted.clone()),
            match &opened {
                Some(path) => Span::new(path.clone()),
                None => Span::styled("nothing", muted),
            },
        ]),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
    .into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| hidden | on, off | false |
| gitignore | on, off | true |
| sizes | on, off | false |
| guides | on, off | false |
| icon | none, glyphs | none |

### Methods

#### `DirectoryTree`: constructors and associated functions

- `pub fn new(root: impl Into<String>) -> DirectoryTree` - A tree of the entries under `root`, relative to the current directory of the file system, `~` at its start the home directory.

#### `DirectoryTree`: builder methods (take and return `self`)

- `pub fn file_system(mut self, file_system: SharedFileSystem) -> Self` - Listing `file_system`.
- `pub fn controller(mut self, controller: DirectoryTreeController) -> Self` - Read and moved by `controller`.
- `pub fn on_selected(mut self, h: rtwk_framework::Handler<String>) -> Self` - Telling `h` the path of each entry selected.
- `pub fn on_opened(mut self, h: rtwk_framework::Handler<String>) -> Self` - Telling `h` the path of each file opened.
- `pub fn show_hidden(mut self, on: bool) -> Self` - Whether the entries whose names start with a dot show.
- `pub fn gitignore(mut self, on: bool) -> Self` - Whether what the `.gitignore` files ignore is hidden.
- `pub fn watch(mut self, on: bool) -> Self` - Whether the directories expanded are listed again as they change.
- `pub fn show_sizes(mut self, on: bool) -> Self` - Whether each file's size shows.
- `pub fn byte_units(mut self, units: ByteUnits) -> Self` - How the sizes are counted and named.
- `pub fn filter(mut self, filter: FileFilter) -> Self` - Showing the files `filter` accepts.
- `pub fn icon(mut self, icon: impl Fn(&DirectoryEntry) -> Option<String> + 'static) -> Self` - What shows before an entry's name.
- `pub fn show_guides(mut self, on: bool) -> Self` - Whether lines join the entries to their directories.
- `pub fn focus_node(mut self, node: FocusNode) -> Self` - The node of the focus.
- `pub fn autofocus(mut self, on: bool) -> Self` - Whether it takes the focus as it is first built.

### Also used in the code

- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## TaskList

Tasks and how far they have got: pending, running with a spinner and the time it has run, done, failed or skipped; their steps below.

API:

- `rtwk::data::TaskList` [path inferred] (struct, `crates/rtwk-data/src/task_list.rs:156`) Tasks and how far they have got, a row each: build steps, an installer's stages, what an agent does.
  - public fields: `tasks: Rc<[TaskItem]>`, `spinner: Option<SpinnerFrames>`, `show_elapsed: bool`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/data/tasklist`, `crates/rtwk-gallery/src/pages/data_lists.rs:116`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Advance the build from the application clock at phase boundaries.
let tasks = live_tasks(&Run::at(elapsed_ms, false), false);
let list = TaskList::new(tasks)
    .spinner(SpinnerFrames::DOTS)
    .show_elapsed(true);
SizedBox::new(Some(40), None, Some(list.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| step | live, fetch, compile, test, deploy, done | live |
| failed | on, off | false |
| elapsed | on, off | true |
| spinner | dots, line, arc | dots |

### Methods

#### `TaskList`: constructors and associated functions

- `pub fn new(tasks: impl IntoIterator<Item = TaskItem>) -> TaskList` - Shows `tasks`, a row each.

#### `TaskList`: builder methods (take and return `self`)

- `pub fn spinner(mut self, spinner: SpinnerFrames) -> TaskList` - What a running task's mark shows in turn.
- `pub fn show_elapsed(mut self, show: bool) -> TaskList` - Whether a row shows the time its task has run.
- `pub fn style(mut self, style: Style) -> TaskList` - A row's style.

### Also used in the code

- `rtwk::style::SpinnerFrames` (struct, `crates/rtwk-widgets/src/style/glyphs.rs:18`) The frames of a spinner, shown in turn, each for `interval` (twk's `SpinnerFrames`, of `animation/spinner.dart`: defined here, where `Glyphs::spinner` needs it; the animation module's `Spinner` uses this type).
  - public fields: `frames: Cow<'static`, `interval: Duration`
