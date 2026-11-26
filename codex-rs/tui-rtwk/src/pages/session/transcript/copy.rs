//! What the reader copied out of the transcript and what came of it (oracle
//! `transcript_view/selection.rs` `copy_selected_text_with`, `composer_gap.rs`
//! `show_copy_feedback`): the UI asks the host's clipboard with `Intent::Effect` (the app's one
//! door for asks outside the thread RPCs), core keeps the host's answer under the request
//! number (`CodexApp::effect`), and what the gap above the composer says of it is drawn state,
//! kept here in the UI (ADR-013: `Done::Copy` is the fact, the line is the view's).
//!
//! The log is the UI's follower of core's answers, like the controllers `follow_core` keeps in
//! step: after each event it reads the newest answer, and when that is one of its own copies it
//! becomes the feedback. The oracle's rule that a second copy refused as busy leaves the copy
//! under way showing is kept here (`show_copy_feedback`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::foundation::ValueNotifier;
use rtwk::gestures::SelectionTarget;
use rtwk::layout::{CellPainter, CustomPaint};
use rtwk_core::canvas::Canvas;
use rtwk_core::style::{Attrs, Pen};

use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::threads::Item;

use super::TranscriptMode;
use super::find::{Finder, RowTexts};
use super::item_text::{Reading, drawn_blocks};
use super::markdown_copy::{self, Marks};
use super::markdown_text::{self, Kind};
use super::select;
use super::strings::Strings;
use crate::pages::session::cells::text::Part;
use crate::widgets::thread_ui::{Follow, Input, TextSelection, ThreadUi};

/// The bit of the request numbers of the copies the transcript asks (the other bits are listed
/// in `core/src/app/effects.rs`: `1 << 58` is the features', `1 << 59` to `1 << 62` the screens').
pub const REQUEST_BIT: u64 = 1 << 57;

/// A block written in the Markdown source of a message, as the copy feature cuts it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Code(String),
    /// A quote: its source lines without the `>` marks, and what a copy of it answers with (its
    /// directive lines left out).
    Quote { source: String, copied: String },
}

/// What the mode reads of a message's source: handed in by the feature that begins it (a page does
/// not name a feature).
#[derive(Clone, Copy)]
pub struct Cutter {
    /// The code blocks and quotes of a source, in the order they stand in it.
    pub pieces: fn(&str) -> Vec<Piece>,
    /// The Markdown a copy of the whole response answers with (`None` where there is none).
    pub whole: fn(&str, &str) -> Option<String>,
}

/// How many sent copies are remembered: the host answers them in order and the app keeps as
/// many answers.
const KEPT: usize = 32;

/// What the reader asks to be copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyJob {
    pub text: String,
    pub format: CopyFormat,
    pub destination: CopyDestination,
    /// The text the selection had, to let it go once the host confirms the copy
    /// (`tui.copy_on_select_clear_selection`; oracle `pending_copy`): only a selection that
    /// still reads the same, not one made since.
    pub clear_after: Option<String>,
    /// The transcript goes back to its last row once the host confirms the copy (Enter on a
    /// selection: copy and follow; oracle `CopyAndFollow`).
    pub follow: bool,
}

impl CopyJob {
    /// What the reader selected, as plain text for the clipboard. An `explicit` copy (a key or
    /// the right button) lets the selection go once the host confirms it; one a release made
    /// does so only when `tui.copy_on_select_clear_selection` says (oracle `selection_copy`:
    /// `SelectionCopy::EXPLICIT`, `SelectionCopy::on_select`).
    pub fn selected(text: String, explicit: bool, settings: &TuiSettings) -> CopyJob {
        CopyJob {
            clear_after: (explicit || settings.deselect_after_copy).then(|| text.clone()),
            text,
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            follow: false,
        }
    }

    /// Text that is not a selection (a warning's details): copied as it is, nothing to let go.
    pub fn plain(text: String) -> CopyJob {
        CopyJob {
            text,
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            clear_after: None,
            follow: false,
        }
    }

    /// Back to the last row once the copy is confirmed.
    #[must_use]
    pub fn following(mut self) -> CopyJob {
        self.follow = true;
        self
    }

    /// The ask the host runs.
    pub fn ask(&self) -> Ask {
        Ask::Copy {
            text: self.text.clone(),
            format: self.format.clone(),
            destination: self.destination.clone(),
        }
    }
}

