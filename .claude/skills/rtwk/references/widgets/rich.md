<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Rich

[All widgets](../widgets.md)

## MarkdownView

A Markdown document as widgets, a blank row between the blocks; its links the app's with onLinkTap, a MarkdownController scrolling to a heading.

API:

- `rtwk_rich::markdown::MarkdownView` (struct, `crates/rtwk-rich/src/markdown/view.rs:448`) The Markdown document `data` as widgets, its blocks one under another with a blank row between them, in the styles of `theme`, by default `MarkdownTheme::from_theme` the app's theme (the lines are ...
  - public fields: `data: Option<Rc<str>>`, `blocks: Option<Rc<[Block]>>`, `appended: Option<Rc<str>>`, `definitions: Definitions`, `theme: Option<MarkdownTheme>`, `hyperlinks: bool`, `hyperlink_schemes: Option<BTreeSet<String>>`, `controller: Option<MarkdownController>`

### Code

The gallery's Code tab at the initial options (route `#/rich/markdownview`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:147`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::animation::SpinnerFrames;
use rtwk::layout::{CrossAxisAlignment, Expanded, SizedBox};
use rtwk::scroll::{ScrollController, Scrollbar, SingleChildScrollView};
use rtwk::style::{Span, Style, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, column, row};
use rtwk_core::style::Attrs;
use rtwk_rich::markdown::MarkdownView;

pub(crate) const THINK: i64 = 700;
pub(crate) const PIECE: i64 = 40;
pub(crate) const HOLD: i64 = 3_500;
const CHUNK: usize = 5;

/// Uses the inline Markdown view with exact scrollbar metrics.
pub(crate) fn document(source: &'static str, width: i32) -> AnyWidget {
    LayoutBuilder::new(move |cx, room| {
        let theme = Theme::of(cx);
        SizedBox::new(
            Some(width.min(room.max_width)),
            Some(room.max_height.clamp(6, 24)),
            Some(
                Scrollbar::new(SingleChildScrollView::new(MarkdownView::new(source)))
                    .style(theme.border.clone())
                    .into(),
            ),
        )
        .into()
    })
    .into()
}

fn boundary(source: &str, characters: usize) -> usize {
    source
        .char_indices()
        .nth(characters)
        .map_or(source.len(), |(i, _)| i)
}

pub(crate) fn next_frame(ms: i64, source: &str) -> (usize, i64, i64) {
    let pieces = source.chars().count().div_ceil(CHUNK);
    let duration = THINK + PIECE * i64::try_from(pieces).expect("sample length");
    let cycle = duration + HOLD;
    let time = ms.rem_euclid(cycle);
    let base = ms.saturating_sub(time);
    let piece = if time < THINK {
        0
    } else {
        usize::try_from((time - THINK) / PIECE + 1)
            .unwrap_or_default()
            .min(pieces)
    };
    let next = if piece == pieces {
        cycle
    } else {
        THINK + PIECE * i64::try_from(piece).expect("piece")
    };
    (piece, base.saturating_add(next), duration)
}

pub(crate) fn answer(
    question: &'static str,
    source: &'static str,
    width: i32,
    ms: i64,
) -> AnyWidget {
    let (piece, _, duration) = next_frame(ms, source);
    let shown = boundary(source, piece * CHUNK);
    let from = boundary(source, piece.saturating_sub(1) * CHUNK);
    let frames = SpinnerFrames::DOTS.frames;
    let done = shown == source.len();
    let spinner = (ms / 80).rem_euclid(i64::try_from(frames.len()).expect("spinner frame count"));
    let spinner = usize::try_from(spinner).expect("nonnegative frame");
    let spinner = frames[spinner].clone();
    let scroll = ScrollController::with(0, true);
    LayoutBuilder::new(move |cx, room| {
        let theme = Theme::of(cx);
        let heading = Style::new().attrs(Attrs::BOLD);
        let status = if done {
            let tenths = (duration + 50) / 100;
            format!("answered in {}.{} s", tenths / 10, tenths % 10)
        } else if piece == 0 {
            "thinking…".into()
        } else {
            "answering…".into()
        };
        SizedBox::new(
            Some(width.min(room.max_width)),
            Some(room.max_height.clamp(6, 24)),
            Some(
                column![
                    Text::rich(vec![
                        Span::styled("› ", theme.primary.merge(&heading)),
                        Span::styled(question, heading),
                    ]),
                    Expanded::new(
                        1,
                        Scrollbar::new(
                            MarkdownView::new(&source[..shown])
                                .appended(&source[from..shown])
                                .into_viewport()
                                .scroll_controller(scroll.clone()),
                        )
                        .style(theme.border.clone()),
                    ),
                    row![
                        (if done {
                            Text::new("✓")
                        } else {
                            Text::new(spinner.clone())
                        })
                        .style(if done {
                            theme.success.clone()
                        } else {
                            theme.primary.clone()
                        }),
                        Text::new(status).style(theme.muted.clone()),
                    ]
                    .spacing(1),
                ]
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .spacing(1)
                .into(),
            ),
        )
        .into()
    })
    .into()
}

