//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hasher};
use std::rc::Rc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::{ListView, ScrollNotification};
use rtwk::style::Attrs;
use rtwk::{AnyWidget, Cx, Key, NotificationListener, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::backtrack::nth_user;
use codex_tui_core::threads::{Item, ItemKey, ThreadModel};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::widgets::thread_ui::{Follow, TextSelection};

use super::cells::strings as cells_strings;
use super::cells::{CellEnv, Place, cell};
use super::strings as chat_strings;
use super::transcript::selection::SelectionHost;
use super::transcript::strings as transcript_strings;
use super::transcript::turn_details::{
    Anchor, FoldRow, Kind, Plan, PromptFold, Row, Unfolded, Words, arrow, prompt_label,
};

/// The transcript with a prompt marked (Flutter: a `ListView` whose item
/// `selected: true` is drawn highlighted and scrolled into view when it
/// changes): the nth prompt a browsing sheet offers, reversed in its body.
#[derive(Clone, PartialEq)]
pub(super) struct Pane {
    pub follow: Follow,
    /// What the reader has selected in the list: the one controller of the thread's `ThreadUi`.
    pub selection: TextSelection,
    pub detailed: bool,
    pub mark: Option<usize>,
}

pub(super) struct ViewportState {
    layout: RefCell<Option<RowLayout>>,
    /// The folds the reader opened, and a counter that tells the rows it changed.
    open: Unfolded,
    opened: u64,
    /// The prompt the list was last scrolled to show.
    shown: RefCell<Option<ItemKey>>,
    /// The first row the list showed, to see rows come in above it.
    first: RefCell<Option<(ThreadId, ItemKey)>>,
    /// The last visible row the reader's thread had (its key and a fingerprint of it, not a copy:
    /// the thread model is the one place the row lives), to see it change: a row after it, or
    /// the row itself grown; rows above it (an older page) are none of the reader's news.
    tail: Option<(ThreadId, ItemKey, u64)>,
}

struct RowLayout {
    thread: ThreadId,
    revision: u64,
    detailed: bool,
    collapse: bool,
    opened: u64,
    shown: Shown,
}

/// The rows of the list and the plan of the folds they were made from.
#[derive(Clone)]
struct Shown {
    rows: Rc<[Row]>,
    plan: Rc<Plan>,
}

/// What the list's state is told: older history is wanted, or a fold was tapped.
pub(super) enum Msg {
    Load(ThreadId),
    Toggle(Vec<ItemKey>),
}

impl ViewportState {
    /// The visible rows: runs of explorations share one, and the reasoning
    /// summary and the warnings show only in the detailed transcript (oracle
    /// `ReasoningSummaryCell::transcript_only`, `WarningHistoryCell`). With the
    /// turn details collapsed the plan of the folds decides instead.
    fn rows(&self, thread: &ThreadModel, detailed: bool, collapse: bool) -> Shown {
        let mut cache = self.layout.borrow_mut();
        let transcript = thread.transcript();
        let revision = transcript.layout_revision();
        if let Some(layout) = cache.as_ref()
            && layout.thread == *thread.id()
            && layout.revision == revision
            && layout.detailed == detailed
            && layout.collapse == collapse
            && layout.opened == self.opened
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
            let rows = plan.rows(transcript, &self.open, hidden);
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
            opened: self.opened,
            shown: shown.clone(),
        });
        shown
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
    ) {
        let transcript = thread.transcript();
        let first = rows
            .first()
            .and_then(|row| transcript.get(row.range.start))
            .map(|(key, _)| (thread.id().clone(), key.clone()));
        let before = self.first.replace(first.clone());
        let (Some(before), Some(now)) = (before, first) else {
            return;
        };
        if reverse || before.0 != now.0 || before.1 == now.1 || !widget.follow.away() {
            return;
        }
        let at_start = widget
            .follow
            .scroll()
            .as_scroll_controller()
            .try_position()
            .is_ok_and(|position| position.pixels() <= 0);
        let moved = rows.iter().position(|row| {
            transcript
                .get(row.range.start)
                .is_some_and(|(key, _)| *key == before.1)
        });
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
        }
    }
}