/// What the last copy came to, until the next answer: the gap above the composer words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feedback {
    /// How many answers have been taken in: a view tells a new answer from the one it has
    /// shown (a pending copy that ends is a new answer, with its own five seconds).
    pub version: u64,
    /// The copy this is the answer of.
    pub request: u64,
    /// How the copy was delivered; the host's failure text is the view's to word, so only
    /// that it failed is kept (oracle `Result<CopyStatus, ()>`).
    pub result: Result<CopyStatus, ()>,
    /// How many characters the copy carried (the oracle counts what was copied, not what is
    /// selected).
    pub characters: usize,
    /// The text of the selection to let go of when the copy is delivered.
    pub clear_after: Option<String>,
    /// The reader goes back to the last row when the copy is delivered.
    pub follow: bool,
}

/// What the view provides to the transcript and the composer gap: the feedback to show.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown(pub Option<Feedback>);

struct Sent {
    characters: usize,
    clear_after: Option<String>,
    follow: bool,
}

/// The UI's record of the copies it asked and the feedback they came to.
#[derive(Default)]
pub struct CopyLog {
    asks: u64,
    sent: BTreeMap<u64, Sent>,
    /// The newest answer already taken in.
    seen: Option<(u64, Result<CopyStatus, String>)>,
    answers: u64,
    feedback: Option<Feedback>,
}

impl CopyLog {
    /// A copy is asked: the number it goes under and what the host is asked.
    pub fn ask(&mut self, job: &CopyJob) -> (u64, Ask) {
        self.asks += 1;
        let request = REQUEST_BIT | self.asks;
        self.sent.insert(
            request,
            Sent {
                characters: job.ask().characters(),
                clear_after: job.clear_after.clone(),
                follow: job.follow,
            },
        );
        while self.sent.len() > KEPT {
            self.sent.pop_first();
        }
        (request, job.ask())
    }

    /// Takes in the newest answer of the app (`CodexApp::latest_effect`), once, if it is one of
    /// the copies asked.
    pub fn take_in(&mut self, latest: Option<(u64, &Done)>) {
        let Some((request, Done::Copy(result))) = latest else {
            return;
        };
        if request & REQUEST_BIT == 0 || self.seen.as_ref() == Some(&(request, result.clone())) {
            return;
        }
        self.seen = Some((request, result.clone()));
        let Some(sent) = self.sent.get(&request) else {
            return;
        };
        let result = result.clone().map_err(|_| ());
        // A second copy while one is under way is refused by the host as busy; the feedback
        // of the one under way stays (oracle `show_copy_feedback`).
        if result == Ok(CopyStatus::Busy)
            && self
                .feedback
                .as_ref()
                .is_some_and(|shown| matches!(shown.result, Ok(CopyStatus::Pending(_))))
        {
            return;
        }
        self.answers += 1;
        self.feedback = Some(Feedback {
            version: self.answers,
            request,
            result,
            characters: sent.characters,
            clear_after: sent.clear_after.clone(),
            follow: sent.follow,
        });
    }

    pub fn shown(&self) -> Shown {
        Shown(self.feedback.clone())
    }
}

// ---- copy mode (oracle `transcript_view/copy_mode.rs`) ----------------------------------------
//
// `/copy` on the chat's own transcript walks a cursor over what the reader can copy as one piece:
// the whole last response (where it starts), each code block and quote of the messages, each
// prompt; the footer says which (`Copy code block`) and what the keys do. Enter copies the target
// and goes back to the last row; what is copied is the target's own source (a code block without
// its fence, the response as Markdown), not what is drawn.
//
// The model is a controller (Flutter's `ValueNotifier<CopyState>`), one per thread, found by the
// thread's `ThreadUi` (`mode_of`, as the select mode's is): the entry begins it
// (`enter`), the page's key handler moves it (`key`), the pane gives it the targets it read of the
// items (`read`) and shows the target by asking the selection area to select its range (`show`), as
// it does a match of Find. The targets are a flat list in the order of the transcript; Up and Down
// move along it, the whole response sits after its end.
//
// The oracle finds the blocks in the rendered layout; this reads them from the blocks the cell
// draws (`markdown_text::found`) and from a prompt's source lines.

/// What a cell fills for the copy target it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// A prompt as a whole: its surface, padding rows included.
    Row,
    /// The top-level block `n` of a message.
    Block(usize),
}

