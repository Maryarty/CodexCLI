//! Select mode (oracle `transcript_view/select_mode.rs`, `select_mode/motion.rs`): a caret that
//! the keyboard moves over the text of the transcript, a mark that starts a range at it, and the
//! keys that copy the range or let it go. `leader [` (the keymap's `select_transcript`) begins it.
//!
//! The model is a controller (Flutter's `ValueNotifier<SelectState>`), one per thread, held by
//! the thread's `ThreadUi` (`ThreadUi::held`; `of` finds it from the finder that travels with
//! the thread's controllers): the key action begins it, the page's key handler moves it (`key`), the pane gives
//! it the rows (`RowTexts`, the transcript as the list's selection reads it) and shows it by
//! asking the selection area for the caret's grapheme, or the marked range (`show`), and the
//! footer says what the keys do (`hints`). Nothing here is drawn: the area paints the range
//! REVERSED as the oracle does, and the caret's cell reversed, bold and underlined as a decoration
//! (`decorate`; the oracle's `render_select_cursor`).
//!
//! Positions are a row of the list (oldest first) and a byte of its text; a caret stops on the
//! lines of the text that are not gutters. The oracle moves over display rows (a wrapped line is
//! several): this moves over the lines of the text, a gap row of the pack. A page is the rows in
//! view of the list, less one (`area.height - 1`), and half a page half of them, rounded up; the
//! pager's keys (rebound or not) move the caret where the mode's own do not name them.
//!
//! A Find accepted in the mode closes and leaves its query here (the oracle keeps it open and
//! reading, and shows the mode's footer all the same): `n` looks for the match before the caret,
//! `N` for the one after the grapheme it is on, and the caret goes to it (`find::repeat`).

#[path = "select/keyboard.rs"]
mod keyboard;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::editing::TextBuffer;
use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::foundation::ValueNotifier;
use rtwk::gestures::{SelectionDecoration, SelectionTarget};
use rtwk::render::{Pen, RenderId};
use rtwk::style::{Attrs, Style};

use codex_tui_core::keymap::actions::{
    GLOBAL_SELECT_TRANSCRIPT, PAGER_HALF_PAGE_DOWN, PAGER_HALF_PAGE_UP, PAGER_JUMP_BOTTOM,
    PAGER_JUMP_TOP, PAGER_PAGE_DOWN, PAGER_PAGE_UP, PAGER_SCROLL_DOWN, PAGER_SCROLL_UP,
};
use codex_tui_core::keymap::{ActionId, Keymap};

use super::TranscriptMode;
use super::find::{self, Direction, Finder, RowTexts};
use crate::keymap::{chords_of, pressed};
use crate::widgets::thread_ui::{Follow, TextSelection};

/// A place in the transcript: a row of the list (oldest first) and a byte of its text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub offset: usize,
}

/// What a mark selects by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    /// `v` and Space: from the mark through the grapheme the caret is on.
    Character,
    /// `V`: whole lines.
    Line,
}

#[derive(Clone, Copy, Debug)]
struct Mark {
    anchor: Pos,
    unit: Unit,
}

/// One line of the text a caret can stop on.
#[derive(Clone, Copy, Debug)]
struct Line {
    row: usize,
    start: usize,
    end: usize,
}

impl Line {
    fn empty(&self) -> bool {
        self.start == self.end
    }
}

/// What the page does about a key of select mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The model moved: nothing else to do.
    Moved,
    /// `y` and the copy chords: copy what is marked.
    Copy,
    /// Enter with a mark: copy it and go to the last row once it is delivered.
    CopyAndFollow,
    /// Enter without a mark, `q`, Esc without a mark: the mode ends.
    Exit,
    /// `/`: Find opens over the transcript.
    Find,
}

/// What a key means in select mode, before the caret's text is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Do {
    Copy,
    Enter,
    Esc,
    Quit,
    Find,
    Mark(Unit),
    Swap,
    Repeat(bool),
    Extend(Motion),
    Go(Motion),
    /// A page of the list's height, less one row, towards the sign's end.
    Page(isize),
    /// Half the list's height, rounded up.
    Half(isize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Left,
    Right,
    Rows(isize),
    WordForward,
    WordBackward,
    WordEnd,
    LineStart,
    LineEnd,
    Beginning,
    End,
}

