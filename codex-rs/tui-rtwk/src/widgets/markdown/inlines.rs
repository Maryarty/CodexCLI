//! What the oracle's Markdown renderer does with inline content that rtwk's `MarkdownView` does
//! not (markdown_render.rs `soft_break`, `push_link`, `pop_link`;
//! markdown_render/local_links.rs):
//!
//! - a line break of the source is a line break of the screen, not a space (`soft_break`
//!   pushes a new line); in a table cell it stays a space;
//! - a link to a local file reads its label and then the path, relative to the session's
//!   directory and in code style: `the lib (src/lib.rs:12:3)`; a link with no label text reads
//!   the path alone;
//! - a line break straight after such a link stays inline when the next text starts with a
//!   colon, so `[a](/x/a.rs:3)` over `: why` reads `a (a.rs:3): why`;
//! - an ordered list is numbered `1.` whatever delimiter its source has (`1)`);
//! - a URL written in the text reads with its character references decoded (`a&amp;b` is
//!   `a&b`), in its label and in its target.
//!
//! Flutter: `MarkdownBody(builders: {'a': LocalFileLink(cwd)}, softLineBreak: true)`. rtwk-rich
//! takes no element builders, so the parsed tree is rewritten before the view shows it
//! (`cells/citation.rs` does the same for directives): the view lays out, wraps and styles the
//! result as it does any text. Reference links resolve first, so a definition that points at a
//! local file reads as one.

use std::sync::Arc;

use rtwk_rich::markdown::{
    Block, Definitions, MdBlock, MdInline, MdList, MdListItem, MdTable, decode_entity,
    resolve_references,
};

use codex_tui_core::threads::local_links;

/// `blocks` as the oracle reads them: references resolved, local links and line breaks
/// rewritten (module doc), local file paths relative to `cwd`.
pub(super) fn rewritten(
    blocks: &[Block],
    definitions: &Definitions,
    cwd: Option<&str>,
) -> Vec<Block> {
    resolve_references(blocks, definitions)
        .iter()
        .map(|block| self::block(block, cwd))
        .collect()
}

fn block(block: &Block, cwd: Option<&str>) -> Block {
    match &**block {
        MdBlock::Paragraph(children) => Arc::new(MdBlock::Paragraph(inlines(children, cwd, true))),
        MdBlock::Heading { level, children } => Arc::new(MdBlock::Heading {
            level: *level,
            children: inlines(children, cwd, true),
        }),
        MdBlock::BlockQuote(children) => Arc::new(MdBlock::BlockQuote(blocks(children, cwd))),
        MdBlock::List(list) => Arc::new(MdBlock::List(MdList {
            items: list
                .items
                .iter()
                .map(|item| {
                    Arc::new(MdListItem {
                        children: blocks(&item.children, cwd),
                        checked: item.checked,
                    })
                })
                .collect(),
            // The oracle numbers every ordered list `1.`, whatever delimiter the source has.
            marker: if list.start.is_some() {
                '.'
            } else {
                list.marker
            },
            ..list.clone()
        })),
        MdBlock::Table(table) => {
            let cells = |cells: &[Vec<MdInline>]| -> Vec<Vec<MdInline>> {
                cells.iter().map(|cell| inlines(cell, cwd, false)).collect()
            };
            Arc::new(MdBlock::Table(MdTable {
                align: table.align.clone(),
                header: cells(&table.header),
                rows: table.rows.iter().map(|row| cells(row)).collect(),
            }))
        }
        // a code block, a rule, and a block of an extension the app does not turn on (alerts)
        _ => Arc::clone(block),
    }
}

fn blocks(blocks: &[Block], cwd: Option<&str>) -> Vec<Block> {
    blocks.iter().map(|b| block(b, cwd)).collect()
}

