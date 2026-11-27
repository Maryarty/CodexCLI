//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::gestures::SelectionTarget;
use rtwk::layout::{Align, Alignment, Column, CrossAxisAlignment, MainAxisSize};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::{
    ItemTexts, ListView, OverscrollNotification, ScrollBehavior, ScrollConfiguration,
    ScrollController, ScrollNotification,
};
use rtwk::style::{Attrs, Style};
use rtwk::text::segmenter_of;
use rtwk::{
    AnyWidget, BuildCx, Cx, GlobalKey, Key, LayoutBuilder, NotificationListener, Rebuild, State,
    StatefulWidget, StatelessWidget, impl_widget,
};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::backtrack::nth_user;
use codex_tui_core::threads::{HistoryState, Item, ItemKey, ThreadModel, TurnState};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::widgets::finder::More;
use crate::widgets::thread_scope::thread_scope;
use crate::widgets::thread_ui::{Follow, TextSelection};

use super::cells::disclosure::tile;
use super::cells::strings as cells_strings;
use super::cells::text::{Env, Room};
use super::cells::user;
use super::cells::{CellEnv, Place, cell};
use super::strings as chat_strings;
use super::transcript::activity_focus::{self, Activity, Stop, Walk};
use super::transcript::copy::{self as copying, Targets};
use super::transcript::find::{Finder, Held, Hit, RowTexts, SelectedTool};
use super::transcript::item_text::{Reading, body_nodes, mounted_parts};
use super::transcript::markdown_copy::{self, Marks};
use super::transcript::row_text::RowTextCache;
use super::transcript::select::{self, Pos, Select};
use super::transcript::selection::{SelectedRevision, SelectionHost};
use super::transcript::strings as transcript_strings;
use super::transcript::turn_details::{
    Anchor, FoldRow, Kind, Plan, PromptFold, Row, Words, arrow, prompt_label, reveal,
};
use super::transcript_activity::{ActivityStamp, has_control};

/// The transcript with a prompt marked (Flutter: a `ListView` whose item
/// `selected: true` is drawn highlighted and scrolled into view when it
/// changes): the nth prompt a browsing sheet offers, reversed in its body.
#[derive(Clone, PartialEq)]
pub(super) struct Pane {
    pub follow: Follow,
    /// What the reader has selected in the list: the one controller of the thread's `ThreadUi`.
    pub selection: TextSelection,
    /// Find over it: the one finder of the thread's `ThreadUi`.
    pub find: Finder,
    pub detailed: bool,
    pub mark: Option<usize>,
}

/// How many frames after a swap of the list the reader is put back again (the first lands on
/// estimates of the rows not laid out yet).
const KEEPS: u8 = 3;

/// A row the list was put at the top for (`ListController::jump_to_index_rows`), and where the
/// list settled with it, to tell that the reader has not moved since.
#[derive(Clone, Copy)]
struct Placed {
    index: usize,
    blank: usize,
    moves: u64,
    pixels: Option<i64>,
}

pub(super) struct ViewportState {
    layout: RefCell<Option<RowLayout>>,
    held_layout: RefCell<Option<RowLayout>>,
    activity: RefCell<Option<ActivityLayout>>,
    /// The prompt the list was last scrolled to show.
    shown: RefCell<Option<ItemKey>>,
    /// The first row the list showed, to see rows come in above it.
    first: RefCell<Option<(ThreadId, ItemKey)>>,
    /// Actual constraints of the pane, supplied by its layout builder.
    width: i32,
    revealed_rows: RefCell<Option<RevealedRows>>,
    scan_first: RefCell<Option<(ThreadId, ItemKey, usize)>>,
    scan_match: RefCell<Option<(ItemKey, String)>>,
    /// Where each reader that was left scrolled up stood (Flutter's `PageStorage` bucket of a
    /// `PageStorageKey` per list): a thread's reader is its own, so it is as it was when the
    /// reader comes back (#57499).
    left: Vec<(Follow, i64)>,
    /// The restore a thread's list waits to make once it knows its extents.
    restoring: Option<Restore>,
    /// The finder of the thread shown, and what it told this list to listen with: the list
    /// builds again when Find moves.
    finding: RefCell<Option<(Finder, VoidCallback)>>,
    /// The folds of the thread shown, and its listener: the list builds again when the reader
    /// opens one or the focus moves.
    folding: RefCell<Option<(Activity, VoidCallback)>>,
    /// The select mode of the thread shown, and its listener: the list builds again when the
    /// caret moves.
    selecting: RefCell<Option<(Select, VoidCallback)>>,
    /// The selection of the thread shown, and its listener: the list builds again when something
    /// comes to be selected, to read the Markdown of the rows the selection is copied with.
    watching: RefCell<Option<(TextSelection, VoidCallback)>>,
    /// The reader of the thread shown, and its listener: the list builds again when the reader
    /// moves the view by a key (a match held in the live tail is let go).
    moving: RefCell<Option<(Follow, VoidCallback)>>,
    /// The thread and the revision of its rows whose Markdown `Marks` holds.
    marked: RefCell<Option<(ThreadId, u64)>>,
    /// The texts of the rows Find reads, kept while they are the same so that what the selection
    /// area was asked to select stays named by the same model.
    texts: RefCell<Option<Rc<RowTexts>>>,
    row_text_cache: RefCell<RowTextCache>,
    /// The list was built as one that reads its rows as text: another widget than the plain list,
    /// so a list that comes in its place starts at the end.
    texted: Cell<bool>,
    /// The number of rows the list was last built with: a list takes the count of its items when
    /// it is updated, so a jump to an item asked for while the count changes names the wrong
    /// child (the reverse list counts its children from the top).
    counted: Cell<usize>,
    /// The row the list was last put at its top for, when older rows joined above the reader at
    /// the start of the transcript: kept while the reader does not move, to put the list there
    /// again where a pixel offset is no longer the place (another list is built with other
    /// extents of the rows above).
    anchor: Cell<Option<Placed>>,
    /// What copy mode can copy of those rows, kept while the rows are the same ones (it parses the
    /// messages' Markdown).
    targets: RefCell<Option<(Rc<RowTexts>, Rc<Targets>)>>,
}

/// A one-shot listener on a controller: the move to where its reader was, made when its list
/// has told its extents (`ScrollPosition::apply_extents`).
struct Restore {
    controller: ScrollController,
    listener: VoidCallback,
}

impl Restore {
    fn arm(controller: ScrollController, pixels: i64) -> Restore {
        let slot: Rc<RefCell<Option<VoidCallback>>> = Rc::default();
        let listener: VoidCallback = {
            let (controller, slot) = (controller.clone(), slot.clone());
            Rc::new(move || {
                let Ok(position) = controller.try_position() else {
                    return;
                };
                if !position.has_extents() {
                    return;
                }
                controller.jump_to(pixels);
                if let Some(this) = slot.borrow_mut().take() {
                    controller.remove_listener(&this);
                }
            })
        };
        *slot.borrow_mut() = Some(listener.clone());
        controller.add_listener(listener.clone());
        Restore {
            controller,
            listener,
        }
    }

    fn disarm(self) {
        self.controller.remove_listener(&self.listener);
    }
}

struct RowLayout {
    thread: ThreadId,
    revision: u64,
    detailed: bool,
    collapse: bool,
    opened: u64,
    shown: Shown,
}

