<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Rich

Markdown, code, diffs, JSON, big text, QR codes and program output with its colors.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## MarkdownView

A Markdown document as widgets, a blank row between the blocks; its links the app's with on_link_tap, a MarkdownController scrolling to a heading.

- `rtwk_rich::markdown::MarkdownView` (struct) in `crates/rtwk-rich/src/markdown/view.rs:441`: The Markdown document `data` as widgets, its blocks one under another with a blank row between them, in the styles of theme, by default `MarkdownTheme::from_theme` the app's theme (the lines are super::markdown_blocks_lines's: see there for how each block looks).

### Code

````rust
use rtwk::animation::SpinnerFrames;
use rtwk::layout::{CrossAxisAlignment, Expanded, SizedBox};
use rtwk::scroll::{ScrollController, Scrollbar, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, column, row};
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
                            SingleChildScrollView::new(
                                MarkdownView::new(&source[..shown]).appended(&source[from..shown]),
                            )
                            .controller(scroll.clone()),
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

**rtwk-rich** puts *rich text* in the terminal: Markdown, code and diffs, each a widget.

Jump to [what is new](#what-is-new) or [still to come](#still-to-come).

## What is new

- `MarkdownView` streams an answer in
- `CodeView` highlights ten languages
  - Rust, Go and Python
  - JSON, YAML and Bash
- `DiffView` marks the words that changed

1. Add the crate:
2. Import `rtwk_rich::*`
3. Show a document

```bash
cargo add rtwk-rich
```

```rust
let notes = std::fs::read_to_string("NOTES.md")?;
let view = MarkdownView::new(notes);
let page = ScrollView::new(view);
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

Read the [guide](https://docs.rs/rtwk-rich) for more.

---"#, 50)
}

````

### Options

| option | values | starts at |
| --- | --- | --- |
| content | document, chat answer | document |
| code wrap | grapheme, word, none | grapheme |
| width | 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells, 57 cells, 58 cells, 59 cells, 60 cells, 61 cells, 62 cells, 63 cells, 64 cells, 65 cells, 66 cells, 67 cells, 68 cells, 69 cells, 70 cells, 71 cells, 72 cells | 50 cells |
| links go to | the terminal, the app, the app only | the terminal |

## CodeView

Source code highlighted for its language, with line numbers, its long lines wrapped or cut off at the width; a CodeViewController scrolls it to a line, highlights lines and finds text.

- `rtwk_rich::highlight::CodeView` (struct) in `crates/rtwk-rich/src/highlight/code_view.rs:1113`: `code` in `language`, each token in the style the SyntaxTheme at its place (SyntaxTheme::of) gives its kind: its lines one under another, and with line_numbers each line's number in a gutter before it.

### Code

```rust
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::layout::SizedBox;
use rtwk::render::{TextSegmenter, break_lines};
use rtwk::scroll::{Scrollbar, SingleChildScrollView};
use rtwk::style::Theme;
use rtwk::text::TextWrap;
use rtwk::{AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
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
        r#"use std::collections::HashMap;

/// The words of a text by how often they come, the most first.
fn word_counts(text: &str) -> Vec<(&str, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    let mut pairs: Vec<_> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    pairs
}

fn main() {
    let text = "the cat and the hat and the bat";
    for (word, n) in word_counts(text).iter().take(3) {
        println!("{word:>5} {n}");
    }
}"#,
        &rtwk_rich::highlight::RUST,
        &[(10, "~")],
        "word",
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

| option | values | starts at |
| --- | --- | --- |
| language | Python, Rust, Go, JavaScript, TypeScript, JSON, YAML, Bash, diff, plain | Rust |
| line numbers | false, true | true |
| changes | false, true | false |
| wrap | none, word, grapheme | word |
| width | 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells, 57 cells, 58 cells, 59 cells, 60 cells, 61 cells, 62 cells, 63 cells, 64 cells, 65 cells, 66 cells, 67 cells, 68 cells, 69 cells, 70 cells, 71 cells, 72 cells, 73 cells, 74 cells, 75 cells, 76 cells, 77 cells, 78 cells, 79 cells, 80 cells | 56 cells |
| rows | all, 10 | all |
| first line | 1, 120 | 1 |
| to line 18 | none, least, ½ | none |
| highlight | none, 14, 13–16 | none |
| find | false, true | false |

## SyntaxThemeScope

The styles of the code below it, a style for each kind of token, in place of those derived from the theme: a SyntaxTheme of its own, or the derived one with a few changed.

- `rtwk_rich::highlight::SyntaxThemeScope` (struct) in `crates/rtwk-rich/src/highlight/syntax_theme.rs:179`: Sets the SyntaxTheme the widgets below it use (SyntaxTheme::of): a `CodeView`, a `JsonView`, a `DiffView` and the code blocks of a `MarkdownView` take it.

### Code

```rust
use rtwk::style::{Attrs, Style, Theme};
use rtwk::{AnyWidget, BuildCx, StatelessWidget, impl_widget};
use rtwk_rich::highlight::{CodeView, RUST, SyntaxTheme, SyntaxThemeScope};

const SOURCE: &str = r#"/// Says hello to the user, twice.
#[must_use]
fn hello() -> String {
    let name = "user";
    format!("Hello, {name}! ").repeat(2)
}
"#;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SyntaxPreview(pub(crate) usize);

impl StatelessWidget for SyntaxPreview {
    fn build(&self, cx: &mut BuildCx<'_>) -> AnyWidget {
        let code = CodeView::new(SOURCE).language(&RUST);
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

| option | values | starts at |
| --- | --- | --- |
| syntax theme | copy_with, own, none | copy_with |

## DiffView

The changes between two texts, unified or side by side, the words that changed marked and both sides highlighted; its lines are selected and copied without their numbers.

- `rtwk_rich::diff::DiffView` (struct) in `crates/rtwk-rich/src/diff/view.rs:98`: `diff` in a list of rows that scrolls, laid out as layout says.

### Code

```rust
use rtwk::gestures::SelectionArea;
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
    let mut diff = DiffView::new(Diff::between(before, after, context, Some("src/greet.rs")))
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
                // drag across the lines to select their code
                Scrollbar::new(SelectionArea::new(diff.clone()))
                    .style(theme.border.clone())
                    .into(),
            ),
        )
        .into()
    })
    .into()
}

