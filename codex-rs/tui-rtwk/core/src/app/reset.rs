//! The usage-limit reset flow's half that needs the app (oracle `chatwidget/usage.rs`,
//! `app/event_dispatch.rs` `RateLimitsLoaded`): the account's limits are read for a reset's sake
//! (`RateLimitOrigin::Menu`, `ResetPicker`, `ResetConsume`, `Hint`), and what a read or a
//! redemption leaves is kept by `screens::usage::reset::State`. The state is a pure model with its
//! own tests; this file only starts the reads and asks and hands the answers to it.

use std::rc::Rc;

use super::session::refresh_rates;
use super::{AppCtx, AppFx, CodexApp, Command, active_backend, apply_thread};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::{Answer, RpcError};
use crate::screens::status::{RateLimitOrigin, ResetCredits};
use crate::screens::usage::{self, reset};
use crate::screens::{ScreenCommand, ScreenReply};
use crate::threads::{Item, Notice, ThreadEv, ToolStatus};

fn state(app: &mut CodexApp) -> &mut reset::State {
    &mut Rc::make_mut(&mut app.screens).usage.reset
}

/// The account's reset reads and redemptions. Usage report intents stay in the screen.
pub(super) fn intent(app: &mut CodexApp, intent: &usage::Intent, fx: &mut AppFx) -> bool {
    let backend = active_backend(app);
    let origin = match intent {
        usage::Intent::Menu => state(app).open_menu().map(RateLimitOrigin::Menu),
        usage::Intent::Resets => {
            let state = state(app);
            state.picker_backend = Some(backend);
            Some(RateLimitOrigin::ResetPicker(state.open_resets()))
        }
        usage::Intent::Redeem { credit_id } => {
            let next = state(app).redeem(credit_id.clone());
            send_redemption(app, next, fx);
            return true;
        }
        usage::Intent::Retry => {
            let next = state(app).retry();
            send_redemption(app, next, fx);
            return true;
        }
        _ => return false,
    };
    if let Some(origin) = origin {
        refresh_rates(app, backend, origin, fx);
    }
    true
}

/// The startup prefetch of the limits, which also looks for resets to tell the user of (oracle
/// `start_rate_limit_reset_startup_check`): once per account, until the count is known.
pub(super) fn startup(app: &mut CodexApp, fx: &mut AppFx) {
    if app.screens.usage.reset.available.is_some() || app.screens.usage.reset.hint_out() {
        refresh_rates(app, active_backend(app), RateLimitOrigin::Startup, fx);
        return;
    }
    let request = state(app).start_hint();
    refresh_rates(app, active_backend(app), RateLimitOrigin::Hint(request), fx);
}

/// A read of the limits ended: `credits` is what it said of the reset credits, `Ok(None)` for an
/// answer that carried none and `Err` for a read that failed.
pub(super) fn read_done(
    app: &mut CodexApp,
    origin: RateLimitOrigin,
    credits: Result<Option<ResetCredits>, ()>,
    fx: &mut AppFx,
) {
    match origin {
        RateLimitOrigin::Menu(request) => state(app).menu_read(request, credits),
        RateLimitOrigin::ResetPicker(request) => state(app).list_read(request, credits),
        RateLimitOrigin::ResetConsume => state(app).after_reset_read(credits),
        RateLimitOrigin::Hint(request) => {
            if let Some(count) = state(app).hint_read(request, credits)
                && let Some(thread) = app.session.active.clone()
            {
                if output_active(app, &thread) {
                    state(app).park_hint(count);
                } else {
                    tell_hint(app, &thread, count, fx);
                }
            }
        }
        RateLimitOrigin::Status(_) | RateLimitOrigin::UsageMenu | RateLimitOrigin::Startup => {}
    }
}

/// Whether the thread shows output that is still being made: a streaming answer, a running tool
/// call, patch or web search, or a running hook (oracle `usage_history_insertion_blocked`: a
/// stream, the active cell, the hook cell). A line inserted among them would be out of order.
fn output_active(app: &CodexApp, thread: &ThreadId) -> bool {
    let Some(model) = app.threads.get(thread) else {
        return false;
    };
    let transcript = model.transcript();
    !model.running_hooks().is_empty()
        || transcript.iter().any(|(key, item)| {
            transcript.is_agent_streaming(key)
                || match item {
                    Item::Tool(tool) => tool.status == ToolStatus::Running,
                    Item::Patch(patch) => patch.status == ToolStatus::Running,
                    Item::Web(web) => !web.completed,
                    _ => false,
                }
        })
}

fn tell_hint(app: &mut CodexApp, thread: &ThreadId, count: i64, fx: &mut AppFx) {
    let item = Item::Notice(Notice::ResetAvailable { count });
    apply_thread(app, thread, ThreadEv::Appended { turn: None, item }, fx);
}

/// A thread event was applied: a hint that waited for the output to settle is told once none is
/// active (oracle `commit_pending_usage_output`).
pub(super) fn settle_hint(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    if !app.screens.usage.reset.hint_waits()
        || app.session.active.as_ref() != Some(thread)
        || output_active(app, thread)
    {
        return;
    }
    if let Some(count) = state(app).take_hint() {
        tell_hint(app, thread, count, fx);
    }
}

fn send_redemption(app: &mut CodexApp, next: reset::Next, fx: &mut AppFx) {
    let reset::Next::Redeem { key, credit_id } = next else {
        return;
    };
    let active = active_backend(app);
    let state = state(app);
    let selected = state.picker_backend.unwrap_or(active);
    let Some(redemption) = state.redemption.as_mut() else {
        return;
    };
    let backend = *redemption.backend.get_or_insert(selected);
    let command = usage::Command::Redeem(reset::Consume::Redeem {
        key: key.clone(),
        credit_id,
    });
    fx.ask(app, AppCtx::Redemption { key, backend }, |_, ticket| {
        Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Usage(command),
        }
    });
}

/// A redemption which reset the windows refreshes the same account's credits.
pub(super) fn answered(
    app: &mut CodexApp,
    backend: BackendId,
    key: &str,
    answer: Answer,
    fx: &mut AppFx,
) {
    let result = match answer {
        Answer::Screen(ScreenReply::Usage(usage::Reply::Redeemed(outcome))) => Ok(outcome),
        Answer::Failed(RpcError::Remote(message) | RpcError::Malformed(message)) => Err(message),
        Answer::Failed(error) => Err(format!("{error:?}")),
        _ => Err(format!("{:?}", RpcError::Mismatch)),
    };
    if state(app).redeemed(key, result) == reset::Next::Read {
        refresh_rates(app, backend, RateLimitOrigin::ResetConsume, fx);
    }
}

/// The account changed: what was asked of the old one is not the new one's.
pub(super) fn clear(app: &mut CodexApp) {
    state(app).clear();
}

/// The warnings a sample of the limits made due (oracle `on_rate_limit_snapshot_from`): one
/// visible notice each, in the thread that is open. With none open there is nowhere to tell them,
/// and the sample is not taken, so they come with the next one a thread can show.
pub(super) fn warn(
    app: &mut CodexApp,
    warnings: Vec<crate::screens::status::Warning>,
    fx: &mut AppFx,
) {
    let Some(thread) = app.session.active.clone() else {
        return;
    };
    for warning in warnings {
        let item = Item::Notice(Notice::RateLimitWarning {
            remaining_percent: warning.remaining_percent,
            window: warning.window,
        });
        apply_thread(app, &thread, ThreadEv::Appended { turn: None, item }, fx);
    }
}
