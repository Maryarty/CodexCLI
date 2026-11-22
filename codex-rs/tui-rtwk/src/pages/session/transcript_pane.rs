//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hasher};
use std::ops::Range;
use std::rc::Rc;

use rtwk::gestures::SelectionArea;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::{ListView, ScrollNotification};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, NotificationListener, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::{Item, ThreadModel};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::widgets::thread_ui::Follow;

use super::cells::strings as cells_strings;
use super::strings as chat_strings;

#[derive(Clone, PartialEq)]
pub(super) struct TranscriptPane(pub Follow, pub bool);

pub(super) struct ViewportState {
    layout: RefCell<Option<RowLayout>>,
    /// The last visible row the reader's thread had (a fingerprint of it, not
    /// a copy: the thread model is the one place the row lives), to see it
    /// change.
    tail: Option<(ThreadId, usize, u64)>,
}

struct RowLayout {
    thread: ThreadId,
    revision: u64,
    detailed: bool,
    rows: Rc<[Range<usize>]>,
}

impl ViewportState {
    /// The visible rows: runs of explorations share one, and the reasoning
    /// summary and the warnings show only in the detailed transcript (oracle
    /// `ReasoningSummaryCell::transcript_only`, `WarningHistoryCell`).
    fn rows(&self, thread: &ThreadModel, detailed: bool) -> Rc<[Range<usize>]> {
        let mut cache = self.layout.borrow_mut();
        let transcript = thread.transcript();
        let revision = transcript.layout_revision();
        if let Some(layout) = cache.as_ref()
            && layout.thread == *thread.id()
            && layout.revision == revision
            && layout.detailed == detailed
        {
            return layout.rows.clone();
        }
        let mut rows: Vec<Range<usize>> = Vec::new();
        let mut exploration = None;
        for (index, (key, item)) in transcript.iter().enumerate() {
            if !detailed && item.is_detailed_only() {
                continue;
            }
            let turn = item.is_exploration().then_some(&key.turn);
            if turn.is_some()
                && turn == exploration
                && let Some(group) = rows.last_mut()
            {
                group.end = index + 1;
            } else {
                rows.push(index..index + 1);
            }
            exploration = turn;
        }
        let rows: Rc<[_]> = rows.into();
        *cache = Some(RowLayout {
            thread: thread.id().clone(),
            revision,
            detailed,
            rows: rows.clone(),
        });
        rows
    }
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

impl StatefulWidget for TranscriptPane {
    type State = ViewportState;

    fn create_state(&self) -> Self::State {
        ViewportState {
            layout: RefCell::default(),
            tail: None,
        }
    }
}

impl State for ViewportState {
    type Widget = TranscriptPane;
    type Msg = ThreadId;

    fn did_change_dependencies(&mut self, widget: &TranscriptPane, cx: &mut Cx<'_, Self>) {
        // The end of the transcript changed while the reader is away (a new
        // row, or the last one grew): news for the control and the footer
        // (`sync_history_tail`). A thread the reader has just opened has none.
        let thread = watch::<ThreadModel, _>(cx);
        let rows = self.rows(&thread, widget.1);
        let transcript = thread.transcript();
        let tail = rows
            .last()
            .and_then(|range| transcript.get(range.end - 1))
            .map(|(_, item)| (thread.id().clone(), rows.len(), fingerprint(item)));
        if let (Some(before), Some(now)) = (&self.tail, &tail)
            && before.0 == now.0
            && before != now
        {
            widget.0.note_activity();
        }
        self.tail = tail;
    }

    fn update(&mut self, thread: ThreadId, _: &TranscriptPane, cx: &mut Cx<'_, Self>) -> Rebuild {
        let handlers = read::<Hs, _>(cx);
        cx.emit(&handlers.load_history, thread);
        Rebuild::No
    }

