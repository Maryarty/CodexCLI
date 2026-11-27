//! The app's half of the feature slices (ADR-012): the app as a read-only view for a
//! feature's step, and the outbox carried out after it. A feature never holds the app; it is
//! handed `&mut` of its own state and an `AppCx`, says what it wants, and this file does it,
//! in order, through the app's own doors (`session_note`, `apply_thread`, the wishes).

use std::rc::Rc;

use super::effects::{self, Done};
use super::{AppCtx, AppFx, CodexApp, Command, Now, Wake, apply_thread, session_note};
use crate::features::{
    self, AppCx, AppView, FeatureCtx, FeatureFact, FeatureIntent, FeatureThreadIntent,
    InterestStep, Scope,
};
use crate::features::{ChatFacts, Connection, FeatureStates, Out, Refusal};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::{Address, Answer, RpcError};
use crate::screens::chat::{facts, plan_mode::PromptPriority};
use crate::screens::status::RateRead;
use crate::screens::{ScreenThreadIntent, chat};
use crate::threads::queue::{self, ModalOperation, QueueEvent, QueueOperation};
use crate::threads::{Note, NoteKey, ThreadEv};
use codex_app_server_protocol::{ErrorNotification, ServerNotification, ServerRequest};

/// The app as a read-only view, for the code of the app that asks what a feature would read.
pub(super) fn view(app: &CodexApp) -> AppView<'_> {
    AppView::new(
        app.session.active.as_ref(),
        &app.threads,
        &app.screens,
        &app.tui,
        &app.modes,
        app.transcript_text.as_ref(),
    )
    .with_access(&app.daybreak)
    .with_keys(&app.keymap)
    .with_banner(shown_banner(app))
    .with_rates(shown_rates(app))
    .with_navigation_pending(navigation_pending(app))
    .at(app.now)
}

/// App-owned openings can still be in flight when their source conversation is on display.
pub(super) fn navigation_pending(app: &CodexApp) -> bool {
    app.outgoing.any(|ctx| {
        matches!(
            ctx,
            AppCtx::Opening { .. }
                | AppCtx::OpenThread {
                    cancelled: false,
                    ..
                }
        ) || matches!(ctx, AppCtx::NewConversation { source, .. } if source == &app.session.active)
    })
}

/// The backend banner occurrence of the account the thread in use belongs to.
fn shown_banner(app: &CodexApp) -> Option<&crate::screens::chat::BannerOccurrence> {
    app.session
        .backend_banners
        .get(&super::session::active_backend(app))
}

/// The limits the account the thread in use belongs to has now.
fn shown_rates(app: &CodexApp) -> Option<&crate::screens::status::RateLimits> {
    app.session
        .rate_limits
        .get(&super::session::active_backend(app))
}

