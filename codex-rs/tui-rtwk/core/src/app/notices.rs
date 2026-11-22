//! What the host announces to the open conversation: a newer release (`UpdateAvailable`) and the
//! connected server's other version (`ServerVersion`), and the Daybreak facts that word a cyber
//! refusal. (Oracle `app/startup.rs`: the notices are inserted into the first session's history
//! once it exists; `chatwidget/turn_runtime.rs` `cyber_policy_notice`.)

use super::{AppFx, CodexApp, apply_thread};
use crate::ids::ThreadId;
use crate::screens::cells::Daybreak;
use crate::threads::{Item, Notice, ThreadEv};

/// Puts `notice` in the shown conversation's history; with none yet it waits for the first one.
pub(super) fn announce(app: &mut CodexApp, notice: Notice, fx: &mut AppFx) {
    match app.session.active.clone() {
        Some(thread) if app.threads.contains(&thread) => append(app, &thread, notice, fx),
        _ => app.pending_notices.push(notice),
    }
}

/// The browser did not open a page: the shown conversation says so as an error line (oracle
/// `add_error_message("Failed to open browser for {url}: {err}")`, worded by the host); with
/// none shown there is no line to add it to.
pub(super) fn open_failed(app: &mut CodexApp, message: String, fx: &mut AppFx) {
    let Some(thread) = app.session.active.clone() else {
        return;
    };
    let ev = ThreadEv::Appended {
        turn: None,
        item: Item::Error { message },
    };
    apply_thread(app, &thread, ev, fx);
}

/// The notices that waited for a conversation go into `thread`, the first one attached.
pub(super) fn flush(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    for notice in std::mem::take(&mut app.pending_notices) {
        append(app, thread, notice, fx);
    }
}

fn append(app: &mut CodexApp, thread: &ThreadId, notice: Notice, fx: &mut AppFx) {
    let ev = ThreadEv::Appended {
        turn: None,
        item: Item::Notice(notice),
    };
    apply_thread(app, thread, ev, fx);
}

/// How the Daybreak setting words a cyber refusal on `thread` right now (oracle
/// `cyber_policy_notice`: the thread's own setting, the account's eligibility and the catalog's
/// programs for the model the thread runs on).
pub(super) fn daybreak_of(app: &CodexApp, thread: &ThreadId) -> Daybreak {
    let slot = app.threads.get(thread).map(|model| &model.screen_slots().chat);
    let model = slot
        .and_then(|chat| chat.model.as_deref())
        .or(app.screens.chat.status.model.as_deref());
    let enabled = slot.and_then(|chat| chat.daybreak).unwrap_or(false);
    app.daybreak.notice(model, enabled)
}
