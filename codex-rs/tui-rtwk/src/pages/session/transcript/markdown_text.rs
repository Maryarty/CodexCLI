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
//! copy that spans both (`before`), as the area derives it from the layout. A formula is the text
//! the view draws for it (`widgets::markdown::math`: rtwk-math's one row, or its source); a display
//! formula between the text of a paragraph cuts the paragraph into nodes one under another, its
//! rows a node of their own, in the form that fits the width where the caller knows it
//! (`nodes_in`) and in the widest where it does not (G-new-markdown-text).
//! `tests/pages/markdown_text.rs` mounts a corpus and holds the copy of the model equal to what
//! the area copies; a block the corpus has not pinned answers `None`, and the item then has no
//! text (the `Gap` of `cells::text`).

use std::ops::Range;

use rtwk_rich::markdown::{Block, MdBlock, MdInline};

use super::markdown_copy::{self, Inline, Line};
use crate::widgets::markdown::math;

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
    /// What a copy of a selection of it says about its formatting (`markdown_copy`).
    pub copy: Line,
}

/// The nodes of a parsed source (`parse_markdown`), or `None` where one of the blocks is not
/// modelled: a heading gets its `#`s here.
pub fn text_nodes(blocks: &[Block]) -> Option<Vec<Node>> {
    Walk::of(blocks, false, None).map(|walk| walk.nodes)
}

/// The nodes of the blocks a cell draws (`cells::citation::blocks_of`, `widgets::markdown::view`):
/// a heading carries its `#` marker as drawn text already, so none is added.
pub fn nodes(blocks: &[Block]) -> Option<Vec<Node>> {
    Walk::of(blocks, true, None).map(|walk| walk.nodes)
}

/// `nodes` for a view `width` columns wide: a display formula is drawn in the form that fits
/// the block it is in (rows, one row, its source), which depends on the width. Without a width
/// (`nodes`) the model takes the rows.
pub fn nodes_in(blocks: &[Block], width: usize) -> Option<Vec<Node>> {
    Walk::of(blocks, true, Some(width)).map(|walk| walk.nodes)
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
    /// A table: only a table the author wrote in a fenced block of Markdown is a block to copy,
    /// the view draws it as a table and the fence is gone; the caller matches it with its source.
    Table,
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
    /// The index of the block among the top-level blocks when it is one of them (not inside a
    /// quote or a list item): the view can draw that block apart from the rest, as a fill.
    pub top: Option<usize>,
}

/// The code blocks and the top-level quotes of drawn `blocks` (a quote's own code blocks too, a
/// list item's), in the order of the view, with the nodes they are.
pub fn found(blocks: &[Block]) -> Vec<Found> {
    Walk::of(blocks, true, None).map_or_else(Vec::new, |walk| walk.found)
}

/// The nodes of a column of blocks, laid in the order of the view.
struct Walk {
    nodes: Vec<Node>,
    found: Vec<Found>,
    marked: bool,
    /// The columns of the view, where the caller knows them.
    width: Option<usize>,
    /// The top-level block being walked.
    top: usize,
    /// The quotes and list items around the block being walked, outermost first.
    frames: Vec<Frame>,
}

/// A container a block is in, for the prefix a copy of it writes.
enum Frame {
    Quote,
    Item {
        /// What the item's first line starts with: its bullet or number, and a task's box.
        marker: String,
        /// The width of the bullet or number, which the item's later lines are indented by.
        width: usize,
        /// The item's first block has been walked.
        used: bool,
    },
}

/// The text and the annotations of a run of inlines.
#[derive(Default)]
struct Built {
    text: String,
    line: Line,
}

impl Built {
    fn add(&mut self, text: &str, stack: &[Inline]) {
        self.text.push_str(text);
        self.line.push(text.len(), stack);
    }
}

impl Walk {
    fn of(blocks: &[Block], marked: bool, width: Option<usize>) -> Option<Walk> {
        let mut walk = Walk {
            nodes: Vec::new(),
            found: Vec::new(),
            marked,
            width,
            top: 0,
            frames: Vec::new(),
        };
        walk.blocks(blocks, "\n\n")?;
        Some(walk)
    }

