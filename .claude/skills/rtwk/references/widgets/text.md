<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Text

[All widgets](../widgets.md)

## Text

A paragraph in one style, its lines broken to fit, cut off and aligned as asked.

API:

- `rtwk::text::Text` (struct, `crates/rtwk-widgets/src/text/widgets.rs:66`) Text in `style` merged onto the `DefaultTextStyle` above it: a string (`data`), or `spans`, each in its own style merged onto that one (`Text::rich`), a span with a gradient (`Span::gradient`) in its colors grapheme by ...
  - public fields: `data: Option<Str>`, `spans: Option<Rc<[Span]>>`, `style: Style`, `wrap: TextWrap`, `overflow: TextOverflow`, `max_lines: Option<usize>`, `text_align: TextAlign`, `on_link_tap: Option<Handler<Arc<str>>>`

### Code

The gallery's Code tab at the initial options (route `#/text/text`, `crates/rtwk-gallery/src/pages/mod.rs:116`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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
                .align(TextAlign::Left)
                .max_lines(None),
            )
            .into(),
    ),
)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| wrap | word, grapheme, none | word |
| overflow | clip, ellipsis, ellipsisStart, ellipsisMiddle | ellipsis |
| align | left, center, right, justify | left |
| lines | all, 3, 1 | all |
| width | 14..50 (a slider) | 32 cells |

### Methods

#### `Text`: constructors and associated functions

- `pub fn new(data: impl Into<Str>) -> Text` - Shows `data`; the text breaks at words, is clipped, shows as many lines as it has and is aligned left unless told otherwise.
- `pub fn rich(spans: impl Into<Rc<[Span]>>) -> Text` - Shows `spans`, each in its style merged onto `style`, as one paragraph: a line breaks between spans as it does inside one.
- `pub fn line(data: impl Into<Str>) -> Text` - Shows `data` on one line, cut off with the ellipsis where it is wider than its room: `Text::new` with `TextWrap::None`, `TextOverflow::Ellipsis` and one line at most.
- `pub fn rich_line(spans: impl Into<Rc<[Span]>>) -> Text` - `spans` as `Text::line` shows a string: `Text::rich` on one line, cut off with the ellipsis.

#### `Text`: builder methods (take and return `self`)

- `pub fn style(mut self, style: Style) -> Text` - In `style`, merged onto the `DefaultTextStyle` above.
- `pub fn wrap(mut self, wrap: TextWrap) -> Text` - Breaking its lines as `wrap` says.
- `pub fn overflow(mut self, overflow: TextOverflow) -> Text` - Showing text cut off as `overflow` says.
- `pub fn max_lines(mut self, max_lines: impl Into<Option<usize>>) -> Text` - Showing at most `max_lines` lines (none for any number).
- `pub fn text_align(mut self, align: TextAlign) -> Text` - Placing each line as `align` says.
- `pub fn on_link_tap(mut self, h: Handler<Arc<str>>) -> Text` - Telling `h` the link of the text clicked on.

#### `Text`: methods

- `pub fn configure(&self, cx: &mut Cx<'_>, o: &mut RenderStyledParagraph) -> Change` - Configures `o` from this text, at `cx`: its spans in their styles merged onto the `DefaultTextStyle`, the theme's ellipsis.

### Also used in the code

- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk::text::TextOverflow` (enum, `crates/rtwk-unicode/src/fit.rs:30`) How a paragraph (or `fit_text`) shows that it cut text off.
  - variants: `Clip`, `Ellipsis`, `EllipsisStart`, `EllipsisMiddle`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`

## Rich text

Spans in styles of their own: attributes, colors and links, 'text'.bold.fg(…) for short.

API:

- `rtwk::style::Span` (struct, `crates/rtwk-widgets/src/style/style.rs:296`) Text in one `Style`, what styled text is made of, its text colors along a `Gradient` when it has one (`Span::gradient`) (twk's `Span`).
  - public fields: `text: Str`, `style: Style`, `gradient: Option<Gradient>`, `on_tap: Option<Handler<()>>`
- `rtwk::text::Text` (struct, `crates/rtwk-widgets/src/text/widgets.rs:66`) Text in `style` merged onto the `DefaultTextStyle` above it: a string (`data`), or `spans`, each in its own style merged onto that one (`Text::rich`), a span with a gradient (`Span::gradient`) in its colors grapheme by ...
  - public fields: `data: Option<Str>`, `spans: Option<Rc<[Span]>>`, `style: Style`, `wrap: TextWrap`, `overflow: TextOverflow`, `max_lines: Option<usize>`, `text_align: TextAlign`, `on_link_tap: Option<Handler<Arc<str>>>`

### Code

The gallery's Code tab at the initial options (route `#/text/rich-text`, `crates/rtwk-gallery/src/pages/mod.rs:438`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
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
    Span::new("a link to dart.dev").link("https://dart.dev").underline(),
])
.on_link_tap(on_link.clone())
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| labels | on, off | true |

### Methods

#### `Span`: constructors and associated functions

