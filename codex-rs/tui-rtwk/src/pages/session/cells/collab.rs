// cell: Item::Collab(_)
//! The multi-agent rows (oracle multi_agents.rs `collab_event`): a title
//! after a dim bullet, its detail lines under a dim `└` branch. Spawn, input,
//! wait, close and resume of the model's collab tools and a sub-agent's
//! activity share it.
//!
//! Flutter: a stateless widget, a `Column` of one `Text.rich` per row.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{AgentOutcome, AgentRef, CollabEvent, SpawnRequest, SubAgentKind};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use super::text::Part;
use codex_tui_core::threads::Item;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollabCell {
    pub event: Arc<CollabEvent>,
}

impl_widget!(stateless eq CollabCell);

impl StatelessWidget for CollabCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let (title, details) = rows(&self.event, &tokens, strings);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::rich(
                [
                    vec![Span::styled(strings.collab_bullet, faint(&tokens))],
                    title,
                ]
                .concat(),
            ));
        for (index, detail) in details.into_iter().enumerate() {
            let lead = if index == 0 {
                Span::styled(strings.hook_branch, faint(&tokens))
            } else {
                Span::new(strings.hook_body_indent)
            };
            column = column.child(Text::rich([vec![lead], detail].concat()));
        }
        column.into()
    }
}

/// The texts of the cell: the title after its bullet, then a `Text` per detail row (the branch or
/// its blank first). The styles `rows` is given are the theme's and make no text.
pub fn texts(event: &CollabEvent, strings: &Strings) -> Vec<Part> {
    let tokens = Tokens::new(std::rc::Rc::new(rtwk::prelude::ThemeData::DARK));
    let (title, details) = rows(event, &tokens, strings);
    let joined =
        |lead: &str, spans: Spans| -> String {
            std::iter::once(lead.to_owned())
                .chain(spans.iter().map(|span| span.text.to_string()))
                .collect()
        };
    let mut parts = vec![Part::text(joined(strings.collab_bullet, title))];
    for (index, detail) in details.into_iter().enumerate() {
        let lead = if index == 0 {
            strings.hook_branch
        } else {
            strings.hook_body_indent
        };
        parts.push(Part::text(joined(lead, detail)));
    }
    parts
}

/// Oracle `.dim()`.
fn faint(tokens: &Tokens) -> Style {
    tokens.dim().attrs(Attrs::DIM)
}

type Spans = Vec<Span>;

/// The title's spans and the detail rows.
fn rows(event: &CollabEvent, tokens: &Tokens, strings: &Strings) -> (Spans, Vec<Spans>) {
    let bold = || tokens.bold();
    let titled = |prefix: &str, agent: &AgentRef| {
        let mut spans = vec![
            Span::styled(prefix.to_owned(), bold()),
            Span::styled(strings.gap, bold()),
        ];
        spans.extend(label(agent, strings, tokens));
        spans
    };
    let prompt = |prompt: &Option<String>| -> Vec<Spans> {
        prompt
            .iter()
            .map(|prompt| vec![Span::new(prompt.clone())])
            .collect()
    };
    match event {
        CollabEvent::Spawned {
            agent,
            request,
            prompt: text,
        } => {
            let mut title = titled(strings.collab_spawned, agent);
            title.extend(request_spans(request.as_ref(), strings, tokens));
            (title, prompt(text))
        }
        CollabEvent::SpawnFailed { prompt: text } => (
            vec![Span::styled(strings.collab_spawn_failed, bold())],
            prompt(text),
        ),
        CollabEvent::SentInput {
            agent,
            prompt: text,
        } => (titled(strings.collab_sent_input, agent), prompt(text)),
        CollabEvent::Waiting { agents } => match agents.as_slice() {
            [agent] => (titled(strings.collab_waiting_for, agent), Vec::new()),
            [] => (
                vec![Span::styled(strings.collab_waiting_agents, bold())],
                Vec::new(),
            ),
            many => (
                vec![Span::styled(
                    strings
                        .collab_waiting_count
                        .replace("{}", &many.len().to_string()),
                    bold(),
                )],
                many.iter()
                    .map(|agent| label(agent, strings, tokens))
                    .collect(),
            ),
        },
        CollabEvent::FinishedWaiting { agents } => (
            vec![Span::styled(strings.collab_finished_waiting, bold())],
            if agents.is_empty() {
                vec![vec![Span::new(strings.collab_none_completed)]]
            } else {
                agents
                    .iter()
                    .map(|(agent, outcome)| {
                        let mut spans = label(agent, strings, tokens);
                        spans.push(Span::styled(strings.collab_state_separator, faint(tokens)));
                        spans.extend(outcome_spans(outcome, tokens, strings));
                        spans
                    })
                    .collect()
            },
        ),
        CollabEvent::Closed { agent } => (titled(strings.collab_closed, agent), Vec::new()),
        CollabEvent::Resuming { agent } => (titled(strings.collab_resuming, agent), Vec::new()),
        CollabEvent::Resumed { agent, outcome } => (
            titled(strings.collab_resumed, agent),
            vec![match outcome {
                Some(outcome) => outcome_spans(outcome, tokens, strings),
                None => error_spans(Some(strings.collab_resume_failed), tokens, strings),
            }],
        ),
        CollabEvent::Activity { kind, path } => (
            vec![
                Span::styled(
                    match kind {
                        SubAgentKind::Started => strings.collab_started,
                        SubAgentKind::Interacted => strings.collab_interacted,
                        SubAgentKind::Interrupted => strings.collab_interrupted,
                        SubAgentKind::Completed => strings.collab_completed,
                    },
                    bold(),
                ),
                Span::styled(strings.collab_tick, tokens.accent()),
                Span::styled(path.clone(), tokens.accent()),
                Span::styled(strings.collab_tick, tokens.accent()),
            ],
            Vec::new(),
        ),
    }
}

