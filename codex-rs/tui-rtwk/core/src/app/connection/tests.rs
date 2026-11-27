//! The loss of a connection answers an ask of every kind (`AppCtx`). It builds the ctx of a real
//! feature, so the rules let this file name one (`ROOT_TESTS`).

use super::*;
use crate::app::{AppCtx, AppEv, TurnOperation, apply_app, worktree};
use crate::features::FeatureCtx;
use crate::ids::{Cwd, ThreadId, Ticket};
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
        AppCtx::Redemption { .. } => "Redemption",
        AppCtx::Nudge { .. } => "Nudge",
        AppCtx::NewConversation { .. } => "NewConversation",
        AppCtx::Removal { .. } => "Removal",
        AppCtx::OpenThread { .. } => "OpenThread",
        AppCtx::Opening { .. } => "Opening",
        AppCtx::Worktree { .. } => "Worktree",
        AppCtx::SessionRead => "SessionRead",
        AppCtx::OpenSession { .. } => "OpenSession",
        AppCtx::Feature(_) => "Feature",
        AppCtx::TurnOperation(operation) => match operation {
            TurnOperation::Structured { .. } => "Structured",
            TurnOperation::SafetyRetry { .. } => "SafetyRetry",
        },
    }
}

const KINDS: usize = 17;

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
            superseded: false,
        },
        AppCtx::Redemption {
            key: "redeem".to_owned(),
            backend,
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
        AppCtx::TurnOperation(TurnOperation::Structured { run: 1, backend }),
        AppCtx::TurnOperation(TurnOperation::SafetyRetry {
            generation: Ticket::new(1),
            backend,
            attaching: true,
        }),
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
        let address = app.address(BackendId::default());
        app.outgoing.insert(ticket, address, ctx);
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
    let address = app.address(other);
    app.outgoing.insert(
        ticket,
        address,
        AppCtx::RateLimits {
            backend: other,
            origin: status::RateLimitOrigin::Startup,
            superseded: false,
        },
    );
    lost(&mut app, BackendId::default(), &mut AppFx::default());
    assert_eq!(app.outgoing.address(ticket), Some(address));
    lost(&mut app, other, &mut AppFx::default());
    assert!(!app.outgoing.contains(ticket));
}