fn preview() -> AnyWidget {
    document(r#"# Release notes

**twk_rich** puts *rich text* in the terminal: Markdown, code and diffs, each a widget.

Jump to [what is new](#what-is-new) or [still to come](#still-to-come).

## What is new

- `MarkdownView` streams an answer in
- `CodeView` highlights ten languages
  - Dart, Rust, Go and Python
  - JSON, YAML and Bash
- `DiffView` marks the words that changed

1. Add the package:
2. Import `package:twk_rich/twk_rich.dart`
3. Show a document

```bash
dart pub add twk_rich
```

```dart
final notes = await File('NOTES.md').readAsString();
runApp(SingleChildScrollView(
  child: MarkdownView(notes),
));
```

> Every block is a widget: this quote, the lists above and the table below.

| widget       | shows       | scrolls |
|:-------------|:------------|:-------:|
| MarkdownView | documents   |   no    |
| CodeView     | source code |   no    |
| DiffView     | changes     |   yes   |

### Still to come

- [x] tables and task lists
- [ ] ~~images~~ pictures in cells

Read the [guide](https://dart.dev/tools) for more.

---"#, 50)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| content | document, answer | document |
| codeWrap | grapheme, word, none | grapheme |
| width | 30..72 (a slider) | 50 cells |
| links | terminal, app, app only | terminal |

### Methods

#### `MarkdownView`: constructors and associated functions

- `pub fn from_snapshot(snapshot: super::MarkdownSnapshot) -> super::ComposedMarkdownView` - Shows a stream snapshot in a composed viewport with bounded width and height.
- `pub fn new(data: impl Into<Rc<str>>) -> MarkdownView` - Shows the document `data`, parsing it again only where text was added at its end.
- `pub fn blocks(blocks: impl Into<Rc<[Block]>>, definitions: Definitions) -> MarkdownView` - `blocks` parsed already, shown as those of a document are, their references resolved by `definitions`: such as the blocks of an answer still live (see `super::MarkdownStream`).

#### `MarkdownView`: builder methods (take and return `self`)

- `pub fn into_viewport(self) -> super::ComposedMarkdownView` - Converts this inline view into a composed document with its own scroll and selection viewport.
- `pub fn appended(mut self, appended: impl Into<Rc<str>>) -> MarkdownView` - The text that was added to the `data` before.
- `pub fn theme(mut self, theme: MarkdownTheme) -> MarkdownView` - In the styles of `theme`.
- `pub fn hyperlinks(mut self, on: bool) -> MarkdownView` - Whether a link and an image are hyperlinks.
- `pub fn hyperlink_schemes(mut self, schemes: BTreeSet<String>) -> MarkdownView` - Writing hyperlinks to the destinations of these schemes only.
- `pub fn controller(mut self, controller: MarkdownController) -> MarkdownView` - Held by `controller`.
- `pub fn code_wrap(self, wrap: TextWrap) -> ConfiguredMarkdownView` - Sets the code block wrapping mode.
- `pub fn tab_width(self, width: usize) -> ConfiguredMarkdownView` - Sets how many cells apart the tab stops are in fenced code.
- `pub fn on_link_tap(self, handler: Handler<MarkdownLinkTap>) -> ConfiguredMarkdownView` - Emits the destination and optional title when the left button is pressed and released on the same link or image cell.

### Also used in the code

- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::scroll::Scrollbar` (struct, `crates/rtwk-scroll/src/scrollbar.rs:145`) Shows how far the outermost `Scrollable` inside its child is scrolled, as a thumb on a track beside it, and scrolls it with the mouse.
  - public fields: `child: AnyWidget`, `axis: Option<Axis>`, `visibility: ScrollbarVisibility`, `time_to_fade: Duration`, `style: Option<Style>`, `slim: bool`, `track_style: Option<Style>`
- `rtwk::scroll::SingleChildScrollView` (struct, `crates/rtwk-scroll/src/single_child_scroll_view.rs:65`) Shows its child in a window that scrolls along `scroll_direction` (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `scroll_direction: Axis`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`, `padding: Option<EdgeInsets>`, `child: Option<AnyWidget>`
- `rtwk::style::SpinnerFrames` (struct, `crates/rtwk-widgets/src/style/glyphs.rs:18`) The frames of a spinner, shown in turn, each for `interval` (twk's `SpinnerFrames`, of `animation/spinner.dart`: defined here, where `Glyphs::spinner` needs it; the animation module's `Spinner` uses this type).
  - public fields: `frames: Cow<'static`, `interval: Duration`
- `rtwk::scroll::ScrollController` (struct, `crates/rtwk-scroll/src/scrollable.rs:703`) Reads and moves the position of the `Scrollable` it is given to: `offset`, `jump_to`, `ensure_visible`.
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).

## CodeView

Source code highlighted for its language, with line numbers, its long lines wrapped or cut off at the width; a CodeViewController scrolls it to a line, highlights lines and finds text.

API:

- `rtwk_rich::highlight::CodeView` (struct, `crates/rtwk-rich/src/highlight/code_view.rs:1024`) `code` in `language`, each token in the style the `SyntaxTheme` at its place (`SyntaxTheme::of`) gives its kind: its lines one under another, and with `line_numbers` each line's number in a gutter before it.
  - public fields: `code: Rc<str>`, `language: &'static Language`, `line_numbers: bool`, `first_line_number: i64`, `tab_width: usize`, `wrap: TextWrap`, `signs: Option<Rc<BTreeMap<i64`, `controller: Option<CodeViewController>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `highlight_style: Option<Style>`, `match_style: Option<Style>`, `current_match_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/rich/codeview`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:207`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::layout::SizedBox;
use rtwk::scroll::{Scrollbar, SingleChildScrollView};
use rtwk::style::Theme;
use rtwk::text::TextWrap;
use rtwk::{AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::unicode::TextSegmenter;
use rtwk_render::objects::text::break_lines;
use rtwk_rich::highlight::{CodeSign, CodeView, CodeViewController, Language};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CodeOptions {
    pub(crate) numbers: bool,
    pub(crate) signs: bool,
    pub(crate) wrap: usize,
    pub(crate) width: i32,
    pub(crate) max_rows: Option<i32>,
    pub(crate) first: i64,
    pub(crate) goto: usize,
    pub(crate) lit: usize,
    pub(crate) find: bool,
}

impl CodeOptions {
    fn wrap(self) -> TextWrap {
        [TextWrap::None, TextWrap::Word, TextWrap::Grapheme][self.wrap]
    }
}

pub(crate) fn code_height(
    source: &str,
    options: CodeOptions,
    segmenter: TextSegmenter,
    width: i32,
    height: i32,
) -> i32 {
    let lines = source.split('\n').count();
    let digits = (options.first - 1 + i64::try_from(lines).expect("line count"))
        .to_string()
        .len();
    let gutter = if options.signs { 2 } else { 0 }
        + if options.numbers {
            i32::try_from(digits + 1).expect("gutter width")
        } else {
            0
        };
    let width = (width.min(options.width) - 1 - gutter).max(1);
    let mut rows = 0;
    for line in source.split('\n') {
        let expanded;
        let line = if line.contains('\t') {
            let mut text = String::new();
            for ch in line.chars() {
                if ch == '\t' {
                    let count = 4 - text.len() % 4;
                    text.extend(std::iter::repeat_n(' ', count));
                } else {
                    text.push(ch);
                }
            }
            expanded = text;
            &expanded
        } else {
            line
        };
        rows += i32::try_from(break_lines(&segmenter, line, width, options.wrap()).len())
            .expect("row count");
    }
    rows.min(height).min(options.max_rows.unwrap_or(i32::MAX))
}

pub(crate) fn code(
    source: &'static str,
    language: &'static Language,
    changed: &'static [(i64, &'static str)],
    query: &'static str,
    options: CodeOptions,
) -> AnyWidget {
    AnyWidget::from(CodePreview {
        source,
        language,
        changed,
        query,
        options,
    })
    .with_key(Key::value((source, options)))
}

#[derive(Debug)]
struct CodePreview {
    source: &'static str,
    language: &'static Language,
    changed: &'static [(i64, &'static str)],
    query: &'static str,
    options: CodeOptions,
}