fn table() -> Vec<(&'static str, Do)> {
    use Motion::{Beginning, End, LineEnd, LineStart, Rows, WordBackward, WordEnd, WordForward};
    vec![
        ("ctrl+c", Do::Copy),
        ("super+c", Do::Copy),
        ("ctrl+shift+c", Do::Copy),
        ("ctrl+insert", Do::Copy),
        ("y", Do::Copy),
        ("enter", Do::Enter),
        ("esc", Do::Esc),
        ("q", Do::Quit),
        ("/", Do::Find),
        ("v", Do::Mark(Unit::Character)),
        ("space", Do::Mark(Unit::Character)),
        ("shift+v", Do::Mark(Unit::Line)),
        ("V", Do::Mark(Unit::Line)),
        ("o", Do::Swap),
        ("n", Do::Repeat(false)),
        ("shift+n", Do::Repeat(true)),
        ("N", Do::Repeat(true)),
        ("shift+left", Do::Extend(Motion::Left)),
        ("shift+right", Do::Extend(Motion::Right)),
        ("shift+up", Do::Extend(Rows(-1))),
        ("shift+down", Do::Extend(Rows(1))),
        ("left", Do::Go(Motion::Left)),
        ("right", Do::Go(Motion::Right)),
        ("up", Do::Go(Rows(-1))),
        ("down", Do::Go(Rows(1))),
        ("h", Do::Go(Motion::Left)),
        ("l", Do::Go(Motion::Right)),
        ("k", Do::Go(Rows(-1))),
        ("j", Do::Go(Rows(1))),
        ("w", Do::Go(WordForward)),
        ("b", Do::Go(WordBackward)),
        ("e", Do::Go(WordEnd)),
        ("home", Do::Go(LineStart)),
        ("0", Do::Go(LineStart)),
        ("end", Do::Go(LineEnd)),
        ("$", Do::Go(LineEnd)),
        ("shift+$", Do::Go(LineEnd)),
        ("pageup", Do::Page(-1)),
        ("ctrl+b", Do::Page(-1)),
        ("pagedown", Do::Page(1)),
        ("ctrl+f", Do::Page(1)),
        ("ctrl+u", Do::Half(-1)),
        ("ctrl+d", Do::Half(1)),
        ("g", Do::Go(Beginning)),
        ("ctrl+home", Do::Go(Beginning)),
        ("shift+g", Do::Go(End)),
        ("G", Do::Go(End)),
        ("ctrl+end", Do::Go(End)),
    ]
}

struct Data {
    active: bool,
    keyboard: Option<keyboard::Range>,
    /// The caret has not been placed: the pane does it with the first rows it reads.
    placed: bool,
    /// The first frame of select mode can take over the native area's range.
    adopt: bool,
    cursor: Pos,
    mark: Option<Mark>,
    /// The display column up and down keep.
    column: Option<usize>,
    follow: Option<Follow>,
    rows: Option<Rc<RowTexts>>,
    /// What the area was last asked to select, and the revision of the rows it was asked on.
    shown: Option<(Pos, Pos, u64)>,
    /// The caret's cell the area was last asked to decorate: where it starts, where it ends, and
    /// the revision of the rows it was asked on.
    decorated: Option<(Pos, usize, u64)>,
    /// The list's render object, once a selection of it has told it.
    list: Option<RenderId>,
    /// The caret moved: the pane scrolls it into view.
    reveal: bool,
    watched: bool,
    /// The pane refused the mode (nothing selectable); the host that held it reads this once.
    refused: bool,
    /// The query of the Find that was accepted in this mode, lowercased: what `n` and `N` look for
    /// (the oracle keeps that Find open and reading; here it closes and leaves its query).
    query: String,
    /// `g` asked for the first row while older pages are still to come
    /// (`TranscriptHistoryState::LoadingBeginning`): the caret goes to the first row once the last
    /// page is in; any other key of the mode ends the wait.
    beginning: bool,
}

struct Inner {
    data: RefCell<Data>,
    ticks: Rc<ValueNotifier<u32>>,
    /// The area's listener is ours: it is not asked again.
    attached: Cell<bool>,
}

/// The select mode of one thread's transcript: a handle that is equal to its clones.
#[derive(Clone)]
pub struct Select(Rc<Inner>);

impl PartialEq for Select {
    fn eq(&self, other: &Select) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for Select {
    fn default() -> Select {
        Select(Rc::new(Inner {
            data: RefCell::new(Data {
                active: false,
                keyboard: None,
                placed: false,
                adopt: false,
                cursor: Pos::default(),
                mark: None,
                column: None,
                follow: None,
                rows: None,
                shown: None,
                decorated: None,
                list: None,
                reveal: false,
                watched: false,
                refused: false,
                query: String::new(),
                beginning: false,
            }),
            ticks: Rc::new(ValueNotifier::new(0)),
            attached: Cell::new(false),
        }))
    }
}

/// The select mode of the thread whose finder this is (made on first ask, held with the thread's
/// controllers: `ThreadUi::held`).
pub fn of(finder: &Finder) -> Select {
    finder.held(Select::default)
}

/// What `key` does in select mode, if it does anything (the table of `handle_select_mode_key`).
fn what(key: &KeyPress) -> Option<Do> {
    table().into_iter().find_map(|(spec, what)| {
        KeyCombo::parse(spec)
            .ok()
            .filter(|combo| combo.accepts_press(key))
            .map(|_| what)
    })
}

/// What a key does of the chord that begins select mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    /// The leader: the next key may complete the chord.
    Waiting,
    /// The chord is complete: `select_transcript` (`begin_select_mode`).
    Begin,
}

