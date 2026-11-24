//! Side conversations (`/side`, `/btw`) as a feature slice (ADR-012; oracle `app/side.rs`): a fork
//! of the thread in use, asked quick questions while the parent stays as it was.
//!
//! What core keeps is what IO and the commands need (ADR-013): which threads are side
//! conversations of which parent (`State::link`), what the parent last did while the side is shown
//! (`ParentStatus`, the oracle's `SideThreadState::parent_status`), and the forks whose boundary is
//! still going in (the prompt of `/btw` waits for it). The thread lifecycle is not the slice's
//! (ADR-012, "half a slice"): the fork is `ThreadAsk::ForkAt { side: true }` (an ephemeral
//! `thread/fork` with the side policy as developer instructions), switching and discarding go
//! through the app's own doors (`app/side.rs`), which read the decisions made here (`State::start`)
//! and report what happened as intents (`Linked`, `Dropped`, `Parent`). The fork's history is
//! reference-only to the model: the policy and the boundary go in as `thread/inject_items`
//! (`Command::Boundary`), and a `/btw` prompt is sent once they are in (`State::take_ready`).
//!
//! The words are `src/features/side/strings.rs`; the footer label (`Side from main thread · ...`)
//! is made by `label` from the facts here.

mod policy;

use std::collections::BTreeMap;

use codex_app_server_protocol::{
    ClientRequest, RequestId, ServerNotification, ServerRequest, ThreadInjectItemsParams,
    TurnStatus,
};

use super::{AppCx, Feature, Never};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use policy::developer_instructions;

/// The marker type of the feature.
pub struct Side;

/// A parent thread's condition that changed while its side conversation is shown
/// (`SideParentStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentStatus {
    NeedsInput,
    NeedsApproval,
    Failed,
    Interrupted,
    Closed,
    Finished,
}

impl ParentStatus {
    /// The label's part (`main needs input`, `parent finished`...).
    pub fn label(self, parent_is_main: bool) -> &'static str {
        match (self, parent_is_main) {
            (ParentStatus::NeedsInput, true) => "main needs input",
            (ParentStatus::NeedsInput, false) => "parent needs input",
            (ParentStatus::NeedsApproval, true) => "main needs approval",
            (ParentStatus::NeedsApproval, false) => "parent needs approval",
            (ParentStatus::Failed, true) => "main failed",
            (ParentStatus::Failed, false) => "parent failed",
            (ParentStatus::Interrupted, true) => "main interrupted",
            (ParentStatus::Interrupted, false) => "parent interrupted",
            (ParentStatus::Closed, true) => "main closed",
            (ParentStatus::Closed, false) => "parent closed",
            (ParentStatus::Finished, true) => "main finished",
            (ParentStatus::Finished, false) => "parent finished",
        }
    }

    /// A status the user can act on (an approval or a question waits): it clears when the request
    /// is resolved or an item starts.
    pub fn is_actionable(self) -> bool {
        matches!(self, ParentStatus::NeedsInput | ParentStatus::NeedsApproval)
    }

    /// `SideParentStatus::for_request`: what a server request of the parent makes the parent.
    pub fn for_request(request: &ServerRequest) -> Option<Self> {
        match request {
            ServerRequest::ToolRequestUserInput { .. } => Some(ParentStatus::NeedsInput),
            ServerRequest::CommandExecutionRequestApproval { .. }
            | ServerRequest::FileChangeRequestApproval { .. }
            | ServerRequest::McpServerElicitationRequest { .. }
            | ServerRequest::PermissionsRequestApproval { .. }
            | ServerRequest::ApplyPatchApproval { .. }
            | ServerRequest::ExecCommandApproval { .. } => Some(ParentStatus::NeedsApproval),
            _ => None,
        }
    }
}

/// What a notification or request of the parent does to its side conversations' status
/// (`SideParentStatusChange`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusChange {
    Set(ParentStatus),
    Clear,
    ClearActionable,
}

impl StatusChange {
    /// `SideParentStatusChange::for_notification`.
    pub fn for_notification(notification: &ServerNotification) -> Option<Self> {
        match notification {
            ServerNotification::TurnStarted(_) => Some(StatusChange::Clear),
            ServerNotification::TurnCompleted(notification) => match &notification.turn.status {
                TurnStatus::Completed => Some(StatusChange::Set(ParentStatus::Finished)),
                TurnStatus::Interrupted => Some(StatusChange::Set(ParentStatus::Interrupted)),
                TurnStatus::Failed => Some(StatusChange::Set(ParentStatus::Failed)),
                TurnStatus::InProgress => None,
            },
            ServerNotification::ThreadClosed(_) => Some(StatusChange::Set(ParentStatus::Closed)),
            ServerNotification::ItemStarted(_) | ServerNotification::ServerRequestResolved(_) => {
                Some(StatusChange::ClearActionable)
            }
            _ => None,
        }
    }
}

/// A side conversation and the thread it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub parent: ThreadId,
    /// Parent-thread condition that changed while this side thread was visible.
    pub status: Option<ParentStatus>,
}

