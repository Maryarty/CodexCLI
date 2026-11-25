//! The finder: the incremental literal search over a transcript, its one match and the position
//! it came from (oracle `transcript_view/search.rs`).
//!
//! The search is a controller (Flutter's `ValueNotifier<FindState>` of a `SearchDelegate`), one
//! per thread's `ThreadUi` (`ThreadUi::find`; `Input::Find` is what it holds while it is open):
//! the page's key action begins it, the footer's field edits its query and the keys of the footer
//! step and end it, the transcript's pane reads its match and runs the scan the query asked for.
//! It reads rows as text through `Rows`; the transcript names its own (`pages::session::
//! transcript::find`, which is also where the finder keeps the names it has always had), and the
//! footer's words are `find_footer`'s.
//!
//! Where the oracle reads one entry per frame, bounded, the scan here reads the whole transcript
//! in the frame the query changed: the rows are in memory and a literal scan of them is the
//! cost of laying out one screen.

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use rtwk::editing::{TextBuffer, TextEditingController};
use rtwk::focus::{FocusNode, KeyCombo, KeyPress};
use rtwk::foundation::ValueNotifier;

use super::thread_ui::{Follow, TextSelection};

/// Where the search stands (oracle `search.rs` `Progress`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Nothing asked since the query last changed, or the answer is shown.
    Idle,
    /// The query changed: the scan starts again.
    Restart,
    /// Entries are being read.
    Scanning,
    /// The match lies in history that has not been loaded.
    AwaitingHistory,
    /// A match is shown.
    Found,
    /// No entry is left to read in the direction.
    Exhausted,
}

/// What the status row is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub progress: Progress,
    /// The reader has accepted the match and reads it (the footer says `Find · …`).
    pub reading: bool,
    /// A match is the current one.
    pub has_match: bool,
    /// The query editor is empty.
    pub query_empty: bool,
    /// The query was cut to 4 KiB.
    pub truncated: bool,
    /// Loading older history failed.
    pub history_failed: bool,
}

impl Status {
    /// A finder that has not been asked anything.
    pub fn idle() -> Status {
        Status {
            progress: Progress::Idle,
            reading: false,
            has_match: false,
            query_empty: true,
            truncated: false,
            history_failed: false,
        }
    }
}

/// The longest query (`QUERY_BYTES`): an edit past it is undone and says so.
pub const QUERY_BYTES: usize = 4096;

/// Which way a step reads from the match: towards the oldest row or the newest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Older,
    Newer,
}

/// How far Find is: not open, the query being edited (the footer's field has the keys), or a match
/// accepted and read (the composer has the keys again; `SearchMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Closed,
    Editing,
    Reading,
}

/// A match: bytes `range` of the text of row `row` (oldest first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub row: usize,
    pub range: Range<usize>,
}

/// The rows the search reads, oldest first.
pub trait Rows {
    fn count(&self) -> usize;
    fn text(&self, row: usize) -> Cow<'_, str>;
}

/// What the reader asked of the scan since it last ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Want {
    Nothing,
    /// The query changed (or Find just began): back to where the reader was, then from the newest.
    Restart,
    /// `Ctrl+P` or `Ctrl+N`: on from the match.
    Step(Direction),
}

/// Where the reader was when Find began: what cancelling puts back.
#[derive(Clone, Copy, Debug)]
struct Saved {
    at_tail: bool,
    pixels: i64,
}

struct Data {
    mode: Mode,
    progress: Progress,
    want: Want,
    /// The query, lowercased: what the scan reads.
    folded: String,
    current: Option<Hit>,
    /// The match the pane has not shown yet.
    unshown: bool,
    /// The selection area holds the match.
    selected: bool,
    truncated: bool,
    history_failed: bool,
    saved: Option<Saved>,
    follow: Option<Follow>,
    selection: Option<TextSelection>,
}

struct Inner {
    data: RefCell<Data>,
    query: TextEditingController,
    node: FocusNode,
    /// What the field was given as the text it holds when the query last changed: an edit of the
    /// field that is not a change of the query (the caret) does not restart the scan.
    seen: RefCell<String>,
    ticks: Rc<ValueNotifier<u32>>,
    tell: Cell<bool>,
    /// The controllers of the thread that keep pace with Find's input (`Finder::held`).
    held: RefCell<Vec<(TypeId, Box<dyn Any>)>>,
}