- `pub fn new(text: impl Into<Str>) -> Span` - `text`, its style unset.
- `pub fn styled(text: impl Into<Str>, style: Style) -> Span` - `text` in `style` (twk's `Span(text, style)`).
- `pub fn gradient(text: impl Into<Str>, gradient: Gradient) -> Span` - `text`, its text colors along `gradient` (twk's `Span.gradient`).

#### `Span`: builder methods (take and return `self`)

- `pub fn bold(self) -> Span` - This span in bold.
- `pub fn dim(self) -> Span` - This span dimmed.
- `pub fn italic(self) -> Span` - This span in italics.
- `pub fn underline(self) -> Span` - This span underlined with a single line.
- `pub fn double_underline(self) -> Span` - This span underlined with a double line.
- `pub fn curly_underline(self) -> Span` - This span underlined with a curly line.
- `pub fn dotted_underline(self) -> Span` - This span underlined with a dotted line.
- `pub fn dashed_underline(self) -> Span` - This span underlined with a dashed line.
- `pub fn reverse(self) -> Span` - This span in reverse video.
- `pub fn strike(self) -> Span` - This span struck through.
- `pub fn overline(self) -> Span` - This span with a line over it.
- `pub fn fg(self, color: Color) -> Span` - This span in the text color `color`, opaque.
- `pub fn bg(self, color: Color) -> Span` - This span on the background `color`, opaque.
- `pub fn link(self, url: impl Into<Arc<str>>) -> Span` - This span as a hyperlink to `url` (OSC 8).
- `pub fn tap(mut self, on_tap: Handler<()>) -> Span` - This span calling `on_tap` when it is clicked, in place of the handler it had.

#### `Text`: constructors and associated functions

- `pub fn new(data: impl Into<Str>) -> Text` - Shows `data`; the text breaks at words, is clipped, shows as many lines as it has and is aligned left unless told otherwise.
- `pub fn rich(spans: impl Into<Rc<[Span]>>) -> Text` - Shows `spans`, each in its style merged onto `style`, as one paragraph: a line breaks between spans as it does inside one.
- `pub fn line(data: impl Into<Str>) -> Text` - Shows `data` on one line, cut off with the ellipsis where it is wider than its room: `Text::new` with `TextWrap::None`, `TextOverflow::Ellipsis` and one line at most.
- `pub fn rich_line(spans: impl Into<Rc<[Span]>>) -> Text` - `spans` as `Text::line` shows a string: `Text::rich` on one line, cut off with the ellipsis.

#### `Text`: builder methods (take and return `self`)

- `pub fn style(mut self, style: Style) -> Text` - In `style`, merged onto the `DefaultTextStyle` above.
- `pub fn wrap(mut self, wrap: TextWrap) -> Text` - Breaking its lines as `wrap` says.
- `pub fn overflow(mut self, overflow: TextOverflow) -> Text` - Showing text cut off as `overflow` says.
- `pub fn max_lines(mut self, max_lines: impl Into<Option<usize>>) -> Text` - Showing at most `max_lines` lines (none for any number).
- `pub fn text_align(mut self, align: TextAlign) -> Text` - Placing each line as `align` says.
- `pub fn on_link_tap(mut self, h: Handler<Arc<str>>) -> Text` - Telling `h` the link of the text clicked on.

#### `Text`: methods

- `pub fn configure(&self, cx: &mut Cx<'_>, o: &mut RenderStyledParagraph) -> Change` - Configures `o` from this text, at `cx`: its spans in their styles merged onto the `DefaultTextStyle`, the theme's ellipsis.

## DefaultTextStyle

The style of the text below it, which a Text merges its own over.

API:

- `rtwk::text::DefaultTextStyle` (struct, `crates/rtwk-widgets/src/text/widgets.rs:375`) The style the `Text`s below it are drawn in under their own: `style` merged onto that of the `DefaultTextStyle` above it (twk's `DefaultTextStyle`).
  - public fields: `style: Style`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/text/defaulttextstyle`, `crates/rtwk-gallery/src/pages/mod.rs:451`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| style | bold, italic, green, on navy | bold |

### Methods

#### `DefaultTextStyle`: constructors and associated functions

- `pub fn new(style: Style, child: impl Into<AnyWidget>) -> DefaultTextStyle` - Gives the `Text`s in `child`'s subtree `style`, merged onto the one above.
- `pub fn merge(style: Option<Style>, child: impl Into<AnyWidget>) -> DefaultTextStyle` - As `new` (twk's `DefaultTextStyle.merge`): a field `style` leaves unset stays as the style above has it.
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> Style` - The style the `Text`s at `cx` are drawn in under their own: that of the nearest `DefaultTextStyle`, merged onto those above it; an empty style without one.

## Right to left

Hebrew and Arabic among left-to-right text: put in the order it reads where the app reorders it (TextOrder.visual), left as stored where the terminal reorders it itself.

API:

- `rtwk_render::TextOrder` [path inferred] (enum, `crates/rtwk-render/src/tree.rs:531`) The order a tree draws text in, from left to right (`RenderTree::set_text_order`): where it holds right-to-left characters, as they are stored or as they are read.
  - variants: `Logical`, `Visual`

### Code

The gallery's Code tab at the initial options (route `#/text/right-to-left`, `crates/rtwk-gallery/src/pages/mod.rs:464`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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
        Text::new(format!("\u{200e}\u{202d}{shown}\u{202c}")).align(if right {
            TextAlign::Right
        } else {
            TextAlign::Left
        }).into()
    }))
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| order | visual, logical | visual |
| align | start, end, left, right | start |