struct ActivityLayout {
    rows: Rc<[Row]>,
    stamp: ActivityStamp,
    walk: Rc<Walk>,
}

struct RevealedRows {
    base: Rc<[Row]>,
    keys: Vec<ItemKey>,
    rows: Rc<[Row]>,
}

/// The rows of the list and the plan of the folds they were made from.
#[derive(Clone)]
struct Shown {
    rows: Rc<[Row]>,
    plan: Rc<Plan>,
}

/// What the list's state is told: older history is wanted, or a fold was tapped.
pub(super) enum Msg {
    Room(i32),
    Load(ThreadId),
    /// The reader waits for the first row (`Follow::beginning`) and the history is now in this
    /// state: the next page is asked for, the last one takes the reader to the first row, a
    /// refused one ends the wait.
    Beginning(ThreadId, HistoryState),
    Toggle(Vec<ItemKey>),
    /// Find moved: its footer and its match are drawn from the list's build.
    Find,
    /// Find closed and the list built again: the reader goes back.
    Restore,
    /// The list became another widget under a reader who was away from the last row: the new
    /// list starts at the end, and the reader goes back to where they were. The rows it has not
    /// measured yet are estimates, so the move is made again (the count) once they are.
    Keep(i64, u8),
    /// The match Find found is scrolled to once the list has the rows it was found in: the index
    /// of its row in the list (`ListController`'s, a reverse list counts from its bottom).
    Reveal(usize),
    /// The list settled at the row `keep_first` put it at.
    Anchored,
    /// The list is put at its anchor again, as many more times as the count says (the rows above
    /// it are measured as they are built).
    KeepRow(usize, usize, u8),
}

impl ViewportState {
    /// The visible rows: runs of explorations share one, and the reasoning
    /// summary and the warnings show only in the detailed transcript (oracle
    /// `ReasoningSummaryCell::transcript_only`, `WarningHistoryCell`). With the
    /// turn details collapsed the plan of the folds decides instead.
    fn rows(
        layout: &RefCell<Option<RowLayout>>,
        thread: &ThreadModel,
        detailed: bool,
        collapse: bool,
        activity: &Activity,
    ) -> Shown {
        let mut cache = layout.borrow_mut();
        let transcript = thread.transcript();
        let revision = transcript.layout_revision();
        if let Some(layout) = cache.as_ref()
            && layout.thread == *thread.id()
            && layout.revision == revision
            && layout.detailed == detailed
            && layout.collapse == collapse
            && layout.opened == activity.revision()
        {
            return layout.shown.clone();
        }
        let hidden = |index: usize| {
            !detailed
                && transcript.get(index).is_some_and(|(_, item)| {
                    item.is_detailed_only() || is_terminal_interaction(item)
                })
        };
        let shown = if collapse && !detailed {
            let plan = Plan::of(transcript);
            let rows = plan.rows(transcript, &activity.open(), hidden);
            Shown {
                rows: rows.into(),
                plan: Rc::new(plan),
            }
        } else {
            let mut rows: Vec<Row> = Vec::new();
            let mut run = None;
            for (index, (key, item)) in transcript.iter().enumerate() {
                if hidden(index) {
                    continue;
                }
                let kind = item.group().map(|group| (group, &key.turn));
                if kind.is_some()
                    && kind == run
                    && let Some(group) = rows.last_mut()
                {
                    group.range.end = index + 1;
                } else {
                    rows.push(Row {
                        range: index..index + 1,
                        kind: Kind::Cell,
                    });
                }
                run = kind;
            }
            Shown {
                rows: rows.into(),
                plan: Rc::new(Plan::default()),
            }
        };
        *cache = Some(RowLayout {
            thread: thread.id().clone(),
            revision,
            detailed,
            collapse,
            opened: activity.revision(),
            shown: shown.clone(),
        });
        shown
    }
}

impl ViewportState {
    /// The list builds again when `finder` moves (its footer and its match are drawn from the
    /// build): one listener, on the finder of the thread shown.
    fn listen(&self, finder: &Finder, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.finding.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == finder) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.listenable().remove_listener(&listener);
        }
        let listener: VoidCallback = Rc::new(move || send(Msg::Find));
        finder.listenable().add_listener(listener.clone());
        *held = Some((finder.clone(), listener));
    }

    /// The list builds again when the folds of the thread shown change (one opens or closes, the
    /// focus moves): one listener, on its `activity`.
    fn listen_folds(&self, activity: &Activity, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.folding.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == activity) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.listenable().remove_listener(&listener);
        }
        let listener: VoidCallback = Rc::new(move || send(Msg::Find));
        activity.listenable().add_listener(listener.clone());
        *held = Some((activity.clone(), listener));
    }

    /// The list builds again when `select` moves: one listener, on the select mode of the thread
    /// shown.
    fn listen_select(&self, select: &Select, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.selecting.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == select) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.listenable().remove_listener(&listener);
        }
        let listener: VoidCallback = Rc::new(move || send(Msg::Find));
        select.listenable().add_listener(listener.clone());
        *held = Some((select.clone(), listener));
    }

    /// The list builds again when the reader moves the view by a key: one listener, on the reader
    /// of the thread shown.
    fn listen_moves(&self, follow: &Follow, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.moving.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == follow) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.moves_listenable().remove_listener(&listener);
        }
        let listener: VoidCallback = Rc::new(move || send(Msg::Find));
        follow.moves_listenable().add_listener(listener.clone());
        *held = Some((follow.clone(), listener));
    }

    /// The list builds again when something comes to be selected or nothing is any more: one
    /// listener, on the selection of the thread shown.
    fn listen_selection(&self, selection: &TextSelection, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.watching.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == selection) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.controller().remove_listener(&listener);
        }
        let listener: VoidCallback = Rc::new(move || send(Msg::Find));
        selection.controller().add_listener(listener.clone());
        *held = Some((selection.clone(), listener));
    }

    /// The Markdown bodies of the rows, for the copy of a selection nobody made through a mode
    /// (a drag, a double click): kept while the selection lasts and the rows are the same.
    fn mark(
        &self,
        marks: &Marks,
        selection: &TextSelection,
        thread: &ThreadModel,
        rows: &[Row],
        reading: &Reading<'_>,
    ) {
        let transcript = thread.transcript();
        if !selection.controller().has_selection() {
            if self.marked.borrow_mut().take().is_some() {
                marks.fill(Vec::new());
            }
            return;
        }
        marks.attach(selection.controller());
        let stamp = (thread.id().clone(), transcript.layout_revision());
        if self.marked.borrow().as_ref() == Some(&stamp) {
            return;
        }
        let bodies = rows
            .iter()
            .filter_map(|row| transcript.get(row.range.start))
            .filter_map(|(_, item)| body_nodes(item, reading))
            .map(Rc::from)
            .collect();
        marks.fill(bodies);
        *self.marked.borrow_mut() = Some(stamp);
    }

    /// What copy mode can copy of `texts`' rows: the prompts, the messages' code blocks and quotes
    /// and the last response. Kept while the rows are the same.
    fn targets_of(
        &self,
        texts: &Rc<RowTexts>,
        thread: &ThreadModel,
        rows: &[Row],
        header: bool,
        reading: &Reading<'_>,
        (table, cutter): (&'static transcript_strings::Strings, &copying::Cutter),
    ) -> Rc<Targets> {
        if let Some((held, targets)) = self.targets.borrow().as_ref()
            && Rc::ptr_eq(held, texts)
        {
            return targets.clone();
        }
        let transcript = thread.transcript();
        let lead = usize::from(header);
        let mut targets = Targets::default();
        for (at, row) in rows.iter().enumerate() {
            if let Kind::Tools(group) = &row.kind
                && !group.is_lone_search(transcript)
            {
                continue;
            }
            let Some((_, item)) = transcript.get(row.range.start) else {
                continue;
            };
            if let Some(parts) = mounted_parts(item, reading) {
                targets.add(lead + at, item, &parts, reading, table, cutter);
            }
        }
        let targets = Rc::new(targets);
        *self.targets.borrow_mut() = Some((texts.clone(), targets.clone()));
        targets
    }

    fn with_reveals(&self, base: Rc<[Row]>, thread: &ThreadModel, keys: Vec<ItemKey>) -> Rc<[Row]> {
        let mut kept = self.revealed_rows.borrow_mut();
        if keys.is_empty() {
            kept.take();
            return base;
        }
        if let Some(old) = kept.as_ref()
            && Rc::ptr_eq(&old.base, &base)
            && old.keys == keys
        {
            return old.rows.clone();
        }
        let mut rows = base.to_vec();
        for (index, (key, _)) in thread.transcript().iter().enumerate() {
            if keys.contains(key) {
                reveal(&mut rows, index);
            }
        }
        let rows: Rc<[Row]> = rows.into();
        *kept = Some(RevealedRows {
            base,
            keys,
            rows: rows.clone(),
        });
        rows
    }

    /// A prepend shifts the raw candidates independently of the mounted rows.
    fn reconcile_scan(
        &self,
        thread: &ThreadModel,
        scan: &[Row],
        lead: usize,
        reverse: bool,
        search: &Finder,
    ) {
        let transcript = thread.transcript();
        let now = scan
            .first()
            .and_then(|row| transcript.get(row.range.start))
            .map(|(key, _)| (thread.id().clone(), key.clone(), lead));
        let Some((before_thread, key, before_lead)) = self.scan_first.replace(now) else {
            return;
        };
        if &before_thread != thread.id() {
            return;
        }
        let Some(at) = scan.iter().position(|row| {
            transcript
                .get(row.range.start)
                .is_some_and(|(at, _)| *at == key)
        }) else {
            return;
        };
        let added = (at + lead).saturating_sub(before_lead);
        if added > 0 {
            if !reverse {
                search.forget_place();
            }
            search.rows_joined(added);
        }
    }

    /// Keeps the same selectable text model while the mounted text stays equal.
    fn keep_texts(&self, fresh: RowTexts) -> Rc<RowTexts> {
        let mut kept = self.texts.borrow_mut();
        if let Some(old) = kept.as_ref()
            && **old == fresh
        {
            return old.clone();
        }
        let fresh = Rc::new(fresh);
        *kept = Some(fresh.clone());
        fresh
    }
}