/// The finder of one thread's transcript: a handle that is equal to its clones.
#[derive(Clone)]
pub struct Finder(Rc<Inner>);

impl PartialEq for Finder {
    fn eq(&self, other: &Finder) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Finder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.0.data.borrow();
        f.debug_struct("Finder")
            .field("mode", &data.mode)
            .field("progress", &data.progress)
            .field("current", &data.current)
            .finish()
    }
}

impl Default for Finder {
    fn default() -> Finder {
        Finder::new()
    }
}

impl Finder {
    fn new() -> Finder {
        Finder(Rc::new(Inner {
            data: RefCell::new(Data {
                mode: Mode::Closed,
                progress: Progress::Idle,
                want: Want::Nothing,
                folded: String::new(),
                current: None,
                unshown: false,
                selected: false,
                truncated: false,
                history_failed: false,
                saved: None,
                follow: None,
                selection: None,
            }),
            query: TextEditingController::new(""),
            node: FocusNode::new(),
            seen: RefCell::default(),
            ticks: Rc::new(ValueNotifier::new(0)),
            tell: Cell::new(true),
            held: RefCell::default(),
        }))
    }

    /// A controller of this thread's transcript that begins and ends with Find's input (the
    /// select and copy modes), made by `make` on the first ask and held as long as the finder is,
    /// which is as long as the thread's `ThreadUi` (`ThreadUi::held`): no registry outside it.
    pub fn held<T: Clone + 'static>(&self, make: impl FnOnce() -> T) -> T {
        let id = TypeId::of::<T>();
        let found = self
            .0
            .held
            .borrow()
            .iter()
            .find(|(held, _)| *held == id)
            .and_then(|(_, held)| held.downcast_ref::<T>().cloned());
        if let Some(found) = found {
            return found;
        }
        // `make` may ask for another controller: the borrow is over
        let made = make();
        self.0.held.borrow_mut().push((id, Box::new(made.clone())));
        made
    }

    /// Ticks when anything the footer or the pane shows changes.
    pub fn listenable(&self) -> Rc<ValueNotifier<u32>> {
        self.0.ticks.clone()
    }

    fn notify(&self) {
        if self.0.tell.get() {
            self.0.ticks.set_value(self.0.ticks.value().wrapping_add(1));
        }
    }

    pub fn mode(&self) -> Mode {
        self.0.data.borrow().mode
    }

    /// Find is open (editing or reading).
    pub fn active(&self) -> bool {
        self.mode() != Mode::Closed
    }

    /// A match is accepted and read: the page's keys (`Ctrl+P`, `Ctrl+N`, Esc) are Find's.
    pub fn reading(&self) -> bool {
        self.mode() == Mode::Reading
    }

    /// The query's editor, which the footer's field edits.
    pub fn query(&self) -> TextEditingController {
        self.0.query.clone()
    }

    /// The field's focus.
    pub fn node(&self) -> FocusNode {
        self.0.node.clone()
    }

    /// What the footer's words are read from.
    pub fn status(&self) -> Status {
        let data = self.0.data.borrow();
        Status {
            progress: data.progress,
            reading: data.mode == Mode::Reading,
            has_match: data.current.is_some(),
            query_empty: data.folded.is_empty(),
            truncated: data.truncated,
            history_failed: data.history_failed,
        }
    }

    /// The match, if there is one.
    pub fn current(&self) -> Option<Hit> {
        self.0.data.borrow().current.clone()
    }

    /// F3 (`begin_search`): the query is edited. What the reader sees now is what cancelling
    /// brings back; a match that is being read is kept while the query is edited again.
    pub fn begin(&self, follow: &Follow, selection: &TextSelection) {
        if self.mode() == Mode::Editing {
            return;
        }
        {
            let mut data = self.0.data.borrow_mut();
            let reading = data.mode == Mode::Reading;
            data.mode = Mode::Editing;
            data.follow = Some(follow.clone());
            data.selection = Some(selection.clone());
            if !reading || data.current.is_none() {
                data.want = Want::Restart;
                data.progress = Progress::Restart;
            }
            if !reading {
                data.saved = Some(saved(follow));
            }
        }
        self.notify();
    }

    /// The field's text changed: the query is the new text, and the scan starts again.
    pub fn changed(&self) {
        let text = self.0.query.text();
        if *self.0.seen.borrow() == *text {
            return;
        }
        let text = if text.len() > QUERY_BYTES {
            // an edit past the limit is undone (`QUERY_BYTES` in `handle_search_key`)
            let kept = self.0.seen.borrow().clone();
            self.0.data.borrow_mut().truncated = true;
            self.set_query(&kept);
            self.notify();
            return;
        } else {
            text
        };
        *self.0.seen.borrow_mut() = text.to_string();
        {
            let mut data = self.0.data.borrow_mut();
            data.folded = fold(&text);
            data.current = None;
            data.unshown = false;
            data.want = Want::Restart;
            data.progress = Progress::Restart;
            data.truncated = false;
        }
        self.notify();
    }

    fn set_query(&self, text: &str) {
        *self.0.seen.borrow_mut() = text.to_owned();
        self.0.tell.set(false);
        self.0.query.set_text(text);
        self.0.tell.set(true);
    }

    /// A paste into the query (`paste_search`): line breaks are plain `\n`, and what does not fit
    /// within the limit, by whole graphemes, is left out and said.
    pub fn paste(&self, text: &str) {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let remaining = QUERY_BYTES.saturating_sub(self.0.query.text().len());
        let end = fitting(&normalized, remaining);
        if end > 0 {
            self.0.query.insert(&normalized[..end]);
        }
        self.0.data.borrow_mut().truncated = end < normalized.len();
        self.changed();
        if end < normalized.len() {
            self.0.data.borrow_mut().truncated = true;
            self.notify();
        }
    }

    /// Enter on a query: the match shown is accepted and read. An empty query, or one nothing is
    /// found for yet, stays open (`handle_search_key`).
    pub fn accept(&self) -> bool {
        {
            let mut data = self.0.data.borrow_mut();
            if data.mode != Mode::Editing || data.folded.is_empty() {
                return false;
            }
            if data.current.is_none() && !matches!(data.progress, Progress::Idle) {
                return false;
            }
            data.mode = Mode::Reading;
            data.progress = match data.progress {
                Progress::Exhausted => Progress::Exhausted,
                _ if data.current.is_some() => Progress::Found,
                _ => Progress::Idle,
            };
        }
        self.notify();
        true
    }

    /// `Ctrl+P` and `Ctrl+N`: the next match from this one, in `direction`.
    pub fn step(&self, direction: Direction) {
        {
            let mut data = self.0.data.borrow_mut();
            if data.mode == Mode::Closed || data.folded.is_empty() {
                return;
            }
            data.want = Want::Step(direction);
            data.progress = Progress::Scanning;
        }
        self.notify();
    }

    /// Esc while the query is edited, `Ctrl+C` anywhere: Find closes and the reader is where they
    /// were when it began (`cancel_search`).
    pub fn cancel(&self) {
        let (saved, follow) = {
            let data = self.0.data.borrow();
            if data.mode == Mode::Closed {
                return;
            }
            (data.saved, data.follow.clone())
        };
        self.close();
        if let (Some(saved), Some(follow)) = (saved, follow) {
            put_back(&follow, saved);
        }
    }

    /// Esc while a match is read: Find closes and the reader is back at the last row
    /// (`jump_to_latest`).
    pub fn latest(&self) {
        let follow = self.0.data.borrow().follow.clone();
        self.close();
        if let Some(follow) = follow {
            follow.jump_to_latest();
        }
    }

    /// Find closes: no query, no match, nothing selected.
    fn close(&self) {
        let selection = {
            let mut data = self.0.data.borrow_mut();
            data.mode = Mode::Closed;
            data.progress = Progress::Idle;
            data.want = Want::Nothing;
            data.folded.clear();
            data.current = None;
            data.unshown = false;
            data.truncated = false;
            data.history_failed = false;
            data.saved = None;
            let held = std::mem::take(&mut data.selected);
            data.selection.take().filter(|_| held)
        };
        self.set_query("");
        if let Some(selection) = selection {
            selection.controller().clear();
        }
        self.notify();
    }

    /// Runs the scan the reader asked for over `rows`, and tells the match it found, once: the
    /// pane scrolls to it and selects it. A scan that finds none leaves the status saying so.
    pub fn advance(&self, rows: &dyn Rows) -> Option<Hit> {
        let asked = {
            let data = self.0.data.borrow();
            (data.mode != Mode::Closed && data.want != Want::Nothing).then(|| {
                (
                    data.want,
                    data.folded.clone(),
                    data.current.clone(),
                    data.follow.clone(),
                    data.saved,
                )
            })
        };
        let Some((want, folded, current, follow, saved)) = asked else {
            return self.take_unshown();
        };
        let count = rows.count();
        let found = match want {
            Want::Nothing => return None,
            Want::Restart => {
                // a refined query leaves no old match at the top: the reader goes back first
                if let (Some(follow), Some(saved)) = (&follow, saved) {
                    put_back(follow, saved);
                }
                if folded.is_empty() {
                    let mut data = self.0.data.borrow_mut();
                    data.want = Want::Nothing;
                    data.progress = Progress::Idle;
                    data.selected = false;
                    drop(data);
                    self.notify();
                    return None;
                }
                search(
                    rows,
                    &folded,
                    (count.saturating_sub(1), usize::MAX),
                    Direction::Older,
                )
            }
            Want::Step(direction) => {
                let from = match (&current, direction) {
                    (Some(hit), Direction::Older) => (hit.row, hit.range.start),
                    (Some(hit), Direction::Newer) => (hit.row, hit.range.end),
                    (None, Direction::Older) => (count.saturating_sub(1), usize::MAX),
                    (None, Direction::Newer) => (0, 0),
                };
                search(rows, &folded, from, direction)
            }
        };
        {
            let mut data = self.0.data.borrow_mut();
            data.want = Want::Nothing;
            match found {
                Some(hit) => {
                    data.current = Some(hit);
                    data.progress = Progress::Found;
                    data.unshown = true;
                }
                None => {
                    data.progress = Progress::Exhausted;
                    if want == Want::Restart {
                        data.current = None;
                    }
                }
            }
        }
        self.notify();
        self.take_unshown()
    }

    fn take_unshown(&self) -> Option<Hit> {
        let mut data = self.0.data.borrow_mut();
        if std::mem::take(&mut data.unshown) {
            data.selected = true;
            data.current.clone()
        } else {
            None
        }
    }
}

