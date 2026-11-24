//! Side conversations (`/side`, `/btw`) as a feature slice (ADR-012; oracle `app/side.rs`): a fork
//! of the thread in use, asked quick questions while the parent stays as it was.
//!
//! The slice is the whole feature. What core keeps for IO and the commands (ADR-013) is which
//! threads are side conversations of which parent (`State::link`), what the parent last did while
//! the side is shown (`ParentStatus`, the oracle's `SideThreadState::parent_status`) and the forks
//! whose boundary is still going in (the prompt of `/btw` waits for it). The thread's life goes
//! through the lifecycle doors of `AppCx` (ADR-012, "lifecycle doors"): the fork is
//! `spawn_thread(Fork { ephemeral })` (an ephemeral `thread/fork` with the side policy as developer
//! instructions; the fork starts empty on the screen), answered in `thread_opened`; the side
//! conversation of a parent is shown again with `open_thread`, the prompt of `/btw` is `submit`ted
//! once the boundary (`thread/inject_items`, `Command::Boundary`) is in, a prompt that could not
//! go is `give_back`, and the conversation the user returned from is `discard`ed. What the parent
//! did is the slice's own reading of the server's stream (`project`, `project_request`,
//! `on_event`), and what the chat reads of the thread in use (the commands a side conversation
//! lacks are gated by a flag, the footer names the parent) the slice says through
//! `AppCx::chat_facts` whenever the thread shown or the parent's status changes (`on_shown`).
//!
//! The words are `src/features/side/strings.rs`; the footer label
//! is data here (`Footer`) and words in the view.

mod policy;

use std::collections::BTreeMap;

use codex_app_server_protocol::{
    ClientRequest, RequestId, ServerNotification, ServerRequest, ThreadInjectItemsParams,
    TurnStatus,
};

use super::{AppCx, AppView, ChatFacts, Refusal, Slash, ThreadSpawn};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use crate::protocol::targets;
use crate::rpc::RpcError;
use crate::threads::{Tone, UserMessage};

pub use policy::developer_instructions;

/// The marker type of the feature.
pub struct Side;

pub use crate::screens::chat::facts::{SideFooter as Footer, SideStatus as ParentStatus};

/// `SideParentStatus::for_request`: what a server request of the parent makes the parent.
pub fn status_for_request(request: &ServerRequest) -> Option<ParentStatus> {
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

    /// What the footer says while `shown` is displayed (`sync_side_thread_ui`): over a side
    /// conversation its parent and the parent's status, over a parent that has one the key that
    /// switches. `parent_label` words a parent that is not the main thread; `switch` is the label
    /// of the toggle key's first binding. The words are the view's (`src/features/side`).
    pub fn footer(
        &self,
        shown: &ThreadId,
        parent_is_main: bool,
        parent_label: &str,
        switch: Option<&str>,
    ) -> Option<Footer> {
        let Some(link) = self.links.get(shown) else {
            return switch
                .filter(|_| self.side_of(shown).is_some())
                .map(|key| Footer::Parent {
                    switch: key.to_owned(),
                });
        };
        Some(Footer::Side {
            parent_is_main,
            parent_label: parent_label.to_owned(),
            status: link.status,
            switch: switch.map(str::to_owned),
        })
    }
}

/// What the user says of side conversations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/side` and `/btw`, typed in `source` (none before the session has started), with the
    /// prompt of `/btw`: `State::start` says what to do.
    Start {
        source: Option<ThreadId>,
        prompt: Option<String>,
    },
    /// Return from `child` to its parent and drop it: the parent is shown, then the side
    /// conversation's turn is interrupted, the server stops holding it and the app forgets it
    /// (`select_agent_thread_and_discard_side`, `discard_side_thread`). A verb the view says for
    /// `Ctrl+C` and `Ctrl+D` over an empty composer in a side conversation. A `child` that is not
    /// shown (its parent is) is only dropped.
    Return { child: ThreadId },
}

/// What the stream says of a parent that has a side conversation (`SideParentStatusChange`): its
/// side conversation shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
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
    /// The fork of `parent` that `/side` or `/btw` asked for, with the prompt that waits for it.
    Fork {
        parent: ThreadId,
        prompt: Option<String>,
    },
    /// The side conversation `parent` already has is shown again, with the prompt that waits
    /// for it.
    Reuse {
        parent: ThreadId,
        prompt: Option<String>,
    },
    /// The parent of `child` is shown again: `child` is dropped once it is.
    Return { child: ThreadId },
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

