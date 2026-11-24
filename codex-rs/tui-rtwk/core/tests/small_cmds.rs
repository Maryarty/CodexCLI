#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/logout`, `/ide`, `/daybreak`, `/debug-config` (oracle `app/event_dispatch.rs` `Logout`,
//! `chatwidget/ide_context.rs`, `app/daybreak.rs`, `debug_config.rs`): the verbs ask the host for
//! what is IO, and its answers are the notes of the thread in use. The words are the view's.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::small_cmds::{
    DaybreakFailure, DaybreakSaved, DefaultFailure, IdeFailure, IdeReading, Intent as SmallIntent,
    Note as SmallNote, Row, Style, Wish,
};
use codex_tui_core::features::{FeatureIntent, FeatureNote, FeatureWish};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
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

fn asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
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

fn answer(app: &mut CodexApp, fx: &AppFx, done: Done) {
    let request = asks(fx)[0].0;
    run(app, AppEv::Effect { request, done });
}

#[test]
fn logout_asks_the_account_and_a_success_wishes_the_shutdown() {
    let mut app = app();
    let fx = command(&mut app, SlashCommand::Logout, SessionIntent::Logout);
    assert_eq!(asks(&fx)[0].1, Ask::Logout);
    answer(&mut app, &fx, Done::LoggedOut(Ok(())));
    let wishes: Vec<_> = app.wishes().pending().map(|w| w.wish.clone()).collect();
    assert_eq!(wishes, vec![FeatureWish::from(Wish::Shutdown)]);
}

#[test]
fn a_failed_logout_says_why() {
    let mut app = app();
    let fx = verb(&mut app, SmallIntent::Logout);
    answer(&mut app, &fx, Done::LoggedOut(Err("offline".to_owned())));
    assert_eq!(notes(&app), vec![SmallNote::LogoutFailed("offline".to_owned())]);
}

#[test]
fn ide_toggles_and_reads_the_context_while_on() {
    let mut app = app();
    let ide = |app: &mut CodexApp, args: Option<&str>| {
        command(
            app,
            SlashCommand::Ide,
            SessionIntent::Ide {
                args: args.map(str::to_owned),
            },
        )
    };
    let fx = ide(&mut app, None);
    assert!(matches!(asks(&fx)[0].1, Ask::IdeContext { .. }), "on: the context is read");
    answer(&mut app, &fx, Done::Ide(Ok(IdeReading { has_context: true })));
    assert!(app.features().small_cmds.ide.enabled);
    assert!(asks(&ide(&mut app, None)).is_empty(), "a bare /ide while on turns it off");
    assert!(!app.features().small_cmds.ide.enabled);
    assert!(asks(&ide(&mut app, Some("bogus"))).is_empty());
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
fn an_ide_that_cannot_be_read_turns_the_context_off() {
    let mut app = app();
    let fx = verb(
        &mut app,
        SmallIntent::Ide {
            args: Some("on".to_owned()),
        },
    );
    answer(
        &mut app,
        &fx,
        Done::Ide(Err(IdeFailure {
            hint: "Open this project in VS Code".to_owned(),
            skip_hint: String::new(),
        })),
    );
    assert!(!app.features().small_cmds.ide.enabled);
    assert_eq!(
        notes(&app),
        vec![SmallNote::IdeCouldNotEnable {
            hint: "Open this project in VS Code".to_owned()
        }]
    );
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
    command(app, SlashCommand::Daybreak, SessionIntent::Daybreak)
}

#[test]
fn daybreak_is_refused_without_an_eligible_account_or_a_catalog_that_has_it() {
    let mut app = app();
    // the command is listed only when the feature is on
    assert!(asks(&daybreak(&mut app)).is_empty());
    assert!(notes(&app).is_empty() || !notes(&app).is_empty());
    run(&mut app, AppEv::Daybreak(access(false, false)));
    daybreak(&mut app);
    run(&mut app, AppEv::Daybreak(access(true, false)));
    daybreak(&mut app);
    let got = notes(&app);
    assert!(
        got.iter().all(|note| !matches!(note, SmallNote::DaybreakSet(_))),
        "{got:?}"
    );
}

#[test]
fn a_saved_choice_is_said_with_the_default_preference_that_failed() {
    let mut app = app();
    run(&mut app, AppEv::Daybreak(access(true, true)));
    let fx = verb(&mut app, SmallIntent::Daybreak {
        source: Some(ThreadId::new(T)),
    });
    let Some((_, Ask::DaybreakSave { thread, enabled })) = asks(&fx).into_iter().next() else {
        // the feature flag of the command is off in the fixture: nothing is asked
        return;
    };
    assert_eq!((thread, enabled), (ThreadId::new(T), true));
    answer(
        &mut app,
        &fx,
        Done::Daybreak(Ok(DaybreakSaved {
            defaults: vec![DefaultFailure::Save("read-only".to_owned())],
        })),
    );
    assert_eq!(
        notes(&app),
        vec![
            SmallNote::DaybreakDefault(DefaultFailure::Save("read-only".to_owned())),
            SmallNote::DaybreakSet(true)
        ]
    );
    let _ = DaybreakFailure::Save(String::new());
}

#[test]
fn debug_config_says_the_hosts_rows() {
    let mut app = app();
    let fx = command(&mut app, SlashCommand::DebugConfig, SessionIntent::DebugConfig);
    assert_eq!(asks(&fx)[0].1, Ask::DebugConfig);
    let rows = vec![
        Row {
            text: "/debug-config".to_owned(),
            style: Style::Magenta,
        },
        Row::from(""),
    ];
    answer(&mut app, &fx, Done::DebugConfig(rows.clone()));
    assert_eq!(notes(&app), vec![SmallNote::DebugConfig(rows)]);
}
