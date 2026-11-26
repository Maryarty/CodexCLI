#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/rename` (oracle `chatwidget/interaction.rs` `show_rename_prompt`, `app/thread_title.rs`):
//! a bare `/rename` wishes the name prompt over the thread, prefilled with its name; the prompt
//! says it is interested in the thread's title suggestion, which the feature asks the host for and
//! keeps while the prompt is open (the field, and whether the user typed, are the UI's); a submit
//! renames, an empty name is an error line, a side conversation cannot be renamed. The words
//! (`Rename thread`, `Name thread`, the labels) are the view's: these tests read the data.

use codex_app_server_protocol::{ServerNotification, ThreadNameUpdatedNotification};
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app, apply_app_from,
};
use codex_tui_core::commands::{CommandFlags, SessionIntent, SlashCommand};
use codex_tui_core::features::rename::{self, Interest, Note as RenameNote};
use codex_tui_core::features::{
    FeatureCommand, FeatureIntent, FeatureInterest, FeatureNote, FeatureReply, FeatureWish,
    InterestStep, Remote,
};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenEv, ScreenIntent, chat};
use codex_tui_core::threads::structured::Kind;
use codex_tui_core::threads::{Failure, Item, Note, NoteKey, ThreadAsk};
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::structured_turn::{
    TEMP, ends, ends_in, says, says_in, stream, the_ask, thread_asks, unsubscribes, walk, walk_in,
};

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
            intent: SessionIntent::Feature {
                args: name.unwrap_or_default().to_owned(),
            },
        }))),
    )
}

/// What the name prompt of the UI says when the user submits it.
fn submit(app: &mut CodexApp, name: &str) -> AppFx {
    let intent = rename::Intent::Set {
        thread: ThreadId::new(T),
        generation: generation(app),
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

fn generation(app: &CodexApp) -> rename::Gen {
    let Some(rename::Wish::Name { generation, .. }) = wishes(app).pop() else {
        panic!("the name prompt has a generation");
    };
    generation
}

fn interested(app: &CodexApp) -> FeatureInterest {
    FeatureInterest::Rename(Interest::Title {
        thread: ThreadId::new(T),
        generation: generation(app),
    })
}

/// The prompt is up: its interest in the title suggestion is the first, and asks the host.
fn open_prompt(app: &mut CodexApp) -> AppFx {
    let interest = interested(app);
    run(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(interest))),
    )
}

fn close_prompt(app: &mut CodexApp) {
    let interest = interested(app);
    run(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(interest))),
    );
}

/// What the feature knows of the suggestion for the prompt of the thread.
fn title(app: &CodexApp) -> Option<Remote<Option<String>>> {
    app.features()
        .rename
        .title(&ThreadId::new(T), generation(app))
        .cloned()
}

/// A conversation to name the thread by: one exchange.
fn talked(app: &mut CodexApp) {
    stream(
        app,
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "userMessage", "id": "u1", "clientId": null,
            "content": [{"type": "text", "text": "fix the login timeout", "text_elements": []}]}}}),
    );
    says_in(app, T, "turn-1", "I raised the timeout.");
}

/// An app whose thread has been talked in.
pub fn talked_app() -> CodexApp {
    let mut app = ready();
    talked(&mut app);
    app
}

/// The name prompt of the thread comes up: the run of the suggestion begins, and its first
/// step is what comes back.
pub fn open_title(app: &mut CodexApp) -> AppFx {
    rename(app, None);
    open_prompt(app)
}

/// A thread that has been talked in, its name prompt up: the run of the suggestion has begun.
fn prompted() -> (CodexApp, AppFx) {
    let mut app = talked_app();
    let fx = open_title(&mut app);
    (app, fx)
}

/// The run of the suggestion goes through to the end with the model's `answer`.
fn suggested(app: &mut CodexApp, first: &AppFx, answer: &str) -> AppFx {
    walk(app, first, json!({}));
    says(app, answer);
    ends(app, "completed")
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
            generation: 1,
            existing: "Old name".to_owned(),
        }]
    );
    assert_eq!(
        thread_asks(&fx),
        vec![],
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
            generation: 1,
            existing: String::new(),
        }]
    );
}

#[test]
fn the_prompt_being_up_asks_the_door_for_a_suggestion() {
    let (app, fx) = prompted();
    assert!(
        matches!(the_ask(&fx).1, ThreadAsk::McpServers { .. }),
        "{:?}",
        fx.commands()
    );
    assert_eq!(title(&app), Some(Remote::loading()));
}

