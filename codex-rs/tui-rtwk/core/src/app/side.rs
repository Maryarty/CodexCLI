//! The app's half of side conversations (`/side`, `/btw`; oracle `app/side.rs`): the lifecycle the
//! slice (`features::side`) does not own (ADR-012, "half a slice"). The slice decides what a
//! `/side` does (`State::start`) and keeps the links between a parent and its side conversation;
//! this file forks the thread (`ThreadAsk::ForkAt { side: true }`, an ephemeral fork that starts
//! empty on the screen), shows the parent again and drops a side conversation the user returned
//! from (`discard_side_thread`: its turn is interrupted, the server stops holding it, the app
//! forgets it), keeps what the screen reads in step with the thread shown (the footer's facts and
//! the flag that gates the commands a side conversation lacks), and carries the parent's status
//! to its side conversation (`apply_side_parent_status_change`).

use codex_app_server_protocol::{ServerNotification, ServerRequest};

use super::{
    AppCtx, AppFx, CodexApp, Command, apply_thread, features, open_thread, session_note, submit,
};
use crate::features::side::{Intent, Start, StatusChange, ParentStatus, start_failure};
use crate::features::{FeatureIntent, FeatureNote};
use crate::ids::ThreadId;
use crate::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use crate::rpc::RpcError;
use crate::screens::chat::facts;
use crate::screens::{ScreenCommand, ScreenThreadIntent, agents, chat};
use crate::threads::{Note, NoteKey, ThreadAsk, ThreadEv, UserMessage};

/// What a side conversation that is being forked is given when it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Opening {
    pub parent: ThreadId,
    /// The `/btw` prompt, sent once the boundary is in.
    pub prompt: Option<String>,
}

fn say(app: &mut CodexApp, thread: Option<ThreadId>, note: crate::features::side::Note, fx: &mut AppFx) {
    let note = Note::from(NoteKey::Feature(FeatureNote::Side(note)));
    session_note(app, thread, note, fx);
}

/// A prompt that could not go (`restore_side_user_message`) goes back to the composer.
fn restore(app: &mut CodexApp, thread: &Option<ThreadId>, prompt: Option<String>, fx: &mut AppFx) {
    let (Some(thread), Some(prompt)) = (thread, prompt) else {
        return;
    };
    let draft = ScreenThreadIntent::Chat(chat::ThreadIntent::Draft(prompt));
    apply_thread(app, thread, ThreadEv::ScreenIntent(draft), fx);
}

/// `/side` and `/btw` (`handle_start_side`), typed in `source`.
pub(super) fn start(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    prompt: Option<String>,
    fx: &mut AppFx,
) {
    let prompt = prompt.filter(|prompt| !prompt.trim().is_empty());
    match app.features.side.start(source.as_ref()) {
        Start::Block(note) => {
            restore(app, &source, prompt, fx);
            say(app, source, note, fx);
        }
        Start::Reuse(child) => {
            open_thread(app, &child, fx);
            if app.session.active.as_ref() == Some(&child) {
                if let Some(prompt) = prompt {
                    submit(app, &child, UserMessage::text(prompt), fx);
                }
            } else {
                restore(app, &source, prompt, fx);
            }
        }
        Start::Fork => {
            let Some(parent) = source else {
                return;
            };
            if app
                .outgoing
                .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
            {
                restore(app, &Some(parent), prompt, fx);
                return;
            }
            let ticket = app.mint.next();
            let backend = app.screens.agents.owner_of(parent.as_str());
            let ask = ThreadAsk::ForkAt {
                thread: parent.to_string(),
                cwd: None,
                side: true,
            };
            app.outgoing.insert(
                ticket,
                AppCtx::NewConversation {
                    source: Some(parent.clone()),
                    backend,
                    then: None,
                    plan: None,
                    draft: None,
                    side: Some(Opening { parent, prompt }),
                },
            );
            fx.commands.push(Command::Screen {
                ticket,
                thread: ThreadId::default(),
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
            });
        }
    }
}

/// The fork is open here (`apply_side_creation`): it is a side conversation of its parent, and
/// its boundary goes in.
pub(super) fn linked(app: &mut CodexApp, child: &ThreadId, opening: Opening, fx: &mut AppFx) {
    let intent = Intent::Linked {
        child: child.clone(),
        parent: opening.parent,
        prompt: opening.prompt,
    };
    features::intent(app, FeatureIntent::Side(intent), fx);
}

