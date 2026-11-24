<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Style

Themes, the terminal's brightness, and colors that run across a background or text.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Theme

The styles of an app by what they are for, which the widgets below it draw in: a preset, or the terminal's own colors.

- `rtwk::style::Theme` (struct) in `crates/rtwk-widgets/src/style/theme.rs:894`: The theme of the widgets below it (Theme::of) (twk's `Theme`): `data`, or with Theme::adaptive the `dark` or the `light` theme by the brightness of the terminal (TerminalBrightness), or with Theme::terminal the terminal's own colors.

### Code

```rust
fn themed(on_pressed: Handler<()>, checked: bool, on_checked: Handler<bool>, on: bool, on_switched: Handler<bool>) -> impl Into<AnyWidget> {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| theme | terminal, dark, light, nord, mocha, latte | nord |
| cycle | false, true | false |
| glyphs | unicode, ascii | unicode |

## TerminalBrightness

Whether the terminal is dark or light, as it reports it; Theme::adaptive picks a dark or a light theme by it.

- `rtwk::style::TerminalBrightness` (struct) in `crates/rtwk-widgets/src/style/theme.rs:1038`: Whether the terminal's colors are dark or light, for the widgets below it (TerminalBrightness::of) (twk's `TerminalBrightness`); Theme::adaptive picks its theme by it.

### Code

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
        Brightness::Light => ("light", "catppuccin latte"),
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
AnyWidget::from(adaptive)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| brightness | the terminal's, dark, light | the terminal's |
| dark theme | nord, dark, mocha | nord |
| light theme | latte, light | latte |

## GradientBackground

A background whose color changes from cell to cell, along the columns or down the rows, from stop to stop.

- `rtwk::style::GradientBackground` (struct) in `crates/rtwk-widgets/src/style/gradient_widgets.rs:23`: Its child over a gradient (twk's `GradientBackground`): each cell a blank on the gradient's color there, its text color kept, which shows where the child leaves the background as it is.

### Code

```rust
fn flowing(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
    // Redraw from the application clock while the gradient flows.
    use rtwk::style::Axis;
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| colors | theme, sunset, ocean, palette | sunset |
| direction | columns, rows | columns |
| stops | even, inset, early | even |
| flow | false, true | true |

## GradientForeground

Text whose color changes from glyph to glyph along each line, or from line to line down the rows.

- `rtwk::style::GradientForeground` (struct) in `crates/rtwk-widgets/src/style/gradient_widgets.rs:43`: Its child with its text in the colors of a gradient (twk's `GradientForeground`): each glyph in that of its column (or row), a two-column glyph in that of its first; what the child paints keeps its background and attributes, its text color replaced.

### Code

```rust
fn flowing(elapsed_ms: i64, cx: &mut Cx<'_>) -> AnyWidget {
    // Redraw from the application clock while the gradient flows.
    use rtwk::style::Axis;
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| colors | theme, sunset, ocean, palette | theme |
| direction | columns, rows | columns |
| stops | even, inset, early | even |
| flow | false, true | true |
