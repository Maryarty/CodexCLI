//! An assistant message as a `MarkdownView` that reads its directives
//! (oracle markdown_render/inline_directives.rs): `:codex-file-citation{path=…}`
//! shows as the file's name in the transcript's code style, and
//! `:codex-followup[label]{prompt=…}` as its label (inline Markdown) with the
//! prompt never drawn. A message still streaming shows what
//! `threads::stream_preview` lets through.
//!
//! Flutter: `MarkdownBody(inlineSyntaxes: [FileCitationSyntax()])`. rtwk-rich
//! takes no inline syntax, so the document is parsed here with the directives
//! masked (the oracle masks them too, so their punctuation cannot change how
//! the Markdown around them parses), and the node that stands for a directive
//! becomes a code node before the view shows the tree. Where the parser put a
//! directive in code, a link's text or destination, a reference definition or
//! HTML, the source text is put back: those stay literal.

use std::borrow::Cow;
use std::sync::Arc;

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::provider::Provider;
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::{
    Block, Definitions, MarkdownStream, MarkdownTheme, MarkdownView, MdBlock, MdCodeBlock,
    MdInline, MdList, MdListItem, MdTable,
};

use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_core::threads::citations::{Directive, directives, label};
use codex_tui_core::threads::{inline_vis, stream_preview};

use crate::widgets::markdown::prepared;

use super::markdown::{plain_lists, preferred};
use super::strings::Strings;

/// Private-use characters around a directive's number: letters to the
/// Markdown parser, so flanking and escapes around them are as around text.
const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';

/// `source` as the transcript shows an assistant message, `live` while it
/// streams: inline visualizations read as their note, table-holding Markdown
/// fences lifted, directives read against the session `cwd`.
pub fn view(source: &str, live: bool, cwd: Option<&str>, strings: &Strings) -> MarkdownView {
    view_with(source, live, cwd, strings, Rendering::default())
}

/// The Markdown of a message in `theme`, drawn under the user's `tui.rendering` preferences:
/// the widget reads them where it is built, so a change of the setting redraws it.
///
/// Flutter: a `Consumer<RenderingPrefs>` around the `MarkdownBody`.
pub fn cited(
    source: &str,
    live: bool,
    cwd: Option<&str>,
    strings: &'static Strings,
    theme: MarkdownTheme,
) -> AnyWidget {
    Cited {
        source: source.to_owned(),
        live,
        cwd: cwd.map(str::to_owned),
        strings,
        theme,
        filled: None,
    }
    .into()
}

/// `cited` with the top-level block `filled.block` drawn apart, filled: the copy mode's target
/// (oracle: every cell of the block's rows is the selection's). The widget draws the message's
/// rows whole, `filled.bullet` in front of the first, for the fill reaches the gutter too.
pub fn cited_filled(
    source: &str,
    live: bool,
    cwd: Option<&str>,
    strings: &'static Strings,
    theme: MarkdownTheme,
    filled: Filled,
) -> AnyWidget {
    Cited {
        source: source.to_owned(),
        live,
        cwd: cwd.map(str::to_owned),
        strings,
        theme,
        filled: Some(filled),
    }
    .into()
}

/// The block of a message that is drawn filled, and the message's bullet.
#[derive(Clone, PartialEq)]
pub struct Filled {
    pub block: usize,
    pub bullet: String,
    pub bullet_style: Style,
}

#[derive(Clone)]
struct Cited {
    filled: Option<Filled>,
    source: String,
    live: bool,
    cwd: Option<String>,
    strings: &'static Strings,
    theme: MarkdownTheme,
}

// a strings table is a `'static` constant: the same table is the same address
impl PartialEq for Cited {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.live == other.live
            && self.cwd == other.cwd
            && std::ptr::eq(self.strings, other.strings)
            && self.theme == other.theme
            && self.filled == other.filled
    }
}

struct CitedState;

impl StatefulWidget for Cited {
    type State = CitedState;

    fn create_state(&self) -> CitedState {
        CitedState
    }
}

impl_widget!(stateful eq Cited);

impl State for CitedState {
    type Widget = Cited;
    type Msg = ();