impl ViewportState {
    /// A prompt newly marked scrolls into view, the least that shows it
    /// (`ListController::ensure_index_visible`, the list's index of its row).
    fn show(
        &self,
        marked: &Option<ItemKey>,
        thread: &ThreadModel,
        rows: &[Row],
        has_header: bool,
        reverse: bool,
        widget: &Pane,
    ) {
        let mut shown = self.shown.borrow_mut();
        if *shown == *marked {
            return;
        }
        shown.clone_from(marked);
        let Some(marked) = marked else { return };
        let transcript = thread.transcript();
        let Some(position) = transcript.iter().position(|(key, _)| key == marked) else {
            return;
        };
        let Some(row) = rows.iter().position(|row| row.range.contains(&position)) else {
            return;
        };
        let count = rows.len() + usize::from(has_header);
        let index = row + usize::from(has_header);
        widget.follow.scroll().ensure_index_visible(if reverse {
            count - 1 - index
        } else {
            index
        });
    }
}

impl ViewportState {
    /// Rows came in above the first row the reader had (an older page joined) while the list
    /// stood at its first line: the list keeps a row in place only where it is scrolled from its
    /// start, so the row the reader was at is brought back to the top (the oracle's
    /// `prepend_snapshot_history` keeps the anchor; Flutter's reverse list does it by its origin).
    fn keep_first(
        &self,
        thread: &ThreadModel,
        rows: &[Row],
        lead: usize,
        reverse: bool,
        widget: &Pane,
    ) -> bool {
        let transcript = thread.transcript();
        let first = rows
            .first()
            .and_then(|row| transcript.get(row.range.start))
            .map(|(key, _)| (thread.id().clone(), key.clone()));
        let before = self.first.replace(first.clone());
        let (Some(before), Some(now)) = (before, first) else {
            return false;
        };
        if before.0 != now.0 || before.1 == now.1 {
            return false;
        }
        if !reverse {
            widget.follow.shifted();
        }
        let moved = rows.iter().position(|row| {
            transcript
                .get(row.range.start)
                .is_some_and(|(key, _)| *key == before.1)
        });
        if reverse || widget.follow.at_tail() {
            return false;
        }
        let at_start = widget
            .follow
            .scroll()
            .as_scroll_controller()
            .try_position()
            .is_ok_and(|position| position.pixels() <= 0);
        if let (true, Some(row)) = (at_start, moved) {
            // the cell has a blank row above it now that something comes before it (`cell`'s
            // `below`), which the reader never saw: the list starts under it
            let index = rows[row].range.start;
            let joins = matches!(
                index
                    .checked_sub(1)
                    .and_then(|before| transcript.get(before)),
                Some((_, Item::User(_)))
            ) && transcript.timing_after(index.saturating_sub(1)).is_none();
            let user = matches!(transcript.get(index), Some((_, Item::User(_))));
            let blank = usize::from(!(user && joins));
            widget.follow.scroll().jump_to_index_rows(row + lead, blank);
            self.anchor.set(Some(Placed {
                index: row + lead,
                blank,
                moves: widget.follow.moves(),
                pixels: None,
            }));
            return true;
        }
        false
    }
}

/// The write to a background terminal shows only in the detailed
/// transcript (oracle `UnifiedExecInteractionCell` has no display lines).
fn is_terminal_interaction(item: &Item) -> bool {
    matches!(item, Item::Notice(notice) if notice.detailed_only())
}

/// The item of `key` is of the turn that is running: the live tail (oracle `EntryKey::Live`, the
/// active cell).
fn shown_hit(hit: Hit, scan: &[Row], shown: &[Row], lead: usize) -> Option<Hit> {
    let item = scan.get(hit.row.checked_sub(lead)?)?.range.start;
    let row = shown.iter().position(|row| row.range.start == item)?;
    Some(Hit {
        row: lead + row,
        ..hit
    })
}

fn in_running_turn(thread: &ThreadModel, key: &ItemKey) -> bool {
    matches!(thread.turn(), TurnState::Running(running) if running.turn == key.turn)
}

/// The ids of the fold a row draws, if it draws one: a prompt with messages behind it, a run of
/// calls (a lone finished search is the row it is).
fn fold_of(row: &Row, transcript: &codex_tui_core::threads::Transcript) -> Option<Vec<ItemKey>> {
    match &row.kind {
        Kind::Prompt(prompt) => Some(vec![prompt.id.clone()]),
        Kind::Tools(group) if !group.is_lone_search(transcript) => Some(group.ids.clone()),
        _ => None,
    }
}

