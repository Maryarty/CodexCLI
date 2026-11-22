//! What an answer leaves in the thread's history (the oracle's `new_approval_decision_cell`,
//! `history_cell/approvals.rs`, and the plain permissions line of `handle_permissions_decision`):
//! `✔ You approved codex to run <command> this time`, the words under a two-column hanging
//! indent, the verbs bold and the command dim; a permissions answer is one plain line.
//!
//! Flutter: a `Row` of the mark and an `Expanded` `Text.rich`, in the transcript's `ListView`.
//! The model says what happened (`Decided`, core); the words are `strings.rs`'s.

use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};
use rtwk_core::unicode::grapheme_boundaries;

use codex_tui_core::screens::approvals::decided::{Actor, Decided, Grant, Outcome, Subject};

use super::strings::{self, Strings};
use crate::app::theme::Tokens;

/// The most graphemes of a command or a prefix the cell shows (`truncate_exec_snippet`).
const SNIPPET_GRAPHEMES: usize = 80;
/// The cells of the mark and its blank: what the rows under it keep free.
const MARK_CELLS: i32 = 2;

/// An answer in the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionCell {
    pub decided: Decided,
}

impl_widget!(stateless eq DecisionCell);

impl StatelessWidget for DecisionCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let (actor, subject, outcome) = match &self.decided {
            Decided::Permissions(grant) => return Text::new(grant_line(s, *grant)).into(),
            Decided::Request {
                actor,
                subject,
                outcome,
            } => (*actor, subject, outcome),
        };
        let say = Say {
            s,
            actor,
            subject,
            bold: tokens.bold(),
            dim: tokens.dim(),
        };
        let mark = if approves(outcome) {
            Span::styled(s.approved_mark, tokens.success())
        } else {
            Span::styled(s.refused_mark, tokens.error())
        };
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(
                Some(MARK_CELLS),
                None,
                Some(Text::rich(vec![mark]).into()),
            ))
            .child(Expanded::new(1, Text::rich(say.summary(outcome))))
            .into()
    }
}

fn grant_line(s: &Strings, grant: Grant) -> &'static str {
    match grant {
        Grant::Nothing => s.granted_nothing,
        Grant::Turn => s.granted_turn,
        Grant::Strict => s.granted_strict,
        Grant::Session => s.granted_session,
    }
}

/// `✔` for what the user or the reviewer let through, `✗` for the rest.
fn approves(outcome: &Outcome) -> bool {
    matches!(
        outcome,
        Outcome::Approved
            | Outcome::ApprovedForSession
            | Outcome::ApprovedWithPrefix(_)
            | Outcome::NetworkRule { allow: true, .. }
    )
}

/// The spans of one cell: who, what, about what.
struct Say<'a> {
    s: &'a Strings,
    actor: Actor,
    subject: &'a Subject,
    bold: rtwk::style::Style,
    dim: rtwk::style::Style,
}