    fn build(&self, widget: &Cited, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // a tree with no app above it (a widget on its own) has the config's defaults
        let rendering = Provider::<TuiSettings>::maybe_of(cx, true)
            .map_or_else(Rendering::default, |settings| settings.rendering);
        let view = view_with(
            &widget.source,
            widget.live,
            widget.cwd.as_deref(),
            widget.strings,
            rendering,
        );
        if let Some(filled) = &widget.filled
            && let Some(blocks) = view.blocks.clone()
            && filled.block < blocks.len()
        {
            return self.filled(widget, filled, &blocks, &view);
        }
        crate::widgets::markdown::themed(view, widget.theme.clone()).into()
    }
}

impl CitedState {
    /// The message as its rows: the blocks before the filled one, the filled one in a box of the
    /// full width, the blocks after; the blank row between two blocks of a document is one
    /// `SizedBox` between them, so the rows are those of the whole view.
    fn filled(
        &self,
        widget: &Cited,
        filled: &Filled,
        blocks: &[Block],
        view: &MarkdownView,
    ) -> AnyWidget {
        let at = filled.block;
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let mut first = true;
        for (range, fill) in [
            (0..at, false),
            (at..at + 1, true),
            (at + 1..blocks.len(), false),
        ] {
            if range.is_empty() {
                continue;
            }
            if !first {
                column = column.child(SizedBox::new(None, Some(1), None));
            }
            let part = crate::widgets::markdown::themed(
                MarkdownView::blocks(blocks[range].to_vec(), view.definitions.clone()),
                widget.theme.clone(),
            );
            let gutter: AnyWidget = if first {
                Text::new(filled.bullet.clone())
                    .style(filled.bullet_style.clone())
                    .into()
            } else {
                Text::new(" ".repeat(filled.bullet.chars().count())).into()
            };
            let row = Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .spacing(1)
                .overflow_intended(true)
                .child(ExcludeSelection::new(gutter))
                .child(Expanded::new(
                    1,
                    Padding::new(EdgeInsets::only(0, 0, 1, 0), part),
                ));
            column = column.child(if fill {
                AnyWidget::from(
                    BoxWidget::new()
                        .style(Style::new().attrs(Attrs::REVERSE))
                        .child(row),
                )
            } else {
                row.into()
            });
            first = false;
        }
        column.into()
    }
}

/// `view` under the user's `tui.rendering` preferences: a disabled table or list renderer keeps
/// its source text (mermaid and math have no renderer here, so their sources show either way).
pub fn view_with(
    source: &str,
    live: bool,
    cwd: Option<&str>,
    strings: &Strings,
    rendering: Rendering,
) -> MarkdownView {
    let source = if live {
        stream_preview::visible(source)
    } else {
        source
    };
    let source = inline_vis::rewrite(source, strings.visualization_unavailable);
    let lifted = preferred(&source, rendering);
    let found = directives(&lifted);
    // The marker characters are the app's: text that holds them is shown as it is.
    if found.is_empty() || lifted.contains([OPEN, CLOSE]) {
        if rendering.lists {
            return crate::widgets::markdown::view_in(lifted.as_ref(), cwd);
        }
        let mut stream = MarkdownStream::new(&lifted);
        let blocks = plain_lists(stream.blocks());
        let definitions = stream.definitions().clone();
        return MarkdownView::blocks(prepared(&blocks, &definitions, cwd), definitions);
    }
    let found: Vec<Directive> = found
        .into_iter()
        .filter(|directive| !in_html(&lifted[..directive.range().start]))
        .collect();
    let mut masked = String::with_capacity(lifted.len());
    let mut end = 0;
    for (index, directive) in found.iter().enumerate() {
        masked.push_str(&lifted[end..directive.range().start]);
        masked.push_str(&format!("{OPEN}{index}{CLOSE}"));
        end = directive.range().end;
    }
    masked.push_str(&lifted[end..]);
    let directives = Directives {
        source: &lifted,
        found: &found,
        cwd,
    };
    let mut stream = MarkdownStream::new(&masked);
    let blocks: Vec<_> = stream
        .blocks()
        .iter()
        .map(|block| Arc::new(directives.block(block)))
        .collect();
    let blocks = if rendering.lists {
        blocks
    } else {
        plain_lists(&blocks)
    };
    let definitions: Definitions = stream
        .definitions()
        .iter()
        .map(|(key, definition)| {
            let mut definition = definition.clone();
            definition.destination = directives.restore(&definition.destination).into_owned();
            definition.title = definition
                .title
                .map(|title| directives.restore(&title).into_owned());
            (key.clone(), definition)
        })
        .collect();
    MarkdownView::blocks(prepared(&blocks, &definitions, cwd), definitions)
}