/// Oracle `agent_label_spans`: the nickname (bold) or the thread id, then
/// the role in brackets.
fn label(agent: &AgentRef, strings: &Strings, tokens: &Tokens) -> Spans {
    let nickname = agent
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|nickname| !nickname.is_empty());
    let role = agent
        .role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty());
    let mut spans = vec![match nickname {
        Some(nickname) => Span::styled(nickname.to_owned(), tokens.accent().attrs(Attrs::BOLD)),
        None => Span::styled(agent.id.clone(), tokens.accent()),
    }];
    if let Some(role) = role {
        spans.push(Span::styled(strings.gap, faint(tokens)));
        spans.push(Span::new(strings.collab_role_open));
        spans.push(Span::new(role.to_owned()));
        spans.push(Span::new(strings.collab_role_close));
    }
    spans
}

/// Oracle `spawn_request_spans`.
fn request_spans(request: Option<&SpawnRequest>, strings: &Strings, tokens: &Tokens) -> Spans {
    let Some(request) = request else {
        return Vec::new();
    };
    let details = if request.model.is_empty() {
        strings.collab_request_effort.replace("{}", &request.effort)
    } else {
        strings
            .collab_request
            .replace("{model}", &request.model)
            .replace("{effort}", &request.effort)
    };
    vec![
        Span::styled(strings.gap, faint(tokens)),
        // The oracle's magenta, the theme's accent (status_line.rs).
        Span::styled(details, tokens.accent()),
    ]
}

/// Oracle `status_summary_spans`.
fn outcome_spans(outcome: &AgentOutcome, tokens: &Tokens, strings: &Strings) -> Spans {
    match outcome {
        AgentOutcome::PendingInit => {
            vec![Span::styled(strings.collab_pending_init, tokens.accent())]
        }
        AgentOutcome::Running => vec![Span::styled(
            strings.collab_running,
            tokens.accent().attrs(Attrs::BOLD),
        )],
        AgentOutcome::Interrupted => {
            vec![Span::styled(
                strings.collab_interrupted_state,
                tokens.warning(),
            )]
        }
        AgentOutcome::Completed(message) => {
            let mut spans = vec![Span::styled(
                strings.collab_completed_state,
                tokens.success(),
            )];
            if let Some(message) = message {
                spans.push(Span::styled(
                    strings.collab_message_separator,
                    faint(tokens),
                ));
                spans.push(Span::new(message.clone()));
            }
            spans
        }
        AgentOutcome::Errored(message) => error_spans(
            Some(message.as_deref().unwrap_or(strings.collab_errored)),
            tokens,
            strings,
        ),
        AgentOutcome::Shutdown => vec![Span::new(strings.collab_shutdown)],
        AgentOutcome::NotFound => vec![Span::styled(strings.collab_not_found, tokens.error())],
    }
}

/// Oracle `error_summary_spans`: `Error`, then ` - ` and the message when
/// it is not empty.
fn error_spans(message: Option<&str>, tokens: &Tokens, strings: &Strings) -> Spans {
    let mut spans = vec![Span::styled(strings.collab_error, tokens.error())];
    if let Some(message) = message.filter(|message| !message.is_empty()) {
        spans.push(Span::styled(
            strings.collab_message_separator,
            faint(tokens),
        ));
        spans.push(Span::new(message.to_owned()));
    }
    spans
}

/// A multi-agent call or a sub-agent's activity.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Collab(event) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        CollabCell {
            event: event.clone(),
        }
        .into(),
    )
}
