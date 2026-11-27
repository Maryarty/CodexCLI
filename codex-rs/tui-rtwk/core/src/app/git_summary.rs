//! Lifetime of the status line's workspace read. The workspace and its connection are captured
//! when the app issues the effect. The result has no authority over a different workspace.

use super::{AppEv, AppFx, BackendEv, CodexApp, Command, ConnectionState, ServerNotification};
use crate::app::effects::Ask;
use crate::ids::{BackendId, TurnId};
use crate::rpc::Address;
use crate::screens::chat::facts::{GitWorkspace, wants_git};
use crate::threads::TurnState;
use std::rc::Rc;

pub(super) fn observes(event: &AppEv) -> bool {
    matches!(
        event,
        AppEv::Reply(..)
            | AppEv::Intent(_)
            | AppEv::Screen(_)
            | AppEv::Effect { .. }
            | AppEv::Connected
            | AppEv::Disconnected
            | AppEv::StartFailed(_)
    ) || matches!(event, AppEv::Backend(BackendEv::Notification(note)) if matches!(&**note,
            ServerNotification::ThreadStarted(_) | ServerNotification::ThreadSettingsUpdated(_)
                | ServerNotification::TurnCompleted(_) | ServerNotification::Error(_)))
}

/// The displayed running turn that a terminal notification can finish. We check again after
/// apply, because some errors defer finalization to a subsequent terminal event.
pub(super) fn finishing_turn(app: &CodexApp, origin: BackendId, event: &AppEv) -> Option<TurnId> {
    let AppEv::Backend(BackendEv::Notification(note)) = event else {
        return None;
    };
    let (thread, turn) = match &**note {
        ServerNotification::TurnCompleted(note) => (&note.thread_id, &note.turn.id),
        ServerNotification::Error(note) if !note.will_retry => (&note.thread_id, &note.turn_id),
        _ => return None,
    };
    let id = app
        .session
        .active
        .as_ref()
        .filter(|id| id.as_str() == thread)?;
    if app.screens.agents.owner_of(id.as_str()) != origin {
        return None;
    }
    match app.threads.get(id)?.turn() {
        TurnState::Running(running) if running.turn.as_str() == turn => Some(running.turn.clone()),
        _ => None,
    }
}

pub(super) fn ended(app: &CodexApp, turn: &TurnId) -> bool {
    !app.session
        .active
        .as_ref()
        .and_then(|id| app.threads.get(id))
        .is_some_and(
            |model| matches!(model.turn(), TurnState::Running(running) if running.turn == *turn),
        )
}

fn workspace(app: &CodexApp) -> Option<GitWorkspace> {
    if !wants_git(&app.screens.chat.status) || app.screens.chat.facts.shutting_down {
        return None;
    }
    let thread = app.session.active.as_ref()?;
    let backend = app.screens.agents.owner_of(thread.as_str());
    if app.session.connections.get(&backend) != Some(&ConnectionState::Connected) {
        return None;
    }
    let cwd = app.threads.get(thread)?.screen_slots().chat.cwd.as_ref()?;
    (!cwd.is_empty()).then(|| GitWorkspace {
        thread: thread.clone(),
        address: app.address(backend),
        cwd: cwd.clone(),
    })
}

pub(super) fn sync(app: &mut CodexApp, refresh: bool, fx: &mut AppFx) {
    let workspace = workspace(app);
    if !app
        .screens
        .chat
        .facts
        .git_workspace_changed(workspace.as_ref(), refresh)
    {
        if refresh && workspace.is_some() {
            Rc::make_mut(&mut app.screens)
                .chat
                .facts
                .refresh_after_git();
        }
        return;
    }
    let request = workspace.as_ref().map_or(0, |_| app.mint.next().seq());
    Rc::make_mut(&mut app.screens)
        .chat
        .facts
        .begin_git(workspace.clone(), request);
    if let Some(GitWorkspace {
        thread,
        cwd,
        address,
    }) = workspace
    {
        fx.commands.push(Command::Effect {
            request,
            ask: Ask::GitSummary {
                thread,
                cwd,
                address,
            },
        });
    }
}

pub(super) fn retire(app: &mut CodexApp, backend: BackendId) {
    Rc::make_mut(&mut app.screens)
        .chat
        .facts
        .retire_git(backend);
}

pub(super) fn may_dispatch(
    app: &CodexApp,
    request: u64,
    thread: &crate::ids::ThreadId,
    cwd: &str,
    address: Address,
) -> bool {
    app.screens
        .chat
        .facts
        .pending_git(request)
        .is_some_and(|workspace| {
            workspace.address == address && workspace.thread == *thread && workspace.cwd == cwd
        })
}

pub(super) fn accepts(app: &CodexApp, address: Address, request: u64) -> bool {
    app.address(address.backend) == address
        && app
            .screens
            .chat
            .facts
            .pending_git(request)
            .is_some_and(|workspace| workspace.address == address)
}

pub(super) fn answer(
    app: &mut CodexApp,
    done: crate::screens::chat::facts::GitSummary,
    fx: &mut AppFx,
) {
    if Rc::make_mut(&mut app.screens).chat.facts.git_answered(done) {
        sync(app, true, fx);
    }
}
