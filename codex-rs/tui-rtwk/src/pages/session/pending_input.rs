//! What the user sent while a turn runs and has not gone out yet, above the
//! composer (the oracle's `bottom_pane/pending_input_preview.rs`): the first
//! queued messages, then how many more wait. The thread model's queue is the
//! only source; the widget keeps nothing.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::provider::select;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::ThreadModel;

use crate::app::theme;
use crate::l10n::strings as l10n;

/// The queued messages shown by text; the rest is `more queued`.
const QUEUE_PREVIEW: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingInput;

impl_widget!(stateless eq PendingInput);

impl StatelessWidget for PendingInput {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let (shown, total) = select::<ThreadModel, (Vec<String>, usize), _>(cx, |thread| {
            (
                thread
                    .queue()
                    .iter()
                    .take(QUEUE_PREVIEW)
                    .map(|message| message.text.clone())
                    .collect(),
                thread.queue().len(),
            )
        });
        if shown.is_empty() {
            return SizedBox::shrink().into();
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for text in &shown {
            column = column.child(
                Text::new(strings.queued(text).to_string())
                    .max_lines(Some(2))
                    .overflow(TextOverflow::Ellipsis)
                    .style(tokens.dim()),
            );
        }
        let remaining = total.saturating_sub(QUEUE_PREVIEW);
        if remaining > 0 {
            column = column.child(Text::line(strings.more_queued(remaining)).style(tokens.dim()));
        }
        column.into()
    }
}
