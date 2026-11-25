//! The summary of the questions that wait while their editor is closed (`question_summary` of
//! `bottom_pane/questions.rs`): `  ? N questions`, and under it the key that opens the editor.
//!
//! Flutter: a `Column` of two `Text.rich` rows.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE;

use super::strings;
use crate::app::theme::Tokens;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::pages::session) struct QuestionSummary {
    pub count: usize,
    /// The seconds left of the countdown, in its last twenty (`· 15s`).
    pub countdown: Option<u64>,
}

impl_widget!(stateless eq QuestionSummary);

impl StatelessWidget for QuestionSummary {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let strings = strings::of(cx);
        let tokens = Tokens::of(cx);
        let keymap = crate::keymap::keymap(cx);
        let mut title = vec![
            Span::styled(strings.question_mark, super::dim()),
            Span::styled(
                strings.questions(self.count),
                tokens.accent().attrs(Attrs::BOLD),
            ),
        ];
        if let Some(seconds) = self.countdown {
            title.push(Span::styled(strings.countdown(seconds), super::dim()));
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::rich_line(title));
        if let Some(key) = crate::keymap::label(&keymap, CHAT_EDIT_QUEUED_MESSAGE) {
            column = column.child(Text::rich_line(vec![
                Span::styled(strings.item_hang, rtwk::style::Style::new()),
                Span::styled(key, super::bold()),
                Span::styled(strings.answer_rest, super::dim()),
            ]));
        }
        column.into()
    }
}
