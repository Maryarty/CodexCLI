//! The bare `/goal` summary (oracle `chatwidget/goal_menu.rs` `goal_summary_lines`, added as plain
//! history lines): the goal as rows, no bullet, the labels dim and the heading bold.
//!
//! Flutter: a `Column` of `Text.rich`; the words are `strings.rs`'s.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::Goal;

use super::strings::{Row, message_and_hint, summary};
use crate::app::theme::Tokens;
use codex_tui_core::features::goal::Note;

/// What the bare `/goal` left in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryCell {
    pub goal: Goal,
}

impl_widget!(stateless eq SummaryCell);

impl StatelessWidget for SummaryCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for row in summary(&self.goal) {
            let child: AnyWidget = match row {
                Row::Heading(text) => Text::rich(vec![Span::styled(text, tokens.bold())]).into(),
                Row::Field(label, value) => Text::rich(vec![
                    Span::styled(label, tokens.dim()),
                    Span::styled(value, Style::new()),
                ])
                .into(),
                Row::Blank => SizedBox::new(None, Some(1), None).into(),
                Row::Hint(text) => Text::rich(vec![Span::styled(text, tokens.dim())]).into(),
            };
            column = column.child(child);
        }
        column.into()
    }
}

/// A note with a hint after its message on the same line (oracle `new_info_event`): the bullet
/// dim, the message plain, the hint dim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HintCell {
    pub note: Note,
}

impl_widget!(stateless eq HintCell);

impl StatelessWidget for HintCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let bullet = crate::pages::session::cells::strings::of(cx).bullet;
        let (message, hint) = message_and_hint(&self.note);
        let mut spans = vec![
            Span::styled(bullet, tokens.dim()),
            Span::styled(message, Style::new()),
        ];
        if let Some(hint) = hint {
            spans.push(Span::styled(super::strings::HINT_SEPARATOR, Style::new()));
            spans.push(Span::styled(hint, tokens.dim()));
        }
        Text::rich(spans).into()
    }
}
