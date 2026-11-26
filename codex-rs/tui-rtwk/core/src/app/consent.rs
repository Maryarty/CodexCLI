//! Holds session actions until the destination folder's trust is resolved.

use std::rc::Rc;

use super::{AppFx, CodexApp, lifecycle, session};
use crate::features::Refusal;
use crate::screens::agents;
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::{self, Outcome, Target, Then};
use crate::threads::{CdRefusal, Note, NoteKey, ThreadAsk};

pub(super) fn begin(
    app: &mut CodexApp,
    then: Then,
    target: Target,
    cancel: TrustCancel,
    fx: &mut AppFx,
) {
    if let Some(then) = consent::begin(
        &mut Rc::make_mut(&mut app.screens).onboarding,
        then,
        target,
        cancel,
        fx,
    ) {
        go(app, then, fx);
    }
}

/// Advance the held action once, after the event or intent that settled it.
pub(super) fn follow(app: &mut CodexApp, fx: &mut AppFx) {
    if !consent::settled(&app.screens.onboarding) {
        return;
    }
    let Some(outcome) = consent::take(&mut Rc::make_mut(&mut app.screens).onboarding) else {
        return;
    };
    match outcome {
        Outcome::Go(then) => go(app, then, fx),
        Outcome::Cancelled(then) => refuse_feature(app, then, fx),
        Outcome::Failed { message, then } => {
            let source = if let Then::Cd { source, .. } = &then {
                source.clone()
            } else {
                Rc::make_mut(&mut app.screens).agents.notice =
                    Some(agents::Notice::TrustCheckFailed(message.clone()));
                app.session.active.clone()
            };
            session::session_note(
                app,
                source,
                Note::from(NoteKey::CdRefused(CdRefusal::TrustCheck(message))),
                fx,
            );
            refuse_feature(app, then, fx);
        }
    }
}

fn go(app: &mut CodexApp, then: Then, fx: &mut AppFx) {
    match then {
        Then::Cd { source, ask } => session::start_in(app, source, ask, fx),
        Then::Open(id) => {
            super::open_verified(app, &id, fx);
        }
        Then::New { cwd } => super::send_agent_command(
            app,
            agents::Out {
                command: agents::Command::Thread(ThreadAsk::Start { cwd }),
                ctx: agents::Ctx::Start,
            },
            fx,
        ),
        Then::Fresh { feature: None } => super::start_first_thread(app, fx),
        Then::Fresh { feature: Some(ctx) } => lifecycle::start_after_consent(app, ctx, fx),
    }
}

fn refuse_feature(app: &mut CodexApp, then: Then, fx: &mut AppFx) {
    if let Then::Fresh { feature: Some(ctx) } = then {
        super::features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
    }
}
