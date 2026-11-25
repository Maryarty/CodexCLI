//! Find (oracle `transcript_view/search.rs`): the incremental literal search over the whole
//! transcript, its one match and the position it came from.
//!
//! The search is a controller (Flutter's `ValueNotifier<FindState>` of a `SearchDelegate`) that
//! lives in `widgets::finder`, one per thread's `ThreadUi` (`ThreadUi::find`, and `Input::Find`
//! while it is open): the page's key action begins it, the footer's field edits its query and the
//! keys of the footer step and end it, the transcript's pane reads its match (it scrolls the list
//! to it, and the selection area shows it by selecting it, REVERSED as the oracle draws a match)
//! and runs the scan the query asked for. This file is Find's door for the transcript: the names
//! it has always had (`Finder`, `Mode`, `Hit`, `Key`, `key`), the entry that reads the finder of
//! a thread's controllers, and the rows the scan reads. The words of the footer are
//! `find_footer`'s.
//!
//! What the search reads is the rows of the transcript as text (`RowTexts`, in the order they
//! are read: the oldest first), through the same `ItemTexts` model the list's selection reads
//! (`item_text`); a row whose text the cells do not model yet (`cells::text::Gap::Rows`) has
//! none, so nothing is found in it. A row's text is its `Text`s' own texts joined by a line
//! break, and nothing between the blocks of a Markdown body: the offsets of a row are a node and
//! an offset in the node's text (`item_text::mounted_parts`), and what lies between two blocks
//! is in a copy of a selection across them, which the area derives from the layout.
//!
//! Where the oracle reads one entry per frame, bounded, the scan here reads the whole transcript
//! in the frame the query changed: the rows are in memory and a literal scan of them is the
//! cost of laying out one screen.

use std::borrow::Cow;

use rtwk::gestures::SelectionPoint;
use rtwk::render::RenderId;
use rtwk::scroll::ItemTexts;

pub use crate::widgets::finder::{Direction, Finder, Hit, Key, Mode, QUERY_BYTES, Rows, key};
use crate::widgets::thread_ui::ThreadUi;

/// The finder of a thread's controllers (the one `begin` opens and the pane reads).
pub fn finder(ui: &ThreadUi) -> Finder {
    ui.find.clone()
}

/// The finder of a thread's controllers while it reads an accepted match: the page's keys
/// (`Ctrl+P`, `Ctrl+N`, Esc) are its while it does.
pub fn reading(ui: &ThreadUi) -> Option<Finder> {
    ui.find.reading().then(|| ui.find.clone())
}

/// The rows of a transcript as the list shows them: what the list's `ItemTexts` model reads (the
/// selection area counts the items of a reverse list by the render list's children, from the top:
/// oldest first) and the scan runs over.
pub struct RowTexts {
    /// The text of each row, oldest first.
    texts: Vec<String>,
    /// Per row, per line of its text: the line is a gutter (a bullet, a prompt mark) the select
    /// mode's caret never stops on and a copy leaves out. Empty when no row has one.
    gutters: Vec<Vec<bool>>,
    /// Where each row starts in the text of the list (the rows are joined by a line break).
    starts: Vec<usize>,
    reverse: bool,
    revision: u64,
}

impl RowTexts {
    pub fn new(texts: Vec<String>, reverse: bool, revision: u64) -> RowTexts {
        let mut at = 0;
        let starts = texts
            .iter()
            .map(|text| {
                let start = at;
                at += text.len() + 1;
                start
            })
            .collect();
        RowTexts {
            texts,
            gutters: Vec::new(),
            starts,
            reverse,
            revision,
        }
    }

    /// The rows with their gutter lines (`gutters[row][line]`).
    pub fn with_gutters(mut self, gutters: Vec<Vec<bool>>) -> RowTexts {
        self.gutters = gutters;
        self
    }

    /// How many rows there are.
    pub fn row_count(&self) -> usize {
        self.texts.len()
    }

    /// The text of row `row` (oldest first): empty past the last.
    pub fn row_text(&self, row: usize) -> &str {
        self.texts.get(row).map_or("", String::as_str)
    }

    /// Whether line `line` of row `row` is a gutter.
    pub fn is_gutter(&self, row: usize, line: usize) -> bool {
        self.gutters
            .get(row)
            .and_then(|lines| lines.get(line))
            .copied()
            .unwrap_or(false)
    }

    /// The revision of the transcript the rows were read from.
    pub fn revision_of(&self) -> u64 {
        self.revision
    }

    /// The place in the list's text of the byte `offset` of row `row`, as the selection area
    /// names it: `list` is the render object of the list (`SelectionAreaController::start`).
    pub fn point(&self, list: RenderId, row: usize, offset: usize) -> SelectionPoint {
        SelectionPoint {
            object: list,
            offset: self.starts.get(row).copied().unwrap_or(0) + offset,
        }
    }

    /// The `ListController`'s index of row `row` (oldest first): a reverse list counts from its
    /// bottom.
    pub fn index_of(&self, row: usize) -> usize {
        if self.reverse {
            self.texts.len() - 1 - row
        } else {
            row
        }
    }
}

impl PartialEq for RowTexts {
    fn eq(&self, other: &RowTexts) -> bool {
        self.reverse == other.reverse && self.texts == other.texts
    }
}

impl Rows for RowTexts {
    fn count(&self) -> usize {
        self.texts.len()
    }

    fn text(&self, row: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.texts.get(row).map_or("", String::as_str))
    }
}

impl ItemTexts for RowTexts {
    fn count(&self) -> usize {
        self.texts.len()
    }

    fn text(&self, index: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.texts.get(index).map_or("", String::as_str))
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}
