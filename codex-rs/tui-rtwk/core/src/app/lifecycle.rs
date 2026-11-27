//! The app's half of the lifecycle doors a feature has (ADR-012 "lifecycle doors", ADR-003): the
//! app owns every `ThreadModel`, so a feature that wants a thread opened, made or dropped says so
//! (`AppCx::open_thread`, `spawn_thread`, `discard`, `submit`, `give_back`) and this file runs it through
//! the app's own steps (`open_thread`, the `thread/start` and `thread/fork` of `NewConversation`,
//! `apply_thread`, `submit`), as the oracle's `app/session_lifecycle.rs` and `thread_routing.rs`
//! do. The slice decides *what*; the lifecycle RPCs (`ThreadAsk`) stay in `threads.rs`.

use codex_app_server_protocol as wire;

use super::{
    AppCtx, AppFx, CodexApp, Command, PreparationCheckpoint, apply_thread, features, is_listed,
    open_verified, submit,
};
use crate::features::{FeatureCtx, Lifecycle, Refusal, ThreadSpawn};
use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::{Address, Answer, RpcError};
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::{self, Target};
use crate::screens::{ScreenCommand, ScreenThreadIntent, agents, chat};
use crate::threads::{ThreadAsk, ThreadEv, ThreadReply};

impl super::AttachedHistory {
    /// The wire request described by an attachment operation.
    pub(crate) fn wire(&self, parent: &ThreadId, id: wire::RequestId) -> wire::ClientRequest {
        match self {
            Self::Discover => ThreadAsk::ListAttachedSide {
                parent: parent.clone(),
                cursor: None,
            },
            Self::Verify { child } => ThreadAsk::Read {
                thread: child.to_string(),
            },
            Self::Archive { child } => ThreadAsk::Archive {
                thread: child.clone(),
            },
        }
        .wire(id)
    }
}