/// The stop of F4 a row draws, if it draws one: its fold, or the cell of a call or a patch that
/// has a control (`+ Show details`). The key of a cell is its row's first item, as `tile` has it.
fn stop_of(
    row: &Row,
    transcript: &codex_tui_core::threads::Transcript,
    strings: &super::cells::strings::Strings,
    room: Room,
) -> Option<Stop> {
    if let Some(fold) = fold_of(row, transcript) {
        return Some(Stop::fold(fold));
    }
    let (key, item) = transcript.get(row.range.start)?;
    let has = match item {
        Item::Patch(patch) => !patch.changes.is_empty(),
        Item::Tool(tool) => has_control(tool, strings, room),
        _ => false,
    };
    has.then(|| Stop::cell(key.clone()))
}

impl ViewportState {
    fn activity_walk(
        &self,
        rows: &Rc<[Row]>,
        thread: &Rc<ThreadModel>,
        strings: &'static super::cells::strings::Strings,
        room: Room,
    ) -> Rc<Walk> {
        let stamp = ActivityStamp::of(thread.transcript(), strings, room);
        let mut cache = self.activity.borrow_mut();
        if let Some(layout) = cache.as_ref()
            && Rc::ptr_eq(&layout.rows, rows)
            && layout.stamp == stamp
        {
            return layout.walk.clone();
        }
        let walk = Rc::new(Walk::new({
            let (rows, thread) = (rows.clone(), thread.clone());
            move || {
                rows.iter()
                    .enumerate()
                    .filter_map(|(index, row)| {
                        stop_of(row, thread.transcript(), strings, room).map(|stop| (index, stop))
                    })
                    .collect()
            }
        }));
        *cache = Some(ActivityLayout {
            rows: rows.clone(),
            stamp,
            walk: walk.clone(),
        });
        walk
    }
}

impl StatefulWidget for Pane {
    type State = ViewportState;

    fn create_state(&self) -> Self::State {
        ViewportState {
            layout: RefCell::default(),
            held_layout: RefCell::default(),
            activity: RefCell::default(),
            shown: RefCell::default(),
            first: RefCell::default(),
            width: Room::default().width,
            revealed_rows: RefCell::default(),
            scan_first: RefCell::default(),
            scan_match: RefCell::default(),
            left: Vec::new(),
            restoring: None,
            finding: RefCell::default(),
            folding: RefCell::default(),
            selecting: RefCell::default(),
            watching: RefCell::default(),
            moving: RefCell::default(),
            marked: RefCell::default(),
            texts: RefCell::default(),
            row_text_cache: RefCell::default(),
            texted: Cell::new(false),
            counted: Cell::new(0),
            anchor: Cell::new(None),
            targets: RefCell::default(),
        }
    }
}

impl State for ViewportState {
    type Widget = Pane;
    type Msg = Msg;

    fn did_update_widget(&mut self, old: &Pane, widget: &Pane, _cx: &mut Cx<'_, Self>) {
        if old.follow == widget.follow {
            return;
        }
        // another thread's reader: the one left keeps its place if it was away from the last
        // row, the one come to is put back where it was
        if let Some(restore) = self.restoring.take() {
            restore.disarm();
        }
        self.left.retain(|(follow, _)| *follow != old.follow);
        if !old.follow.at_tail()
            && let Ok(position) = old.follow.scroll().as_scroll_controller().try_position()
        {
            self.left.push((old.follow.clone(), position.pixels()));
        }
        if let Some(at) = self
            .left
            .iter()
            .position(|(follow, _)| *follow == widget.follow)
        {
            let (_, pixels) = self.left.remove(at);
            self.restoring = Some(Restore::arm(
                widget.follow.scroll().as_scroll_controller().clone(),
                pixels,
            ));
        }
    }

    fn dispose(&mut self) {
        if let Some(restore) = self.restoring.take() {
            restore.disarm();
        }
        if let Some((finder, listener)) = self.finding.get_mut().take() {
            finder.listenable().remove_listener(&listener);
        }
        if let Some((select, listener)) = self.selecting.get_mut().take() {
            select.listenable().remove_listener(&listener);
        }
        if let Some((follow, listener)) = self.moving.get_mut().take() {
            follow.moves_listenable().remove_listener(&listener);
        }
        if let Some((activity, listener)) = self.folding.get_mut().take() {
            activity.listenable().remove_listener(&listener);
        }
    }

