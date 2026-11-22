//! What the startup prompts leave in the history (the oracle's
//! `insert_history_lines`): the line a failed save of the cwd prompt's choice
//! leaves (`cwd_prompt.rs`, `persist_remembered_cwd_selection`).
//!
//! Flutter: a stateless widget, a `Column` of one `Text` per line, red as the
//! oracle's `.red()`. The lines are the model's (`startup.history`); the chat
//! shows them above its own cells.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::provider::select;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::startup::HistoryLine;

use crate::app::theme::Tokens;

use super::strings;

/// The lines the prompts left, oldest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoryLines;

impl_widget!(stateless eq HistoryLines);

impl StatelessWidget for HistoryLines {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let lines = select::<ScreenStates, Vec<HistoryLine>, _>(cx, |states| {
            states.startup.history.clone()
        });
        let strings = strings::of(cx);
        let error = Tokens::of(cx).error();
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for line in lines {
            let text = match line {
                HistoryLine::CwdPreferenceNotSaved => strings.cwd_not_saved,
            };
            column = column.child(Text::new(text).style(error.clone()));
        }
        column.into()
    }
}
