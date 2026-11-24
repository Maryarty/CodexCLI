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
    Rc::make_mut(&mut app.session)
        .backend_banners
        .remove(&backend);
    Rc::make_mut(&mut app.session).reserve.remove(&backend);
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
mod tests {
    use super::super::{AppEv, apply_app, worktree};
    use super::*;
    use crate::features::FeatureCtx;
    use crate::ids::{Cwd, ThreadId};
    use crate::screens::{ScreenEv, agents, chat, resume, select, startup, status};
    use std::collections::BTreeSet;

    /// The kind of an ask: exhaustive, so that a new `AppCtx` is listed here and in `samples`.
    fn kind(ctx: &AppCtx) -> &'static str {
        match ctx {
            AppCtx::StartThread { .. } => "StartThread",
            AppCtx::Agents(_) => "Agents",
            AppCtx::Resume(_) => "Resume",
            AppCtx::Select(_) => "Select",
            AppCtx::RateLimits { .. } => "RateLimits",
            AppCtx::Nudge { .. } => "Nudge",
            AppCtx::NewConversation { .. } => "NewConversation",
            AppCtx::Removal { .. } => "Removal",
            AppCtx::OpenThread { .. } => "OpenThread",
            AppCtx::Opening { .. } => "Opening",
            AppCtx::Worktree { .. } => "Worktree",
            AppCtx::SessionRead => "SessionRead",
            AppCtx::OpenSession { .. } => "OpenSession",
            AppCtx::Structured { .. } => "Structured",
            AppCtx::Feature(_) => "Feature",
        }
    }

    const KINDS: usize = 15;

    /// One ask of every kind, all on the first backend.
    fn samples() -> Vec<AppCtx> {
        let backend = BackendId::default();
        let thread = ThreadId::new("thread-1");
        vec![
            AppCtx::StartThread { switch: None },
            AppCtx::Agents(agents::Ctx::List { append: false }),
            AppCtx::Resume(resume::Ctx::Page {
                backend,
                generation: 0,
                request: 0,
                search: None,
            }),
            AppCtx::Select(select::Ctx::Models { request: 1 }),
            AppCtx::RateLimits {
                backend,
                origin: status::RateLimitOrigin::Startup,
            },
            AppCtx::Nudge {
                backend,
                nudge: chat::Nudge::Credits,
                thread: Some(thread.clone()),
            },
            AppCtx::NewConversation {
                source: None,
                backend,
                name: None,
                plan: None,
                draft: None,
                spawned: None,
            },
            AppCtx::Removal {
                thread: thread.clone(),
                action: agents::Action::Archive,
            },
            AppCtx::OpenThread {
                thread: thread.clone(),
                backend,
                cancelled: false,
            },
            AppCtx::Opening {
                thread: thread.clone(),
                ctx: FeatureCtx::Plugins(crate::features::plugins::Ctx::List {
                    cwd: Cwd::new("/work"),
                    request: 1,
                }),
            },
            AppCtx::Worktree {
                backend,
                step: worktree::Step::Creating { chat: None },
            },
            AppCtx::SessionRead,
            AppCtx::OpenSession {
                action: startup::Action::Resume,
                thread,
            },
            AppCtx::Structured { run: 1, backend },
            AppCtx::Feature(FeatureCtx::Plugins(crate::features::plugins::Ctx::List {
                cwd: Cwd::new("/work"),
                request: 1,
            })),
        ]
    }

    /// A remote server that drops answers none of the asks in flight; the host drops their
    /// futures. Each kind of ask is answered by the loss, and an ask of another connection is not.
    #[test]
    fn a_loss_answers_an_ask_of_every_kind_in_flight() {
        let all = samples();
        let kinds: BTreeSet<_> = all.iter().map(kind).collect();
        assert_eq!(kinds.len(), KINDS, "one sample for every kind of ask");
        let mut app = CodexApp::new();
        for ctx in all {
            let ticket = app.mint.next();
            app.outgoing.insert(ticket, ctx);
        }
        assert_eq!(app.outgoing.len(), KINDS);
        let mut fx = AppFx::default();
        apply_app(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
            &mut fx,
        );
        let left: Vec<_> = app.outgoing.tickets().collect();
        assert!(left.is_empty(), "still in flight: {left:?}");
    }

    #[test]
    fn a_loss_keeps_the_asks_of_another_connection() {
        let other = BackendId(1);
        let mut app = CodexApp::new();
        let ticket = app.mint.next();
        app.outgoing.insert(
            ticket,
            AppCtx::RateLimits {
                backend: other,
                origin: status::RateLimitOrigin::Startup,
            },
        );
        let ctx = app.outgoing.get(ticket).cloned().unwrap();
        assert_eq!(ctx.on_lost(&app, BackendId::default()), OnLost::Keep);
        assert_eq!(ctx.on_lost(&app, other), OnLost::Fail);
    }
}
