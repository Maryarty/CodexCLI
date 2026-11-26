//! The two steps of a connection's life the app takes: it is lost (`lost`) and it is there again
//! (`up`). One path for every way a loss is told: the embedded server's and the cloud backend's
//! `AppEv::Disconnected`, and a remote server's `Reconnecting` (the host drops the futures in
//! flight and restores the connection, `host/src/in_process.rs`; the oracle's request on a lost
//! connection errors out and the overview refreshes after `finish_reconnect`).
//!
//! A loss settles the asks registered on that connection and tells the features,
//! which let go of what they held for it.

use super::{AppFx, BackendId, CodexApp, features};
use crate::features::Connection;
use crate::screens::onboarding;
use std::rc::Rc;

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
    app.retire_outgoing(backend, fx);
    if backend == BackendId::default() {
        features::connection(app, Connection::Lost, fx);
    }
}

/// A connection of `backend` is up (after a reconnect too): what the surfaces still want is asked
/// again and the features are told.
pub(super) fn up(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    if backend == BackendId::default() {
        features::connection(app, Connection::Up, fx);
    }
    features::reconnected(app, backend, fx);
}

#[cfg(test)]
mod tests;
