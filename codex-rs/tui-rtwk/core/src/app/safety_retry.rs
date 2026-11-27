//! The app owns a retry from explicit confirmation through admission on the exact fork.
//! The source keeps its input while the app owns the operation. Each step spends one addressed
//! ticket. The interruption also needs the source turn's terminal notification.

use codex_app_server_protocol::Thread;

use super::{AppCtx, AppFx, CodexApp, Command, Now, TurnOperation, Wake, apply_thread};
use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::{Address, Answer, RpcError};
use crate::screens::{ScreenCommand, ScreenThreadEv, agents, chat, select};
use crate::threads::safety::retry::{
    AwaitingInterrupt, History, InterruptSignal, Interruption, Invalid, Origin, ReadItems,
    ReadSource, ReadTurns, ReadyToFork,
};
use crate::threads::safety::{self, RetryLifecycle, RetryOffer, SafetyEv};
use crate::threads::{Failure, Item, ThreadAsk, ThreadEv, ThreadReply, TurnEnd, UserMessage};

const TIMEOUT_MILLIS: i64 = 30_000;

pub(super) struct Run {
    origin: Origin,
    phase: Phase,
    pending: Option<Ticket>,
    original: UserMessage,
    deadline: Option<Now>,
}

enum Phase {
    Interrupt(AwaitingInterrupt),
    Source(ReadSource),
    Turns(ReadTurns),
    Items(ReadItems),
    Fork(ReadyToFork),
    Start {
        child: Box<Thread>,
        message: UserMessage,
    },
}

impl Run {
    fn is_current(&self, app: &CodexApp) -> bool {
        let source = &self.origin;
        app.session.active.as_ref() == Some(&source.thread)
            && app.address(source.address.backend) == source.address
            && app.screens.agents.owner_of(source.thread.as_str()) == source.address.backend
            && app.threads.get(&source.thread).is_some_and(|model| {
                model.has_writer() && model.safety().holds_retry(source.generation)
            })
    }

    pub(super) fn accepts(&self, ticket: Ticket, generation: Ticket, answer: &Answer) -> bool {
        self.pending == Some(ticket)
            && self.origin.generation == generation
            && (matches!(answer, Answer::Failed(_))
                || matches!(
                    (&self.phase, answer),
                    (
                        Phase::Interrupt(_),
                        Answer::Thread(ThreadReply::RetryInterrupted)
                    ) | (Phase::Source(_), Answer::Thread(ThreadReply::Read { .. }))
                        | (Phase::Turns(_), Answer::Thread(ThreadReply::RetryTurns(_)))
                        | (Phase::Items(_), Answer::Thread(ThreadReply::RetryItems(_)))
                        | (Phase::Fork(_), Answer::Thread(ThreadReply::RetryForked(_)))
                        | (
                            Phase::Start { .. },
                            Answer::Thread(ThreadReply::RetryStarted { .. })
                        )
                ))
    }
}

pub(super) fn accepts(app: &CodexApp, ticket: Ticket, generation: Ticket, answer: &Answer) -> bool {
    app.safety_retry
        .as_ref()
        .is_some_and(|run| run.accepts(ticket, generation, answer))
}

fn lifecycle(app: &mut CodexApp, thread: &ThreadId, step: RetryLifecycle, fx: &mut AppFx) {
    apply_thread(
        app,
        thread,
        ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Safety(
            SafetyEv::RetryLifecycle(step),
        ))),
        fx,
    );
}

pub(super) fn begin(app: &mut CodexApp, thread: ThreadId, offer: RetryOffer, fx: &mut AppFx) {
    if app.safety_retry.is_some()
        || app.session.active.as_ref() != Some(&thread)
        || app.screens.chat.flags.side_conversation_active
    {
        return;
    }
    let backend = app.screens.agents.owner_of(thread.as_str());
    if app.screens.agents.is_cloud(backend) {
        return;
    }
    let Some(model) = app.threads.get(&thread) else {
        return;
    };
    let Some(approved) = safety::confirmed_retry(model, &offer) else {
        return;
    };
    if app.outgoing.any_on(backend, |ctx| {
        matches!(
            ctx,
            AppCtx::Select(select::Ctx::Settings {
                setting: select::Setting::Permissions,
                ..
            })
        )
    }) {
        failure(
            app,
            &thread,
            "Wait for permissions to update before forking.",
            fx,
        );
        return;
    }
    if model.screen_slots().chat.daybreak == Some(true)
        && (!app.daybreak.eligible
            || app.daybreak.models.iter().any(|entry| {
                entry.model == offer.model()
                    && entry
                        .programs
                        .is_some_and(|programs| programs.standard && !programs.daybreak)
            }))
    {
        failure(
            app,
            &thread,
            "Daybreak support for the offered model could not be confirmed.",
            fx,
        );
        return;
    }
    let address = app.address(backend);
    let generation = app.mint.next();
    let attempt = AwaitingInterrupt::new(approved, address, generation);
    let original = attempt.message().clone();
    let origin = attempt.origin.clone();
    lifecycle(app, &thread, RetryLifecycle::Hold { generation, offer }, fx);

    let ask = ThreadAsk::RetryInterrupt {
        thread,
        turn: origin.turn.clone(),
    };
    issue(
        app,
        Run {
            origin,
            phase: Phase::Interrupt(attempt),
            pending: None,
            original,
            deadline: None,
        },
        ask,
        fx,
    );
}