    /// The columns of the block being walked: the view's, less what the quotes and list items
    /// around it take.
    fn room(&self) -> Option<usize> {
        let taken: usize = self
            .frames
            .iter()
            .map(|frame| match frame {
                Frame::Quote => "> ".len(),
                Frame::Item { width, .. } => *width,
            })
            .sum();
        self.width.map(|width| width.saturating_sub(taken))
    }

    /// A node of `text`, nested as the blocks around it say.
    fn leaf(&mut self, text: String, mut line: Line) {
        let innermost = self
            .frames
            .iter()
            .rposition(|frame| matches!(frame, Frame::Item { .. }));
        for (at, frame) in self.frames.iter_mut().enumerate() {
            match frame {
                Frame::Quote => {
                    for prefix in [
                        &mut line.prefix,
                        &mut line.continuation,
                        &mut line.item_prefix,
                    ] {
                        prefix.push_str("> ");
                    }
                }
                Frame::Item {
                    marker,
                    width,
                    used,
                } => {
                    let spaces = format!("{:1$}", "", *width);
                    line.prefix.push_str(if *used { &spaces } else { marker });
                    line.continuation.push_str(&spaces);
                    line.item_prefix.push_str(if Some(at) == innermost {
                        marker
                    } else {
                        &spaces
                    });
                    *used = true;
                }
            }
        }
        self.nodes.push(Node {
            text,
            before: "",
            copy: line,
        });
    }

    /// A node of the inlines `built`.
    fn inline_leaf(&mut self, built: Built) {
        self.leaf(built.text, built.line);
    }

