//! The app's half of the feature slices (ADR-012): the app as a read-only view for a
//! feature's step, and the outbox carried out after it. A feature never holds the app; it is
//! handed `&mut` of its own state and an `AppCx`, says what it wants, and this file does it,
//! in order, through the app's own doors (`session_note`, `apply_thread`, the wishes).

use std::rc::Rc;

use super::effects::{self, Done};
use super::{AppCtx, AppFx, CodexApp, Command, apply_thread, session_note};
use crate::features::{
    self, AppCx, AppView, FeatureCtx, FeatureFact, FeatureIntent, FeatureThreadIntent, InterestStep,
};
use crate::features::{FeatureStates, Out};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::{ScreenThreadIntent, chat};
use crate::threads::{Note, NoteKey, ThreadEv};

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
        );
        let mut cx = AppCx::new(view);
        step(Rc::make_mut(&mut app.features), &mut cx);
        cx.into_out()
    };
    for out in out {
        carry(app, out, fx);
    }
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