/// Where the reader is now.
fn saved(follow: &Follow) -> Saved {
    let pixels = follow
        .scroll()
        .as_scroll_controller()
        .try_position()
        .map(|position| position.pixels())
        .unwrap_or(0);
    Saved {
        at_tail: follow.at_tail(),
        pixels,
    }
}

/// The reader goes back to where `saved` was.
fn put_back(follow: &Follow, saved: Saved) {
    if saved.at_tail {
        follow.jump_to_latest();
    } else {
        follow.scroll().as_scroll_controller().jump_to(saved.pixels);
    }
}

/// The lowercase of `text`: what the query and the rows are compared as (`search` takes the
/// query folded).
pub fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// The longest prefix of `text` within `limit` bytes that ends between two graphemes.
fn fitting(text: &str, limit: usize) -> usize {
    let buffer = TextBuffer::new(text);
    let mut end = 0;
    while end < text.len() {
        let next = buffer.character_end_after(end);
        if next <= end || next > limit {
            break;
        }
        end = next;
    }
    end
}

/// The first match of the folded `query` in `text` from `direction` (the last when older), as
/// bytes of `text` itself: the lowercase of a character can be longer or shorter than it, so the
/// match is mapped back to the characters it covers (`find_literal`).
fn find_literal(text: &str, query: &str, direction: Direction) -> Option<Range<usize>> {
    let mut folded = String::new();
    let mut spans = Vec::new();
    for (start, character) in text.char_indices() {
        for lowercase in character.to_lowercase() {
            folded.push(lowercase);
            spans.push((folded.len(), start..start + character.len_utf8()));
        }
    }
    let start = match direction {
        Direction::Older => folded.rfind(query),
        Direction::Newer => folded.find(query),
    }?;
    let first = spans.partition_point(|(end, _)| *end <= start);
    let last = spans.partition_point(|(end, _)| *end < start + query.len());
    Some(spans.get(first)?.1.start..spans.get(last)?.1.end)
}