struct CodeState {
    controller: Option<CodeViewController>,
    wake: Option<(AppBinding, u64)>,
}

impl StatefulWidget for CodePreview {
    type State = CodeState;

    fn create_state(&self) -> CodeState {
        let options = self.options;
        let query = self.query;
        let controller = (options.goto != 0 || options.lit != 0 || options.find).then(|| {
            let ctl = CodeViewController::with_query(if options.find { query } else { "" });
            let past = options.first - 1;
            match options.lit {
                1 => ctl.highlight_lines([past + 14], None),
                2 => ctl.highlight_range(past + 13, past + 16, None),
                _ => {}
            }
            ctl
        });
        CodeState {
            controller,
            wake: None,
        }
    }
}

impl State for CodeState {
    type Widget = CodePreview;
    type Msg = ();

    fn init_state(&mut self, widget: &CodePreview, cx: &mut Cx<'_, Self>) {
        if widget.options.goto != 0
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let id = binding.schedule_frame_callback(cx.map(|_: Duration| ()), None);
            self.wake = Some((binding, id));
        }
    }

    fn update(&mut self, (): (), widget: &CodePreview, _: &mut Cx<'_, Self>) -> Rebuild {
        self.wake = None;
        if let Some(controller) = &self.controller
            && controller.lines_in_view().is_some()
        {
            // Apply the alignment once the CodeView's scrollable has a viewport.
            controller.scroll_to_line(
                widget.options.first + 17,
                (widget.options.goto == 2).then_some(0.5),
            );
        }
        Rebuild::No
    }

    fn build(&self, widget: &CodePreview, _: &mut Cx<'_, Self>) -> AnyWidget {
        let CodePreview {
            source,
            language,
            changed,
            options,
            ..
        } = *widget;
        let controller = self.controller.clone();
        LayoutBuilder::new(move |cx, room| {
            let theme = Theme::of(cx);
            let mut code = CodeView::new(source)
                .language(language)
                .line_numbers(options.numbers)
                .first_line_number(options.first)
                .wrap(options.wrap());
            if options.signs {
                for &(line, sign) in changed {
                    let style = if sign == "+" {
                        &theme.success
                    } else {
                        &theme.warning
                    };
                    code = code.sign(
                        options.first - 1 + line,
                        CodeSign::new(sign).style(style.clone()),
                    );
                }
            }
            let contents: AnyWidget = match &controller {
                Some(ctl) => code.controller(ctl.clone()).into(),
                None => SingleChildScrollView::new(code).into(),
            };
            SizedBox::new(
                Some(options.width.min(room.max_width)),
                Some(code_height(
                    source,
                    options,
                    cx.segmenter(),
                    room.max_width,
                    room.max_height,
                )),
                Some(Scrollbar::new(contents).style(theme.border.clone()).into()),
            )
            .into()
        })
        .into()
    }

    fn dispose(&mut self) {
        if let Some((binding, id)) = self.wake.take() {
            binding.cancel_frame_callback(id);
        }
        if let Some(controller) = self.controller.take() {
            controller.dispose();
        }
    }
}
impl_widget!(stateful CodePreview);

