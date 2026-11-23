//! The settings dialogs' app side (S15). What a dialog needs from another screen's state is built
//! here (the status-line and title setups read the chat's status facts); what a dialog saved is
//! written to the config file (`config/batchWrite`, as the oracle's `config_update.rs` does) and
//! what the session does once the file has it (the chat's status facts, the facts the dialogs
//! open from) follows the answer, so a refused write leaves everything as it was.
//!
//! NOT DECLARED YET: `core/src/app.rs` (claimed by another task) needs `mod settings;`, the
//! `AppCtx::Settings(settings::Ctx)` variant with its answer arm and the intent route
//! (`hub_budget.txt` AppCtx 13 -> 14); `select::Setting` needs `Theme`, `StatusLine`, `Title`,
//! `Memories` and `pages/session/strings.rs` their words. The compiled and green patch is the
//! block's Need (checkpoint).

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::select;
use crate::screens::settings::{self, Dialog, Ev, Intent, Open, persist};
use crate::screens::{ScreenCommand, ScreenReply, chat};

/// An intent of the settings dialogs, and the writes it left to make.
pub(super) fn intent(app: &mut CodexApp, intent: Intent, fx: &mut AppFx) {
    let screens = Rc::make_mut(&mut app.screens);
    let built = match intent {
        Intent::Open(Dialog::StatusLine) => {
            Some(settings::items::status_line(&screens.chat.status))
        }
        Intent::Open(Dialog::Title) => Some(settings::items::title(&screens.chat.status)),
        _ => None,
    };
    match built {
        Some(setup) => settings::apply(&mut screens.settings, Ev::Opened(Open::Setup(setup)), fx),
        None => settings::apply_intent(&mut screens.settings, intent, fx),
    }
    write(app, fx);
}

/// The edits the dialogs recorded since the last call, each written as one `config/batchWrite`.
fn write(app: &mut CodexApp, fx: &mut AppFx) {
    let edits = Rc::make_mut(&mut app.screens).settings.take_edits();
    for write in edits.iter().filter_map(persist::write_of) {
        let ticket = app.mint.next();
        let command = settings::Command::Write {
            edits: write.edits.clone(),
        };
        app.outgoing
            .insert(ticket, AppCtx::Settings(settings::Ctx::Write(write)));
        fx.commands.push(Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Settings(command),
        });
    }
}

/// What the write of `subject` was for, as the words of its failure line name it.
fn setting_of(subject: &persist::Subject) -> select::Setting {
    match subject {
        persist::Subject::Theme(_) => select::Setting::Theme,
        persist::Subject::StatusLine { .. } => select::Setting::StatusLine,
        persist::Subject::Title(_) => select::Setting::Title,
        persist::Subject::Memories(_) => select::Setting::Memories,
    }
}

/// A line of the transcript of the thread in use (`add_error_message`, `add_warning_message`).
fn say(app: &mut CodexApp, note: select::Note, fx: &mut AppFx) {
    if !note.in_transcript() {
        return;
    }
    let thread = app.session.active.clone();
    let note = crate::threads::Note::from(crate::threads::NoteKey::Select(note));
    super::session_note(app, thread, note, fx);
}

fn text(error: RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => text,
        RpcError::Mismatch => String::new(),
        RpcError::Provider(error) => error.to_string(),
    }
}

/// The answer of a write.
pub(super) fn answer(app: &mut CodexApp, ctx: settings::Ctx, answer: Answer, fx: &mut AppFx) {
    let settings::Ctx::Write(write) = ctx;
    let subject = write.subject;
    let setting = setting_of(&subject);
    let result = match answer {
        Answer::Screen(ScreenReply::Settings(settings::Reply::Written { overridden })) => {
            Ok(overridden)
        }
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    match result {
        Ok(overridden) => {
            saved(app, subject, fx);
            if overridden {
                say(app, select::Note::Overridden(setting), fx);
            }
        }
        Err(error) => {
            let screens = Rc::make_mut(&mut app.screens);
            settings::apply(&mut screens.settings, Ev::SaveFailed(subject), fx);
            let error = text(error);
            say(app, select::Note::SaveFailed { setting, error }, fx);
        }
    }
}

/// The file has what the dialog saved: the facts the session reads follow (`StatusLineSetup`,
/// `TerminalTitleSetup`: the chat takes the items once they are saved).
fn saved(app: &mut CodexApp, subject: persist::Subject, fx: &mut AppFx) {
    let screens = Rc::make_mut(&mut app.screens);
    let mut facts = (*screens.chat.status).clone();
    match &subject {
        persist::Subject::StatusLine {
            items,
            use_theme_colors,
        } => {
            facts.items = Some(items.clone());
            facts.colors = *use_theme_colors;
            chat::apply(&mut screens.chat, chat::Ev::Status(Box::new(facts)), fx);
        }
        persist::Subject::Title(items) => {
            facts.terminal_title = Some(items.clone());
            chat::apply(&mut screens.chat, chat::Ev::Status(Box::new(facts)), fx);
        }
        persist::Subject::Theme(_) | persist::Subject::Memories(_) => {}
    }
    settings::apply(&mut screens.settings, Ev::Saved(subject), fx);
}