/// The app-scoped state: the side conversations this process holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    links: BTreeMap<ThreadId, Link>,
    /// The forks whose boundary is going in, with the prompt (`/btw`) that waits for it.
    preparing: BTreeMap<ThreadId, Option<String>>,
    /// The prompts whose boundary is in: sent by the app (`take_ready`).
    ready: Vec<(ThreadId, String)>,
}

/// What `/side` does in the thread it was typed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Start {
    /// Not now; the note says why and the prompt goes back to the composer.
    Block(Note),
    /// This parent has a side conversation already: it is shown again (`/side` reuses it).
    Reuse(ThreadId),
    /// A new fork.
    Fork,
}

impl State {
    /// `side_start_block_message`, then the reuse of the parent's own side conversation
    /// (`handle_start_side`): `source` is the thread `/side` was typed in.
    pub fn start(&self, source: Option<&ThreadId>) -> Start {
        let Some(source) = source else {
            return Start::Block(Note::MainUnavailable);
        };
        if self.is_side(source) {
            return Start::Block(Note::AlreadyOpen);
        }
        match self.side_of(source) {
            Some(child) => Start::Reuse(child.clone()),
            None => Start::Fork,
        }
    }

    /// `thread` is a side conversation.
    pub fn is_side(&self, thread: &ThreadId) -> bool {
        self.links.contains_key(thread)
    }

    /// The side conversation and its parent, for a side thread.
    pub fn link(&self, thread: &ThreadId) -> Option<&Link> {
        self.links.get(thread)
    }

    /// The side conversation `parent` has (`side_thread_for_parent`).
    pub fn side_of(&self, parent: &ThreadId) -> Option<&ThreadId> {
        self.links
            .iter()
            .find_map(|(child, link)| (link.parent == *parent).then_some(child))
    }

    /// The thread `Ctrl+/` switches to from `shown`: the parent of a side conversation, the side
    /// conversation of a parent (`toggle_side_conversation`).
    pub fn counterpart(&self, shown: &ThreadId) -> Option<&ThreadId> {
        match self.links.get(shown) {
            Some(link) => Some(&link.parent),
            None => self.side_of(shown),
        }
    }

    /// The prompts whose boundary went in since the last call: they are to be sent.
    pub fn take_ready(&mut self) -> Vec<(ThreadId, String)> {
        std::mem::take(&mut self.ready)
    }

    /// The footer's label while `shown` is displayed (`sync_side_thread_ui`): over a side
    /// conversation `Side from main thread · main finished · ctrl + / to switch · ctrl + c to
    /// close`, over a parent that has one `ctrl + / for side`. `parent_label` words a parent that
    /// is not the main thread; `switch` is the label of the toggle key's first binding.
    pub fn label(
        &self,
        shown: &ThreadId,
        parent_is_main: bool,
        parent_label: &str,
        switch: Option<&str>,
    ) -> Option<String> {
        let Some(link) = self.links.get(shown) else {
            return switch
                .filter(|_| self.side_of(shown).is_some())
                .map(|key| format!("{key} for side"));
        };
        let mut parts = Vec::new();
        if parent_is_main {
            parts.push("from main thread".to_owned());
        } else {
            parts.push(format!("from parent thread ({parent_label})"));
        }
        if let Some(status) = link.status {
            parts.push(status.label(parent_is_main).to_owned());
        }
        if let Some(key) = switch {
            parts.push(format!("{key} to switch"));
        }
        parts.push("ctrl + c to close".to_owned());
        Some(format!("Side {}", parts.join(" · ")))
    }
}

/// What the user or the app says of side conversations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/side` and `/btw`, typed in `source` (none before the session has started), with the
    /// prompt of `/btw`. Run by the app (`app/side.rs`): it asks `State::start` what to do.
    Start {
        source: Option<ThreadId>,
        prompt: Option<String>,
    },
    /// Return from `child` and drop it: its turn is interrupted, the server stops holding it and
    /// the app forgets it (`discard_side_thread`). A verb the view says for `Ctrl+C` over an
    /// empty composer; the view shows the parent (`OpenThread`) itself.
    Discard { child: ThreadId },
    /// The fork `child` of `parent` is open here: it is a side conversation from now on, and its
    /// boundary is injected (`prompt` is sent once it is in).
    Linked {
        child: ThreadId,
        parent: ThreadId,
        prompt: Option<String>,
    },
    /// `child` is gone (discarded, or the thread was removed).
    Dropped { child: ThreadId },
    /// What `parent` did: its side conversation shows it.
    Parent {
        parent: ThreadId,
        change: StatusChange,
    },
}

/// The RPCs of the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `thread/inject_items`: the policy and the boundary, before the first side turn.
    Boundary,
}

/// What `Command::Boundary` answered: nothing but that it went in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply;

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// The boundary of this fork.
    Boundary(ThreadId),
}

