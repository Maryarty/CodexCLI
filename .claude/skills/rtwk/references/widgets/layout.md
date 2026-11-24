<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Layout

Boxes, rows and columns, stacks, grids and wraps: where a child goes and how much room it gets.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Box

A border, titles on it, padding and a background around a child.

- `rtwk::layout::BoxWidget` (struct) in `crates/rtwk-widgets/src/layout/box_.rs:226`: A box around `child`: `style` fills it, `border` lines its sides, `titles` sit on its top and bottom edges, and `padding` is between the border and the child (twk's `Box`, over a `RenderDecoratedBox`).
- `rtwk::layout::Border` (struct) in `crates/rtwk-widgets/src/layout/box_.rs:44`: The border of a BoxWidget: the lines it is drawn with, for each side that has one its style, and whether it merges with the lines already drawn.

### Code

```rust
let theme = Theme::of(cx);
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .padding(EdgeInsets::all(1))
    .child(Text::new("Compiling 14 crates…\nrtwk-layout     0.4 s\nrtwk-render     0.9 s"))
    .titles([
        BorderTitle::new(" build ")
            .style(theme.text.merge(&Style::new().attrs(Attrs::BOLD)))
            .alignment(Alignment::TOP_LEFT),
        BorderTitle::new(" 2 warnings ")
            .style(theme.warning.clone())
            .alignment(Alignment::BOTTOM_RIGHT),
    ]);
SizedBox::new(Some(36), Some(8), Some(panel.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| border | single, rounded, double, heavy, ascii, none | rounded |
| sides | all, top bottom, top left, left | all |
| titles | false, true | true |
| title at | TOP_LEFT, TOP_CENTER, TOP_RIGHT, BOTTOM_LEFT, BOTTOM_CENTER | TOP_LEFT |
| padding | 0, 1, 2 | 1 |
| surface | false, true | false |

## ColoredBox, DecoratedBox

A child's cells filled with a color, and a BoxDecoration (a fill, a border, titles) drawn under the child or over its edges.

- `rtwk::layout::ColoredBox` (struct) in `crates/rtwk-widgets/src/layout/container.rs:108`: Fills its cells with color, then paints its child over it: the child's text takes the color as its background where it sets none.
- `rtwk::layout::DecoratedBox` (struct) in `crates/rtwk-widgets/src/layout/container.rs:267`: Paints decoration under child or over it, as position says (twk's `DecoratedBox`).

### Code

```rust
let theme = Theme::of(cx);
let decoration = BoxDecoration::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .title(BorderTitle::new(" notes ").style(theme.muted.clone()));
let notes = Padding::new(EdgeInsets::all(1), Text::new("release 0.4.0\n3 commits, 2 fixes"));
DecoratedBox::new(decoration, ColoredBox::new(theme.selection.bg.unwrap_or(Color::DEFAULT), notes))
    .position(DecorationPosition::Background)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| position | background, foreground | background |
| fill | ColoredBox, gradient | ColoredBox |
| border | single, rounded, double, heavy, ascii | rounded |
| titles | false, true | true |

## Container

Align, Padding, a color or a decoration, a size and a margin put together around one child.

- `rtwk::layout::Container` (struct) in `crates/rtwk-widgets/src/layout/container.rs:408`: The layout widgets with one child, put together as Flutter's `Container` puts them, from the inside out (twk's `Container`): `child`, or without one a box as big as the constraints allow (as small along an unbounded axis); Align by `alignment`; Padding by `padding`; ColoredBox in `color`; DecoratedBox with `decoration`, then with `foreground_decoration` over it all; ConstrainedBox within `constraints`, tightened to `width` and `height`; Padding by `margin`.

### Code

```rust
let theme = Theme::of(cx);
Container::new()
    .margin(EdgeInsets::all(1))
    .padding(EdgeInsets::all(1))
    .width(30)
    .height(7)
    .alignment(Alignment::CENTER)
    .decoration(BoxDecoration::new()
        .style(theme.surface.clone())
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded)))
    .child(Text::new("deploy to staging"))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| fill | decoration, color | decoration |
| alignment | None, TOP_LEFT, CENTER, BOTTOM_RIGHT | CENTER |
| width | none, 24, 30, 36 | 30 |
| height | none, 5, 7, 9 | 7 |
| padding | 0, 1, 2 | 1 |
| margin | 0, 1, 2 | 1 |
| foreground | false, true | false |

## Divider

A line across, and a VerticalDivider down; merged, they join the lines drawn before them where they meet.

- `rtwk::layout::Divider` (struct) in `crates/rtwk-widgets/src/layout/box_.rs:633`: A line across the room it gets, one row tall, drawn with `kind` in `style`; with `merge` it joins the lines drawn before it, its ends meeting a line they land on in a joint (twk's `Divider`, over a `RenderDivider`).

### Code

```rust
let theme = Theme::of(cx);
let kind = BorderType::Single;
let horizontal = Divider::new().kind(kind).style(theme.border.clone()).merge(true);
let vertical = VerticalDivider::new().kind(kind).style(theme.border.clone()).merge(true);
let text = |x, y, label| Positioned::new(Text::new(label)).left(x).top(y);
let muted = |x, label| Positioned::new(Text::new(label).style(theme.muted.clone()))
    .left(x).top(3);