/// `select_transcript` is a chord (the leader, then `[` by default): `leader` is the flag the
/// host keeps between the two keys. `None` where the key is no part of it.
pub fn chord(keymap: &Keymap, leader: &Cell<bool>, key: &KeyPress) -> Option<Chord> {
    let chords = chords_of(keymap, GLOBAL_SELECT_TRANSCRIPT);
    if leader.replace(false) {
        chords
            .iter()
            .any(|(_, done)| done.accepts_press(key))
            .then_some(Chord::Begin)
    } else if chords.iter().any(|(prefix, _)| prefix.accepts_press(key)) {
        leader.set(true);
        Some(Chord::Waiting)
    } else {
        None
    }
}

/// What the pager's keys (`keymap.pager`: rebound or not) do in select mode: they move the caret
/// where the mode's own keys do not name them (`pager_motion`).
fn pager(keymap: &Keymap, key: &KeyPress) -> Option<Do> {
    let is = |id: ActionId| pressed(keymap, id, key);
    [
        (PAGER_JUMP_TOP, Do::Go(Motion::Beginning)),
        (PAGER_JUMP_BOTTOM, Do::Go(Motion::End)),
        (PAGER_SCROLL_UP, Do::Go(Motion::Rows(-1))),
        (PAGER_SCROLL_DOWN, Do::Go(Motion::Rows(1))),
        (PAGER_PAGE_UP, Do::Page(-1)),
        (PAGER_PAGE_DOWN, Do::Page(1)),
        (PAGER_HALF_PAGE_UP, Do::Half(-1)),
        (PAGER_HALF_PAGE_DOWN, Do::Half(1)),
    ]
    .into_iter()
    .find_map(|(id, what)| is(id).then_some(what))
}

impl Select {
    /// Notifies when the mode begins, ends or moves.
    pub fn listenable(&self) -> Rc<ValueNotifier<u32>> {
        self.0.ticks.clone()
    }

    fn notify(&self) {
        self.0.ticks.set_value(self.0.ticks.value().wrapping_add(1));
    }

    /// Select mode is on.
    pub fn active(&self) -> bool {
        self.0.data.borrow().active
    }

    /// Whether a range is marked.
    pub fn marked(&self) -> bool {
        self.0.data.borrow().mark.is_some()
    }

    /// `select_transcript`: the mode begins where the reader is; the caret is placed with the
    /// first rows the pane reads.
    pub fn begin(&self, follow: &Follow) {
        {
            let mut data = self.0.data.borrow_mut();
            if data.active {
                return;
            }
            data.active = true;
            data.keyboard = None;
            data.placed = false;
            data.adopt = true;
            data.list = None;
            data.mark = None;
            data.column = None;
            data.shown = None;
            data.query.clear();
            data.follow = Some(follow.clone());
        }
        self.notify();
    }

    /// The mode ends: nothing is marked and the area lets go.
    pub fn end(&self, selection: &TextSelection) {
        {
            let mut data = self.0.data.borrow_mut();
            if !data.active {
                return;
            }
            data.active = false;
            data.mark = None;
            data.shown = None;
            data.rows = None;
            data.query.clear();
        }
        selection.controller().clear();
        self.notify();
    }

    /// Whether a Find was accepted in this mode: its query is what `n` and `N` look for, and what
    /// Enter on an empty query goes back to.
    pub fn remembers(&self) -> bool {
        let data = self.0.data.borrow();
        data.active && !data.query.is_empty()
    }

    /// What the footer says: the keys of the mode, by whether a range is marked.
    pub fn hints(&self) -> Option<bool> {
        let data = self.0.data.borrow();
        data.active.then_some(data.mark.is_some())
    }

    /// The caret goes to the match of an accepted Find (`accept_search_into_select_mode`; a range
    /// that was marked stays, its active end moves), and `query` is what `n` and `N` look for.
    pub fn land(&self, hit: Pos, query: &str) {
        {
            let mut data = self.0.data.borrow_mut();
            if !data.active {
                return;
            }
            // on the grapheme the match starts in (a match of a combining mark alone), or the next
            // line's start where it began on a line break
            let at = data
                .rows
                .clone()
                .map_or(hit, |rows| place(&rows, &lines(&rows), hit));
            data.query = query.chars().flat_map(char::to_lowercase).collect();
            data.cursor = at;
            data.column = None;
            data.reveal = true;
            data.shown = None;
        }
        self.notify();
    }