/// The blocks the cell draws for `source` (what `view_with` hands the view: the directives read,
/// the user's `rendering` applied, the links and headings rewritten as the oracle reads them).
/// A heading carries its `#` marker as drawn text; `transcript::markdown_text` writes its own,
/// so a text model over these blocks (find, the select mode and copy of a message that is not
/// mounted, a cell that wraps this body) takes the marker off the heading first.
pub fn blocks_of(
    source: &str,
    live: bool,
    cwd: Option<&str>,
    strings: &Strings,
    rendering: Rendering,
) -> Vec<Block> {
    view_with(source, live, cwd, strings, rendering)
        .blocks
        .map(|blocks| blocks.to_vec())
        .unwrap_or_default()
}

/// The inline nodes of a follow-up's label: its own Markdown, the first
/// paragraph's inlines; a label that parses to anything else is its text.
fn label_inlines(label: &str) -> Vec<MdInline> {
    let mut stream = MarkdownStream::new(label);
    match stream.blocks() {
        [block] => match block.as_ref() {
            MdBlock::Paragraph(inlines) => inlines.clone(),
            _ => vec![MdInline::text(label)],
        },
        _ => vec![MdInline::text(label)],
    }
}

/// Whether the text before a directive leaves an HTML tag or comment open
/// (`<span title='` and `<!-- `): raw HTML is literal.
fn in_html(before: &str) -> bool {
    let Some(open) = before.rfind('<') else {
        return false;
    };
    if before[..open]
        .bytes()
        .rev()
        .take_while(|b| *b == b'\\')
        .count()
        % 2
        == 1
    {
        return false;
    }
    let tail = &before[open..];
    if tail.contains("\n\n") || tail.contains('`') {
        return false;
    }
    if let Some(comment) = tail.strip_prefix("<!--") {
        return !comment.contains("-->");
    }
    tail[1..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?'))
        && !tail.contains('>')
}

struct Directives<'a> {
    source: &'a str,
    found: &'a [Directive],
    cwd: Option<&'a str>,
}

