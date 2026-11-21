//! What a submitted line does: slash commands, the warnings opener, typed failures.

use rtwk::layout::{EdgeInsets, Padding};
use rtwk::provider::read;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::commands::{DispatchCx, Notice, Outcome, SlashCommand, Target, dispatch};
use codex_tui_core::screens::composer::{self, ShellLine};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, dialogs, popovers};
use codex_tui_core::threads::{Failure, ThreadModel, UserMessage};

use crate::app::view::Hs;
use crate::pages::session::composer::clear_sent;
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
pub(super) fn prompt(text: &'static str, style: Style) -> AnyWidget {
    let label = text.trim_end_matches(' ');
    Padding::new(EdgeInsets::only(0, 0, 1, 0), Text::line(label).style(style)).into()
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
    // What the model takes whole: nothing over the backend's limit (Enter
    // refuses it and the draft stays, `prepare_submission_text`).
    if let Some(provided) = composer::too_large(line) {
        let notice = Notice::InputTooLarge { provided };
        screen(cx, chat::Intent::Notice(notice));
        return;
    }
    // A draft in shell mode runs as the user's own shell command (`!cmd`,
    // `submit_shell_command`), through the thread, a turn or none running;
    // before the first thread the app keeps it with the messages.
    if let Some(shell) = composer::shell_line(line, w.ui.draft.shell()) {
        // a thread that exists but is not taking input (no connection) keeps
        // the draft; with no thread yet the app queues it
        if !facts.thread_started && session.active.is_some() {
            return;
        }
        if states.chat.notice.is_some() {
            screen(cx, chat::Intent::ClearNotice);
        }
        match shell {
            ShellLine::Help => {
                clear_sent(&w.ui);
                screen(cx, chat::Intent::Notice(Notice::ShellHelp));
            }
            ShellLine::Run(command) => {
                record(cx);
                clear_sent(&w.ui);
                let intent = chat::Intent::Shell {
                    thread: session.active.clone(),
                    command,
                };
                screen(cx, intent);
            }
        }
        return;
    }
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
                // the draft's images go with its text; a line that is not
                // the draft (a popup's pick) goes as text
                let table = crate::pages::session::composer::strings::of(cx);
                let draft = w.ui.draft.submission(&w.ui.composer, |n| table.image(n));
                let message = if draft.text.trim() == line.trim() {
                    draft
                } else {
                    UserMessage::text(line)
                };
                cx.emit(&hs.submit, message);
            }
        }
        Outcome::Open(opened) => {
            record(cx);
            clear_sent(&w.ui);
            if matches!(opened.target, Target::Status | Target::Usage) {
                let intent = if opened.target == Target::Status {
                    ScreenIntent::Status(codex_tui_core::screens::status::Intent::Open)
                } else {
                    ScreenIntent::Usage(codex_tui_core::screens::usage::Intent::RefreshRateLimits)
                };
                cx.emit(&hs.screen, intent);
            }
            if opened.command == SlashCommand::Warnings {
                open_warnings(cx, hs);
            } else {
                open(cx, opened.target);
            }
        }
        Outcome::Session { command, intent } => {
            record(cx);
            clear_sent(&w.ui);
            screen(
                cx,
                chat::Intent::Session {
                    thread: session.active.clone(),
                    command,
                    intent,
                },
            );
        }
        Outcome::Insert(text) => w.ui.composer.reset(text),
        Outcome::Quit => cx.emit(&hs.quit, ()),
        // The line stays in the composer to be corrected.
        Outcome::Notice(notice) => screen(cx, chat::Intent::Notice(notice)),
    }
}
