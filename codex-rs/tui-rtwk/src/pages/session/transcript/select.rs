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
//! REVERSED as the oracle does, and the host paints the caret's cell bold and underlined too
//! (`caret`).
//!
//! Positions are a row of the list (oldest first) and a byte of its text; a caret stops on the
//! lines of the text that are not gutters. The oracle moves over display rows (a wrapped line is
//! several): this moves over the lines of the text, a gap row of the pack.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::editing::TextBuffer;
use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::foundation::ValueNotifier;
use rtwk::gestures::SelectionTarget;
use rtwk::render::RenderId;

use super::TranscriptMode;
use super::find::{Finder, RowTexts};
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

/// Rows a page and half a page move by, where the list's height is not known.
const PAGE: isize = 8;
const HALF: isize = 4;

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
        ("pageup", Do::Go(Rows(-PAGE))),
        ("ctrl+b", Do::Go(Rows(-PAGE))),
        ("pagedown", Do::Go(Rows(PAGE))),
        ("ctrl+f", Do::Go(Rows(PAGE))),
        ("ctrl+u", Do::Go(Rows(-HALF))),
        ("ctrl+d", Do::Go(Rows(HALF))),
        ("g", Do::Go(Beginning)),
        ("ctrl+home", Do::Go(Beginning)),
        ("shift+g", Do::Go(End)),
        ("G", Do::Go(End)),
        ("ctrl+end", Do::Go(End)),
    ]
}

struct Data {
    active: bool,
    /// The caret has not been placed: the pane does it with the first rows it reads.
    placed: bool,
    cursor: Pos,
    mark: Option<Mark>,
    /// The display column up and down keep.
    column: Option<usize>,
    follow: Option<Follow>,
    rows: Option<Rc<RowTexts>>,
    /// What the area was last asked to select, and the revision of the rows it was asked on.
    shown: Option<(Pos, Pos, u64)>,
    /// The list's render object, once a selection of it has told it.
    list: Option<RenderId>,
    /// The caret moved: the pane scrolls it into view.
    reveal: bool,
    watched: bool,
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
                placed: false,
                cursor: Pos::default(),
                mark: None,
                column: None,
                follow: None,
                rows: None,
                shown: None,
                list: None,
                reveal: false,
                watched: false,
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

    /// Whether the caret alone is shown (no mark): the host draws its cell bold and underlined.
    pub fn caret(&self) -> bool {
        let data = self.0.data.borrow();
        data.active && data.mark.is_none()
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
            data.placed = false;
            data.mark = None;
            data.column = None;
            data.shown = None;
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
        }
        selection.controller().clear();
        self.notify();
    }

    /// What the footer says: the keys of the mode, by whether a range is marked.
    pub fn hints(&self) -> Option<bool> {
        let data = self.0.data.borrow();
        data.active.then_some(data.mark.is_some())
    }

    /// The caret goes to the match of an accepted Find (`accept_search_into_select_mode`), the
    /// range that was marked gone.
    pub fn land(&self, hit: Pos) {
        {
            let mut data = self.0.data.borrow_mut();
            if !data.active {
                return;
            }
            data.cursor = hit;
            data.mark = None;
            data.column = None;
            data.reveal = true;
            data.shown = None;
        }
        self.notify();
    }

    /// A key of the mode: what the page does of it, or `None` where the key is not the mode's.
    pub fn key(&self, key: &KeyPress) -> Option<Step> {
        if !self.active() {
            return None;
        }
        let what = what(key)?;
        let rows = self.0.data.borrow().rows.clone()?;
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
            Do::Repeat(_) => Step::Moved,
            Do::Extend(motion) => {
                if !self.marked() {
                    self.mark(Unit::Character);
                }
                self.go(&rows, motion);
                Step::Moved
            }
            Do::Go(motion) => {
                self.go(&rows, motion);
                Step::Moved
            }
        };
        self.notify();
        Some(step)
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

    fn go(&self, rows: &RowTexts, motion: Motion) {
        let lines = lines(rows);
        let (cursor, column) = {
            let data = self.0.data.borrow();
            (data.cursor, data.column)
        };
        let (to, keep) = moved(rows, &lines, cursor, column, motion);
        let mut data = self.0.data.borrow_mut();
        data.cursor = to;
        data.column = keep;
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
            drop(data);
            self.notify();
            return false;
        }
        if !data.placed {
            data.placed = true;
            let last = lines.iter().rev().find(|line| !line.empty()).or(lines.last());
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
        // the list's object is the one a selection of it tells (a range across rows needs it)
        if self.0.data.borrow().list.is_none()
            && let Some(point) = controller.start()
        {
            self.0.data.borrow_mut().list = Some(point.object);
        }
        let wanted = self.range(rows);
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
            controller.select(
                SelectionTarget::of(rows),
                to.row,
                0,
                rows.row_text(to.row).len(),
            );
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
        }
        self.notify();
    }

    /// Whether the mode's listener has been put on the pane (`watched`), once.
    pub fn watching(&self) -> bool {
        !std::mem::replace(&mut self.0.data.borrow_mut().watched, true)
    }
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

fn index_of(lines: &[Line], pos: Pos) -> usize {
    lines
        .iter()
        .position(|line| line.row == pos.row && (line.start..=line.end).contains(&pos.offset))
        .unwrap_or(0)
}

fn at(line: &Line) -> Pos {
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
fn offset_at(rows: &RowTexts, line: &Line, column: usize) -> usize {
    let text = rows.row_text(line.row);
    let mut offset = line.start;
    while offset < line.end {
        let next = grapheme_end(text, offset, line.end);
        if next <= offset || column_of(rows, line, next) > column {
            break;
        }
        if next >= line.end {
            return last_grapheme(rows, line).offset;
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
                .map_or(stay, |line| (last_grapheme(rows, line), None))
        }
        Motion::Right => {
            let next = grapheme_end(text, cursor.offset, line.end);
            if next < line.end {
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
                .map_or(stay, |line| (at(line), None))
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
                    offset: offset_at(rows, &to, column),
                },
                Some(column),
            )
        }
        Motion::LineStart => (at(&line), None),
        Motion::LineEnd => (last_grapheme(rows, &line), None),
        Motion::Beginning => (at(&lines[0]), None),
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
                .map_or(stay, |line| (at(line), None))
        }
        Motion::WordBackward => {
            let buffer = TextBuffer::new(text);
            if cursor.offset > line.start {
                let start =
                    buffer.word_start_before_in(cursor.offset, false, line.start, line.end);
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
