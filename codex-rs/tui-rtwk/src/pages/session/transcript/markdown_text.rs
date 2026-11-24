//! The text of a Markdown message that is not mounted: what a `SelectionArea`
//! over its `MarkdownView` selects, written from the blocks.
//!
//! Find, the select mode and copy read the items of the transcript that are
//! scrolled out of the list (`ListView::item_text`); a message's body is a rtwk
//! `MarkdownView` whose strings are a column of `Text`s the app cannot read
//! before it is mounted (rtwk-gaps: "text of a Markdown document that is not
//! mounted"). This is the app's model of that column: a block is its text, a
//! blank line lies between blocks, the marks the view draws as gutters (the
//! bullets and numbers of a list, the `>` of a quote, the fence of a code
//! block) are not in it, a heading keeps its `#`s, a link is its text and its
//! destination. `tests/pages/markdown_text.rs` mounts a corpus and holds the two
//! equal; a block the corpus has not pinned answers `None`, and the item then
//! has no text (the `Gap` of `cells::text`).

use rtwk_rich::markdown::{Block, MdBlock, MdInline, MdList, MdTable};

/// The `#`s of a heading, one per level.
const MARKS: &str = "######";

/// The text of the blocks of a parsed source (`parse_markdown`), or `None` where one of them is
/// not modelled: a heading gets its `#`s here.
pub fn text(blocks: &[Block]) -> Option<String> {
    text_of(blocks, false)
}

/// The text of the blocks a cell draws (`cells::citation::blocks_of`, `widgets::markdown::view`):
/// a heading carries its `#` marker as drawn text already, so none is added.
pub fn drawn(blocks: &[Block]) -> Option<String> {
    text_of(blocks, true)
}

fn text_of(blocks: &[Block], marked: bool) -> Option<String> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        out.push(block_text(block, marked)?);
    }
    Some(out.join("\n\n"))
}

fn block_text(block: &MdBlock, marked: bool) -> Option<String> {
    Some(match block {
        MdBlock::Paragraph(inlines) => inline_text(inlines)?,
        MdBlock::Heading { children, .. } if marked => inline_text(children)?,
        MdBlock::Heading { level, children } => {
            let marks = &MARKS[..usize::from(*level).clamp(1, MARKS.len())];
            format!("{marks} {}", inline_text(children)?)
        }
        MdBlock::CodeBlock(code) => code.code.trim_end_matches('\n').to_owned(),
        MdBlock::BlockQuote(children) => text_of(children, marked)?,
        MdBlock::List(list) => list_text(list, marked)?,
        MdBlock::Table(table) => table_text(table)?,
        MdBlock::ThematicBreak => return None,
    })
}

fn list_text(list: &MdList, marked: bool) -> Option<String> {
    // a tight list has no blank row between its items nor between the blocks of
    // one item; a loose list has it between both
    let sep = if list.tight { "\n" } else { "\n\n" };
    let mut items = Vec::with_capacity(list.items.len());
    for item in &list.items {
        let mut parts = Vec::with_capacity(item.children.len());
        for child in &item.children {
            parts.push(block_text(child, marked)?);
        }
        items.push(parts.join(sep));
    }
    Some(items.join(sep))
}

fn table_text(table: &MdTable) -> Option<String> {
    let mut header = Vec::with_capacity(table.header.len());
    for cell in &table.header {
        header.push(inline_text(cell)?);
    }
    let mut rows = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        for cell in row {
            rows.push(inline_text(cell)?);
        }
    }
    let header = header.join("\n");
    if rows.is_empty() {
        return Some(header);
    }
    Some(format!("{header}\n\n{}", rows.join("\n")))
}

fn inline_text(inlines: &[MdInline]) -> Option<String> {
    let mut out = String::new();
    push(inlines, &mut out)?;
    Some(out)
}

fn push(inlines: &[MdInline], out: &mut String) -> Option<()> {
    for inline in inlines {
        match inline {
            MdInline::Text(text) | MdInline::Code(text) => out.push_str(text),
            MdInline::SoftBreak | MdInline::HardBreak => out.push('\n'),
            MdInline::Emphasis(children)
            | MdInline::Strong(children)
            | MdInline::Strikethrough(children) => push(children, out)?,
            MdInline::Link {
                destination,
                children,
                ..
            } => {
                let start = out.len();
                push(children, out)?;
                if out[start..] != **destination {
                    out.push_str(" (");
                    out.push_str(destination);
                    out.push(')');
                }
            }
            MdInline::Image { children, .. } => push(children, out)?,
            // resolved before the view draws them; not modelled unresolved
            MdInline::Reference { .. } => return None,
        }
    }
    Some(())
}