fn preview() -> AnyWidget {
    diff(r#"use std::fs;

/// Greets `name` on the terminal.
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

/// The names to greet, one a line.
fn read_names(path: &str) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    let lines = text.lines();
    lines.filter(|l| !l.is_empty()).map(String::from).collect()
}

fn main() {
    let names = read_names(&std::env::args().nth(1).unwrap());
    for name in &names {
        println!("{}", greet(name));
    }
    println!("Greeted {} names.", names.len());
}"#, r#"use std::fs;

/// Greets `name` on the terminal, in `language`.
fn greet(name: &str, language: &str) -> String {
    format!("{}, {name}!", GREETINGS.iter().find(|g| g.0 == language).map_or("Hello", |g| g.1))
}

const GREETINGS: [(&str, &str); 2] = [("en", "Hello"), ("uk", "Привіт")];

/// The names to greet, one a line.
fn read_names(path: &str) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    let lines = text.lines();
    lines.filter(|l| !l.is_empty()).map(String::from).collect()
}

fn main() {
    let names = read_names(&std::env::args().nth(1).unwrap());
    let language = std::env::args().nth(2).unwrap_or_else(|| "en".into());
    for name in &names {
        println!("{}", greet(name, &language));
    }
}"#, false, 3, false)
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| layout | Unified, SideBySide | Unified |
| context | 3 lines, 1, 0 | 3 lines |
| edit live | false, true | false |
| language | the path's, plain | the path's |

## JsonView