impl Say<'_> {
    fn who(&self) -> Span {
        Span::new(match self.actor {
            Actor::User => self.s.you,
            Actor::Guardian => self.s.auto_reviewer,
        })
    }

    fn bold(&self, text: &'static str) -> Span {
        Span::styled(text, self.bold.clone())
    }

    fn dim(&self, text: String) -> Span {
        Span::styled(text, self.dim.clone())
    }

    /// The command the cell names, or none when it is empty.
    fn command(&self) -> Option<String> {
        match self.subject {
            Subject::Command(command) => {
                Some(snippet(self.s, command)).filter(|text| !text.is_empty())
            }
            Subject::Network { .. } => None,
        }
    }

    fn summary(&self, outcome: &Outcome) -> Vec<Span> {
        let s = self.s;
        // What the request was about, spelled the way `outcome` needs it: a command (or this
        // request when there is none) or network access to a target.
        let about = |run: &'static str, request: &'static str, network: &'static str| match (
            self.subject,
            self.command(),
        ) {
            (Subject::Network { target }, _) => {
                vec![Span::new(network), self.dim(target.clone())]
            }
            (Subject::Command(_), Some(snippet)) => {
                vec![Span::new(run), self.dim(snippet)]
            }
            (Subject::Command(_), None) => vec![Span::new(request)],
        };
        let mut spans = Vec::new();
        match outcome {
            Outcome::Approved | Outcome::ApprovedForSession => {
                spans.push(self.who());
                spans.push(self.bold(s.verb_approved));
                spans.extend(about(
                    s.cell_codex_to_run,
                    s.cell_this_request,
                    s.cell_network_to,
                ));
                spans.push(self.bold(if matches!(outcome, Outcome::Approved) {
                    s.cell_this_time
                } else {
                    s.cell_every_time
                }));
            }
            Outcome::ApprovedWithPrefix(prefix) => {
                spans.push(self.who());
                spans.push(self.bold(s.verb_approved));
                spans.push(Span::new(s.cell_always_run));
                spans.push(self.dim(snippet(s, prefix)));
            }
            Outcome::NetworkRule { host, allow } => {
                let target = match self.subject {
                    Subject::Network { target } => target.clone(),
                    Subject::Command(_) => host.clone(),
                };
                spans.push(self.who());
                if *allow {
                    spans.push(self.bold(s.verb_persisted));
                    spans.push(Span::new(s.cell_persisted_network_to));
                    spans.push(self.dim(target));
                } else {
                    spans.push(self.bold(s.verb_denied));
                    spans.push(Span::new(s.cell_network_to));
                    spans.push(self.dim(target));
                    spans.push(Span::new(s.cell_saved_rule));
                }
            }
            Outcome::Denied => match (self.actor, self.subject) {
                // The reviewer's own words are for commands; network access names the actor.
                (Actor::Guardian, Subject::Command(_)) => {
                    spans.push(Span::new(s.cell_request));
                    spans.push(self.bold(s.verb_denied));
                    if let Some(snippet) = self.command() {
                        spans.push(Span::new(s.cell_denied_run));
                        spans.push(self.dim(snippet));
                    }
                }
                _ => {
                    spans.push(self.who());
                    spans.push(self.bold(s.verb_did_not_approve));
                    spans.extend(about(
                        s.cell_codex_to_run,
                        s.cell_this_request,
                        s.cell_network_to,
                    ));
                }
            },
            Outcome::TimedOut => {
                spans.push(Span::new(s.cell_review));
                spans.push(self.bold(s.verb_timed_out));
                match (self.subject, self.command()) {
                    (Subject::Network { target }, _) => {
                        spans.push(Span::new(s.cell_timed_out_access));
                        spans.push(self.dim(target.clone()));
                    }
                    (Subject::Command(_), Some(snippet)) => {
                        spans.push(Span::new(s.cell_timed_out_run));
                        spans.push(self.dim(snippet));
                    }
                    (Subject::Command(_), None) => {
                        spans.push(Span::new(s.cell_timed_out_request));
                    }
                }
            }
            Outcome::Aborted => {
                spans.push(self.who());
                spans.push(self.bold(s.verb_canceled));
                spans.extend(about(
                    s.cell_canceled_run,
                    s.cell_this_request,
                    s.cell_canceled_network,
                ));
            }
        }
        spans
    }
}

/// `exec_snippet`: the first line of a command with the dots after it when there are more, and
/// at most eighty graphemes of it (seventy-seven and the dots when it is longer).
fn snippet(s: &Strings, command: &str) -> String {
    let line = match command.split_once('\n') {
        Some((first, _)) => format!("{first}{}", s.cell_more_lines),
        None => command.to_string(),
    };
    if grapheme_boundaries(&line).nth(SNIPPET_GRAPHEMES).is_none() {
        return line;
    }
    let keep = SNIPPET_GRAPHEMES - grapheme_boundaries(s.cell_cut).count();
    match grapheme_boundaries(&line).nth(keep) {
        Some((end, _)) => format!("{}{}", &line[..end], s.cell_cut),
        None => line,
    }
}
