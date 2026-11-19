//! What a submitted line does: slash commands, the warnings opener, typed failures.

use rtwk::layout::SizedBox;
use rtwk::provider::read;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::commands::{DispatchCx, Outcome, SlashCommand, Target, dispatch};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, dialogs, popovers};
use codex_tui_core::threads::{Failure, ThreadModel};

use crate::app::view::Hs;
use crate::pages::session::open::open;

/// The chat screen of one thread's controllers.
use super::session_page::*;

/// The words of a typed failure.
pub(super) fn failure_text(strings: &crate::l10n::strings::Shared, failure: &Failure) -> String {
    match failure {
        Failure::Message(message) => strings.error(message).to_string(),
        Failure::TurnFailed => strings.chat.turn_failed.to_string(),
        Failure::Lagged(skipped) => strings.lagged(*skipped).to_string(),
        Failure::Stopped => strings.chat.server_stopped.to_string(),
        Failure::ReplyMismatch => strings.chat.reply_mismatch.to_string(),
    }
}

/// The composer's prompt, as wide as its text: a trailing blank is a cell.
pub(super) fn prompt(text: &'static str) -> AnyWidget {
    let width = i32::try_from(text.chars().count()).unwrap_or(2);
    SizedBox::new(Some(width), None, Some(Text::new(text).into())).into()
}

/// A line the user sent from the composer: a command opens its screen, asks
/// the session or says why not (the decision is core's `dispatch`); anything
/// else is a message for the model.
/// F2 and `/warnings`: the viewer over the retained warnings (with none it
/// shows its empty page, `No warnings`).
pub(super) fn open_warnings(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    cx.emit(
        &hs.screen,
        ScreenIntent::from(dialogs::Intent::OpenWarnings),
    );
    open(cx, Target::Dialogs);
}

pub(super) fn run_line(line: &str, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    // An empty line is nothing to run: the composer was already taken by the
    // pick or the submit that came with the same Enter.
    if line.trim().is_empty() {
        return;
    }
    let states = read::<ScreenStates, _>(cx);
    let thread = read::<ThreadModel, _>(cx);
    let session = read::<SessionSlice, _>(cx);
    let facts = DispatchCx {
        flags: states.chat.flags,
        task_running: thread.is_running(),
        side_conversation: states.chat.flags.side_conversation_active,
        // TODO(session-core): `review_mode` is the thread's code-review state
        // (`review.is_review_mode`); false until the session models review.
        review_mode: false,
        thread_started: matches!(session.status, Session::Ready),
        has_backend_auth: states.chat.signed_in,
    };
    let screen = |cx: &mut Cx<'_, ChatState>, intent: chat::Intent| {
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let record = |cx: &mut Cx<'_, ChatState>| {
        let intent = popovers::Intent::Record(line.to_string());
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let outcome = dispatch(line, &facts);
    if states.chat.notice.is_some() && !matches!(outcome, Outcome::Notice(_)) {
        screen(cx, chat::Intent::ClearNotice);
    }
    match outcome {
        Outcome::NotACommand => {
            // The decision is `apply`'s (R10): it starts a turn or queues
            // the message; the composer clears when it has.
            if !line.trim().is_empty() {
                record(cx);
                cx.emit(&hs.submit, line.to_string());
            }
        }
        Outcome::Open(opened) => {
            record(cx);
            w.ui.composer.clear();
            if opened.command == SlashCommand::Warnings {
                open_warnings(cx, hs);
            } else {
                open(cx, opened.target);
            }
        }
        Outcome::Session { command, intent } => {
            record(cx);
            w.ui.composer.clear();
            screen(cx, chat::Intent::Session { command, intent });
        }
        Outcome::Insert(text) => w.ui.composer.reset(text),
        Outcome::Quit => cx.emit(&hs.quit, ()),
        // The line stays in the composer to be corrected.
        Outcome::Notice(notice) => screen(cx, chat::Intent::Notice(notice)),
    }
}
