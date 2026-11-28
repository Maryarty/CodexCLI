<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Layout

<!-- not rtwk: Padding -->

Layout is by constraints: a parent gives each child constraints, the child
picks a size within them, and the parent places it. Everything is whole cells,
so sizes, paddings and offsets are columns and rows, optional `i32`s in the
API (`SizedBox::new(Some(20), None, Some(child))`).

## Rows and columns

<!-- file: crates/rtwk/examples/guides/layout.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::ViewCx;
use rtwk::focus::KeyBindings;
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, SizedBox,
};
use rtwk::prelude::*;
use rtwk::style::Span;
use rtwk::widgets::Builder;

pub fn main() -> ExitCode {
    run_app((), view)
}

pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    KeyBindings::new(
        [("q", cx.on(|_, cx| cx.quit()))],
        Builder::new(|cx| {
            Column::new()
                // each child as wide as the column, not centred across it
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::rich([
                    Span::new(" rtwk").bold(),
                    Span::new(" · layout").dim(),
                ]))
                // the rows the title and the hints leave
                .child(Expanded::new(
                    1,
                    Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        // a fixed width
                        .child(SizedBox::new(
                            Some(20),
                            None,
                            Some(
                                BoxWidget::new()
                                    .border(Border::all(Style::new()).kind(BorderType::Rounded))
                                    .title(BorderTitle::new(" Files "))
                                    .padding(EdgeInsets::x(1))
                                    .child(Text::new("src/\ntests/\nCargo.toml"))
                                    .into(),
                            ),
                        ))
                        // the columns the files leave
                        .child(Expanded::new(
                            1,
                            BoxWidget::new()
                                .border(Border::all(Style::new()).kind(BorderType::Rounded))
                                .title(BorderTitle::new(" Preview "))
                                .padding(EdgeInsets::x(1))
                                .child(Text::new(
                                    "Everything is measured in cells. A Row gives its \
                                     fixed children their width first, then shares what \
                                     is left among the Expanded ones.",
                                )),
                        )),
                ))
                .child(Text::new(" q quits").style(Theme::of(cx).muted.clone()))
                .into()
        }),
    )
    .into()
}
```

```text
 rtwk · layout
╭ Files ───────────╮╭ Preview ───────────────────╮
│ src/             ││ Everything is measured in  │
│ tests/           ││ cells. A Row gives its     │
│ Cargo.toml       ││ fixed children their width │
╰──────────────────╯╰────────────────────────────╯
 q quits