/// One piece the reader can copy: a range of a row's text, what it is called, and what is copied
/// when it is not what is drawn there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The row of the list, oldest first.
    pub row: usize,
    /// What the cell of the row fills while the piece is the target (the oracle paints every cell
    /// of the rows of a block, blank ones too), where the cell can: none for the rest, whose
    /// glyphs the selection reverses.
    pub fill: Option<Fill>,
    /// The bytes of the row's text (gutters included, as the area counts them).
    pub range: Range<usize>,
    pub label: &'static str,
    pub payload: Option<String>,
}

/// What the pane read of the transcript for the mode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Targets {
    /// The pieces in the order of the transcript.
    pub list: Vec<Target>,
    /// The last response as a whole: its payload is the Markdown the copy answers with.
    pub response: Option<Target>,
}

impl Targets {
    /// Adds what row `row` (the item `item`, mounted as `parts`) holds: a prompt and the blocks
    /// written in it, a message's or a plan's code and quotes; the last response that has
    /// something visible is the whole response.
    pub fn add(
        &mut self,
        row: usize,
        item: &Item,
        parts: &[Part],
        reading: &Reading<'_>,
        table: &'static Strings,
        cutter: &Cutter,
    ) {
        let text = parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        // what is drawn decides, except for a prompt: it is kept where what it says is not blank
        // though what is drawn of it is (`8.`, a list marker with nothing after it: the Markdown
        // draws no text for it), and its range is then the row's whole text (oracle
        // `user_source`, copy_mode.rs)
        let drawn = extent(parts).filter(|&(first, last)| !text[first..last].trim().is_empty());
        let said = matches!(item, Item::User(prompt) if !prompt.shown().text.trim().is_empty());
        let Some((first, last)) = drawn.or_else(|| {
            said.then(|| extent(parts).unwrap_or((0, text.len())))
        }) else {
            return;
        };
        let mut found = Vec::new();
        match item {
            Item::User(prompt) => {
                let shown = prompt.shown();
                found.push(Target {
                    row,
                    fill: Some(Fill::Row),
                    range: first..last,
                    label: table.copy_user,
                    payload: (!shown.text.is_empty()).then(|| shown.text.to_string()),
                });
                found.extend(literal_blocks(
                    row,
                    &text,
                    (first, last),
                    &shown.text,
                    table,
                    cutter,
                ));
            }
            Item::Agent { text: source, .. } | Item::Plan { text: source } => {
                let cwd = reading.cwd.unwrap_or("");
                if let Some(markdown) = (cutter.whole)(source, cwd) {
                    self.response = Some(Target {
                        row,
                        fill: None,
                        range: first..last,
                        label: table.copy_whole,
                        payload: Some(markdown),
                    });
                }
                if !reading.raw {
                    found.extend(rendered_blocks(
                        row, item, source, parts, reading, table, cutter,
                    ));
                }
            }
            _ => return,
        }
        found.sort_by_key(|target| (target.range.start, std::cmp::Reverse(target.range.end)));
        self.list.extend(found);
    }
}

/// Where each part starts in the text of its row (the parts are joined by one line break).
fn starts_of(parts: &[Part]) -> Vec<usize> {
    let mut at = 0;
    parts
        .iter()
        .map(|part| {
            let start = at;
            at += part.text.len() + 1;
            start
        })
        .collect()
}

/// The bytes the non-gutter parts of a row span in its text: where the first starts to where the
/// last ends.
fn extent(parts: &[Part]) -> Option<(usize, usize)> {
    let mut span: Option<(usize, usize)> = None;
    for (part, start) in parts.iter().zip(starts_of(parts)) {
        if !part.gutter {
            let end = start + part.text.len();
            span = Some(span.map_or((start, end), |(first, _)| (first, end)));
        }
    }
    span
}