/// The next match from `from` (a row and a byte of its text) in `direction`, never wrapping: in
/// the row itself before (older) or after (newer) the byte, then in each row beyond. The query is
/// folded (`fold`); the scan is the finder's own and select mode's `n`/`N` (`find::repeat`).
pub fn search(
    rows: &dyn Rows,
    query: &str,
    from: (usize, usize),
    direction: Direction,
) -> Option<Hit> {
    let count = rows.count();
    if count == 0 || query.is_empty() {
        return None;
    }
    let first = from.0.min(count - 1);
    let mut row = first;
    loop {
        let text = rows.text(row);
        let window = if row == first {
            let at = floor_boundary(&text, from.1);
            match direction {
                Direction::Older => 0..at,
                Direction::Newer => at..text.len(),
            }
        } else {
            0..text.len()
        };
        if let Some(found) = find_literal(&text[window.clone()], query, direction) {
            return Some(Hit {
                row,
                range: window.start + found.start..window.start + found.end,
            });
        }
        row = match direction {
            Direction::Older => row.checked_sub(1)?,
            Direction::Newer => (row + 1 < count).then_some(row + 1)?,
        };
    }
}

fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// What a key means to Find, wherever it is read (editing or reading).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Enter: accept the match shown (editing); while reading, swallowed.
    Accept,
    /// Esc (editing) and `Ctrl+C`: close, back where the reader was.
    Cancel,
    /// Esc while reading: close, the reader at the last row.
    Latest,
    Older,
    Newer,
}