fn issue(app: &mut CodexApp, mut run: Run, ask: ThreadAsk, fx: &mut AppFx) {
    if !run.is_current(app) {
        release(app, run, fx);
        return;
    }
    let target = ask
        .thread()
        .map_or_else(|| run.origin.thread.clone(), ThreadId::new);
    let attaching = matches!(run.phase, Phase::Fork(_) | Phase::Start { .. });
    let ticket = fx.ask(
        app,
        AppCtx::TurnOperation(TurnOperation::SafetyRetry {
            generation: run.origin.generation,
            backend: run.origin.address.backend,
            attaching,
        }),
        |_, ticket| Command::Screen {
            ticket,
            thread: target,
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
    run.pending = Some(ticket);
    run.deadline = deadline(app);
    app.safety_retry = Some(run);
}

fn deadline(app: &CodexApp) -> Option<Now> {
    (app.now.millis() > 0).then(|| app.now.plus_millis(TIMEOUT_MILLIS))
}

fn with_signal(app: &mut CodexApp, mut run: Run, signal: InterruptSignal, fx: &mut AppFx) {
    let Phase::Interrupt(waiting) = run.phase else {
        app.safety_retry = Some(run);
        return;
    };
    match waiting.observe(signal) {
        Interruption::Waiting(waiting) => {
            run.phase = Phase::Interrupt(waiting);
            app.safety_retry = Some(run);
        }
        Interruption::Ready(read) => {
            let ask = ThreadAsk::Read {
                thread: read.origin().thread.to_string(),
            };
            run.phase = Phase::Source(read);
            issue(app, run, ask, fx);
        }
    }
}

pub(super) fn answered(
    app: &mut CodexApp,
    ticket: Ticket,
    generation: Ticket,
    address: Address,
    answer: Answer,
    fx: &mut AppFx,
) {
    if !accepts(app, ticket, generation, &answer) {
        return;
    }
    let Some(mut run) = app.safety_retry.take() else {
        return;
    };
    run.pending = None;
    if run.origin.address != address || !run.is_current(app) {
        release(app, run, fx);
        return;
    }
    let next = match (run.phase, answer) {
        (phase, Answer::Failed(error)) => {
            run.phase = phase;
            fail(app, run, error, fx);
            return;
        }
        (Phase::Interrupt(waiting), Answer::Thread(ThreadReply::RetryInterrupted)) => {
            run.phase = Phase::Interrupt(waiting);
            with_signal(app, run, InterruptSignal::Acknowledged, fx);
            return;
        }
        (Phase::Source(read), Answer::Thread(ThreadReply::Read { thread, .. })) => {
            read.verify(&thread).map(|turns| {
                let ask = ThreadAsk::RetryTurns(turns.request());
                (Phase::Turns(turns), ask)
            })
        }
        (Phase::Turns(turns), Answer::Thread(ThreadReply::RetryTurns(page))) => {
            turns.verify(*page).map(|history| match history {
                History::Complete(fork) => fork_request(fork),
                History::NeedsItems(items) => {
                    let ask = ThreadAsk::RetryItems(items.request());
                    (Phase::Items(items), ask)
                }
            })
        }
        (Phase::Items(items), Answer::Thread(ThreadReply::RetryItems(page))) => {
            items.verify(*page).map(fork_request)
        }
        (Phase::Fork(fork), Answer::Thread(ThreadReply::RetryForked(child))) => {
            if app.threads.contains(&ThreadId::new(&child.id)) {
                Err(Invalid::WrongFork)
            } else {
                fork.forked(child).map(|start| {
                    let client_id = app.mint.submission_id(run.origin.generation);
                    let (child, message, request) = start.start(client_id);
                    (
                        Phase::Start { child, message },
                        ThreadAsk::RetryStart(request),
                    )
                })
            }
        }
        (
            Phase::Start { child, message },
            Answer::Thread(ThreadReply::RetryStarted { thread, turn }),
        ) => {
            if child.id == thread.as_str() && !app.threads.contains(&thread) {
                let backend = run.origin.address.backend;
                super::attach_thread_with_metadata(
                    app,
                    &thread,
                    backend,
                    crate::threads::Origin::Started,
                    Some(&child),
                    fx,
                );
                lifecycle(
                    app,
                    &thread,
                    RetryLifecycle::Started {
                        turn,
                        message: Box::new(message),
                    },
                    fx,
                );
                super::show_held_thread(app, thread.clone(), fx);
                super::fetch_history(app, &thread, fx);
                release_parts(app, run.origin, None, fx);
            } else {
                fail_parts(
                    app,
                    run.origin,
                    run.original,
                    Invalid::WrongFork.message(),
                    fx,
                );
            }
            return;
        }
        _ => Err(Invalid::UnexpectedReceipt),
    };
    match next {
        Ok((phase, ask)) => {
            run.phase = phase;
            issue(app, run, ask, fx);
        }
        Err(error) => fail_parts(app, run.origin, run.original, error.message(), fx),
    }
}

fn fork_request(fork: ReadyToFork) -> (Phase, ThreadAsk) {
    let ask = ThreadAsk::RetryFork(fork.request());
    (Phase::Fork(fork), ask)
}

pub(super) fn observe(app: &mut CodexApp, thread: &ThreadId, event: &ThreadEv, fx: &mut AppFx) {
    let Some(run) = app
        .safety_retry
        .as_ref()
        .filter(|run| run.origin.thread == *thread)
    else {
        return;
    };
    let checked = &run.origin.turn;
    let cancelled = match event {
        ThreadEv::StreamLost | ThreadEv::ConnectionLost => true,
        ThreadEv::TurnStarted { turn, .. } => turn != checked,
        ThreadEv::TurnError { turn, .. } => turn == checked,
        ThreadEv::TurnEnded { turn, end } | ThreadEv::TurnCompleted { turn, end, .. } => {
            if turn != checked {
                return;
            }
            if *end == TurnEnd::Interrupted {
                if let Some(run) = app.safety_retry.take() {
                    with_signal(app, run, InterruptSignal::TurnInterrupted, fx);
                }
                return;
            }
            true
        }
        ThreadEv::AgentDelta { key, delta } => key.turn == *checked && !delta.is_empty(),
        ThreadEv::ItemStarted { key, item, .. } | ThreadEv::ItemUpsert { key, item, .. } => {
            key.turn == *checked && matches!(item, Item::Agent { text, .. } if !text.is_empty())
        }
        _ => false,
    };
    if cancelled {
        cancel(app, fx);
    }
}

pub(super) fn left(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    if app
        .safety_retry
        .as_ref()
        .is_some_and(|run| run.origin.thread == *thread)
    {
        cancel(app, fx);
    }
    lifecycle(app, thread, RetryLifecycle::RetireOffer, fx);
}

pub(super) fn lost(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    if app
        .safety_retry
        .as_ref()
        .is_some_and(|run| run.origin.address.backend == backend)
    {
        cancel(app, fx);
    }
    app.outgoing.retain(|ctx| !matches!(ctx, AppCtx::TurnOperation(TurnOperation::SafetyRetry { backend: owner, .. }) if *owner == backend));
}

pub(super) fn tick(app: &mut CodexApp, fx: &mut AppFx) {
    let due = deadline(app);
    if let Some(run) = app.safety_retry.as_mut() {
        let deadline = run.deadline.or(due);
        run.deadline = deadline;
        if deadline.is_some_and(|due| app.now.reached(due))
            && let Some(run) = app.safety_retry.take()
        {
            if let Some(ticket) = run.pending {
                app.outgoing.drop_ticket(ticket);
            }
            fail_parts(
                app,
                run.origin,
                run.original,
                Invalid::TimedOut.message(),
                fx,
            );
        }
    }
}

pub(super) fn wake(app: &CodexApp, other: Option<Wake>) -> Option<Wake> {
    let Some(due) = app.safety_retry.as_ref().and_then(|run| run.deadline) else {
        return other;
    };
    Some(Wake::new(
        app.now,
        other.map_or(due, |wake| due.min(wake.at())),
    ))
}

fn cancel(app: &mut CodexApp, fx: &mut AppFx) {
    if let Some(run) = app.safety_retry.take() {
        release(app, run, fx);
    }
}

fn release(app: &mut CodexApp, run: Run, fx: &mut AppFx) {
    if let Some(ticket) = run.pending {
        app.outgoing.drop_ticket(ticket);
    }
    release_parts(app, run.origin, None, fx);
}

fn release_parts(app: &mut CodexApp, origin: Origin, restore: Option<UserMessage>, fx: &mut AppFx) {
    lifecycle(
        app,
        &origin.thread,
        RetryLifecycle::Release {
            generation: origin.generation,
            restore: restore.map(Box::new),
        },
        fx,
    );
}

fn fail(app: &mut CodexApp, run: Run, error: RpcError, fx: &mut AppFx) {
    let Failure::Message(message) = Failure::from(error) else {
        fail_parts(
            app,
            run.origin,
            run.original,
            Invalid::UnexpectedReceipt.message(),
            fx,
        );
        return;
    };
    fail_parts(app, run.origin, run.original, &message, fx);
}

fn fail_parts(
    app: &mut CodexApp,
    origin: Origin,
    original: UserMessage,
    message: &str,
    fx: &mut AppFx,
) {
    failure(app, &origin.thread, message, fx);
    release_parts(app, origin, Some(original), fx);
}

fn failure(app: &mut CodexApp, thread: &ThreadId, message: &str, fx: &mut AppFx) {
    apply_thread(
        app,
        thread,
        ThreadEv::Failure(Failure::Message(message.to_owned())),
        fx,
    );
}