#[test]
fn the_suggestion_asks_the_threads_own_model_read_only_with_the_recent_conversation() {
    let (mut app, fx) = prompted();
    let asked = walk(&mut app, &fx, json!({"docs": {"command": "x"}}));
    let [
        _,
        ThreadAsk::StartTemporary(temporary),
        ThreadAsk::StructuredTurn {
            thread,
            kind,
            prompt,
            schema,
            effort,
        },
    ] = asked.as_slice()
    else {
        panic!("the requests of a run: {asked:?}");
    };
    // no catalog and no ChatGPT account: the thread's own model, the turn's own effort
    assert_eq!(
        (temporary.kind, temporary.model.as_deref()),
        (Kind::Title, None)
    );
    assert_eq!(temporary.mcp_servers.keys().collect::<Vec<_>>(), ["docs"]);
    assert_eq!((thread.as_str(), *kind, effort), (TEMP, Kind::Title, &None));
    assert!(
        prompt.ends_with(
            "<conversation>\n<message role=\"user\">fix the login timeout</message>\n\
<message role=\"assistant\">I raised the timeout.</message>\n</conversation>"
        ),
        "{prompt}"
    );
    assert_eq!(schema["properties"]["title"]["maxLength"], json!(36));
}

/// The title model is asked for, with a low effort, on a ChatGPT account of the OpenAI provider
/// whose catalog has it.
#[test]
fn a_chatgpt_account_with_the_title_model_asks_for_it() {
    use codex_tui_core::screens::status::{self, Account, CatalogModel};
    let mut app = ready();
    talked(&mut app);
    let facts = chat::StatusFacts {
        model: Some("gpt-5.5".into()),
        provider: Some("openai".into()),
        ..chat::StatusFacts::default()
    };
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(facts)))),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Status(status::Ev::Account(Some(
            Account::ChatGpt {
                email: None,
                plan: None,
                workspace: None,
            },
        )))),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Status(status::Ev::Catalog(vec![CatalogModel {
            model: "gpt-5.6-luna".into(),
            display_name: "Luna".into(),
            default_effort: None,
        }]))),
    );
    rename(&mut app, None);
    let fx = open_prompt(&mut app);
    let asked = walk(&mut app, &fx, json!({}));
    let [
        _,
        ThreadAsk::StartTemporary(temporary),
        ThreadAsk::StructuredTurn { effort, .. },
    ] = asked.as_slice()
    else {
        panic!("the requests of a run: {asked:?}");
    };
    assert_eq!(temporary.model.as_deref(), Some("gpt-5.6-luna"));
    assert_eq!(
        effort,
        &Some(codex_protocol::openai_models::ReasoningEffort::Low)
    );
}