    /// A key of the mode: what the page does of it, or `None` where the key is not the mode's.
    pub fn key(&self, keymap: &Keymap, key: &KeyPress) -> Option<Step> {
        if !self.active() {
            return None;
        }
        let what = what(key).or_else(|| pager(keymap, key))?;
        let rows = self.0.data.borrow().rows.clone()?;
        self.0.data.borrow_mut().beginning = false;
        let step = match what {
            Do::Copy => {
                if self.marked() {
                    Step::Copy
                } else {
                    return Some(Step::Moved);
                }
            }
            Do::Enter => {
                if self.marked() {
                    Step::CopyAndFollow
                } else {
                    Step::Exit
                }
            }
            Do::Esc => {
                if self.marked() {
                    self.clear_mark();
                    Step::Moved
                } else {
                    Step::Exit
                }
            }
            Do::Quit => Step::Exit,
            Do::Find => Step::Find,
            Do::Mark(unit) => {
                self.mark(unit);
                Step::Moved
            }
            Do::Swap => {
                self.swap();
                Step::Moved
            }
            Do::Repeat(newer) => {
                self.again(&rows, newer);
                Step::Moved
            }
            Do::Extend(motion) => {
                if !self.marked() {
                    self.mark(Unit::Character);
                }
                self.go(&rows, motion);
                Step::Moved
            }
            Do::Go(motion) => {
                self.go(&rows, motion);
                self.0.data.borrow_mut().beginning = motion == Motion::Beginning;
                Step::Moved
            }
            Do::Page(pages) => {
                let page = (self.height() - 1).max(1);
                self.go(&rows, Motion::Rows(pages * page));
                Step::Moved
            }
            Do::Half(pages) => {
                let half = ((self.height() + 1) / 2).max(1);
                self.go(&rows, Motion::Rows(pages * half));
                Step::Moved
            }
        };
        self.notify();
        Some(step)
    }

    /// A key of the mode, while Find is not asking (`handle_select_mode_key`): the model takes it
    /// and the mode does what the step says (a copy lets the mark go, Enter's copy leaves the mode
    /// with what it selected, Exit ends it, `/` opens Find from where the reader is); `host` is
    /// told the step first, for what is the host's (the copy itself, the way out). Whether the key
    /// was the mode's; a key that is not goes on.
    pub fn take(
        &self,
        keymap: &Keymap,
        key: &KeyPress,
        selection: &TextSelection,
        finder: &Finder,
        host: &dyn Fn(Step),
    ) -> bool {
        if !self.active() || finder.active() {
            return false;
        }
        let Some(step) = self.key(keymap, key) else {
            return false;
        };
        host(step);
        match step {
            Step::Moved => {}
            Step::Copy => self.copied(),
            Step::CopyAndFollow => self.leave(),
            Step::Exit => self.end(selection),
            Step::Find => {
                if let Some(follow) = self.reader() {
                    finder.begin(&follow, selection);
                }
            }
        }
        true
    }

    /// A copy was asked for: what was marked is let go, the caret stays on its grapheme.
    pub fn copied(&self) {
        self.clear_mark();
        self.notify();
    }

    fn clear_mark(&self) {
        let mut data = self.0.data.borrow_mut();
        data.mark = None;
        data.column = None;
        data.shown = None;
    }

    fn mark(&self, unit: Unit) {
        let mut data = self.0.data.borrow_mut();
        data.column = None;
        data.shown = None;
        data.mark = match data.mark {
            Some(held) if held.unit == unit => None,
            Some(held) => Some(Mark { unit, ..held }),
            None => Some(Mark {
                anchor: data.cursor,
                unit,
            }),
        };
    }

    fn swap(&self) {
        let mut data = self.0.data.borrow_mut();
        let cursor = data.cursor;
        if let Some(mark) = data.mark {
            data.cursor = mark.anchor;
            data.mark = Some(Mark {
                anchor: cursor,
                ..mark
            });
            data.column = None;
            data.shown = None;
            data.reveal = true;
        }
    }

    /// The rows in view: the list's own height, where it has been laid out (a page and half a
    /// page are `area.height` in the oracle's select mode).
    fn height(&self) -> isize {
        let follow = self.0.data.borrow().follow.clone();
        let Some(follow) = follow else {
            return 0;
        };
        let scroll = follow.scroll();
        let height = scroll
            .as_scroll_controller()
            .try_position()
            .map_or(0, |position| position.viewport_dimension());
        isize::try_from(height).unwrap_or(0)
    }

    fn go(&self, rows: &RowTexts, motion: Motion) {
        let lines = lines(rows);
        let (cursor, column) = {
            let data = self.0.data.borrow();
            (data.cursor, data.column)
        };
        let (to, keep) = moved(rows, &lines, cursor, column, motion, false);
        let mut data = self.0.data.borrow_mut();
        data.cursor = to;
        data.column = keep;
        data.shown = None;
        data.reveal = true;
    }

    /// `n` (older) and `N` (newer): the next match of the accepted Find's query from the caret
    /// (`repeat_select_search`): before it for older, after the grapheme it is on for newer, never
    /// wrapping. The caret goes to the match, a range marked going on from it.
    fn again(&self, rows: &RowTexts, newer: bool) {
        let (query, cursor) = {
            let data = self.0.data.borrow();
            (data.query.clone(), data.cursor)
        };
        if query.is_empty() {
            return;
        }
        let (from, direction) = if newer {
            let text = rows.row_text(cursor.row);
            let after = TextBuffer::new(text).character_end_after(cursor.offset.min(text.len()));
            (after, Direction::Newer)
        } else {
            (cursor.offset, Direction::Older)
        };
        let Some(hit) = find::repeat(rows, &query, (cursor.row, from), direction) else {
            return;
        };
        let to = place(
            rows,
            &lines(rows),
            Pos {
                row: hit.row,
                offset: hit.range.start,
            },
        );
        let mut data = self.0.data.borrow_mut();
        data.cursor = to;
        data.column = None;
        data.shown = None;
        data.reveal = true;
    }

