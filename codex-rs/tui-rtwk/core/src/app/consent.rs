//! The folder trust asked over the chat (`/cd`): when the question ended, what
//! waited for it goes on (the session starts in the folder) or says why not.
//! The state is the onboarding model's (`screens::onboarding::consent`); this
//! is the app's half, called after every event and intent that can end it.

use std::rc::Rc;

use super::{AppFx, CodexApp, session};
use crate::screens::onboarding::consent::{self, Outcome, Then};
use crate::threads::{CdRefusal, Note, NoteKey};

/// The question ended: `/cd` starts the session in the folder (the user
/// trusted it, or it needed no question), or says the folder's trust could
/// not be read (oracle `resume_config.rs`: `Unable to check folder trust`).
/// A question the user declined left nothing to do.
pub(super) fn follow(app: &mut CodexApp, fx: &mut AppFx) {
    if !consent::settled(&app.screens.onboarding) {
        return;
    }
    let Some(outcome) = consent::take(&mut Rc::make_mut(&mut app.screens).onboarding) else {
        return;
    };
    match outcome {
        Outcome::Go(Then::Cd { source, ask }) => session::start_in(app, source, ask, fx),
        Outcome::Failed {
            message,
            then: Then::Cd { source, .. },
        } => session::session_note(
            app,
            source,
            Note::from(NoteKey::CdRefused(CdRefusal::TrustCheck(message))),
            fx,
        ),
    }
}
