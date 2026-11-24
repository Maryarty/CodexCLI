//! What the host announces to the open conversation: a newer release (`UpdateAvailable`) and the
//! connected server's other version (`ServerVersion`), and the Daybreak facts that word a cyber
//! refusal. (Oracle `app/startup.rs`: the notices are inserted into the first session's history
//! once it exists; `chatwidget/turn_runtime.rs` `cyber_policy_notice`), and the card a
//! conversation opens with (`history_cell/session.rs` `new_session_info`).

use std::rc::Rc;

use super::{AppFx, CodexApp, apply_thread};
use crate::ids::ThreadId;
use crate::screens::cells::{Daybreak, SessionInfo};
use crate::screens::tips;
use crate::threads::{Item, Notice, ThreadEv};

/// Puts `notice` in the shown conversation's history; with none yet it waits for the first one.
pub(super) fn announce(app: &mut CodexApp, notice: Notice, fx: &mut AppFx) {
    match app.session.active.clone() {
        Some(thread) if app.threads.contains(&thread) => append(app, &thread, notice, fx),
        _ => app.pending_notices.push(notice),
    }
}

/// An error the host worded: the shown conversation keeps it as an error line (oracle
/// `add_error_message`); with none shown there is no line to add it to.
pub(super) fn error_line(app: &mut CodexApp, message: String, fx: &mut AppFx) {
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

/// The host's startup facts came: the shown conversation, when one attached before them, gets the
/// card it would have opened with.
pub(super) fn startup(app: &mut CodexApp, fx: &mut AppFx) {
    if let Some(thread) = app.session.active.clone() {
        session_card(app, &thread, fx);
    }
}

/// The card `thread` opens with, once (oracle `on_session_configured`): the help of the user's
/// first run, else a tip (the model-availability notice the host found, for the first
/// conversation, else the pick of the catalog) and the model note. The host's startup facts
/// open the door; without them (a test's bare app) no card.
pub(super) fn session_card(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    let Some(startup) = app.screens.cells.startup.as_ref() else {
        return;
    };
    let first_run = startup.first_run;
    let Some(model) = app.threads.get(thread) else {
        return;
    };
    let carded = model
        .transcript()
        .iter()
        .any(|(_, item)| matches!(item, Item::Notice(Notice::SessionInfo(_))));
    if carded {
        return;
    }
    let used = model.screen_slots().chat.model.clone();
    // the oracle takes the override at every session, whether the tip is shown or not
    let announced = Rc::make_mut(&mut app.screens)
        .cells
        .startup
        .as_mut()
        .and_then(|startup| startup.tooltip.take());
    let used = used.as_deref().unwrap_or_default();
    // the oracle's requested model is the chat's current one, which the session just set to the
    // used one unless a collaboration mask names another; ours carry none
    let info = SessionInfo::opening(
        first_run,
        app.tui.show_tooltips,
        || announced.or_else(|| tips::pick(thread, &app.keymap)),
        used,
        used,
    );
    if let Some(info) = info {
        append(app, thread, Notice::SessionInfo(info), fx);
    }
}

/// The line of a forked thread (oracle `emit_forked_thread_event`, on a normal conversation, not
/// a side one), once: the thread it came from and that thread's name when we hold it (the oracle
/// reads it of the server; a parent we do not hold goes by its id).
pub(super) fn forked_line(
    app: &mut CodexApp,
    thread: &ThreadId,
    details: &codex_app_server_protocol::Thread,
    fx: &mut AppFx,
) {
    let Some(from) = details.forked_from_id.clone() else {
        return;
    };
    if crate::screens::chat::blocks_direct_input(details) {
        return;
    }
    let Some(model) = app.threads.get(thread) else {
        return;
    };
    if model
        .transcript()
        .iter()
        .any(|(_, item)| matches!(item, Item::Notice(Notice::ForkedThread { .. })))
    {
        return;
    }
    let title = app
        .threads
        .get(&ThreadId::new(&from))
        .and_then(|parent| parent.screen_slots().chat.name.clone())
        .filter(|name| !name.trim().is_empty());
    append(app, thread, Notice::ForkedThread { from, title }, fx);
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
    let slot = app
        .threads
        .get(thread)
        .map(|model| &model.screen_slots().chat);
    let model =
        slot.and_then(|chat| chat.model.as_deref())
            .or(app.screens.chat.status.model.as_deref());
    // What `/daybreak` saved for the thread comes before what its metadata said.
    let enabled = app
        .threads
        .get(thread)
        .and_then(|model| {
            // the one read of a slice's part in the hub: fw-hubs-b's (`appstate` in the budget)
            let saved = model
                .view()
                .parts()
                .and_then(|parts| parts.small_cmds.saved());
            saved.or(model.screen_slots().chat.daybreak)
        })
        .unwrap_or(false);
    app.daybreak.notice(model, enabled)
}