/// Runs one step of the features with the app as a view, then carries out what it said.
pub(super) fn run(
    app: &mut CodexApp,
    fx: &mut AppFx,
    step: impl FnOnce(&mut FeatureStates, &mut AppCx<'_>),
) {
    let backend = super::session::active_backend(app);
    let navigating = navigation_pending(app);
    let out = {
        let view = AppView::new(
            app.session.active.as_ref(),
            &app.threads,
            &app.screens,
            &app.tui,
            &app.modes,
            app.transcript_text.as_ref(),
        )
        .with_access(&app.daybreak)
        .with_keys(&app.keymap)
        .with_banner(app.session.backend_banners.get(&backend))
        .with_rates(app.session.rate_limits.get(&backend))
        .with_navigation_pending(navigating)
        .at(app.now);
        let mut cx = AppCx::new(view);
        step(Rc::make_mut(&mut app.features), &mut cx);
        cx.into_out()
    };
    for out in out {
        carry(app, out, fx);
    }
}

/// The shell told the time (`AppEv::Clock`): the app keeps it, the screens that word a time
/// read it, and what asked to be woken at a moment the clock has now reached is woken, with its
/// own intent through the door every other intent takes. A wake is delivered once per clock
/// event: an intent that leaves its `wake_at` standing is asked for again a second later
/// (`Wake::MIN_WAIT_MILLIS`), never in a loop.
pub(super) fn clock(app: &mut CodexApp, now: Now, fx: &mut AppFx) {
    app.now = now;
    Rc::make_mut(&mut app.screens).set_now(now);
    super::structured::expire(app, now, fx);
    for due in features::wakes_due(&app.features, now) {
        intent(app, due, fx);
    }
    let woken: Vec<_> = app
        .threads
        .iter()
        .map(|(id, model)| (id.clone(), model.wakes_due(now)))
        .filter(|(_, due)| !due.is_empty())
        .collect();
    for (thread, due) in woken {
        for due in due {
            apply_thread(app, &thread, ThreadEv::FeatureIntent(due), fx);
        }
    }
}

/// The nearest moment anything of the app wants the clock told: a feature's `wake_at`, a thread's
/// features' `thread_wake_at`, a screen that words ages (`agents::wake_at`) or reads on a timer
/// (`chat::wake_at`).
pub(super) fn wake(app: &CodexApp) -> Option<Wake> {
    let nearest = |next: Option<Now>, at: Option<Now>| match (next, at) {
        (Some(next), Some(at)) => Some(next.min(at)),
        (next, at) => next.or(at),
    };
    let mut next = features::next_wake(&app.features);
    for (_, model) in app.threads.iter() {
        next = nearest(next, model.next_wake());
    }
    next = nearest(
        next,
        crate::screens::agents::wake_at(&app.screens.agents, app.now),
    );
    next = nearest(next, crate::screens::chat::wake_at(&app.screens.chat));
    // the footer's goal clock: the thread in use is told the time when its elapsed label changes
    next = nearest(
        next,
        app.session
            .active
            .as_ref()
            .and_then(|id| app.threads.get(id))
            .and_then(|model| {
                crate::screens::chat::facts::goal_label_change_in(model, app.now.secs())
            })
            .map(|secs| {
                Now::at_secs(app.now.secs().saturating_add(secs)).in_zone(app.now.utc_offset_secs())
            }),
    );
    next = nearest(next, app.structured.wake_at());
    // the cloud switch's refusal is shown for one apply: an idle app is woken to take it away
    if app.session.cloud_switch == Some(crate::app::CloudSwitch::Failed) {
        next = nearest(next, Some(app.now));
    }
    next.map(|at| Wake::new(app.now, at))
}

/// One thing a feature asked for.
fn carry(app: &mut CodexApp, out: Out, fx: &mut AppFx) {
    match out {
        Out::Ask {
            thread,
            command,
            ctx,
        } => {
            fx.ask(app, AppCtx::Feature(ctx), |_, ticket| Command::Feature {
                ticket,
                thread,
                command,
            });
        }
        Out::Effect { ask, ctx } => {
            let workspace = ask
                .workspace_receipt()
                .zip(ask.thread())
                .map(|(receipt, thread)| effects::WorkspaceEffect {
                    thread: thread.clone(),
                    address: app.address(app.screens.agents.owner_of(thread.as_str())),
                    receipt,
                });
            let request = app.effects.ask_for_feature(ctx, workspace);
            fx.commands.push(effects::command(request, ask));
        }
        Out::Note { thread, note } => {
            let thread = thread.or_else(|| app.session.active.clone());
            session_note(app, thread, Note::from(NoteKey::Feature(note)), fx);
        }
        Out::Thread { thread, intent } => {
            apply_thread(app, &thread, ThreadEv::FeatureIntent(intent), fx);
        }
        Out::Wish {
            scope,
            wish,
            queue_modal,
        } => {
            let queue_operation = if let Some(modal) = queue_modal {
                let Scope::Thread(thread) = &scope else {
                    return;
                };
                let active = app.session.active.as_ref() == Some(thread);
                let operation = match modal.operation {
                    Some(operation) => {
                        let issued = app
                            .threads
                            .get(thread)
                            .and_then(|model| model.due_command())
                            .is_some_and(|due| due.operation() == &operation);
                        if !issued {
                            return;
                        }
                        operation
                    }
                    None if active && app.threads.contains(thread) => {
                        QueueOperation::modal(thread.clone(), app.mint.next().seq())
                    }
                    None => return,
                };
                let operation = ModalOperation::new(modal.kind, operation);
                let address = modal_address(app, thread);
                queue_event(
                    app,
                    thread,
                    QueueEvent::opening_modal(operation.clone(), address),
                    fx,
                );
                if !active {
                    queue_event(
                        app,
                        thread,
                        QueueEvent::ModalFinished(operation.cancelled()),
                        fx,
                    );
                    return;
                }
                Some(operation)
            } else {
                None
            };
            Rc::make_mut(&mut app.wishes).push(scope, wish, queue_operation);
        }
        Out::Lifecycle(lifecycle) => super::lifecycle::carry(app, lifecycle, fx),
        Out::Chat(facts) => chat_facts(app, facts, fx),
        Out::Verb(verb) => super::verbs::carry(app, verb, fx),
        Out::End => fx.ends = true,
        Out::Settled { thread } => {
            let intent = FeatureThreadIntent::Settled;
            apply_thread(app, &thread, ThreadEv::FeatureIntent(intent), fx);
            // The queue is still the thread's own (`threads.rs`), no feature's `on_settled`:
            // what waited for the settled thing goes on (`maybe_send_next_queued_input`). It
            // runs after the features' hooks, as its own event, so it sees a turn they started.
            let resume =
                ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::QueueResume));
            apply_thread(app, &thread, resume, fx);
        }
    }
}

