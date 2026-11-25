//! The text of a Markdown message that is not mounted: what a `SelectionArea`
//! over its `MarkdownView` selects, written from the blocks.
//!
//! Find, the select mode and copy read the items of the transcript that are
//! scrolled out of the list (`ListView::item_text`); a message's body is a rtwk
//! `MarkdownView` whose strings are a column of `Text`s the app cannot read
//! before it is mounted (rtwk-gaps: "text of a Markdown document that is not
//! mounted"). This is the app's model of that column, a [`Node`] for each `Text`: a block
//! is its text, the marks the view draws as gutters (the bullets and numbers of a list, the `>`
//! of a quote, the fence of a code block) are not in it, a heading keeps its `#`s, a link is its
//! text and its destination. A selection is addressed per node, `(node, start, end)` in the
//! node's own text; the blank line that lies between two blocks is not in either text but in a
//! copy that spans both (`before`), as the area derives it from the layout.
//! `tests/pages/markdown_text.rs` mounts a corpus and holds the copy of the model equal to what
//! the area copies; a block the corpus has not pinned answers `None`, and the item then has no
//! text (the `Gap` of `cells::text`).

use std::ops::Range;

use rtwk_rich::markdown::{Block, MdBlock, MdInline};

/// The `#`s of a heading, one per level.
const MARKS: &str = "######";

/// One `Text` of the view: the leaf the area selects in. The text is the node's own; what lies
/// between it and the node before it exists only in a copy of a selection that spans both, and
/// the area derives that from the layout (`"\n"` where the nodes are on rows that follow each
/// other, `"\n\n"` where rows lie between): `before` is that for the blocks as the view lays them
/// out. The model a list's selection maps a row through joins the nodes by `"\n"` alone (the
/// nodes' own lengths), so a range of a row is kept as the node it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub text: String,
    /// What a copy puts before this node when the node before it is selected too (empty for the
    /// first node of the view).
    pub before: &'static str,
}

/// The nodes of a parsed source (`parse_markdown`), or `None` where one of the blocks is not
/// modelled: a heading gets its `#`s here.
pub fn text_nodes(blocks: &[Block]) -> Option<Vec<Node>> {
    Walk::of(blocks, false).map(|walk| walk.nodes)
}

/// The nodes of the blocks a cell draws (`cells::citation::blocks_of`, `widgets::markdown::view`):
/// a heading carries its `#` marker as drawn text already, so none is added.
pub fn nodes(blocks: &[Block]) -> Option<Vec<Node>> {
    Walk::of(blocks, true).map(|walk| walk.nodes)
}

/// What a copy of a selection over `nodes` whole answers: the texts with what lies between them.
pub fn copied(nodes: &[Node]) -> String {
    let mut out = String::new();
    for (at, node) in nodes.iter().enumerate() {
        if at > 0 {
            out.push_str(node.before);
        }
        out.push_str(&node.text);
    }
    out
}

/// The text of the blocks of a parsed source (`parse_markdown`), or `None` where one of them is
/// not modelled: a heading gets its `#`s here.
pub fn text(blocks: &[Block]) -> Option<String> {
    text_nodes(blocks).map(|nodes| copied(&nodes))
}

/// The text of the blocks a cell draws, as the area copies the whole view.
pub fn drawn(blocks: &[Block]) -> Option<String> {
    nodes(blocks).map(|nodes| copied(&nodes))
}

/// What a block of a message is for the copy mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Code,
    Quote,
}

/// A block the copy mode can select: the nodes it is (a code block is one, a quote the nodes of its
/// children) and what a copy of them is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub kind: Kind,
    /// The indices in [`nodes`]: the selection runs from the start of the first to the end of the
    /// last.
    pub nodes: Range<usize>,
    pub text: String,
}

/// The code blocks and the top-level quotes of drawn `blocks` (a quote's own code blocks too, a
/// list item's), in the order of the view, with the nodes they are.
pub fn found(blocks: &[Block]) -> Vec<Found> {
    Walk::of(blocks, true).map_or_else(Vec::new, |walk| walk.found)
}

/// The nodes of a column of blocks, laid in the order of the view.
struct Walk {
    nodes: Vec<Node>,
    found: Vec<Found>,
    marked: bool,
}

impl Walk {
    fn of(blocks: &[Block], marked: bool) -> Option<Walk> {
        let mut walk = Walk {
            nodes: Vec::new(),
            found: Vec::new(),
            marked,
        };
        walk.blocks(blocks, "\n\n")?;
        Some(walk)
    }

    fn leaf(&mut self, text: String) {
        self.nodes.push(Node { text, before: "" });
    }

    /// `blocks` one after the other, `sep` between them.
    fn blocks(&mut self, blocks: &[Block], sep: &'static str) -> Option<()> {
        for (at, block) in blocks.iter().enumerate() {
            let start = self.nodes.len();
            self.block(block)?;
            if at > 0
                && let Some(first) = self.nodes.get_mut(start)
            {
                first.before = sep;
            }
        }
        Some(())
    }

    fn block(&mut self, block: &MdBlock) -> Option<()> {
        match block {
            MdBlock::Paragraph(inlines) => self.leaf(inline_text(inlines)?),
            MdBlock::Heading { children, .. } if self.marked => self.leaf(inline_text(children)?),
            MdBlock::Heading { level, children } => {
                let marks = &MARKS[..usize::from(*level).clamp(1, MARKS.len())];
                self.leaf(format!("{marks} {}", inline_text(children)?));
            }
            MdBlock::CodeBlock(code) => {
                let text = code.code.trim_end_matches('\n').to_owned();
                if !text.trim().is_empty() {
                    let at = self.nodes.len();
                    self.found.push(Found {
                        kind: Kind::Code,
                        nodes: at..at + 1,
                        text: text.clone(),
                    });
                }
                self.leaf(text);
            }
            MdBlock::BlockQuote(children) => {
                let (start, at) = (self.nodes.len(), self.found.len());
                self.blocks(children, "\n\n")?;
                let text = copied(&self.nodes[start..]);
                if !text.is_empty() {
                    // the quote comes before the blocks it holds
                    self.found.insert(
                        at,
                        Found {
                            kind: Kind::Quote,
                            nodes: start..self.nodes.len(),
                            text,
                        },
                    );
                }
            }
            MdBlock::List(list) => {
                // a tight list has no blank row between its items nor between the blocks of
                // one item; a loose list has it between both
                let sep = if list.tight { "\n" } else { "\n\n" };
                for (at, item) in list.items.iter().enumerate() {
                    let start = self.nodes.len();
                    self.blocks(&item.children, sep)?;
                    if at > 0
                        && let Some(first) = self.nodes.get_mut(start)
                    {
                        first.before = sep;
                    }
                }
            }
            MdBlock::Table(table) => {
                // the header's cells, a blank row, then the cells of the rows: a node a cell
                for (at, cell) in table.header.iter().enumerate() {
                    let start = self.nodes.len();
                    self.leaf(inline_text(cell)?);
                    if at > 0 {
                        self.nodes[start].before = "\n";
                    }
                }
                let mut first = true;
                for cell in table.rows.iter().flatten() {
                    let start = self.nodes.len();
                    self.leaf(inline_text(cell)?);
                    self.nodes[start].before = if first { "\n\n" } else { "\n" };
                    first = false;
                }
            }
            MdBlock::ThematicBreak => return None,
        }
        Some(())
    }
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