let table = Stack::new().fit(StackFit::Expand)
    .child(BoxWidget::new().border(Border::all(theme.border.clone()).kind(kind)))
    .child(Positioned::new(Text::new("release 0.4.0").style(Style::new().attrs(Attrs::BOLD)))
        .left(2).top(1))
    .child(Positioned::new(horizontal).left(0).right(0).top(2))
    .child(Positioned::new(vertical).left(17).top(2).bottom(0))
    .child(muted(2, "added"))
    .child(text(2, 4, "Grid tracks\nDockArea"))
    .child(muted(19, "fixed"))
    .child(text(19, 4, "wide glyphs\nwheel scroll"));
SizedBox::new(Some(36), Some(7), Some(table.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| type | single, rounded, double, heavy, ascii | single |
| merge | false, true | true |

## Shadow

A drop shadow: the cells beside and below a child, moved by its offset, what is under them darkened.

- `rtwk::layout::Shadow` (struct) in `crates/rtwk-widgets/src/layout/shadow.rs:117`: Casts shadow from its child on the cells around it (twk's `Shadow`): the box is as big as the child and the room the shadow takes beside it.

### Code

```rust
let theme = Theme::of(cx);
let shadow = BoxShadow { dx: 2, dy: 1, alpha: 0x80, ..BoxShadow::default() };
let panel = BoxWidget::new()
    .style(theme.surface.merge(&theme.text))
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .padding(EdgeInsets::x(1))
    .child(Text::new("a box with a shadow"));
let stack = Stack::new().fit(StackFit::Expand)
    .child(Text::new("what is under a shadow keeps its glyphs; their colours darken. ".repeat(5)).style(theme.muted.clone()))
    .child(Positioned::new(Shadow::new(panel).shadow(shadow)).left(5).top(2));
SizedBox::new(Some(36), Some(8), Some(stack.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| offset | 2, 1, 1, 1, -2, -1 | 2, 1 |
| alpha | ¼, ½, ¾ | ½ |

## Padding

Empty cells around a child.

- `rtwk::layout::Padding` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:42`: Insets its child by padding: as big as the child and the padding.

### Code

```rust
let theme = Theme::of(cx);
BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Padding::new(
        EdgeInsets::x(4),
        BoxWidget::new().style(theme.selection.clone()).child(Text::new("the child,\npadded from the box")),
    ))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| padding | all 1, x 4, y 2, left 8, left 3, top 1 | x 4 |

## Align, Center

A child placed in the room it is given; Center in the middle of it.

- `rtwk::layout::Align` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:103`: Places its child inside itself as alignment says (twk's `Align`, over a `RenderPositionedBox`).
- `rtwk::layout::Center` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:191`: An Align that puts its child in the middle, the odd cell after it (twk's `Center`).

### Code

```rust
let theme = Theme::of(cx);
let room = |title: &str, child: AnyWidget| SizedBox::new(
    Some(30), Some(5),
    Some(BoxWidget::new()
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .title(BorderTitle::new(format!(" {title} ")).style(theme.muted.clone()))
        .child(child)
        .into()),
);
let aligned = Align::new(Alignment::new(1, 1), Text::new("aligned"));
let centred = Center::new(Text::new("centred"));
column![
    room("Alignment::BOTTOM_RIGHT", aligned.into()),
    room("Center", centred.into()),
]
.main_axis_size(MainAxisSize::Min)
.spacing(1)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment x | left, center, right | right |
| alignment y | top, center, bottom | bottom |
| width factor | none, 1, 2, 3 | none |
| height factor | none, 1, 2, 3 | none |

## SizedBox

A child of a size: SizedBox::expand fills the room, SizedBox::shrink takes none.

- `rtwk::layout::SizedBox` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:273`: Makes its child width wide and height tall, as far as the constraints allow; along an axis left none, the child is as long as it wants (twk's `SizedBox`, over a `RenderConstrainedBox` with tight constraints).

### Code

```rust
let theme = Theme::of(cx);
let child = BoxWidget::new().style(theme.selection.clone()).child(Text::new("16 × 3"));
let sized = SizedBox::new(Some(16), Some(3), Some(child.into()));
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Center::new(sized));
SizedBox::new(Some(40), Some(9), Some(panel.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| constructor | width, height, .expand, .shrink | width, height |
| width | 0 cells, 1 cells, 2 cells, 3 cells, 4 cells, 5 cells, 6 cells, 7 cells, 8 cells, 9 cells, 10 cells, 11 cells, 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells | 16 cells |
| height | 0 rows, 1 rows, 2 rows, 3 rows, 4 rows, 5 rows, 6 rows, 7 rows, 8 rows, 9 rows | 3 rows |

## ConstrainedBox

Constraints a child is laid out within, merged with those it is given.

- `rtwk::layout::ConstrainedBox` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:379`: Lays its child out within constraints as well as its own: each limit is kept within those the box gets (twk's `ConstrainedBox`, over a `RenderConstrainedBox`).

### Code

```rust
let theme = Theme::of(cx);
let bounded = |text: &'static str| ConstrainedBox::new(
    BoxConstraints::new(12, 32, 1, 3),
    BoxWidget::new()
        .style(theme.selection.clone())
        .padding(EdgeInsets::x(1))
        .child(Text::new(text)),
);
column![bounded("short"), bounded("A child as wide as its text, within the bounds.")]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| min width | 0, 12, 20 | 12 |
| max width | 20, 32, 48 | 32 |

## LimitedBox, OverflowBox

Other constraints than the room gives: LimitedBox limits a child only where the room has no limit, OverflowBox and SizedOverflowBox give it others in its place, cut off where it sticks out.

- `rtwk::layout::LimitedBox` (struct) in `crates/rtwk-widgets/src/layout/overflow.rs:41`: Limits its child's size only where the room is unbounded (a scroll view's main axis, a row's width in an unbounded parent); where the room is bounded the child gets it as it is (twk's `LimitedBox`).
- `rtwk::layout::OverflowBox` (struct) in `crates/rtwk-widgets/src/layout/overflow.rs:243`: Lays its child out within other limits than the ones it gets, and places it by `alignment` in the room it has (twk's `OverflowBox`): a child bigger than the box overflows it, and is cut off at the box's edges by the parent's clip when painted.

### Code

```rust
let theme = Theme::of(cx);
let inner = OverflowBox::new(Text::new("a line wider than the box, cut off at its edges").style(theme.selection.clone()))
    .max_width(48)
    .alignment(Alignment::CENTER);
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .title(BorderTitle::new(" 28 × 3 ").style(theme.muted.clone()))
    .child(inner);
SizedBox::new(Some(30), Some(5), Some(panel.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| widget | OverflowBox, SizedOverflowBox, LimitedBox | OverflowBox |
| alignment | CENTER_LEFT, CENTER, CENTER_RIGHT | CENTER |

## FractionallySizedBox

A child a fraction of the room it is given.

- `rtwk::layout::FractionallySizedBox` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:439`: Makes its child a fraction of the room there is: `width_factor` of the most width the box may have, `height_factor` of the most height (`0.5` for half, `1.0 / 3.0` for a third), each rounded to the nearest cell, a half up (twk's `FractionallySizedBox`, over a `RenderFractionallySizedBox`).

### Code

```rust
let theme = Theme::of(cx);
let colored = BoxWidget::new()
    .style(theme.selection.clone())
    .child(Center::new(Text::new("0.5 × 0.5")));
let fraction = FractionallySizedBox::new(colored)
    .width_factor(0.5)
    .height_factor(0.5)
    .alignment(Alignment::CENTER);
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(fraction);
SizedBox::new(Some(40), Some(9), Some(panel.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| width factor | ¼, ½, ¾, 1 | ½ |
| height factor | ¼, ½, ¾, 1 | ½ |
| alignment | CENTER, TOP_LEFT, BOTTOM_RIGHT | CENTER |
| room | 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells | 40 cells |

## IntrinsicWidth/Height

A child as wide or as tall as it would like to be: stretched items all as wide as the widest, boxes all as tall as the tallest.

- `rtwk::layout::IntrinsicWidth` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:539`: Makes its child as wide as its content would like to be, within the room there is, rounded up to a multiple of `step_width` when given; with `step_height`, as tall as its content would like to be at that width, rounded up to a multiple of it, too (twk's `IntrinsicWidth`, over a `RenderIntrinsicWidth`).
- `rtwk::layout::IntrinsicHeight` (struct) in `crates/rtwk-widgets/src/layout/basic.rs:619`: Makes its child as tall as its content would like to be, within the room there is: a `Row` under it with `CrossAxisAlignment::Stretch` makes all its children as tall as the tallest (twk's `IntrinsicHeight`, over a `RenderIntrinsicHeight`).

### Code

```rust
let theme = Theme::of(cx);
let menu_item = |label, style| BoxWidget::new()
    .style(style)
    .padding(EdgeInsets::x(1))
    .child(Text::new(label));
let menu = column![
    menu_item("Open…", theme.selection.clone()),
    menu_item("Save as…", theme.surface.clone()),
    menu_item("Close window", theme.surface.clone()),
].cross_axis_alignment(CrossAxisAlignment::Stretch);
let card = |text| SizedBox::new(Some(10), None, Some(BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Text::new(text))
    .into()));
let cards = row![card("one"), card("three lines here")]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .spacing(1);
let content = row![IntrinsicWidth::new(menu), IntrinsicHeight::new(cards)]
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(4);
SizedBox::new(Some(44), Some(7), Some(content.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| IntrinsicWidth | false, true | true |
| IntrinsicHeight | false, true | true |

## Row, Column, Flex

Children one after another along an axis, aligned along and across it.

- `rtwk::layout::Row` (struct) in `crates/rtwk-widgets/src/layout/flex.rs:197`: Its children side by side (twk's `Row`), over a `RenderFlex`.
- `rtwk::layout::Column` (struct) in `crates/rtwk-widgets/src/layout/flex.rs:192`: Its children one under another (twk's `Column`), over a `RenderFlex`.
- `rtwk::layout::Flex` (struct) in `crates/rtwk-widgets/src/layout/flex.rs:186`: Its children along `direction` (twk's `Flex`), over a `RenderFlex`: Row and Column are its two directions.

### Code

```rust
let theme = Theme::of(cx);
let chip = |label, color: &Style, padding| -> AnyWidget {
    let reversed = color.merge(&Style::new().attrs(Attrs::REVERSE));
    BoxWidget::new()
        .style(reversed.clone())
        .padding(padding)
        .child(Text::new(label).style(reversed))
        .into()
};
let one = chip("one", &theme.primary, EdgeInsets::x(1));
let two = chip("two", &theme.accent, EdgeInsets::all(1));
let three = chip("three", &theme.success, EdgeInsets::x(1));
let flex: AnyWidget = row![one, two, three]
    .main_axis_size(MainAxisSize::Max)
    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .spacing(1)
    .into();

SizedBox::new(
    Some(44),
    Some(11),
    Some(
        BoxWidget::new()
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .child(flex)
            .into(),
    ),
)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| direction | row, column | row |
| main axis | Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly | SpaceBetween |
| cross axis | start, end, center, stretch | center |
| spacing | 0, 1, 2 | 1 |
| main axis size | max, min | max |

## Expanded, Spacer

Children of a Row or Column that share the room the others leave by their flex factors: Expanded fills its share, a Flexible may take less, a Spacer is empty.

- `rtwk::layout::Expanded` (struct) in `crates/rtwk-widgets/src/layout/flex.rs:270`: Flexible with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::layout::Spacer` (struct) in `crates/rtwk-widgets/src/layout/flex.rs:348`: An empty Expanded: a gap that takes `flex` parts of the free room, to push its siblings apart (twk's `Spacer`).

### Code

```rust
let theme = Theme::of(cx);
let rounded = || Border::all(theme.border.clone()).kind(BorderType::Rounded);
let expanded = Expanded::new(1, BoxWidget::new()
    .border(Border::all(theme.accent.clone()).kind(BorderType::Rounded))
    .child(Text::new("Expanded 1").wrap(TextWrap::None)));
let flexible = Flexible {
    fit: FlexFit::Loose,
    ..Flexible::new(1, BoxWidget::new().border(rounded()).child(Text::new("Flexible")))
};
let content = row![
    expanded, flexible, Spacer::new(1),
    BoxWidget::new().border(rounded()).child(Text::new("end")),
].cross_axis_alignment(CrossAxisAlignment::Stretch);
SizedBox::new(Some(48), Some(3), Some(content.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| Expanded flex | 1, 2, 3 | 1 |
| Flexible fit | loose, tight | loose |
| width | 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 48 cells |

## Stack, Positioned

Children over one another, pinned to the edges or aligned.

- `rtwk::layout::Stack` (struct) in `crates/rtwk-widgets/src/layout/stack.rs:36`: Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
- `rtwk::layout::Positioned` (struct) in `crates/rtwk-widgets/src/layout/stack.rs:107`: Pins its child, in a Stack, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.

### Code

```rust
let theme = Theme::of(cx);
let stack = Stack::new()
    .alignment(Alignment::new(0, 0))
    .fit(StackFit::Loose)
    .child(SizedBox::expand())
    .child(BoxWidget::new().style(theme.selection.clone())
        .padding(EdgeInsets::x(1)).child(Text::new("not positioned")))
    .child(Positioned::new(Badge::new("top right")).right(0).top(0))
    .child(Positioned::new(Text::new("bottom left").style(theme.muted.clone()))
        .left(0).bottom(0));
ConstrainedBox::new(
    BoxConstraints::new(38, 38, 0, 9),
    BoxWidget::new().border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .child(stack),
)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment x | left, center, right | center |
| alignment y | top, center, bottom | center |
| fit | loose, expand, passthrough | loose |

## Transform

A child shown moved by whole cells, laid out where it was: Transform::translate, the one transform a terminal has.

- `rtwk::layout::Transform` (struct) in `crates/rtwk-widgets/src/layout/overflow.rs:638`: Moves where its child is drawn, by `offset` cells, without changing its layout (twk's `Transform.translate`): the box is as big as the child and the child is drawn `offset` away from it, cut off at the edges of what the parent draws.

### Code

```rust
const OFFSETS: [(i32, i32); 12] = [
    (0, -2), (5, -2), (9, -1), (10, 0),
    (9, 1), (5, 2), (0, 2), (-5, 2),
    (-9, 1), (-10, 0), (-9, -1), (-5, -2),
];

fn moved(step: u64, cx: &mut Cx<'_>) -> impl Into<AnyWidget> {
    let theme = Theme::of(cx);
    // step is the elapsed milliseconds / 200.
    let (dx, dy) = OFFSETS[(step % 12) as usize];
    let content = Center::new(BoxWidget::new().style(theme.selection.clone())
        .padding(EdgeInsets::x(1)).child(Text::new("moved")));
    let panel = BoxWidget::new()
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .child(Transform::translate(dx, dy, content));
    SizedBox::new(Some(32), Some(7), Some(panel.into()))
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| offset | round, fixed | round |
| dx | -12 cells, -11 cells, -10 cells, -9 cells, -8 cells, -7 cells, -6 cells, -5 cells, -4 cells, -3 cells, -2 cells, -1 cells, 0 cells, 1 cells, 2 cells, 3 cells, 4 cells, 5 cells, 6 cells, 7 cells, 8 cells, 9 cells, 10 cells, 11 cells, 12 cells | 6 cells |
| dy | -2 rows, -1 rows, 0 rows, 1 rows, 2 rows | 1 rows |

## Visibility, Offstage

A child hidden: Offstage keeps it laid out and running without painting it; Visibility takes its room away too, or keeps it.

- `rtwk::layout::Visibility` (struct) in `crates/rtwk-widgets/src/layout/visibility.rs:262`: Shows its child, or hides it (twk's `Visibility`): when `visible` is off it shows replacement (an empty box by default) in its place, or, with `maintain_state`, keeps the child built offstage, or, with `maintain_size`, keeps it laid out and takes its room, drawing nothing.
- `rtwk::layout::Offstage` (struct) in `crates/rtwk-widgets/src/layout/visibility.rs:222`: Keeps its child built — its state alive, its tree laid out — but neither shown nor given room, while offstage (twk's `Offstage`): the box is as small as its constraints allow, and the child is not painted nor hit by the mouse.

### Code

```rust
let theme = Theme::of(cx);
let child = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .padding(EdgeInsets::x(1)).child(Text::new("the child"));
column![
    Text::new("above the child"),
    Visibility::new(false, child)
        .maintain_state(false)
        .maintain_size(false),
    Text::new("below the child"),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| widget | Visibility, Offstage | Visibility |
| hidden | false, true | true |
| keeps | nothing, state, size | nothing |
| replacement | false, true | false |

## ExpansionTile, Accordion

Sections that fold under their headers: Enter, Space or a click opens one, Right and Left open and close it; in an accordion one is open at a time, Up and Down go from header to header.

- `rtwk::controls::ExpansionTile` (struct) in `crates/rtwk-controls/src/expansion.rs:47`: A focusable header that expands or collapses its content.
- `rtwk::controls::Accordion` (struct) in `crates/rtwk-controls/src/expansion.rs:592`: A vertical list of expansion tiles.

### Code

```rust
struct Sections;

struct SectionsState {
    verify: bool,
    last: Option<String>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Verify(bool),
    Changed(&'static str, bool),
}

impl StatefulWidget for Sections {
    type State = SectionsState;
    fn create_state(&self) -> SectionsState {
        SectionsState { verify: true, last: None }
    }
}

impl State for SectionsState {
    type Widget = Sections;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Sections, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Verify(value) => self.verify = value,
            Msg::Changed(title, expanded) => {
                let verb = if expanded { "opened" } else { "closed" };
                self.last = Some(format!("{verb} {title}"));
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Sections, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let section = |title: &'static str, subtitle: &'static str, child: AnyWidget, open| {
            let tile = ExpansionTile::new(Text::new(title), child)
                .initially_expanded(open)
                .duration(Duration::from_millis(150))
                .on_expansion_changed(cx.map(move |value| Msg::Changed(title, value)));
            let tile = tile.subtitle(Text::new(subtitle));
            tile
        };
        let tiles = [
            section("General", "Name and theme", Text::new("Name: rtwk\nTheme: dark").into(), true),
            section("Network", "Proxy and timeouts", Text::new("Proxy: none\nTimeout: 30 s").into(), false),
            section("Security", "TLS and certificates", column![
                Text::new("TLS: 1.3"),
                Checkbox::new(self.verify).label("verify certificates")
                    .on_changed(cx.map(Msg::Verify)),
            ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).into(), false),
        ];
        let sections: AnyWidget = Accordion::new(tiles).allow_multiple(false).into();
        let last = self.last.as_deref().unwrap_or("nothing");
        column![
            SizedBox::new(Some(40), None, Some(sections)),
            Text::rich([Span::styled("did: ", theme.muted.clone()),
                if self.last.is_some() { Span::new(last.to_owned()) }
                else { Span::styled(last.to_owned(), theme.muted.clone()) }]),
        ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
        .into()
    }
}
impl_widget!(stateful Sections);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| in an accordion | false, true | true |
| several open | false, true | false |
| subtitles | false, true | true |
| duration | 150 ms, 400 ms, none | 150 ms |

## Grid, GridPlacement

Children in the cells of tracks: fixed, a share of the rest, or as long as their content; a child may span several.

- `rtwk::layout::Grid` (struct) in `crates/rtwk-widgets/src/layout/grid.rs:34`: Its children in a grid of tracks (twk's `Grid`, over a `RenderGrid`): a child with a GridPlacement goes in its cell, the others after the one before them.
- `rtwk::layout::GridPlacement` (struct) in `crates/rtwk-widgets/src/layout/grid.rs:144`: Puts its child, in a Grid, in a cell and over some tracks (twk's `GridPlacement`): a row and a column, or neither (after the child before it), and spans of 1 track or more.

### Code

```rust
let theme = Theme::of(cx);
let cell = |label| BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Text::new(label));
let grid = Grid::new()
    .columns([GridTrack::fixed(12), GridTrack::fr(1), GridTrack::content()])
    .rows([GridTrack::content(), GridTrack::fr(1), GridTrack::content()])
    .column_gap(1).row_gap(0)
    .child(GridPlacement::new(cell("header · span(1, 3)")).span(1, 3))
    .child(cell("fixed(12)"))
    .child(cell("fr(1)"))
    .child(cell("content()"))
    .child(GridPlacement::new(cell("span(1, 2)")).at(2, 1).span(1, 2));
SizedBox::new(Some(44), Some(12), Some(grid.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| third column | .content(), .fixed(14), .fr(1) | .content() |
| column gap | 0, 1, 2 | 1 |
| row gap | 0, 1 | 0 |
| width | 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 44 cells |

## Wrap

Children in runs that go on in the next run where they would not fit: tags, buttons and key hints on a terminal that narrows.

- `rtwk::layout::Wrap` (struct) in `crates/rtwk-widgets/src/layout/wrap.rs:25`: Its children one after another along `direction`, a new run where the next does not fit (twk's `Wrap`, over a `RenderWrap`): tags that flow onto the next line.

### Code

```rust
let theme = Theme::of(cx);
let width = 36;
let rounded = || Border::all(theme.border.clone()).kind(BorderType::Rounded);
let wrap = Wrap::new()
    .direction(Axis::Horizontal)
    .alignment(WrapAlignment::Start)
    .run_alignment(WrapAlignment::Start)
    .cross_axis_alignment(WrapCrossAlignment::Center)
    .spacing(1).run_spacing(0)
    .child(Badge::new("rust"))
    .child(Badge::new("terminal").kind(StatusKind::Info))
    .child(BoxWidget::new().border(rounded()).child(Text::new("a box")))
    .child(Badge::new("rtwk").kind(StatusKind::Success))
    .child(Badge::new("日本語").kind(StatusKind::Warning))
    .child(Badge::new("layout"))
    .child(Badge::new("runs").kind(StatusKind::Info));
let panel = BoxWidget::new().border(rounded()).child(wrap);
SizedBox::new(Some(width), Some(11), Some(panel.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells | 36 cells |
| narrowing | false, true | false |
| direction | horizontal, vertical | horizontal |
| alignment | Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly | Start |
| run alignment | Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly | Start |
| cross axis | start, end, center | center |
| spacing | 0, 1, 2 | 1 |
| run spacing | 0, 1 | 0 |

## CustomPaint

Cells a CellPainter draws itself, under the child or over it, painted again when what it paints changes.

- `rtwk::layout::CustomPaint` (struct) in `crates/rtwk-widgets/src/layout/custom_paint.rs:115`: Paints cells with a CellPainter under its child and another over it (twk's `CustomPaint`).

### Code

```rust
// The wave's height is measured in eighths of a terminal cell.
#[derive(Debug, PartialEq, Eq)]
struct Wave { phase: i32, style: Style }
impl CellPainter for Wave {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        const BLOCKS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        let pen = self.style.pen_in(canvas.styles_mut());
        for x in 0..width {
            let u = (x + self.phase) % 24;
            let eighths = 4 + 3 * u.min(24 - u);
            for row in 0..height {
                let left = eighths - 8 * row;
                if left > 0 {
                    canvas.text(x, height - 1 - row, BLOCKS[left.min(8) as usize], pen);
                }
            }
        }
    }
    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>() != Some(self)
    }
}

fn waves(step: u64, cx: &mut Cx<'_>) -> impl Into<AnyWidget> {
    let theme = Theme::of(cx);
    // step is the elapsed milliseconds / 100.
    let phase = (step % 24) as i32;
    let wave = Wave { phase, style: theme.primary.clone() };
    let label = Text::new("cells of its own")
        .style(theme.text.merge(&Style::new().attrs(Attrs::BOLD)));
    CustomPaint::new()
        .painter(wave)
        .child(SizedBox::new(Some(36), Some(6), Some(Center::new(label).into())))
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| painter | painter, foreground_painter | painter |
| moving | false, true | true |
| style | primary, accent, success | primary |

## SplitView

Panes the mouse or the arrows resize by their dividers; Tab focuses one. The last one collapses.

- `rtwk::layout::SplitView` (struct) in `crates/rtwk-widgets/src/layout/split_view.rs:48`: Panes separated by dividers that respond to the mouse and keys.

### Code

```rust
use rtwk::layout::Axis;

struct Panes;

struct PanesState {
    split: SplitController,
}

impl StatefulWidget for Panes {
    type State = PanesState;
    fn create_state(&self) -> PanesState {
        PanesState { split: SplitController::new().with_fractions([1.0, 2.0, 1.0]) }
    }
}

impl State for PanesState {
    type Widget = Panes;
    type Msg = ();

    fn build(&self, _: &Panes, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let pane = |title, text, collapsible| SplitPane::new(
            BoxWidget::new()
                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded).merge(true))
                .title(BorderTitle::new(title).style(theme.muted.clone()))
                .padding(EdgeInsets::only(1, 0, 0, 0)).child(Text::new(text)),
        ).min_size(8).collapsible(collapsible);
        let split = SplitView::new([
            pane("files", "box.rs\nflex.rs\ngrid.rs\nwrap.rs", false),
            pane("editor", "Drag a divider, or Tab to one and press the arrows.", false),
            pane("outline", "Drag its divider to the edge.", true),
        ]).controller(&self.split)
          .direction(Axis::Horizontal).share_edges(true).kind(BorderType::Single)
          .style(theme.border.clone());

        let controller = self.split.clone();
        let fractions = ListenableBuilder::new(self.split.listenable(), move |_, _| {
            let fractions = controller.fractions().iter().map(|f| format!("{f:.2}"))
                .collect::<Vec<_>>().join("  ");
            let mut spans = vec![Span::styled("fractions: ", theme.muted.clone()), Span::new(fractions)];
            let collapsed = controller.collapsed();
            if !collapsed.is_empty() {
                spans.push(Span::styled("  collapsed: ", theme.muted.clone()));
                spans.push(Span::new(format!("{collapsed:?}")));
            }
            Text::rich(spans).into()
        });
        column![Flexible::new(1, SizedBox::new(Some(48), Some(11), Some(split.into()))), fractions]
            .main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
        .into()
    }
}
impl_widget!(stateful Panes);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| direction | horizontal, vertical | horizontal |
| share edges | false, true | true |
| dividers | single, rounded, double, heavy, ascii | single |

## DockArea

Panels in splits and tabs: a tab dragged onto a target of a group docks its panel there, and dropped elsewhere floats it; a window's title bar dragged onto one docks it again.

- `rtwk::controls::DockArea` (struct) in `crates/rtwk-controls/src/dock_area.rs:48`: A tabbed area whose panels can be resized, docked and floated.

### Code

```rust
fn docked() -> DockLayout {
    let root = DockSplit::with_fractions(Axis::Horizontal, [
        DockTabs::new(["files", "find"]).into(), DockTabs::new(["editor"]).into(),
    ], [1.0, 2.0]);
    DockLayout::new(Some(root.into()), [DockWindow::new("log").at(23, 4).sized(26, 6)])
}

struct Docks;

struct DocksState {
    controller: DockController,
    count: u32,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Reset,
    Add,
}

impl StatefulWidget for Docks {
    type State = DocksState;
    fn create_state(&self) -> DocksState {
        DocksState { controller: DockController::new(docked()), count: 0 }
    }
}

impl State for DocksState {
    type Widget = Docks;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Docks, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Reset => self.controller.set_layout(docked()),
            Msg::Add => self.count += 1,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Docks, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let panel = |id, title, child: AnyWidget| DockPanel::new(id, title, Padding::new(EdgeInsets::x(1), child));
        let clicks = column![
            Text::rich([Span::styled("count: ", theme.muted.clone()), Span::new(self.count.to_string())]),
            Button::new("Add one").on_pressed(cx.send(Msg::Add)),
        ].cross_axis_alignment(CrossAxisAlignment::Start);
        let area = DockArea::new(&self.controller, [
            panel("files", "Files", Text::new("Drag a tab onto a target to dock it there.").into()),
            panel("find", "Find", Text::new("Find in files.").into()),
            panel("editor", "Editor", clicks.into()),
            panel("log", "Log", Text::new("Drop a tab off the targets to float it.").into()),
        ]).tab_strip(true);
        column![
            SizedBox::new(Some(50), Some(10), Some(area.into())),
            Button::new("Dock them as they were").on_pressed(cx.send(Msg::Reset)),
        ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
        .into()
    }
}
impl_widget!(stateful Docks);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| tab strip | false, true | true |
| window frame | by the theme, raised, bordered | by the theme |

## Scaffold

An app's frame: a header, a sidebar beside the body, a status bar and the key hints, each part a zone of the focus; the sidebar collapsed on a narrow screen and resized by a drag.

- `rtwk::navigation::Scaffold` (struct) in `crates/rtwk/src/navigation/scaffold.rs:684`: The frame most full-screen apps are built in: `header` rows on top, `sidebar` beside `body` under them, the `status_bar` row and the key hints (a KeyHintsBar, with `key_hints`) at the bottom.

### Code

```rust
struct Notes;

struct NotesState {
    inbox: bool,
    archive: bool,
    notes: u32,
    controller: ScaffoldController,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Inbox(bool),
    Archive(bool),
    New,
}

impl StatefulWidget for Notes {
    type State = NotesState;
    fn create_state(&self) -> NotesState {
        NotesState {
            inbox: true,
            archive: false,
            notes: 3,
            controller: ScaffoldController::new().initial_sidebar_width(14),
        }
    }
}

impl State for NotesState {
    type Widget = Notes;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Notes, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Inbox(value) => self.inbox = value,
            Msg::Archive(value) => self.archive = value,
            Msg::New => self.notes += 1,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Notes, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let sidebar = column![
            Checkbox::new(self.inbox).label("inbox").on_changed(cx.map(Msg::Inbox)),
            Checkbox::new(self.archive).label("archive").on_changed(cx.map(Msg::Archive)),
        ].cross_axis_alignment(CrossAxisAlignment::Start);
        let body = column![
            Button::new("New note").on_pressed(cx.send(Msg::New)),
            Text::rich([Span::styled("notes: ", theme.muted.clone()), Span::new(self.notes.to_string())]),
        ].cross_axis_alignment(CrossAxisAlignment::Start).spacing(1);
        let scaffold = Scaffold::new(Padding::new(EdgeInsets::all(1), body))
            .controller(&self.controller)
            .sidebar(Padding::new(EdgeInsets::all(1), sidebar))
            .sidebar_side(SidebarSide::Left)
            .resizable_sidebar(true)
            .key_hints(true)
            .header(Text::new("notes"))
            .status_bar(StatusBar::new()
                .left(Text::new(format!("{} notes", self.notes))).right(Text::new("saved")))
            .collapsed_below(40);
        SizedBox::new(Some(48), Some(12), Some(scaffold.into()))
        .into()
    }
}
impl_widget!(stateful Notes);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells | 48 cells |
| sidebar side | left, right | left |
| resizable | false, true | true |
| key hints | false, true | true |
| header | false, true | true |
| status bar | false, true | true |
| collapsed below | none, 36, 40, 44 | 40 |