    fn update(&mut self, msg: Msg, widget: &Pane, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Room(width) => {
                if self.width == width {
                    return Rebuild::No;
                }
                self.width = width;
                Rebuild::Yes
            }
            Msg::Load(thread) => {
                let handlers = read::<Hs, _>(cx);
                cx.emit(&handlers.load_history, thread);
                Rebuild::No
            }
            Msg::Beginning(thread, state) => {
                // the reader's wait for the first row, and the caret's of select mode (`g`)
                let caret = select::of(&widget.find);
                let (reading, selecting) = (widget.follow.beginning(), caret.beginning());
                if reading || selecting {
                    match state {
                        HistoryState::Partial => {
                            let handlers = read::<Hs, _>(cx);
                            cx.emit(&handlers.load_history, thread);
                        }
                        HistoryState::Complete => {
                            if reading {
                                widget.follow.finish_beginning();
                            }
                            caret.finish_beginning();
                        }
                        HistoryState::Failed => {
                            widget.follow.cancel_beginning();
                            caret.cancel_beginning();
                        }
                        HistoryState::Loading => {}
                    }
                }
                Rebuild::No
            }
            Msg::Toggle(ids) => {
                activity_focus::of(&widget.find).toggle(&ids);
                Rebuild::Yes
            }
            Msg::Find => Rebuild::Yes,
            Msg::Restore => {
                if let Some(pixels) = widget.find.go_back()
                    && let Some(binding) = AppBinding::maybe_of(cx)
                {
                    binding.add_post_frame_callback(
                        cx.map(move |_: Duration| Msg::Keep(pixels, KEEPS - 1)),
                    );
                }
                Rebuild::No
            }
            Msg::Anchored => {
                let pixels = widget
                    .follow
                    .scroll()
                    .as_scroll_controller()
                    .try_position()
                    .map(|position| position.pixels())
                    .ok();
                if let Some(mut anchor) = self.anchor.get() {
                    anchor.pixels = pixels;
                    self.anchor.set(Some(anchor));
                }
                Rebuild::No
            }
            Msg::KeepRow(index, blank, again) => {
                widget.follow.scroll().jump_to_index_rows(index, blank);
                if again > 0
                    && let Some(binding) = AppBinding::maybe_of(cx)
                {
                    binding.add_post_frame_callback(
                        cx.map(move |_: Duration| Msg::KeepRow(index, blank, again - 1)),
                    );
                }
                Rebuild::No
            }
            Msg::Reveal(index) => {
                widget.follow.scroll().ensure_index_visible(index);
                Rebuild::No
            }
            Msg::Keep(pixels, again) => {
                let controller = widget.follow.scroll().as_scroll_controller().clone();
                let moved = controller
                    .try_position()
                    .is_ok_and(|position| position.pixels() != pixels);
                if moved {
                    controller.jump_to(pixels);
                }
                if again > 0
                    && let Some(binding) = AppBinding::maybe_of(cx)
                {
                    binding.add_post_frame_callback(
                        cx.map(move |_: Duration| Msg::Keep(pixels, again - 1)),
                    );
                }
                Rebuild::No
            }
        }
    }

    fn build(&self, widget: &Pane, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let search = widget.find.clone();
        let caret = select::of(&search);
        let copier = copying::mode_of(&search);
        let reads = search.active() || caret.active() || caret.extending() || copier.active();
        if !reads {
            self.row_text_cache.borrow_mut().clear();
            self.texts.borrow_mut().take();
            self.targets.borrow_mut().take();
        }
        // Find, keyboard selection and copy mode read live rows. Detailed rows also read across
        // cells when arranging adjacent prompts and computer calls. The ordinary mouse range
        // uses its retained revision. TailNews watches incoming deltas separately.
        let cells = cells_strings::of(cx);
        let room = Room {
            width: self.width,
            segmenter: segmenter_of(cx),
        };
        let reads_across_rows = reads || widget.detailed || widget.mark.is_some();
        let live = if reads_across_rows {
            watch::<ThreadModel, _>(cx)
        } else {
            select::<ThreadModel, _, _>(cx, move |thread| {
                let transcript = thread.transcript();
                (
                    thread.id().clone(),
                    transcript.layout_revision(),
                    transcript.reset_revision(),
                    ActivityStamp::of(transcript, cells, room),
                    thread.can_load_history(),
                    thread.history_state(),
                    thread.read_only(),
                    thread.screen_slots().chat.cwd.clone(),
                )
            });
            read::<ThreadModel, _>(cx)
        };
        // while Find holds a match in the live tail the list draws the thread as it was found
        // there, until the reader steps, refines the query or Find closes
        let held = Held::of(&widget.find);
        if !widget.find.active() || widget.find.asked() || held.moved(widget.follow.moves()) {
            held.clear();
        }
        let selected_tool = SelectedTool::of(&search).read(&live, caret.active());
        let selection =
            SelectedRevision::of(&search).read(&live, &widget.follow, &widget.selection, !reads);
        let uses_snapshot = reads_across_rows || selection.is_some();
        let thread = held
            .thread()
            .or(selected_tool)
            .or(selection)
            .unwrap_or_else(|| live.clone());
        let strings = l10n::of(cx);
        let tokens = theme::Tokens::of(cx);
        let prompt_surface = tokens.prompt_surface(cx, true);
        // `"• ".dim()`: the terminal's own foreground, faint
        let agent_style = Style::new().attrs(Attrs::DIM);
        let bullet = strings.agent_line("").to_string();
        let table = chat_strings::of(cx);
        let (clock, today) = select::<ScreenStates, _, _>(cx, |states| {
            let clock = states.cells.clock;
            (
                clock,
                super::cells::separator::viewer_date(states.agents.now, clock),
            )
        });
        let metrics =
            select::<ScreenStates, _, _>(cx, |states| states.cells.runtime_metrics.clone());
        let facts = select::<ScreenStates, _, _>(cx, |states| states.chat.status.clone());
        let cwd = thread
            .screen_slots()
            .chat
            .cwd
            .as_deref()
            .or(facts.cwd.as_deref());
        let header = cwd.filter(|cwd| !cwd.is_empty()).map(|cwd| {
            super::transcript_header::header(cwd, &facts, &tokens, table)
                .with_key(Key::value("session header"))
        });
        let markdown = tokens.transcript_markdown(cx);
        let thread_id = thread.id().clone();
        let can_load = live.can_load_history();
        let detailed = widget.detailed;
        let raw = select::<ScreenStates, _, _>(cx, |states| states.chat.raw_output);
        let show_raw_reasoning =
            select::<ScreenStates, _, _>(cx, |states| states.cells.show_raw_reasoning);
        let collapse = select::<TuiSettings, _, _>(cx, |settings| settings.collapse_turn_details);
        let rendering = select::<TuiSettings, _, _>(cx, |settings| settings.rendering);
        let activity = activity_focus::of(&widget.find);
        let disclosed = activity.disclosed();
        // News follows the live transcript. A retained revision has its own row layout
        // so reading it cannot invalidate the cache for the incoming tail.
        let live_rows = Self::rows(&self.layout, &live, detailed, collapse, &activity);
        let tail = live_rows
            .rows
            .last()
            .and_then(|row| live.transcript().get(row.range.end - 1))
            .map(|(key, _)| key.clone());
        let Shown { rows, plan } = if Rc::ptr_eq(&live, &thread) {
            self.held_layout.borrow_mut().take();
            live_rows
        } else {
            Self::rows(&self.held_layout, &thread, detailed, collapse, &activity)
        };
        let reverse = !thread.read_only() && header.is_none();
        let header = header.filter(|_| !can_load);
        let lead = usize::from(header.is_some());
        // while Find is open or select mode is on the list reads its rows as text: the scan runs
        // over them, and the match it found (or the caret, or the range marked) is scrolled to and
        // selected, which is how the oracle draws it (REVERSED)
        let env = Env {
            cells,
            chat: table,
            detailed,
            expanded: false,
        };
        let usage =
            reads.then(|| select::<FeatureStates, _, _>(cx, |states| states.thread_usage.clone()));
        let card_figure = |number| usage.as_ref().and_then(|usage| usage.card_figure(number));
        let reading = Reading {
            env: &env,
            bullet: &bullet,
            cwd,
            rendering,
            raw,
            raw_reasoning: show_raw_reasoning,
            revealed: search.active(),
            room,
            status_words: super::cells::status::strings::of(cx),
            approval_words: crate::sheets::approvals::strings::of(cx),
            usage: usage.as_ref().map(|_| &card_figure as _),
        };
        let history = live.history_state();
        let more = match history {
            HistoryState::Partial => More::Loadable,
            HistoryState::Loading => More::Loading,
            HistoryState::Failed => More::Failed,
            HistoryState::Complete => More::None,
        };
        let load = cx.local_sender();
        if widget.follow.beginning() || caret.beginning() {
            load(Msg::Beginning(thread_id.clone(), history));
        }
        let scan = if search.active() && collapse && !detailed {
            thread
                .transcript()
                .iter()
                .enumerate()
                .filter(|(_, (_, item))| !item.is_detailed_only() && !is_terminal_interaction(item))
                .map(|(index, _)| Row {
                    range: index..index + 1,
                    kind: Kind::Cell,
                })
                .collect::<Rc<[Row]>>()
        } else {
            rows.clone()
        };
        let mut opened = false;
        let mut hit = None;
        if search.active() {
            self.reconcile_scan(&thread, &scan, lead, reverse, &search);
            if search.asked() {
                disclosed.set_found(None);
            }
            let texts = self.row_text_cache.borrow_mut().read(
                &thread,
                &scan,
                header.is_some(),
                reverse,
                &reading,
                None,
            );
            let scan_entry = |hit: &Hit| {
                let row = scan.get(hit.row.checked_sub(lead)?)?;
                let (key, _) = thread.transcript().get(row.range.start)?;
                Some((key, texts.scan_text(hit.row)))
            };
            let mut scan_match = self.scan_match.borrow_mut();
            if let Some((held_key, before)) = scan_match.as_ref()
                && let Some((key, now)) = search.current().as_ref().and_then(scan_entry)
                && key == held_key
            {
                search.reflow_match(before, now.as_ref());
            }
            hit = search.advance(&texts, more);
            match search.current().as_ref().and_then(scan_entry) {
                Some((key, text))
                    if scan_match.as_ref().is_none_or(|(old, previous)| {
                        old != key || previous.as_str() != text.as_ref()
                    }) =>
                {
                    *scan_match = Some((key.clone(), text.into_owned()));
                }
                Some(_) => {}
                None => {
                    scan_match.take();
                }
            }

            if search.take_load() {
                load(Msg::Load(thread.id().clone()));
            }
            if let Some(hit) = &hit
                && let Some(row) = hit.row.checked_sub(lead).and_then(|at| scan.get(at))
                && let Some((key, item)) = thread.transcript().get(row.range.start)
            {
                let before = disclosed.found();
                disclosed
                    .set_found(matches!(item, Item::Tool(_) | Item::Patch(_)).then(|| key.clone()));
                opened = disclosed.found() != before;
                if in_running_turn(&thread, key) {
                    held.set(thread.clone(), widget.follow.moves());
                }
            }
        } else {
            self.scan_first.borrow_mut().take();
            self.scan_match.borrow_mut().take();
            disclosed.set_found(None);
        }
        let current = search.active().then(|| search.current()).flatten();
        let current_row = current
            .as_ref()
            .and_then(|hit| hit.row.checked_sub(lead))
            .and_then(|at| scan.get(at));
        let current_entry = current_row.and_then(|row| thread.transcript().get(row.range.start));
        let rows = if collapse && !detailed {
            let mut revealed = disclosed.pins();
            revealed.extend(current_entry.map(|(key, _)| key.clone()));
            self.with_reveals(rows, &thread, revealed)
        } else {
            rows
        };
        let count = rows.len() + lead;
        let grew = self.counted.replace(count) != count;
        self.listen_folds(&activity, cx.local_sender());
        // the folds and the cells with a control the chat's own list has, for the keys that walk
        // them (F4); the Ctrl+T transcript has none, and leaves the chat's alone. A raw row has
        // no control.
        if detailed || raw {
            self.activity.borrow_mut().take();
        }
        if !detailed {
            let walk = (!raw).then(|| self.activity_walk(&rows, &thread, cells, reading.room));
            activity.set_folds(walk.as_ref());
            if let Some(ids) = activity.take_reveal()
                && let Some(logical) = walk.and_then(|walk| walk.row(&ids))
            {
                let logical = logical + usize::from(header.is_some());
                widget.follow.scroll().ensure_index_visible(if reverse {
                    count - 1 - logical
                } else {
                    logical
                });
            }
        }
        let marked = widget
            .mark
            .and_then(|nth| nth_user(thread.transcript(), nth))
            .map(|(key, _)| key.clone());
        self.show(&marked, &thread, &rows, header.is_some(), reverse, widget);
        if self.keep_first(
            &thread,
            &rows,
            usize::from(header.is_some()),
            reverse,
            widget,
        ) && let Some(binding) = AppBinding::maybe_of(cx)
        {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Anchored));
        }
        let folds = transcript_strings::of(cx);
        self.listen(&search, cx.local_sender());
        self.listen_moves(&widget.follow, cx.local_sender());

        // The list reads its rows as text while Find is open, select mode is on or copy mode is:
        // the widget of that is not the plain list, and the one that takes the place of the other
        // starts at the end. A reader who was away from the last row is put back after the frame
        // that swaps them (before the move Find's own close asks for).
        if reads != self.texted.replace(reads)
            && !widget.follow.at_tail()
            && self.restoring.is_none()
            && !search.going_back()
            && let Ok(position) = widget.follow.scroll().as_scroll_controller().try_position()
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let pixels = position.pixels();
            match self.anchor.take() {
                // the reader is where the list was put for a row: the list that takes the place
                // is put at the row again, not at the offset the other list measured
                Some(anchor)
                    if anchor.pixels == Some(pixels) && anchor.moves == widget.follow.moves() =>
                {
                    binding.add_post_frame_callback(
                        cx.map(move |_: Duration| Msg::KeepRow(anchor.index, anchor.blank, KEEPS)),
                    );
                }
                _ => binding
                    .add_post_frame_callback(cx.map(move |_: Duration| Msg::Keep(pixels, KEEPS))),
            }
        }
        // Find closed: the list builds again and takes no move made in the frame that does, so the
        // reader is put back after it
        if search.going_back()
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Restore));
        }
        self.listen_select(&caret, cx.local_sender());
        self.listen_selection(&widget.selection, cx.local_sender());
        self.mark(
            &markdown_copy::of(&search),
            &widget.selection,
            &thread,
            &rows,
            &reading,
        );
        let (texts, targets) = if reads {
            let texts = self.keep_texts(self.row_text_cache.borrow_mut().read(
                &thread,
                &rows,
                header.is_some(),
                reverse,
                &reading,
                Some(&disclosed),
            ));
            let targets = copier.cutter().filter(|_| copier.active()).map(|cutter| {
                self.targets_of(
                    &texts,
                    &thread,
                    &rows,
                    header.is_some(),
                    &reading,
                    (folds, &cutter),
                )
            });
            (Some(texts), targets)
        } else {
            (None, None)
        };
        if let Some(texts) = &texts {
            // Accepting into select mode makes the expansion part of the reader's view.
            if caret.read(texts) && search.reading() {
                if let Some(hit) = search
                    .current()
                    .and_then(|hit| shown_hit(hit, &scan, &rows, lead))
                {
                    if let Some((key, item)) = current_entry {
                        disclosed.pin(key, matches!(item, Item::Tool(_) | Item::Patch(_)));
                    }
                    caret.land(
                        Pos {
                            row: hit.row,
                            offset: hit.range.start,
                        },
                        &search.query().text(),
                    );
                }
                search.accepted();
            }
            if search.active() {
                if let Some(hit) = hit.and_then(|hit| shown_hit(hit, &scan, &rows, lead)) {
                    // A newly shown member belongs to the list after this frame.
                    if search.scrolls() {
                        let index = texts.index_of(hit.row);
                        match AppBinding::maybe_of(cx) {
                            Some(binding) if grew || opened => binding.add_post_frame_callback(
                                cx.map(move |_: Duration| Msg::Reveal(index)),
                            ),
                            _ => widget.follow.scroll().ensure_index_visible(index),
                        }
                    }
                    widget.selection.controller().select(
                        SelectionTarget::of(texts),
                        hit.row,
                        hit.range.start,
                        hit.range.end,
                    );
                }
            } else if copier.active() {
                disclosed.set_found(None);
                if let Some(targets) = &targets {
                    copier.read(targets);
                }
                copier.show(texts, &widget.selection);
                if let Some(row) = copier.reveal() {
                    widget
                        .follow
                        .scroll()
                        .ensure_index_visible(texts.index_of(row));
                }
            } else if caret.active() || caret.extending() {
                if caret.active() {
                    caret.show(texts, &widget.selection);
                } else {
                    caret.show_extension(texts, &widget.selection);
                }
                if let Some(row) = caret.reveal() {
                    widget
                        .follow
                        .scroll()
                        .ensure_index_visible(texts.index_of(row));
                }
            }
        } else {
            *self.texts.borrow_mut() = None;
        }
        if !search.active() {
            disclosed.set_found(None);
        }
        let toggled = cx.map(Msg::Toggle);
        let state = activity.clone();
        let control = tokens.dim().attrs(Attrs::DIM);
        let focus = tokens.accent().attrs(Attrs::BOLD);
        let cwd = cwd.map(str::to_owned);
        // where a row went when rows came in above it (an older page joined): the list keeps the
        // row the reader is at on its line (Flutter's `findChildIndexCallback`)
        let item_keys = thread.transcript().item_keys();
        let finder = {
            let (item_keys, rows) = (item_keys.clone(), rows.clone());
            move |key: &Key| -> Option<usize> {
                let logical = if lead == 1 && *key == Key::value("session header") {
                    0
                } else {
                    rows.iter().position(|row| {
                        item_keys
                            .get(row.range.start)
                            .is_some_and(|item| *key == Key::value(item.clone()))
                    })? + lead
                };
                Some(if reverse {
                    count - 1 - logical
                } else {
                    logical
                })
            }
        };
        let draw_rows = rows.clone();
        let draw: DrawRow = Rc::new(move |thread, listed| {
            let row = &draw_rows[listed - lead];
            let range = &row.range;
            let index = range.start;
            let transcript = thread.transcript();
            let Some((key, item)) = transcript.get(index) else {
                return AnyWidget::default();
            };
            let previous = index.checked_sub(1).and_then(|index| transcript.get(index));
            // The detailed transcript keeps reasoning between computer calls inside the
            // group. Only the reasoning gets an extra row, not the next call.
            let joins_computer = detailed
                && item.group() == Some(codex_tui_core::threads::ItemGroup::Computer)
                && (0..index)
                    .rev()
                    .map_while(|above| transcript.get(above))
                    .take_while(|(above, _)| above.turn == key.turn)
                    .find(|(_, item)| !matches!(item, Item::Reasoning { .. }))
                    .is_some_and(|(_, item)| {
                        item.group() == Some(codex_tui_core::threads::ItemGroup::Computer)
                    });
            let separator = (previous.is_some() || lead != 0) && !joins_computer;
            let place = Place {
                separator,
                // only the detailed transcript (oracle `TranscriptView`) draws prompts with no
                // turn between them as one block; the chat keeps each prompt's own rows
                joins_prompt: detailed
                    && {
                        // the nearest prompt or cell above that draws rows (a blank prompt draws none)
                        let drawn = (0..index).rev().find(|above| {
                        !matches!(transcript.get(*above), Some((_, Item::User(prompt))) if user::is_blank(prompt))
                    });
                        drawn.is_some_and(|above| {
                            matches!(transcript.get(above), Some((_, Item::User(_))))
                                && (above..index)
                                    .all(|turn| transcript.timing_after(turn).is_none())
                        })
                    },
                highlighted: marked.as_ref() == Some(key),
                fill: copier.fill_of(listed),
                streaming: transcript.is_agent_streaming(key),
                streamed_shell_output_lines: transcript.streamed_shell_output_line_count(key),
                rest: (index + 1..range.end)
                    .filter_map(|index| match transcript.get(index) {
                        Some((_, Item::Tool(tool))) => Some(tool.clone()),
                        _ => None,
                    })
                    .collect(),
            };
            let words = Words {
                table: folds,
                cells,
            };
            let timing_facts = |turn| super::cells::separator::Facts {
                today,
                clock,
                metrics: metrics.get(turn),
            };
            let fold = match &row.kind {
                Kind::Tools(group) if !group.is_lone_search(transcript) => {
                    let calls = group.calls(transcript, words);
                    let open = state.is_open(&group.ids);
                    let search = calls
                        .last()
                        .is_some_and(|call| call.starts_with(folds.search_prefix));
                    let label = group.label(transcript, &calls, words);
                    Some(
                        FoldRow {
                            anchor: Anchor::Calls,
                            control: format!("{} {label}", arrow(open, search, words)),
                            calls: if open { calls } else { Vec::new() },
                            ids: group.ids.clone(),
                            style: control.clone(),
                            focus: state.is_focused(&group.ids).then(|| focus.clone()),
                            toggled: toggled.clone(),
                        }
                        .into(),
                    )
                }
                _ => None,
            };
            let row_cell: AnyWidget = match fold {
                Some(fold) => fold,
                None => {
                    let drawn = cell(
                        item,
                        &CellEnv {
                            tokens: &tokens,
                            markdown: &markdown,
                            prompt_surface: &prompt_surface,
                            chat: table,
                            cells,
                            bullet: &bullet,
                            bullet_style: &agent_style,
                            cwd: cwd.as_deref(),
                            detailed,
                            raw,
                            show_raw_reasoning,
                        },
                        &place,
                    );
                    // a cell with a control is the thread's controller's: a tap, a key and Find
                    // open and shut it the same way
                    if matches!(item, Item::Tool(_) | Item::Patch(_)) {
                        tile(&activity, key.clone(), drawn)
                    } else {
                        drawn
                    }
                }
            };
            // The cell is always the first child of one column, so a timing row
            // arriving below it does not rebuild the cell's state (the patch
            // disclosure's `expanded`).
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(row_cell);
            if let Kind::Prompt(PromptFold { id, hidden, last }) = &row.kind {
                let timing = transcript.timing_after(*last).and_then(|timing| {
                    let turn = transcript.get(*last).map(|(key, _)| &key.turn)?;
                    super::cells::separator::label(timing, timing_facts(turn), cells)
                });
                let open = state.is_open(std::slice::from_ref(id));
                let label = prompt_label(*hidden, timing, words);
                column = column.child(FoldRow {
                    anchor: Anchor::Turn,
                    control: format!("{} {label}", arrow(open, false, words)),
                    calls: Vec::new(),
                    ids: vec![id.clone()],
                    style: control.clone(),
                    focus: state
                        .is_focused(std::slice::from_ref(id))
                        .then(|| focus.clone()),
                    toggled: toggled.clone(),
                });
            }
            let last = range.end - 1;
            if !plan.folds_timing(last)
                && let Some(timing) = transcript.timing_after(last)
            {
                column = column.children(super::cells::separator::completion(
                    timing,
                    timing_facts(&key.turn),
                    cells,
                ));
            }
            column.into()
        });
        let snapshot = uses_snapshot.then_some(thread);
        let listed_thread = thread_id.clone();
        let list = ListView::builder(count, move |index| {
            let listed = if reverse { count - 1 - index } else { index };
            if listed == 0
                && let Some(header) = &header
            {
                return header.clone();
            }
            let row = &rows[listed - lead];
            let Some(key) = item_keys.get(row.range.start) else {
                return AnyWidget::default();
            };
            let fold_end = match &row.kind {
                Kind::Prompt(fold) => Some(fold.last),
                _ => None,
            };
            let mounted: AnyWidget = MountedRow {
                thread: listed_thread.clone(),
                item: key.clone(),
                range: row.range.clone(),
                fold_end,
                listed,
                snapshot: snapshot.clone(),
                draw: draw.clone(),
            }
            .into();
            mounted.with_key(Key::value(key.clone()))
        })
        .find_index(finder)
        .controller(widget.follow.scroll())
        .stick_to_bottom(!reverse)
        .reverse(reverse)
        .shrink_wrap(reverse);
        let list: AnyWidget = match &texts {
            Some(texts) => list
                .selection_text(texts.clone() as Rc<dyn ItemTexts>)
                .into(),
            None => list.into(),
        };
        // A short reverse list reports its measured height to Align. Its live rows can
        // grow during layout without rebuilding the list widget.
        let list: AnyWidget = if reverse {
            Align::new(Alignment::BOTTOM_LEFT, list).into()
        } else {
            list
        };
        let list = AnyWidget::from(WheelPolicy { child: list }).with_key(Key::value(reverse));
        let send = cx.local_sender();
        let follow = widget.follow.clone();
        // every thread owns its reader (#57499; Flutter's `PageStorageKey` per list): the list of
        // another thread is another element, so the offset the render list held is never taken
        // for the controller of the thread now shown
        let loading = thread_id.clone();
        let list: AnyWidget = NotificationListener::<ScrollNotification>::new(list)
            .on_notification(move |scroll| {
                let metrics = scroll.metrics;
                let distance = if reverse {
                    metrics.max_scroll_extent.saturating_sub(metrics.pixels)
                } else {
                    metrics.pixels
                };
                if can_load && scroll.depth() == 0 && distance <= 3 {
                    send(Msg::Load(loading.clone()));
                }
                // the wheel: a move nobody asked for supersedes the wait for the first row
                follow.moved_to(metrics.pixels);
                // back at the last row: the news is seen
                if !follow.away() {
                    follow.clear_unseen();
                } else if follow.selection_cleared() && !follow.at_tail() {
                    send(Msg::Find);
                }
                false
            })
            .into();
        // the wheel at an end moves nothing, and still is the reader's
        let wheel = widget.follow.clone();
        let list: AnyWidget = NotificationListener::<OverscrollNotification>::new(list)
            .on_notification(move |scroll| {
                wheel.cancel_beginning();
                if scroll.depth() == 0
                    && (scroll.overscroll < 0) == reverse
                    && wheel.selection_cleared()
                    && wheel.at_tail()
                {
                    wheel.jump_to_latest();
                }
                false
            })
            .into();
        let list = thread_scope(&thread_id, list);
        let host = SelectionHost {
            child: list,
            thread: thread_id,
            selection: widget.selection.clone(),
            follow: widget.follow.clone(),
            detailed,
            raw,
            caret: caret.caret(),
            find: widget.find.clone(),
        };
        let tail_news: AnyWidget = TailNews {
            item: tail,
            follow: widget.follow.clone(),
            child: host.into(),
        }
        .into();
        let width = self.width;
        let changed = cx.local_sender();
        LayoutBuilder::new(move |_, bounds| {
            if bounds.max_width != width {
                changed(Msg::Room(bounds.max_width));
            }
            tail_news.clone()
        })
        .into()
    }
}