    /// `blocks` one after the other, `sep` between them.
    fn blocks(&mut self, blocks: &[Block], sep: &'static str) -> Option<()> {
        for (at, block) in blocks.iter().enumerate() {
            let start = self.nodes.len();
            if self.frames.is_empty() {
                self.top = at;
            }
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
            MdBlock::Paragraph(inlines) => match math::pieces(inlines) {
                // a display formula between the text is a block of its own, the pieces one
                // under another with no gap between them
                Some(pieces) => {
                    for (at, piece) in pieces.iter().enumerate() {
                        let start = self.nodes.len();
                        match piece {
                            math::Piece::Text(text) => self.inline_leaf(built(text)?),
                            math::Piece::Display(source) => {
                                let text = math::display(source, self.room());
                                self.leaf(text, Line::default());
                            }
                        }
                        if at > 0 {
                            self.nodes[start].before = "\n";
                        }
                    }
                }
                None => self.inline_leaf(built(inlines)?),
            },
            MdBlock::Heading { level, children } => {
                // the marker is drawn text already, or is put before the text here
                let marker = format!("{} ", &MARKS[..usize::from(*level).clamp(1, MARKS.len())]);
                let mut heading = Built::default();
                if !self.marked {
                    heading.add(&marker, &[]);
                }
                push(children, &mut Vec::new(), &mut heading)?;
                heading.line.heading = if heading.text.starts_with(&marker) {
                    marker.len()
                } else {
                    0
                };
                self.inline_leaf(heading);
            }
            MdBlock::CodeBlock(code) => {
                let text = code.code.trim_end_matches('\n').to_owned();
                if !text.trim().is_empty() {
                    let at = self.nodes.len();
                    self.found.push(Found {
                        kind: Kind::Code,
                        nodes: at..at + 1,
                        text: text.clone(),
                        top: self.frames.is_empty().then_some(self.top),
                    });
                }
                self.leaf(text, Line::code());
            }
            MdBlock::BlockQuote(children) => {
                let (start, at) = (self.nodes.len(), self.found.len());
                let top = self.frames.is_empty().then_some(self.top);
                self.frames.push(Frame::Quote);
                self.blocks(children, "\n\n")?;
                self.frames.pop();
                let text = copied(&self.nodes[start..]);
                if !text.is_empty() {
                    // the quote comes before the blocks it holds
                    self.found.insert(
                        at,
                        Found {
                            kind: Kind::Quote,
                            nodes: start..self.nodes.len(),
                            text,
                            top,
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
                    let number = match list.start {
                        Some(first) => format!("{}{} ", first + at as u64, list.marker),
                        None => format!("{} ", list.marker),
                    };
                    let width = number.len();
                    let task = match item.checked {
                        Some(true) => "[x] ",
                        Some(false) => "[ ] ",
                        None => "",
                    };
                    self.frames.push(Frame::Item {
                        marker: format!("{number}{task}"),
                        width,
                        used: false,
                    });
                    self.blocks(&item.children, sep)?;
                    self.frames.pop();
                    if at > 0
                        && let Some(first) = self.nodes.get_mut(start)
                    {
                        first.before = sep;
                    }
                }
            }
            MdBlock::Table(table) => {
                // the header's cells, a blank row, then the cells of the rows: a node a cell
                let first_node = self.nodes.len();
                let top = self.frames.is_empty().then_some(self.top);
                for (at, cell) in table.header.iter().enumerate() {
                    let start = self.nodes.len();
                    self.leaf(inline_text(cell)?, Line::cell());
                    if at > 0 {
                        self.nodes[start].before = "\n";
                    }
                }
                let mut first = true;
                for cell in table.rows.iter().flatten() {
                    let start = self.nodes.len();
                    self.leaf(inline_text(cell)?, Line::cell());
                    self.nodes[start].before = if first { "\n\n" } else { "\n" };
                    first = false;
                }
                if self.nodes.len() > first_node {
                    self.found.push(Found {
                        kind: Kind::Table,
                        nodes: first_node..self.nodes.len(),
                        text: copied(&self.nodes[first_node..]),
                        top,
                    });
                }
            }
            // a rule has no text, and a block of an extension the app does not turn on (alerts)
            // is not modelled
            _ => return None,
        }
        Some(())
    }
}

fn inline_text(inlines: &[MdInline]) -> Option<String> {
    built(inlines).map(|built| built.text)
}

/// The text of `inlines` and what it is wrapped in, byte by byte.
fn built(inlines: &[MdInline]) -> Option<Built> {
    let mut out = Built::default();
    push(inlines, &mut Vec::new(), &mut out)?;
    Some(out)
}

fn push(inlines: &[MdInline], stack: &mut Vec<Inline>, out: &mut Built) -> Option<()> {
    for inline in inlines {
        match inline {
            MdInline::Text(text) => out.add(text, stack),
            MdInline::Code(text) => {
                stack.push(Inline::Code);
                out.add(text, stack);
                stack.pop();
            }
            MdInline::SoftBreak => out.add("\n", stack),
            MdInline::HardBreak => {
                out.line.hard_breaks.push(out.text.len());
                out.add("\n", stack);
            }
            MdInline::Emphasis(children) => inside(children, Inline::Delimiter("_"), stack, out)?,
            MdInline::Strong(children) => inside(children, Inline::Delimiter("**"), stack, out)?,
            MdInline::Strikethrough(children) => {
                inside(children, Inline::Delimiter("~~"), stack, out)?;
            }
            MdInline::Link {
                destination,
                children,
                ..
            } => {
                stack.push(if markdown_copy::web(destination) {
                    Inline::Link(destination.clone())
                } else {
                    Inline::Ignored
                });
                let start = out.text.len();
                push(children, stack, out)?;
                if out.text[start..] != **destination {
                    out.add(" (", stack);
                    out.add(destination, stack);
                    out.add(")", stack);
                }
                stack.pop();
            }
            MdInline::Image { children, .. } => push(children, stack, out)?,
            // resolved before the view draws them; not modelled unresolved
            MdInline::Reference { .. } => return None,
            // a formula is the text the view draws for it (`widgets::markdown::math`), plain: the
            // oracle copies a formula as it draws it
            MdInline::Math { source, display } => {
                out.add(&math::inline(source, *display), stack);
            }
            // footnote references belong to an extension the app does not turn on
            _ => return None,
        }
    }
    Some(())
}

fn inside(
    children: &[MdInline],
    inline: Inline,
    stack: &mut Vec<Inline>,
    out: &mut Built,
) -> Option<()> {
    stack.push(inline);
    push(children, stack, out)?;
    stack.pop();
    Some(())
}
