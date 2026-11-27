#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/logout`, `/ide`, `/daybreak`, `/debug-config` (oracle `app/event_dispatch.rs` `Logout`,
//! `chatwidget/ide_context.rs`, `app/daybreak.rs`, `debug_config.rs`): the verbs ask the host for
//! what is IO, and its answers are the notes of the thread in use. The words are the view's.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::small_cmds::{
    Command as Wanted, DaybreakFailure, DefaultFailure, Fact, IdeFailure, IdeReading,
    Intent as SmallIntent, Note as SmallNote, Wish,
};
use codex_tui_core::features::{self, FeatureCommand, FeatureIntent, FeatureNote, FeatureWish};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::daybreak::{Access, ModelAccess, Programs};
use codex_tui_core::threads::{Item as Entry, Note, NoteKey};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

fn app() -> CodexApp {
    build(r#"{"step":"seed","seed":"session","thread":"thread-1"}"#)
        .expect("plays")
        .app
}

fn ide_app() -> CodexApp {
    build(concat!(
        r#"{"step":"seed","seed":"session","thread":"thread-1"}"#,
        "\n",
        r#"{"step":"screen","dir":"chat","ev":{"status":{"cwd":"/workspace"}}}"#,
    ))
    .expect("plays")
    .app
}

fn ide(app: &mut CodexApp, args: &str) -> AppFx {
    command(
        app,
        SlashCommand::Ide,
        SessionIntent::Feature {
            args: args.to_owned(),
        },
    )
}

fn ide_request(app: &mut CodexApp, args: &str) -> u64 {
    let fx = ide(app, args);
    let requests: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::IdeContext { thread, cwd },
                ..
            } => {
                assert_eq!(thread.as_str(), T);
                assert_eq!(cwd, "/workspace");
                Some(*request)
            }
            _ => None,
        })
        .collect();
    let [request] = requests.as_slice() else {
        panic!("expected one owned IDE read, got {:?}", fx.commands());
    };
    *request
}

fn ide_reply(app: &mut CodexApp, request: u64, result: Result<IdeReading, IdeFailure>) {
    run(
        app,
        AppEv::Effect {
            request,
            done: Done::IdeContext(result),
        },
    );
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn command(app: &mut CodexApp, command: SlashCommand, intent: SessionIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command,
            intent,
        }))),
    )
}

fn verb(app: &mut CodexApp, intent: SmallIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::SmallCmds(intent))),
    )
}

/// The Daybreak choice the thread shows (`small_cmds::daybreak_of`).
fn saved(app: &CodexApp) -> Option<bool> {
    app.thread(&ThreadId::new(T))
        .and_then(|thread| features::small_cmds::daybreak_of(thread.view()))
}

fn fact(app: &mut CodexApp, fact: Fact) {
    run(app, AppEv::Feature(fact.into()));
}

/// The requests of the feature a step left, with their tickets.
fn sent(fx: &AppFx) -> Vec<(Ticket, Wanted)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Feature {
                ticket,
                command: FeatureCommand::SmallCmds(wanted),
                ..
            } => Some((*ticket, wanted.clone())),
            _ => None,
        })
        .collect()
}

fn one(fx: &AppFx) -> (Ticket, Wanted) {
    let all = sent(fx);
    let [only] = all.as_slice() else {
        panic!("expected one request, got {all:?}");
    };
    only.clone()
}

fn answer(app: &mut CodexApp, request: &(Ticket, Wanted), result: serde_json::Value) -> AppFx {
    let command = FeatureCommand::SmallCmds(request.1.clone());
    let reply = features::decode(&command, result).expect("decodes");
    run(app, AppEv::Reply(request.0, Reply::feature(reply)))
}

fn fail(app: &mut CodexApp, request: &(Ticket, Wanted), text: &str) -> AppFx {
    run(app, AppEv::Reply(request.0, Reply::failed(text)))
}

