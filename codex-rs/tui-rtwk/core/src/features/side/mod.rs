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
mod saved;

use std::collections::BTreeMap;

use codex_app_server_protocol::{
    ClientRequest, RequestId, ServerNotification, ServerRequest, ThreadInjectItemsParams,
    ThreadInjectItemsResponse, TurnStatus,
};

use super::{AppCx, AppView, ChatFacts, Refusal, Slash, ThreadSpawn};
use crate::commands::{CommandRule, Need, SlashCommand};
use crate::ids::ThreadId;
use crate::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use crate::protocol::targets;
use crate::rpc::RpcError;
use crate::threads::{Tone, UserMessage};

pub use policy::developer_instructions;
pub use saved::{SavedChat, SavedChats, SavedReply};

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

/// Whether a side conversation is held only for this client or persists on the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    Temporary,
    Attached,
}

/// A side conversation and the thread it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub parent: ThreadId,
    pub storage: Storage,
    /// Parent-thread condition that changed while this side thread was visible.
    pub status: Option<ParentStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Children {
    ordered: Vec<ThreadId>,
    visits: Vec<ThreadId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Activation {
    parent: ThreadId,
    enter: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Preparation {
    prompt: Option<String>,
    activation: Option<Activation>,
    pending_link: Option<Link>,
}

/// The app-scoped state: the side conversations this process holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    links: BTreeMap<ThreadId, Link>,
    children: BTreeMap<ThreadId, Children>,
    /// Navigation is disarmed permanently on the first departure from the initiating parent.
    opening: Option<Activation>,
    preparing: BTreeMap<ThreadId, Preparation>,
    parked: BTreeMap<ThreadId, String>,
    saved: saved::State,
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
    pub fn start(&self, source: Option<&ThreadId>, fresh: bool) -> Start {
        let Some(source) = source else {
            return Start::Block(Note::MainUnavailable);
        };
        if self.is_side(source) {
            return Start::Block(Note::AlreadyOpen);
        }
        if let Some(child) = self.side_of(source).filter(|_| !fresh) {
            Start::Reuse(child.clone())
        } else {
            Start::Fork
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

    /// The last discovery state for this parent, if discovery has been requested.
    pub fn saved(&self, parent: &ThreadId) -> Option<&SavedChats> {
        self.saved.get(parent)
    }

    /// The backend has acknowledged archiving this side conversation.
    pub fn is_archived(&self, child: &ThreadId) -> bool {
        self.saved.is_archived(child)
    }

    /// The children accepted for this parent, in creation order.
    pub fn children_of(&self, parent: &ThreadId) -> &[ThreadId] {
        self.children
            .get(parent)
            .map_or(&[], |children| children.ordered.as_slice())
    }

    /// The last shown child of this parent, or the first available one.
    pub fn side_of(&self, parent: &ThreadId) -> Option<&ThreadId> {
        let children = self.children.get(parent)?;
        children.visits.last().or_else(|| children.ordered.first())
    }

    fn previous_child(&self, parent: &ThreadId, child: &ThreadId) -> Option<&ThreadId> {
        self.children
            .get(parent)?
            .visits
            .iter()
            .rev()
            .find(|id| *id != child)
    }

    fn remember(&mut self, child: ThreadId, link: Link) {
        if self.links.contains_key(&child) {
            return;
        }
        self.children
            .entry(link.parent.clone())
            .or_insert_with(|| Children {
                ordered: Vec::new(),
                visits: Vec::new(),
            })
            .ordered
            .push(child.clone());
        self.links.insert(child, link);
    }

    fn shown(&mut self, child: &ThreadId) {
        self.saved.depart(child);
        for activation in self.opening.iter_mut().chain(
            self.preparing
                .values_mut()
                .filter_map(|preparing| preparing.activation.as_mut()),
        ) {
            if activation.parent != *child {
                activation.enter = false;
            }
        }
        if let Some(link) = self.links.get(child)
            && let Some(children) = self.children.get_mut(&link.parent)
        {
            children.visits.retain(|id| id != child);
            children.visits.push(child.clone());
        }
    }

    fn forget(&mut self, child: &ThreadId) {
        let Some(link) = self.links.remove(child) else {
            return;
        };
        let Some(children) = self.children.get_mut(&link.parent) else {
            return;
        };
        children.ordered.retain(|id| id != child);
        children.visits.retain(|id| id != child);
        if children.ordered.is_empty() {
            self.children.remove(&link.parent);
        }
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
    /// Start a fresh side conversation from the shown workspace parent.
    Create {
        parent: ThreadId,
    },
    /// Open a specific child of the shown conversation family.
    Select {
        parent: ThreadId,
        child: ThreadId,
    },
    /// Close a workspace tab, moving from an active child to its most recently used sibling.
    Unload {
        parent: ThreadId,
        child: ThreadId,
    },
    RefreshSaved {
        parent: ThreadId,
    },
    OpenSaved {
        parent: ThreadId,
        child: ThreadId,
    },
    ArchiveSaved {
        parent: ThreadId,
        child: ThreadId,
    },
    /// Return from `child` to its parent and drop it: the parent is shown, then the side
    /// conversation's turn is interrupted, the server stops holding it and the app forgets it
    /// (`select_agent_thread_and_discard_side`, `discard_side_thread`). A verb the view says for
    /// `Ctrl+C` and `Ctrl+D` over an empty composer in a side conversation. A `child` that is not
    /// shown (its parent is) is only dropped.
    Return {
        child: ThreadId,
    },
}

/// What the stream says of a parent that has a side conversation (`SideParentStatusChange`): its
/// side conversation shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Archived(ThreadId),
    Unarchived(ThreadId),
    Parent {
        parent: ThreadId,
        change: StatusChange,
    },
}

/// The RPCs of the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `thread/inject_items`: the policy and the boundary, before the first side turn.
    Boundary {
        storage: Storage,
    },
    Saved(crate::app::AttachedHistory),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Boundary,
    Saved(SavedReply),
}

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Saved(saved::Ctx),
    /// The boundary of this fork.
    Boundary(ThreadId),
    Rollback {
        child: ThreadId,
        parent: ThreadId,
        prepare_error: String,
    },
    /// The fork of `parent` that `/side` or `/btw` asked for, with the prompt that waits for it.
    Fork {
        parent: ThreadId,
        prompt: Option<String>,
        retained: bool,
    },
    /// The side conversation `parent` already has is shown again, with the prompt that waits
    /// for it.
    Reuse {
        parent: ThreadId,
        prompt: Option<String>,
    },
    /// The parent of `child` is shown again: `child` is dropped once it is.
    Return {
        child: ThreadId,
    },
}

/// What the feature says in the transcript; all of them are errors in the oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `'/side' is unavailable until the main thread is ready.`
    MainUnavailable,
    /// `'/{command}' is unavailable before the session starts.` (`/side` or `/btw` typed where the
    /// chat has no thread yet: the chat's own refusal, before the app is asked)
    BeforeSession(&'static str),
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
    if unlink(state, child, cx) {
        cx.discard(child);
    }
}

fn unlink(state: &mut State, child: &ThreadId, cx: &mut AppCx<'_>) -> bool {
    if cx.view().active() == Some(child) || !state.is_side(child) {
        return false;
    }
    state.forget(child);
    state.preparing.remove(child);
    state.parked.remove(child);
    cx.chat_facts(chat_facts(state, cx.view()));
    true
}

fn close_side(
    state: &mut State,
    parent: &ThreadId,
    child: ThreadId,
    workspace: bool,
    cx: &mut AppCx<'_>,
) {
    if !state
        .link(&child)
        .is_some_and(|link| &link.parent == parent)
    {
        return;
    }
    let shown = cx.view().active();
    if shown == Some(&child) {
        let next = if workspace {
            state.previous_child(parent, &child).unwrap_or(parent)
        } else {
            parent
        };
        cx.open_thread(next, Ctx::Return { child });
    } else if shown == Some(parent)
        || shown
            .and_then(|thread| state.link(thread))
            .is_some_and(|link| &link.parent == parent)
    {
        drop_side(state, &child, cx);
    }
}

/// `/side` and `/btw` (`handle_start_side`), typed in `source`.
fn start(
    state: &mut State,
    source: Option<ThreadId>,
    prompt: Option<String>,
    fresh: bool,
    retained: bool,
    cx: &mut AppCx<'_>,
) {
    let prompt = prompt.filter(|prompt| !prompt.trim().is_empty());
    if retained
        && let Some(parent) = source.as_ref()
        && (state
            .opening
            .as_ref()
            .is_some_and(|activation| &activation.parent == parent)
            || state.preparing.values().any(|preparation| {
                preparation
                    .pending_link
                    .as_ref()
                    .is_some_and(|link| &link.parent == parent)
            }))
    {
        return restore(cx, parent, prompt);
    }
    match (state.start(source.as_ref(), fresh), source) {
        (Start::Block(note), source) => {
            if let Some(thread) = &source {
                restore(cx, thread, prompt);
            }
            cx.note(source, note);
        }
        (Start::Reuse(child), Some(parent)) => {
            if (prompt.is_some() && state.preparing.contains_key(&child))
                || state.saved.unavailable_child(&parent, &child)
            {
                return restore(cx, &parent, prompt);
            }
            starting(cx);
            cx.open_thread(&child, Ctx::Reuse { parent, prompt });
        }
        (Start::Fork, Some(parent)) => {
            if retained && state.opening.is_some() {
                return restore(cx, &parent, prompt);
            }
            starting(cx);
            let spawn = if retained {
                state.opening = Some(Activation {
                    parent: parent.clone(),
                    enter: true,
                });
                ThreadSpawn::SideFork {
                    parent: parent.clone(),
                    temporary_policy: developer_instructions(),
                    attached_policy: policy::instructions(Storage::Attached),
                }
            } else {
                ThreadSpawn::Fork {
                    parent: parent.clone(),
                    ephemeral: Some(developer_instructions()),
                }
            };
            cx.spawn_thread(
                spawn,
                Ctx::Fork {
                    parent,
                    prompt,
                    retained,
                },
            );
        }
        _ => {}
    }
}

/// The footer says `Side starting...` until the thread is shown or the start fails
/// (`request_side_conversation`'s context label). The thread in use is not a side conversation:
/// `/side` there is blocked.
fn starting(cx: &mut AppCx<'_>) {
    cx.chat_facts(ChatFacts {
        side_conversation: false,
        side: Some(Footer::Starting),
    });
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
        const COMMAND_RULES: &'static [CommandRule] = &[
            CommandRule::new(SlashCommand::Side).requiring(Need::OutsideReview),
            CommandRule::new(SlashCommand::Btw).requiring(Need::OutsideReview),
        ];
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
                if slash.source.is_none() {
                    return cx.note(None, Note::BeforeSession(slash.command.command()));
                }
                let tui = cx.view().facts().tui();
                let retained = tui.workspace_enabled && tui.transcript_owned;
                let source = slash.source.clone();
                let prompt = slash.into_args();
                let fresh = retained && prompt.as_ref().is_some_and(|prompt| !prompt.trim().is_empty());
                start(state, source, prompt, fresh, retained, cx);
            }

            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Start { source, prompt } => start(state, source, prompt, false, false, cx),
                    Intent::Create { parent } => {
                        let view = cx.view();
                        let tui = view.facts().tui();
                        if tui.workspace_enabled && tui.transcript_owned
                            && view.active() == Some(&parent)
                            && view.thread(&parent).is_some_and(|thread| thread.core().has_turn_history())
                        {
                            start(state, Some(parent), None, true, true, cx);
                        }
                    }
                    Intent::Select { parent, child } => {
                        let view = cx.view();
                        let shown = view.active();
                        let family_shown = shown == Some(&parent)
                            || shown.and_then(|id| state.link(id))
                                .is_some_and(|link| link.parent == parent);
                        if family_shown
                            && state.link(&child).is_some_and(|link| link.parent == parent)
                            && !state.preparing.contains_key(&child)
                            && !state.saved.unavailable_child(&parent, &child)
                        {
                            cx.open_thread(&child, Ctx::Reuse { parent, prompt: None });
                        }
                    }
                    Intent::Unload { parent, child } => close_side(state, &parent, child, true, cx),
                    Intent::RefreshSaved { parent } => saved::refresh(state, parent, cx),
                    Intent::OpenSaved { parent, child } => saved::open(state, parent, child, cx),
                    Intent::ArchiveSaved { parent, child } => saved::archive(state, parent, child, cx),
                    Intent::Return { child } => {
                        let Some(link) = state.links.get(&child) else {
                            return;
                        };
                        close_side(state, &link.parent.clone(), child, false, cx);
                    }
                }
            }

            /// `SideParentStatusChange::for_notification` of the stream.
            fn project(notification: &ServerNotification) -> Option<Event> {
                match notification {
                    ServerNotification::ThreadArchived(event) => return Some(Event::Archived(ThreadId::new(&event.thread_id))),
                    ServerNotification::ThreadUnarchived(event) => return Some(Event::Unarchived(ThreadId::new(&event.thread_id))),
                    _ => {}
                }
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
                let (parent, change) = match event {
                    Event::Archived(child) => return saved::archived(state, child, cx),
                    Event::Unarchived(child) => return state.saved.unarchived(&child),
                    Event::Parent { parent, change } => (parent, change),
                };
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
                    (Ctx::Saved(ctx), result) => saved::opened(state, ctx, result, cx),
                    (Ctx::Fork { parent, prompt, retained }, Ok(child)) => {
                        // `child.parent_status`: the side conversation the parent already had.
                        let status = state
                            .side_of(&parent)
                            .and_then(|other| state.links.get(other))
                            .and_then(|link| link.status);
                        let storage = if retained && cx.view().thread(&child)
                            .is_some_and(|thread| !thread.slots().chat.ephemeral)
                        {
                            Storage::Attached
                        } else {
                            Storage::Temporary
                        };
                        let link = Link { parent, status, storage };
                        let (activation, pending_link) = if retained {
                            (state.opening.take(), Some(link))
                        } else {
                            state.remember(child.clone(), link);
                            (None, None)
                        };
                        state.preparing.insert(child.clone(), Preparation { prompt, activation, pending_link });
                        // a turn interrupted here is no news (`should_suppress_interrupted_turn_notice`)
                        cx.quiet_interrupts(&child, true);
                        cx.ask(
                            Some(&child),
                            Command::Boundary { storage },
                            Ctx::Boundary(child.clone()),
                        );
                    }
                    // `side_start_error_message`: what a fork that failed says.
                    (Ctx::Fork { parent, retained, .. }, Err(Refusal::Failed(error))) => {
                        if retained {
                            state.opening = None;
                        }
                        cx.chat_facts(chat_facts(state, cx.view()));
                        cx.note(Some(parent), start_failure(&failure(&error)));
                    }
                    (Ctx::Fork { parent, prompt, retained }, Err(Refusal::Busy)) => {
                        if retained {
                            state.opening = None;
                        }
                        cx.chat_facts(chat_facts(state, cx.view()));
                        restore(cx, &parent, prompt);
                    }
                    (Ctx::Reuse { prompt, .. }, Ok(child)) => {
                        if let Some(prompt) = prompt {
                            cx.submit(&child, UserMessage::text(prompt));
                        }
                    }
                    (Ctx::Reuse { parent, prompt }, Err(_)) => {
                        cx.chat_facts(chat_facts(state, cx.view()));
                        restore(cx, &parent, prompt);
                    }
                    (Ctx::Return { child }, Ok(_)) => drop_side(state, &child, cx),
                    // The parent cannot be shown: the side conversation stays as it is.
                    (Ctx::Return { .. }, Err(_)) => {}
                    (Ctx::Rollback { child, parent, prepare_error }, result) => {
                        if let Err(refusal) = result {
                            let error = match refusal {
                                Refusal::Failed(error) => failure(&error),
                                Refusal::Busy => format!("{refusal:?}"),
                            };
                            cx.note(Some(parent.clone()), Note::CloseFailed { thread: child.clone(), error });
                        }
                        cx.note(Some(parent), Note::PrepareFailed { thread: child, error: prepare_error });
                    }
                    (Ctx::Boundary(_), _) => {}
                }
            }

            /// What the chat reads of the thread shown (`sync_side_thread_ui`).
            fn on_shown(state: &mut State, thread: &ThreadId, cx: &mut AppCx<'_>) {
                state.shown(thread);
                saved::discard_archived_background(state, thread, cx);
                cx.chat_facts(chat_facts(state, cx.view()));
                if let Some(prompt) = state.parked.remove(thread) {
                    cx.submit(thread, UserMessage::text(prompt));
                }
            }

            /// The boundary is in: the prompt that waited for it goes. It is not: the side conversation
            /// stays as it is, with the line saying so (it can be closed), and the prompt is not sent.
            fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
                if let Ctx::Saved(ctx) = ctx {
                    return saved::answered(state, ctx, result, cx);
                }
                let Ctx::Boundary(child) = ctx else {
                    return;
                };
                let Some(preparation) = state.preparing.remove(&child) else {
                    return;
                };
                match result {
                    Ok(Reply::Boundary) => {
                        if let Some(link) = preparation.pending_link {
                            cx.finish_preparation(&child);
                            state.remember(child.clone(), link);
                        }
                        match preparation.activation {
                            Some(activation) if cx.view().active() != Some(&child) => {
                                if let Some(prompt) = preparation.prompt {
                                    state.parked.insert(child.clone(), prompt);
                                }
                                if activation.enter
                                    && cx.view().active() == Some(&activation.parent)
                                    && !cx.view().navigation_pending()
                                {
                                    cx.open_thread(&child, Ctx::Reuse { parent: activation.parent, prompt: None });
                                } else {
                                    cx.chat_facts(chat_facts(state, cx.view()));
                                }
                            }
                            _ => {
                                if let Some(prompt) = preparation.prompt {
                                    cx.submit(&child, UserMessage::text(prompt));
                                }
                            }
                        }
                    }
                    failed => {
                        cx.chat_facts(chat_facts(state, cx.view()));
                        let error = failed.err().unwrap_or(RpcError::Mismatch);
                        let prepare_error = failure(&error);
                        if let Some(link) = preparation.pending_link {
                            if link.storage == Storage::Attached {
                                cx.rollback_thread(&child, Ctx::Rollback {
                                    child: child.clone(), parent: link.parent, prepare_error,
                                });
                            } else {
                                cx.discard(&child);
                                cx.note(Some(link.parent), Note::PrepareFailed { thread: child, error: prepare_error });
                            }
                        } else {
                            cx.note(Some(child.clone()), Note::PrepareFailed { thread: child, error: prepare_error });
                        }
                    }
                }
            }

            fn note_tone(_note: &Note) -> Tone {
                Tone::Error
            }

            fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
                match command {
                    Command::Boundary { storage } => ClientRequest::ThreadInjectItems {
                        request_id: id,
                        params: ThreadInjectItemsParams {
                            thread_id: thread.to_string(),
                            items: policy::fork_items(*storage),
                        },
                    },
                    Command::Saved(work) => work.wire(thread, id),
                }
            }

            fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
                match command {
                    Command::Boundary { .. } => serde_json::from_value::<ThreadInjectItemsResponse>(result)
                        .map(|_| Reply::Boundary).map_err(|error| RpcError::Malformed(error.to_string())),
                    Command::Saved(work) => saved::decode(work, result).map(Reply::Saved),
                }
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
        assert_eq!(
            state.start(None, false),
            Start::Block(Note::MainUnavailable)
        );
        assert_eq!(state.start(Some(&id("main")), false), Start::Fork);
        state.remember(
            id("side"),
            Link {
                parent: id("main"),
                storage: Storage::Temporary,
                status: None,
            },
        );
        assert_eq!(
            state.start(Some(&id("main")), false),
            Start::Reuse(id("side"))
        );
        assert_eq!(
            state.start(Some(&id("side")), false),
            Start::Block(Note::AlreadyOpen)
        );
    }

    #[test]
    fn the_footer_follows_the_thread_shown() {
        let mut state = State::default();
        state.remember(
            id("side"),
            Link {
                parent: id("main"),
                storage: Storage::Temporary,
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