const CHORDS: [(&str, Key); 4] = [
    ("enter", Key::Accept),
    ("ctrl+c", Key::Cancel),
    ("ctrl+p", Key::Older),
    ("ctrl+n", Key::Newer),
];

/// What `key` does to a Find in `mode`, if it does anything.
pub fn key(mode: Mode, key: &KeyPress) -> Option<Key> {
    if mode == Mode::Closed {
        return None;
    }
    if KeyCombo::parse("esc").is_ok_and(|combo| combo.accepts_press(key)) {
        return Some(if mode == Mode::Reading {
            Key::Latest
        } else {
            Key::Cancel
        });
    }
    CHORDS.iter().find_map(|(spec, step)| {
        KeyCombo::parse(spec)
            .ok()
            .filter(|combo| combo.accepts_press(key))
            .map(|_| *step)
    })
}

impl Finder {
    /// What `taken` does: the one place the keys of Find act (the footer's field, and the page
    /// while a match is read). Whether the key was Find's.
    pub fn take(&self, taken: Key) -> bool {
        match taken {
            Key::Accept => {
                if self.mode() == Mode::Reading {
                    // a held Enter must not reach the composer's draft
                    return true;
                }
                self.accept();
            }
            Key::Cancel => self.cancel(),
            Key::Latest => self.latest(),
            Key::Older => self.step(Direction::Older),
            Key::Newer => self.step(Direction::Newer),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Vec<&'static str>);

    impl Rows for Fixed {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn text(&self, row: usize) -> Cow<'_, str> {
            Cow::Borrowed(self.0[row])
        }
    }

    #[test]
    fn a_match_is_found_without_regard_to_case_and_mapped_to_the_source() {
        assert_eq!(
            find_literal("The NEEDLE", "needle", Direction::Newer),
            Some(4..10)
        );
        // `İ` lowercases to two characters; the match covers the one it comes from
        assert_eq!(find_literal("İx", "x", Direction::Newer), Some(2..3));
    }

    #[test]
    fn the_scan_goes_from_the_newest_and_stops_at_the_ends() {
        let rows = Fixed(vec!["USER] a", "USER] b", "USER] c"]);
        let first = search(&rows, "user]", (2, usize::MAX), Direction::Older).expect("newest");
        assert_eq!(first.row, 2);
        let older = search(
            &rows,
            "user]",
            (first.row, first.range.start),
            Direction::Older,
        )
        .expect("older");
        assert_eq!(older.row, 1);
        let oldest = search(&rows, "user]", (0, 0), Direction::Older);
        assert_eq!(oldest, None);
        let newer = search(&rows, "user]", (1, 7), Direction::Newer).expect("newer");
        assert_eq!(newer.row, 2);
        assert_eq!(search(&rows, "user]", (2, 7), Direction::Newer), None);
    }

    #[test]
    fn the_second_match_of_a_row_follows_the_first() {
        let rows = Fixed(vec!["ab ab"]);
        let a = search(&rows, "ab", (0, 0), Direction::Newer).expect("first");
        assert_eq!(a.range, 0..2);
        let b = search(&rows, "ab", (0, a.range.end), Direction::Newer).expect("second");
        assert_eq!(b.range, 3..5);
        let back = search(&rows, "ab", (0, b.range.start), Direction::Older).expect("back");
        assert_eq!(back.range, 0..2);
    }
}
