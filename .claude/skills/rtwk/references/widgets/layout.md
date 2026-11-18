<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Layout

[All widgets](../widgets.md)

## Box

A border, titles on it, padding and a background around a child.

API:

- `rtwk::layout::BoxWidget` (struct, `crates/rtwk-widgets/src/layout/box_.rs:226`) A box around `child`: `style` fills it, `border` lines its sides, `titles` sit on its top and bottom edges, and `padding` is between the border and the child (twk's `Box`, over a `RenderDecoratedBox`).
  - public fields: `child: Option<AnyWidget>`, `style: Option<Style>`, `border: Option<Border>`, `titles: Vec<BorderTitle>`, `padding: EdgeInsets`

### Code

The gallery's Code tab at the initial options (route `#/layout/box`, `crates/rtwk-gallery/src/pages/mod.rs:1455`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .padding(EdgeInsets::all(1))
    .child(Text::new("Compiling 14 libraries…\nlib/src/layout    0.4 s\nlib/src/render    0.9 s"))
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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| border | single, rounded, double, heavy, ascii, none | rounded |
| sides | all, top bottom, top left, left | all |
| titles | on, off | true |
| at | topLeft, topCenter, topRight, bottomLeft, bottomCenter | topLeft |
| padding | 0, 1, 2 | 1 |
| surface | on, off | false |

### Methods

#### `BoxWidget`: constructors and associated functions

- `pub fn new() -> BoxWidget` - An empty box that fills nothing.

#### `BoxWidget`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> BoxWidget` - With `child` inside.
- `pub fn style(mut self, style: Style) -> BoxWidget` - Filled with `style`.
- `pub fn border(mut self, border: Border) -> BoxWidget` - Lined by `border`.
- `pub fn title(mut self, title: BorderTitle) -> BoxWidget` - With `title` on an edge, after those so far.
- `pub fn titles(mut self, titles: impl IntoIterator<Item = BorderTitle>) -> BoxWidget` - With `titles` on its edges.
- `pub fn padding(mut self, padding: EdgeInsets) -> BoxWidget` - `padding` cells between the border and the child.
- `pub fn scrollbar(self, bar: impl BoxScrollbar) -> ScrolledBox` - With the bar of the outermost scrollable in the child drawn on the border, as `bar` has it (twk's `scrollbar`): on the border's right side for a vertical scrollable and its bottom for a horizontal one, a side the ...

### Also used in the code

- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`

## ColoredBox, DecoratedBox

A child's cells filled with a color, and a BoxDecoration (a fill, a border, titles) drawn under the child or over its edges.

API:

- `rtwk::layout::ColoredBox` (struct, `crates/rtwk-widgets/src/layout/container.rs:108`) Fills its cells with `color`, then paints its child over it: the child's text takes the color as its background where it sets none.
  - public fields: `color: Color`, `child: Option<AnyWidget>`
- `rtwk::layout::DecoratedBox` (struct, `crates/rtwk-widgets/src/layout/container.rs:267`) Paints `decoration` under `child` or over it, as `position` says (twk's `DecoratedBox`).
  - public fields: `decoration: BoxDecoration`, `position: DecorationPosition`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/coloredbox-decoratedbox`, `crates/rtwk-gallery/src/pages/mod.rs:1511`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| position | background, foreground | background |
| fill | color, gradient | color |
| border | single, rounded, double, heavy, ascii | rounded |
| titles | on, off | true |

### Methods

#### `ColoredBox`: constructors and associated functions

- `pub fn new(color: Color, child: impl Into<AnyWidget>) -> ColoredBox` - Fills its cells with `color` under `child`.
- `pub fn empty(color: Color) -> ColoredBox` - As `new`, without a child.

#### `DecoratedBox`: constructors and associated functions

- `pub fn new(decoration: BoxDecoration, child: impl Into<AnyWidget>) -> DecoratedBox` - Paints `decoration` under `child`.
- `pub fn empty(decoration: BoxDecoration) -> DecoratedBox` - As `new`, the decoration alone.

#### `DecoratedBox`: builder methods (take and return `self`)

- `pub fn position(mut self, position: DecorationPosition) -> DecoratedBox` - Over the child, or under it.

### Also used in the code

- `rtwk::layout::BoxDecoration` (struct, `crates/rtwk-widgets/src/layout/container.rs:178`) What a `DecoratedBox` paints: a `style` that fills it, or a `gradient` under it, a `border` around it and `titles` on its edges, as a `BoxWidget` draws them (twk's `BoxDecoration`).
  - public fields: `style: Option<Style>`, `border: Option<Border>`, `titles: Vec<BorderTitle>`, `gradient: Option<Gradient>`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`
- `rtwk::layout::DecorationPosition` (enum, `crates/rtwk-widgets/src/layout/container.rs:150`) Where a `DecoratedBox` paints its decoration: under its child or over it.
  - variants: `Background`, `Foreground`

## Container

Align, Padding, a color or a decoration, a size and a margin put together around one child, as Flutter's Container puts them.

API:

- `rtwk::layout::Container` (struct, `crates/rtwk-widgets/src/layout/container.rs:408`) The layout widgets with one child, put together as Flutter's `Container` puts them, from the inside out (twk's `Container`): `child`, or without one a box as big as the constraints allow (as small along an unbounded ...
  - public fields: `alignment: Option<Alignment>`, `padding: Option<EdgeInsets>`, `color: Option<Color>`, `decoration: Option<BoxDecoration>`, `foreground_decoration: Option<BoxDecoration>`, `width: Option<i32>`, `height: Option<i32>`, `constraints: Option<BoxConstraints>`, `margin: Option<EdgeInsets>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/container`, `crates/rtwk-gallery/src/pages/mod.rs:1547`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| fill | decoration, color | decoration |
| alignment | none, topLeft, center, bottomRight | center |
| width | null, 24, 30, 36 | 30 |
| height | null, 5, 7, 9 | 7 |
| padding | 0, 1, 2 | 1 |
| margin | 0, 1, 2 | 1 |
| foreground | on, off | false |

### Methods

#### `Container`: constructors and associated functions

- `pub fn new() -> Container` - An empty container: as big as the constraints allow.

#### `Container`: builder methods (take and return `self`)

- `pub fn alignment(mut self, a: Alignment) -> Container` - Where the child goes.
- `pub fn padding(mut self, p: EdgeInsets) -> Container` - The empty cells around the child.
- `pub fn color(mut self, c: Color) -> Container` - The background of the box.
- `pub fn decoration(mut self, d: BoxDecoration) -> Container` - What is painted under the child.
- `pub fn foreground_decoration(mut self, d: BoxDecoration) -> Container` - What is painted over the child.
- `pub fn width(mut self, w: i32) -> Container` - The columns the box takes.
- `pub fn height(mut self, h: i32) -> Container` - The rows the box takes.
- `pub fn constraints(mut self, c: BoxConstraints) -> Container` - The limits on the size of the box.
- `pub fn margin(mut self, m: EdgeInsets) -> Container` - The empty cells around the box.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Container` - What the container holds.

### Also used in the code

- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::layout::BoxDecoration` (struct, `crates/rtwk-widgets/src/layout/container.rs:178`) What a `DecoratedBox` paints: a `style` that fills it, or a `gradient` under it, a `border` around it and `titles` on its edges, as a `BoxWidget` draws them (twk's `BoxDecoration`).
  - public fields: `style: Option<Style>`, `border: Option<Border>`, `titles: Vec<BorderTitle>`, `gradient: Option<Gradient>`

## Divider

A line across, and a VerticalDivider down; merged, they join the lines drawn before them where they meet.

API:

- `rtwk::layout::Divider` (struct, `crates/rtwk-widgets/src/layout/box_.rs:633`) A line across the room it gets, one row tall, drawn with `kind` in `style`; with `merge` it joins the lines drawn before it, its ends meeting a line they land on in a joint (twk's `Divider`, over a `RenderDivider`).
  - public fields: `kind: Option<BorderType>`, `style: Style`, `merge: bool`

### Code

The gallery's Code tab at the initial options (route `#/layout/divider`, `crates/rtwk-gallery/src/pages/mod.rs:2211`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| type | single, rounded, double, heavy, ascii | single |
| merge | on, off | true |

### Methods

#### `Divider`: constructors and associated functions

- `pub fn new() -> Divider` - The theme's line, in no style of its own.

#### `Divider`: builder methods (take and return `self`)

- `pub fn kind(mut self, kind: BorderType) -> Divider` - Drawn with `kind`.
- `pub fn style(mut self, style: Style) -> Divider` - In `style`.
- `pub fn merge(mut self, merge: bool) -> Divider` - Joined to the lines it meets, or not.

### Also used in the code

- `rtwk::layout::VerticalDivider` (struct, `crates/rtwk-widgets/src/layout/box_.rs:651`) A line down the room it gets, one column wide, drawn with `kind` in `style`; with `merge` it joins the lines drawn before it (twk's `VerticalDivider`, a `Divider` down).
  - public fields: `kind: Option<BorderType>`, `style: Style`, `merge: bool`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::StackFit` (enum, `crates/rtwk-render/src/objects/stack.rs:23`) How a `RenderStack` lays out the children it places by its alignment.
  - variants: `Loose`, `Expand`, `Passthrough`

## Shadow

A drop shadow: the cells beside and below a child, moved by its offset, what is under them darkened.

API:

- `rtwk::layout::Shadow` (struct, `crates/rtwk-widgets/src/layout/shadow.rs:117`) Casts `shadow` from its child on the cells around it (twk's `Shadow`): the box is as big as the child and the room the shadow takes beside it.
  - public fields: `shadow: BoxShadow`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/shadow`, `crates/rtwk-gallery/src/pages/mod.rs:1604`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| offset | 2, 1, 1, 1, -2, -1 | 2, 1 |
| alpha | 64, 128, 192 | 128 |

### Methods

#### `Shadow`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Shadow` - `child` casting twk's default shadow.

#### `Shadow`: builder methods (take and return `self`)

- `pub fn shadow(mut self, shadow: BoxShadow) -> Shadow` - Casts `shadow`.

### Also used in the code

- `rtwk::layout::BoxShadow` (struct, `crates/rtwk-widgets/src/layout/shadow.rs:38`) A shadow: how far it is cast, in which color, how strongly (twk's `BoxShadow`).
  - public fields: `dx: i32`, `dy: i32`, `color: Color`, `alpha: i32`, `opacity: Option<f64>`
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::StackFit` (enum, `crates/rtwk-render/src/objects/stack.rs:23`) How a `RenderStack` lays out the children it places by its alignment.
  - variants: `Loose`, `Expand`, `Passthrough`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`

## Padding

Empty cells around a child.

API:

- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/padding`, `crates/rtwk-gallery/src/pages/mod.rs:1626`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| insets | all 1, x 4, y 2, left 8, left 3, top 1 | x 4 |

### Methods

#### `Padding`: constructors and associated functions

- `pub fn new(padding: EdgeInsets, child: impl Into<AnyWidget>) -> Padding` - `child` inset by `padding`.
- `pub fn empty(padding: EdgeInsets) -> Padding` - Only the padding, as big as it: the usual gap between the children of a row or a column.

## Align, Center

A child placed in the room it is given; Center in the middle of it.

API:

- `rtwk::layout::Align` (struct, `crates/rtwk-widgets/src/layout/basic.rs:103`) Places its child inside itself as `alignment` says (twk's `Align`, over a `RenderPositionedBox`).
  - public fields: `alignment: Alignment`, `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/align-center`, `crates/rtwk-gallery/src/pages/mod.rs:1639`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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
    room(".bottomRight", aligned.into()),
    room("Center", centred.into()),
]
.main_axis_size(MainAxisSize::Min)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| ax | -1, 0, 1 | 1 |
| ay | -1, 0, 1 | 1 |
| wf | null, 1, 2, 3 | null |
| hf | null, 1, 2, 3 | null |

### Methods

#### `Align`: constructors and associated functions

- `pub fn new(alignment: Alignment, child: impl Into<AnyWidget>) -> Align` - `child` at `alignment`.
- `pub fn empty(alignment: Alignment) -> Align` - As `new`, without a child.

#### `Align`: builder methods (take and return `self`)

- `pub fn width_factor(mut self, f: i32) -> Align` - This many times as wide as the child.
- `pub fn height_factor(mut self, f: i32) -> Align` - This many times as tall as the child.

#### `Center`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Center` - `child` in the middle.

#### `Center`: builder methods (take and return `self`)

- `pub fn width_factor(mut self, f: i32) -> Center` - This many times as wide as the child.
- `pub fn height_factor(mut self, f: i32) -> Center` - This many times as tall as the child.

### Also used in the code

- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`

## SizedBox

A child of a size: SizedBox.expand fills the room, SizedBox.shrink takes none.

API:

- `rtwk::layout::SizedBox` (struct, `crates/rtwk-widgets/src/layout/basic.rs:273`) Makes its child `width` wide and `height` tall, as far as the constraints allow; along an axis left none, the child is as long as it wants (twk's `SizedBox`, over a `RenderConstrainedBox` with tight constraints).
  - public fields: `width: Option<i32>`, `height: Option<i32>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/sizedbox`, `crates/rtwk-gallery/src/pages/mod.rs:1675`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| kind | sized, expand, shrink | sized |
| width | 0..44 (a slider) | 16 cells |
| height | 0..9 (a slider) | 3 rows |

### Methods

#### `SizedBox`: constructors and associated functions

- `pub fn new(width: Option<i32>, height: Option<i32>, child: Option<AnyWidget>) -> SizedBox` - A box `width` × `height` (either may be none) around `child`.
- `pub fn fixed(width: i32, height: i32) -> SizedBox` - A box `width` × `height` with no child.
- `pub fn expand() -> SizedBox` - As big as the constraints allow (twk's `SizedBox.expand`).
- `pub fn shrink() -> SizedBox` - As small as the constraints allow (twk's `SizedBox.shrink`).
- `pub fn square(dimension: Option<i32>) -> SizedBox` - `dimension` columns wide and `dimension` rows tall (twk's `SizedBox.square`); none leaves both to the child.

#### `SizedBox`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> SizedBox` - With `child` inside.

### Also used in the code

- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## ConstrainedBox

Constraints a child is laid out within, merged with those it is given.

API:

- `rtwk::layout::ConstrainedBox` (struct, `crates/rtwk-widgets/src/layout/basic.rs:379`) Lays its child out within `constraints` as well as its own: each limit is kept within those the box gets (twk's `ConstrainedBox`, over a `RenderConstrainedBox`).
  - public fields: `constraints: BoxConstraints`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/constrainedbox`, `crates/rtwk-gallery/src/pages/mod.rs:1716`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| min | 0, 12, 20 | 12 |
| max | 20, 32, 48 | 32 |

### Methods

#### `ConstrainedBox`: constructors and associated functions

- `pub fn new(constraints: BoxConstraints, child: impl Into<AnyWidget>) -> ConstrainedBox` - Lays `child` out within `constraints`.
- `pub fn empty(constraints: BoxConstraints) -> ConstrainedBox` - As `new`, without a child.

### Also used in the code

- `rtwk::layout::BoxConstraints` (struct, `crates/rtwk-render/src/constraints.rs:57`) The sizes a parent lets a child take in a layout: a width from `min_width` to `max_width` and a height from `min_height` to `max_height`, in cells.
  - public fields: `min_width: i32`, `max_width: i32`, `min_height: i32`, `max_height: i32`

## LimitedBox, OverflowBox

Other constraints than the room gives: LimitedBox limits a child only where the room has no limit, OverflowBox and SizedOverflowBox give it others in its place, cut off where it sticks out.

API:

- `rtwk::layout::LimitedBox` (struct, `crates/rtwk-widgets/src/layout/overflow.rs:41`) Limits its child's size only where the room is unbounded (a scroll view's main axis, a row's width in an unbounded parent); where the room is bounded the child gets it as it is (twk's `LimitedBox`).
  - public fields: `max_width: i32`, `max_height: i32`, `child: Option<AnyWidget>`
- `rtwk::layout::OverflowBox` (struct, `crates/rtwk-widgets/src/layout/overflow.rs:243`) Lays its child out within other limits than the ones it gets, and places it by `alignment` in the room it has (twk's `OverflowBox`): a child bigger than the box overflows it, and is cut off at the box's edges by the ...
  - public fields: `alignment: Alignment`, `min_width: Option<i32>`, `max_width: Option<i32>`, `min_height: Option<i32>`, `max_height: Option<i32>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/limitedbox-overflowbox`, `crates/rtwk-gallery/src/pages/mod.rs:1738`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| box | OverflowBox, SizedOverflowBox, LimitedBox | OverflowBox |
| alignment | centerLeft, center, centerRight | center |

### Methods

#### `LimitedBox`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> LimitedBox` - `child`, with no limit yet.
- `pub fn sized(max_width: i32, max_height: i32) -> LimitedBox` - With no child: a box of up to `max_width` × `max_height` where the room is unbounded (a placeholder of a size).

#### `LimitedBox`: builder methods (take and return `self`)

- `pub fn child(mut self, child: impl Into<AnyWidget>) -> LimitedBox` - With `child` inside.
- `pub fn max_width(mut self, w: i32) -> LimitedBox` - Limits the width where it is unbounded.
- `pub fn max_height(mut self, h: i32) -> LimitedBox` - Limits the height where it is unbounded.

#### `OverflowBox`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> OverflowBox` - `child` in the middle, within the box's own limits.

#### `OverflowBox`: builder methods (take and return `self`)

- `pub fn alignment(mut self, a: Alignment) -> OverflowBox` - Where the child goes.
- `pub fn min_width(mut self, v: i32) -> OverflowBox` - The least width the child gets.
- `pub fn max_width(mut self, v: i32) -> OverflowBox` - The most width the child gets.
- `pub fn min_height(mut self, v: i32) -> OverflowBox` - The least height the child gets.
- `pub fn max_height(mut self, v: i32) -> OverflowBox` - The most height the child gets.

### Also used in the code

- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`

## FractionallySizedBox

A child a fraction of the room it is given.

API:

- `rtwk::layout::FractionallySizedBox` (struct, `crates/rtwk-widgets/src/layout/basic.rs:439`) Makes its child a fraction of the room there is: `width_factor` of the most width the box may have, `height_factor` of the most height (`0.5` for half, `1.0 / 3.0` for a third), each rounded to the nearest cell, a half ...
  - public fields: `alignment: Alignment`, `width_factor: Option<f64>`, `height_factor: Option<f64>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/fractionallysizedbox`, `crates/rtwk-gallery/src/pages/mod.rs:1760`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| wf | 0.25, 0.5, 0.75, 1.0 | 0.5 |
| hf | 0.25, 0.5, 0.75, 1.0 | 0.5 |
| alignment | center, topLeft, bottomRight | center |
| room | 16..48 (a slider) | 40 cells |

### Methods

#### `FractionallySizedBox`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> FractionallySizedBox` - `child` in the middle of all the room.

#### `FractionallySizedBox`: builder methods (take and return `self`)

- `pub fn alignment(mut self, a: Alignment) -> FractionallySizedBox` - Where the child goes.
- `pub fn width_factor(mut self, f: f64) -> FractionallySizedBox` - The child's part of the width.
- `pub fn height_factor(mut self, f: f64) -> FractionallySizedBox` - The child's part of the height.

### Also used in the code

- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`

## IntrinsicWidth/Height

A child as wide or as tall as it would like to be: stretched items all as wide as the widest, boxes all as tall as the tallest.

API:

- `rtwk::layout::IntrinsicWidth` (struct, `crates/rtwk-widgets/src/layout/basic.rs:539`) Makes its child as wide as its content would like to be, within the room there is, rounded up to a multiple of `step_width` when given; with `step_height`, as tall as its content would like to be at that width, rounded ...
  - public fields: `step_width: Option<i32>`, `step_height: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::IntrinsicHeight` (struct, `crates/rtwk-widgets/src/layout/basic.rs:619`) Makes its child as tall as its content would like to be, within the room there is: a `Row` under it with `CrossAxisAlignment::Stretch` makes all its children as tall as the tallest (twk's `IntrinsicHeight`, over a ...
  - public fields: `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/intrinsicwidth-height`, `crates/rtwk-gallery/src/pages/mod.rs:1803`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| iw | on, off | true |
| ih | on, off | true |

### Methods

#### `IntrinsicWidth`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> IntrinsicWidth` - `child`, as wide as its content would like.

#### `IntrinsicWidth`: builder methods (take and return `self`)

- `pub fn step_width(mut self, step: i32) -> IntrinsicWidth` - Rounds the width up to a multiple of `step`.
- `pub fn step_height(mut self, step: i32) -> IntrinsicWidth` - Rounds the height up to a multiple of `step`.

#### `IntrinsicHeight`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> IntrinsicHeight` - `child`, as tall as its content would like.

## Row, Column, Flex

Children one after another along an axis, aligned along and across it.

API:

- `rtwk::layout::Row` (struct, `crates/rtwk-widgets/src/layout/flex.rs:197`) Its children side by side (twk's `Row`), over a `RenderFlex`.
- `rtwk::layout::Column` (struct, `crates/rtwk-widgets/src/layout/flex.rs:192`) Its children one under another (twk's `Column`), over a `RenderFlex`.
- `rtwk::layout::Flex` [path inferred] (struct, `crates/rtwk-widgets/src/layout/flex.rs:186`) Its children along `direction` (twk's `Flex`), over a `RenderFlex`: `Row` and `Column` are its two directions.

### Code

The gallery's Code tab at the initial options (route `#/layout/row-column-flex`, `crates/rtwk-gallery/src/pages/mod.rs:167`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| dir | row, column | row |
| main | start, end, center, spaceBetween, spaceAround, spaceEvenly | spaceBetween |
| cross | start, end, center, stretch | center |
| spacing | 0, 1, 2 | 1 |
| size | max, min | max |

### Methods

#### `Row`: constructors and associated functions

- `pub fn new() -> Row` - No children yet; twk's defaults: as wide as it may be, the children at the start and in the middle across, no gap.

#### `Row`: builder methods (take and return `self`)

- `pub fn child(mut self, w: impl Into<AnyWidget>) -> Row` - With `w` after the children so far.
- `pub fn children(mut self, ws: impl IntoIterator<Item = AnyWidget>) -> Row` - With `ws` after the children so far.
- `pub fn main_axis_size(mut self, s: MainAxisSize) -> Row` - How long it is along its axis.
- `pub fn main_axis_alignment(mut self, a: MainAxisAlignment) -> Row` - Where the children go along its axis.
- `pub fn cross_axis_alignment(mut self, a: CrossAxisAlignment) -> Row` - Where the children go across its axis.
- `pub fn spacing(mut self, g: i32) -> Row` - Cells between the children.
- `pub fn overflow_intended(mut self, on: bool) -> Row` - With `on`, it clips its children on purpose, a strip that shows the start of what is longer than it: in a debug build it then draws no overflow marker and logs no warning when they need more room than it has (twk's ...

#### `Row`: methods

- `pub fn direction(&self) -> Axis` - The direction it lays its children out along.

#### `Column`: constructors and associated functions

- `pub fn new() -> Column` - No children yet; twk's defaults: as tall as it may be, the children at the top and in the middle across, no gap.

#### `Column`: builder methods (take and return `self`)

- `pub fn child(mut self, w: impl Into<AnyWidget>) -> Column` - With `w` after the children so far.
- `pub fn children(mut self, ws: impl IntoIterator<Item = AnyWidget>) -> Column` - With `ws` after the children so far.
- `pub fn main_axis_size(mut self, s: MainAxisSize) -> Column` - How long it is along its axis.
- `pub fn main_axis_alignment(mut self, a: MainAxisAlignment) -> Column` - Where the children go along its axis.
- `pub fn cross_axis_alignment(mut self, a: CrossAxisAlignment) -> Column` - Where the children go across its axis.
- `pub fn spacing(mut self, g: i32) -> Column` - Cells between the children.
- `pub fn overflow_intended(mut self, on: bool) -> Column` - With `on`, it clips its children on purpose, a strip that shows the start of what is longer than it: in a debug build it then draws no overflow marker and logs no warning when they need more room than it has (twk's ...

#### `Column`: methods

- `pub fn direction(&self) -> Axis` - The direction it lays its children out along.

#### `Flex`: constructors and associated functions

- `pub fn new(direction: Axis) -> Flex` - No children yet; twk's defaults: as long as it may be along `direction`, the children at the start and in the middle across, no gap.

#### `Flex`: builder methods (take and return `self`)

- `pub fn child(mut self, w: impl Into<AnyWidget>) -> Flex` - With `w` after the children so far.
- `pub fn children(mut self, ws: impl IntoIterator<Item = AnyWidget>) -> Flex` - With `ws` after the children so far.
- `pub fn main_axis_size(mut self, s: MainAxisSize) -> Flex` - How long it is along its axis.
- `pub fn main_axis_alignment(mut self, a: MainAxisAlignment) -> Flex` - Where the children go along its axis.
- `pub fn cross_axis_alignment(mut self, a: CrossAxisAlignment) -> Flex` - Where the children go across its axis.
- `pub fn spacing(mut self, g: i32) -> Flex` - Cells between the children.
- `pub fn overflow_intended(mut self, on: bool) -> Flex` - With `on`, it clips its children on purpose, a strip that shows the start of what is longer than it: in a debug build it then draws no overflow marker and logs no warning when they need more room than it has (twk's ...

#### `Flex`: methods

- `pub fn direction(&self) -> Axis` - The direction it lays its children out along.

## Expanded, Spacer

Children of a Row or Column that share the room the others leave by their flex factors: Expanded fills its share, a Flexible may take less, a Spacer is empty.

API:

- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::layout::Spacer` (struct, `crates/rtwk-widgets/src/layout/flex.rs:348`) An empty `Expanded`: a gap that takes `flex` parts of the free room, to push its siblings apart (twk's `Spacer`).
  - public fields: `flex: i32`

### Code

The gallery's Code tab at the initial options (route `#/layout/expanded-spacer`, `crates/rtwk-gallery/src/pages/mod.rs:1825`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| flex | 1, 2, 3 | 1 |
| fit | loose, tight | loose |
| width | 20..50 (a slider) | 48 cells |

### Methods

#### `Expanded`: constructors and associated functions

- `pub fn new(flex: i32, child: impl Into<AnyWidget>) -> Expanded` - `child` filling a part `flex` of the free room.

#### `Spacer`: constructors and associated functions

- `pub fn new(flex: i32) -> Spacer` - A gap of `flex` parts.

### Also used in the code

- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk::layout::Flexible` (struct, `crates/rtwk-widgets/src/layout/flex.rs:243`) Gives its child, in a `Flex`, a `Row` or a `Column`, a share of the free room (twk's `Flexible`): `flex` against the other flexible children's, all of it (`FlexFit::Tight`) or at most it (`FlexFit::Loose`).
  - public fields: `flex: i32`, `fit: FlexFit`, `child: AnyWidget`
- `rtwk::layout::FlexFit` (enum, `crates/rtwk-render/src/objects/flex.rs:167`) How a flexible child takes its share of the free cells.
  - variants: `Tight`, `Loose`

## Stack, Positioned

Children over one another, pinned to the edges or aligned.

API:

- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/layout/stack-positioned`, `crates/rtwk-gallery/src/pages/mod.rs:1861`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| ax | -1, 0, 1 | 0 |
| ay | -1, 0, 1 | 0 |
| fit | loose, expand, passthrough | loose |

### Methods

#### `Stack`: constructors and associated functions

- `pub fn new() -> Stack` - No children yet; twk's defaults: the top left, each child as big as it wants.

#### `Stack`: builder methods (take and return `self`)

- `pub fn alignment(mut self, a: Alignment) -> Stack` - Where the children that are not positioned go.
- `pub fn fit(mut self, f: StackFit) -> Stack` - How the children that are not positioned are sized.

#### `Positioned`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Positioned` - `child` pinned nowhere yet; give it edges and sizes.
- `pub fn fill(child: impl Into<AnyWidget>) -> Positioned` - `child` filling the stack (twk's `Positioned.fill`): all four edges at 0.

#### `Positioned`: builder methods (take and return `self`)

- `pub fn left(mut self, v: i32) -> Positioned` - Columns from the stack's left edge.
- `pub fn top(mut self, v: i32) -> Positioned` - Rows from the stack's top edge.
- `pub fn right(mut self, v: i32) -> Positioned` - Columns from the stack's right edge.
- `pub fn bottom(mut self, v: i32) -> Positioned` - Rows from the stack's bottom edge.
- `pub fn width(mut self, v: i32) -> Positioned` - The child's width.
- `pub fn height(mut self, v: i32) -> Positioned` - The child's height.

### Also used in the code

- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::layout::StackFit` (enum, `crates/rtwk-render/src/objects/stack.rs:23`) How a `RenderStack` lays out the children it places by its alignment.
  - variants: `Loose`, `Expand`, `Passthrough`
- `rtwk::controls::Badge` (struct, `crates/rtwk-controls/src/status.rs:231`) A short `label` set off by its color: a count, a state; ` 3 `, ` new `.
  - public fields: `label: Str`, `kind: Option<StatusKind>`, `style: Option<Style>`
- `rtwk::layout::ConstrainedBox` (struct, `crates/rtwk-widgets/src/layout/basic.rs:379`) Lays its child out within `constraints` as well as its own: each limit is kept within those the box gets (twk's `ConstrainedBox`, over a `RenderConstrainedBox`).
  - public fields: `constraints: BoxConstraints`, `child: Option<AnyWidget>`
- `rtwk::layout::BoxConstraints` (struct, `crates/rtwk-render/src/constraints.rs:57`) The sizes a parent lets a child take in a layout: a width from `min_width` to `max_width` and a height from `min_height` to `max_height`, in cells.
  - public fields: `min_width: i32`, `max_width: i32`, `min_height: i32`, `max_height: i32`

## Transform

A child shown moved by whole cells, laid out where it was: Transform.translate, the one transform a terminal has.

API:

- `rtwk::layout::Transform` (struct, `crates/rtwk-widgets/src/layout/overflow.rs:638`) Moves where its child is drawn, by `offset` cells, without changing its layout (twk's `Transform.translate`): the box is as big as the child and the child is drawn `offset` away from it, cut off at the edges of what the ...
  - public fields: `offset: (i32`, `transform_hit_tests: bool`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/transform`, `crates/rtwk-gallery/src/pages/mod.rs:1892`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
const OFFSETS: [(i32, i32); 12] = [
    (0, -2), (5, -2), (9, -1), (10, 0),
    (9, 1), (5, 2), (0, 2), (-5, 2),
    (-9, 1), (-10, 0), (-9, -1), (-5, -2),
];

let theme = Theme::of(cx);
// In State::build, step is elapsed milliseconds / 200.
let (dx, dy) = OFFSETS[(self.step % 12) as usize];
let content = Center::new(BoxWidget::new().style(theme.selection.clone())
    .padding(EdgeInsets::x(1)).child(Text::new("moved")));
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Transform::translate(dx, dy, content));
SizedBox::new(Some(32), Some(7), Some(panel.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| move | round, fixed | round |
| dx | -12..12 (a slider) | 6 cells |
| dy | -2..2 (a slider) | 1 rows |

### Methods

#### `Transform`: constructors and associated functions

- `pub fn translate(dx: i32, dy: i32, child: impl Into<AnyWidget>) -> Transform` - `child` moved by `dx` columns and `dy` rows.

#### `Transform`: builder methods (take and return `self`)

- `pub fn transform_hit_tests(mut self, on: bool) -> Transform` - Whether the mouse hits the child where it is drawn.

### Also used in the code

- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## Visibility, Offstage

A child hidden: Offstage keeps it laid out and running without painting it; Visibility takes its room away too, or keeps it.

API:

- `rtwk::layout::Visibility` (struct, `crates/rtwk-widgets/src/layout/visibility.rs:262`) Shows its child, or hides it (twk's `Visibility`): when `visible` is off it shows `replacement` (an empty box by default) in its place, or, with `maintain_state`, keeps the child built offstage, or, with ...
  - public fields: `visible: bool`, `replacement: AnyWidget`, `maintain_state: bool`, `maintain_size: bool`, `child: AnyWidget`
- `rtwk::layout::Offstage` (struct, `crates/rtwk-widgets/src/layout/visibility.rs:222`) Keeps its child built — its state alive, its tree laid out — but neither shown nor given room, while `offstage` (twk's `Offstage`): the box is as small as its constraints allow, and the child is not painted nor hit by ...
  - public fields: `offstage: bool`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/visibility-offstage`, `crates/rtwk-gallery/src/pages/mod.rs:1947`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| widget | Visibility, Offstage | Visibility |
| hidden | on, off | true |
| keep | nothing, state, size | nothing |
| replacement | on, off | false |

### Methods

#### `Visibility`: constructors and associated functions

- `pub fn new(visible: bool, child: impl Into<AnyWidget>) -> Visibility` - `child`, shown when `visible`.

#### `Visibility`: builder methods (take and return `self`)

- `pub fn replacement(mut self, w: impl Into<AnyWidget>) -> Visibility` - What is shown in the child's place while it is not shown.
- `pub fn maintain_state(mut self, on: bool) -> Visibility` - Keeps the child built while it is not shown.
- `pub fn maintain_size(mut self, on: bool) -> Visibility` - Keeps the child's room while it is not shown.

#### `Offstage`: constructors and associated functions

- `pub fn new(offstage: bool, child: impl Into<AnyWidget>) -> Offstage` - `child`, offstage when `offstage`.

## ExpansionTile, Accordion

Sections that fold under their headers: Enter, Space or a click opens one, Right and Left open and close it; in an accordion one is open at a time, Up and Down go from header to header.

API:

- `rtwk::layout::ExpansionTile` (struct, `crates/rtwk-controls/src/expansion.rs:47`) A focusable header that expands or collapses its content.
- `rtwk::layout::Accordion` (struct, `crates/rtwk-controls/src/expansion.rs:592`) A vertical list of expansion tiles.

### Code

The gallery's Code tab at the initial options (route `#/layout/expansiontile-accordion`, `crates/rtwk-gallery/src/pages/mod.rs:2233`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// State holds verify = true and last = None.
// SectionMsg::Verify(value) stores the checkbox value.
// SectionMsg::Changed(index, expanded) records opened/closed for that section.
let theme = Theme::of(cx);
let section = |index, title, subtitle, child: AnyWidget, open| {
    let tile = ExpansionTile::new(Text::new(title), child)
        .initially_expanded(open)
        .duration(Duration::from_millis(150))
        .on_expansion_changed(cx.map(move |value| SectionMsg::Changed(index, value)));
    let tile = tile.subtitle(Text::new(subtitle));
    tile
};
let tiles = [
    section(0, "General", "Name and theme", Text::new("Name: twk\nTheme: dark").into(), true),
    section(1, "Network", "Proxy and timeouts", Text::new("Proxy: none\nTimeout: 30 s").into(), false),
    section(2, "Security", "TLS and certificates", column![
        Text::new("TLS: 1.3"),
        Checkbox::new(self.verify).label("verify certificates")
            .on_changed(cx.map(SectionMsg::Verify)),
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
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| accordion | on, off | true |
| multiple | on, off | false |
| subtitle | on, off | true |
| duration | 150, 400, 0 | 150 |

### Methods

#### `ExpansionTile`: constructors and associated functions

- `pub fn new(title: impl Into<AnyWidget>, child: impl Into<AnyWidget>) -> Self` - A collapsed section with the given header title and content.

#### `ExpansionTile`: builder methods (take and return `self`)

- `pub fn subtitle(mut self, subtitle: impl Into<AnyWidget>) -> Self` - Secondary content beneath the title.
- `pub fn trailing(mut self, trailing: impl Into<AnyWidget>) -> Self` - Content at the right edge of the header.
- `pub fn initially_expanded(mut self, expanded: bool) -> Self` - Whether an internally controlled section starts open.
- `pub fn controller(mut self, controller: impl Into<ExpansionController>) -> Self` - An externally owned controller that the tile observes.
- `pub fn on_expansion_changed(mut self, handler: Handler<bool>) -> Self` - The action emitted for each change of expansion.
- `pub fn duration(mut self, duration: Duration) -> Self` - Time to fold or unfold.
- `pub fn indent(mut self, indent: i32) -> Self` - Cells to indent the content.
- `pub fn focus_node(mut self, node: FocusNode) -> Self` - The focus node for the header.
- `pub fn header_style(mut self, style: Style) -> Self` - Style merged over the theme's text.
- `pub fn expanded_header_style(mut self, style: Style) -> Self` - Style merged over the resting header style while open.
- `pub fn hover_style(mut self, style: Style) -> Self` - Header hover style.

#### `Accordion`: constructors and associated functions

- `pub fn new(children: impl IntoIterator<Item = ExpansionTile>) -> Self` - A group of tiles, with one open at a time.

#### `Accordion`: builder methods (take and return `self`)

- `pub fn allow_multiple(mut self, allow: bool) -> Self` - Whether several tiles may remain open.
- `pub fn initially_expanded(mut self, index: usize) -> Self` - The index of the initially open internally controlled tile.

### Also used in the code

- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`

## Grid, GridPlacement

Children in the cells of tracks: fixed, a share of the rest, or as long as their content; a child may span several.

API:

- `rtwk::layout::Grid` (struct, `crates/rtwk-widgets/src/layout/grid.rs:34`) Its children in a grid of tracks (twk's `Grid`, over a `RenderGrid`): a child with a `GridPlacement` goes in its cell, the others after the one before them.
  - public fields: `columns: Vec<GridTrack>`, `rows: Vec<GridTrack>`, `column_gap: i32`, `row_gap: i32`, `overflow_intended: bool`
- `rtwk::layout::GridPlacement` (struct, `crates/rtwk-widgets/src/layout/grid.rs:144`) Puts its child, in a `Grid`, in a cell and over some tracks (twk's `GridPlacement`): a row and a column, or neither (after the child before it), and spans of 1 track or more.
  - public fields: `row: Option<i32>`, `column: Option<i32>`, `row_span: i32`, `column_span: i32`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/layout/grid-gridplacement`, `crates/rtwk-gallery/src/pages/mod.rs:1985`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let cell = |label| BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .child(Text::new(label));
let grid = Grid::new()
    .columns([GridTrack::fixed(12), GridTrack::fr(1), GridTrack::content()])
    .rows([GridTrack::content(), GridTrack::fr(1), GridTrack::content()])
    .column_gap(1).row_gap(0)
    .child(GridPlacement::new(cell("header · columnSpan: 3")).span(1, 3))
    .child(cell("fixed(12)"))
    .child(cell("fr(1)"))
    .child(cell("content()"))
    .child(GridPlacement::new(cell("columnSpan: 2")).at(2, 1).span(1, 2));
SizedBox::new(Some(44), Some(12), Some(grid.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| third | content, fixed, fr | content |
| cg | 0, 1, 2 | 1 |
| rg | 0, 1 | 0 |
| width | 32..50 (a slider) | 44 cells |

### Methods

#### `Grid`: constructors and associated functions

- `pub fn new() -> Grid` - No tracks and no children yet.

#### `Grid`: builder methods (take and return `self`)

- `pub fn columns(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Grid` - The columns' tracks.
- `pub fn rows(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Grid` - The rows' tracks.
- `pub fn column_gap(mut self, g: i32) -> Grid` - Cells between the columns.
- `pub fn row_gap(mut self, g: i32) -> Grid` - Cells between the rows.
- `pub fn overflow_intended(mut self, on: bool) -> Grid` - With `on`, it clips its tracks on purpose: in a debug build it then draws no overflow marker and logs no warning when they need more room than it has (twk's `overflowIntended`).

#### `GridPlacement`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> GridPlacement` - `child` after the one before it, in one cell.

#### `GridPlacement`: builder methods (take and return `self`)

- `pub fn at(mut self, row: i32, column: i32) -> GridPlacement` - In the cell at `row` and `column`.
- `pub fn span(mut self, rows: i32, columns: i32) -> GridPlacement` - Over `rows` rows and `columns` columns.

### Also used in the code

- `rtwk::layout::GridTrack` (struct, `crates/rtwk-render/src/objects/grid.rs:54`) The length of a column or a row of a `RenderGrid`, in whole cells: fixed, a fraction of the cells the other tracks leave (CSS `fr`), or as long as the children in it ask for (CSS `auto`).
  - public fields: `kind: GridTrackKind`, `fraction: i32`, `min: i32`, `max: i32`

## Wrap

Children in runs that go on in the next run where they would not fit: tags, buttons and key hints on a terminal that narrows.

API:

- `rtwk::layout::Wrap` (struct, `crates/rtwk-widgets/src/layout/wrap.rs:25`) Its children one after another along `direction`, a new run where the next does not fit (twk's `Wrap`, over a `RenderWrap`): tags that flow onto the next line.
  - public fields: `direction: Axis`, `alignment: WrapAlignment`, `run_alignment: WrapAlignment`, `cross_axis_alignment: WrapCrossAlignment`, `spacing: i32`, `run_spacing: i32`, `overflow_intended: bool`

### Code

The gallery's Code tab at the initial options (route `#/layout/wrap`, `crates/rtwk-gallery/src/pages/mod.rs:2028`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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
    .child(Badge::new("dart"))
    .child(Badge::new("terminal").kind(StatusKind::Info))
    .child(BoxWidget::new().border(rounded()).child(Text::new("a box")))
    .child(Badge::new("twk").kind(StatusKind::Success))
    .child(Badge::new("日本語").kind(StatusKind::Warning))
    .child(Badge::new("layout"))
    .child(Badge::new("runs").kind(StatusKind::Info));
let panel = BoxWidget::new().border(rounded()).child(wrap);
SizedBox::new(Some(width), Some(11), Some(panel.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 20..48 (a slider) | 36 cells |
| live | on, off | false |
| direction | horizontal, vertical | horizontal |
| alignment | start, end, center, spaceBetween, spaceAround, spaceEvenly | start |
| runAlignment | start, end, center, spaceBetween, spaceAround, spaceEvenly | start |
| cross | start, end, center | center |
| spacing | 0, 1, 2 | 1 |
| runSpacing | 0, 1 | 0 |

### Methods

#### `Wrap`: constructors and associated functions

- `pub fn new() -> Wrap` - No children yet; twk's defaults: horizontal runs, everything at the start, no gaps.

#### `Wrap`: builder methods (take and return `self`)

- `pub fn direction(mut self, d: Axis) -> Wrap` - The direction of a run.
- `pub fn alignment(mut self, a: WrapAlignment) -> Wrap` - How the children are spread along a run.
- `pub fn run_alignment(mut self, a: WrapAlignment) -> Wrap` - How the runs are spread across.
- `pub fn cross_axis_alignment(mut self, a: WrapCrossAlignment) -> Wrap` - How the children in a run are aligned across.
- `pub fn spacing(mut self, g: i32) -> Wrap` - Cells between the children in a run.
- `pub fn run_spacing(mut self, g: i32) -> Wrap` - Cells between the runs.
- `pub fn overflow_intended(mut self, on: bool) -> Wrap` - With `on`, it clips its runs on purpose: in a debug build it then draws no overflow marker and logs no warning when they need more room than it has (twk's `overflowIntended`).

### Also used in the code

- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::layout::WrapAlignment` (enum, `crates/rtwk-render/src/objects/wrap.rs:23`) Where a `RenderWrap` puts the children of a run along its main axis, in the cells they and the gaps leave free, or its runs across it.
  - variants: `Start`, `End`, `Center`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`
- `rtwk::layout::WrapCrossAlignment` (enum, `crates/rtwk-render/src/objects/wrap.rs:99`) Where a `RenderWrap` puts each child across its main axis, in its run.
  - variants: `Start`, `End`, `Center`
- `rtwk::controls::Badge` (struct, `crates/rtwk-controls/src/status.rs:231`) A short `label` set off by its color: a count, a state; ` 3 `, ` new `.
  - public fields: `label: Str`, `kind: Option<StatusKind>`, `style: Option<Style>`
- `rtwk::controls::StatusKind` (enum, `crates/rtwk-controls/src/status.rs:21`) What a `StatusMessage` or a `Badge` tells: each kind has its `icon` (from the theme's `Glyphs`) and its color in the theme (`style_in`).
  - variants: `Info`, `Success`, `Warning`, `Error`

## CustomPaint

Cells a CellPainter draws itself, under the child or over it, painted again when what it paints changes.

API:

- `rtwk::layout::CustomPaint` (struct, `crates/rtwk-widgets/src/layout/custom_paint.rs:115`) Paints cells with a `CellPainter` under its child and another over it (twk's `CustomPaint`).
  - public fields: `painter: Option<SharedPainter>`, `foreground_painter: Option<SharedPainter>`, `size: (i32`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/layout/custompaint`, `crates/rtwk-gallery/src/pages/mod.rs:2114`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

let theme = Theme::of(cx);
// In State::build, step is elapsed milliseconds / 100.
let phase = (self.step % 24) as i32;
let wave = Wave { phase, style: theme.primary.clone() };
let label = Text::new("cells of its own")
    .style(theme.text.merge(&Style::new().attrs(Attrs::BOLD)));
CustomPaint::new()
    .painter(wave)
    .child(SizedBox::new(Some(36), Some(6), Some(Center::new(label).into())))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| paints | under, over | under |
| moving | on, off | true |
| color | primary, accent, success | primary |

### Methods

#### `CustomPaint`: constructors and associated functions

- `pub fn new() -> CustomPaint` - Nothing painted yet, 0 × 0.

#### `CustomPaint`: builder methods (take and return `self`)

- `pub fn painter(mut self, p: impl CellPainter) -> CustomPaint` - Paints under the child with `p`.
- `pub fn shared_painter(mut self, p: SharedPainter) -> CustomPaint` - Paints under the child with the shared painter `p`: the same one across builds is never painted again for being replaced.
- `pub fn foreground_painter(mut self, p: impl CellPainter) -> CustomPaint` - Paints over the child with `p`.
- `pub fn size(mut self, width: i32, height: i32) -> CustomPaint` - The size, without a child.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> CustomPaint` - With `child` between the painters.

### Also used in the code

- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## SplitView

Panes the mouse or the arrows resize by their dividers; Tab focuses one. The last one collapses.

API:

- `rtwk::layout::SplitView` (struct, `crates/rtwk-widgets/src/layout/split_view.rs:44`) Panes separated by dividers that respond to the mouse and keys.

### Code

The gallery's Code tab at the initial options (route `#/layout/splitview`, `crates/rtwk-gallery/src/pages/mod.rs:2275`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// State owns split = SplitController::new().with_fractions([1.0, 2.0, 1.0]).
let theme = Theme::of(cx);
let pane = |title, text, collapsible| SplitPane::new(
    BoxWidget::new()
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded).merge(true))
        .title(BorderTitle::new(title).style(theme.muted.clone()))
        .padding(EdgeInsets::only(1, 0, 0, 0)).child(Text::new(text)),
).min_size(8).collapsible(collapsible);
let split = SplitView::new([
    pane("files", "box.dart\nflex.dart\ngrid.dart\nwrap.dart", false),
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
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| dir | horizontal, vertical | horizontal |
| share | on, off | true |
| type | single, rounded, double, heavy, ascii | single |

### Methods

#### `SplitView`: constructors and associated functions

- `pub fn new(panes: impl IntoIterator<Item = SplitPane>) -> Self` - A horizontal split with the given panes.

#### `SplitView`: builder methods (take and return `self`)

- `pub fn direction(mut self, direction: Axis) -> Self` - The axis of the split.
- `pub fn controller(mut self, controller: impl Into<SplitController>) -> Self` - The controller to use in place of the view's own.
- `pub fn share_edges(mut self, share: bool) -> Self` - Let adjacent panes share their edge with the divider.
- `pub fn kind(mut self, kind: BorderType) -> Self` - The divider line, overriding the theme.
- `pub fn style(mut self, style: Style) -> Self` - The divider's resting style.
- `pub fn focused_style(mut self, style: Style) -> Self` - The style merged over the resting style while focused or dragged.

### Also used in the code

- `rtwk::layout::SplitController` (struct, `crates/rtwk-widgets/src/layout/split.rs:32`) The shares of the room a `SplitView`'s panes have, and which are folded away (twk's `SplitController`); the view moves them as its dividers are dragged and keyed, the app saves and restores them.
- `rtwk::layout::SplitPane` (struct, `crates/rtwk-widgets/src/layout/split.rs:278`) One pane of a split view: what it holds and how small and how large it may be (twk's `SplitPane`).
  - public fields: `child: AnyWidget`, `min_size: i32`, `max_size: Option<i32>`, `collapsible: bool`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::layout::Flexible` (struct, `crates/rtwk-widgets/src/layout/flex.rs:243`) Gives its child, in a `Flex`, a `Row` or a `Column`, a share of the free room (twk's `Flexible`): `flex` against the other flexible children's, all of it (`FlexFit::Tight`) or at most it (`FlexFit::Loose`).
  - public fields: `flex: i32`, `fit: FlexFit`, `child: AnyWidget`

## DockArea

Panels in splits and tabs: a tab dragged onto a target of a group docks its panel there, and dropped elsewhere floats it; a window's title bar dragged onto one docks it again.

API:

- `rtwk::layout::DockArea` (struct, `crates/rtwk-controls/src/dock_area.rs:48`) A tabbed area whose panels can be resized, docked and floated.

### Code

The gallery's Code tab at the initial options (route `#/layout/dockarea`, `crates/rtwk-gallery/src/pages/mod.rs:2304`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn docked() -> DockLayout {
    let root = DockSplit::with_fractions(Axis::Horizontal, [
        DockTabs::new(["files", "find"]).into(), DockTabs::new(["editor"]).into(),
    ], [1.0, 2.0]);
    DockLayout::new(Some(root.into()), [DockWindow::new("log").at(23, 4).sized(26, 6)])
}
// State owns DockController::new(docked()) and count = 0.
// DockMsg::Reset calls self.controller.set_layout(docked()).
// DockMsg::Add increments self.count and rebuilds.
let theme = Theme::of(cx);
let panel = |id, title, child: AnyWidget| DockPanel::new(id, title, Padding::new(EdgeInsets::x(1), child));
let clicks = column![
    Text::rich([Span::styled("count: ", theme.muted.clone()), Span::new(self.count.to_string())]),
    Button::new("Add one").on_pressed(cx.send(DockMsg::Add)),
].cross_axis_alignment(CrossAxisAlignment::Start);
let area = DockArea::new(&self.controller, [
    panel("files", "Files", Text::new("Drag a tab onto a target to dock it there.").into()),
    panel("find", "Find", Text::new("Find in files.").into()),
    panel("editor", "Editor", clicks.into()),
    panel("log", "Log", Text::new("Drop a tab off the targets to float it.").into()),
]).tab_strip(true);
column![
    SizedBox::new(Some(50), Some(10), Some(area.into())),
    Button::new("Dock them as they were").on_pressed(cx.send(DockMsg::Reset)),
].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| strip | on, off | true |
| frame | null, raised, bordered | null |

### Methods

#### `DockArea`: constructors and associated functions

- `pub fn new(controller: impl Into<DockController>, panels: impl IntoIterator<Item = DockPanel>) -> Self` - The panels to show, placed by `controller`.

#### `DockArea`: builder methods (take and return `self`)

- `pub fn tab_strip(mut self, strip: bool) -> Self` - Whether the tab bars use a strip.
- `pub fn window_frame(mut self, frame: WindowFrame) -> Self` - The frame for the floating windows.

### Also used in the code

- `rtwk::layout::DockLayout` (struct, `crates/rtwk-widgets/src/layout/dock.rs:383`)
- `rtwk::layout::DockSplit` (struct, `crates/rtwk-widgets/src/layout/dock.rs:192`)
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::layout::DockTabs` (struct, `crates/rtwk-widgets/src/layout/dock.rs:129`)
- `rtwk::layout::DockWindow` (struct, `crates/rtwk-widgets/src/layout/dock.rs:314`)
  - public fields: `panel: String`, `left: i32`, `top: i32`, `width: i32`, `height: i32`
- `rtwk::layout::DockController` (struct, `crates/rtwk-widgets/src/layout/dock.rs:709`) Docks, floats, selects and removes panels in a `DockLayout` (twk's `DockController`): the layout is replaced by each, and the listeners are told of each change and of none else.
- `rtwk::layout::DockPanel` (struct, `crates/rtwk-widgets/src/layout/dock.rs:673`) A panel an area can show: its id (the name a layout calls it by), its title (on its tab) and what it holds (twk's `DockPanel`).
  - public fields: `id: String`, `title: String`, `child: AnyWidget`
- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## Scaffold

An app's frame: a header, a sidebar beside the body, a status bar and the key hints, each part a zone of the focus; the sidebar collapsed on a narrow screen and resized by a drag.

API:

- `rtwk::navigation::Scaffold` (struct, `crates/rtwk/src/navigation/scaffold.rs:684`) The frame most full-screen apps are built in: `header` rows on top, `sidebar` beside `body` under them, the `status_bar` row and the key hints (a `KeyHintsBar`, with `key_hints`) at the bottom.
  - public fields: `body: AnyWidget`, `header: Option<AnyWidget>`, `sidebar: Option<AnyWidget>`, `status_bar: Option<AnyWidget>`, `key_hints: bool`, `sidebar_width: i32`, `sidebar_side: SidebarSide`, `resizable_sidebar: bool`, `collapsed_below: Option<i32>`, `on_sidebar_resized: Option<Handler<i32>>`, `controller: Option<ScaffoldController>`

### Code

The gallery's Code tab at the initial options (route `#/layout/scaffold`, `crates/rtwk-gallery/src/pages/mod.rs:2143`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Retained in State, with inbox = true, archive = false and notes = 3.
// controller: ScaffoldController::new().initial_sidebar_width(14)
#[derive(Clone, PartialEq)]
enum NotesMsg { Inbox(bool), Archive(bool), New }
// In State::update:
match msg {
    NotesMsg::Inbox(value) => self.inbox = value,
    NotesMsg::Archive(value) => self.archive = value,
    NotesMsg::New => self.notes += 1,
}
// In State::build:
let theme = Theme::of(cx);
let sidebar = column![
    Checkbox::new(self.inbox).label("inbox").on_changed(cx.map(NotesMsg::Inbox)),
    Checkbox::new(self.archive).label("archive").on_changed(cx.map(NotesMsg::Archive)),
].cross_axis_alignment(CrossAxisAlignment::Start);
let body = column![
    Button::new("New note").on_pressed(cx.send(NotesMsg::New)),
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
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 30..48 (a slider) | 48 cells |
| side | left, right | left |
| resizable | on, off | true |
| hints | on, off | true |
| header | on, off | true |
| status | on, off | true |
| collapsed | null, 36, 40, 44 | 40 |

### Methods

#### `Scaffold`: constructors and associated functions

- `pub fn new(body: impl Into<AnyWidget>) -> Scaffold` - The frame of `body` and of the parts given around it.

#### `Scaffold`: builder methods (take and return `self`)

- `pub fn header(mut self, header: impl Into<AnyWidget>) -> Scaffold` - With `header` rows on top.
- `pub fn sidebar(mut self, sidebar: impl Into<AnyWidget>) -> Scaffold` - With `sidebar` beside the body.
- `pub fn status_bar(mut self, status_bar: impl Into<AnyWidget>) -> Scaffold` - With `status_bar` under the body.
- `pub fn key_hints(mut self, on: bool) -> Scaffold` - With the key hints at the bottom, or without.
- `pub fn sidebar_width(mut self, width: i32) -> Scaffold` - With a sidebar `width` columns wide.
- `pub fn sidebar_side(mut self, side: SidebarSide) -> Scaffold` - With the sidebar on `side` of the body.
- `pub fn resizable_sidebar(mut self, on: bool) -> Scaffold` - With a divider that resizes the sidebar, or without.
- `pub fn collapsed_below(mut self, columns: i32) -> Scaffold` - With the sidebar collapsed on a screen narrower than `columns`.
- `pub fn on_sidebar_resized(mut self, h: Handler<i32>) -> Scaffold` - Told the sidebar's width at the end of a drag and at each key that resized it.
- `pub fn controller(mut self, controller: impl Into<ScaffoldController>) -> Scaffold` - Served by `controller`.

### Also used in the code

- `rtwk::navigation::ScaffoldController` (struct, `crates/rtwk/src/navigation/scaffold.rs:383`) Whether the sidebar of a `Scaffold` is shown and how wide it is: what an app sets to show, hide or size it, and saves to show it as it was.
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`
- `rtwk::navigation::SidebarSide` (enum, `crates/rtwk/src/navigation/scaffold.rs:71`) The side of the body a `Scaffold`'s sidebar is on.
  - variants: `Left`, `Right`
- `rtwk::controls::StatusBar` (struct, `crates/rtwk-controls/src/status_bar.rs:60`) A status bar: one row of three sections, `left` at its left edge, `right` at its right edge and `center` in its middle.
  - public fields: `left: Option<AnyWidget>`, `center: Option<AnyWidget>`, `right: Option<AnyWidget>`, `spacing: i32`, `style: Option<Style>`