/// The write to a background terminal shows only in the detailed
/// transcript (oracle `UnifiedExecInteractionCell` has no display lines).
fn is_terminal_interaction(item: &Item) -> bool {
    matches!(item, Item::Notice(notice) if notice.detailed_only())
}

/// A hash of what the row shows, to tell that it changed (`Item` has no
/// `Hash`; its `Debug` text is written into the hasher, not collected).
fn fingerprint(item: &Item) -> u64 {
    struct Sink(DefaultHasher);
    impl std::fmt::Write for Sink {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0.write(text.as_bytes());
            Ok(())
        }
    }
    let mut sink = Sink(DefaultHasher::new());
    let _ = write!(sink, "{item:?}");
    sink.0.finish()
}

impl StatefulWidget for Pane {
    type State = ViewportState;

    fn create_state(&self) -> Self::State {
        ViewportState {
            layout: RefCell::default(),
            open: Unfolded::default(),
            opened: 0,
            shown: RefCell::default(),
            first: RefCell::default(),
            tail: None,
        }
    }
}

impl State for ViewportState {
    type Widget = Pane;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, widget: &Pane, cx: &mut Cx<'_, Self>) {
        // The end of the transcript changed while the reader is away (a new
        // row, or the last one grew): news for the control and the footer
        // (`sync_history_tail`). A thread the reader has just opened has none.
        let thread = watch::<ThreadModel, _>(cx);
        let collapse = select::<TuiSettings, _, _>(cx, |settings| settings.collapse_turn_details);
        let shown = self.rows(&thread, widget.detailed, collapse);
        let transcript = thread.transcript();
        let tail = shown
            .rows
            .last()
            .and_then(|row| transcript.get(row.range.end - 1))
            .map(|(key, item)| (thread.id().clone(), key.clone(), fingerprint(item)));
        if let (Some(before), Some(now)) = (&self.tail, &tail)
            && before.0 == now.0
            && before != now
        {
            widget.follow.note_activity();
        }
        self.tail = tail;
    }

    fn update(&mut self, msg: Msg, _: &Pane, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Load(thread) => {
                let handlers = read::<Hs, _>(cx);
                cx.emit(&handlers.load_history, thread);
                Rebuild::No
            }
            Msg::Toggle(ids) => {
                self.open.toggle(&ids);
                self.opened += 1;
                Rebuild::Yes
            }
        }
    }

    fn build(&self, widget: &Pane, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let thread = watch::<ThreadModel, _>(cx);
        let strings = l10n::of(cx);
        let tokens = theme::Tokens::of(cx);
        let prompt_surface = tokens.prompt_surface(cx, true);
        let agent_style = tokens.dim().attrs(Attrs::DIM);
        let bullet = strings.agent_line("").to_string();
        let table = chat_strings::of(cx);
        let now = rtwk::provider::read::<ScreenStates, _>(cx).agents.now;
        let clock = select::<ScreenStates, _, _>(cx, |states| states.cells.clock);
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
        let can_load = thread.can_load_history();
        let detailed = widget.detailed;
        let raw = select::<ScreenStates, _, _>(cx, |states| states.chat.raw_output);
        let show_raw_reasoning =
            select::<ScreenStates, _, _>(cx, |states| states.cells.show_raw_reasoning);
        let collapse = select::<TuiSettings, _, _>(cx, |settings| settings.collapse_turn_details);
        let Shown { rows, plan } = self.rows(&thread, detailed, collapse);
        let reverse = !thread.read_only() && header.is_none();
        let header = header.filter(|_| !can_load);
        let count = rows.len() + usize::from(header.is_some());
        let marked = widget
            .mark
            .and_then(|nth| nth_user(thread.transcript(), nth))
            .map(|(key, _)| key.clone());
        self.show(&marked, &thread, &rows, header.is_some(), reverse, widget);
        self.keep_first(
            &thread,
            &rows,
            usize::from(header.is_some()),
            reverse,
            widget,
        );
        let cells = cells_strings::of(cx);
        let folds = transcript_strings::of(cx);
        let toggled = cx.map(Msg::Toggle);
        let open = self.open.clone();
        let control = tokens.dim().attrs(Attrs::DIM);
        let cwd = cwd.map(str::to_owned);
        // where a row went when rows came in above it (an older page joined): the list keeps the
        // row the reader is at on its line (Flutter's `findChildIndexCallback`)
        let finder = {
            let (thread, rows) = (thread.clone(), rows.clone());
            let lead = usize::from(header.is_some());
            move |key: &Key| -> Option<usize> {
                let logical = if lead == 1 && *key == Key::value("session header") {
                    0
                } else {
                    rows.iter().position(|row| {
                        thread
                            .transcript()
                            .get(row.range.start)
                            .is_some_and(|(item, _)| *key == Key::value(item.clone()))
                    })? + lead
                };
                Some(if reverse {
                    count - 1 - logical
                } else {
                    logical
                })
            }
        };
        let list = ListView::builder(count, move |index| {
            let index = if reverse { count - 1 - index } else { index };
            if index == 0
                && let Some(header) = &header
            {
                return header.clone();
            }
            let row = &rows[index - usize::from(header.is_some())];
            let range = &row.range;
            let index = range.start;
            let transcript = thread.transcript();
            let Some((key, item)) = transcript.get(index) else {
                return AnyWidget::default();
            };
            let previous = index.checked_sub(1).and_then(|index| transcript.get(index));
            let separator = previous.is_some() || header.is_some();
            let place = Place {
                separator,
                joins_prompt: matches!(previous, Some((_, Item::User(_))))
                    && transcript.timing_after(index.saturating_sub(1)).is_none(),
                highlighted: marked.as_ref() == Some(key),
                streaming: transcript.is_agent_streaming(key),
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
                now,
                clock,
                metrics: metrics.get(turn),
            };
            let fold = match &row.kind {
                Kind::Tools(group) if !group.is_lone_search(transcript) => {
                    let calls = group.calls(transcript, words);
                    let open = open.is_open(&group.ids);
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
                            toggled: toggled.clone(),
                        }
                        .into(),
                    )
                }
                _ => None,
            };
            let row_cell: AnyWidget = match fold {
                Some(fold) => fold,
                None => cell(
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
                ),
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
                let open = open.is_open(std::slice::from_ref(id));
                let label = prompt_label(*hidden, timing, words);
                column = column.child(FoldRow {
                    anchor: Anchor::Turn,
                    control: format!("{} {label}", arrow(open, false, words)),
                    calls: Vec::new(),
                    ids: vec![id.clone()],
                    style: control.clone(),
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
            let row: AnyWidget = column.into();
            row.with_key(Key::value(key.clone()))
        })
        .find_index(finder)
        .controller(widget.follow.scroll())
        .stick_to_bottom(!reverse)
        .reverse(reverse);
        let send = cx.local_sender();
        let follow = widget.follow.clone();
        let list: AnyWidget = NotificationListener::<ScrollNotification>::new(list)
            .on_notification(move |scroll| {
                let metrics = scroll.metrics;
                let distance = if reverse {
                    metrics.max_scroll_extent.saturating_sub(metrics.pixels)
                } else {
                    metrics.pixels
                };
                if can_load && scroll.depth() == 0 && distance <= 3 {
                    send(Msg::Load(thread_id.clone()));
                }
                // back at the last row: the news is seen
                if follow.at_tail() {
                    follow.clear_unseen();
                }
                false
            })
            .into();
        SelectionHost {
            child: list,
            selection: widget.selection.clone(),
            follow: widget.follow.clone(),
        }
        .into()
    }
}

impl_widget!(stateful eq Pane);
