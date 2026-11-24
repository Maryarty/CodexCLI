<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Data

Tables, trees, charts, gauges, sparklines and calendars.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Sparkline

A series of values as bars an eighth of a cell fine, the newest on the right.

- `rtwk::data::Sparkline` (struct) in `crates/rtwk-data/src/sparkline.rs:48`: `data` as bars a column wide that grow upwards in eighths of a cell, oldest to the left: the last values that fit, the newest at the right edge once they fill it.

### Code

```rust
fn sparkline(cx: &mut Cx<'_>, elapsed_ms: i64) -> SizedBox {
    // Redraw from the application clock every 150 ms.
    let step: i64 = elapsed_ms / 150;
    let data: Vec<_> = (step..step + 48).map(|i| 4.0 + 3.0 * (i as f64 / 4.0).sin()).collect();
    let spark = Sparkline::new(data)
        .style(Theme::of(cx).info.clone());

    SizedBox::new(Some(48), Some(4), Some(spark.into()))
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| data | wave, ramp, steps, noise | wave |
| rows | 1, 2, 4 | 4 |
| max | largest, 10 | largest |
| live | false, true | true |
| style | info, primary, accent, success, warning | info |

## Gauge, LineGauge

How much of something there is: a block filled to a ratio, or a line.

- `rtwk::data::Gauge` (struct) in `crates/rtwk-data/src/gauge.rs:60`: `ratio` of its width filled from the left, in eighths of a cell, on every row, with `label` in the middle of the middle row, or with show_percentage the ratio as a percentage.
- `rtwk::data::LineGauge` (struct) in `crates/rtwk-data/src/gauge.rs:215`: `ratio` as a line after `label` (or with show_percentage the ratio as a percentage) and a blank: the part from the left filled in filled_style, the rest in unfilled_style, to the nearest half cell where kind has a line that ends in the middle of a cell.

### Code

```rust
fn gauges(cx: &mut Cx<'_>, elapsed_ms: i64) -> SizedBox {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| live | false, true | true |
| ratio | 0%, 1%, 2%, 3%, 4%, 5%, 6%, 7%, 8%, 9%, 10%, 11%, 12%, 13%, 14%, 15%, 16%, 17%, 18%, 19%, 20%, 21%, 22%, 23%, 24%, 25%, 26%, 27%, 28%, 29%, 30%, 31%, 32%, 33%, 34%, 35%, 36%, 37%, 38%, 39%, 40%, 41%, 42%, 43%, 44%, 45%, 46%, 47%, 48%, 49%, 50%, 51%, 52%, 53%, 54%, 55%, 56%, 57%, 58%, 59%, 60%, 61%, 62%, 63%, 64%, 65%, 66%, 67%, 68%, 69%, 70%, 71%, 72%, 73%, 74%, 75%, 76%, 77%, 78%, 79%, 80%, 81%, 82%, 83%, 84%, 85%, 86%, 87%, 88%, 89%, 90%, 91%, 92%, 93%, 94%, 95%, 96%, 97%, 98%, 99%, 100% | 75% |
| line | single, heavy, double, ascii | heavy |
| label | own, percent, none | own |
| style | primary, accent, success, warning, info | primary |
| segmented | false, true | false |

## BarChart

Bars in groups, up or across, their values on them and their labels under them.

- `rtwk::data::BarChart` (struct) in `crates/rtwk-data/src/bar_chart.rs:177`: The bars of `groups` side by side, bar_width cells thick and bar_gap apart, the groups group_gap apart; up from the bottom, or with a direction of Axis::Horizontal from the left, in eighths of a cell.

### Code

```rust
fn bars(elapsed_ms: i64) -> LayoutBuilder {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| direction | vertical, horizontal | vertical |
| bar width | 1, 2, 3 | 3 |
| bars | grouped, single | grouped |
| live | false, true | true |

## Chart

Datasets of points on two axes, as lines or dots, with a legend.

- `rtwk::data::Chart` (struct) in `crates/rtwk-data/src/chart.rs:220`: `datasets` plotted against x_axis and y_axis, with a marker finer than the cells (see DotCanvas), each dataset covering those before it cell by cell.

### Code

```rust
fn chart(elapsed_ms: i64) -> LayoutBuilder {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| graph | scatter, line | line |
| marker | BRAILLE, HALF_BLOCK, QUADRANT, SEXTANT, OCTANT, BLOCK, DOT, BAR, custom | BRAILLE |
| legend | TopLeft, Top, TopRight, Left, Right, BottomLeft, Bottom, BottomRight, None | TopRight |
| live | false, true | true |

## DotCanvas

Shapes drawn finer than the cells, in data coordinates, with labels over them.

- `rtwk::data::DotCanvas` (struct) in `crates/rtwk-data/src/dot_canvas.rs:79`: Shapes and labels drawn finer than the cells: `draw` sets dots at points of a data space (x_bounds across, y_bounds upwards), which marker shows several to a cell.

### Code

```rust
fn dots(elapsed_ms: i64) -> LayoutBuilder {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| marker | BRAILLE, HALF_BLOCK, QUADRANT, SEXTANT, OCTANT, BLOCK, DOT, BAR, custom | BRAILLE |
| live | false, true | true |

## Calendar

A month to pick a day of: the arrows move the cursor, PageUp and PageDown a month, Enter or a click picks the day.

- `rtwk::data::Calendar` (struct) in `crates/rtwk-data/src/calendar.rs:422`: The days of a month in weeks, a row each from first_weekday — six rows, so that every month is as tall — under a row of the weekdays' names and, with show_header, the month's name and year, centered: the month of the controller's cursor.

### Code

```rust
fn calendar(cx: &mut Cx<'_>, controller: CalendarController, selected: Rc<ValueNotifier<Option<String>>>) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| first day | monday, sunday | monday |
| week numbers | false, true | false |
| outside days | false, true | false |
| marked days | false, true | false |

## Heatmap, ActivityHeatmap

Values over a grid as shades or colors, its rows and columns labelled; counts by day over weeks, a column a week. The arrows move a cursor, its tooltip saying what is there.

- `rtwk::data::Heatmap` (struct) in `crates/rtwk-data/src/heatmap.rs:220`: `rows` × `columns` cells, each the value there on the scale: from min, by default the smallest value, to max, by default the largest; those beyond them as they are.
- `rtwk::data::ActivityHeatmap` (struct) in `crates/rtwk-data/src/heatmap.rs:1836`: Counts by day as a Heatmap, a column a week from first_weekday and a row a weekday, the last column the week of end: weeks weeks, the oldest at the left.

### Code

```rust
fn heatmap(selected: Rc<ValueNotifier<Option<String>>>) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| widget | ActivityHeatmap, Heatmap | ActivityHeatmap |
| first day | monday, sunday | monday |
| scale | shades, colors | shades |
| legend | false, true | true |
| cell width | 2 columns, 1 column | 2 columns |

## Table

Rows in columns that size by their content or a share, sorted by a click on a title; the arrows select a row or a cell.

- `rtwk::data::Table` (struct) in `crates/rtwk-data/src/table.rs:700`: A table of rows in columns under a header of their titles, which stays while the rows scroll.

### Code

```rust
#[derive(Clone, Debug, PartialEq)]
struct FileRow {
    name: String,
    lines: usize,
    coverage: f64,
}

fn files(controller: TableController, on_activated: Handler<FileRow>) -> LayoutBuilder {
    let columns = [
        TableColumn::new("name", |row: &FileRow| TableCell::new(row.name.clone()))
            .compare(|a, b| a.name.cmp(&b.name)),
        TableColumn::new("lines", |row: &FileRow| {
            TableCell::rich([Span::new(row.lines.to_string()).bold()])
        })
        .align(TextAlign::Right)
        .compare(|a, b| a.lines.cmp(&b.lines)),
        TableColumn::new("covered", |row: &FileRow| {
            TableCell::widget(
                LineGauge::new(row.coverage)
                    .label(format!("{:.0}%", row.coverage * 100.0))
                    .kind(BorderType::Heavy),
            )
        })
        .width(GridTrack::fr(1)),
    ];
    // Retain a controller initialized with Key::value("container.rs".to_owned()).
    let files = [
        ("container.rs", 412, 0.9),
        ("text.rs", 230, 0.75),
        ("flex.rs", 388, 0.6),
        ("grid.rs", 520, 0.4),
        ("stack.rs", 145, 1.0),
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| selection | none, row, cell | row |
| border | none, single, rounded, double, heavy, ascii | single |
| header | false, true | true |
| rows | 5, 10 000 | 5 |

## DescriptionList

Terms and their values, the values in a column of their own, aligned group by group; a long value wraps under itself, and on a narrow screen each goes under its term.

- `rtwk::data::DescriptionList` (struct) in `crates/rtwk-data/src/description_list.rs:171`: Terms and their values, a row each, the values in a column of their own: what `docker inspect`, `kubectl describe` or the header of `htop` prints.

### Code

```rust
let theme = Theme::of(cx);
let description = DescriptionList::new([
    DescriptionItem::new("Status", Text::rich([
        Span::styled("● ", theme.success.clone()),
        Span::new("Running (healthy)"),
    ])),
    DescriptionItem::text("IP Address", "192.168.1.42"),
    DescriptionItem::text("Uptime", "3 days, 4 hours"),
    DescriptionItem::text("Image", "rtwk/server:0.1.0"),
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

| option | values | starts at |
| --- | --- | --- |
| width | 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells, 57 cells, 58 cells, 59 cells, 60 cells | 44 cells |
| terms | left, center, right | left |
| separator | colon, none | colon |
| groups | false, true | true |

## TreeView

Nodes under their parents: the arrows move and expand, Enter or a click on the expander opens one; tests/ loads a while.

- `rtwk::data::TreeView` (struct) in `crates/rtwk-data/src/tree.rs:1195`: `roots` and their descendants, each node a row below its parent and indented a level more, those whose parent controller expands; a node with children shows `▸` before its label, `▾` while expanded.

### Code

```rust
const ROOTS: [&str; 4] = ["src", "tests", "README.md", "Cargo.toml"];

fn children(path: &str) -> &'static [&'static str] {
    match path {
        "src" => &["src/widgets", "src/lib.rs"],
        "src/widgets" => &["src/widgets/container.rs", "src/widgets/flex.rs", "src/widgets/table.rs", "src/widgets/text.rs"],
        "tests" => &["tests/widgets.rs", "tests/table.rs"],
        _ => &[],
    }
}

fn tree(controller: TreeController, load_test: fn(Result<Rc<[&'static str]>, String>) -> NodeChildren<&'static str>) -> LayoutBuilder {
    // Retain the controller in your state, initialized as:
    // TreeController::new()
    //     .with_expanded([Key::value("src"), Key::value("src/widgets")])
    //     .with_selected_key(Key::value("src/widgets/table.rs")).
    // load_test creates a cancellable future that completes after one second.
    let tree = TreeView::new(
        ROOTS,
        move |path| {
            if *path == "tests" {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| guides | false, true | true |
| separate roots | false, true | false |
| max indent | none, 2 | none |
| tests/ | loads, fails | loads |

## TreeTable

A tree in the first column of a table: rows that expand and collapse, sorted within their parents, selected one or many at a time.

- `rtwk::data::TreeTable` (struct) in `crates/rtwk-data/src/tree_table.rs:134`: `roots` and their descendants as the rows of a Table in columns: a tree whose nodes show their columns, as a process viewer, a file manager with sizes and dates, or a browser of JSON or of dependencies shows them.

### Code

```rust
const ROOTS: [&str; 4] = ["src", "tests", "README.md", "Cargo.toml"];

fn children(path: &str) -> &'static [&'static str] {
    match path {
        "src" => &["src/widgets", "src/lib.rs"],
        "src/widgets" => &["src/widgets/container.rs", "src/widgets/flex.rs", "src/widgets/table.rs", "src/widgets/text.rs"],
        "tests" => &["tests/widgets.rs", "tests/table.rs"],
        _ => &[],
    }
}

fn lines(path: &str) -> usize {
    match path {
        "src/widgets/container.rs" => 412,
        "src/widgets/flex.rs" => 388,
        "src/widgets/table.rs" => 960,
        "src/widgets/text.rs" => 230,
        "src/lib.rs" => 48,
        "tests/widgets.rs" => 310,
        "tests/table.rs" => 655,
        "README.md" => 120,
        "Cargo.toml" => 22,
        _ => children(path).iter().map(|child| lines(child)).sum(),
    }
}

fn tree_table(controller: TreeController, table_controller: TableController) -> LayoutBuilder {
    // Retain the controller in your state, initialized as:
    // TreeController::new()
    //     .with_expanded([Key::value("src"), Key::value("src/widgets")])
    //     .with_selected_key(Key::value("src/widgets/table.rs")).
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| selection | none, row, cell | row |
| border | none, single, rounded, double, heavy, ascii | single |
| guides | false, true | true |

## DirectoryTree

The files under a directory as a tree for a side panel: each directory listed as it is opened and again as it changes, what .gitignore ignores left out.

- `rtwk::data::DirectoryTree` (struct) in `crates/rtwk-data/src/directory_tree.rs:968`: The files and directories under root as a tree, for a side panel: each directory listed as it is first expanded, and listed again while it shows as it changes on disk.

### Code

```rust
const ROOT: &str = "/home/user/rtwk";

fn directory(controller: DirectoryTreeController, fixture_fs: SharedFileSystem, on_opened: Handler<String>, opened: Option<String>) -> LayoutBuilder {
    // Retain a DirectoryTreeController with ROOT/src and ROOT/src/widgets expanded.
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| hidden files | false, true | false |
| .gitignore | false, true | true |
| sizes | false, true | false |
| guides | false, true | false |
| icons | none, glyphs | none |

## TaskList

Tasks and how far they have got: pending, running with a spinner and the time it has run, done, failed or skipped; their steps below.

- `rtwk::data::TaskList` (struct) in `crates/rtwk-data/src/task_list.rs:156`: Tasks and how far they have got, a row each: build steps, an installer's stages, what an agent does.

### Code

```rust
fn build_list(tasks: [TaskItem; 4]) -> SizedBox {
    // tasks advance from the application clock at phase boundaries: each step
    // is Pending, then Running with its elapsed time, then Done.
    let list = TaskList::new(tasks)
        .spinner(SpinnerFrames::DOTS)
        .show_elapsed(true);
    SizedBox::new(Some(40), None, Some(list.into()))
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| step | live, fetch, compile, test, deploy, done | live |
| failed | false, true | false |
| elapsed | false, true | true |
| spinner | dots, line, arc | dots |
