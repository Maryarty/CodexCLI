//! Session notices (oracle thread_transcript/other_items.rs `new_info_event`
//! and `new_review_status_line`).

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::screens::chat::{Nudge, NudgeOutcome};
use codex_tui_core::threads::Notice;

use crate::app::theme::Tokens;

use super::safety;
use super::strings::Strings;
use super::text::Part;
use super::update_available;
use super::{interaction, request_input, session_info};

/// The mark and its blank (oracle `"⚠ "` and `"  "`).
const MARK_COLUMNS: i32 = 2;

/// The notice's texts, the mark of a row that has one included.
pub fn texts(notice: &Notice, strings: &Strings) -> Vec<Part> {
    let row = |text: String| vec![Part::text(text)];
    match notice {
        Notice::ContextCompacted => row([strings.bullet, strings.context_compacted].concat()),
        Notice::Interrupted => row(strings.interrupted.to_owned()),
        Notice::InterruptedForSteer => {
            row([strings.bullet, strings.interrupted_for_steer].concat())
        }
        Notice::NoPreviousMessage => row([strings.bullet, strings.no_previous_message].concat()),
        Notice::GoalBudgetReached => row(super::error::line(strings.goal_budget_reached, strings)),
        Notice::EnteredReview(review) => {
            row([strings.review_started.0, review, strings.review_started.1].concat())
        }
        Notice::ExitedReview(review) => {
            row([strings.review_finished.0, review, strings.review_finished.1].concat())
        }
        Notice::AddCreditsNudge { nudge, outcome } => {
            row([strings.bullet, nudge_words(*nudge, *outcome, strings)].concat())
        }
        Notice::SafetyAccessBlock | Notice::CyberPolicy(_) => {
            let words = safety::words(notice, strings).unwrap_or_else(|| unreachable!());
            safety::texts(&words, strings)
        }
        Notice::UpdateAvailable {
            current,
            latest,
            command,
        } => update_available::texts(current, latest, command.as_deref(), strings),
        Notice::ServerVersion {
            message,
            offer_update,
        } => {
            let mut parts = vec![Part::gutter(strings.warning_mark), Part::text(message)];
            if *offer_update {
                parts.push(Part::text(strings.daemon_hint));
                parts.push(Part::text(strings.daemon_warning));
            }
            parts
        }
        Notice::TerminalInteraction { command, stdin } => {
            interaction::texts(command.as_deref(), stdin, strings)
        }
        Notice::ForkedThread { from, title } => row([
            strings.bullet,
            strings.forked_from,
            &match title {
                Some(name) => format!("{name} ({from})"),
                None => from.clone(),
            },
        ]
        .concat()),
        Notice::RecapLoading => row([
            strings.bullet,
            strings.recap_loading.0,
            strings.recap_loading.1,
        ]
        .concat()),
        Notice::Questions {
            questions,
            interrupted,
        } => request_input::texts(questions, *interrupted, strings),
        Notice::SessionInfo(info) => session_info::texts(info, strings),
    }
}

fn nudge_words(nudge: Nudge, outcome: NudgeOutcome, strings: &Strings) -> &'static str {
    let words = match nudge {
        Nudge::Credits => &strings.nudge_credits,
        Nudge::UsageLimit => &strings.nudge_usage_limit,
    };
    match outcome {
        NudgeOutcome::Sent => words[0],
        NudgeOutcome::Cooldown => words[1],
        NudgeOutcome::Failed => words[2],
    }
}

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
            // `new_interruption_notice`: one line in the plain secondary style
            Notice::Interrupted => Text::new(strings.interrupted).into(),
            // `new_info_event`
            Notice::InterruptedForSteer => Text::rich(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.interrupted_for_steer, Style::new()),
            ])
            .into(),
            // `add_info_message`: the bullet, then the line
            Notice::NoPreviousMessage => Text::rich(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.no_previous_message, Style::new()),
            ])
            .into(),
            // `new_error_event`
            Notice::GoalBudgetReached => super::error::ErrorCell {
                message: strings.goal_budget_reached.to_owned(),
            }
            .into(),
            Notice::EnteredReview(text) => review(strings.review_started, text),
            Notice::ExitedReview(text) => review(strings.review_finished, text),
            // `new_info_event`: the bullet, then the message
            Notice::AddCreditsNudge { nudge, outcome } => {
                let message = nudge_words(*nudge, *outcome, strings);
                Text::rich(vec![
                    Span::styled(strings.bullet, tokens.dim()),
                    Span::styled(message, Style::new()),
                ])
                .into()
            }
            Notice::SafetyAccessBlock | Notice::CyberPolicy(_) => {
                match safety::words(&self.notice, strings) {
                    Some(words) => safety::cell(&words, &tokens, strings),
                    None => unreachable!("a refusal notice has words"),
                }
            }
            Notice::UpdateAvailable {
                current,
                latest,
                command,
            } => update_available::cell(current, latest, command.as_deref(), &tokens, strings),
            // `new_server_version_warning`: a warning whose body has up to
            // three rows, the attention tone except the `/daemon` hint
            Notice::ServerVersion {
                message,
                offer_update,
            } => {
                let attention = tokens.warning().attrs(Attrs::BOLD);
                let mut body = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(Text::new(message.clone()).style(attention.clone()));
                if *offer_update {
                    body = body
                        .child(Text::new(strings.daemon_hint).style(tokens.accent()))
                        .child(Text::new(strings.daemon_warning).style(attention.clone()));
                }
                Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(SizedBox::new(
                        Some(MARK_COLUMNS),
                        None,
                        Some(Text::new(strings.warning_mark).style(attention).into()),
                    ))
                    .child(Expanded::new(1, body))
                    .into()
            }
            Notice::TerminalInteraction { command, stdin } => {
                interaction::cell(command.as_deref(), stdin, &tokens, strings)
            }
            // `emit_forked_thread_event`: the parent's name and id in the accent colour
            Notice::ForkedThread { from, title } => {
                let mut row = vec![
                    Span::styled(strings.bullet, tokens.dim()),
                    Span::styled(strings.forked_from, Style::new()),
                ];
                match title {
                    Some(name) => row.extend([
                        Span::styled(name.clone(), tokens.accent()),
                        Span::styled(" (", Style::new()),
                        Span::styled(from.clone(), tokens.accent()),
                        Span::styled(")", Style::new()),
                    ]),
                    None => row.push(Span::styled(from.clone(), tokens.accent())),
                }
                Text::rich(row).into()
            }
            // static bullet: the oracle's reduced-motion form
            Notice::RecapLoading => Text::rich(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.recap_loading.0, tokens.bold()),
                Span::styled(strings.recap_loading.1, tokens.dim()),
            ])
            .into(),
            Notice::Questions {
                questions,
                interrupted,
            } => request_input::cell(questions, *interrupted, &tokens, strings),
            Notice::SessionInfo(info) => {
                let markdown = tokens.transcript_markdown(cx);
                session_info::cell(info, markdown, &tokens, strings)
            }
        }
    }
}
