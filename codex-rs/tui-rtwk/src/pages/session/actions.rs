//! What a submitted line does: slash commands, the warnings opener, typed failures.

use rtwk::layout::{EdgeInsets, Padding};
use rtwk::provider::read;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::commands::{
    DispatchCx, Notice, Outcome, SessionIntent, SlashCommand, Target, dispatch_with_tiers, parent_owned_takes,
};
use codex_tui_core::screens::composer::{self, ShellLine};
use codex_tui_core::screens::{
    ScreenIntent, ScreenStates, ScreenThreadIntent, chat, dialogs, popovers,
};
use codex_tui_core::threads::{Failure, Note, NoteKey, ThreadModel, UserMessage};

use crate::app::view::Hs;
use crate::pages::session::composer::clear_sent;
use crate::pages::session::open::{open, open_confirmation};

/// The chat screen of one thread's controllers.
use super::session_page::*;

/// The words of a typed failure.
pub(super) fn failure_text(strings: &crate::l10n::strings::Shared, failure: &Failure) -> String {
    match failure {
        Failure::Message(message) => strings.error(message).to_string(),
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

/// What a line that is a message for the model does while a turn runs:
/// Enter steers it (`apply` decides), Tab queues it behind the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Delivery {
    Submit,
    Queue,
}

pub(super) fn run_line(line: &str, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs, send: Delivery) {
    // An empty line is nothing to run: the composer was already taken by the
    // pick or the submit that came with the same Enter.
    if line.trim().is_empty() {
        return;
    }
    let states = read::<ScreenStates, _>(cx);
    let thread = read::<ThreadModel, _>(cx);
    let session = read::<SessionSlice, _>(cx);
    // the commands the current model's service tiers add (`/fast`)
    let thread_model = thread.screen_slots().chat.model.clone();
    let tiers = codex_tui_core::screens::chat::facts::tier_commands(
        &states.chat.status,
        thread_model.as_deref(),
        &states.select.catalog,
        &states.chat.service_tiers,
    );
    let mut flags = states.chat.flags;
    flags.service_tier_commands_enabled = !tiers.is_empty();
    let facts = DispatchCx {
        flags,
        task_running: thread.is_running(),
        side_conversation: states.chat.flags.side_conversation_active,
        review_mode: thread.screen_slots().chat.review_mode(),
        thread_started: matches!(session.status, Session::Ready),
        has_backend_auth: states.chat.signed_in,
    };
    let screen = |cx: &mut Cx<'_, ChatState>, intent: chat::Intent| {
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    // What the line answered with is a note of the thread it was sent in.
    let say = |cx: &mut Cx<'_, ChatState>, note: Note| {
        let intent = chat::Intent::Note {
            thread: session.active.clone(),
            note,
        };
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let record = |cx: &mut Cx<'_, ChatState>| {
        let intent = popovers::Intent::Record(line.to_string());
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    // A thread its parent controls takes the commands that do not reach the
    // agent and nothing else; the composer keeps the line
    // (`InputResult::ParentOwnedInputBlocked`, `handle_parent_owned_submission`).
    if thread.screen_slots().chat.parent_owned && !parent_owned_takes(line, flags) {
        say(cx, Note::from(NoteKey::ParentOwned));
        return;
    }
    // What the model takes whole: nothing over the backend's limit (Enter
    // refuses it and the draft stays, `prepare_submission_text`).
    if let Some(provided) = composer::too_large(line) {
        say(cx, Note::from(Notice::InputTooLarge { provided }));
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
        match shell {
            ShellLine::Help => {
                clear_sent(&w.ui);
                say(cx, Note::from(Notice::ShellHelp));
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
    let outcome = dispatch_with_tiers(line, &facts, &tiers);
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
                match send {
                    Delivery::Submit => cx.emit(&hs.submit, message),
                    Delivery::Queue => {
                        let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(message));
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                }
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
            // The settings dialogs are built by the model from its facts
            // (the dialog the command names), then shown by `open`.
            if let Some(dialog) = settings_dialog(opened.command) {
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Settings(codex_tui_core::screens::settings::Intent::Open(dialog)),
                );
            }
            // The confirmation of `/archive` and `/delete` is the model's
            // question (is a turn running?), then the select sheet shows it.
            if let Some(intent) = removal_intent(opened.command) {
                screen(
                    cx,
                    chat::Intent::Session {
                        thread: session.active.clone(),
                        command: opened.command,
                        intent,
                    },
                );
                open_confirmation(cx);
                return;
            }
            // `/model`, `/permissions` and `/apps` fill the selection dialog
            // from the thread's facts, and the app asks the server for the rest.
            if let Some(intent) = select_intent(
                opened.command,
                &states.chat.status,
                states.chat.flags.connectors_enabled,
            ) {
                cx.emit(&hs.screen, ScreenIntent::Select(intent));
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
        Outcome::Tier(tier) => {
            record(cx);
            clear_sent(&w.ui);
            // the tier the thread runs on now, else the config's
            let current = match &thread.screen_slots().chat.service_tier {
                Some(tier) => tier.clone(),
                None => states.chat.status.service_tier.clone(),
            };
            let intent = codex_tui_core::screens::select::Intent::ServiceTier {
                id: tier.id,
                current,
            };
            cx.emit(&hs.screen, ScreenIntent::Select(intent));
        }
        Outcome::Insert(text) => w.ui.composer.reset(text),
        Outcome::Quit => cx.emit(&hs.quit, ()),
        // What the composer does with the line is the oracle's: it keeps an
        // unknown command and one refused while a task runs, recalls the rest.
        Outcome::Notice(notice) => {
            if notice.is_recalled() {
                record(cx);
            }
            match (notice.keeps_draft(), notice.is_recalled()) {
                (false, _) => clear_sent(&w.ui),
                (true, true) => w.ui.draft.recorded(&w.ui.composer),
                (true, false) => {}
            }
            say(cx, Note::from(notice));
        }
    }
}

/// What the model is asked before the select sheet shows the confirmation of
/// `/archive` or `/delete`.
fn removal_intent(command: SlashCommand) -> Option<SessionIntent> {
    match command {
        SlashCommand::Archive => Some(SessionIntent::Archive),
        SlashCommand::Delete => Some(SessionIntent::Delete),
        _ => None,
    }
}

/// The menu a slash command opens in the selection dialog, built from the
/// thread's facts and whether the apps feature is on (the others open their slot empty until they land).
fn select_intent(
    command: SlashCommand,
    facts: &chat::StatusFacts,
    apps: bool,
) -> Option<codex_tui_core::screens::select::Intent> {
    use codex_tui_core::screens::select::{Intent, models, permissions};
    match command {
        SlashCommand::Model => Some(Intent::OpenModels(models::Current::from_facts(facts))),
        SlashCommand::Permissions => Some(Intent::OpenPermissions(
            permissions::Current::from_facts(facts),
        )),
        SlashCommand::Apps => Some(Intent::OpenApps { enabled: apps }),
        _ => None,
    }
}

/// The settings dialog a slash command opens, when the model builds it
/// (the others open their slot empty until their sheet lands).
fn settings_dialog(command: SlashCommand) -> Option<codex_tui_core::screens::settings::Dialog> {
    use codex_tui_core::screens::settings::Dialog;
    match command {
        SlashCommand::Theme => Some(Dialog::Theme),
        SlashCommand::Memories => Some(Dialog::Memories),
        _ => None,
    }
}
