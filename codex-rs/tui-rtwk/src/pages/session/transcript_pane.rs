//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use rtwk::gestures::SelectionArea;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, NotificationListener, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::{ListSpacing, MarkdownTheme, MarkdownView};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::{Item, ThreadModel};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;

use super::cells::strings as cells_strings;
use super::strings as chat_strings;

#[derive(Clone, PartialEq)]
pub(super) struct TranscriptPane(pub ListController, pub bool);

pub(super) struct ViewportState {
    layout: RefCell<Option<RowLayout>>,
}

struct RowLayout {
    thread: ThreadId,
    revision: u64,
    detailed: bool,
    rows: Rc<[Range<usize>]>,
}

impl ViewportState {
    /// The visible rows: runs of explorations share one, and the reasoning
    /// summary shows only in the detailed transcript (oracle
    /// `ReasoningSummaryCell::transcript_only`).
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
            if !detailed && matches!(item, Item::Reasoning { .. }) {
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

impl StatefulWidget for TranscriptPane {
    type State = ViewportState;

    fn create_state(&self) -> Self::State {
        ViewportState {
            layout: RefCell::default(),
        }
    }
}

impl State for ViewportState {
    type Widget = TranscriptPane;
    type Msg = ThreadId;

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
                    let list_spacing = if transcript.is_agent_streaming(key) {
                        ListSpacing::Compact
                    } else {
                        ListSpacing::Uniform
                    };
                    let source = super::cells::markdown::unwrap_markdown_fences(text);
                    let markdown = MarkdownView::new(source.as_ref()).theme(MarkdownTheme {
                        list_spacing,
                        ..markdown.clone()
                    });
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
                    super::cells::summary::web(activity, &tokens, cells),
                ),
                Item::ImageView { name, .. } => cell(
                    separator,
                    super::cells::summary::viewed_image(name, &tokens, cells),
                ),
                Item::Notice(notice) => cell(
                    separator,
                    super::cells::summary::notice(notice, &tokens, cells),
                ),
                Item::Plan { text } => cell(
                    separator,
                    super::cells::plan::proposed_plan(
                        text,
                        prompt_surface.clone(),
                        &markdown,
                        &tokens,
                        cells,
                    ),
                ),
                Item::Reasoning { summary } => cell(
                    separator,
                    super::cells::plan::reasoning(summary, &markdown, &tokens, cells),
                ),
            };
            let row = if let Some(timing) = transcript.timing_after(range.end - 1) {
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(row)
                    .child(super::transcript_activity::completion(
                        timing, now, table, &tokens,
                    ))
                    .into()
            } else {
                row
            };
            row.with_key(Key::value(key.clone()))
        })
        .controller(widget.0.clone())
        .stick_to_bottom(!reverse)
        .reverse(reverse);
        let send = cx.local_sender();
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