    /// The pane gives the mode the rows it reads (while it is on) and asks what to show. Places
    /// the caret the first time; the caret goes to `scroll` when it moved. Whether the mode
    /// stays: it ends where nothing is selectable.
    pub fn read(&self, rows: &Rc<RowTexts>) -> bool {
        let lines = lines(rows);
        let mut data = self.0.data.borrow_mut();
        if !data.active {
            return false;
        }
        data.rows = Some(rows.clone());
        // nothing to select (no line with a character in it): `begin_select_mode` is refused
        if lines.iter().all(Line::empty) {
            data.active = false;
            data.refused = true;
            drop(data);
            self.notify();
            return false;
        }
        if !data.placed {
            data.placed = true;
            let last = lines
                .iter()
                .rev()
                .find(|line| !line.empty())
                .or(lines.last());
            if let Some(last) = last {
                data.cursor = Pos {
                    row: last.row,
                    offset: last.start,
                };
            }
            data.reveal = true;
        }
        // rows that changed under the caret: it stays on a place that is still there
        let cursor = data.cursor;
        let here = lines
            .iter()
            .any(|line| line.row == cursor.row && (line.start..=line.end).contains(&cursor.offset));
        if !here {
            let near = lines
                .iter()
                .rev()
                .find(|line| line.row <= cursor.row)
                .or(lines.first());
            if let Some(near) = near {
                data.cursor = Pos {
                    row: near.row,
                    offset: near.start,
                };
            }
        }
        true
    }

    /// `g` waits for the older pages to bring the first row.
    pub fn beginning(&self) -> bool {
        let data = self.0.data.borrow();
        data.active && data.beginning
    }

    /// The last page is in: the caret goes to the first row (`complete_select_beginning`).
    pub fn finish_beginning(&self) {
        let rows = {
            let mut data = self.0.data.borrow_mut();
            if !std::mem::take(&mut data.beginning) {
                return;
            }
            data.rows.clone()
        };
        if let Some(rows) = rows {
            self.go(&rows, Motion::Beginning);
            self.notify();
        }
    }

    /// The pages were refused: the wait for the first row ends where the caret is.
    pub fn cancel_beginning(&self) {
        self.0.data.borrow_mut().beginning = false;
    }

    /// The row the caret is on, once, when it moved (the pane scrolls it into view).
    pub fn reveal(&self) -> Option<usize> {
        let mut data = self.0.data.borrow_mut();
        std::mem::take(&mut data.reveal).then_some(data.cursor.row)
    }

    /// The caret's place, for Find to start from.
    pub fn cursor(&self) -> Pos {
        self.0.data.borrow().cursor
    }

    /// What the area is to select: the caret's grapheme, or the marked range (end exclusive).
    fn range(&self, rows: &RowTexts) -> Option<(Pos, Pos)> {
        let data = self.0.data.borrow();
        let lines = lines(rows);
        let cursor = data.cursor;
        let (lo, hi) = match data.mark {
            Some(mark) => (mark.anchor.min(cursor), mark.anchor.max(cursor)),
            None => (cursor, cursor),
        };
        let line_of = |pos: Pos| {
            lines
                .iter()
                .find(|line| line.row == pos.row && (line.start..=line.end).contains(&pos.offset))
                .copied()
        };
        let (from, last) = (line_of(lo)?, line_of(hi)?);
        match data.mark.map(|mark| mark.unit) {
            Some(Unit::Line) => Some((
                Pos {
                    row: from.row,
                    offset: from.start,
                },
                Pos {
                    row: last.row,
                    offset: last.end,
                },
            )),
            _ => {
                let end = grapheme_end(rows.row_text(hi.row), hi.offset, last.end);
                (end > hi.offset || lo < hi).then_some((
                    lo,
                    Pos {
                        row: hi.row,
                        offset: end,
                    },
                ))
            }
        }
    }

    /// The caret's cell, bold and underlined whatever else is selected (the oracle's
    /// `render_select_cursor`): the area draws the pen over the cell's text and the selection's
    /// REVERSED over both. `rows` is what the pane reads, none where it reads nothing; the
    /// mode being off, the decoration is let go. Asked only when the cell or the rows changed.
    ///
    /// Flutter: a `TextSpan` of the caret's grapheme in `SelectableRegion`'s second highlight.
    pub fn decorate(&self, rows: Option<&Rc<RowTexts>>, selection: &TextSelection) {
        let rows = rows.filter(|_| self.active());
        let wanted = rows.and_then(|rows| {
            self.cursor_cell(rows)
                .map(|(cursor, end)| (cursor, end, rows.revision_of()))
        });
        {
            let mut data = self.0.data.borrow_mut();
            if data.decorated == wanted {
                return;
            }
            data.decorated = wanted;
        }
        let decorations = match (wanted, rows) {
            (Some((cursor, end, _)), Some(rows)) => vec![
                SelectionDecoration::new(
                    SelectionTarget::of(rows),
                    cursor.offset..end,
                    caret_pen(),
                )
                .item(cursor.row),
            ],
            _ => Vec::new(),
        };
        selection.controller().set_decorations(decorations);
    }