impl_widget!(stateful eq Pane);

/// A list in one thread scope and direction keeps its position when an optional
/// native wheel policy is inserted or removed.
#[derive(Clone, PartialEq)]
struct WheelPolicy {
    child: AnyWidget,
}

struct WheelPolicyState {
    list_key: GlobalKey,
}

impl StatefulWidget for WheelPolicy {
    type State = WheelPolicyState;

    fn create_state(&self) -> Self::State {
        WheelPolicyState {
            list_key: GlobalKey::new(),
        }
    }
}

impl State for WheelPolicyState {
    type Widget = WheelPolicy;
    type Msg = ();

    fn build(&self, widget: &WheelPolicy, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let step = select::<TuiSettings, _, _>(cx, |settings| settings.wheel_rows_per_event);
        let list = widget.child.clone().with_key(&self.list_key);
        match step {
            Some(step) => {
                // Keep positive speeds representable and each native delta within i32, even
                // across the longest stream that rtwk accumulates in i64.
                let max_step = (i32::MAX as f32).next_down();
                let step = (step as f32).clamp(f32::from_bits(1), max_step);
                ScrollConfiguration::new(ScrollBehavior::fixed(step), list).into()
            }
            None => list,
        }
    }
}

impl_widget!(stateful eq WheelPolicy);

