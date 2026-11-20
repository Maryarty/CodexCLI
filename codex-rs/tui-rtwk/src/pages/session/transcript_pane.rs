//! The retained transcript viewport. Inline Markdown uses this list's scroll
//! position and preserves the parser for a row while its item key is mounted.

use rtwk::gestures::SelectionArea;
use rtwk::layout::{CrossAxisAlignment, Expanded, Row};
use rtwk::provider::{read, watch};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, NotificationListener, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::MarkdownView;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ThreadModel};

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;

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
        let thread_id = thread.id().clone();
        let can_load = thread.can_load_history();
        let list = ListView::builder(thread.transcript().len(), move |index| {
            let Some((key, item)) = thread.transcript().get(index) else {
                return AnyWidget::default();
            };
            let row = match item {
                Item::User { text } => AnyWidget::from(
                    Text::new(strings.user_line(text).to_string()).style(tokens.bold()),
                ),
                Item::Agent { text } => Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .spacing(1)
                    .child(Text::new(strings.agent_line("").to_string()))
                    .child(Expanded::new(1, MarkdownView::new(text.as_str())))
                    .into(),
            };
            row.with_key(Key::value(key.clone()))
        })
        .controller(widget.0.clone())
        .stick_to_bottom(true);
        let send = cx.local_sender();
        SelectionArea::new(
            NotificationListener::<ScrollNotification>::new(list).on_notification(move |scroll| {
                if can_load && scroll.depth() == 0 && scroll.metrics.pixels <= 3 {
                    send(thread_id.clone());
                }
                false
            }),
        )
        .into()
    }
}

impl_widget!(stateful eq TranscriptPane);
