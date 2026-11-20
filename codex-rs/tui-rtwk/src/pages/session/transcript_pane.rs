//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use rtwk::gestures::SelectionArea;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::provider::{read, watch};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, NotificationListener, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::MarkdownView;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::{Item, ThreadModel};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;

use super::strings as chat_strings;

#[derive(Clone, PartialEq)]
pub(super) struct TranscriptPane(pub ListController);

pub(super) struct ViewportState;

impl StatefulWidget for TranscriptPane {
    type State = ViewportState;

    fn create_state(&self) -> Self::State {
        ViewportState
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
        let thread_id = thread.id().clone();
        let can_load = thread.can_load_history();
        let count = thread.transcript().len();
        // In the external-writer view, short history begins at the top.
        let reverse = !thread.read_only();
        let list = ListView::builder(count, move |index| {
            let index = if reverse { count - 1 - index } else { index };
            let transcript = thread.transcript();
            let Some((key, item)) = transcript.get(index) else {
                return AnyWidget::default();
            };
            let previous = index.checked_sub(1).and_then(|index| transcript.get(index));
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
                    );
                    // A separator belongs to the transcript, outside the prompt surface.
                    Padding::new(
                        EdgeInsets::only(0, i32::from(previous.is_some() && !joins_prompt), 0, 0),
                        panel,
                    )
                    .into()
                }
                Item::Agent { text } => Padding::new(
                    EdgeInsets::only(0, i32::from(previous.is_some()), 1, 0),
                    Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .spacing(1)
                        .child(
                            Text::new(strings.agent_line("").to_string())
                                .style(agent_style.clone()),
                        )
                        .child(Expanded::new(1, MarkdownView::new(text.as_str()))),
                )
                .into(),
                Item::Tool(tool) => Padding::new(
                    EdgeInsets::only(0, i32::from(previous.is_some()), 1, 0),
                    super::transcript_activity::ToolRow(tool.clone()),
                )
                .into(),
            };
            let row = if let Some(timing) = transcript.timing_after(index) {
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