/// What a thread a feature asked for (`AppCx::spawn`) is told when it is open: the feature's own
/// context for `Feature::thread_opened`, and whether the thread is ephemeral (its history is the
/// model's, not the screen's, so none is read).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Spawned {
    Plain { ctx: FeatureCtx, ephemeral: bool },
    PreparedFork(SideFork),
    Rollback { thread: ThreadId, ctx: FeatureCtx },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SidePhase {
    Probe,
    Parent,
    Attached,
    Temporary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SideFork {
    parent: ThreadId,
    pub backend: BackendId,
    pub phase: SidePhase,
    ctx: FeatureCtx,
    temporary_policy: String,
    attached_policy: String,
}

#[derive(Clone)]
pub(super) struct Preparation {
    pub address: Address,
    pub checkpoint: Option<PreparationCheckpoint>,
    pub retired: bool,
}

/// One lifecycle step a feature asked for.
pub(super) fn carry(app: &mut CodexApp, lifecycle: Lifecycle, fx: &mut AppFx) {
    match lifecycle {
        Lifecycle::Open { thread, ctx } => open(app, thread, ctx, fx),
        Lifecycle::RestoreAttachment { parent, child, ctx } => {
            restore_attachment(app, parent, child, ctx, fx)
        }
        Lifecycle::Spawn { spawn, ctx } => spawn_thread(app, spawn, ctx, fx),
        Lifecycle::Structured { turn, ctx } => super::structured::begin(app, *turn, ctx, fx),
        Lifecycle::Discard { thread, rollback } => {
            if let Some(ctx) = rollback {
                remove_unprepared(app, thread, ctx, fx);
            } else {
                discard(app, &thread, fx);
            }
        }
        Lifecycle::FinishPreparation { thread } => {
            if app
                .preparing_forks
                .get(&thread)
                .is_some_and(|preparation| !preparation.retired)
            {
                app.preparing_forks.remove(&thread);
            }
        }
        Lifecycle::Submit { thread, message } => submit(app, &thread, message, fx),
        Lifecycle::GiveBack { thread, text } => {
            let draft = ScreenThreadIntent::Chat(chat::ThreadIntent::Draft(text));
            apply_thread(app, &thread, ThreadEv::ScreenIntent(draft), fx);
        }
        Lifecycle::QuietInterrupts { thread, quiet } => {
            apply_thread(app, &thread, ThreadEv::QuietInterrupts(quiet), fx);
        }
    }
}

/// `AppCx::open_thread`: a thread this process holds or lists is shown (resumed when it is not
/// held); one it does not know is read first, so a session the server has not got is told to the
/// feature instead of being opened.
fn open(app: &mut CodexApp, thread: ThreadId, ctx: FeatureCtx, fx: &mut AppFx) {
    if is_listed(app, &thread) {
        let result = opened(open_verified(app, &thread, fx), thread);
        return features::thread_opened(app, ctx, result, fx);
    }
    let ask = ThreadAsk::Read {
        thread: thread.to_string(),
    };
    fx.ask(
        app,
        AppCtx::Opening {
            thread,
            ctx,
            attached_to: None,
        },
        |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
}

fn restore_attachment(
    app: &mut CodexApp,
    parent: ThreadId,
    child: ThreadId,
    ctx: FeatureCtx,
    fx: &mut AppFx,
) {
    let backend = app.screens.agents.owner_of(parent.as_str());
    if app.local_side_storage(backend).is_none() || child == parent {
        return features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
    }
    let ask = ThreadAsk::Resume {
        thread: child.to_string(),
    };
    fx.ask(
        app,
        AppCtx::Opening {
            thread: child,
            ctx,
            attached_to: Some(parent),
        },
        |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
}

/// The read of a thread `open` did not know came back: the server has it (it is opened) or not.
pub(super) fn read(
    app: &mut CodexApp,
    thread: ThreadId,
    ctx: FeatureCtx,
    attached_to: Option<ThreadId>,
    request: (Address, Ticket),
    answer: Answer,
    fx: &mut AppFx,
) {
    let result = match (attached_to, answer) {
        (None, Answer::Thread(ThreadReply::Read { .. })) => {
            opened(open_verified(app, &thread, fx), thread)
        }
        (Some(parent), Answer::Thread(ThreadReply::Resumed(opened)))
            if !opened.thread.ephemeral
                && opened.thread.id == thread.as_str()
                && opened.thread.attached_to_thread_id.as_deref() == Some(parent.as_str())
                && opened.thread.history_mode == wire::ThreadHistoryMode::Paginated =>
        {
            let (address, ticket) = request;
            app.preparing_forks.remove(&thread);
            super::attach_thread_with_metadata(
                app,
                &thread,
                address.backend,
                super::Origin::Resumed,
                Some(&opened.thread),
                fx,
            );
            if let Some(snapshot) = opened.permissions {
                super::select::record_permission_settings(
                    app,
                    &thread,
                    crate::screens::select::permissions::Effective {
                        address,
                        at: ticket,
                        snapshot,
                    },
                    fx,
                );
            }
            apply_thread(
                app,
                &thread,
                crate::threads::history_page_of(&opened.thread),
                fx,
            );
            super::fetch_history(app, &thread, fx);
            features::resumed(app, &thread, fx);
            Ok(thread)
        }
        (_, Answer::Failed(error)) => Err(Refusal::Failed(error)),
        _ => Err(Refusal::Failed(crate::rpc::RpcError::Mismatch)),
    };
    features::thread_opened(app, ctx, result, fx);
}

fn opened(accepted: bool, thread: ThreadId) -> Result<ThreadId, Refusal> {
    if accepted {
        Ok(thread)
    } else {
        Err(Refusal::Busy)
    }
}

/// `AppCx::spawn`: one conversation is made at a time (`/new`, `/fork`, `/side` share the
/// answer), so a second ask is refused to the feature that made it.
fn spawn_thread(app: &mut CodexApp, spawn: ThreadSpawn, ctx: FeatureCtx, fx: &mut AppFx) {
    if spawn == ThreadSpawn::Start
        && app.session.active.is_none()
        && app.screens.agents.displayed == BackendId::default()
    {
        if consent::pending(&app.screens.onboarding) {
            return features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
        }
        return super::consent::begin(
            app,
            super::consent::Action::Fresh { feature: Some(ctx) },
            Target::Folder(None),
            TrustCancel::AgentsOverview,
            fx,
        );
    }
    start_spawn(app, spawn, ctx, fx);
}

pub(super) fn start_after_consent(app: &mut CodexApp, ctx: FeatureCtx, fx: &mut AppFx) {
    start_spawn(app, ThreadSpawn::Start, ctx, fx);
}

fn start_spawn(app: &mut CodexApp, spawn: ThreadSpawn, ctx: FeatureCtx, fx: &mut AppFx) {
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
    {
        return features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
    }
    let (source, backend, ephemeral, fork) = match spawn {
        ThreadSpawn::Start => {
            let source = app.session.active.clone();
            let backend = source.as_ref().map_or(app.screens.agents.displayed, |id| {
                app.screens.agents.owner_of(id.as_str())
            });
            (source, backend, false, None)
        }
        ThreadSpawn::Fork { parent, ephemeral } => {
            let backend = app.screens.agents.owner_of(parent.as_str());
            let quiet = ephemeral.is_some();
            let ask = match ephemeral {
                Some(instructions) => ThreadAsk::ForkAt {
                    thread: parent.to_string(),
                    cwd: None,
                    temporary: Some(instructions),
                },
                None => ThreadAsk::Fork {
                    thread: parent.to_string(),
                },
            };
            (Some(parent), backend, quiet, Some(ask))
        }
        ThreadSpawn::SideFork {
            parent,
            temporary_policy,
            attached_policy,
        } => {
            let backend = app.screens.agents.owner_of(parent.as_str());
            let phase = match app.local_side_storage(backend) {
                Some(super::LocalSideStorage::Embedded) => SidePhase::Parent,
                Some(super::LocalSideStorage::Daemon) => SidePhase::Probe,
                None => SidePhase::Temporary,
            };
            return ask_side(
                app,
                SideFork {
                    parent,
                    backend,
                    phase,
                    ctx,
                    temporary_policy,
                    attached_policy,
                },
                fx,
            );
        }
    };
    fx.ask(
        app,
        AppCtx::NewConversation {
            source,
            backend,
            name: None,
            plan: None,
            draft: None,
            spawned: Some(Spawned::Plain { ctx, ephemeral }),
        },
        |app, ticket| match fork {
            Some(ask) => Command::Screen {
                ticket,
                thread: ThreadId::default(),
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
            },
            None => super::start_thread(app, ticket, backend),
        },
    );
}

fn ask_side(app: &mut CodexApp, side: SideFork, fx: &mut AppFx) {
    let ask = match side.phase {
        SidePhase::Probe => ThreadAsk::ProbeAttachedSide {
            thread: side.parent.clone(),
        },
        SidePhase::Parent => ThreadAsk::Read {
            thread: side.parent.to_string(),
        },
        SidePhase::Attached => ThreadAsk::ForkAttachedSide {
            thread: side.parent.clone(),
            instructions: side.attached_policy.clone(),
        },
        SidePhase::Temporary => ThreadAsk::ForkAt {
            thread: side.parent.to_string(),
            cwd: None,
            temporary: Some(side.temporary_policy.clone()),
        },
    };
    fx.ask(
        app,
        AppCtx::NewConversation {
            source: Some(side.parent.clone()),
            backend: side.backend,
            name: None,
            plan: None,
            draft: None,
            spawned: Some(Spawned::PreparedFork(side)),
        },
        |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
}

pub(super) fn side_reply(
    app: &mut CodexApp,
    mut side: SideFork,
    address: Address,
    ticket: Ticket,
    answer: Answer,
    fx: &mut AppFx,
) {
    let result = match (side.phase, answer) {
        (SidePhase::Probe, Answer::Thread(ThreadReply::AttachedSideSupported(supported))) => {
            side.phase = if supported {
                SidePhase::Parent
            } else {
                SidePhase::Temporary
            };
            return ask_side(app, side, fx);
        }
        (SidePhase::Parent, Answer::Thread(ThreadReply::Read { thread, .. }))
            if thread.id == side.parent.as_str() =>
        {
            side.phase =
                if !thread.ephemeral && thread.history_mode == wire::ThreadHistoryMode::Paginated {
                    SidePhase::Attached
                } else {
                    SidePhase::Temporary
                };
            return ask_side(app, side, fx);
        }
        (SidePhase::Attached, Answer::Thread(ThreadReply::AttachedSideStoreUnavailable)) => {
            side.phase = SidePhase::Temporary;
            return ask_side(app, side, fx);
        }
        (
            SidePhase::Attached | SidePhase::Temporary,
            Answer::Thread(ThreadReply::Forked(opened)),
        ) => {
            use super::Origin;
            let thread = &opened.thread;
            let matches_storage = match side.phase {
                SidePhase::Attached => {
                    !thread.ephemeral
                        && thread.history_mode == wire::ThreadHistoryMode::Paginated
                        && thread.attached_to_thread_id.as_deref() == Some(side.parent.as_str())
                }
                SidePhase::Temporary => thread.ephemeral && thread.attached_to_thread_id.is_none(),
                _ => false,
            };
            let child = ThreadId::new(&thread.id);
            if child == side.parent || app.threads.contains(&child) || !matches_storage {
                Err(RpcError::Mismatch)
            } else {
                app.preparing_forks.insert(
                    child.clone(),
                    Preparation {
                        address,
                        checkpoint: (side.phase == SidePhase::Attached).then(|| {
                            PreparationCheckpoint {
                                parent: side.parent.clone(),
                                rollout: thread
                                    .path
                                    .as_ref()
                                    .map(|path| path.as_path().to_path_buf()),
                            }
                        }),
                        retired: false,
                    },
                );
                super::attach_thread_with_metadata(
                    app,
                    &child,
                    side.backend,
                    Origin::Started,
                    Some(thread),
                    fx,
                );
                if let Some(snapshot) = opened.permissions {
                    super::select::record_permission_settings(
                        app,
                        &child,
                        crate::screens::select::permissions::Effective {
                            address,
                            at: ticket,
                            snapshot,
                        },
                        fx,
                    );
                }
                Ok(child)
            }
        }
        (_, Answer::Failed(error)) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    features::thread_opened(app, side.ctx, result.map_err(Refusal::Failed), fx);
}

fn remove_unprepared(app: &mut CodexApp, thread: ThreadId, ctx: FeatureCtx, fx: &mut AppFx) {
    let backend = app.screens.agents.owner_of(thread.as_str());
    if app.session.active.as_ref() == Some(&thread)
        || !app.preparing_forks.get(&thread).is_some_and(|preparation| {
            !preparation.retired
                && preparation.checkpoint.is_some()
                && preparation.address == app.address(backend)
        })
    {
        return features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
    }
    let ask = ThreadAsk::Delete {
        thread: thread.clone(),
    };
    fx.ask(
        app,
        AppCtx::NewConversation {
            source: None,
            backend,
            name: None,
            plan: None,
            draft: None,
            spawned: Some(Spawned::Rollback { thread, ctx }),
        },
        |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
}

pub(super) fn rolled_back(
    app: &mut CodexApp,
    thread: ThreadId,
    ctx: FeatureCtx,
    answer: Answer,
    fx: &mut AppFx,
) {
    let result = match answer {
        Answer::Thread(ThreadReply::Done) => {
            super::forget_thread(app, &thread, fx);
            Ok(thread)
        }
        Answer::Failed(error) => Err(Refusal::Failed(error)),
        _ => Err(Refusal::Failed(RpcError::Mismatch)),
    };
    features::thread_opened(app, ctx, result, fx);
}

/// `AppCx::discard` (`discard_side_thread_in_background`): the thread's turn is interrupted, the
/// server stops holding it and the app forgets it. Never the thread in use.
fn discard(app: &mut CodexApp, child: &ThreadId, fx: &mut AppFx) {
    if app.session.active.as_ref() == Some(child) {
        return;
    }
    let backend = app.screens.agents.owner_of(child.as_str());
    if app
        .threads
        .get(child)
        .is_some_and(|model| model.is_running())
    {
        let start = fx.commands.len();
        apply_thread(app, child, ThreadEv::Interrupt, fx);
        // This thread is about to be forgotten. The interrupt still goes out; its answer
        // no longer has a model to settle.
        for command in &fx.commands[start..] {
            if let Command::InterruptTurn { ticket, .. } = command
                && let Some(address) = app.request_address(*ticket)
            {
                fx.tells.insert(*ticket, address);
            }
        }
    }
    // `thread/unsubscribe`: the answer is nobody's (the thread is gone from here).
    fx.tell(app, backend, |ticket| Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Unsubscribe {
            thread: child.to_string(),
        })),
    });
    super::forget_thread(app, child, fx);
}