/// The fork failed (`side_start_error_message`): the line, and the prompt back where it was typed.
pub(super) fn start_failed(app: &mut CodexApp, source: ThreadId, error: RpcError, fx: &mut AppFx) {
    let message = match error {
        RpcError::Remote(message) | RpcError::Malformed(message) | RpcError::Prepare(message) => {
            message
        }
        other => format!("{other:?}"),
    };
    say(app, Some(source), start_failure(&message), fx);
}

/// Returning from a side conversation drops it (`discard_side_thread_in_background`): done once
/// the parent is the thread shown, never while the side conversation still is.
pub(super) fn discard(app: &mut CodexApp, child: &ThreadId, fx: &mut AppFx) {
    if app.session.active.as_ref() == Some(child) || !app.features.side.is_side(child) {
        return;
    }
    if app.threads.get(child).is_some_and(|model| model.is_running()) {
        apply_thread(app, child, ThreadEv::Interrupt, fx);
    }
    // `thread/unsubscribe`: the answer is nobody's (the thread is gone from here).
    let ticket = app.mint.next();
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Unsubscribe {
            thread: child.to_string(),
        })),
    });
    app.threads.forget(child);
    let dropped = Intent::Dropped {
        child: child.clone(),
    };
    features::intent(app, FeatureIntent::Side(dropped), fx);
    sync(app, fx);
}

/// What the parent did (`apply_side_parent_status_change`).
pub(super) fn observe(app: &mut CodexApp, notification: &ServerNotification, fx: &mut AppFx) {
    let Some(change) = StatusChange::for_notification(notification) else {
        return;
    };
    let Some(thread) = crate::protocol::targets::notification_thread(notification) else {
        return;
    };
    parent_changed(app, ThreadId::new(thread), change, fx);
}

/// A request of the parent that waits for the user (`SideParentStatus::for_request`).
pub(super) fn observe_request(
    app: &mut CodexApp,
    request: &ServerRequest,
    thread: &str,
    fx: &mut AppFx,
) {
    if let Some(status) = ParentStatus::for_request(request) {
        parent_changed(app, ThreadId::new(thread), StatusChange::Set(status), fx);
    }
}

fn parent_changed(app: &mut CodexApp, parent: ThreadId, change: StatusChange, fx: &mut AppFx) {
    if app.features.side.side_of(&parent).is_none() {
        return;
    }
    let intent = Intent::Parent { parent, change };
    features::intent(app, FeatureIntent::Side(intent), fx);
    sync(app, fx);
}

/// The prompts whose boundary is in are sent (`/btw`).
pub(super) fn released(app: &mut CodexApp, fx: &mut AppFx) {
    let ready = std::rc::Rc::make_mut(&mut app.features)
        .side
        .take_ready();
    for (child, prompt) in ready {
        submit(app, &child, UserMessage::text(prompt), fx);
    }
}

/// What the screen reads of the thread shown (`sync_side_thread_ui`): whether it is a side
/// conversation (the commands it lacks are gated by it) and the footer's facts.
pub(super) fn sync(app: &mut CodexApp, fx: &mut AppFx) {
    let shown = app.session.active.clone();
    let side = &app.features.side;
    let active = shown.as_ref().is_some_and(|id| side.is_side(id));
    let footer = shown.as_ref().and_then(|id| {
        let parent = side.link(id).map(|link| &link.parent);
        let is_main = parent.is_none_or(|parent| {
            !app.threads
                .get(parent)
                .is_some_and(|model| model.screen_slots().chat.parent_owned)
        });
        let parent_label = parent
            .and_then(|parent| app.threads.get(parent))
            .map(|model| {
                model
                    .screen_slots()
                    .chat
                    .name
                    .clone()
                    .unwrap_or_else(|| model.view().id().to_string())
            })
            .unwrap_or_default();
        let switch = app.keymap.label(GLOBAL_TOGGLE_SIDE_CONVERSATION);
        side.footer(id, is_main, &parent_label, switch.as_deref())
    });
    if app.screens.chat.flags.side_conversation_active != active {
        std::rc::Rc::make_mut(&mut app.screens)
            .chat
            .flags
            .side_conversation_active = active;
    }
    if app.screens.chat.facts.side != footer {
        let chat = &mut std::rc::Rc::make_mut(&mut app.screens).chat;
        chat::apply(chat, chat::Ev::Facts(facts::Ev::Side(footer)), fx);
    }
}