    /// Where the caret's grapheme starts and ends, none on an empty line.
    fn cursor_cell(&self, rows: &RowTexts) -> Option<(Pos, usize)> {
        let cursor = self.0.data.borrow().cursor;
        let line = lines(rows).into_iter().find(|line| {
            line.row == cursor.row && (line.start..=line.end).contains(&cursor.offset)
        })?;
        let end = grapheme_end(rows.row_text(cursor.row), cursor.offset, line.end);
        (end > cursor.offset).then_some((cursor, end))
    }

    /// Shows what the mode holds in the selection area: the caret's grapheme or the marked range
    /// (a range across rows is a selection of the list as a whole: its object is learned from the
    /// first selection of it, so a range across rows shows from the second frame on).
    pub fn show(&self, rows: &Rc<RowTexts>, selection: &TextSelection) {
        let controller = selection.controller();
        if !self.0.attached.replace(true) {
            // a selection that is let go from outside (a delivered copy, a click) is asked for
            // again: the caret is always shown
            let (watched, again) = (controller.clone(), self.clone());
            controller.add_listener(Rc::new(move || {
                if !watched.has_selection() && again.active() {
                    again.0.data.borrow_mut().shown = None;
                    again.notify();
                }
            }));
        }
        let entering = std::mem::take(&mut self.0.data.borrow_mut().adopt);
        if entering
            && let Some((start, end)) = controller.start().zip(controller.end())
            && let Some((mark, cursor)) = mouse_mark(
                rows,
                &controller.text(),
                (start.object == end.object).then_some(start.offset),
            )
        {
            let mut data = self.0.data.borrow_mut();
            data.mark = Some(mark);
            data.cursor = cursor;
            data.reveal = false;
        }
        self.show_range(rows, selection, self.range(rows), !entering);
    }

    fn show_range(
        &self,
        rows: &Rc<RowTexts>,
        selection: &TextSelection,
        wanted: Option<(Pos, Pos)>,
        learn_list: bool,
    ) {
        let controller = selection.controller();
        if learn_list
            && self.0.data.borrow().list.is_none()
            && let Some(point) = controller.start()
        {
            // Once the mode's selectable list has applied a range, its render object can be
            // used for ranges extending across rows.
            self.0.data.borrow_mut().list = Some(point.object);
        }
        let key = wanted.map(|(from, to)| (from, to, rows.revision_of()));
        {
            let data = self.0.data.borrow();
            if data.shown == key && data.shown.is_some() {
                return;
            }
        }
        let Some((from, to)) = wanted else {
            controller.clear();
            self.0.data.borrow_mut().shown = None;
            return;
        };
        if from.row == to.row {
            controller.select(SelectionTarget::of(rows), from.row, from.offset, to.offset);
        } else if let Some(list) = self.0.data.borrow().list {
            controller.select_between(
                rows.point(list, from.row, from.offset),
                rows.point(list, to.row, to.offset),
            );
        } else {
            // Seed the selectable list's render object by selecting a nonempty part of it.
            let row = if to.offset == 0 { from.row } else { to.row };
            controller.select(SelectionTarget::of(rows), row, 0, rows.row_text(row).len());
            self.notify();
            return;
        }
        self.0.data.borrow_mut().shown = key;
    }

    /// The reader whose list the mode began over (Find opens from it).
    pub fn reader(&self) -> Option<Follow> {
        self.0.data.borrow().follow.clone()
    }

    /// The mode ends, what is selected left as it is: a copy that follows clears it when it is
    /// delivered.
    pub fn leave(&self) {
        {
            let mut data = self.0.data.borrow_mut();
            data.active = false;
            data.mark = None;
            data.shown = None;
            data.rows = None;
            data.query.clear();
        }
        self.notify();
    }

    /// Whether the pane refused the mode, once: it began where nothing is selectable
    /// (`begin_select_mode` is refused, and a sheet that was to hold it is done).
    pub fn refused(&self) -> bool {
        std::mem::take(&mut self.0.data.borrow_mut().refused)
    }

    /// Whether the mode's listener has been put on the pane (`watched`), once.
    pub fn watching(&self) -> bool {
        !std::mem::replace(&mut self.0.data.borrow_mut().watched, true)
    }
}

/// The ordinary transcript selects its rendered children. Match the selection against the
/// logical text of the select mode, excluding gutters. A renderer's byte offset can identify
/// repeated words on one line; repeated matches at the same offset have no unambiguous anchor.
fn mouse_mark(rows: &RowTexts, selected: &str, offset: Option<usize>) -> Option<(Mark, Pos)> {
    let (from, to, whole_lines) = mouse_range(rows, selected, offset)?;
    let mark = Mark {
        anchor: from,
        unit: if whole_lines {
            Unit::Line
        } else {
            Unit::Character
        },
    };
    let cursor = Pos {
        row: to.row,
        offset: TextBuffer::new(rows.row_text(to.row)).character_start_before(to.offset),
    };
    Some((mark, cursor))
}