fn queue_event(app: &mut CodexApp, thread: &ThreadId, event: QueueEvent, fx: &mut AppFx) {
    apply_thread(
        app,
        thread,
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(
            event,
        ))),
        fx,
    );
}

fn modal_address(app: &CodexApp, thread: &ThreadId) -> Address {
    app.address(app.screens.agents.owner_of(thread.as_str()))
}

pub(super) fn retire_queue_modal_choices(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    if app
        .threads
        .get(thread)
        .is_some_and(|model| queue::has_live_modal(model))
    {
        queue_event(app, thread, QueueEvent::modal_owner_left(), fx);
    }
}

/// Apply a native dialog result before releasing its original queue. A late result may
/// release its own hold, but cannot select on behalf of another conversation.
pub(super) fn finish_queue_modal(
    app: &mut CodexApp,
    thread: &ThreadId,
    message: ScreenThreadIntent,
    fx: &mut AppFx,
) -> Result<(), ScreenThreadIntent> {
    let ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(QueueEvent::ModalFinished(
        mut receipt,
    ))) = message
    else {
        return Err(message);
    };
    if receipt.thread() != thread {
        return Ok(());
    }
    if app.session.active.as_ref() == Some(thread)
        && app
            .threads
            .get(thread)
            .is_some_and(|model| queue::modal_is_open(model, &receipt, modal_address(app, thread)))
    {
        run(app, fx, |states, cx| {
            features::apply_modal_choice(states, thread, &mut receipt, cx);
        });
    }
    queue_event(app, thread, QueueEvent::ModalFinished(receipt), fx);
    Ok(())
}

/// An intent of a feature.
pub(super) fn intent(app: &mut CodexApp, intent: FeatureIntent, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::apply_intent(states, intent, cx)
    });
}

/// A slash command a feature runs (`Feature::RUNS`): false when no feature owns it.
pub(super) fn slash(app: &mut CodexApp, slash: features::Slash, fx: &mut AppFx) -> bool {
    let mut owned = false;
    run(app, fx, |states, cx| {
        owned = features::apply_slash(states, slash, cx);
    });
    owned
}

