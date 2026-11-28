<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Text

Paragraphs and spans of styled text, wrapped, cut off, aligned, and right to left.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Text

A paragraph in one style, its lines broken to fit, cut off and aligned as asked.

- `rtwk::text::Text` (struct) in `crates/rtwk-widgets/src/text/widgets.rs:66`: Text in style merged onto the DefaultTextStyle above it: a string (data), or spans, each in its own style merged onto that one (Text::rich), a span with a gradient (Span::gradient) in its colors grapheme by grapheme, laid out as it would be without.

### Code

```rust
let theme = Theme::of(cx);
SizedBox::new(
    Some(32),
    None,
    Some(
        BoxWidget::new()
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .child(
                Text::new(concat!(
                    "Terminals draw text in a grid of cells. A paragraph breaks ",
                    "its lines between words to fit its width; wide characters, ",
                    "漢字 and 🙂, take two cells each.",
                ))
                .wrap(TextWrap::Word)
                .overflow(TextOverflow::Ellipsis)
                .text_align(TextAlign::Left)
                .max_lines(None),
            )
            .into(),
    ),
)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| wrap | word, grapheme, none | word |
| overflow | Clip, Ellipsis, EllipsisStart, EllipsisMiddle | Ellipsis |
| align | left, center, right, justify | left |
| max lines | all, 3, 1 | all |
| width | 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 32 cells |

## Rich text

Spans in styles of their own: attributes, colors and links, 'text'.bold.fg(…) for short.

- `rtwk::style::Span` (struct) in `crates/rtwk-widgets/src/style/style.rs:297`: Text in one Style, what styled text is made of, its text colors along a Gradient when it has one (Span::gradient) (twk's `Span`).
- `rtwk::text::Text` (struct) in `crates/rtwk-widgets/src/text/widgets.rs:66`: Text in style merged onto the DefaultTextStyle above it: a string (data), or spans, each in its own style merged onto that one (Text::rich), a span with a gradient (Span::gradient) in its colors grapheme by grapheme, laid out as it would be without.

### Code

```rust
fn rich(cx: &mut Cx<'_>, on_link: Handler<std::sync::Arc<str>>) -> Text {
    let theme = Theme::of(cx);
    Text::rich([
        Span::styled("attributes\n", theme.muted.clone()),
        Span::new("bold").bold(),
        Span::new(" "),
        Span::new("dim").dim(),
        Span::new(" "),
        Span::new("italic").italic(),
        Span::new(" "),
        Span::new("reverse").reverse(),
        Span::new(" "),
        Span::new("strike").strike(),
        Span::new(" "),
        Span::new("overline").overline(),
        Span::new("\n\n"),
        Span::styled("underlines\n", theme.muted.clone()),
        Span::new("underline").underline(),
        Span::new(" "),
        Span::new("double").double_underline(),
        Span::new(" "),
        Span::new("curly").curly_underline(),
        Span::new(" "),
        Span::new("dotted").dotted_underline(),
        Span::new(" "),
        Span::new("dashed").dashed_underline(),
        Span::new("\n\n"),
        Span::styled("colors\n", theme.muted.clone()),
        Span::new("red").fg(Color::RED),
        Span::new(" "),
        Span::new("color 166").fg(Color::indexed(166)),
        Span::new(" "),
        Span::new("rgb").fg(Color::rgb(64, 128, 224)),
        Span::new(" "),
        Span::new(" on navy ").fg(Color::WHITE).bg(Color::indexed(17)),
        Span::new(" "),
        Span::styled("red curly", Style
        {
            underline_color: Some(Color::RED),
            ..Style::new().attrs(Attrs::CURLY_UNDERLINE)
        },),
        Span::new("\n\n"),
        Span::styled("links\n", theme.muted.clone()),
        Span::new("a link to rust-lang.org").link("https://www.rust-lang.org").underline(),
    ])
    .on_link_tap(on_link.clone())
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| row labels | false, true | true |

## DefaultTextStyle

The style of the text below it, which a Text merges its own over.

- `rtwk::text::DefaultTextStyle` (struct) in `crates/rtwk-widgets/src/text/widgets.rs:375`: The style the Texts below it are drawn in under their own: style merged onto that of the DefaultTextStyle above it (twk's `DefaultTextStyle`).

### Code

```rust
DefaultTextStyle::new(
    Style::new().attrs(Attrs::BOLD),
    column![
        Text::new("This text has the default style."),
        Text::new("This one merges yellow over it.")
            .style(Style::new().fg(Color::YELLOW)),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start),
)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| style | bold, italic, green, on navy | bold |

## Right to left

Hebrew and Arabic among left-to-right text: put in the order it reads where the app reorders it (TextOrder::Visual), left as stored where the terminal reorders it itself.

- `rtwk::render::BidiText` (struct) in `crates/rtwk-unicode/src/bidi/mod.rs:381`: A text's bidirectional layout by UAX #9: its paragraphs, the embedding level of each character and, for a line of a paragraph, the runs it is drawn in.

### Code

```rust
let visual = true;
let align = TextAlign::Start;
let lines = ["Hello is שלום in Hebrew,",
    "and مرحبا in Arabic.", "שלום עולם! (hello world)"];
Column::new()
    .children(lines.map(|text| {
        let bidi = BidiText::new(text, None);
        let rtl = visual
            && bidi.paragraphs()[0].direction() == TextDirection::Rtl;
        let shown = if visual {
            bidi.visual(0, text.len()).into_owned()
        } else {
            text.to_owned()
        };
        let right = match align {
            TextAlign::Start => rtl,
            TextAlign::End => !rtl,
            _ => align == TextAlign::Right,
        };
        Text::new(format!("\u{200e}\u{202d}{shown}\u{202c}")).text_align(if right {
            TextAlign::Right
        } else {
            TextAlign::Left
        }).into()
    }))
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| text order | visual, logical | visual |
| align | start, end, left, right | start |

## ANSI

Output with ANSI colour codes, from a program or a log, as styled spans: all at once with parse_ansi, or as it streams in with AnsiStream.

- `rtwk::ansi::AnsiStream` (struct) in `crates/rtwk-widgets/src/ansi/stream.rs:104`: Output of a program, such as a compiler or a test runner, that arrives in pieces, read as lines of spans: the lines a terminal would show it in, with nothing in them that could act on the terminal they are drawn on (twk's `AnsiStream`).
- `rtwk::ansi::parse_ansi` (fn) in `crates/rtwk-widgets/src/ansi/stream.rs:29`: The lines of `text`, output of a program with ANSI escape sequences, as AnsiStream reads it, a tab moving to the next multiple of 8 columns (twk's `parseAnsi`; parse_ansi_with for another tab width).

### Code

```rust
//  cargo test 
let output = concat!(
    "\r\u{1b}[K\u{1b}[1;32m   Compiling\u{1b}[0m rtwk-rich v0.1.0\r\u{1b}[K\u{1b}[1;32m    Finished\u{1b}[0m `test` profile in 2.0s\r\u{1b}[Ktest a_gap_row ... \u{1b}[31mFAILED\u{1b}[0m\n",
    "assertion `left == right` failed\n",
    "  left: \"⋯ 20 unchanged lines\"\n",
    " right: \"⋯ 21 unchanged lines\"\n",
    "\n",
    "  \u{1b}[2mat tests/diff.rs:88:7\u{1b}[22m\n",
    "\r\u{1b}[Ktest result: \u{1b}[31mFAILED\u{1b}[0m. 20 passed; 1 failed\n",
);
let lines = parse_ansi(output);
Column::new().children(lines.into_iter().map(|line| {
    Text::rich(line).wrap(TextWrap::None).into()
}))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| output | cargo test, ls, log | cargo test |
| read with | parse_ansi, AnsiStream | parse_ansi |
| piece | 7 chars, 16, 64 | 7 chars |
