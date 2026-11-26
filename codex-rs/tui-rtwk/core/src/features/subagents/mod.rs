//! The transcript report produced by opening the subagent picker.
//! Oracle: `app/session_lifecycle.rs` and `app/agent_status_feed.rs`.

use codex_app_server_protocol::{SessionSource, ThreadStatus};
use codex_protocol::protocol::SubAgentSource;

use super::{AppCx, ThreadView};
use crate::ids::ThreadId;
use crate::screens::select::agent_picker::PickerKey;
use crate::threads::{CollabEvent, Item, Notice, SubAgentKind, ToolKind};

pub struct Subagents;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    reported: Option<PickerKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    NoAgents(Option<ThreadId>),
    Report { source: ThreadId, picker: PickerKey },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    NoAgents,
    Running(Vec<Entry>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub recent: Vec<Activity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activity {
    Text(String),
    Command(String),
    Updated(usize),
    Mcp(String, String),
    Tool(String),
    Agent(AgentAction),
    Subagent(SubAgentKind, String),
    Web(String),
    Viewed(String),
    Generated,
    EnteredReview,
    ExitedReview,
    Compacted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentAction {
    Spawned,
    SentInput,
    Resumed,
    Waited,
    Closed,
}

feature! {
    impl Feature for Subagents {
        const NAME: &'static str = "subagents";
        type State = State;
        type Intent = Intent;
        type Note = Note;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::NoAgents(source) => {
                        let view = cx.view();
                        if view.active() == source.as_ref()
                            && source.as_ref().and_then(|id| view.screens().agents.descendants_root(id)).is_none()
                        {
                            cx.note(source, Note::NoAgents);
                        }
                    }
                    Intent::Report { source, picker } => {
                        if state.reported.as_ref() == Some(&picker) {
                            return;
                        }
                        if let Some(note) = report(&source, &picker, cx) {
                            state.reported = Some(picker);
                            cx.note(Some(source), note);
                        }
                    }
                }
            }
        }
    }
}

fn report(source: &ThreadId, picker: &PickerKey, cx: &AppCx<'_>) -> Option<Note> {
    let view = cx.view();
    let agents = &view.screens().agents;
    let root = agents.descendants_root(source)?;
    let active = view.active()?;
    if !agents.wants_descendants(picker, &root)
        || agents.descendants_root(active).as_ref() != Some(&root)
        || view.thread(source).is_none()
    {
        return None;
    }
    let mut saw_path = false;
    let mut running = Vec::new();
    for thread in &agents.threads {
        if agents.hidden.contains(&thread.id)
            || agents.removed.contains(&thread.id)
            || !agents.is_descendant(&root, thread)
        {
            continue;
        }
        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            agent_path: Some(path),
            ..
        }) = &thread.source
        else {
            continue;
        };
        let path = String::from(path.clone());
        if path.trim().is_empty() {
            continue;
        }
        saw_path = true;
        let held = view.thread(&ThreadId::new(&thread.id));
        if is_running(&thread.status, held.as_ref()) {
            running.push(Entry {
                path,
                recent: held.map_or_else(Vec::new, |thread| recent(&thread)),
            });
        }
    }
    saw_path.then_some(Note::Running(running))
}

fn is_running(status: &ThreadStatus, held: Option<&ThreadView<'_>>) -> bool {
    if let Some(core) = held
        .map(ThreadView::core)
        .filter(|core| core.attached() && !core.stream_lost() && !core.history_may_be_incomplete())
    {
        if core.is_running() {
            return true;
        }
        let transcript = core.transcript();
        if transcript
            .last_turn()
            .is_some_and(|turn| core.turn_finished(turn))
        {
            return false;
        }
    }
    matches!(status, ThreadStatus::Active { .. })
}

fn recent(thread: &ThreadView<'_>) -> Vec<Activity> {
    let transcript = thread.core().transcript();
    let mut seen = std::collections::HashSet::new();
    let mut activity: Vec<_> = (0..transcript.len())
        .rev()
        .filter_map(|index| transcript.get(index))
        .filter(|(key, _)| seen.insert(&key.item))
        .filter_map(|(_, item)| summarize(item))
        .take(6)
        .collect();
    activity.reverse();
    activity
}

/// Keep only enough source for the view's terminal grapheme budget.
fn excerpt(text: &str) -> String {
    text.chars().take(960).collect()
}

fn summarize(item: &Item) -> Option<Activity> {
    Some(match item {
        Item::Agent { text, .. } | Item::Plan { text } if !text.trim().is_empty() => {
            Activity::Text(excerpt(text))
        }
        Item::Reasoning { summary, .. } => Activity::Text(excerpt(
            summary.lines().rev().find(|line| !line.trim().is_empty())?,
        )),
        Item::Tool(tool) => match &tool.kind {
            ToolKind::Shell { command, .. } => Activity::Command(excerpt(command)),
            ToolKind::Mcp(call) => Activity::Mcp(excerpt(&call.server), excerpt(&call.tool)),
            ToolKind::Dynamic(call) => Activity::Tool(excerpt(&call.name)),
        },
        Item::Patch(patch) => Activity::Updated(patch.changes.len()),
        Item::Collab(event) => match event.as_ref() {
            CollabEvent::Spawned { .. } | CollabEvent::SpawnFailed { .. } => {
                Activity::Agent(AgentAction::Spawned)
            }
            CollabEvent::SentInput { .. } => Activity::Agent(AgentAction::SentInput),
            CollabEvent::Resuming { .. } | CollabEvent::Resumed { .. } => {
                Activity::Agent(AgentAction::Resumed)
            }
            CollabEvent::Waiting { .. } | CollabEvent::FinishedWaiting { .. } => {
                Activity::Agent(AgentAction::Waited)
            }
            CollabEvent::Closed { .. } => Activity::Agent(AgentAction::Closed),
            CollabEvent::Activity { kind, path } => Activity::Subagent(*kind, excerpt(path)),
        },
        Item::Web(web) => Activity::Web(excerpt(&web.detail)),
        Item::ImageView { path, .. } => Activity::Viewed(excerpt(path)),
        Item::Generated(_) => Activity::Generated,
        Item::Notice(Notice::EnteredReview(_)) => Activity::EnteredReview,
        Item::Notice(Notice::ExitedReview(_)) => Activity::ExitedReview,
        Item::Notice(Notice::ContextCompacted { .. }) => Activity::Compacted,
        _ => return None,
    })
}