### Also used in the code

- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`
- `rtwk_core::unicode::BidiText` (struct, `crates/rtwk-unicode/src/bidi/mod.rs:381`) A text's bidirectional layout by UAX #9: its paragraphs, the embedding level of each character and, for a line of a paragraph, the runs it is drawn in.
- `rtwk_core::unicode::TextDirection` (enum, `crates/rtwk-unicode/src/bidi/mod.rs:208`) The direction text runs in.
  - variants: `Ltr`, `Rtl`

## ANSI

Output with ANSI colour codes, from a program or a log, as styled spans: all at once with parseAnsi, or as it streams in with AnsiStream.

API:

- `rtwk::ansi::AnsiStream` (struct, `crates/rtwk-widgets/src/ansi/stream.rs:104`) Output of a program, such as a compiler or a test runner, that arrives in pieces, read as lines of spans: the lines a terminal would show it in, with nothing in them that could act on the terminal they are drawn on ...
  - public fields: `tab_width: usize`

### Code

The gallery's Code tab at the initial options (route `#/text/ansi`, `crates/rtwk-gallery/src/pages/mod.rs:486`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
//  dart test 
let output = concat!(
    "\r\u{1b}[K00:00 \u{1b}[32m+0\u{1b}[0m: loading test/view_test.dart\r\u{1b}[K00:00 \u{1b}[32m+3\u{1b}[0m: test/view_test.dart: lists and their markers\r\u{1b}[K00:01 \u{1b}[32m+9\u{1b}[0m: test/stream_test.dart: a block settled once\r\u{1b}[K00:01 \u{1b}[32m+12\u{1b}[0m \u{1b}[33m~1\u{1b}[0m: test/code_test.dart: tabs\r\u{1b}[K00:02 \u{1b}[32m+16\u{1b}[0m \u{1b}[33m~1\u{1b}[0m: test/diff_test.dart: a gap row\r\u{1b}[K00:02 \u{1b}[32m+16\u{1b}[0m \u{1b}[33m~1\u{1b}[0m \u{1b}[31m-1\u{1b}[0m: test/diff_test.dart: a gap row [E]\n",
    "  Expected: '⋯ 21 unchanged lines'\n",
    "    Actual: '⋯ 20 unchanged lines'\n",
    "     Which: is different.\n",
    "\n",
    "  \u{1b}[2mtest/diff_test.dart 88:7  main.<fn>\u{1b}[22m\n",
    "\r\u{1b}[K00:03 \u{1b}[32m+21\u{1b}[0m \u{1b}[33m~1\u{1b}[0m \u{1b}[31m-1\u{1b}[0m: \u{1b}[31mSome tests failed.\u{1b}[0m\n",
);
let lines = parse_ansi(output);
Column::new().children(lines.into_iter().map(|line| {
    Text::rich(line).wrap(TextWrap::None).into()
}))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| output | test, ls, log | test |
| read | parseAnsi, AnsiStream | parseAnsi |
| piece | 7, 16, 64 | 7 |

### Methods

#### `AnsiStream`: constructors and associated functions

- `pub fn new() -> Self` - An empty stream, whose tabs move to the next multiple of 8 columns.

#### `AnsiStream`: builder methods (take and return `self`)

- `pub fn tab_width(mut self, width: usize) -> Self` - Tabs moving to the next multiple of `width` columns.

#### `AnsiStream`: methods

- `pub fn lines(&mut self) -> &[Vec<Span>]` - The lines so far: those ended, then the one being written, unless no cell of it is written yet.
- `pub fn settled_lines(&self) -> &[Vec<Span>]` - The lines ended so far: the first `settled` of `lines`, which no output added later changes.
- `pub fn settled(&self) -> usize` - How many of `lines`, from the first, are ended: no output added later changes them.
- `pub fn take_settled(&mut self) -> Vec<Vec<Span>>` - Takes the ended lines out of `lines` and returns them, for a reader that keeps them elsewhere, as a log view does: what is held is then only the line being written, and `settled` is 0.
- `pub fn is_closed(&self) -> bool` - Whether `close` was called.
- `pub fn add(&mut self, text: &str)` - Reads `text`.
- `pub fn add_bytes(&mut self, bytes: &[u8])` - Reads `bytes` of UTF-8; malformed bytes are U+FFFD, each maximal part of a character that is not one a U+FFFD, as twk's decoder makes them.
- `pub fn close(&mut self)` - Ends the output: the last line is ended, a UTF-8 character cut off is U+FFFD, and a sequence cut off is dropped.
- `pub fn reset(&mut self)` - Starts over, as a new stream does (twk's `reset`): the lines, the one being written, a character or a sequence cut off, the style and a link are forgotten, the `tab_width` is kept, and a closed stream is open again.

### Also used in the code

- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
