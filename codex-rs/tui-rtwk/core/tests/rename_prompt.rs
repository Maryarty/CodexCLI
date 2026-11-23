#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/rename` (oracle `chatwidget/interaction.rs` `show_rename_prompt`, `app/thread_title.rs`):
//! a bare `/rename` opens the name prompt over the thread, prefilled with its name, and asks for
//! a title suggestion that fills the field only while the user has not typed; a submit renames,
//! an empty name is an error line, a side conversation cannot be renamed. The words (`Rename
//! thread`, `Name thread`, the labels) are the view's: these tests read the data.

use codex_app_server_protocol::{ServerNotification, ThreadNameUpdatedNotification};
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{CommandFlags, SessionIntent, SlashCommand};
use codex_tui_core::features::rename::{self, Note as RenameNote};
use codex_tui_core::features::{FeatureCommand, FeatureNote, FeaturePrompt, FeatureReply};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::dialogs::{self, Prompt, PromptKind, Suggestion};
use codex_tui_core::screens::{ScreenEv, ScreenIntent, chat};
use codex_tui_core::threads::{Failure, Item, Note, NoteKey};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn named(app: &mut CodexApp, name: &str) {
    let note = ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
        thread_id: T.to_owned(),
        thread_name: Some(name.to_owned()),
    });
    run(app, AppEv::Backend(BackendEv::Notification(Box::new(note))));
}

fn rename(app: &mut CodexApp, name: Option<&str>) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Rename,
            intent: SessionIntent::Rename {
                name: name.map(str::to_owned),
            },
        }))),
    )
}

fn dialog(app: &mut CodexApp, intent: dialogs::Intent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Dialogs(intent))),
    )
}

fn prompt(app: &CodexApp) -> Prompt {
    match &app.screens().dialogs.prompt {
        Some(prompt) => prompt.clone(),
        other => panic!("not the prompt: {other:?}"),
    }
}

/// The request of the suggestion the open asked for.
fn asked(fx: &AppFx) -> u64 {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::Effect {
                request,
                ask: Ask::SuggestTitle { thread },
                ..
            } if thread.as_str() == T => Some(*request),
            _ => None,
        })
        .expect("a SuggestTitle ask")
}

fn answer(app: &mut CodexApp, request: u64, title: Option<&str>) {
    let done = Done::Title(title.map(str::to_owned));
    run(app, AppEv::Effect { request, done });
}

fn renamed_to(fx: &AppFx) -> Option<String> {
    fx.commands().iter().find_map(|c| match c {
        Command::Feature {
            thread,
            command: FeatureCommand::Rename(rename::Command::SetName(name)),
            ..
        } if thread.as_str() == T => Some(name.clone()),
        _ => None,
    })
}

