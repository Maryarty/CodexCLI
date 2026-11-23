#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/rename` (oracle `chatwidget/interaction.rs` `show_rename_prompt`, `app/thread_title.rs`):
//! a bare `/rename` wishes the name prompt over the thread, prefilled with its name; the prompt
//! says it is interested in the thread's title suggestion, which the feature asks the host for and
//! keeps while the prompt is open (the field, and whether the user typed, are the UI's); a submit
//! renames, an empty name is an error line, a side conversation cannot be renamed. The words
//! (`Rename thread`, `Name thread`, the labels) are the view's: these tests read the data.

use codex_app_server_protocol::{ServerNotification, ThreadNameUpdatedNotification};
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{CommandFlags, SessionIntent, SlashCommand};
use codex_tui_core::features::rename::{self, Interest, Note as RenameNote};
use codex_tui_core::features::{
    FeatureCommand, FeatureIntent, FeatureInterest, FeatureNote, FeatureReply, FeatureWish,
    InterestStep, Remote,
};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
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

/// What the name prompt of the UI says when the user submits it.
fn submit(app: &mut CodexApp, name: &str) -> AppFx {
    let intent = rename::Intent::Set {
        thread: ThreadId::new(T),
        name: name.to_owned(),
    };
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Rename(intent))),
    )
}

/// The name prompts the rename wishes the surface to show.
fn wishes(app: &CodexApp) -> Vec<rename::Wish> {
    app.wishes()
        .pending()
        .filter_map(|wish| match &wish.wish {
            FeatureWish::Rename(wish) => Some(wish.clone()),
            _ => None,
        })
        .collect()
}

fn interested() -> FeatureInterest {
    FeatureInterest::Rename(Interest::Title {
        thread: ThreadId::new(T),
    })
}

/// The prompt is up: its interest in the title suggestion is the first, and asks the host.
fn open_prompt(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(interested()))),
    )
}

/// What the feature knows of the suggestion for the prompt of the thread.
fn title(app: &CodexApp) -> Option<Remote<Option<String>>> {
    app.features().rename.title(&ThreadId::new(T)).cloned()
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

#[test]
fn a_bare_rename_wishes_the_prompt_with_the_name() {
    let mut app = ready();
    named(&mut app, "Old name");
    let fx = rename(&mut app, None);
    assert_eq!(
        wishes(&app),
        [rename::Wish::Name {
            thread: ThreadId::new(T),
            existing: "Old name".to_owned(),
        }]
    );
    assert!(
        fx.commands()
            .iter()
            .all(|c| !matches!(c, Command::Effect { .. })),
        "the suggestion is asked for when the prompt is up, not before"
    );
}

#[test]
fn a_thread_without_a_name_wishes_an_empty_field() {
    let mut app = ready();
    rename(&mut app, None);
    assert_eq!(
        wishes(&app),
        [rename::Wish::Name {
            thread: ThreadId::new(T),
            existing: String::new(),
        }]
    );
}

#[test]
fn the_prompt_being_up_asks_for_a_suggestion() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = open_prompt(&mut app);
    asked(&fx);
    assert_eq!(title(&app), Some(Remote::loading()));
}

#[test]
fn the_suggestion_is_kept_for_the_prompt_and_answered_once() {
    let mut app = ready();
    rename(&mut app, None);
    let request = asked(&open_prompt(&mut app));
    answer(&mut app, request, Some("Fix the build"));
    assert_eq!(
        title(&app),
        Some(Remote::Ready(Some("Fix the build".to_owned())))
    );
    // answered once: a second answer finds nothing waiting
    answer(&mut app, request, Some("Another"));
    assert_eq!(
        title(&app),
        Some(Remote::Ready(Some("Fix the build".to_owned())))
    );
}

#[test]
fn a_generator_with_nothing_to_say_is_an_answer_without_a_title() {
    let mut app = ready();
    rename(&mut app, None);
    let request = asked(&open_prompt(&mut app));
    answer(&mut app, request, None);
    assert_eq!(title(&app), Some(Remote::Ready(None)));
}

#[test]
fn an_answer_to_another_request_is_not_the_prompts() {
    let mut app = ready();
    rename(&mut app, None);
    let request = asked(&open_prompt(&mut app));
    answer(&mut app, request + 1, Some("Other"));
    assert_eq!(title(&app), Some(Remote::loading()));
}

#[test]
fn a_late_answer_after_the_prompt_closed_goes_nowhere() {
    let mut app = ready();
    rename(&mut app, None);
    let request = asked(&open_prompt(&mut app));
    run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(interested()))),
    );
    assert_eq!(title(&app), None);
    answer(&mut app, request, Some("Late"));
    assert_eq!(title(&app), None);
}

/// O4: the answer of the first request is told from the second's by its gen: a prompt closed and
/// opened again before the first answer does not take the stale one.
#[test]
fn a_prompt_opened_again_does_not_take_the_earlier_answer() {
    let mut app = ready();
    rename(&mut app, None);
    let first = asked(&open_prompt(&mut app));
    run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(interested()))),
    );
    let second = asked(&open_prompt(&mut app));
    assert!(first != second);
    answer(&mut app, first, Some("Stale"));
    assert_eq!(title(&app), Some(Remote::loading()));
    answer(&mut app, second, Some("Fresh"));
    assert_eq!(title(&app), Some(Remote::Ready(Some("Fresh".to_owned()))));
}

#[test]
fn a_submit_renames_the_thread_with_the_trimmed_name() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = submit(&mut app, "  Fresh name \n");
    assert_eq!(renamed_to(&fx), Some("Fresh name".to_owned()));
}

#[test]
fn an_empty_name_is_the_error_line_and_renames_nothing() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = submit(&mut app, "   ");
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
    assert_eq!(wishes(&app), []);
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
    assert_eq!(wishes(&app), []);
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
