//! An assistant message as a `MarkdownView` that reads its file citations
//! (oracle markdown_render/inline_directives.rs): `:codex-file-citation{path=…}`
//! shows as the file's name in the transcript's code style.
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

use rtwk_rich::markdown::{
    Definitions, MarkdownStream, MarkdownView, MdBlock, MdCodeBlock, MdInline, MdList, MdListItem,
    MdTable,
};

use codex_tui_core::threads::citations::{Citation, citations, label};

use crate::widgets::markdown::with_heading_markers;

use super::markdown::lift_markdown_fences;

/// Private-use characters around a directive's number: letters to the
/// Markdown parser, so flanking and escapes around them are as around text.
const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';

/// `source` as the transcript shows an assistant message: table-holding
/// Markdown fences lifted, citations read against the session `cwd`.
pub fn view(source: &str, cwd: Option<&str>) -> MarkdownView {
    let lifted = lift_markdown_fences(source);
    let found = citations(&lifted);
    // The marker characters are the app's: text that holds them is shown as it is.
    if found.is_empty() || lifted.contains([OPEN, CLOSE]) {
        return crate::widgets::markdown::view(lifted.as_ref());
    }
    let found: Vec<Citation> = found
        .into_iter()
        .filter(|citation| !in_html(&lifted[..citation.range.start]))
        .collect();
    let mut masked = String::with_capacity(lifted.len());
    let mut end = 0;
    for (index, citation) in found.iter().enumerate() {
        masked.push_str(&lifted[end..citation.range.start]);
        masked.push_str(&format!("{OPEN}{index}{CLOSE}"));
        end = citation.range.end;
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
    MarkdownView::blocks(with_heading_markers(&blocks), definitions)
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
    found: &'a [Citation],
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
                Some((citation, length)) => {
                    out.push_str(&self.source[citation.range.clone()]);
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

    /// The citation a marker names, and the length of its digits and closing
    /// character; `text` follows the opening character.
    fn marker(&self, text: &str) -> Option<(&Citation, usize)> {
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
            let Some((citation, length)) = self.marker(after) else {
                break;
            };
            if start > 0 {
                out.push(MdInline::text(&rest[..start]));
            }
            out.push(MdInline::code(&label(&citation.path, self.cwd)));
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