fn notices(app: &CodexApp) -> Vec<RenameNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(Note {
                key: NoteKey::Feature(FeatureNote::Rename(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

fn name_prompt(renaming: bool) -> PromptKind {
    PromptKind::Feature(FeaturePrompt::Rename(rename::Prompt::Name {
        thread: ThreadId::new(T),
        renaming,
    }))
}

#[test]
fn a_bare_rename_opens_the_prompt_with_the_name_and_asks_for_a_suggestion() {
    let mut app = ready();
    named(&mut app, "Old name");
    let fx = rename(&mut app, None);
    let open = prompt(&app);
    assert_eq!(open.kind, name_prompt(true), "the title says Rename thread");
    assert_eq!(open.initial, "Old name");
    assert_eq!(open.suggestion, Suggestion::Loading);
    assert!(!open.edited);
    asked(&fx);
}

#[test]
fn a_thread_without_a_name_starts_empty_and_is_named() {
    let mut app = ready();
    rename(&mut app, None);
    let open = prompt(&app);
    assert_eq!(open.initial, "");
    assert_eq!(open.kind, name_prompt(false), "the title says Name thread");
}

#[test]
fn the_suggestion_fills_the_field_while_the_user_has_not_typed() {
    let mut app = ready();
    let request = asked(&rename(&mut app, None));
    answer(&mut app, request, Some("Fix the build"));
    let open = prompt(&app);
    assert_eq!(
        open.suggestion,
        Suggestion::Applied("Fix the build".to_owned())
    );
    // answered once: a second answer finds nothing waiting
    answer(&mut app, request, Some("Another"));
    assert_eq!(
        prompt(&app).suggestion,
        Suggestion::Applied("Fix the build".to_owned())
    );
}

#[test]
fn typing_first_keeps_the_suggestion_out_of_the_field() {
    let mut app = ready();
    let request = asked(&rename(&mut app, None));
    dialog(&mut app, dialogs::Intent::Typed);
    assert!(prompt(&app).edited);
    answer(&mut app, request, Some("Fix the build"));
    assert_eq!(prompt(&app).suggestion, Suggestion::Idle);
}

#[test]
fn a_generator_with_nothing_to_say_clears_the_loading_line() {
    let mut app = ready();
    let request = asked(&rename(&mut app, None));
    answer(&mut app, request, None);
    assert_eq!(prompt(&app).suggestion, Suggestion::Idle);
}

#[test]
fn an_answer_to_another_request_is_not_the_prompts() {
    let mut app = ready();
    let request = asked(&rename(&mut app, None));
    answer(&mut app, request + 1, Some("Other"));
    assert_eq!(prompt(&app).suggestion, Suggestion::Loading);
}

#[test]
fn a_late_answer_after_the_prompt_closed_goes_nowhere() {
    let mut app = ready();
    let request = asked(&rename(&mut app, None));
    dialog(&mut app, dialogs::Intent::CancelPrompt);
    assert_eq!(app.screens().dialogs.prompt, None);
    answer(&mut app, request, Some("Late"));
    assert_eq!(app.screens().dialogs.prompt, None);
}

#[test]
fn a_submit_renames_the_thread_with_the_trimmed_name() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = dialog(
        &mut app,
        dialogs::Intent::Submit("  Fresh name \n".to_owned()),
    );
    assert_eq!(app.screens().dialogs.prompt, None, "the prompt closes");
    assert_eq!(renamed_to(&fx), Some("Fresh name".to_owned()));
    assert_eq!(app.screens().dialogs.submitted, None, "carried out once");
}

#[test]
fn an_empty_name_is_the_error_line_and_renames_nothing() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = dialog(&mut app, dialogs::Intent::Submit("   ".to_owned()));
    assert_eq!(renamed_to(&fx), None);
    assert_eq!(notices(&app), [RenameNote::NameEmpty]);
}

#[test]
fn a_side_conversation_cannot_be_renamed() {
    let mut app = ready();
    let flags = CommandFlags {
        side_conversation_active: true,
        ..CommandFlags::default()
    };
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Flags(flags))),
    );
    let fx = rename(&mut app, None);
    assert_eq!(app.screens().dialogs.prompt, None);
    assert!(
        fx.commands()
            .iter()
            .all(|c| !matches!(c, Command::Effect { .. }))
    );
    let fx = rename(&mut app, Some("Name"));
    assert_eq!(renamed_to(&fx), None);
    assert_eq!(
        notices(&app),
        [RenameNote::SideRename, RenameNote::SideRename]
    );
}

#[test]
fn a_rename_with_a_name_still_renames_directly() {
    let mut app = ready();
    let fx = rename(&mut app, Some("Direct"));
    assert_eq!(renamed_to(&fx), Some("Direct".to_owned()));
    assert_eq!(app.screens().dialogs.prompt, None);
}

/// The ticket of the thread's `thread/name/set` in `fx`.
fn rename_ticket(fx: &AppFx, thread: &str) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::Feature {
                ticket,
                thread: asked,
                command: FeatureCommand::Rename(rename::Command::SetName(_)),
                ..
            } if asked.as_str() == thread => Some(*ticket),
            _ => None,
        })
        .expect("a thread/name/set")
}

#[test]
fn a_new_conversation_with_a_name_is_named_once_it_has_started() {
    let mut app = ready();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::New,
            intent: SessionIntent::New {
                name: Some("Topic".to_owned()),
            },
        }))),
    );
    assert_eq!(renamed_to(&fx), None, "nothing to name before it starts");
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-2"))),
    );
    rename_ticket(&fx, "thread-2");
}

#[test]
fn a_rename_the_server_refused_is_the_threads_failure() {
    let mut app = ready();
    let fx = rename(&mut app, Some("Direct"));
    let ticket = rename_ticket(&fx, T);
    assert_eq!(app.active_thread().last_error(), None);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("no such thread")),
    );
    assert_eq!(
        app.active_thread().last_error(),
        Some(&Failure::Message("no such thread".to_owned()))
    );
}

#[test]
fn an_accepted_rename_leaves_no_failure() {
    let mut app = ready();
    let fx = rename(&mut app, Some("Direct"));
    let ticket = rename_ticket(&fx, T);
    let ack = Reply::feature(FeatureReply::Rename(rename::Reply::Ack));
    run(&mut app, AppEv::Reply(ticket, ack));
    assert_eq!(app.active_thread().last_error(), None);
}