fn mouse_range(rows: &RowTexts, selected: &str, offset: Option<usize>) -> Option<(Pos, Pos, bool)> {
    let selected = selected.trim_end_matches('\n');
    if selected.is_empty() {
        return None;
    }

    let mut text = String::new();
    let mut starts = Vec::new();
    for line in lines(rows) {
        starts.push((text.len(), line));
        text.push_str(&rows.row_text(line.row)[line.start..line.end]);
        text.push('\n');
    }
    let locate = |at| {
        let (start, line) = starts[starts.partition_point(|(start, _)| *start <= at) - 1];
        (line, line.start + at - start)
    };
    // A selected substring may overlap another occurrence ("aa" in "aaa").
    let first_char = selected.chars().next()?.len_utf8();
    let mut after = 0;
    let mut matches = std::iter::from_fn(|| {
        let start = text.get(after..)?.find(selected)? + after;
        after = start + first_char;
        Some(start)
    })
    .peekable();
    let first = matches.next()?;
    let start = if matches.peek().is_some() {
        let offset = offset?;
        let mut at_offset = std::iter::once(first).chain(matches).filter(|&index| {
            let (line, at) = locate(index);
            at - line.start == offset
        });
        let only = at_offset.next()?;
        if at_offset.next().is_some() {
            return None;
        }
        only
    } else {
        first
    };
    let (first, from) = locate(start);
    let (last, to) = locate(start + selected.len());
    Some((
        Pos {
            row: first.row,
            offset: from,
        },
        Pos {
            row: last.row,
            offset: to,
        },
        from == first.start && to == last.end,
    ))
}

/// The lines of the text a caret can stop on, in the order of the text.
fn lines(rows: &RowTexts) -> Vec<Line> {
    let mut lines = Vec::new();
    for row in 0..rows.row_count() {
        let text = rows.row_text(row);
        let mut start = 0;
        for (nth, line) in text.split('\n').enumerate() {
            if !rows.is_gutter(row, nth) {
                lines.push(Line {
                    row,
                    start,
                    end: start + line.len(),
                });
            }
            start += line.len() + 1;
        }
    }
    lines
}

/// Where a caret stands for a match that starts at `at`: on the grapheme there, where that is a
/// stop; the end of a line is not (the match began on its line break): the next line's start, or
/// the last grapheme where none follows (`move_select_cursor_to_search_match`).
fn place(rows: &RowTexts, lines: &[Line], at: Pos) -> Pos {
    let after = |at: Pos| {
        lines
            .iter()
            .find(|line| (line.row, line.start) > (at.row, at.offset))
    };
    let Some(line) = lines
        .iter()
        .find(|line| line.row == at.row && (line.start..=line.end).contains(&at.offset))
    else {
        // a match in a gutter: the next line there is a stop on
        return after(at).or(lines.last()).map_or(at, at_line);
    };
    if line.empty() {
        return at_line(line);
    }
    if at.offset >= line.end {
        return after(at).map_or_else(|| last_grapheme(rows, line), at_line);
    }
    let buffer = TextBuffer::new(rows.row_text(at.row));
    let end = buffer.character_end_after(at.offset);
    Pos {
        row: at.row,
        offset: buffer.character_start_before(end).max(line.start),
    }
}

fn index_of(lines: &[Line], pos: Pos) -> usize {
    lines
        .iter()
        .position(|line| line.row == pos.row && (line.start..=line.end).contains(&pos.offset))
        .unwrap_or(0)
}

fn at_line(line: &Line) -> Pos {
    Pos {
        row: line.row,
        offset: line.start,
    }
}

/// The byte after the grapheme at `offset` of `text`, within a line that ends at `end`.
fn grapheme_end(text: &str, offset: usize, end: usize) -> usize {
    if offset >= end {
        return offset;
    }
    TextBuffer::new(text)
        .character_end_after(offset)
        .min(end)
        .max(offset)
}

/// What the caret's cell is drawn in whatever else is selected: the oracle's `render_select_cursor`
/// patches reversed, bold and underlined on it, so it stays reversed while Find shows a match
/// elsewhere.
fn caret_pen() -> Pen {
    Style::new()
        .attrs(Attrs::REVERSE | Attrs::BOLD | Attrs::UNDERLINE)
        .pen()
}

/// The start of the last grapheme of `line` (its start where it is empty).
fn last_grapheme(rows: &RowTexts, line: &Line) -> Pos {
    let text = rows.row_text(line.row);
    let offset = if line.empty() {
        line.start
    } else {
        TextBuffer::new(text)
            .character_start_before(line.end)
            .max(line.start)
    };
    Pos {
        row: line.row,
        offset,
    }
}

/// The display column of `offset` in its line.
fn column_of(rows: &RowTexts, line: &Line, offset: usize) -> usize {
    rtwk_core::unicode::string_width(
        &rows.row_text(line.row)[line.start..offset.max(line.start)],
        rtwk_core::unicode::TextMode::Graphemes,
        1,
    )
}