JSON, or a JsonValue built in code, as a tree that folds, each kind of value in its color: c copies the path of the value selected, y the value, / filters the keys.

- `rtwk_rich::json::JsonView` (struct) in `crates/rtwk-rich/src/json/view.rs:351`: A JSON document or value as a tree: each object and array a row that folds, its entries below it, indented and joined to it by guides, and a row that closes it; each value in the style of its kind in theme, keys, strings, numbers, `true`, `false` and `null` and brackets apart.

### Code

```rust
use rtwk::layout::{CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Span, Theme, ThemeData};
use rtwk::text::Text;
use rtwk::{
    AnyWidget, Cx, Key, LayoutBuilder, Rebuild, Sender, State, StatefulWidget, column, impl_widget,
};
use rtwk_rich::json::{JsonView, JsonViewController};

const PACKAGE: &str = r#"{
  "name":"rtwk", "version":"0.1.0",
  "description":"Widgets for terminal apps, built in Rust",
  "private":true, "score":0.97, "homepage":null,
  "toolchain":{"rust":"1.85"},
  "dependencies":{"clap":"^4.5.0","serde":"^1.0.0","anyhow":"^1.0.0"},
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

| option | values | starts at |
| --- | --- | --- |
| expanded depth | 1, 2, 0 | 1 |
| sort keys | false, true | false |
| strings cut at | 200, 24 | 200 |

## BigText

Text in letters several cells tall, drawn with full or half blocks or with box drawing, in a font of its own.

- `rtwk_rich::bigtext::BigText` (struct) in `crates/rtwk-rich/src/bigtext/widget.rs:45`: `text` in big letters: each line of it set in `font`, or in the first of `fonts` it fits in, the last one when it fits in none; lines are a blank row apart, each placed as `align` says.

### Code

```rust
use std::rc::Rc;

use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::FocusNode;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Border, BorderType, BoxWidget, Flexible, MainAxisSize, SizedBox};
use rtwk::render::TextSegmenter;
use rtwk::style::{Style, Theme, ThemeData};
use rtwk::text::TextAlign;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget};
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
            2 => "rtwk\ngallery".to_owned(),
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
        2 => "rtwk\ngallery",
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

| option | values | starts at |
| --- | --- | --- |
| font | fit, large, block, thin, small | fit |
| text | typed, clock, two lines | typed |
| align | left, center, right | left |
| style | primary, accent, success, warning, info | primary |

## QrCode

Text as a QR code a phone's camera reads: in half blocks, two modules to a cell, black on white whatever the theme, with as much of the quiet zone around it as the rows allow.

- `rtwk_rich::qr::QrCode` (struct) in `crates/rtwk-rich/src/qr/code.rs:75`: What twk's `QrCode` widget lays out and paints: a symbol, or else text encoded at a level of error correction (QrSymbol::encode; data that does not fit in version 40 at that level is the QrError its layout returns), with a quiet zone of light modules around it, in dark modules on light ones (black on white by default, whatever the theme and the terminal's colors, since a scanner looks for dark modules on a light ground).
- `rtwk_rich::qr::QrCodeWidget` (struct) in `crates/rtwk-rich/src/qr/widget.rs:26`: A terminal QR widget, drawn in black on white with a four-module quiet zone by default.

### Code

```rust
use std::sync::{Arc, OnceLock};

use rtwk::layout::{CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Span, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, row};
use rtwk_rich::qr::{BLACK, QrCode, QrCodeWidget, QrErrorCorrection, QrSymbol, WHITE};

const PAYLOADS: [&str; 3] = [
    "https://crates.io/crates/rtwk",
    "WIFI:S:rtwk;T:WPA;P:secret;;",
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

| option | values | starts at |
| --- | --- | --- |
| text | link, wifi, digits | digits |
| level | low, medium, quartile, high | medium |
| quiet zone | 0 modules, 1 modules, 2 modules, 3 modules, 4 modules | 4 modules |
| inverted | false, true | false |