/// `list` rewritten; `flow` is false inside a table cell, where a line break stays a space.
fn inlines(list: &[MdInline], cwd: Option<&str>, flow: bool) -> Vec<MdInline> {
    let mut out = Vec::with_capacity(list.len());
    // The last node written was a local link's path: `line_ends_with_local_link_target`.
    let mut after_link = false;
    for (index, node) in list.iter().enumerate() {
        let mut linked = false;
        match node {
            MdInline::SoftBreak if flow => {
                if !(after_link && colon_follows(list.get(index + 1))) {
                    out.push(MdInline::HardBreak);
                }
            }
            MdInline::Link {
                destination,
                children,
                ..
            } if local_links::is_local_path_like(destination) => {
                out.extend(local_link(destination, children, cwd));
                linked = flow;
            }
            _ => out.push(inline(node, cwd, flow)),
        }
        after_link = linked;
    }
    out
}

/// Whether the text `node` starts, after its blanks, with a colon.
fn colon_follows(node: Option<&MdInline>) -> bool {
    matches!(node, Some(MdInline::Text(text)) if text.trim_start().starts_with(':'))
}

fn inline(node: &MdInline, cwd: Option<&str>, flow: bool) -> MdInline {
    let children =
        |children: &[MdInline]| -> Arc<[MdInline]> { inlines(children, cwd, flow).into() };
    match node {
        MdInline::Emphasis(c) => MdInline::Emphasis(children(c)),
        MdInline::Strong(c) => MdInline::Strong(children(c)),
        MdInline::Strikethrough(c) => MdInline::Strikethrough(children(c)),
        // A URL written in the text is a link of its own text; its character references stay
        // undecoded in rtwk's parse, the oracle decodes them before it finds the URL.
        MdInline::Link {
            destination,
            children: c,
            title,
        } if matches!(&c[..], [MdInline::Text(text)] if text == destination) => {
            let decoded: Arc<str> = decoded(destination).into();
            MdInline::Link {
                destination: Arc::clone(&decoded),
                children: Arc::from([MdInline::Text(decoded)]),
                title: title.clone(),
            }
        }
        MdInline::Link {
            destination,
            children: c,
            title,
        } => MdInline::Link {
            destination: Arc::clone(destination),
            children: children(c),
            title: title.clone(),
        },
        other => other.clone(),
    }
}

/// `text` with its character references (`&amp;`, `&#x41;`) replaced by what they stand for.
fn decoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.find(';').filter(|end| {
            *end <= 32
                && rest[1..*end]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '#')
        });
        match end.and_then(|end| decode_entity(&rest[..=end]).map(|c| (c, end))) {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A local link as nodes: `label (path)` where the label has text, `path` alone where not.
/// The label's line breaks are spaces (`push_local_link_label_break`).
fn local_link(destination: &str, label: &[MdInline], cwd: Option<&str>) -> Vec<MdInline> {
    let path = MdInline::code(&local_links::display(destination, cwd));
    if plain(label).trim().is_empty() {
        return vec![path];
    }
    let mut out = inlines(label, cwd, false);
    spaced(&mut out);
    out.extend([MdInline::text(" ("), path, MdInline::text(")")]);
    out
}

/// A hard break in a label is a space as a soft break is.
fn spaced(nodes: &mut [MdInline]) {
    for node in nodes {
        match node {
            MdInline::HardBreak => *node = MdInline::SoftBreak,
            MdInline::Emphasis(children)
            | MdInline::Strong(children)
            | MdInline::Strikethrough(children) => {
                let mut copy = children.to_vec();
                spaced(&mut copy);
                *children = copy.into();
            }
            _ => {}
        }
    }
}

/// The text `nodes` read as: what the oracle tests for "a label with text in it".
fn plain(nodes: &[MdInline]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            MdInline::Text(text) | MdInline::Code(text) => out.push_str(text),
            MdInline::Emphasis(children)
            | MdInline::Strong(children)
            | MdInline::Strikethrough(children) => out.push_str(&plain(children)),
            MdInline::Link { children, .. } => out.push_str(&plain(children)),
            MdInline::Image { .. } => out.push_str(&node.alt()),
            MdInline::Reference { children, .. } => out.push_str(&plain(children)),
            MdInline::SoftBreak | MdInline::HardBreak => out.push(' '),
            // footnote references and math belong to extensions the app does not turn on
            _ => {}
        }
    }
    out
}