```

- `Column` and `Row` lay their children out along their axis (`.child(..)`, or
  the `column!` and `row!` macros of the facade). A child with a fixed size
  gets it first; `Expanded::new(flex, child)` and `Flexible` share the rest by
  their `flex`, and `Spacer::new(flex)` is an empty `Expanded`.
  `.spacing(n)` puts cells between the children.
- Across their axis they centre their children unless
  `.cross_axis_alignment(..)` says otherwise: `CrossAxisAlignment::Stretch` makes
  each as wide (or tall) as the row or column, `Start` puts them at its start.
- `SizedBox` gives a fixed size, `rtwk::layout::Padding` empty cells around a
  child, `Center` and `Align` place a child in the room it is given.
- `BoxWidget` is the workhorse (`Box` is std's): a `.border(..)`
  (`Border::all(Style::new()).kind(BorderType::Rounded)`, or `BorderType::Single`,
  `BorderType::Double`, `BorderType::Heavy`, `BorderType::Ascii`), `.title(..)` on it, `.padding(..)` inside it and a
  `.style(..)` that fills it.

## Room that has no end

`ListView`, `Table`, `TreeView` and other scrolling widgets take all the room
they get. In a `Column` put them in `Expanded`, or they take the whole height
and push the rest off the screen (an overflow the log reports). Inline, where
the app's height is what it asks for, a list asks for none: give it a height
with `SizedBox::new(None, Some(8), Some(list))`. A list or a scroll view left
no rows records it at the debug level each time it goes to none (not a
warning: a panel closing does the same), which the debug console shows and a
failing test prints.

What has no end to grow into takes its child's size instead: an `Align` or a
`Center` in a scroll view, or in a list's row, whose height has no end, is as
tall as its child, so `Align::new(Alignment::CENTER_LEFT, ..)` in a row of a
`ListView` keeps the row one line tall.

## Room that is given

Where the room has an end, the same widgets take all of it.

- A `Column` or `Row` is as long as the room along its axis
  (`MainAxisSize::Max`, the default), however little its children need, also
  inside another `Column`: a header made of a `Column` of two lines, above a
  list in an `Expanded`, is as tall as the whole room and leaves the list no
  rows. A panel in a `Column` takes the screen the same way. Give what should
  be as big as its children `.main_axis_size(MainAxisSize::Min)`.
- An `Align` or a `Center` with no factor is as big as the room, on both axes:
  in a `Row` that has a height (a line of a list beside other widgets, in a
  `SizedBox` or an `Expanded`) it takes the whole height. `.height_factor(1)`
  (`.width_factor(1)` along the other axis) makes it as tall as its child. To
  keep a child to the left or at the top there is often no `Align` to write:
  `cross_axis_alignment(CrossAxisAlignment::Start)` of the `Row` or `Column`
  places its children across it, `main_axis_alignment` along it.

## A row of a list

A row with a label that gives way and something fixed after it needs no
`LayoutBuilder` and no measuring of text:
`Row::new().child(Expanded::new(1, Text::line(label))).child(Text::new(" 12 KB"))`.
The `Row` gives the fixed child its width, `Expanded` the label the rest, and
`Text::line` cuts it with `…` in cells (wide letters whole; `Text::rich_line`
for spans). Columns of fixed widths are the same with
`SizedBox::new(Some(n), None, Some(Text::line(..).into()))`, a number aligned
right `Text::new(n).text_align(TextAlign::Right).wrap(TextWrap::None)`. Several rows
that share their columns, with a header, sorting and a cursor, are a `Table`.
The cursor's `▎` bar, the idle selection and the keys come from
`SelectionList` (its item builder builds the row), a title over a subtitle
with something before and after from `ListTile`.

A list that loads more near its end, or shows where it is, listens to the
scrolling. A `ScrollController` notifies on its first layout and whenever the
view moves: listen to it (`ListenableBuilder`) and read
`try_position()`: `pixels()`, `max_scroll_extent()` and `viewport_dimension()`
(the [recipes](recipes.md#a-pager-from-the-scroll-controller) show it). A
`NotificationListener<ScrollUpdateNotification>` above the list reads
`metrics.extent_after()` (cells of content after the window; `at_end()`) of the
keys, the wheel, a controller and a `SelectionList`'s cursor moving it. A list
that all fits sends no notification, and none is sent for the first layout:
ask for the first page by the size of what is loaded.

## Text in cells

The list row above cuts a label that is on the screen. A `String` an app holds,
cut before it is a widget, is
`fit_text(&segmenter_of(cx), text, n, TextOverflow::Ellipsis, "…")`
(`rtwk_core::unicode`): at most `n` cells, `…` in place of the rest, no wide
letter split. `segmenter_of(cx)` (`rtwk::text`) is the terminal's own
`TextSegmenter`; a plain `TextSegmenter::default()` counts a character of
ambiguous width as one cell, where an ambiguous-wide terminal counts two and
the cut leaves a cell off. Use the same for `seg.width(text)`, and prefer the
widget wherever the text is shown at once. A string padded to `n` cells is
`pad_cells(&seg, text, n, TextAlign::Right)` (or `TextAlign::Left`, `TextAlign::Center`), counted in
cells, not in `len()`. `TextOverflow::EllipsisStart` or `TextOverflow::EllipsisMiddle` cut the
start or the middle, and `Clip` clips. `break_lines(&seg, text, n, ..)` gives
the (start, end) offsets of the lines a text breaks into at `n` cells.

A row's background across its whole width is a `ColoredBox` around it: a
`Span`'s background (`Span::new("x").bg(color)`) paints only the cells of its
text. A mark on every line a text wraps into, a quotation's bar, is a
`BoxWidget` with a border on its left only (`Border::new().left(Style::new())`;
its `.kind(..)` picks the line, `BorderType::Ascii` makes it `|`), and an indent
a `Padding`; the text wraps inside what they leave, wide letters and all, and
nothing is counted by hand.

A space at the end of a text is not a cell of it. With `TextWrap::Word`, the
default, the spaces that end a line hang past it, neither counted nor drawn, so
`Text::new("◐ ")` in a `Row` is one cell wide and the `Text` after it touches
the mark. `Text::new("◐ ").wrap(TextWrap::None)` keeps the space: two cells. (A
mark and its gap are better apart: a `SizedBox::new(Some(1), None, None)`
between them.)

<!-- file: crates/rtwk/examples/guides/cell_text.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::ViewCx;
use rtwk::focus::KeyBindings;
use rtwk::layout::{Border, BoxWidget, ColoredBox, CrossAxisAlignment, EdgeInsets};
use rtwk::prelude::*;
use rtwk::text::{TextWrap, segmenter_of};
use rtwk::widgets::Builder;
use rtwk_core::unicode::{TextAlign, TextOverflow, fit_text, pad_cells};

pub fn main() -> ExitCode {
    run_app((), view)
}

pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    KeyBindings::new(
        [("q", cx.on(|_, cx| cx.quit()))],
        Builder::new(|cx| {
            let seg = segmenter_of(cx);
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                // a String cut to 12 cells, wide letters whole
                .child(Text::new(
                    fit_text(
                        &seg,
                        "Warnings (12) and 你好世界",
                        12,
                        TextOverflow::Ellipsis,
                        "…",
                    )
                    .into_owned(),
                ))
                // the row's background across its whole width, not under its text
                .child(ColoredBox::new(Color::BLUE, Text::new(" NORMAL ")))
                // a gutter on every line the text breaks into: a left border
                .child(
                    BoxWidget::new()
                        .border(Border::new().left(Style::new()))
                        .padding(EdgeInsets::only(1, 0, 0, 0))
                        .child(Text::new(
                            "A quotation with 你好世界 that wraps over lines.",
                        )),
                )
                // a String padded to 10 cells by the cells it takes, not its length
                .child(Text::new(format!(
                    "[{}]",
                    pad_cells(&seg, "你好", 10, TextAlign::Left)
                )))
                // a space at the end of a wrapped text hangs; with none it stays
                .child(
                    Row::new()
                        .child(Text::new("◐ "))
                        .child(Text::new("|"))
                        .child(Text::new("◐ ").wrap(TextWrap::None))
                        .child(Text::new("|")),
                )
                .into()
        }),
    )
    .into()
}
```

