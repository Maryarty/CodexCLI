//! The app's half of the feature slices (ADR-012): the app as a read-only view for a
//! feature's step, and the outbox carried out after it. A feature never holds the app; it is
//! handed `&mut` of its own state and an `AppCx`, says what it wants, and this file does it,
//! in order, through the app's own doors (the select screen, the dialogs, `session_note`,
//! `apply_thread`).

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command, apply_thread, session_note};
use crate::features::{
    self, AppCx, AppView, FeatureCtx, FeatureIntent, FeaturePick, FeaturePrompt,
};
use crate::features::{FeatureStates, Out};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::dialogs::{Open, Prompt};
use crate::screens::select::{self, Menu};
use crate::threads::{Note, NoteKey, ThreadEv};

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
        Out::Menu(menu) => select::apply(
            &mut Rc::make_mut(&mut app.screens).select,
            select::Ev::Opened(Menu::Feature(menu)),
            fx,
        ),
        Out::Prompt { prompt, initial } => {
            Rc::make_mut(&mut app.screens).dialogs.open =
                Some(Open::Prompt(Prompt::feature(prompt, initial)));
        }
        Out::Note { thread, note } => {
            let thread = thread.or_else(|| app.session.active.clone());
            session_note(app, thread, Note::from(NoteKey::Feature(note)), fx);
        }
        Out::Thread { thread, intent } => {
            apply_thread(app, &thread, ThreadEv::FeatureIntent(intent), fx);
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

/// What a feature's menu settled on.
pub(super) fn picked(app: &mut CodexApp, pick: FeaturePick, fx: &mut AppFx) {
    run(app, fx, |states, cx| features::picked(states, pick, cx));
}

/// A feature's prompt, submitted with `text`.
pub(super) fn prompt_submitted(
    app: &mut CodexApp,
    prompt: FeaturePrompt,
    text: String,
    fx: &mut AppFx,
) {
    run(app, fx, |states, cx| {
        features::prompt_submitted(states, prompt, text, cx);
    });
}

/// A resumed thread is in use: every feature's `on_resumed`, in list order.
pub(super) fn resumed(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    run(app, fx, |states, cx| {
        features::on_resumed(states, thread, cx)
    });
}
