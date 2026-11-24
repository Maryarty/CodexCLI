//! The app's half of the feature slices (ADR-012): the app as a read-only view for a
//! feature's step, and the outbox carried out after it. A feature never holds the app; it is
//! handed `&mut` of its own state and an `AppCx`, says what it wants, and this file does it,
//! in order, through the app's own doors (`session_note`, `apply_thread`, the wishes).

use std::rc::Rc;

use super::effects::{self, Done};
use super::{AppCtx, AppFx, CodexApp, Command, Now, Wake, apply_thread, session_note};
use crate::features::{
    self, AppCx, AppView, FeatureCtx, FeatureFact, FeatureIntent, FeatureThreadIntent, InterestStep,
};
use crate::features::{ChatFacts, Connection, FeatureStates, Out, Refusal};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::chat::facts;
use crate::screens::{ScreenThreadIntent, chat};
use crate::threads::{Note, NoteKey, ThreadEv};
use codex_app_server_protocol::{ServerNotification, ServerRequest};

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
    .at(app.now)
}

/// Runs one step of the features with the app as a view, then carries out what it said.
pub(super) fn run(
    app: &mut CodexApp,
    fx: &mut AppFx,
    step: impl FnOnce(&mut FeatureStates, &mut AppCx<'_>),
) {
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
    next = nearest(next, app.structured.wake_at());
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
            let ticket = app.mint.next();
            app.outgoing.insert(ticket, AppCtx::Feature(ctx));
            fx.commands.push(Command::Feature {
                ticket,
                thread,
                command,
            });
        }
        Out::Effect { ask, ctx } => {
            let request = app.effects.ask_for_feature(ctx);
            fx.commands.push(effects::command(request, ask));
        }
        Out::Note { thread, note } => {
            let thread = thread.or_else(|| app.session.active.clone());
            session_note(app, thread, Note::from(NoteKey::Feature(note)), fx);
        }
        Out::Thread { thread, intent } => {
            apply_thread(app, &thread, ThreadEv::FeatureIntent(intent), fx);
        }
        Out::Wish { scope, wish } => Rc::make_mut(&mut app.wishes).push(scope, wish),
        Out::Lifecycle(lifecycle) => super::lifecycle::carry(app, lifecycle, fx),
        Out::Chat(facts) => chat_facts(app, facts, fx),
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

/// The connection is new: what the surfaces still want is asked again.
pub(super) fn reconnected(app: &mut CodexApp, fx: &mut AppFx) {
    for interest in app.interests.all() {
        run(app, fx, |states, cx| {
            features::on_interest(states, &interest, true, cx)
        });
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

#[cfg(test)]
mod tests;
