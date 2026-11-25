//! Multi-agent events: what the model's collab tools (spawn, send input,
//! wait, resume, close) and a sub-agent's activity leave in the transcript.
//! Oracle `multi_agents.rs` (`tool_call_history_cell`,
//! `sub_agent_activity_history_cell`): the protocol is read once here (R4),
//! the view draws these values.
//!
//! The oracle adds a cell per notification, so a wait (and a resume) is two
//! rows, one when it starts and one when it ends: the start gets a key of
//! its own and stays when the end arrives.

use std::collections::HashMap;

use codex_app_server_protocol::{
    CollabAgentState, CollabAgentStatus, CollabAgentTool, CollabAgentToolCallStatus,
    SubAgentActivityKind, ThreadItemPayload,
};
use codex_protocol::ThreadId as ProtocolThreadId;

use crate::ids::ItemId;

/// An agent as a row names it: its nickname and role when the app knows
/// them (oracle `AgentMetadata`), else its thread id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRef {
    pub id: String,
    pub nickname: Option<String>,
    pub role: Option<String>,
}

/// The model and effort a spawn asked for, as the row shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnRequest {
    pub model: String,
    pub effort: String,
}

/// An agent's last known state (oracle `status_summary_spans`); a message is
/// whitespace-joined and cut to the oracle's preview length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentOutcome {
    // `Completed` has no message when it is empty; `Errored` has none when the
    // server sent none (the view words it).
    PendingInit,
    Running,
    Interrupted,
    Completed(Option<String>),
    Errored(Option<String>),
    Shutdown,
    NotFound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubAgentKind {
    Started,
    Interacted,
    Interrupted,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollabEvent {
    Spawned {
        agent: AgentRef,
        request: Option<SpawnRequest>,
        prompt: Option<String>,
    },
    SpawnFailed {
        prompt: Option<String>,
    },
    SentInput {
        agent: AgentRef,
        prompt: Option<String>,
    },
    /// Nobody, one agent, or several (listed under the title).
    Waiting {
        agents: Vec<AgentRef>,
    },
    /// Empty: no agent had completed.
    FinishedWaiting {
        agents: Vec<(AgentRef, AgentOutcome)>,
    },
    Closed {
        agent: AgentRef,
    },
    Resuming {
        agent: AgentRef,
    },
    /// `None`: the call failed and no state came with it.
    Resumed {
        agent: AgentRef,
        outcome: Option<AgentOutcome>,
    },
    Activity {
        kind: SubAgentKind,
        path: String,
    },
}

/// An agent's nickname and role.
pub type AgentName = (Option<String>, Option<String>);

impl CollabEvent {
    /// The event with each agent's nickname and role filled in by `lookup`
    /// (thread id -> nickname, role): the oracle's `AgentMetadata`, which a
    /// row reads when it is made.
    pub fn named(&self, lookup: &dyn Fn(&str) -> AgentName) -> CollabEvent {
        let name = |agent: &AgentRef| {
            let (nickname, role) = lookup(&agent.id);
            AgentRef {
                id: agent.id.clone(),
                nickname,
                role,
            }
        };
        let mut event = self.clone();
        match &mut event {
            CollabEvent::Spawned { agent, .. }
            | CollabEvent::SentInput { agent, .. }
            | CollabEvent::Closed { agent }
            | CollabEvent::Resuming { agent }
            | CollabEvent::Resumed { agent, .. } => *agent = name(agent),
            CollabEvent::Waiting { agents } => {
                agents.iter_mut().for_each(|agent| *agent = name(agent));
            }
            CollabEvent::FinishedWaiting { agents } => {
                agents
                    .iter_mut()
                    .for_each(|(agent, _)| *agent = name(agent));
            }
            CollabEvent::SpawnFailed { .. } | CollabEvent::Activity { .. } => {}
        }
        event
    }
}

/// The prompt preview length (oracle `COLLAB_PROMPT_PREVIEW_GRAPHEMES`).
const PROMPT_PREVIEW: usize = 160;
/// The error preview length (oracle `COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES`).
const ERROR_PREVIEW: usize = 160;
/// The response preview length (oracle `COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES`).
const RESPONSE_PREVIEW: usize = 240;

/// The entry of a collab or sub-agent item, keyed as the oracle's cells
/// follow one another; none for a call the oracle shows nothing for.
pub(super) fn entry(payload: &ThreadItemPayload) -> Option<(ItemId, super::Item)> {
    let (key, event) = match payload {
        ThreadItemPayload::CollabAgentToolCall {
            id,
            tool,
            status,
            receiver_thread_ids,
            prompt,
            model,
            reasoning_effort,
            agents_states,
            ..
        } => {
            let running = matches!(status, CollabAgentToolCallStatus::InProgress);
            let first = receiver_thread_ids.first().filter(|id| parsed(id));
            let prompt = prompt
                .as_deref()
                .map(str::trim)
                .filter(|prompt| !prompt.is_empty())
                .map(|prompt| preview(prompt, PROMPT_PREVIEW));
            match tool {
                // The v2 tools are shown through the sub-agent activity.
                CollabAgentTool::SendMessage
                | CollabAgentTool::FollowupTask
                | CollabAgentTool::InterruptAgent
                | CollabAgentTool::ListAgents => return None,
                CollabAgentTool::SpawnAgent if running => return None,
                CollabAgentTool::SpawnAgent => (
                    id.clone(),
                    match first {
                        Some(agent) => CollabEvent::Spawned {
                            agent: agent_ref(agent),
                            request: spawn_request(model.as_deref(), reasoning_effort.as_ref()),
                            prompt,
                        },
                        None => CollabEvent::SpawnFailed { prompt },
                    },
                ),
                CollabAgentTool::SendInput if running => return None,
                CollabAgentTool::SendInput => (
                    id.clone(),
                    CollabEvent::SentInput {
                        agent: agent_ref(first?),
                        prompt,
                    },
                ),
                CollabAgentTool::ResumeAgent if running => (
                    begin_key(id),
                    CollabEvent::Resuming {
                        agent: agent_ref(first?),
                    },
                ),
                CollabAgentTool::ResumeAgent => (
                    id.clone(),
                    CollabEvent::Resumed {
                        agent: agent_ref(first?),
                        outcome: first_state(receiver_thread_ids, agents_states).map(outcome),
                    },
                ),
                CollabAgentTool::Wait if running => (
                    begin_key(id),
                    CollabEvent::Waiting {
                        agents: receiver_thread_ids
                            .iter()
                            .filter(|id| parsed(id))
                            .map(|id| agent_ref(id))
                            .collect(),
                    },
                ),
                CollabAgentTool::Wait => (
                    id.clone(),
                    CollabEvent::FinishedWaiting {
                        agents: waited(receiver_thread_ids, agents_states),
                    },
                ),
                CollabAgentTool::CloseAgent if running => return None,
                CollabAgentTool::CloseAgent => (
                    id.clone(),
                    CollabEvent::Closed {
                        agent: agent_ref(first?),
                    },
                ),
            }
        }
        ThreadItemPayload::SubAgentActivity {
            id,
            kind,
            agent_path,
            ..
        } => (
            id.clone(),
            CollabEvent::Activity {
                kind: match kind {
                    SubAgentActivityKind::Started => SubAgentKind::Started,
                    SubAgentActivityKind::Interacted => SubAgentKind::Interacted,
                    SubAgentActivityKind::Interrupted => SubAgentKind::Interrupted,
                    SubAgentActivityKind::Completed => SubAgentKind::Completed,
                },
                path: agent_path.clone(),
            },
        ),
        _ => return None,
    };
    Some((
        ItemId::new(&key),
        super::Item::Collab(std::sync::Arc::new(event)),
    ))
}

/// The model and effort a spawn in progress asked for, with its call id: the oracle keeps them
/// (`pending_collab_spawn_requests`) for the row its completion makes, which the server's
/// completed item may not carry.
pub fn requested_spawn(payload: &ThreadItemPayload) -> Option<(String, SpawnRequest)> {
    let ThreadItemPayload::CollabAgentToolCall {
        id,
        tool: CollabAgentTool::SpawnAgent,
        status: CollabAgentToolCallStatus::InProgress,
        model,
        reasoning_effort,
        ..
    } = payload
    else {
        return None;
    };
    Some((
        id.clone(),
        spawn_request(model.as_deref(), reasoning_effort.as_ref())?,
    ))
}

/// A call that begins and ends is two rows: the beginning's key is its own.
fn begin_key(id: &str) -> String {
    let mut key = id.to_owned();
    key.push_str("#begin");
    key
}

fn parsed(thread_id: &str) -> bool {
    ProtocolThreadId::from_string(thread_id).is_ok()
}

fn agent_ref(id: &str) -> AgentRef {
    AgentRef {
        id: id.to_owned(),
        nickname: None,
        role: None,
    }
}

/// Oracle `spawn_request_spans`: shown unless model and effort are both the
/// defaults' emptiness (an empty model with the default effort).
fn spawn_request(
    model: Option<&str>,
    effort: Option<&codex_protocol::openai_models::ReasoningEffort>,
) -> Option<SpawnRequest> {
    let (model, effort) = (model?.trim(), effort?);
    if model.is_empty() && *effort == codex_protocol::openai_models::ReasoningEffort::default() {
        return None;
    }
    Some(SpawnRequest {
        model: model.to_owned(),
        effort: effort.to_string(),
    })
}

fn first_state<'a>(
    receivers: &[String],
    states: &'a HashMap<String, CollabAgentState>,
) -> Option<&'a CollabAgentState> {
    receivers
        .iter()
        .find_map(|id| states.get(id))
        .or_else(|| states.iter().min_by(|l, r| l.0.cmp(r.0)).map(|(_, s)| s))
}

