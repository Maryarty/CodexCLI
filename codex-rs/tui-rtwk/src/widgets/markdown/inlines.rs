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
//!   colon, so `[a](/x/a.rs:3)` over `: why` reads `a (a.rs:3): why`.
//!
//! Flutter: `MarkdownBody(builders: {'a': LocalFileLink(cwd)}, softLineBreak: true)`. rtwk-rich
//! takes no element builders, so the parsed tree is rewritten before the view shows it
//! (`cells/citation.rs` does the same for directives): the view lays out, wraps and styles the
//! result as it does any text. Reference links resolve first, so a definition that points at a
//! local file reads as one.

use std::sync::Arc;

use rtwk_rich::markdown::{
    Block, Definitions, MdBlock, MdInline, MdList, MdListItem, MdTable, resolve_references,
};

use codex_tui_core::screens::cells::local_links;

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
        MdBlock::CodeBlock(_) | MdBlock::ThematicBreak => Arc::clone(block),
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
        }
    }
    out
}
