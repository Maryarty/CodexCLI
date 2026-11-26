//! The app's half of the lifecycle doors a feature has (ADR-012 "lifecycle doors", ADR-003): the
//! app owns every `ThreadModel`, so a feature that wants a thread opened, made or dropped says so
//! (`AppCx::open_thread`, `spawn_thread`, `discard`, `submit`, `give_back`) and this file runs it through
//! the app's own steps (`open_thread`, the `thread/start` and `thread/fork` of `NewConversation`,
//! `apply_thread`, `submit`), as the oracle's `app/session_lifecycle.rs` and `thread_routing.rs`
//! do. The slice decides *what*; the lifecycle RPCs (`ThreadAsk`) stay in `threads.rs`.

use super::{
    AppCtx, AppFx, CodexApp, Command, apply_thread, features, is_listed, open_verified, submit,
};
use crate::features::{FeatureCtx, Lifecycle, Refusal, ThreadSpawn};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::Answer;
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::{self, Target};
use crate::screens::{ScreenCommand, ScreenReply, ScreenThreadIntent, agents, chat};
use crate::threads::{ThreadAsk, ThreadEv};

/// What a thread a feature asked for (`AppCx::spawn`) is told when it is open: the feature's own
/// context for `Feature::thread_opened`, and whether the thread is ephemeral (its history is the
/// model's, not the screen's, so none is read).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Spawned {
    pub ctx: FeatureCtx,
    pub ephemeral: bool,
}

/// One lifecycle step a feature asked for.
pub(super) fn carry(app: &mut CodexApp, lifecycle: Lifecycle, fx: &mut AppFx) {
    match lifecycle {
        Lifecycle::Open { thread, ctx } => open(app, thread, ctx, fx),
        Lifecycle::Spawn { spawn, ctx } => spawn_thread(app, spawn, ctx, fx),
        Lifecycle::Structured { turn, ctx } => super::structured::begin(app, *turn, ctx, fx),
        Lifecycle::Discard { thread } => discard(app, &thread, fx),
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
    fx.ask(app, AppCtx::Opening { thread, ctx }, |_, ticket| {
        Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        }
    });
}

/// The read of a thread `open` did not know came back: the server has it (it is opened) or not.
pub(super) fn read(
    app: &mut CodexApp,
    thread: ThreadId,
    ctx: FeatureCtx,
    answer: Answer,
    fx: &mut AppFx,
) {
    let result = match answer {
        Answer::Screen(ScreenReply::Agents(agents::Reply::Read { .. })) => {
            opened(open_verified(app, &thread, fx), thread)
        }
        Answer::Failed(error) => Err(Refusal::Failed(error)),
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
    };
    fx.ask(
        app,
        AppCtx::NewConversation {
            source,
            backend,
            name: None,
            plan: None,
            draft: None,
            spawned: Some(Spawned { ctx, ephemeral }),
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
    app.threads.forget(child);
}