fn preview() -> AnyWidget {
    code(
        r#"import 'package:twk/twk.dart';

/// Counts the presses of the space bar.
final class Counter extends StatefulWidget {
  const Counter({super.key, this.start = 0});

  final int start;

  @override
  State<Counter> createState() => _CounterState();
}

final class _CounterState extends State<Counter> {
  late var _count = widget.start;

  @override
  Widget build(BuildContext context) => KeyBindings(
    {'space': () => setState(() => _count++)},
    child: Text('Pressed $_count times, ${_count.isEven ? 'even' : 'odd'}'),
  );
}"#,
        &rtwk_rich::highlight::DART,
        &[(14, "~"), (18, "+")],
        "count",
        CodeOptions {
    numbers: true,
    signs: false,
    wrap: 1,
    width: 56,
    max_rows: None,
    first: 1,
    goto: 0,
    lit: 0,
    find: false,
},
    )
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| lang | Dart, Python, Rust, Go, JavaScript, TypeScript, JSON, YAML, Bash, diff, plain | Dart |
| numbers | on, off | true |
| signs | on, off | false |
| wrap | none, word, grapheme | word |
| width | 24..80 (a slider) | 56 cells |
| rows | all, 10 | all |
| first | 1, 120 | 1 |
| goto | none, least, middle | none |
| lit | none, 14, 13–16 | none |
| find | on, off | false |

### Methods

#### `CodeView`: constructors and associated functions

- `pub fn new(code: impl Into<Rc<str>>) -> CodeView` - Shows `code` as plain text: no line numbers, counting from 1 when shown, tabs 4 columns apart, lines not wrapped, no column of signs, and as tall as its lines unless a controller, a focus node or autofocus makes it ...

#### `CodeView`: builder methods (take and return `self`)

- `pub fn language(mut self, language: &'static Language) -> CodeView` - In `language`.
- `pub fn line_numbers(mut self, on: bool) -> CodeView` - With a gutter of line numbers.
- `pub fn first_line_number(mut self, n: i64) -> CodeView` - Numbering the first line `n`.
- `pub fn tab_width(mut self, width: usize) -> CodeView` - Tabs `width` columns apart.
- `pub fn wrap(mut self, wrap: TextWrap) -> CodeView` - Breaking lines wider than the view as `wrap` says.
- `pub fn signs(mut self, signs: impl Into<BTreeMap<i64, CodeSign>>) -> CodeView` - With a column of signs: `signs` by line number.
- `pub fn sign(mut self, line: i64, sign: CodeSign) -> CodeView` - With a column of signs and `sign` beside line `line`.
- `pub fn controller(mut self, controller: CodeViewController) -> CodeView` - Controlled by `controller`.
- `pub fn focus_node(mut self, node: FocusNode) -> CodeView` - With `node` for its focus.
- `pub fn autofocus(mut self, on: bool) -> CodeView` - Taking the focus when first shown.
- `pub fn highlight_style(mut self, style: Style) -> CodeView` - Highlighting lines in `style` by default.
- `pub fn match_style(mut self, style: Style) -> CodeView` - Drawing what the query finds in `style`.
- `pub fn current_match_style(mut self, style: Style) -> CodeView` - Drawing the current match in `style`.

#### `CodeView`: methods

- `pub fn scrolls(&self) -> bool` - Whether it scrolls its lines itself (a controller, a focus node or autofocus).

### Also used in the code

- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk_rich::highlight::CodeViewController` (struct, `crates/rtwk-rich/src/highlight/code_view.rs:403`) What is done to the `CodeView` it is given to: scrolled to a line (`scroll_to_line`), with the lines in its window read back (`lines_in_view`); lines highlighted (`highlight_lines`), an error or the line a debugger ...
- `rtwk::binding::AppBinding` (struct, `crates/rtwk-widgets/src/binding/app_binding.rs:327`) The app a widget runs in (twk's `AppBinding`): one per running app, found from a build by `maybe_of`.
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk_rich::highlight::CodeSign` (struct, `crates/rtwk-rich/src/highlight/code_view.rs:49`) A mark in the gutter of a `CodeView` beside a line (`CodeView::signs`): `glyph`, a character or a few (`●`, `E`, `▎`), in `style` merged onto the text's.
  - public fields: `glyph: String`, `style: Style`
- `rtwk::scroll::SingleChildScrollView` (struct, `crates/rtwk-scroll/src/single_child_scroll_view.rs:65`) Shows its child in a window that scrolls along `scroll_direction` (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `scroll_direction: Axis`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`, `padding: Option<EdgeInsets>`, `child: Option<AnyWidget>`
- `rtwk::scroll::Scrollbar` (struct, `crates/rtwk-scroll/src/scrollbar.rs:145`) Shows how far the outermost `Scrollable` inside its child is scrolled, as a thumb on a track beside it, and scrolls it with the mouse.
  - public fields: `child: AnyWidget`, `axis: Option<Axis>`, `visibility: ScrollbarVisibility`, `time_to_fade: Duration`, `style: Option<Style>`, `slim: bool`, `track_style: Option<Style>`
- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`

## SyntaxThemeScope

The styles of the code below it, a style for each kind of token, in place of those derived from the theme: a SyntaxTheme of its own, or the derived one with a few changed.

API:

- `rtwk_rich::highlight::SyntaxThemeScope` (struct, `crates/rtwk-rich/src/highlight/syntax_theme.rs:179`) Sets the `SyntaxTheme` the widgets below it use (`SyntaxTheme::of`): a `CodeView`, a `JsonView`, a `DiffView` and the code blocks of a `MarkdownView` take it.
  - public fields: `theme: SyntaxTheme`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/rich/syntaxthemescope`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:253`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::style::{Style, Theme};
use rtwk::{AnyWidget, BuildCx, StatelessWidget, impl_widget};
use rtwk_core::style::Attrs;
use rtwk_rich::highlight::{CodeView, DART, SyntaxTheme, SyntaxThemeScope};

const SOURCE: &str = r"/// Says hello to the user, twice.
@override
Widget build(BuildContext context) {
  final name = 'user', times = 2;
  return Text('Hello, $name! ' * times);
}
";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SyntaxPreview(pub(crate) usize);

impl StatelessWidget for SyntaxPreview {
    fn build(&self, cx: &mut BuildCx<'_>) -> AnyWidget {
        let code = CodeView::new(SOURCE).language(&DART);
        let syntax = match self.0 {
            0 => {
                let theme = Theme::of(cx);
                let mut syntax =
                    SyntaxTheme::from_theme(&rtwk_rich::style::Theme::from_data(&theme));
                syntax.keyword = theme.error.merge(&Style::new().attrs(Attrs::BOLD));
                syntax.string = theme.warning.clone();
                syntax.comment = theme.muted.clone();
                syntax
            }
            1 => SyntaxTheme {
                keyword: Style::new().attrs(Attrs::BOLD),
                r#type: Style::new().attrs(Attrs::BOLD),
                string: Style::new().attrs(Attrs::ITALIC),
                variable: Style::new().attrs(Attrs::UNDERLINE),
                comment: Style::new().attrs(Attrs::DIM),
                ..SyntaxTheme::new()
            },
            _ => return code.into(),
        };
        SyntaxThemeScope::new(syntax, code).into()
    }
}

impl_widget!(stateless eq SyntaxPreview);

fn preview() -> AnyWidget {
    SyntaxPreview(0).into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| syntax | copyWith, own, none | copyWith |

### Methods

#### `SyntaxThemeScope`: constructors and associated functions

- `pub fn new(theme: SyntaxTheme, child: impl Into<AnyWidget>) -> SyntaxThemeScope` - Sets `theme` for `child` and what is below it.

### Also used in the code

- `rtwk_rich::highlight::CodeView` (struct, `crates/rtwk-rich/src/highlight/code_view.rs:1024`) `code` in `language`, each token in the style the `SyntaxTheme` at its place (`SyntaxTheme::of`) gives its kind: its lines one under another, and with `line_numbers` each line's number in a gutter before it.
  - public fields: `code: Rc<str>`, `language: &'static Language`, `line_numbers: bool`, `first_line_number: i64`, `tab_width: usize`, `wrap: TextWrap`, `signs: Option<Rc<BTreeMap<i64`, `controller: Option<CodeViewController>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `highlight_style: Option<Style>`, `match_style: Option<Style>`, `current_match_style: Option<Style>`
- `rtwk_rich::highlight::SyntaxTheme` (struct, `crates/rtwk-rich/src/highlight/syntax_theme.rs:23`) The style of each `TokenKind`.
  - public fields: `plain: Style`, `keyword: Style`, `function: Style`, `variable: Style`, `constant: Style`, `string: Style`, `escape: Style`, `number: Style`, `comment: Style`, `annotation: Style`, `key: Style`, `punctuation: Style`, `operator: Style`, `inserted: Style`, `deleted: Style`, `heading: Style`

## DiffView

The changes between two texts, unified or side by side, the words that changed marked and both sides highlighted.

API:

- `rtwk_rich::diff::DiffView` (struct, `crates/rtwk-rich/src/diff/view.rs:87`) `diff` in a list of rows that scrolls, laid out as `layout` says.
  - public fields: `diff: Rc<Diff>`, `layout: DiffLayout`, `language: Option<&'static Language>`, `tab_width: usize`, `theme: Option<DiffTheme>`, `controller: Option<ScrollController>`, `focus_node: Option<FocusNode>`, `autofocus: bool`

### Code

The gallery's Code tab at the initial options (route `#/rich/diffview`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:271`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::layout::SizedBox;
use rtwk::scroll::Scrollbar;
use rtwk::style::Theme;
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_rich::diff::{Diff, DiffLayout, DiffView};
use rtwk_rich::highlight::PLAIN;

pub(crate) fn diff(
    before: &str,
    after: &str,
    side_by_side: bool,
    context: usize,
    plain: bool,
) -> AnyWidget {
    let mut diff = DiffView::new(Diff::between(
        before,
        after,
        context,
        Some("bin/greet.dart"),
    ))
    .layout(if side_by_side {
        DiffLayout::SideBySide
    } else {
        DiffLayout::Unified
    });
    if plain {
        diff = diff.language(&PLAIN);
    }
    LayoutBuilder::new(move |cx, room| {
        let theme = Theme::of(cx);
        SizedBox::new(
            Some(room.max_width.min(if side_by_side { 100 } else { 64 })),
            Some(room.max_height.clamp(6, 22)),
            Some(
                Scrollbar::new(diff.clone())
                    .style(theme.border.clone())
                    .into(),
            ),
        )
        .into()
    })
    .into()
}

fn preview() -> AnyWidget {
    diff(r#"import 'dart:io';

/// Greets [name] on the terminal.
String greet(String name) {
  return 'Hello, ' + name + '!';
}

/// The names to greet, one a line.
List<String> readNames(String path) {
  final file = File(path);
  final lines = file.readAsLinesSync();
  return lines.where((l) => l.isNotEmpty).toList();
}

void main(List<String> args) {
  final names = readNames(args.first);
  for (final name in names) {
    print(greet(name));
  }
  print('Greeted ${names.length} names.');
}"#, r#"import 'dart:io';

/// Greets [name] on the terminal, in [language].
String greet(String name, {String language = 'en'}) {
  return '${hello[language]}, $name!';
}

const hello = {'en': 'Hello', 'uk': 'Привіт'};

/// The names to greet, one a line.
List<String> readNames(String path) {
  final file = File(path);
  final lines = file.readAsLinesSync();
  return lines.where((l) => l.isNotEmpty).toList();
}

void main(List<String> args) {
  final names = readNames(args.first);
  final language = args.length > 1 ? args[1] : 'en';
  for (final name in names) {
    print(greet(name, language: language));
  }
}"#, false, 3, false)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| layout | unified, sideBySide | unified |
| context | 3, 1, 0 | 3 |
| live | on, off | false |
| language | path, plain | path |

### Methods

#### `DiffView`: constructors and associated functions

- `pub fn new(diff: impl Into<Rc<Diff>>) -> DiffView` - Shows `diff` unified, its code in the language of each file's path, its tabs 4 columns apart, in the styles of the app's theme.

#### `DiffView`: builder methods (take and return `self`)

- `pub fn layout(mut self, layout: DiffLayout) -> DiffView` - In `layout`.
- `pub fn language(mut self, language: &'static Language) -> DiffView` - The code of every file in `language`.
- `pub fn tab_width(mut self, width: usize) -> DiffView` - Tabs `width` columns apart (at least 1).
- `pub fn theme(mut self, theme: DiffTheme) -> DiffView` - In the styles of `theme`.
- `pub fn controller(mut self, controller: ScrollController) -> DiffView` - Scrolled by `controller`.
- `pub fn focus_node(mut self, node: FocusNode) -> DiffView` - With `node` for its focus.
- `pub fn autofocus(mut self, on: bool) -> DiffView` - Taking the focus when first shown.

### Also used in the code

- `rtwk_rich::diff::Diff` (struct, `crates/rtwk-rich/src/diff/model.rs:144`) The changes to one file or more.
  - public fields: `files: Vec<FileDiff>`
- `rtwk_rich::diff::DiffLayout` (enum, `crates/rtwk-rich/src/diff/view.rs:27`) How a `DiffView` puts the two sides of a diff.
  - variants: `Unified`, `SideBySide`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::scroll::Scrollbar` (struct, `crates/rtwk-scroll/src/scrollbar.rs:145`) Shows how far the outermost `Scrollable` inside its child is scrolled, as a thumb on a track beside it, and scrolls it with the mouse.
  - public fields: `child: AnyWidget`, `axis: Option<Axis>`, `visibility: ScrollbarVisibility`, `time_to_fade: Duration`, `style: Option<Style>`, `slim: bool`, `track_style: Option<Style>`

## JsonView

JSON or a Dart value as a tree that folds, each kind of value in its color: c copies the path of the value selected, y the value, / filters the keys.

API:

- `rtwk_rich::json::JsonView` (struct, `crates/rtwk-rich/src/json/view.rs:351`) A JSON document or value as a tree: each object and array a row that folds, its entries below it, indented and joined to it by guides, and a row that closes it; each value in the style of its kind in `theme`, keys, ...
  - public fields: `controller: Option<JsonViewController>`, `root_name: String`, `on_path_copied: Option<Rc<dyn Fn(&str)>>`, `theme: Option<SyntaxTheme>`, `sort_keys: bool`, `max_string_length: Option<usize>`, `focus_node: Option<FocusNode>`, `autofocus: bool`

### Code

The gallery's Code tab at the initial options (route `#/rich/jsonview`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:319`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::layout::{CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Span, Theme, ThemeData};
use rtwk::text::Text;
use rtwk::{
    AnyWidget, Cx, Key, LayoutBuilder, Rebuild, Sender, State, StatefulWidget, column, impl_widget,
};
use rtwk_rich::json::{JsonView, JsonViewController};

const PACKAGE: &str = r#"{
  "name":"twk", "version":"0.1.0",
  "description":"Widgets for terminal apps, laid out as Flutter lays them",
  "private":true, "score":0.97, "homepage":null,
  "environment":{"sdk":"^3.13.5"},
  "dependencies":{"args":"^2.7.0","meta":"^1.16.0","path":"^1.9.1"},
  "scripts":["quick","handover","bench"]
}"#;

type StatusLine = fn(Option<&str>, u64, &ThemeData) -> AnyWidget;

#[derive(Debug)]
pub(crate) struct JsonPreview {
    pub(crate) depth: usize,
    pub(crate) sort: bool,
    pub(crate) cut: usize,
    pub(crate) status: Option<StatusLine>,
}

pub(crate) struct JsonState {
    view: JsonView,
    controllers: [JsonViewController; 3],
    sender: Option<Sender<String>>,
    copied: Option<String>,
    serial: u64,
}

impl StatefulWidget for JsonPreview {
    type State = JsonState;

    fn create_state(&self) -> JsonState {
        JsonState {
            view: JsonView::parse(PACKAGE),
            controllers: std::array::from_fn(JsonViewController::with_expanded_depth),
            sender: None,
            copied: None,
            serial: 0,
        }
    }
}

impl State for JsonState {
    type Widget = JsonPreview;
    type Msg = String;

    fn init_state(&mut self, _: &JsonPreview, cx: &mut Cx<'_, Self>) {
        self.sender = Some(cx.channel());
    }

    fn update(&mut self, path: String, _: &JsonPreview, _: &mut Cx<'_, Self>) -> Rebuild {
        self.copied = Some(path);
        self.serial += 1;
        Rebuild::Yes
    }

    fn build(&self, widget: &JsonPreview, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let sender = self
            .sender
            .as_ref()
            .expect("initialized copy channel")
            .clone();
        let view = self
            .view
            .clone()
            .controller(self.controllers[widget.depth].clone())
            .sort_keys(widget.sort)
            .max_string_length(Some(widget.cut))
            .on_path_copied(move |path| {
                let _ = sender.send(path.to_owned());
            });
        let view = AnyWidget::from(view).with_key(Key::value(widget.depth));
        let status =
            widget.status.unwrap_or(plain_status)(self.copied.as_deref(), self.serial, &theme);
        LayoutBuilder::new(move |_, room| {
            column![
                SizedBox::new(
                    Some(50),
                    Some((room.max_height - 2).clamp(2, 14)),
                    Some(view.clone())
                ),
                status.clone(),
            ]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1)
            .into()
        })
        .into()
    }

    fn dispose(&mut self) {
        for controller in &self.controllers {
            controller.dispose();
        }
    }
}

impl_widget!(stateful JsonPreview);

fn plain_status(path: Option<&str>, _: u64, theme: &ThemeData) -> AnyWidget {
    Text::rich(vec![
        Span::styled("copied: ", theme.muted.clone()),
        match path {
            Some(path) => Span::new(path.to_owned()),
            None => Span::styled("nothing", theme.muted.clone()),
        },
    ])
    .into()
}

fn preview() -> AnyWidget {
    JsonPreview { depth: 1, sort: false, cut: 200, status: None }.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| depth | 1, 2, 0 | 1 |
| sort | on, off | false |
| cut | 200, 24 | 200 |

### Methods

#### `JsonView`: constructors and associated functions

- `pub fn new(value: impl Into<Rc<JsonValue>>) -> JsonView` - Shows `value` as a tree.
- `pub fn parse(source: impl Into<Rc<str>>) -> JsonView` - Shows the JSON value `source` is the text of, or where it is not one the error it fails with, and the line and column where.

#### `JsonView`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: JsonViewController) -> JsonView` - With `controller` for what is expanded and selected.
- `pub fn root_name(mut self, name: impl Into<String>) -> JsonView` - With `name` for the first segment of the paths.
- `pub fn on_path_copied(mut self, f: impl Fn(&str) + 'static) -> JsonView` - Calling `f` with the path `c` copied.
- `pub fn theme(mut self, theme: SyntaxTheme) -> JsonView` - In the styles of `theme`.
- `pub fn sort_keys(mut self, on: bool) -> JsonView` - Showing the keys of objects in order.
- `pub fn max_string_length(mut self, max: Option<usize>) -> JsonView` - Cutting strings longer than `max` characters; none for never.
- `pub fn focus_node(mut self, node: FocusNode) -> JsonView` - With `node` for its focus.
- `pub fn autofocus(mut self, on: bool) -> JsonView` - Taking the focus when first shown.

### Also used in the code

- `rtwk_rich::json::JsonViewController` (struct, `crates/rtwk-rich/src/json/view.rs:78`) What a `JsonView` shows: the objects and arrays expanded, and the value selected, both by path (`json_path`); it notifies its listeners of each change.
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

## BigText

Text in letters several cells tall, drawn with full or half blocks or with box drawing, in a font of its own.

API:

- `rtwk_rich::bigtext::BigText` (struct, `crates/rtwk-rich/src/bigtext/widget.rs:45`) `text` in big letters: each line of it set in `font`, or in the first of `fonts` it fits in, the last one when it fits in none; lines are a blank row apart, each placed as `align` says.
  - public fields: `text: String`, `fonts: Vec<&'static BigFont>`, `align: TextAlign`, `style: Style`, `missing: MissingGlyph`

### Code

The gallery's Code tab at the initial options (route `#/rich/bigtext`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:349`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use std::rc::Rc;

use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::FocusNode;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Border, BorderType, BoxWidget, Flexible, MainAxisSize, SizedBox};
use rtwk::style::{Style, Theme, ThemeData};
use rtwk::text::TextAlign;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget};
use rtwk_core::unicode::TextSegmenter;
use rtwk_rich::bigtext::{BigFont, BigText, BigTextLayout};

const FONTS: [&[&BigFont]; 5] = [
    &BigFont::SIZES,
    &[&BigFont::LARGE],
    &[&BigFont::BLOCK],
    &[&BigFont::THIN],
    &[&BigFont::SMALL],
];
const ALIGN: [TextAlign; 3] = [TextAlign::Left, TextAlign::Center, TextAlign::Right];

pub(crate) fn clock_text(ms: i64) -> String {
    let second = 9 * 3_600 + 41 * 60 + ms / 1_000;
    format!(
        "{:02}:{:02}:{:02}",
        (second / 3_600).rem_euclid(24),
        (second / 60).rem_euclid(60),
        second.rem_euclid(60)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BigPreview {
    pub(crate) font: usize,
    pub(crate) text: usize,
    pub(crate) align: usize,
    pub(crate) color: usize,
    pub(crate) time: i64,
}

impl BigPreview {
    fn style(self, theme: &ThemeData) -> Style {
        [
            &theme.primary,
            &theme.accent,
            &theme.success,
            &theme.warning,
            &theme.info,
        ][self.color]
            .clone()
    }
}

pub(crate) struct BigState {
    field: TextEditingController,
    focus: Rc<FocusNode>,
}

impl StatefulWidget for BigPreview {
    type State = BigState;
    fn create_state(&self) -> BigState {
        BigState {
            field: TextEditingController::new("Hello"),
            focus: Rc::new(FocusNode::new()),
        }
    }
}

impl State for BigState {
    type Widget = BigPreview;
    type Msg = ();

    fn update(&mut self, (): (), _: &BigPreview, _: &mut Cx<'_, Self>) -> Rebuild {
        Rebuild::Yes
    }

    fn build(&self, widget: &BigPreview, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let text = match widget.text {
            1 => clock_text(widget.time),
            2 => "twk\ngallery".to_owned(),
            _ => self.field.text().to_string(),
        };
        let display = BigText::new(text)
            .fonts(FONTS[widget.font])
            .align(ALIGN[widget.align])
            .style(widget.style(&theme));
        if widget.text != 0 {
            return display.into();
        }
        let node = self.focus.clone();
        let field = ListenableBuilder::new(node.clone(), move |cx, field| {
            let theme = Theme::of(cx);
            let border = if node.has_focus() {
                &theme.primary
            } else {
                &theme.border
            };
            BoxWidget::new()
                .border(Border::all(border.clone()).kind(BorderType::Rounded))
                .child(field.expect("field").clone())
                .into()
        })
        .child(
            TextField::new()
                .controller(self.field.clone())
                .focus_node(self.focus.as_ref().clone())
                .placeholder("type here")
                .max_length(24)
                .on_changed(cx.map(|_: Rc<str>| ())),
        );
        column![
            Flexible::new(1, display),
            SizedBox::new(Some(30), None, Some(field.into())),
        ]
        .main_axis_size(MainAxisSize::Min)
        .spacing(1)
        .into()
    }

    fn dispose(&mut self) {
        self.field.dispose();
    }
}
impl_widget!(stateful eq BigPreview);

pub(crate) fn big_height(font: usize, text: usize, width: i32, height: i32) -> i32 {
    let field = if text == 0 { 4 } else { 0 };
    let sample = match text {
        1 => "09:41:00",
        2 => "twk\ngallery",
        _ => "Hello",
    };
    let mut layout = BigTextLayout::new(sample);
    layout.set_fonts(FONTS[font]);
    let (_, _, rows) = layout.layout(
        &TextSegmenter::default(),
        0,
        width.max(0) as usize,
        0,
        (height - field).max(0) as usize,
    );
    i32::try_from(rows).expect("sample height") + field
}

fn preview() -> AnyWidget {
    BigPreview { font: 0, text: 0, align: 0, color: 0, time: 2_000 }.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| font | fit, large, block, thin, small | fit |
| text | typed, clock, lines | typed |
| align | left, center, right | left |
| color | primary, accent, success, warning, info | primary |

### Methods

#### `BigText`: constructors and associated functions

- `pub fn new(text: impl Into<String>) -> BigText` - Draws `text` in big letters, fitting it to the fonts of `BigFont::SIZES`, left-aligned, a missing glyph a box.

#### `BigText`: builder methods (take and return `self`)

- `pub fn font(mut self, font: &'static BigFont) -> BigText` - Draws in `font` whatever the room (twk's `font`, which takes the place of `fonts`).
- `pub fn fonts(mut self, fonts: &[&'static BigFont]) -> BigText` - Fits the text to `fonts`, tried in order; not empty.
- `pub fn align(mut self, align: TextAlign) -> BigText` - Places each line across the width as `align` says.
- `pub fn style(mut self, style: Style) -> BigText` - Merges `style` onto the default text style.
- `pub fn missing(mut self, missing: MissingGlyph) -> BigText` - Draws a character no glyph stands for as `missing` says.

### Also used in the code

- `rtwk_rich::bigtext::BigFont` (struct, `crates/rtwk-rich/src/bigtext/mod.rs:38`) A font of letters several cells tall: a pixel font drawn with full or half blocks, or letters of box drawing.
  - public fields: `height: usize`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`
- `rtwk::editing::TextEditingController` (struct, `crates/rtwk-editing/src/text_editing.rs:418`) The `TextEditingValue` a field edits, which notifies its listeners of each change.
- `rtwk::focus::FocusNode` (struct, `crates/rtwk-framework/src/focus/node.rs:153`) A place in a tree's focus tree: the node of a `Focus` widget, which can have the focus and gets the keys (twk's `FocusNode`).
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::layout::Flexible` (struct, `crates/rtwk-widgets/src/layout/flex.rs:243`) Gives its child, in a `Flex`, a `Row` or a `Column`, a share of the free room (twk's `Flexible`): `flex` against the other flexible children's, all of it (`FlexFit::Tight`) or at most it (`FlexFit::Loose`).
  - public fields: `flex: i32`, `fit: FlexFit`, `child: AnyWidget`
- `rtwk_rich::bigtext::BigTextLayout` (struct, `crates/rtwk-rich/src/bigtext/mod.rs:280`) What twk's `RenderBigText` lays out, without painting: `text` set in the first of its fonts it fits in at the largest size the room allows, the last when it fits in none.
- `rtwk_core::unicode::TextSegmenter` (struct, `crates/rtwk-unicode/src/segment.rs:253`) Segments text into terminal cells and computes their widths.
  - public fields: `mode: TextMode`

## QrCode

Text as a QR code a phone's camera reads: in half blocks, two modules to a cell, black on white whatever the theme, with as much of the quiet zone around it as the rows allow.

API:

- `rtwk_rich::qr::QrCode` (struct, `crates/rtwk-rich/src/qr/code.rs:75`) What twk's `QrCode` widget lays out and paints: a symbol, or else text encoded at a level of error correction (`QrSymbol::encode`; data that does not fit in version 40 at that level is the `QrError` its layout returns), ...

### Code

The gallery's Code tab at the initial options (route `#/rich/qrcode`, `crates/rtwk-gallery/src/pages/rich_catalog.rs:386`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use std::sync::{Arc, OnceLock};

use rtwk::layout::{CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Span, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, row};
use rtwk_rich::qr::{BLACK, QrCode, QrCodeWidget, QrErrorCorrection, QrSymbol, WHITE};

const PAYLOADS: [&str; 3] = [
    "https://pub.dev/packages/twk",
    "WIFI:S:twk;T:WPA;P:secret;;",
    "20261019",
];
static SYMBOLS: [OnceLock<Arc<QrSymbol>>; 12] = [const { OnceLock::new() }; 12];

fn symbol(text: usize, level: usize) -> Arc<QrSymbol> {
    SYMBOLS[text * 4 + level]
        .get_or_init(|| {
            Arc::new(
                QrSymbol::encode(PAYLOADS[text], QrErrorCorrection::ALL[level])
                    .expect("valid example payload"),
            )
        })
        .clone()
}

fn rows_for(symbol: &QrSymbol, quiet: usize, wide: bool, available: usize) -> (usize, usize) {
    let size = symbol.size();
    let factor = if wide { 1 } else { 2 };
    let quiet = (available.saturating_mul(factor).saturating_sub(size) / 2).min(quiet);
    ((size + 2 * quiet).div_ceil(factor), quiet)
}

pub(crate) fn qr_height(text: usize, level: usize, quiet: usize, available: i32) -> i32 {
    let (rows, _) = rows_for(&symbol(text, level), quiet, false, available as usize);
    i32::try_from(rows.max(3)).expect("QR row count")
}

pub(crate) fn qr(text: usize, level: usize, quiet: usize, invert: bool) -> AnyWidget {
    let symbol = symbol(text, level);
    LayoutBuilder::new(move |cx, room| {
        let theme = Theme::of(cx);
        let wide = QrCode::is_wide(&cx.segmenter());
        let available = usize::try_from(room.max_height).unwrap_or_default();
        let (_, quiet) = rows_for(&symbol, quiet, wide, available);
        let (least, _) = rows_for(&symbol, 0, wide, available);
        let code = QrCodeWidget::from_symbol(symbol.clone()).quiet_zone(quiet);
        let code = if invert {
            code.dark(WHITE).light(BLACK)
        } else {
            code
        };
        let mut detail = vec![
            Span::styled("version ", theme.muted.clone()),
            Span::new(format!("{}\n", symbol.version())),
            Span::styled("mask ", theme.muted.clone()),
            Span::new(format!("{}\n", symbol.mask())),
            Span::styled("mode ", theme.muted.clone()),
            Span::new(symbol.mode().name()),
        ];
        if least > available {
            detail.push(Span::styled("\n\nneeds ", theme.warning.clone()));
            detail.push(Span::new(format!("{least} rows")));
        }
        row![code, Text::rich(detail)]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(2)
            .into()
    })
    .into()
}

fn preview() -> AnyWidget {
    qr(2, 1, 4, false)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| text | link, wifi, digits | digits |
| level | low, medium, quartile, high | medium |
| quiet | 0..4 (a slider) | 4 modules |
| invert | on, off | false |

### Methods

#### `QrCode`: constructors and associated functions

- `pub fn new(data: impl Into<String>, level: QrErrorCorrection) -> QrCode` - `data` encoded at `level`, with a quiet zone of 4 modules, black on white.
- `pub fn from_symbol(symbol: impl Into<Arc<QrSymbol>>) -> QrCode` - `symbol` as a QR code, with a quiet zone of 4 modules, black on white.
- `pub fn is_wide(seg: &TextSegmenter) -> bool` - Whether the canvas `seg` measures draws half blocks two columns wide.

#### `QrCode`: methods

- `pub fn data(&self) -> Option<&str>` - The text encoded where there is no symbol made already.
- `pub fn level(&self) -> QrErrorCorrection` - The error correction the text is encoded with (medium where the code has a symbol).
- `pub fn symbol(&self) -> Option<&Arc<QrSymbol>>` - The symbol drawn in place of text encoded.
- `pub fn set_data(&mut self, data: &str, level: QrErrorCorrection) -> bool` - Draws `data` encoded at `level`.
- `pub fn set_symbol(&mut self, symbol: &Arc<QrSymbol>) -> bool` - Draws `symbol` in place of text encoded.
- `pub fn quiet_zone(&self) -> usize` - The light modules around the symbol on each side.
- `pub fn set_quiet_zone(&mut self, quiet_zone: usize) -> bool` - Sets the quiet zone; whether that changes what is laid out.
- `pub fn dark(&self) -> Color` - The color of the dark modules.
- `pub fn set_dark(&mut self, dark: Color) -> bool` - Sets the color of the dark modules; whether that is to be painted again.
- `pub fn light(&self) -> Color` - The color of the light modules and the quiet zone.
- `pub fn set_light(&mut self, light: Color) -> bool` - Sets the color of the light modules and the quiet zone; whether that is to be painted again.
- `pub fn shown(&mut self) -> Result<&QrSymbol, QrError>` - The symbol drawn: the one given, or the text encoded at its level (once, until the text or level changes).
- `pub fn side(&mut self) -> Result<usize, QrError>` - The modules of a side with the quiet zone.
- `pub fn columns(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The columns the code takes on a canvas `seg` measures.
- `pub fn lines(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The rows the code takes on a canvas `seg` measures.
- `pub fn layout(&mut self, seg: &TextSegmenter, min_w: usize, max_w: usize, min_h: usize, max_h: usize) -> Result<(usize, usize), QrError>` - The size, width and height in cells, within the constraints `min_w..=max_w` and `min_h..=max_h`.
- `pub fn min_intrinsic_width(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The least width, which is the most too: the columns of the code.
- `pub fn max_intrinsic_width(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The most width: the columns of the code.
- `pub fn min_intrinsic_height(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The least height, which is the most too: the rows of the code.
- `pub fn max_intrinsic_height(&mut self, seg: &TextSegmenter) -> Result<usize, QrError>` - The most height: the rows of the code.
- `pub fn paint(&mut self, seg: &TextSegmenter, width: usize, height: usize, f: impl FnMut(QrRun<'_>)) -> Result<(), QrError>` - Paints the code on a canvas `seg` measures of `width` x `height` cells with its top left corner at the origin: `f` is given the runs of text, row by row, in half blocks (a row of the code a run, dark on light) or, where ...

### Also used in the code

- `rtwk_rich::qr::QrSymbol` (struct, `crates/rtwk-rich/src/qr/symbol.rs:217`) Text as a QR code, ISO/IEC 18004 (Model 2): a square of `size` x `size` modules, each dark or light (`is_dark`), which a scanner reads with a quiet zone of light modules around it, 4 wide by the standard.
- `rtwk_rich::qr::QrErrorCorrection` (enum, `crates/rtwk-rich/src/qr/symbol.rs:18`) How much of a QR code may be lost and the code still read: the four levels of error correction, each restoring about the share of its codewords it names, at the cost of room for data (twk's `QrErrorCorrection`).
  - variants: `Low`, `Medium`, `Quartile`, `High`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk_rich::qr::QrCodeWidget` (struct, `crates/rtwk-rich/src/qr/widget.rs:26`) A terminal QR widget, drawn in black on white with a four-module quiet zone by default.