```text
Warnings (1…
 NORMAL
│ A quotation with
│ 你好世界 that
│ wraps over lines.
[你好      ]
◐|◐ |
```

## Limits and the room

`SizedBox` gives one size. `ConstrainedBox` keeps its child within limits and
leaves the rest to it: `BoxConstraints::new(min_width, max_width, min_height,
max_height)` counted in cells, with `UNBOUNDED` for a limit that limits nothing
and `BoxConstraints::tight(w, h)` for one size. Whatever is inside still gets
no more than its parent gave. `LayoutBuilder` hands its builder the
constraints it was given, `max_width` and `max_height` the room in cells, to
choose by: the app below shows the room and keeps a text to 30 cells.

<!-- file: crates/rtwk/examples/guides/constraints.rs -->
```rust
use std::process::ExitCode;

use rtwk::LayoutBuilder;
use rtwk::app::ViewCx;
use rtwk::focus::KeyBindings;
use rtwk::layout::{
    Border, BorderType, BoxConstraints, BoxWidget, ConstrainedBox, CrossAxisAlignment, UNBOUNDED,
};
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    run_app((), view)
}

pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    KeyBindings::new(
        [("q", cx.on(|_, cx| cx.quit()))],
        LayoutBuilder::new(|_, constraints| {
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                // the room this part of the app is given, in cells
                .child(Text::new(format!(
                    "room: {} x {}",
                    constraints.max_width, constraints.max_height
                )))
                // min_width, max_width, min_height, max_height: at most 30 wide, any height
                .child(ConstrainedBox::new(
                    BoxConstraints::new(10, 30, 0, UNBOUNDED),
                    BoxWidget::new()
                        .border(Border::all(Style::new()).kind(BorderType::Rounded))
                        .child(Text::new(
                            "A message that wraps at thirty cells, however wide the \
                             terminal is.",
                        )),
                ))
                .into()
        }),
    )
    .into()
}
```

