//! `/rename` (oracle `chatwidget/interaction.rs` `show_rename_prompt`, `app/thread_title.rs`
//! `suggest_thread_name`): the name prompt of the thread in use, the title the generator
//! suggests for it, and the rename a submit makes. The prompt is `dialogs::Open::Prompt`; the
//! field is the view's, so the suggestion fills it only while `Prompt::edited` is false.

use std::rc::Rc;

use super::session_note;
use crate::app::effects::{Ask, Done, command};
use crate::app::{AppFx, CodexApp, apply_thread};
use crate::commands::{Notice, SlashCommand};
use crate::ids::ThreadId;
use crate::screens::ScreenThreadIntent;
use crate::screens::chat;
use crate::screens::dialogs::prompt::{self, normalize_thread_name};
use crate::screens::dialogs::{Open, Prompt};
use crate::threads::{Note, ThreadEv};

/// `ensure_thread_rename_allowed`: a side conversation cannot be renamed.
pub(super) fn blocked(app: &CodexApp) -> bool {
    app.screens.chat.flags.side_conversation_active
}

/// A bare `/rename`: the prompt over the thread in use, prefilled with its name, and the ask
/// of a suggestion from its conversation.
pub(super) fn open(app: &mut CodexApp, source: Option<ThreadId>, fx: &mut AppFx) {
    let Some(thread) = source.filter(|id| app.threads.get(id).is_some()) else {
        let notice = Notice::SessionStarting(SlashCommand::Rename);
        return session_note(app, None, Note::from(notice), fx);
    };
    if blocked(app) {
        return session_note(app, Some(thread), Note::from(Notice::SideRename), fx);
    }
    let existing = app
        .threads
        .get(&thread)
        .and_then(|model| model.screen_slots().chat.name.clone());
    let dialogs = &mut Rc::make_mut(&mut app.screens).dialogs;
    let request = prompt::REQUEST_BIT | dialogs.next_stamp();
    dialogs.open = Some(Open::Prompt(Prompt::rename(
        thread.clone(),
        existing.as_deref(),
        Some(request),
    )));
    fx.commands
        .push(command(request, Ask::SuggestTitle { thread }));
}

/// The generator's answer to the prompt's ask; false when `request` is not the one it waits
/// for.
pub(in crate::app) fn suggested(app: &mut CodexApp, request: u64, done: &Done) -> bool {
    let waits = matches!(&app.screens.dialogs.open, Some(Open::Prompt(prompt)) if prompt.waits_for(request));
    let Done::Title(title) = done else {
        return false;
    };
    if !waits {
        return false;
    }
    if let Some(Open::Prompt(prompt)) = &mut Rc::make_mut(&mut app.screens).dialogs.open {
        prompt.suggested(title.clone());
    }
    true
}

/// The name the user submitted: the thread is renamed, an empty name is the error line.
pub(in crate::app) fn submitted(app: &mut CodexApp, thread: ThreadId, text: &str, fx: &mut AppFx) {
    match normalize_thread_name(text) {
        None => session_note(app, Some(thread), Note::from(Notice::ThreadNameEmpty), fx),
        Some(_) if blocked(app) => {
            session_note(app, Some(thread), Note::from(Notice::SideRename), fx)
        }
        Some(name) => {
            let intent = chat::ThreadIntent::Rename(name);
            let ev = ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(intent));
            apply_thread(app, &thread, ev, fx);
        }
    }
}