    fn build(&self, widget: &TranscriptPane, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let thread = watch::<ThreadModel, _>(cx);
        let strings = l10n::of(cx);
        let tokens = theme::Tokens::of(cx);
        let prompt_surface = tokens.prompt_surface(cx, true);
        let agent_style = tokens.dim().attrs(Attrs::DIM);
        let table = chat_strings::of(cx);
        let now = rtwk::provider::read::<ScreenStates, _>(cx).agents.now;
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
        let detailed = widget.1;
        let rows = self.rows(&thread, detailed);
        let reverse = !thread.read_only() && header.is_none();
        let header = header.filter(|_| !can_load);
        let count = rows.len() + usize::from(header.is_some());
        let cells = cells_strings::of(cx);
        let cwd = cwd.map(str::to_owned);
        let list = ListView::builder(count, move |index| {
            let index = if reverse { count - 1 - index } else { index };
            if index == 0
                && let Some(header) = &header
            {
                return header.clone();
            }
            let range = &rows[index - usize::from(header.is_some())];
            let index = range.start;
            let transcript = thread.transcript();
            let Some((key, item)) = transcript.get(index) else {
                return AnyWidget::default();
            };
            let previous = index.checked_sub(1).and_then(|index| transcript.get(index));
            let separator = previous.is_some() || header.is_some();
            let row: AnyWidget = match item {
                Item::User(prompt) => {
                    let joins_prompt = matches!(previous, Some((_, Item::User(_))))
                        && transcript.timing_after(index.saturating_sub(1)).is_none();
                    let panel = super::transcript_prompt::panel(
                        prompt,
                        prompt_surface.clone(),
                        &tokens,
                        table,
                        joins_prompt,
                        &markdown,
                    );
                    // A separator belongs to the transcript, outside the prompt surface.
                    Padding::new(
                        EdgeInsets::only(0, i32::from(separator && !joins_prompt), 0, 0),
                        panel,
                    )
                    .into()
                }
                Item::Agent { text } => {
                    let markdown = crate::widgets::markdown::themed(
                        super::cells::citation::view(text, cwd.as_deref()),
                        markdown.clone(),
                    );
                    Padding::new(
                        EdgeInsets::only(0, i32::from(separator), 1, 0),
                        Row::new()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .spacing(1)
                            .child(
                                Text::new(strings.agent_line("").to_string())
                                    .style(agent_style.clone()),
                            )
                            .child(Expanded::new(1, markdown)),
                    )
                    .into()
                }
                Item::Tool(tool) => Padding::new(
                    EdgeInsets::only(0, i32::from(separator), 1, 0),
                    super::transcript_activity::ToolRow {
                        first: tool.clone(),
                        rest: (index + 1..range.end)
                            .filter_map(|index| match transcript.get(index) {
                                Some((_, Item::Tool(tool))) => Some(tool.clone()),
                                _ => None,
                            })
                            .collect(),
                        detailed,
                    },
                )
                .into(),
                Item::Patch(activity) => cell(
                    separator,
                    super::cells::patch::PatchCell {
                        activity: activity.clone(),
                        cwd: cwd.clone(),
                        detailed,
                    }
                    .into(),
                ),
                Item::Web(activity) => cell(
                    separator,
                    super::cells::web::WebCell {
                        activity: activity.clone(),
                    }
                    .into(),
                ),
                Item::ImageView { name, path } => cell(
                    separator,
                    super::cells::viewed_image::ViewedImageCell {
                        name: name.clone(),
                        path: path.clone(),
                        detailed,
                    }
                    .into(),
                ),
                Item::Notice(notice) => cell(
                    separator,
                    super::cells::notice::NoticeCell {
                        notice: notice.clone(),
                    }
                    .into(),
                ),
                Item::Plan { text } => cell(
                    separator,
                    super::cells::plan::proposed_plan(
                        text,
                        cwd.as_deref(),
                        prompt_surface.clone(),
                        &markdown,
                        &tokens,
                        cells,
                    ),
                ),
                Item::Delegated(message) => cell(
                    separator,
                    super::cells::delegated::DelegatedCell {
                        message: message.clone(),
                    }
                    .into(),
                ),
                Item::Generated(image) => cell(
                    separator,
                    super::cells::generated_image::GeneratedImageCell {
                        image: image.clone(),
                    }
                    .into(),
                ),
                Item::Collab(event) => cell(
                    separator,
                    super::cells::collab::CollabCell {
                        event: event.clone(),
                    }
                    .into(),
                ),
                Item::Hook(run) => cell(
                    separator,
                    super::cells::hook::HookCell { run: run.clone() }.into(),
                ),
                Item::PlanUpdate(update) => cell(
                    separator,
                    super::cells::plan_update::PlanUpdateCell {
                        update: update.clone(),
                        detailed,
                    }
                    .into(),
                ),
                Item::Error { message } => cell(
                    separator,
                    super::cells::error::ErrorCell {
                        message: message.clone(),
                    }
                    .into(),
                ),
                Item::Deprecation { summary, details } => cell(
                    separator,
                    super::cells::deprecation::DeprecationCell {
                        summary: summary.clone(),
                        details: details.clone(),
                    }
                    .into(),
                ),
                Item::Warning { message } => cell(
                    separator,
                    super::cells::warning::WarningCell {
                        message: message.clone(),
                    }
                    .into(),
                ),
                Item::Reasoning { summary } => cell(
                    separator,
                    super::cells::reasoning::ReasoningCell {
                        summary: summary.clone(),
                    }
                    .into(),
                ),
            };
            // The cell is always the first child of one column, so a timing row
            // arriving below it does not rebuild the cell's state (the patch
            // disclosure's `expanded`).
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(row);
            if let Some(timing) = transcript.timing_after(range.end - 1) {
                column = column.child(super::transcript_activity::completion(
                    timing, now, table, &tokens,
                ));
            }
            let row: AnyWidget = column.into();
            row.with_key(Key::value(key.clone()))
        })
        .controller(widget.0.scroll())
        .stick_to_bottom(!reverse)
        .reverse(reverse);
        let send = cx.local_sender();
        let follow = widget.0.clone();
        SelectionArea::new(
            NotificationListener::<ScrollNotification>::new(list).on_notification(move |scroll| {
                let metrics = scroll.metrics;
                let distance = if reverse {
                    metrics.max_scroll_extent.saturating_sub(metrics.pixels)
                } else {
                    metrics.pixels
                };
                if can_load && scroll.depth() == 0 && distance <= 3 {
                    send(thread_id.clone());
                }
                // back at the last row: the news is seen
                if follow.at_tail() {
                    follow.clear_unseen();
                }
                false
            }),
        )
        .into()
    }
}

impl_widget!(stateful eq TranscriptPane);

/// A cell below the one before it, a blank row between them; it takes the
/// whole width (only the agent's answer keeps a column free, oracle
/// messages.rs `right_reserve = 1`).
fn cell(separator: bool, child: AnyWidget) -> AnyWidget {
    Padding::new(EdgeInsets::only(0, i32::from(separator), 0, 0), child).into()
}