```text
room: 50 x 8
╭──────────────────────────╮
│A message that wraps at   │
│thirty cells, however wide│
│the terminal is.          │
╰──────────────────────────╯
```

A `LayoutBuilder` builds again when its constraints change, as a resize does;
the [hints](#hints-that-depend-on-the-layout) below choose a layout with one.

## Hints that depend on the layout

`KeyHintsBar` shows the keys of the `KeyBindings` on the path of the focus. A
key that exists only in one layout, `s` here when the panes are side by side,
belongs to a `KeyBindings` below the `LayoutBuilder`, which knows the layout:
its handler is `None` in the other one, and a key bound to `None` is no key, so
the bar leaves it out and the key does nothing. The layer is always there, so
the focus inside it stays where it is as the terminal is resized. The bar
follows a resize a frame later: the `LayoutBuilder` builds in the layout of the
frame the resize draws, and the keys it changes are built in the frame after,
which is asked for at once.

<!-- file: crates/rtwk/examples/guides/hints.rs -->
```rust
use std::process::ExitCode;

use rtwk::LayoutBuilder;
use rtwk::app::ViewCx;
use rtwk::controls::KeyHintsBar;
use rtwk::focus::KeyBindings;
use rtwk::layout::{BoxWidget, CrossAxisAlignment};
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    run_app(Hints::default(), view)
}

#[derive(Default)]
pub struct Hints {
    pub swapped: bool,
}

pub fn view(app: &Hints, cx: &mut ViewCx<Hints>) -> AnyWidget {
    let swapped = app.swapped;
    let swap = cx.on(|app, _| app.swapped = !app.swapped);
    KeyBindings::new(
        [("q", cx.on(|_, cx| cx.quit()))],
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Expanded::new(
                1,
                LayoutBuilder::new(move |_, constraints| {
                    let wide = constraints.max_width >= 40;
                    let one = || BoxWidget::new().child(Text::new("one"));
                    let two = || BoxWidget::new().child(Text::new("two"));
                    // below the LayoutBuilder, so the layout decides the key; the layer stays
                    // in the tree, only its key comes and goes
                    KeyBindings::new(
                        // none in a narrow layout: no key, so no hint
                        [("s", wide.then(|| swap.clone()))],
                        Focus::new(if wide {
                            let (left, right) = if swapped {
                                (two(), one())
                            } else {
                                (one(), two())
                            };
                            Row::new()
                                .child(Expanded::new(1, left))
                                .child(Expanded::new(1, right))
                                .into()
                        } else {
                            AnyWidget::from(one())
                        })
                        .autofocus(),
                    )
                    .hints([("s", "swap")])
                    .into()
                }),
            ))
            .child(KeyHintsBar::new()),
    )
    .hints([("q", "quit")])
    .into()
}
```

A `KeyBindings` above the `LayoutBuilder` has no layout to ask, and shows its
hint in every one.

## More layouts

`Stack` and `Positioned` layer children, `Wrap` flows them onto the next row,
`Grid` places them in tracks whose borders can merge, and `SplitView` gives
panes a divider the mouse drags. All of them are in the catalog's
[Layout](../widgets/layout.md) group, with the Rust that builds each.