/// A text with its lines' trailing blanks and its ends cut: what two spellings of one block are
/// compared by (oracle `normalized`).
fn normalized(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// A drawn quote without the `>` marks of its lines.
fn unquoted(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix('>')
                .map_or(line, |line| line.strip_prefix(' ').unwrap_or(line))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The code and the quotes of the source of a message, as the copy picker lists them: the code
/// without its fence, a quote without its `>` marks and the directive lines of the agent app.
fn sources(cutter: &Cutter, source: &str) -> (Vec<String>, Vec<String>) {
    let (mut codes, mut quotes) = (Vec::new(), Vec::new());
    for piece in (cutter.pieces)(source) {
        match piece {
            Piece::Code(content) => codes.push(content),
            Piece::Quote { copied, .. } => quotes.push(copied),
        }
    }
    (codes, quotes)
}

/// The code blocks and quotes a message or plan draws, each with the source the author wrote
/// where one matches the drawn text (indented code and tables may have none: what is drawn is
/// then copied).
fn rendered_blocks(
    row: usize,
    item: &Item,
    source: &str,
    parts: &[Part],
    reading: &Reading<'_>,
    table: &'static Strings,
    cutter: &Cutter,
) -> Vec<Target> {
    // the body is the last parts, a part for each node the cell draws the blocks as: when it is
    // not (a cell that is drawn another way) there is no block to point at
    let blocks = drawn_blocks(item, reading);
    let Some(nodes) = markdown_text::nodes(&blocks) else {
        return Vec::new();
    };
    let Some(first) = parts.len().checked_sub(nodes.len()) else {
        return Vec::new();
    };
    if parts[first..]
        .iter()
        .zip(&nodes)
        .any(|(part, node)| part.text != node.text)
    {
        return Vec::new();
    }
    // where each part starts in the text of the row: a node's range is the node and its offset
    let starts = starts_of(parts);
    let end_of = |node: usize| starts[first + node] + parts[first + node].text.len();
    let (mut codes, mut quotes) = sources(cutter, source);
    markdown_text::found(&blocks)
        .into_iter()
        .filter_map(|block| {
            let (label, payload) = match block.kind {
                Kind::Code => (
                    table.copy_code,
                    take_source(&mut codes, &block.text, normalized),
                ),
                Kind::Quote => (
                    table.copy_quote,
                    take_source(&mut quotes, &unquoted(&block.text), normalized),
                ),
                // a table is a block of its own where a fenced block of Markdown held it: the
                // fence is gone from the drawing, its body is the payload; a table of the prose
                // is no block
                Kind::Table => (
                    table.copy_code,
                    Some(take_source(&mut codes, &block.text, cells_of)?),
                ),
            };
            Some(Target {
                row,
                fill: block.top.map(Fill::Block),
                range: starts[first + block.nodes.start]..end_of(block.nodes.end - 1),
                label,
                payload,
            })
        })
        .collect()
}

/// The source in `pool` that reads as `drawn` by `key`, taken out of it.
fn take_source(pool: &mut Vec<String>, drawn: &str, key: fn(&str) -> String) -> Option<String> {
    let want = key(drawn);
    let at = pool.iter().position(|source| key(source) == want)?;
    Some(pool.remove(at))
}

/// The words of the cells of a table, drawn or written (`| A | B |`, its delimiter row left
/// out): what a drawn table and its source are compared by.
fn cells_of(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let rest = line.replace(['|', ':', '-', ' ', '\t'], "");
            !(rest.is_empty() && line.contains('-'))
        })
        .flat_map(|line| line.split(|c: char| c == '|' || c.is_whitespace()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The code blocks and quotes written in a prompt, which is drawn as its source: found in its
/// text by what they say (oracle `literal_block_ranges`).
fn literal_blocks(
    row: usize,
    text: &str,
    (first, last): (usize, usize),
    source: &str,
    table: &'static Strings,
    cutter: &Cutter,
) -> Vec<Target> {
    let mut found = Vec::new();
    let mut cursor = first;
    for piece in (cutter.pieces)(source) {
        match piece {
            Piece::Code(content) => {
                let content = content.trim_end_matches('\n');
                if content.trim().is_empty() {
                    continue;
                }
                if let Some(at) = text[cursor..last].find(content).map(|at| at + cursor) {
                    found.push(Target {
                        row,
                        fill: None,
                        range: at..at + content.len(),
                        label: table.copy_code,
                        payload: Some(content.to_owned()),
                    });
                    cursor = at + content.len();
                }
            }
            Piece::Quote {
                source: content,
                copied,
            } => {
                let Some(line) = content.lines().find(|line| !line.trim().is_empty()) else {
                    continue;
                };
                let Some(at) = text[cursor..last].find(line).map(|at| at + cursor) else {
                    continue;
                };
                let start = text[..at]
                    .rfind('\n')
                    .map_or(0, |newline| newline + 1)
                    .max(first);
                // as many lines as the quote has, whether the drawing keeps its `>` marks or not;
                // a blank line of its source is a line of the drawing where the drawing has one
                // (the view draws the paragraphs of a quote on rows that follow each other)
                let mut end = start;
                let mut drawn = text[start..last].split_inclusive('\n').peekable();
                for source in content.trim_end_matches('\n').lines() {
                    let Some(line) = drawn.peek() else {
                        break;
                    };
                    if source.trim().is_empty() && !unquoted(line).trim().is_empty() {
                        continue;
                    }
                    end += line.len();
                    drawn.next();
                }
                let end = text[..end].trim_end_matches('\n').len().max(start);
                found.push(Target {
                    row,
                    fill: None,
                    range: start..end,
                    label: table.copy_quote,
                    payload: Some(copied),
                });
                cursor = end;
            }
        }
    }
    found
}

/// What a key of copy mode does to the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The target moved, or the key is the mode's and does nothing.
    Moved,
    /// Enter: copy the target, and go back to the last row once the host confirms.
    Copy,
    /// Esc: the mode ends, the transcript is at its last row.
    Exit,
    /// PageUp, PageDown: the transcript scrolls by pages.
    Page(i8),
}

#[derive(Clone, Copy, Debug)]
enum Do {
    Up,
    Down,
    Home,
    End,
    Copy,
    Exit,
    Page(i8),
    Other,
}

/// The keys of the mode (`handle_copy_mode_key`): the vim pairs map onto the arrows; every other
/// key is the mode's and does nothing, so no text reaches the composer.
const KEYS: &[(&str, Do)] = &[
    ("up", Do::Up),
    ("k", Do::Up),
    ("down", Do::Down),
    ("j", Do::Down),
    ("home", Do::Home),
    ("g", Do::Home),
    ("end", Do::End),
    ("shift+g", Do::End),
    ("G", Do::End),
    ("esc", Do::Exit),
    ("enter", Do::Copy),
    ("ctrl+c", Do::Copy),
    ("super+c", Do::Copy),
    ("ctrl+shift+c", Do::Copy),
    ("ctrl+insert", Do::Copy),
    ("pageup", Do::Page(-1)),
    ("pagedown", Do::Page(1)),
];

fn what(key: &KeyPress) -> Do {
    KEYS.iter()
        .find_map(|(spec, what)| {
            KeyCombo::parse(spec)
                .ok()
                .filter(|combo| combo.accepts_press(key))
                .map(|_| *what)
        })
        .unwrap_or(Do::Other)
}

/// What a copy of the target answers with where it is not what the area selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payload {
    pub text: String,
    pub format: CopyFormat,
}

struct Data {
    active: bool,
    /// The first target is chosen with the first targets the pane reads.
    placed: bool,
    /// The target held: an index of the list, or the whole response (`None`).
    at: Option<usize>,
    targets: Rc<Targets>,
    follow: Option<Follow>,
    /// What the feature reads of a source, handed in when it began the mode.
    cutter: Option<Cutter>,
    /// What the area was last asked to select: the row, the range and the revision of the rows.
    shown: Option<(usize, Range<usize>, u64)>,
    /// The target moved: the pane scrolls it into view.
    reveal: bool,
    /// What Enter chose to copy, for the copy that the page sends next.
    pending: Option<Payload>,
    /// What the target held draws (a filled target is not selected: the selection reverses the
    /// glyphs it covers, and the fill has done that already), for the copy of a target with no
    /// source of its own.
    drawn: Option<String>,
}

struct Inner {
    data: RefCell<Data>,
    /// The Markdown bodies a selection that no mode made is copied with (`markdown_copy`).
    marks: Marks,
    /// The select mode's: the pane and the footer listen to the one notifier.
    ticks: Rc<ValueNotifier<u32>>,
}

/// The copy mode of one thread's transcript: a handle that is equal to its clones.
#[derive(Clone)]
pub struct CopyMode(Rc<Inner>);

impl PartialEq for CopyMode {
    fn eq(&self, other: &CopyMode) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The copy mode of the thread whose finder this is (made on first ask, held with the thread's
/// controllers: `ThreadUi::held`).
pub fn mode_of(finder: &Finder) -> CopyMode {
    let marks = markdown_copy::of(finder);
    finder.held(|| {
        CopyMode(Rc::new(Inner {
            data: RefCell::new(Data {
                active: false,
                placed: false,
                at: None,
                targets: Rc::default(),
                follow: None,
                cutter: None,
                shown: None,
                reveal: false,
                pending: None,
                drawn: None,
            }),
            marks,
            ticks: select::of(finder).listenable(),
        }))
    })
}

/// `/copy` over the transcript of `ui`: the mode begins at the last row, on the last response.
/// Refused (false) while Find or another mode of input holds the keys.
pub fn enter(ui: &ThreadUi, cutter: Cutter) -> bool {
    if ui.input() != Input::Plain {
        return false;
    }
    mode_of(&ui.find).begin(&ui.follow, cutter);
    ui.modes.hold(Input::Copy);
    true
}

impl CopyMode {
    fn notify(&self) {
        self.0.ticks.set_value(self.0.ticks.value().wrapping_add(1));
    }

    /// Copy mode is on.
    pub fn active(&self) -> bool {
        self.0.data.borrow().active
    }

    /// The mode begins: the transcript goes to its last row and the first target is chosen with
    /// the first targets the pane reads (`begin_copy_mode`).
    pub fn begin(&self, follow: &Follow, cutter: Cutter) {
        {
            let mut data = self.0.data.borrow_mut();
            if data.active {
                return;
            }
            follow.jump_to_latest();
            data.active = true;
            data.placed = false;
            data.at = None;
            data.shown = None;
            data.pending = None;
            data.follow = Some(follow.clone());
            data.cutter = Some(cutter);
        }
        self.notify();
    }

    /// The mode ends: the area lets go of the target.
    pub fn end(&self, selection: &TextSelection) {
        self.leave();
        selection.controller().clear();
    }

    /// The mode ends, what is selected left as it is: a copy that follows clears it when it is
    /// delivered.
    pub fn leave(&self) {
        {
            let mut data = self.0.data.borrow_mut();
            data.active = false;
            data.shown = None;
            data.targets = Rc::default();
        }
        self.notify();
    }

    /// What the feature that began the mode reads of a source with.
    pub fn cutter(&self) -> Option<Cutter> {
        self.0.data.borrow().cutter
    }

    /// The reader whose list the mode began over (Esc goes back to its last row).
    pub fn reader(&self) -> Option<Follow> {
        self.0.data.borrow().follow.clone()
    }

    fn current(&self) -> Option<Target> {
        let data = self.0.data.borrow();
        match data.at {
            None => data.targets.response.clone(),
            Some(at) => data.targets.list.get(at).cloned(),
        }
    }

    /// What the cell of `row` of the list fills while the mode holds a piece of it as the target
    /// (the oracle paints all the cells of the target's rows, blank ones included).
    pub fn fill_of(&self, row: usize) -> Option<Fill> {
        if !self.active() {
            return None;
        }
        self.current()
            .filter(|target| target.row == row)
            .and_then(|target| target.fill)
    }

    /// What the footer's first row calls the target held (`code block`), once the mode is on; none
    /// where there is no target (the footer then says the whole response, as the oracle does).
    pub fn label(&self) -> Option<&'static str> {
        if !self.active() {
            return None;
        }
        self.current().map(|target| target.label)
    }

    /// The targets the pane last read (what the footer and the cursor choose among).
    pub fn targets(&self) -> Rc<Targets> {
        self.0.data.borrow().targets.clone()
    }

    /// The targets the pane read: the first read chooses where the mode starts (the whole last
    /// response, else the last piece).
    pub fn read(&self, targets: &Rc<Targets>) {
        let changed = {
            let mut data = self.0.data.borrow_mut();
            if !data.active {
                return;
            }
            let before = (data.at, data.targets.clone());
            if !Rc::ptr_eq(&data.targets, targets) && *data.targets != **targets {
                data.targets = targets.clone();
            }
            // a read with no target yet (the rows are not drawn) places nothing: the next one does
            let anything = data.targets.response.is_some() || !data.targets.list.is_empty();
            if !data.placed && anything {
                data.placed = true;
                data.at = match data.targets.response {
                    Some(_) => None,
                    None => data.targets.list.len().checked_sub(1),
                };
                data.reveal = true;
                data.shown = None;
            }
            if let Some(at) = data.at
                && at >= data.targets.list.len()
            {
                data.at = data.targets.list.len().checked_sub(1);
                data.shown = None;
            }
            // what the footer names follows the targets: it is told when they or the cursor moved
            before.0 != data.at || !Rc::ptr_eq(&before.1, &data.targets)
        };
        if changed {
            self.notify();
        }
    }

    /// A key of the mode: what the page does of it (every key is the mode's, one that is none of
    /// its table does nothing).
    pub fn key(&self, key: &KeyPress) -> Option<Step> {
        if !self.active() {
            return None;
        }
        let step = match what(key) {
            Do::Up => self.go(|at, n, _| match at {
                None => n.checked_sub(1).map(Some),
                Some(at) => at.checked_sub(1).map(Some),
            }),
            Do::Down => self.go(|at, n, response| match at {
                None => None,
                Some(at) if at + 1 < n => Some(Some(at + 1)),
                Some(_) => response.then_some(None),
            }),
            Do::Home => self.go(|_, n, _| (n > 0).then_some(Some(0))),
            Do::End => self.go(|_, n, response| {
                if response {
                    Some(None)
                } else {
                    n.checked_sub(1).map(Some)
                }
            }),
            Do::Copy => Step::Copy,
            Do::Exit => Step::Exit,
            Do::Page(pages) => Step::Page(pages),
            Do::Other => Step::Moved,
        };
        Some(step)
    }

    /// Moves the target as `to` says (from the one held, how many pieces there are and whether
    /// there is a whole response), or leaves it where `to` has no answer.
    fn go(&self, to: impl Fn(Option<usize>, usize, bool) -> Option<Option<usize>>) -> Step {
        {
            let mut data = self.0.data.borrow_mut();
            let (n, response) = (data.targets.list.len(), data.targets.response.is_some());
            if let Some(next) = to(data.at, n, response)
                && next != data.at
            {
                data.at = next;
                data.shown = None;
                data.reveal = true;
            }
        }
        self.notify();
        Step::Moved
    }

    /// The row the target moved to, once: the pane scrolls it into view.
    pub fn reveal(&self) -> Option<usize> {
        if !std::mem::take(&mut self.0.data.borrow_mut().reveal) {
            return None;
        }
        self.current().map(|target| target.row)
    }

    /// The target is shown as the selection of the area (it paints it REVERSED, as the oracle
    /// does).
    pub fn show(&self, rows: &Rc<RowTexts>, selection: &TextSelection) {
        let controller = selection.controller();
        let wanted = self
            .current()
            .map(|target| (target.row, target.range, rows.revision_of()));
        if wanted.is_some() && self.0.data.borrow().shown == wanted {
            return;
        }
        let filled = self.current().is_some_and(|target| target.fill.is_some());
        match &wanted {
            Some((row, range, _)) if !filled => {
                controller.select(SelectionTarget::of(rows), *row, range.start, range.end);
            }
            _ => controller.clear(),
        }
        let drawn = wanted
            .as_ref()
            .filter(|_| filled)
            .and_then(|(row, range, _)| rows.row_text(*row).get(range.clone()))
            .map(str::to_owned);
        let mut data = self.0.data.borrow_mut();
        data.shown = wanted;
        data.drawn = drawn;
    }

    /// Enter: what the target copies as is kept for the copy the page sends next ([`shape`]), and
    /// the mode ends with the selection left for that copy to read and clear.
    ///
    /// [`shape`]: CopyMode::shape
    pub fn copying(&self) {
        let target = self.current();
        let whole = self.0.data.borrow().at.is_none();
        let drawn = self.0.data.borrow().drawn.clone();
        self.0.data.borrow_mut().pending = target.and_then(|target| {
            target.payload.or(drawn).map(|text| Payload {
                text,
                format: if whole {
                    CopyFormat::Markdown
                } else {
                    CopyFormat::PlainText
                },
            })
        });
        self.leave();
    }

    /// What Enter chose to copy: the copy of a target that is filled, not selected, has this for
    /// its text.
    pub fn kept(&self) -> Option<String> {
        self.0
            .data
            .borrow()
            .pending
            .as_ref()
            .map(|payload| payload.text.clone())
    }

    /// The copy of the selection the page is about to send, as the target copies: its own source
    /// where it has one. A copy that is not the mode's (nothing was kept) is left as it is.
    pub fn shape(&self, mut job: CopyJob) -> CopyJob {
        if let Some(payload) = self.0.data.borrow_mut().pending.take() {
            job.text = payload.text;
            job.format = payload.format;
            return job;
        }
        self.marked(job)
    }

    /// A copy of what the reader selected in a Markdown message also offers the selection as
    /// Markdown (`markdown_copy`: its HTML flavour keeps the words' formatting), the plain text
    /// the same.
    #[must_use]
    pub fn marked(&self, mut job: CopyJob) -> CopyJob {
        if job.format == CopyFormat::PlainText
            && let Some(source) = self.0.marks.selected(&job.text)
        {
            job.format = CopyFormat::MarkdownSelection(source.into());
        }
        job
    }
}

/// Copying as a mode of the transcript (`mode_list!`): no fixture states of its own.
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "copy";
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn job(text: &str) -> CopyJob {
        CopyJob {
            text: text.to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            clear_after: None,
            follow: false,
        }
    }

    /// The app has kept `done` as the newest answer, under `request`.
    fn answer(log: &mut CopyLog, request: u64, done: Done) {
        log.take_in(Some((request, &done)));
    }

    fn feedback(log: &CopyLog) -> Option<Feedback> {
        log.shown().0
    }

    #[test]
    fn nothing_is_shown_until_the_host_answers() {
        let mut log = CopyLog::default();
        log.ask(&job("row 03"));
        assert_eq!(feedback(&log), None);
    }

    #[test]
    fn a_confirmed_copy_is_counted_in_what_was_copied_not_in_bytes() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("größe ✓"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Confirmed)));
        let got = feedback(&log).expect("feedback");
        assert_eq!(got.result, Ok(CopyStatus::Confirmed));
        assert_eq!(got.characters, 7);
    }

    #[test]
    fn a_failure_keeps_only_that_it_failed() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("x"));
        answer(&mut log, request, Done::Copy(Err("no tool".into())));
        assert_eq!(feedback(&log).expect("feedback").result, Err(()));
    }

    #[test]
    fn a_pending_copy_that_ends_is_a_new_answer() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("abc"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Pending(1))));
        let pending = feedback(&log).expect("feedback");
        assert_eq!(pending.result, Ok(CopyStatus::Pending(1)));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Confirmed)));
        let done = feedback(&log).expect("feedback");
        assert_eq!(done.result, Ok(CopyStatus::Confirmed));
        assert!(done.version > pending.version);
    }

    #[test]
    fn a_busy_answer_leaves_the_copy_under_way_showing() {
        let mut log = CopyLog::default();
        let (first, _) = log.ask(&job("abc"));
        answer(&mut log, first, Done::Copy(Ok(CopyStatus::Pending(1))));
        let under_way = feedback(&log);
        let (second, _) = log.ask(&job("def"));
        answer(&mut log, second, Done::Copy(Ok(CopyStatus::Busy)));
        assert_eq!(feedback(&log), under_way);
        // and what the first comes to afterwards is shown
        answer(&mut log, first, Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(
            feedback(&log).expect("feedback").result,
            Ok(CopyStatus::Confirmed)
        );
    }

    #[test]
    fn busy_alone_is_shown() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("abc"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Busy)));
        assert_eq!(
            feedback(&log).expect("feedback").result,
            Ok(CopyStatus::Busy)
        );
    }

    #[test]
    fn an_answer_to_another_ask_is_none_of_its_business() {
        let mut log = CopyLog::default();
        log.ask(&job("abc"));
        answer(&mut log, 7, Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(feedback(&log), None);
    }
}