#[test]
fn the_suggestion_is_kept_for_the_prompt_and_answered_once() {
    let (mut app, fx) = prompted();
    let fx = suggested(&mut app, &fx, r#"{"title": "Fix the build."}"#);
    assert_eq!(unsubscribes(&fx), [TEMP], "the temporary thread is let go");
    assert_eq!(
        title(&app),
        Some(Remote::Ready(Some("Fix the build".to_owned())))
    );
    // answered once: a second message of the thread finds nothing waiting
    says(&mut app, r#"{"title": "Another"}"#);
    ends(&mut app, "completed");
    assert_eq!(
        title(&app),
        Some(Remote::Ready(Some("Fix the build".to_owned())))
    );
}

/// A title the schema does not allow, a run that fails, and a server that refuses are all an
/// answer without a title: the prompt keeps the name it has.
#[test]
fn a_generator_with_nothing_to_say_is_an_answer_without_a_title() {
    for answer in [
        "not json",
        r#"{"title": "  "}"#,
        r#"{"title": "a", "extra": 1}"#,
    ] {
        let (mut app, fx) = prompted();
        suggested(&mut app, &fx, answer);
        assert_eq!(title(&app), Some(Remote::Ready(None)), "{answer}");
    }
    let (mut app, fx) = prompted();
    walk(&mut app, &fx, json!({}));
    let fx = ends(&mut app, "failed");
    assert_eq!(unsubscribes(&fx), [TEMP]);
    assert_eq!(title(&app), Some(Remote::Ready(None)));
    let (mut app, fx) = prompted();
    let (ticket, _) = the_ask(&fx);
    crate::structured_turn::refuse(&mut app, ticket, "no server");
    assert_eq!(title(&app), Some(Remote::Ready(None)));
}

/// `suggest_thread_name`: a conversation with nothing to name it by has no suggestion, and the
/// door is not asked.
#[test]
fn a_conversation_with_nothing_to_name_it_by_has_no_suggestion() {
    let mut app = ready();
    rename(&mut app, None);
    let fx = open_prompt(&mut app);
    assert_eq!(thread_asks(&fx), vec![]);
    assert_eq!(title(&app), Some(Remote::Ready(None)));
    // commentary is not a substantive message
    stream(
        &mut app,
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": "c1", "text": "thinking", "phase": "commentary"}}}),
    );
    close_prompt(&mut app);
    rename(&mut app, None);
    let fx = open_prompt(&mut app);
    assert_eq!(thread_asks(&fx), vec![]);
    assert_eq!(title(&app), Some(Remote::Ready(None)));
}

/// `cloud_conversations_skip_tui_generated_titles`: the cloud owns the name of its conversations.
#[test]
fn a_cloud_conversation_has_no_suggestion() {
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::agents::Ev;
    const CLOUD: BackendId = BackendId(1);
    let mut app = ready();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::CloudBackend(CLOUD))),
    );
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    let mut fx = AppFx::default();
    let note: ServerNotification = serde_json::from_value(json!({"method": "item/completed",
        "params": {"threadId": "cloud-1", "turnId": "turn-1", "completedAtMs": 0,
        "startedAtMs": 0, "item": {"type": "userMessage", "id": "u1", "clientId": null,
        "content": [{"type": "text", "text": "fix it", "text_elements": []}]}}}))
    .unwrap();
    apply_app_from(
        &mut app,
        CLOUD,
        AppEv::Backend(BackendEv::Notification(Box::new(note))),
        &mut fx,
    );
    let cloud = FeatureInterest::Rename(Interest::Title {
        thread: ThreadId::new("cloud-1"),
        generation: 1,
    });
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(cloud))),
    );
    assert_eq!(
        thread_asks(&fx),
        vec![],
        "the cloud names its own conversations"
    );
    assert_eq!(
        app.features()
            .rename
            .title(&ThreadId::new("cloud-1"), 1)
            .cloned(),
        Some(Remote::Ready(None))
    );
}

#[test]
fn a_late_answer_after_the_prompt_closed_goes_nowhere() {
    let (mut app, fx) = prompted();
    walk(&mut app, &fx, json!({}));
    close_prompt(&mut app);
    assert_eq!(title(&app), None);
    says(&mut app, r#"{"title": "Late"}"#);
    let fx = ends(&mut app, "completed");
    assert_eq!(title(&app), None);
    assert_eq!(
        unsubscribes(&fx),
        [TEMP],
        "the temporary thread is let go all the same"
    );
}

/// O4: the answer of the first request is told from the second's by its gen: a prompt closed and
/// opened again before the first answer does not take the stale one.
#[test]
fn a_prompt_opened_again_does_not_take_the_earlier_answer() {
    let (mut app, first) = prompted();
    walk_in(&mut app, &first, json!({}), "temp-a", "turn-a");
    close_prompt(&mut app);
    let second = open_title(&mut app);
    walk_in(&mut app, &second, json!({}), "temp-b", "turn-b");
    says_in(&mut app, "temp-a", "turn-a", r#"{"title": "Stale"}"#);
    ends_in(&mut app, "temp-a", "turn-a", "completed");
    assert_eq!(title(&app), Some(Remote::loading()));
    says_in(&mut app, "temp-b", "turn-b", r#"{"title": "Fresh"}"#);
    ends_in(&mut app, "temp-b", "turn-b", "completed");
    assert_eq!(title(&app), Some(Remote::Ready(Some("Fresh".to_owned()))));
}

#[test]
fn closing_an_earlier_prompt_does_not_cancel_its_replacement() {
    let mut app = ready();
    open_title(&mut app);
    let earlier = interested(&app);
    open_title(&mut app);
    run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(earlier))),
    );
    assert_eq!(title(&app), Some(Remote::Ready(None)));
    let fx = submit(&mut app, "New name");
    assert_eq!(renamed_to(&fx), Some("New name".to_owned()));
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
    assert_eq!(thread_asks(&fx), vec![]);
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
