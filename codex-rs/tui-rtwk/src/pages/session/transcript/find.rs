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
//! none, so nothing is found in it.
//!
//! Where the oracle reads one entry per frame, bounded, the scan here reads the whole transcript
//! in the frame the query changed: the rows are in memory and a literal scan of them is the
//! cost of laying out one screen.

use std::borrow::Cow;

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
    reverse: bool,
    revision: u64,
}

impl RowTexts {
    pub fn new(texts: Vec<String>, reverse: bool, revision: u64) -> RowTexts {
        RowTexts {
            texts,
            reverse,
            revision,
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