/// A prompt that could not go (`restore_side_user_message`) goes back to the composer it was
/// typed in.
fn restore(cx: &mut AppCx<'_>, thread: &ThreadId, prompt: Option<String>) {
    if let Some(prompt) = prompt {
        cx.give_back(thread, prompt);
    }
}

/// Drops the side conversation `child` (`discard_side_thread`). Done once its parent is the thread
/// shown, never while the side conversation still is.
fn drop_side(state: &mut State, child: &ThreadId, cx: &mut AppCx<'_>) {
    if cx.view().active() == Some(child) || !state.is_side(child) {
        return;
    }
    state.links.remove(child);
    state.preparing.remove(child);
    cx.discard(child);
    cx.chat_facts(chat_facts(state, cx.view()));
}

/// `/side` and `/btw` (`handle_start_side`), typed in `source`.
fn start(state: &State, source: Option<ThreadId>, prompt: Option<String>, cx: &mut AppCx<'_>) {
    let prompt = prompt.filter(|prompt| !prompt.trim().is_empty());
    match (state.start(source.as_ref()), source) {
        (Start::Block(note), source) => {
            if let Some(thread) = &source {
                restore(cx, thread, prompt);
            }
            cx.note(source, note);
        }
        (Start::Reuse(child), Some(parent)) => {
            cx.open_thread(&child, Ctx::Reuse { parent, prompt });
        }
        (Start::Fork, Some(parent)) => cx.spawn_thread(
            ThreadSpawn::Fork {
                parent: parent.clone(),
                ephemeral: Some(developer_instructions()),
            },
            Ctx::Fork { parent, prompt },
        ),
        _ => {}
    }
}

/// What the screen reads of the thread shown (`sync_side_thread_ui`): whether it is a side
/// conversation (the commands it lacks are gated by it) and the footer's facts.
fn chat_facts(state: &State, view: AppView<'_>) -> ChatFacts {
    let shown = view.active();
    let side = shown.is_some_and(|id| state.is_side(id));
    let footer = shown.and_then(|id| {
        let parent = state.link(id).map(|link| &link.parent);
        let parent_view = parent.and_then(|parent| view.thread(parent));
        let is_main = !parent_view
            .as_ref()
            .is_some_and(|parent| parent.slots().chat.parent_owned);
        let parent_label = parent_view
            .map(|parent| {
                parent
                    .slots()
                    .chat
                    .name
                    .clone()
                    .unwrap_or_else(|| parent.id().to_string())
            })
            .unwrap_or_default();
        let switch = view.facts().key_label(GLOBAL_TOGGLE_SIDE_CONVERSATION);
        state.footer(id, is_main, &parent_label, switch.as_deref())
    });
    ChatFacts {
        side_conversation: side,
        side: footer,
    }
}

