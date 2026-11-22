//! Session notices (oracle thread_transcript/other_items.rs `new_info_event`
//! and `new_review_status_line`).

use rtwk::style::{Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::screens::chat::{Nudge, NudgeOutcome};
use codex_tui_core::threads::Notice;

use crate::app::theme::Tokens;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeCell {
    pub notice: Notice,
}

impl_widget!(stateless eq NoticeCell);

impl StatelessWidget for NoticeCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let review = |(open, close): (&'static str, &'static str), review: &str| {
            Text::rich(vec![
                Span::new(open),
                Span::new(review.to_owned()),
                Span::new(close),
            ])
            .style(tokens.accent())
            .into()
        };
        match &self.notice {
            Notice::ContextCompacted => Text::rich(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.context_compacted, Style::new()),
            ])
            .into(),
            Notice::EnteredReview(text) => review(strings.review_started, text),
            Notice::ExitedReview(text) => review(strings.review_finished, text),
            // `new_info_event`: the bullet, then the message
            Notice::AddCreditsNudge { nudge, outcome } => {
                let words = match nudge {
                    Nudge::Credits => &strings.nudge_credits,
                    Nudge::UsageLimit => &strings.nudge_usage_limit,
                };
                let message = match outcome {
                    NudgeOutcome::Sent => words[0],
                    NudgeOutcome::Cooldown => words[1],
                    NudgeOutcome::Failed => words[2],
                };
                Text::rich(vec![
                    Span::styled(strings.bullet, tokens.dim()),
                    Span::styled(message, Style::new()),
                ])
                .into()
            }
        }
    }
}