impl Directives<'_> {
    /// `text` with every marker replaced by its directive's source.
    fn restore<'t>(&self, text: &'t str) -> Cow<'t, str> {
        if !text.contains(OPEN) {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find(OPEN) {
            out.push_str(&rest[..start]);
            let after = &rest[start + OPEN.len_utf8()..];
            match self.marker(after) {
                Some((directive, length)) => {
                    out.push_str(&self.source[directive.range().clone()]);
                    rest = &after[length..];
                }
                None => {
                    out.push(OPEN);
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        Cow::Owned(out)
    }

    /// The directive a marker names, and the length of its digits and closing
    /// character; `text` follows the opening character.
    fn marker(&self, text: &str) -> Option<(&Directive, usize)> {
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        let index: usize = text[..digits].parse().ok()?;
        text[digits..].starts_with(CLOSE).then_some(())?;
        Some((self.found.get(index)?, digits + CLOSE.len_utf8()))
    }

    fn block(&self, block: &MdBlock) -> MdBlock {
        match block {
            MdBlock::Paragraph(inlines) => MdBlock::Paragraph(self.inlines(inlines, true)),
            MdBlock::Heading { level, children } => MdBlock::Heading {
                level: *level,
                children: self.inlines(children, true),
            },
            MdBlock::CodeBlock(code) => MdBlock::CodeBlock(MdCodeBlock {
                code: self.restore(&code.code).into_owned(),
                info: self.restore(&code.info).into_owned(),
                fenced: code.fenced,
            }),
            MdBlock::ThematicBreak => MdBlock::ThematicBreak,
            MdBlock::BlockQuote(blocks) => MdBlock::BlockQuote(self.blocks(blocks)),
            MdBlock::List(list) => MdBlock::List(MdList {
                items: list
                    .items
                    .iter()
                    .map(|item| {
                        Arc::new(MdListItem {
                            children: self.blocks(&item.children),
                            checked: item.checked,
                        })
                    })
                    .collect(),
                ..list.clone()
            }),
            MdBlock::Table(table) => {
                let cells = |cells: &[Vec<MdInline>]| -> Vec<Vec<MdInline>> {
                    cells.iter().map(|cell| self.inlines(cell, true)).collect()
                };
                MdBlock::Table(MdTable {
                    align: table.align.clone(),
                    header: cells(&table.header),
                    rows: table.rows.iter().map(|row| cells(row)).collect(),
                })
            }
        }
    }

    fn blocks(&self, blocks: &[Arc<MdBlock>]) -> Vec<Arc<MdBlock>> {
        blocks
            .iter()
            .map(|block| Arc::new(self.block(block)))
            .collect()
    }

    /// `inlines` with each marker a code node (`read`) or its source (inside
    /// code, a link or an image: literal). Texts side by side are one text
    /// first, so a marker never splits across them.
    fn inlines(&self, inlines: &[MdInline], read: bool) -> Vec<MdInline> {
        let mut out = Vec::with_capacity(inlines.len());
        let mut text = String::new();
        for inline in inlines {
            if let MdInline::Text(piece) = inline {
                text.push_str(piece);
                continue;
            }
            self.flush(&mut text, &mut out, read);
            out.push(self.inline(inline, read));
        }
        self.flush(&mut text, &mut out, read);
        out
    }

    fn flush(&self, text: &mut String, out: &mut Vec<MdInline>, read: bool) {
        if text.is_empty() {
            return;
        }
        let mut rest = text.as_str();
        while read && let Some(start) = rest.find(OPEN) {
            let after = &rest[start + OPEN.len_utf8()..];
            let Some((directive, length)) = self.marker(after) else {
                break;
            };
            if start > 0 {
                out.push(MdInline::text(&rest[..start]));
            }
            match directive {
                Directive::Citation(citation) => {
                    out.push(MdInline::code(&label(&citation.path, self.cwd)));
                }
                // The label is Markdown of its own, parsed apart from the text around it (the
                // oracle keeps an unmatched `[` as a sentinel so a label cannot open a block).
                Directive::Followup(followup) => out.extend(label_inlines(&followup.label)),
            }
            rest = &after[length..];
        }
        if !rest.is_empty() {
            out.push(MdInline::text(&self.restore(rest)));
        }
        text.clear();
    }

    fn inline(&self, inline: &MdInline, read: bool) -> MdInline {
        let nodes = |children: &[MdInline], read: bool| -> Arc<[MdInline]> {
            self.inlines(children, read).into()
        };
        match inline {
            MdInline::Text(text) => MdInline::text(&self.restore(text)),
            MdInline::Code(code) => MdInline::code(&self.restore(code)),
            MdInline::Emphasis(children) => MdInline::Emphasis(nodes(children, read)),
            MdInline::Strong(children) => MdInline::Strong(nodes(children, read)),
            MdInline::Strikethrough(children) => MdInline::Strikethrough(nodes(children, read)),
            MdInline::Link {
                destination,
                children,
                title,
            } => MdInline::Link {
                destination: self.restore(destination).as_ref().into(),
                children: nodes(children, false),
                title: title
                    .as_deref()
                    .map(|title| self.restore(title).as_ref().into()),
            },
            MdInline::Image {
                source,
                children,
                title,
            } => MdInline::Image {
                source: self.restore(source).as_ref().into(),
                children: nodes(children, false),
                title: title
                    .as_deref()
                    .map(|title| self.restore(title).as_ref().into()),
            },
            MdInline::Reference {
                label,
                children,
                kind,
                image,
            } => MdInline::Reference {
                label: self.restore(label).as_ref().into(),
                children: nodes(children, false),
                kind: *kind,
                image: *image,
            },
            MdInline::SoftBreak => MdInline::SoftBreak,
            MdInline::HardBreak => MdInline::HardBreak,
        }
    }
}
