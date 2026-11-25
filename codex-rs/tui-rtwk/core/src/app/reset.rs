//! The usage-limit reset flow's half that needs the app (oracle `chatwidget/usage.rs`,
//! `app/event_dispatch.rs` `RateLimitsLoaded`): the account's limits are read for a reset's sake
//! (`RateLimitOrigin::Menu`, `ResetPicker`, `ResetConsume`, `Hint`), and what a read or a
//! redemption leaves is kept by `screens::usage::reset::State`. The state is a pure model with its
//! own tests; this file only starts the reads and asks and hands the answers to it.

use std::rc::Rc;

use super::session::refresh_rates;
use super::{AppFx, CodexApp, active_backend, apply_thread};
use crate::ids::ThreadId;
use crate::screens::status::{RateLimitOrigin, ResetCredits};
use crate::screens::usage::{self, reset};
use crate::threads::{Item, Notice, ThreadEv, ToolStatus};

fn state(app: &mut CodexApp) -> &mut reset::State {
    &mut Rc::make_mut(&mut app.screens).usage.reset
}

/// `Intent::Menu` and `Intent::Resets`: the reads they need are started here; every other usage
/// intent is the model's own (`usage::step`). False for an intent that is not one of these.
pub(super) fn intent(app: &mut CodexApp, intent: &usage::Intent, fx: &mut AppFx) -> bool {
    let origin = match intent {
        usage::Intent::Menu => state(app).open_menu().map(RateLimitOrigin::Menu),
        usage::Intent::Resets => Some(RateLimitOrigin::ResetPicker(state(app).open_resets())),
        _ => return false,
    };
    if let Some(origin) = origin {
        refresh_rates(app, active_backend(app), origin, fx);
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
    model.hook_status().is_some()
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

/// The host's answer to a redemption: a reset reads the credits again for the count left.
pub(super) fn redeemed(
    app: &mut CodexApp,
    key: &str,
    result: Result<reset::Outcome, String>,
    fx: &mut AppFx,
) {
    if state(app).redeemed(key, result) == reset::Next::Read {
        refresh_rates(app, active_backend(app), RateLimitOrigin::ResetConsume, fx);
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