/// The answer of a feature's app-scoped RPC.
pub(super) fn answer(app: &mut CodexApp, ctx: FeatureCtx, answer: Answer, fx: &mut AppFx) {
    let result = match answer {
        Answer::Feature(reply) => Ok(reply),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    run(app, fx, |states, cx| {
        features::answered(states, ctx, result, cx)
    });
}

/// The host's answer to an ask a feature made (`AppCx::effect`), under the request number the
/// app gave it: it reaches the feature that asked and is kept for no screen. A number no
/// feature waits for any more (its last answer came) is dropped.
pub(super) fn effect_done(app: &mut CodexApp, request: u64, done: Done, fx: &mut AppFx) {
    let Some(ctx) = app.effects.feature_answered(request, &done) else {
        return;
    };
    run(app, fx, |states, cx| {
        features::effect_done(states, ctx, done, cx)
    });
}

/// A surface wants or lets go of what a feature shows (ADR-013 И): the first to want it and the
/// last to let go reach the feature's `on_interest`.
pub(super) fn interest(app: &mut CodexApp, step: InterestStep, fx: &mut AppFx) {
    let (interest, open) = match step {
        InterestStep::Open(interest) => {
            if !Rc::make_mut(&mut app.interests).raise(&interest) {
                return;
            }
            (interest, true)
        }
        InterestStep::Close(interest) => {
            if !Rc::make_mut(&mut app.interests).lower(&interest) {
                return;
            }
            (interest, false)
        }
    };
    run(app, fx, |states, cx| {
        features::on_interest(states, &interest, open, cx)
    });
}

/// The connection is new: ask again for the interests it serves.
pub(super) fn reconnected(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    use features::FeatureInterest;

    for interest in app.interests.all() {
        let agents = &app.screens.agents;
        let owner = match &interest {
            FeatureInterest::Providers(_) => BackendId::default(),
            FeatureInterest::Rename(interest) => agents.owner_of(interest.thread().as_str()),
            FeatureInterest::Images(interest) => agents.owner_of(interest.source.thread.as_str()),
            _ => agents.displayed,
        };
        if owner != backend {
            continue;
        }
        run(app, fx, |states, cx| {
            features::on_interest(states, &interest, true, cx)
        });
    }
}

/// The displayed conversation's consent and presence do not carry to a new visit.
pub(super) fn retire_ide(app: &mut CodexApp) {
    Rc::make_mut(&mut app.features).small_cmds.ide.retire();
}

/// Workspace replies depend on the connection's locality. Forget the old handle's requests
/// and snapshots, retaining the mounted images' interests for the next handle.
pub(super) fn retire_workspace_effects(app: &mut CodexApp, backend: BackendId) {
    app.effects.retire_workspace_effects(backend);
    if backend == BackendId::default() {
        retire_ide(app);
    }
    let agents = &app.screens.agents;
    features::retire_image_snapshots(&mut app.features, |thread| {
        agents.owner_of(thread.as_str()) == backend
    });
}

/// The replacement already has its new request address, even when it did not emit Up/Lost.
pub(super) fn reconnect_images(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    for interest in app.interests.all() {
        if let features::FeatureInterest::Images(read) = &interest
            && app.screens.agents.owner_of(read.source.thread.as_str()) == backend
        {
            run(app, fx, |states, cx| {
                features::on_interest(states, &interest, true, cx)
            });
        }
    }
}

/// The surface showed the wish `seq`, or gives it up.
pub(super) fn wish_shown(app: &mut CodexApp, seq: u64) {
    Rc::make_mut(&mut app.wishes).shown(seq);
}

/// What the chat reads of the thread in use that a feature knows (`Out::Chat`): the flag that
/// gates the commands a side conversation lacks and the footer's label; both are written only
/// when they changed.
fn chat_facts(app: &mut CodexApp, wanted: ChatFacts, fx: &mut AppFx) {
    if app.screens.chat.flags.side_conversation_active != wanted.side_conversation {
        Rc::make_mut(&mut app.screens)
            .chat
            .flags
            .side_conversation_active = wanted.side_conversation;
    }
    if app.screens.chat.facts.side != wanted.side {
        let chat = &mut Rc::make_mut(&mut app.screens).chat;
        chat::apply(chat, chat::Ev::Facts(facts::Ev::Side(wanted.side)), fx);
    }
}

/// The displayed connection reported rolling usage. Ignore notifications of a different account
/// here, where the original address is known.
pub(super) fn rolling_limits(
    app: &mut CodexApp,
    backend: BackendId,
    snapshot: &codex_app_server_protocol::RateLimitSnapshot,
    fx: &mut AppFx,
) {
    if super::session::active_backend(app) == backend {
        run(app, fx, |states, cx| {
            features::rolling_rate_limits(states, backend, snapshot, cx);
        });
    }
}

pub(super) fn rolling_remedy(
    app: &mut CodexApp,
    backend: BackendId,
    snapshot: &codex_app_server_protocol::RateLimitSnapshot,
) {
    if let Some(reached) = snapshot.rate_limit_reached_type
        && features::change_rate_remedy(
            &mut app.features,
            backend,
            features::RateRemedy::Rolling(reached),
        )
    {
        super::supersede_rate_banner(app, backend);
    }
}

/// The thread has passed the app's server ownership gate. Its terminal error can change only
/// that backend's remedy before the limit feature decides whether a fallback is needed.
pub(super) fn rate_failure(
    app: &mut CodexApp,
    backend: BackendId,
    error: &ErrorNotification,
    fx: &mut AppFx,
) {
    let kind = features::rate_failure_of(error);
    if kind == Some(features::RateFailure::UsageLimit)
        && features::change_rate_remedy(
            &mut app.features,
            backend,
            features::RateRemedy::UsageLimitError,
        )
    {
        super::supersede_rate_banner(app, backend);
        if app.screens.chat.status.account_usage {
            super::session::refresh_rates(
                app,
                backend,
                crate::screens::status::RateLimitOrigin::Startup,
                fx,
            );
        }
    }
    let address = app.address(backend);
    let thread = ThreadId::new(&error.thread_id);
    let can_nudge = !super::owner_nudge_in_flight(app);
    run(app, fx, |states, cx| {
        features::rate_failed(states, thread, address, kind, can_nudge, cx);
    });
}

/// A workspace fallback belongs to one routed failure. Close its authority when the shown
/// thread changes or a banner becomes applicable to it, including after a model change.
pub(super) fn sync_rate_nudge(app: &mut CodexApp) {
    let Some((thread, address)) = features::rate_nudge_origin(&app.features) else {
        return;
    };
    if app.session.active.as_ref() != Some(thread)
        || super::session::active_backend(app) != address.backend
        || app.address(address.backend) != address
        || (shown_banner(app).is_some() && features::rate_banner_shows(view(app)))
    {
        features::dismiss_rate_nudge(&mut app.features);
    }
}

/// Retire the offer of the connection or account that ended.
pub(super) fn retire_rate_offer(app: &mut CodexApp, backend: BackendId) {
    features::retire_rate_offer(&mut app.features, backend);
}

pub(super) fn forget_rate_remedy(app: &mut CodexApp, backend: BackendId) {
    features::forget_rate_remedy(&mut app.features, backend);
}

/// Resolve which completion prompt may be published before the routed event reaches the model.
/// Replay and repeat completion cannot reserve a second offer. A server that omitted the
/// start still gets the limit prompt, as long as it is not completing a different running turn.
pub(super) fn completion_priority(
    app: &mut CodexApp,
    thread: &ThreadId,
    ev: &ThreadEv,
    address: Address,
    fx: &mut AppFx,
) -> PromptPriority {
    let ThreadEv::TurnCompleted { turn, .. } = ev else {
        return PromptPriority::Ordinary;
    };
    if app.session.active.as_ref() != Some(thread)
        || !app.threads.get(thread).is_some_and(|model| {
            !model.turn_finished(turn) && model.running_turn().is_none_or(|running| running == turn)
        })
    {
        return PromptPriority::Ordinary;
    }
    let mut priority = PromptPriority::Ordinary;
    run(app, fx, |states, cx| {
        priority = features::reserve_completion_prompt(states, thread, turn, address, cx);
    });
    priority
}

/// A notification of the server's stream: every feature that follows it is told its event
/// (`Feature::project`, `on_event`), in list order.
pub(super) fn observe(app: &mut CodexApp, notification: &ServerNotification, fx: &mut AppFx) {
    for event in features::project(notification) {
        run(app, fx, |states, cx| features::on_event(states, event, cx));
    }
}

/// A server request, observe-only (`Feature::project_request`): the app asks the user as before.
pub(super) fn observe_request(app: &mut CodexApp, request: &ServerRequest, fx: &mut AppFx) {
    for event in features::project_request(request) {
        run(app, fx, |states, cx| features::on_event(states, event, cx));
    }
}

/// A thread a feature asked for (`AppCx::spawn`, `open_thread`) is open, or could not be had.
pub(super) fn thread_opened(
    app: &mut CodexApp,
    ctx: FeatureCtx,
    result: Result<ThreadId, Refusal>,
    fx: &mut AppFx,
) {
    run(app, fx, |states, cx| {
        features::thread_opened(states, ctx, result, cx)
    });
}

/// A structured turn a feature asked for ended, or could not be had.
pub(super) fn structured_done(
    app: &mut CodexApp,
    ctx: FeatureCtx,
    result: Result<String, crate::features::StructuredFailure>,
    fx: &mut AppFx,
) {
    run(app, fx, |states, cx| {
        features::structured_done(states, ctx, result, cx)
    });
}

/// The account's limits were read (`AppCtx::RateLimits`): every feature's `on_rate_limits_read`.
pub(super) fn on_rate_limits_read(
    app: &mut CodexApp,
    backend: BackendId,
    read: &RateRead,
    fx: &mut AppFx,
) {
    run(app, fx, |states, cx| {
        features::on_rate_limits_read(states, backend, read, cx)
    });
}

/// What a feature says for the error of a failed turn instead of the server's `message`.
pub(super) fn fault_words(app: &CodexApp, thread: &ThreadId, message: &str) -> Option<String> {
    features::fault_words(&app.features, thread, message, &view(app))
}

/// The feature that asked for a structured turn still wants it (`Feature::structured_wanted`).
pub(super) fn structured_wanted(app: &CodexApp, ctx: &FeatureCtx) -> bool {
    features::structured_wanted(&app.features, ctx, &view(app))
}

/// A user message of `thread` completed while it had no name: every feature's `on_user_message`.
pub(super) fn on_user_message(app: &mut CodexApp, thread: &ThreadId, text: &str, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::on_user_message(states, thread, text, cx)
    });
}

/// The thread in use is `thread`: every feature's `on_shown`, in list order.
pub(super) fn on_shown(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    run(app, fx, |states, cx| features::on_shown(states, thread, cx));
}

/// The connection came up, was lost or is being closed: every feature's `on_connection`.
pub(super) fn connection(app: &mut CodexApp, connection: Connection, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::on_connection(states, connection, cx)
    });
}

/// A fact the host sent unasked, for the feature it names.
pub(super) fn fact(app: &mut CodexApp, fact: FeatureFact, fx: &mut AppFx) {
    run(app, fx, |states, cx| features::on_fact(states, fact, cx));
}

/// A resumed thread is in use: every feature's `on_resumed`, in list order.
pub(super) fn resumed(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::on_resumed(states, thread, cx)
    });
}

/// The user interrupted the running turn of `thread`: every feature's `on_interrupt`.
pub(super) fn interrupted(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::on_interrupt(states, thread, cx)
    });
}

#[cfg(test)]
mod tests;