/// Every cell of its box reversed over what the child drew (a foreground painter): the copy
/// mode's target fill, the oracle's selection style on all of the target's cells, blanks and
/// padding included. A cell keeps its other attributes and colors; the selection is not
/// drawn on a filled target, so the two never toggle each other.
#[derive(Debug)]
struct Reversed;

impl CellPainter for Reversed {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        let (ox, oy) = (canvas.origin_x(), canvas.origin_y());
        let (cx0, cy0, cx1, cy1) = canvas.clip_bounds();
        let (x0, x1) = ((ox).max(cx0), (ox + width).min(cx1));
        for ay in (oy).max(cy0)..(oy + height).min(cy1) {
            for ax in x0..x1 {
                let at = (ax as usize, ay as usize);
                let id = canvas.surface().style(at.0, at.1);
                let attrs = canvas.styles().attrs_of(id);
                if attrs & Attrs::REVERSE != Attrs::NONE {
                    continue;
                }
                let pen = canvas
                    .styles_mut()
                    .pen(&Pen::new().attrs(attrs | Attrs::REVERSE));
                let style = canvas.styles_mut().compose(id, pen);
                let width = canvas.surface().width();
                canvas.surface_mut().cells_styles_mut().1[at.1 * width + at.0] = style.0;
            }
            canvas
                .surface_mut()
                .mark_dirty(ay as usize, x0 as usize, x1.max(x0) as usize);
        }
    }

    fn should_repaint(&self, _old: &dyn CellPainter) -> bool {
        false
    }
}

/// `child` with every cell of its box reversed ([`Reversed`]).
pub fn reversed(child: impl Into<rtwk::AnyWidget>) -> CustomPaint {
    CustomPaint::new().foreground_painter(Reversed).child(child)
}
