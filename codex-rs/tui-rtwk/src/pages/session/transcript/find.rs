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
use std::cell::RefCell;
use std::rc::Rc;

use codex_tui_core::threads::{ItemKey, ThreadModel};
use rtwk::gestures::SelectionPoint;
use rtwk::render::RenderId;
use rtwk::scroll::ItemTexts;

use crate::widgets::finder::search;
pub use crate::widgets::finder::{Direction, Finder, Hit, Key, Mode, QUERY_BYTES, Rows, fold, key};
use crate::widgets::thread_ui::ThreadUi;

/// The next match of `query` (folded, as the finder holds it: `finder::fold`) from `from`, a row
/// and a byte of its text, in `direction`, never wrapping: select mode's `n` and `N` look from the
/// caret, where the finder's own step goes on from its match. The finder's scan
/// (`widgets::finder::search`) over the rows of the transcript.
pub fn repeat(
    rows: &RowTexts,
    query: &str,
    from: (usize, usize),
    direction: Direction,
) -> Option<Hit> {
    search(rows, query, from, direction)
}

/// The thread as it was when Find landed on a match in its live tail (oracle `held_reading`: the
/// view keeps the snapshot of the cells the match was found in, so the live match stays displayed
/// while the answer grows, commits or is replaced). The pane draws and reads this thread in place
/// of the live one until the reader steps to another match, the query changes or Find closes.
/// The model is shared (`Rc`), so the snapshot costs what the model's `make_mut` copies. Held by
/// the thread's finder (`Finder::held`).
#[derive(Clone, Default)]
pub struct Held(Rc<RefCell<Option<Snapshot>>>);

/// The held thread, and how many times the reader had moved the view when it was taken.
type Snapshot = (Rc<ThreadModel>, u64);

impl Held {
    /// The thread's, held by its finder.
    pub fn of(finder: &Finder) -> Held {
        finder.held(Held::default)
    }

    /// The thread as it was matched, if a live match is held.
    pub fn thread(&self) -> Option<Rc<ThreadModel>> {
        self.0.borrow().as_ref().map(|(thread, _)| thread.clone())
    }

    /// The match is in the live tail of `thread`, and the reader had moved the view `moves`
    /// times.
    pub fn set(&self, thread: Rc<ThreadModel>, moves: u64) {
        *self.0.borrow_mut() = Some((thread, moves));
    }

    /// The reader moved the view since the match was held: what they page to is the transcript as
    /// it is.
    pub fn moved(&self, moves: u64) -> bool {
        self.0
            .borrow()
            .as_ref()
            .is_some_and(|(_, held)| *held != moves)
    }

    /// Nothing is held: the next frame reads the thread as it is.
    pub fn clear(&self) {
        self.0.borrow_mut().take();
    }
}

/// A mutable tool's revision is held while keyboard selection reads it.
#[derive(Clone, Default)]
pub(in crate::pages::session) struct SelectedTool(Rc<RefCell<Option<Rc<ThreadModel>>>>);

impl SelectedTool {
    pub(in crate::pages::session) fn of(finder: &Finder) -> Self {
        finder.held(Self::default)
    }

    pub(in crate::pages::session) fn read(
        &self,
        live: &Rc<ThreadModel>,
        selecting: bool,
    ) -> Option<Rc<ThreadModel>> {
        use codex_tui_core::threads::{Item, ToolStatus};

        let mut selected = self.0.borrow_mut();
        if !selecting {
            selected.take();
            return None;
        }
        let transcript = live.transcript();
        // New rows at the tail do not change the selected revision. A
        // prepend or replacement changes the first item and lets paging work
        // on the new physical list.
        if selected.as_ref().is_some_and(|before| {
            before.id() != live.id()
                || before.transcript().get(0).map(|(key, _)| key)
                    != transcript.get(0).map(|(key, _)| key)
        }) {
            selected.take();
        }
        if selected.is_none()
            && transcript.get(transcript.len().saturating_sub(1))
                .is_some_and(|(_, item)| matches!(item, Item::Tool(tool) if tool.status == ToolStatus::Running))
        {
            *selected = Some(live.clone());
        }
        selected.clone()
    }
}

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
    /// The first item's key is the row identity used by the list. The session header has no item.
    keys: Vec<Option<ItemKey>>,
    /// Per row, per line of its text: the line is a gutter (a bullet, a prompt mark) the select
    /// mode's caret never stops on and a copy leaves out. Empty when no row has one.
    gutters: Vec<Vec<bool>>,
    /// Where each row starts in the text of the list (the rows are joined by a line break).
    starts: Vec<usize>,
    reverse: bool,
    revision: u64,
}

impl RowTexts {
    pub fn new(
        texts: Vec<String>,
        keys: Vec<Option<ItemKey>>,
        reverse: bool,
        revision: u64,
    ) -> RowTexts {
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
            keys,
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

    /// The transcript item whose row this is, if the row has one.
    pub(super) fn item_key(&self, row: usize) -> Option<&ItemKey> {
        self.keys.get(row)?.as_ref()
    }

    /// The current logical row of this item, following any older history that joined.
    pub(super) fn row_of_item(&self, key: &ItemKey) -> Option<usize> {
        self.keys.iter().position(|item| item.as_ref() == Some(key))
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
        self.reverse == other.reverse
            && self.texts == other.texts
            && self.keys == other.keys
            && self.gutters == other.gutters
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