type DrawRow = Rc<dyn Fn(&ThreadModel, usize) -> AnyWidget>;

/// A mounted item keeps its identity as the provider publishes new versions of its contents.
#[derive(Clone)]
struct MountedRow {
    thread: ThreadId,
    item: ItemKey,
    range: Range<usize>,
    fold_end: Option<usize>,
    listed: usize,
    snapshot: Option<Rc<ThreadModel>>,
    draw: DrawRow,
}

impl PartialEq for MountedRow {
    fn eq(&self, other: &Self) -> bool {
        // The drawing function and the row's inputs are supplied together by the list builder.
        self.listed == other.listed && Rc::ptr_eq(&self.draw, &other.draw)
    }
}

impl StatelessWidget for MountedRow {
    fn build(&self, cx: &mut BuildCx<'_>) -> AnyWidget {
        let Some(snapshot) = &self.snapshot else {
            let (thread_id, item, range, fold_end) = (
                self.thread.clone(),
                self.item.clone(),
                self.range.clone(),
                self.fold_end,
            );
            let stamp = select::<ThreadModel, _, _>(cx, move |thread| {
                let transcript = thread.transcript();
                if thread.id() != &thread_id || transcript.get(range.start)?.0 != &item {
                    return None;
                }
                // A group reads every item it spans. Revisions increase across the transcript,
                // so an update to any of those items changes the group's maximum.
                let revision = range
                    .clone()
                    .filter_map(|at| transcript.item_revision(transcript.get(at)?.0))
                    .max();
                Some((
                    revision,
                    transcript.is_agent_streaming(&item),
                    transcript.timing_after(range.end - 1),
                    fold_end.and_then(|end| transcript.timing_after(end)),
                ))
            });
            return if stamp.is_some() {
                (self.draw)(&read::<ThreadModel, _>(cx), self.listed)
            } else {
                AnyWidget::default()
            };
        };
        select::<ThreadModel, _, _>(cx, |_| ());
        (self.draw)(snapshot, self.listed)
    }
}

impl_widget!(stateless eq MountedRow);

/// The reader's footer and follow control observe the visible live tail separately from the list.
#[derive(Clone, PartialEq)]
struct TailNews {
    item: Option<ItemKey>,
    follow: Follow,
    child: AnyWidget,
}

#[derive(Default)]
struct TailState {
    last: RefCell<Option<(ThreadId, ItemKey, u64)>>,
}

impl StatefulWidget for TailNews {
    type State = TailState;

    fn create_state(&self) -> TailState {
        TailState::default()
    }
}

impl State for TailState {
    type Widget = TailNews;
    type Msg = ();

    fn build(&self, widget: &TailNews, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let item = widget.item.clone();
        let tail = select::<ThreadModel, _, _>(cx, move |thread| {
            let item = item.as_ref()?;
            let revision = thread.transcript().item_revision(item)?;
            Some((thread.id().clone(), item.clone(), revision))
        });
        if let (Some(before), Some(now)) = (&*self.last.borrow(), &tail)
            && before.0 == now.0
            && before != now
        {
            widget.follow.note_activity();
        }
        self.last.replace(tail);
        widget.child.clone()
    }
}

impl_widget!(stateful eq TailNews);
