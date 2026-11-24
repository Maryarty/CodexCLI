//! The two steps of a connection's life the app takes: it is lost (`lost`) and it is there again
//! (`up`). One path for every way a loss is told: the embedded server's and the cloud backend's
//! `AppEv::Disconnected`, and a remote server's `Reconnecting` (the host drops the futures in
//! flight and restores the connection, `host/src/in_process.rs`; the oracle's request on a lost
//! connection errors out and the overview refreshes after `finish_reconnect`).
//!
//! A loss answers every ask in flight on the connection (`AppCtx::on_lost`: a new kind of ask has
//! to say whether it is one), so no operation waits for an answer that will not come, and tells
//! the features, which let go of what they held for it.

use super::{AppCtx, AppFx, BackendId, CodexApp, detach_threads, features, structured};
use crate::features::Connection;
use crate::screens::onboarding;
use std::rc::Rc;

/// What a loss of a connection does to an ask in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OnLost {
    /// The ask went to the lost connection: it is answered with a failure.
    Fail,
    /// The ask is on another connection.
    Keep,
}

impl AppCtx {
    /// What the loss of `lost` does to this ask. Every kind says which connection it went to: no
    /// `_` arm, so that a new kind has to choose.
    pub(super) fn on_lost(&self, app: &CodexApp, lost: BackendId) -> OnLost {
        let sent = match self {
            Self::StartThread { switch } => switch.unwrap_or_default(),
            Self::RateLimits { backend, .. }
            | Self::Nudge { backend, .. }
            | Self::NewConversation { backend, .. }
            | Self::OpenThread { backend, .. }
            | Self::Structured { backend, .. }
            | Self::Worktree { backend, .. } => *backend,
            Self::Resume(ctx) => ctx.backend(),
            Self::Select(
                crate::screens::select::Ctx::Fallback { backend }
                | crate::screens::select::Ctx::Switch { backend, .. },
            ) => *backend,
            // the startup page's read goes to the first backend
            Self::SessionRead => BackendId::default(),
            Self::Removal { thread, .. }
            | Self::OpenSession { thread, .. }
            | Self::Opening { thread, .. } => app.screens.agents.owner_of(thread.as_str()),
            Self::Agents(crate::screens::agents::Ctx::History { thread }) => {
                app.screens.agents.owner_of(thread)
            }
            // what the displayed backend is asked
            Self::Agents(_) | Self::Select(_) | Self::Feature(_) => super::active_backend(app),
        };
        if sent == lost {
            OnLost::Fail
        } else {
            OnLost::Keep
        }
    }
}

/// The connection of `backend` is gone. Every ask in flight on it is answered with a failure
/// (which leaves its operation `Loading` no more), what the startup asked of it is asked again
/// on the next one, the threads it held let go of their writer, the runs that wait for a stream
/// end, and the features are told.
pub(super) fn lost(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    // The startup asks went to the lost connection.
    if backend == BackendId::default() {
        onboarding::disconnected(&mut Rc::make_mut(&mut app.screens).onboarding);
    }
    app.reset_requests(backend);
    app.forget_reads_of(backend);
    app.attaching_events
        .retain(|(source, ..)| *source != backend);
    detach_threads(app, backend, fx);
    app.fail_in_flight(backend);
    structured::lost(app, backend, fx);
    features::connection(app, Connection::Lost, fx);
}

/// A connection of `backend` is up (after a reconnect too): what the surfaces still want is asked
/// again and the features are told.
pub(super) fn up(app: &mut CodexApp, fx: &mut AppFx) {
    features::reconnected(app, fx);
    features::connection(app, Connection::Up, fx);
}

#[cfg(test)]
mod tests;