/// What the feature says in the transcript; all of them are errors in the oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `'/side' is unavailable until the main thread is ready.`
    MainUnavailable,
    /// `'/side' is unavailable until the current conversation has started. Send a message first,
    /// then try /side again.`
    NoStartedConversation,
    /// `A side conversation is already open. Press ctrl + c to return before starting another.`
    AlreadyOpen,
    /// `Failed to start side conversation: {error}`
    StartFailed(String),
    /// `Failed to prepare side conversation {thread}: {error}`
    PrepareFailed { thread: ThreadId, error: String },
    /// `Failed to close side conversation {thread}; it is still open: {error}`
    CloseFailed { thread: ThreadId, error: String },
}

/// `side_start_error_message`: what a fork that failed says. The server has no rollout to fork
/// before the first message.
pub fn start_failure(error: &str) -> Note {
    if error.contains("no rollout found for thread id")
        || error.contains("includeTurns is unavailable before first user message")
    {
        Note::NoStartedConversation
    } else {
        Note::StartFailed(error.to_owned())
    }
}

/// The failure of an RPC as a line.
fn failure(error: &RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) | RpcError::Prepare(message) => {
            message.clone()
        }
        other => format!("{other:?}"),
    }
}

impl Feature for Side {
    const NAME: &'static str = "side";

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            // The lifecycle half, run by the app before the slice is asked.
            Intent::Start { .. } | Intent::Discard { .. } => {}
            Intent::Linked {
                child,
                parent,
                prompt,
            } => {
                // `child.parent_status`: the side conversation the parent already had.
                let status = state
                    .side_of(&parent)
                    .and_then(|other| state.links.get(other))
                    .and_then(|link| link.status);
                state.links.insert(child.clone(), Link { parent, status });
                state.preparing.insert(child.clone(), prompt);
                cx.ask(Some(&child), Command::Boundary, Ctx::Boundary(child.clone()));
            }
            Intent::Dropped { child } => {
                state.links.remove(&child);
                state.preparing.remove(&child);
                state.ready.retain(|(thread, _)| *thread != child);
            }
            Intent::Parent { parent, change } => {
                for link in state.links.values_mut().filter(|link| link.parent == parent) {
                    match change {
                        StatusChange::Set(status) => link.status = Some(status),
                        StatusChange::Clear => link.status = None,
                        StatusChange::ClearActionable => {
                            if link.status.is_some_and(ParentStatus::is_actionable) {
                                link.status = None;
                            }
                        }
                    }
                }
            }
        }
    }

    /// The boundary is in: the prompt that waited for it goes. It is not: the side conversation
    /// stays as it is, with the line saying so (it can be closed), and the prompt is not sent.
    fn answered(
        state: &mut State,
        ctx: Ctx,
        result: Result<Reply, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let Ctx::Boundary(child) = ctx;
        let Some(prompt) = state.preparing.remove(&child) else {
            return;
        };
        match result {
            Ok(Reply) => {
                if let Some(prompt) = prompt {
                    state.ready.push((child, prompt));
                }
            }
            Err(error) => cx.note(
                Some(child.clone()),
                Note::PrepareFailed {
                    thread: child,
                    error: failure(&error),
                },
            ),
        }
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Error
    }

    fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
        match command {
            Command::Boundary => ClientRequest::ThreadInjectItems {
                request_id: id,
                params: ThreadInjectItemsParams {
                    thread_id: thread.to_string(),
                    items: policy::fork_items(),
                },
            },
        }
    }

    fn decode(_command: &Command, _result: serde_json::Value) -> Result<Reply, RpcError> {
        Ok(Reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> ThreadId {
        ThreadId::new(name)
    }

    #[test]
    fn start_asks_the_state_of_the_thread() {
        let mut state = State::default();
        assert_eq!(state.start(None), Start::Block(Note::MainUnavailable));
        assert_eq!(state.start(Some(&id("main"))), Start::Fork);
        state.links.insert(
            id("side"),
            Link {
                parent: id("main"),
                status: None,
            },
        );
        assert_eq!(state.start(Some(&id("main"))), Start::Reuse(id("side")));
        assert_eq!(state.start(Some(&id("side"))), Start::Block(Note::AlreadyOpen));
    }

    #[test]
    fn the_label_follows_the_thread_shown() {
        let mut state = State::default();
        state.links.insert(
            id("side"),
            Link {
                parent: id("main"),
                status: Some(ParentStatus::Finished),
            },
        );
        assert_eq!(
            state.label(&id("side"), true, "", Some("ctrl + /")).as_deref(),
            Some("Side from main thread · main finished · ctrl + / to switch · ctrl + c to close")
        );
        assert_eq!(
            state.label(&id("main"), true, "", Some("ctrl + /")).as_deref(),
            Some("ctrl + / for side")
        );
        assert_eq!(state.label(&id("other"), true, "", Some("ctrl + /")), None);
    }

    #[test]
    fn a_missing_rollout_is_the_first_message_note() {
        assert_eq!(
            start_failure("thread/fork failed: no rollout found for thread id 019d"),
            Note::NoStartedConversation
        );
        assert_eq!(
            start_failure("transport disconnected"),
            Note::StartFailed("transport disconnected".to_owned())
        );
    }
}