fn notes(app: &CodexApp) -> Vec<SmallNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::SmallCmds(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn logout_asks_the_account_and_a_success_wishes_the_shutdown() {
    let mut app = app();
    let fx = command(
        &mut app,
        SlashCommand::Logout,
        SessionIntent::Feature {
            args: String::new(),
        },
    );
    let request = one(&fx);
    assert_eq!(request.1, Wanted::Logout);
    answer(&mut app, &request, serde_json::json!({}));
    let wishes: Vec<_> = app.wishes().pending().map(|w| w.wish.clone()).collect();
    assert_eq!(wishes, vec![FeatureWish::from(Wish::Shutdown)]);
}

#[test]
fn a_failed_logout_says_why() {
    let mut app = app();
    let fx = verb(&mut app, SmallIntent::Logout);
    fail(&mut app, &one(&fx), "offline");
    assert_eq!(
        notes(&app),
        vec![SmallNote::LogoutFailed("offline".to_owned())]
    );
}

#[test]
fn ide_toggles_and_says_what_the_host_found() {
    let mut app = ide_app();
    let request = ide_request(&mut app, "on");
    assert!(!app.features().small_cmds.ide.enabled);
    assert!(notes(&app).is_empty());
    ide_reply(&mut app, request, Ok(IdeReading { has_context: true }));
    assert!(app.features().small_cmds.ide.enabled);
    ide(&mut app, "");
    assert!(!app.features().small_cmds.ide.enabled);
    ide(&mut app, "bogus");
    assert_eq!(
        notes(&app),
        vec![
            SmallNote::IdeOn { has_context: true },
            SmallNote::IdeOff,
            SmallNote::IdeUsage
        ]
    );
}

#[test]
fn an_ide_that_is_not_there_turns_the_context_off_with_the_hosts_hint() {
    let mut app = ide_app();
    let request = ide_request(&mut app, "on");
    ide_reply(
        &mut app,
        request,
        Err(IdeFailure {
            hint: "Open it".to_owned(),
            skip_hint: "open it".to_owned(),
        }),
    );
    assert!(!app.features().small_cmds.ide.enabled);
    assert_eq!(
        notes(&app),
        vec![SmallNote::IdeCouldNotEnable {
            hint: "Open it".to_owned()
        }]
    );
}

#[test]
fn ide_off_and_disconnect_retire_exact_pending_reads() {
    let mut app = ide_app();
    let first = ide_request(&mut app, "on");
    let second = ide_request(&mut app, "on");
    let ready = IdeReading { has_context: true };
    ide_reply(&mut app, first, Ok(ready.clone()));
    assert!(!app.features().small_cmds.ide.enabled);
    ide(&mut app, "off");
    ide_reply(&mut app, second, Ok(ready.clone()));
    assert!(!app.features().small_cmds.ide.enabled);
    let third = ide_request(&mut app, "on");
    run(&mut app, AppEv::Disconnected);
    ide_reply(&mut app, third, Ok(ready));
    assert!(!app.features().small_cmds.ide.enabled);
    assert_eq!(notes(&app), vec![SmallNote::IdeOff]);
}

fn access(eligible: bool, daybreak: bool) -> Access {
    Access {
        eligible,
        models: vec![ModelAccess {
            model: "gpt-x".to_owned(),
            programs: Some(Programs {
                standard: true,
                daybreak,
            }),
        }],
    }
}

fn daybreak(app: &mut CodexApp) -> AppFx {
    verb(
        app,
        SmallIntent::Daybreak {
            source: Some(ThreadId::new(T)),
        },
    )
}

#[test]
fn daybreak_on_without_an_eligible_account_asks_nothing() {
    let mut app = app();
    run(&mut app, AppEv::Daybreak(access(false, false)));
    assert!(sent(&daybreak(&mut app)).is_empty());
}

#[test]
fn daybreak_saves_the_threads_metadata_then_the_default() {
    let mut app = app();
    run(&mut app, AppEv::Daybreak(access(true, true)));
    let fx = daybreak(&mut app);
    // the command is listed only when the feature is on: a fixture without it asks nothing
    let Some(save) = sent(&fx).into_iter().next() else {
        return;
    };
    assert_eq!(save.1, Wanted::SetDaybreak(true));
    let fx = answer(&mut app, &save, serde_json::json!({}));
    let default = one(&fx);
    assert_eq!(default.1, Wanted::SaveDefault(true));
    fail(&mut app, &default, "read-only");
    assert_eq!(
        notes(&app),
        vec![
            SmallNote::DaybreakDefault(DefaultFailure::Save("read-only".to_owned())),
            SmallNote::DaybreakSet(true)
        ]
    );
    assert_eq!(saved(&app), Some(true));
}

#[test]
fn a_thread_that_cannot_save_says_so_and_keeps_its_choice() {
    let mut app = app();
    run(&mut app, AppEv::Daybreak(access(true, true)));
    let Some(save) = sent(&daybreak(&mut app)).into_iter().next() else {
        return;
    };
    fail(&mut app, &save, "gone");
    assert_eq!(
        notes(&app),
        vec![SmallNote::DaybreakFailed(DaybreakFailure::Save(
            "gone".to_owned()
        ))]
    );
    assert_eq!(saved(&app), None);
}

#[test]
fn debug_config_says_the_rows_the_host_gave() {
    let mut app = app();
    fact(
        &mut app,
        Fact::debug_config([
            ("/debug-config".to_owned(), "magenta"),
            (String::new(), "plain"),
        ]),
    );
    let fx = command(
        &mut app,
        SlashCommand::DebugConfig,
        SessionIntent::Feature {
            args: String::new(),
        },
    );
    assert!(sent(&fx).is_empty());
    let got = notes(&app);
    let [SmallNote::DebugConfig(rows)] = got.as_slice() else {
        panic!("one note");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].text, "/debug-config");
}