/// The byte of `line` at display column `column`, or its last grapheme.
fn offset_at(rows: &RowTexts, line: &Line, column: usize, boundary: bool) -> usize {
    let text = rows.row_text(line.row);
    let mut offset = line.start;
    while offset < line.end {
        let next = grapheme_end(text, offset, line.end);
        if next <= offset || column_of(rows, line, next) > column {
            break;
        }
        if next >= line.end {
            return if boundary {
                line.end
            } else {
                last_grapheme(rows, line).offset
            };
        }
        offset = next;
    }
    offset
}

/// Where `motion` takes the caret from `cursor`, and the display column to keep (up and down).
fn moved(
    rows: &RowTexts,
    lines: &[Line],
    cursor: Pos,
    column: Option<usize>,
    motion: Motion,
    boundary: bool,
) -> (Pos, Option<usize>) {
    let here = index_of(lines, cursor);
    let line = lines[here];
    let text = rows.row_text(line.row);
    let stay = (cursor, None);
    match motion {
        Motion::Left => {
            if cursor.offset > line.start {
                let to = TextBuffer::new(text)
                    .character_start_before(cursor.offset)
                    .max(line.start);
                return (
                    Pos {
                        row: line.row,
                        offset: to,
                    },
                    None,
                );
            }
            // the blank lines between are not stops
            lines[..here]
                .iter()
                .rev()
                .find(|line| !line.empty())
                .map_or(stay, |line| {
                    let to = if boundary {
                        Pos {
                            row: line.row,
                            offset: line.end,
                        }
                    } else {
                        last_grapheme(rows, line)
                    };
                    (to, None)
                })
        }
        Motion::Right => {
            let next = grapheme_end(text, cursor.offset, line.end);
            if next < line.end || (boundary && next > cursor.offset) {
                return (
                    Pos {
                        row: line.row,
                        offset: next,
                    },
                    None,
                );
            }
            lines[here + 1..]
                .iter()
                .find(|line| !line.empty())
                .map_or(stay, |line| (at_line(line), None))
        }
        Motion::Rows(by) => {
            let target = here
                .saturating_add_signed(by)
                .min(lines.len().saturating_sub(1));
            if target == here {
                return (cursor, column);
            }
            let column = column.unwrap_or_else(|| column_of(rows, &line, cursor.offset));
            let to = lines[target];
            (
                Pos {
                    row: to.row,
                    offset: offset_at(rows, &to, column, boundary),
                },
                Some(column),
            )
        }
        Motion::LineStart => (at_line(&line), None),
        Motion::LineEnd => (last_grapheme(rows, &line), None),
        Motion::Beginning => (at_line(&lines[0]), None),
        Motion::End => (last_grapheme(rows, &lines[lines.len() - 1]), None),
        Motion::WordForward => {
            let buffer = TextBuffer::new(text);
            let end = buffer.word_end_after_in(cursor.offset, false, line.start, line.end);
            if end < line.end {
                let after = buffer.word_end_after_in(end, false, line.start, line.end);
                let start = buffer.word_start_before_in(after, false, line.start, line.end);
                return (
                    Pos {
                        row: line.row,
                        offset: start.max(end.min(after)),
                    },
                    None,
                );
            }
            lines[here + 1..]
                .iter()
                .find(|line| !line.empty())
                .map_or(stay, |line| (at_line(line), None))
        }
        Motion::WordBackward => {
            let buffer = TextBuffer::new(text);
            if cursor.offset > line.start {
                let start = buffer.word_start_before_in(cursor.offset, false, line.start, line.end);
                return (
                    Pos {
                        row: line.row,
                        offset: start,
                    },
                    None,
                );
            }
            lines[..here]
                .iter()
                .rev()
                .find(|line| !line.empty())
                .map_or(stay, |line| {
                    let buffer = TextBuffer::new(rows.row_text(line.row));
                    let start = buffer.word_start_before_in(line.end, false, line.start, line.end);
                    (
                        Pos {
                            row: line.row,
                            offset: start,
                        },
                        None,
                    )
                })
        }
        Motion::WordEnd => {
            let buffer = TextBuffer::new(text);
            let end = buffer.word_end_after_in(cursor.offset, false, line.start, line.end);
            if end > grapheme_end(text, cursor.offset, line.end) {
                let on = buffer.character_start_before_in(end, line.start, line.end);
                return (
                    Pos {
                        row: line.row,
                        offset: on,
                    },
                    None,
                );
            }
            lines[here + 1..]
                .iter()
                .find(|line| !line.empty())
                .map_or(stay, |line| {
                    let buffer = TextBuffer::new(rows.row_text(line.row));
                    let end = buffer.word_end_after_in(line.start, false, line.start, line.end);
                    let on = buffer.character_start_before_in(end, line.start, line.end);
                    (
                        Pos {
                            row: line.row,
                            offset: on,
                        },
                        None,
                    )
                })
        }
    }
}

/// The caret as a mode of the transcript (`mode_list!`): no fixture states of its own.
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "caret";
}