/// Oracle `wait_complete_lines`: the receivers that have a state, in their
/// order, then the other agents with one by thread id.
fn waited(
    receivers: &[String],
    states: &HashMap<String, CollabAgentState>,
) -> Vec<(AgentRef, AgentOutcome)> {
    let mut seen = Vec::new();
    let mut entries: Vec<(AgentRef, AgentOutcome)> = receivers
        .iter()
        .filter(|id| parsed(id))
        .filter_map(|id| {
            let state = states.get(id)?;
            seen.push(id.as_str());
            Some((agent_ref(id), outcome(state)))
        })
        .collect();
    let mut extras: Vec<(&String, &CollabAgentState)> = states
        .iter()
        .filter(|(id, _)| parsed(id) && !seen.contains(&id.as_str()))
        .collect();
    extras.sort_by_key(|(id, _)| {
        ProtocolThreadId::from_string(id)
            .map(|id| id.to_string())
            .ok()
    });
    entries.extend(
        extras
            .into_iter()
            .map(|(id, state)| (agent_ref(id), outcome(state))),
    );
    entries
}

fn outcome(state: &CollabAgentState) -> AgentOutcome {
    let message = |limit: usize| {
        state
            .message
            .as_deref()
            .map(|message| message.split_whitespace().collect::<Vec<_>>().join(" "))
            .map(|message| preview(&message, limit))
    };
    match state.status {
        CollabAgentStatus::PendingInit => AgentOutcome::PendingInit,
        CollabAgentStatus::Running => AgentOutcome::Running,
        CollabAgentStatus::Interrupted => AgentOutcome::Interrupted,
        CollabAgentStatus::Completed => {
            AgentOutcome::Completed(message(RESPONSE_PREVIEW).filter(|message| !message.is_empty()))
        }
        CollabAgentStatus::Errored => AgentOutcome::Errored(message(ERROR_PREVIEW)),
        CollabAgentStatus::Shutdown => AgentOutcome::Shutdown,
        CollabAgentStatus::NotFound => AgentOutcome::NotFound,
    }
}

/// Oracle `truncate_text`: at most `max` characters, the last three of them
/// `...` when cut. (It counts scalar values, not grapheme clusters: the core
/// crate carries no segmentation table.)
fn preview(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max - 3).collect();
    format!("{kept}...")
}
