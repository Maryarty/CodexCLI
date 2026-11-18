<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Style

[All widgets](../widgets.md)

## Theme

The styles of an app by what they are for, which the widgets below it draw in: a preset, or the terminal's own colors.

API:

- `rtwk::style::Theme` (struct, `crates/rtwk-widgets/src/style/theme.rs:861`) The theme of the widgets below it (`Theme::of`) (twk's `Theme`): `data`, or with `Theme::adaptive` the `dark` or the `light` theme by the brightness of the terminal (`TerminalBrightness`), or with `Theme::terminal` the ...
  - public fields: `data: Option<Rc<ThemeData>>`, `dark: Option<Rc<ThemeData>>`, `light: Option<Rc<ThemeData>>`, `child: AnyWidget`
- `rtwk::style::ThemeData` (struct, `crates/rtwk-widgets/src/style/theme.rs:131`) The styles of an app by what they are for, which widgets draw in (twk's `ThemeData`): the background slots (`background`, `surface`, `selection`) set a background, the others a text color.
  - public fields: `brightness: Brightness`, `background: Style`, `surface: Style`, `text: Style`, `muted: Style`, `primary: Style`, `accent: Style`, `border: Style`, `selection: Style`, `error: Style`, `warning: Style`, `success: Style`, `info: Style`, `chrome: Option<Style>`, `well: Option<Style>`, `faint: Option<Style>`, `dim: Option<Style>`, `selection_idle: Option<Style>`, `hover: Option<Style>`, `on_primary: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/style/theme`, `crates/rtwk-gallery/src/pages/style.rs:59`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn swatch_card(
    theme: &ThemeData,
    named: Option<(&str, AnyWidget)>,
) -> SizedBox {
    let swatches = |entries: &[(&str, &Style)]| {
        Text::rich(entries.iter()
            .map(|(name, style)| Span::styled(format!("{name}  "), (*style).clone()))
            .collect::<Vec<_>>())
    };
    let mut sample = column![
        swatches(&[("text", &theme.text), ("muted", &theme.muted), ("primary", &theme.primary),
            ("accent", &theme.accent), ("border", &theme.border)]),
        swatches(&[("error", &theme.error), ("warning", &theme.warning),
            ("success", &theme.success), ("info", &theme.info)]),
        SizedBox::new(None, Some(1), None),
        BoxWidget::new()
            .style(theme.surface.clone())
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .titles([BorderTitle::new(" surface ").style(theme.muted.clone())])
            .padding(EdgeInsets::x(1))
            .child(Text::rich([
                Span::new("a panel, with "),
                Span::styled(" selection ", theme.selection.clone()),
                Span::new(" in it"),
            ])),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut card = BoxWidget::new()
        .style(theme.background.merge(&theme.text))
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .padding(EdgeInsets::x(2));
    if let Some((name, controls)) = named {
        sample = sample
            .child(SizedBox::new(None, Some(1), None))
            .child(controls)
            .child(SizedBox::new(None, Some(1), None))
            .child(row![
                StatusMessage::new("Saved").kind(StatusKind::Success),
                Expanded::new(1, LineGauge::new(0.6)
                    .unfilled_style(Style::new().attrs(Attrs::DIM))),
            ].spacing(2));
        card = card.titles([BorderTitle::new(format!(" {name} "))
            .style(theme.text.merge(&Style::new().attrs(Attrs::BOLD)))]);
    }
    SizedBox::new(Some(46), None, Some(card.child(sample).into()))
}

let (name, data) = ("nord", ThemeData::NORD);
// The values and the handlers are kept by the application.
let controls = row![
    Button::new("Button").on_pressed(on_pressed),
    Checkbox::new(checked).label("Checkbox").on_changed(on_checked),
    Switch::new(on).label("Switch").on_changed(on_switched),
].spacing(3);
let controls = AnyWidget::from(controls);
Theme::new(data, LayoutBuilder::new(move |cx, _| {
    swatch_card(&Theme::of(cx), Some((name, controls.clone()))).into()
}))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| theme | terminal, dark, light, nord, mocha, latte | nord |
| cycle | on, off | false |
| glyphs | unicode, ascii | unicode |

### Methods

#### `Theme`: constructors and associated functions

- `pub fn new(data: impl Into<Rc<ThemeData>>, child: impl Into<AnyWidget>) -> Theme` - Gives the widgets below `data` whatever the terminal's brightness.
- `pub fn terminal(child: impl Into<AnyWidget>) -> Theme` - Gives the widgets below the terminal's own colors (`ThemeData::terminal_of`).
- `pub fn adaptive(child: impl Into<AnyWidget>) -> Theme` - Gives the widgets below the dark theme on a dark terminal and the light one on a light one, `ThemeData::DARK` and `ThemeData::LIGHT` unless `dark` and `light` say otherwise, switched when the terminal reports another ...
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> Rc<ThemeData>` - The theme at `cx`: that of the nearest `Theme`; without one `ThemeData::DARK` on a dark terminal and `ThemeData::LIGHT` on a light one, and the terminal's own colors where the terminal did not say which it is ...

#### `Theme`: builder methods (take and return `self`)

- `pub fn dark(mut self, data: impl Into<Rc<ThemeData>>) -> Theme` - This adaptive theme with `data` on a dark terminal.
- `pub fn light(mut self, data: impl Into<Rc<ThemeData>>) -> Theme` - This adaptive theme with `data` on a light terminal.

#### `ThemeData`: constructors and associated functions

- `pub const fn new(brightness: Brightness, background: Style, surface: Style, text: Style, muted: Style, primary: Style, accent: Style, border: Style, selection: Style, error: Style, warning: Style, success: Style, info: Style) -> ThemeData` - A theme of the twelve slots and its brightness, every token derived (twk's `ThemeData(...)` without the optional ones): set them with `copy_with` or the struct's fields.
- `pub const fn terminal(brightness: Brightness) -> ThemeData` - The terminal's own colors, so that its color scheme shows through: its default background and text color and the colors of its palette; greys of the 256 colors, which no color scheme changes, for `surface`, `muted`, ...
- `pub fn terminal_on(background: u32, foreground: Option<u32>, palette: Palette) -> ThemeData` - The terminal's own colors on a terminal that told its default `background` and `foreground` (`0xRRGGBB`; `foreground` white on a dark background and black on a light one where it did not): as `ThemeData::terminal` of ...
- `pub fn terminal_of<S>(cx: &mut Cx<'_, S>) -> Rc<ThemeData>` - The terminal's own colors as `cx` has them: `ThemeData::terminal_on` where the terminal told its background (`TerminalBrightness`), else `ThemeData::terminal` of its brightness.
- `pub fn lerp(a: &ThemeData, b: &ThemeData, t: f64) -> ThemeData` - The theme `t` of the way from `a` to `b`, for a cross-fade: `a` at 0, `b` at 1.

#### `ThemeData`: methods

- `pub fn copy_with(&self, f: impl FnOnce(&mut ThemeData)) -> ThemeData` - This theme with the fields `f` changes (twk's `copyWith`): `ThemeData::DARK.copy_with(|t| t.pointer_mode = PointerMode::Off)`.
- `pub fn sets_pointer_shapes(&self) -> bool` - Whether the widgets set the pointer's shape at all.
- `pub fn chrome(&self) -> Style` - The app's frame around its content (a header, a sidebar, the key hints), a background: a step from `background` towards `surface`, half of it on a dark theme, 0.55 on a light one; `surface` where either is not RGB.
- `pub fn well(&self) -> Style` - A well sunk into the chrome, a background: text fields.
- `pub fn faint(&self) -> Style` - Quiet lines, a text color: rules that only separate.
- `pub fn dim(&self) -> Style` - Text that matters least, a text color: counts, line numbers, sizes.
- `pub fn selection_idle(&self) -> Style` - A selection whose list has not the focus, a background: `selection` 0.35 of the way to `background`; `selection` where either is not RGB.
- `pub fn hover(&self) -> Style` - What the mouse is over, a background: `background` 0.08 of the way to `text`; `selection_idle` where either is not RGB.
- `pub fn fill(&self, color: Option<Color>) -> Style` - A fill of `color` with text that reads on it, a background and a text color: a badge, a focused button, a tab or a cursor lit.
- `pub fn on_primary(&self) -> Style` - Text on a `primary` fill, a text color: a focused button.
- `pub fn shade(&self) -> Style` - What a modal barrier dims what is under it with, a background: black.
- `pub fn shade_alpha(&self) -> f64` - How much of `shade` a modal barrier blends over what is under it, from 0 to 1: 0.5 on a dark theme, 0.28 on a light one.

### Also used in the code

- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::controls::StatusMessage` (struct, `crates/rtwk-controls/src/status.rs:110`) A `message` of a `kind` after the kind's `icon` in its color: `✓ Saved 3 files`.
  - public fields: `message: Str`, `kind: StatusKind`, `title: Option<Str>`, `icon: Option<Str>`, `style: Option<Style>`, `icon_style: Option<Style>`
- `rtwk::controls::StatusKind` (enum, `crates/rtwk-controls/src/status.rs:21`) What a `StatusMessage` or a `Badge` tells: each kind has its `icon` (from the theme's `Glyphs`) and its color in the theme (`style_in`).
  - variants: `Info`, `Success`, `Warning`, `Error`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::data::LineGauge` (struct, `crates/rtwk-data/src/gauge.rs:215`) `ratio` as a line after `label` (or with `show_percentage` the ratio as a percentage) and a blank: the part from the left filled in `filled_style`, the rest in `unfilled_style`, to the nearest half cell where `kind` has ...
  - public fields: `ratio: f64`, `label: Option<Str>`, `show_percentage: bool`, `kind: Option<BorderType>`, `style: Style`, `filled_style: Style`, `unfilled_style: Style`, `segmented: bool`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Switch` (struct, `crates/rtwk-controls/src/toggle.rs:193`) A switch before `label`, on while `value` is true and off while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `pill: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## TerminalBrightness

Whether the terminal is dark or light, as it reports it; Theme.adaptive picks a dark or a light theme by it.

API:

- `rtwk::style::TerminalBrightness` (struct, `crates/rtwk-widgets/src/style/theme.rs:1008`) Whether the terminal's colors are dark or light, for the widgets below it (`TerminalBrightness::of`) (twk's `TerminalBrightness`); `Theme::adaptive` picks its theme by it.
  - public fields: `brightness: Brightness`, `known: bool`, `background: Option<u32>`, `foreground: Option<u32>`, `palette: Palette`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/style/terminalbrightness`, `crates/rtwk-gallery/src/pages/style.rs:79`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn swatch_card(
    theme: &ThemeData,
    named: Option<(&str, AnyWidget)>,
) -> SizedBox {
    let swatches = |entries: &[(&str, &Style)]| {
        Text::rich(entries.iter()
            .map(|(name, style)| Span::styled(format!("{name}  "), (*style).clone()))
            .collect::<Vec<_>>())
    };
    let mut sample = column![
        swatches(&[("text", &theme.text), ("muted", &theme.muted), ("primary", &theme.primary),
            ("accent", &theme.accent), ("border", &theme.border)]),
        swatches(&[("error", &theme.error), ("warning", &theme.warning),
            ("success", &theme.success), ("info", &theme.info)]),
        SizedBox::new(None, Some(1), None),
        BoxWidget::new()
            .style(theme.surface.clone())
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .titles([BorderTitle::new(" surface ").style(theme.muted.clone())])
            .padding(EdgeInsets::x(1))
            .child(Text::rich([
                Span::new("a panel, with "),
                Span::styled(" selection ", theme.selection.clone()),
                Span::new(" in it"),
            ])),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut card = BoxWidget::new()
        .style(theme.background.merge(&theme.text))
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .padding(EdgeInsets::x(2));
    if let Some((name, controls)) = named {
        sample = sample
            .child(SizedBox::new(None, Some(1), None))
            .child(controls)
            .child(SizedBox::new(None, Some(1), None))
            .child(row![
                StatusMessage::new("Saved").kind(StatusKind::Success),
                Expanded::new(1, LineGauge::new(0.6)
                    .unfilled_style(Style::new().attrs(Attrs::DIM))),
            ].spacing(2));
        card = card.titles([BorderTitle::new(format!(" {name} "))
            .style(theme.text.merge(&Style::new().attrs(Attrs::BOLD)))]);
    }
    SizedBox::new(Some(46), None, Some(card.child(sample).into()))
}

let adaptive = LayoutBuilder::new(|cx, _| {
    let (brightness, name) = match TerminalBrightness::of(cx) {
        Brightness::Dark => ("dark", "nord"),
        Brightness::Light => ("light", "catppuccinLatte"),
    };
    let card = LayoutBuilder::new(|cx, _| swatch_card(&Theme::of(cx), None).into());
    column![
        Text::rich([Span::new("The terminal is "), Span::new(brightness).bold(), Span::new(".")]),
        Text::rich([Span::new("Theme.adaptive picks "), Span::new(name).bold()])
            .style(Theme::of(cx).muted.clone()),
        Theme::adaptive(card).dark(ThemeData::NORD).light(ThemeData::CATPPUCCIN_LATTE),
    ].main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start).into()
});
// Use the brightness reported by the application's terminal.
adaptive.into()
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| brightness | none, dark, light | none |
| dark | nord, dark, mocha | nord |
| light | latte, light | latte |

### Methods

#### `TerminalBrightness`: constructors and associated functions

- `pub fn new(brightness: Brightness, child: impl Into<AnyWidget>) -> TerminalBrightness` - Gives `child`'s subtree `brightness`, which the terminal said (unless `known` says otherwise).
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> Brightness` - The brightness at `cx`: that of the nearest `TerminalBrightness`, dark without one.
- `pub fn is_known<S>(cx: &mut Cx<'_, S>) -> bool` - Whether the brightness at `cx` is the terminal's own word: the nearest one's `known`, false without one.
- `pub fn background_of<S>(cx: &mut Cx<'_, S>) -> Option<u32>` - The terminal's default background at `cx`; `None` without one or where the terminal did not tell it.
- `pub fn foreground_of<S>(cx: &mut Cx<'_, S>) -> Option<u32>` - The terminal's default text color at `cx`; `None` without one or where the terminal did not tell it.
- `pub fn palette_of<S>(cx: &mut Cx<'_, S>) -> Palette` - The palette the terminal told at `cx`; empty without one.

#### `TerminalBrightness`: builder methods (take and return `self`)

- `pub fn known(mut self, known: bool) -> TerminalBrightness` - This with `known` (false: the brightness is a guess).
- `pub fn background(mut self, rgb: u32) -> TerminalBrightness` - This with the background the terminal told.
- `pub fn foreground(mut self, rgb: u32) -> TerminalBrightness` - This with the text color the terminal told.
- `pub fn palette(mut self, palette: Palette) -> TerminalBrightness` - This with the palette the terminal told.

### Also used in the code

- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::controls::StatusMessage` (struct, `crates/rtwk-controls/src/status.rs:110`) A `message` of a `kind` after the kind's `icon` in its color: `✓ Saved 3 files`.
  - public fields: `message: Str`, `kind: StatusKind`, `title: Option<Str>`, `icon: Option<Str>`, `style: Option<Style>`, `icon_style: Option<Style>`
- `rtwk::controls::StatusKind` (enum, `crates/rtwk-controls/src/status.rs:21`) What a `StatusMessage` or a `Badge` tells: each kind has its `icon` (from the theme's `Glyphs`) and its color in the theme (`style_in`).
  - variants: `Info`, `Success`, `Warning`, `Error`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::data::LineGauge` (struct, `crates/rtwk-data/src/gauge.rs:215`) `ratio` as a line after `label` (or with `show_percentage` the ratio as a percentage) and a blank: the part from the left filled in `filled_style`, the rest in `unfilled_style`, to the nearest half cell where `kind` has ...
  - public fields: `ratio: f64`, `label: Option<Str>`, `show_percentage: bool`, `kind: Option<BorderType>`, `style: Style`, `filled_style: Style`, `unfilled_style: Style`, `segmented: bool`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::style::Brightness` (enum, `crates/rtwk-input/src/events.rs:543`) Whether colors are dark or light: a terminal's background, its color scheme (`InputEvent::ColorScheme`).
  - variants: `Dark`, `Light`

## GradientBackground

A background whose color changes from cell to cell, along the columns or down the rows, from stop to stop.

API:

- `rtwk::style::GradientBackground` (struct, `crates/rtwk-widgets/src/style/gradient_widgets.rs:23`) Its child over a gradient (twk's `GradientBackground`): each cell a blank on the gradient's color there, its text color kept, which shows where the child leaves the background as it is.
  - public fields: `gradient: Gradient`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/style/gradientbackground`, `crates/rtwk-gallery/src/pages/style.rs:104`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock while the gradient flows.
let theme = Theme::of(cx);
let colors = [Color::rgb(0xff, 0x5f, 0x6d), Color::rgb(0xff, 0xc3, 0x71)];
let gradient = Gradient::new(colors)
    .direction(Axis::Horizontal)
    .phase(f64::from(i32::try_from(elapsed_ms.rem_euclid(4_000)).unwrap()) / 4_000.0);

let card = BoxWidget::new()
    .style(theme.surface.clone())
    .padding(EdgeInsets::only(2, 1, 2, 1))
    .child(Text::new("a box over a gradient"));
let center = Center::new(card);
let backdrop = SizedBox::new(Some(44), Some(9), Some(center.into()));
GradientBackground::new(gradient, Some(backdrop.into())).into()
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| colors | theme, sunset, ocean, palette | sunset |
| direction | columns, rows | columns |
| stops | even, inset, early | even |
| flow | on, off | true |

### Methods

#### `GradientBackground`: constructors and associated functions

- `pub fn new(gradient: Gradient, child: Option<AnyWidget>) -> GradientBackground` - `child` over `gradient`.

### Also used in the code

- `rtwk::style::Gradient` (struct, `crates/rtwk-widgets/src/style/gradient.rs:40`) Colors that change from cell to cell along the columns of a widget, left to right (`Axis::Horizontal`), or along its rows, top to bottom (`Axis::Vertical`) (twk's `Gradient`).
  - public fields: `colors: Rc<[Color]>`, `stops: Option<Rc<[f64]>>`, `direction: Axis`, `phase: Option<f64>`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## GradientForeground

Text whose color changes from glyph to glyph along each line, or from line to line down the rows.

API:

- `rtwk::style::GradientForeground` (struct, `crates/rtwk-widgets/src/style/gradient_widgets.rs:43`) Its child with its text in the colors of a gradient (twk's `GradientForeground`): each glyph in that of its column (or row), a two-column glyph in that of its first; what the child paints keeps its background and ...
  - public fields: `gradient: Gradient`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/style/gradientforeground`, `crates/rtwk-gallery/src/pages/style.rs:111`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Redraw from the application clock while the gradient flows.
let theme = Theme::of(cx);
let colors = [theme.primary.fg.unwrap_or(Color::DEFAULT), theme.accent.fg.unwrap_or(Color::DEFAULT)];
let gradient = Gradient::new(colors)
    .direction(Axis::Horizontal)
    .phase(f64::from(i32::try_from(elapsed_ms.rem_euclid(4_000)).unwrap()) / 4_000.0);

column![
    GradientForeground::new(gradient.clone(), Some(Text::new("████████╗██╗   ██╗██╗\n╚══██╔══╝██║   ██║██║\n   ██║   ██║   ██║██║\n   ██║   ██║   ██║██║\n   ██║   ╚██████╔╝██║\n   ╚═╝    ╚═════╝ ╚═╝\n\nColors that change from glyph to glyph\nalong each line, or down the rows.").into())),
    Text::rich([
        Span::styled("a span: ", theme.muted.clone()),
        Span::gradient("Span.gradient, glyph by glyph", gradient),
    ]),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
.into()
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| colors | theme, sunset, ocean, palette | theme |
| direction | columns, rows | columns |
| stops | even, inset, early | even |
| flow | on, off | true |

### Methods

#### `GradientForeground`: constructors and associated functions

- `pub fn new(gradient: Gradient, child: Option<AnyWidget>) -> GradientForeground` - `child` with its text along `gradient`.

### Also used in the code

- `rtwk::style::Gradient` (struct, `crates/rtwk-widgets/src/style/gradient.rs:40`) Colors that change from cell to cell along the columns of a widget, left to right (`Axis::Horizontal`), or along its rows, top to bottom (`Axis::Vertical`) (twk's `Gradient`).
  - public fields: `colors: Rc<[Color]>`, `stops: Option<Rc<[f64]>>`, `direction: Axis`, `phase: Option<f64>`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