feature! {
    impl Feature for Side {
        const NAME: &'static str = "side";
        const RUNS: &'static [SlashCommand] = &[SlashCommand::Side, SlashCommand::Btw];
        type State = State;
        type Intent = Intent;
        type Command = Command;
        type Reply = Reply;
        type Ctx = Ctx;
        type Note = Note;
        type Event = Event;
        {
            /// `/side` and `/btw`, each with an optional prompt.
            fn slash(state: &mut State, slash: Slash, cx: &mut AppCx<'_>) {
                let source = slash.source.clone();
                let prompt = slash.into_args();
                Self::intent(state, Intent::Start { source, prompt }, cx);
            }

            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Start { source, prompt } => start(state, source, prompt, cx),
                    Intent::Return { child } => {
                        let Some(link) = state.links.get(&child) else {
                            return;
                        };
                        if cx.view().active() == Some(&child) {
                            // The parent first; the side conversation goes once it is shown.
                            let parent = link.parent.clone();
                            cx.open_thread(&parent, Ctx::Return { child });
                        } else {
                            drop_side(state, &child, cx);
                        }
                    }
                }
            }

            /// `SideParentStatusChange::for_notification` of the stream.
            fn project(notification: &ServerNotification) -> Option<Event> {
                Some(Event::Parent {
                    parent: ThreadId::new(targets::notification_thread(notification)?),
                    change: StatusChange::for_notification(notification)?,
                })
            }

            /// `SideParentStatus::for_request`: a request of the parent that waits for the user.
            fn project_request(request: &ServerRequest) -> Option<Event> {
                let status = status_for_request(request)?;
                let (_, thread) = targets::request_target(request);
                Some(Event::Parent {
                    parent: ThreadId::new(thread),
                    change: StatusChange::Set(status),
                })
            }

            /// What the parent did (`apply_side_parent_status_change`): each side conversation of it
            /// shows it, and the footer follows.
            fn on_event(state: &mut State, event: Event, cx: &mut AppCx<'_>) {
                let Event::Parent { parent, change } = event;
                if state.side_of(&parent).is_none() {
                    return;
                }
                for link in state
                    .links
                    .values_mut()
                    .filter(|link| link.parent == parent)
                {
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
                cx.chat_facts(chat_facts(state, cx.view()));
            }

            /// The fork is open here (`apply_side_creation`): it is a side conversation of its parent
            /// from now on and its boundary goes in (before it is shown, so the footer has it). The
            /// side conversation of a parent that was shown again gets the `/btw` prompt.
            fn thread_opened(
                state: &mut State,
                ctx: Ctx,
                result: Result<ThreadId, Refusal>,
                cx: &mut AppCx<'_>,
            ) {
                match (ctx, result) {
                    (Ctx::Fork { parent, prompt }, Ok(child)) => {
                        // `child.parent_status`: the side conversation the parent already had.
                        let status = state
                            .side_of(&parent)
                            .and_then(|other| state.links.get(other))
                            .and_then(|link| link.status);
                        state.links.insert(child.clone(), Link { parent, status });
                        state.preparing.insert(child.clone(), prompt);
                        cx.ask(
                            Some(&child),
                            Command::Boundary,
                            Ctx::Boundary(child.clone()),
                        );
                    }
                    // `side_start_error_message`: what a fork that failed says.
                    (Ctx::Fork { parent, .. }, Err(Refusal::Failed(error))) => {
                        cx.note(Some(parent), start_failure(&failure(&error)));
                    }
                    (Ctx::Fork { parent, prompt }, Err(Refusal::Busy)) => restore(cx, &parent, prompt),
                    (Ctx::Reuse { prompt, .. }, Ok(child)) => {
                        if let Some(prompt) = prompt {
                            cx.submit(&child, UserMessage::text(prompt));
                        }
                    }
                    (Ctx::Reuse { parent, prompt }, Err(_)) => restore(cx, &parent, prompt),
                    (Ctx::Return { child }, Ok(_)) => drop_side(state, &child, cx),
                    // The parent cannot be shown: the side conversation stays as it is.
                    (Ctx::Return { .. }, Err(_)) => {}
                    (Ctx::Boundary(_), _) => {}
                }
            }

            /// What the chat reads of the thread shown (`sync_side_thread_ui`).
            fn on_shown(state: &mut State, _thread: &ThreadId, cx: &mut AppCx<'_>) {
                cx.chat_facts(chat_facts(state, cx.view()));
            }

            /// The boundary is in: the prompt that waited for it goes. It is not: the side conversation
            /// stays as it is, with the line saying so (it can be closed), and the prompt is not sent.
            fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
                let Ctx::Boundary(child) = ctx else {
                    return;
                };
                let Some(prompt) = state.preparing.remove(&child) else {
                    return;
                };
                match result {
                    Ok(Reply) => {
                        if let Some(prompt) = prompt {
                            cx.submit(&child, UserMessage::text(prompt));
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
        assert_eq!(
            state.start(Some(&id("side"))),
            Start::Block(Note::AlreadyOpen)
        );
    }

    #[test]
    fn the_footer_follows_the_thread_shown() {
        let mut state = State::default();
        state.links.insert(
            id("side"),
            Link {
                parent: id("main"),
                status: Some(ParentStatus::Finished),
            },
        );
        assert_eq!(
            state.footer(&id("side"), true, "", Some("ctrl + /")),
            Some(Footer::Side {
                parent_is_main: true,
                parent_label: String::new(),
                status: Some(ParentStatus::Finished),
                switch: Some("ctrl + /".to_owned()),
            })
        );
        assert_eq!(
            state.footer(&id("main"), true, "", Some("ctrl + /")),
            Some(Footer::Parent {
                switch: "ctrl + /".to_owned()
            })
        );
        assert_eq!(state.footer(&id("other"), true, "", Some("ctrl + /")), None);
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
