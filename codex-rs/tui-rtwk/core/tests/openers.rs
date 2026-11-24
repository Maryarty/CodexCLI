#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The openers of P3: `/presets`, `/daemon` and `/subagents` (oracle `slash_dispatch.rs`,
//! `chatwidget/quick_dials.rs`, `app/daemon_menu.rs`, `app/session_lifecycle.rs`
//! `open_agent_picker`). `/presets` is the select screen's gate (the menu itself is the
//! composer's drawn state), `/daemon` is the feature slice `features::daemon` (`Target::Feature`,
//! facts from the host, verbs from the sheet, the menu pages as wishes), `/subagents` is the pure
//! rows `select::agent_picker` derives for the UI's picker (a pick is `Intent::OpenThread`, ADR-003).
//! The provider switch the model picker obeys (`select::Provider`) is covered by `select_live.rs`;
//! the app link needs nothing of the model. The words are the view's: these tests read data.

use std::collections::BTreeSet;

use codex_app_server_protocol::Thread;
use codex_tui_core::app::effects::{Ask, DaemonAsk, DaemonDone, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{DispatchCx, Outcome, Target, dispatch};
use codex_tui_core::features::daemon::{
    Cli, Connection, Facts, Guidance, Menu, Reason, Row as DaemonRow, RowState, Source, State,
    Status, Step, Target as Server, Verb, Wish,
};
use codex_tui_core::features::{FeatureFact, FeatureIntent, FeatureNote, FeatureWish};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::agent_picker::{self, Name, Opened, Row};
use codex_tui_core::screens::select::quick_dials::{Gate, Open};
use codex_tui_core::screens::select::{self, Note};
use codex_tui_core::threads::{Item, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::json;

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

fn asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------------- /presets

fn presets(app: &mut CodexApp, gate: Gate) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(
            select::Intent::Quick {
                gate,
                open: Open::Menu,
            },
        ))),
    );
}

fn said(app: &CodexApp) -> Vec<Note> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Select(note) => Some(note.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn the_command_names_the_select_screen_and_the_menu_is_the_composers() {
    let outcome = dispatch("/presets", &DispatchCx::default());
    assert!(
        matches!(&outcome, Outcome::Open(open) if open.target == Target::Select),
        "{outcome:?}"
    );
    // Where presets can be used the model says nothing and opens no sheet: the menu is drawn by
    // the composer.
    let mut app = ready();
    presets(&mut app, Gate::Ready);
    assert_eq!(said(&app), vec![]);
    assert_eq!(app.screens().select.shows.pending().count(), 0);
}

#[test]
fn presets_say_why_they_do_not_open_where_the_user_is() {
    let mut app = ready();
    presets(&mut app, Gate::Blocked);
    presets(&mut app, Gate::Starting);
    assert_eq!(
        said(&app),
        vec![Note::PresetsUnavailable, Note::PresetsStarting]
    );
}

// -------------------------------------------------------------------- /daemon

fn cli(has_package: bool) -> Cli {
    Cli {
        path: "/usr/bin/codex".to_owned(),
        has_package,
        version: "0.153.0".to_owned(),
    }
}

fn connection(managed: bool) -> Connection {
    Connection {
        version: "v0.153.0".to_owned(),
        local_daemon: true,
        managed,
    }
}

fn local(managed: bool, cli: Option<Cli>) -> Facts {
    Facts {
        target: Server::LocalDaemon,
        connection: Some(connection(managed)),
        cli,
        offline: false,
    }
}

fn told(app: &mut CodexApp, facts: Facts) {
    run(app, AppEv::Feature(FeatureFact::from(facts)));
}

fn verb(app: &mut CodexApp, verb: Verb) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(verb))),
    )
}

fn state(app: &CodexApp) -> &State {
    &app.features().daemon
}

fn menus(app: &CodexApp) -> Vec<Menu> {
    app.wishes()
        .pending()
        .filter_map(|w| match &w.wish {
            FeatureWish::Daemon(Wish::Menu(menu)) => Some(*menu),
            _ => None,
        })
        .collect()
}

#[test]
fn the_command_is_the_feature_that_opens_the_sheet() {
    let outcome = dispatch("/daemon", &DispatchCx::default());
    assert!(
        matches!(&outcome, Outcome::Open(open) if matches!(open.target, Target::Feature(_))),
        "{outcome:?}"
    );
}

#[test]
fn the_menu_rows_follow_the_facts() {
    let page = local(true, Some(cli(true))).page(false);
    assert_eq!(page.status, Status::Running("v0.153.0".to_owned()));
    assert_eq!(page.guidance, None);
    assert_eq!(
        page.rows,
        vec![
            RowState {
                row: DaemonRow::Update(Source::PublicStable),
                disabled: false,
                reason: None
            },
            RowState {
                row: DaemonRow::Update(Source::ThisCli),
                disabled: false,
                reason: None
            },
            RowState {
                row: DaemonRow::Restart,
                disabled: false,
                reason: None
            },
        ]
    );
    // No package: only "this CLI" says why it cannot be used.
    let page = local(true, Some(cli(false))).page(false);
    assert_eq!(page.rows[1].reason, Some(Reason::NoPackage));
    assert_eq!(page.rows[0].reason, None);
    // A server not proved managed cannot be restarted, and the row says why.
    let page = local(false, Some(cli(true))).page(false);
    assert!(page.rows[2].disabled);
    assert_eq!(page.rows[2].reason, Some(Reason::Unverified));
    // A restart in flight disables the row.
    assert!(local(true, Some(cli(true))).page(true).rows[2].disabled);
    // Without a CLI, or on another host: every update is off, with the guidance.
    let page = local(true, None).page(false);
    assert_eq!(page.guidance, Some(Guidance::NoCli));
    assert!(page.rows[..2].iter().all(|r| r.disabled));
    let remote = Facts {
        target: Server::Remote,
        connection: Some(Connection {
            local_daemon: false,
            ..connection(false)
        }),
        cli: Some(cli(true)),
        offline: false,
    };
    let page = remote.page(false);
    assert_eq!(page.status, Status::NotConnected);
    assert_eq!(page.guidance, Some(Guidance::Remote));
    assert!(page.rows.iter().all(|r| r.disabled));
}

#[test]
fn opening_a_managed_daemon_asks_for_the_menu_at_once() {
    let mut app = ready();
    told(&mut app, local(true, Some(cli(true))));
    let fx = verb(&mut app, Verb::Open);
    assert_eq!(asks(&fx), vec![]);
    assert_eq!(menus(&app), vec![Menu::Main]);
}

#[test]
fn opening_an_unverified_daemon_probes_it_first() {
    let mut app = ready();
    told(&mut app, local(false, Some(cli(true))));
    let fx = verb(&mut app, Verb::Open);
    let asked = asks(&fx);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0].1, Ask::BackgroundServer(DaemonAsk::Verify));
    assert_eq!(menus(&app), vec![], "the menu waits for the probe");
    // Asking again while the probe is out sends nothing more.
    let fx = verb(&mut app, Verb::Open);
    assert_eq!(asks(&fx), vec![]);
    // The probe says managed: the menu opens and the restart row is enabled.
    run(
        &mut app,
        AppEv::Effect {
            request: asked[0].0,
            done: Done::BackgroundServer(DaemonDone::Managed(true)),
        },
    );
    assert_eq!(menus(&app), vec![Menu::Main]);
    assert!(!state(&app).page().rows[2].disabled);
}

#[test]
fn a_restart_is_confirmed_asked_once_and_reports_its_failure() {
    let mut app = ready();
    told(&mut app, local(true, Some(cli(true))));
    verb(&mut app, Verb::ConfirmRestart);
    assert_eq!(menus(&app), vec![Menu::ConfirmRestart]);
    let fx = verb(&mut app, Verb::Restart);
    let asked = asks(&fx);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0].1, Ask::BackgroundServer(DaemonAsk::Restart));
    assert!(state(&app).restarting);
    // A second restart while one runs asks nothing.
    let fx = verb(&mut app, Verb::Restart);
    assert_eq!(asks(&fx), vec![]);
    run(
        &mut app,
        AppEv::Effect {
            request: asked[0].0,
            done: Done::BackgroundServer(DaemonDone::Restarted(Err("no socket".to_owned()))),
        },
    );
    assert!(!state(&app).restarting);
    let failed = app
        .active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| {
            matches!(item, Item::Note(note)
                if matches!(&note.key, NoteKey::Feature(FeatureNote::Daemon(_))))
        })
        .count();
    assert_eq!(failed, 1);
}

#[test]
fn an_update_is_confirmed_then_left_for_the_shell() {
    let mut app = ready();
    told(&mut app, local(true, Some(cli(true))));
    verb(&mut app, Verb::ConfirmUpdate(Source::ThisCli));
    assert_eq!(menus(&app), vec![Menu::ConfirmUpdate(Source::ThisCli)]);
    assert_eq!(state(&app).update_wanted(), None);
    let fx = verb(&mut app, Verb::Update(Source::ThisCli));
    assert_eq!(asks(&fx), vec![], "the update runs after Codex exits");
    assert_eq!(state(&app).update_wanted(), Some(Source::ThisCli));
}

#[test]
fn the_state_asks_once_and_updates_need_a_cli() {
    let mut daemon = State::default();
    daemon.told(local(false, Some(cli(true))));
    assert_eq!(daemon.verb(Verb::Open), Some(Step::Ask(DaemonAsk::Verify)));
    assert_eq!(daemon.verb(Verb::Open), None, "the probe is out");
    assert_eq!(daemon.verified(false), Some(Menu::Main));
    assert_eq!(daemon.verified(false), None, "an answer nobody waits for");
    // Not managed: the restart confirmation does not open.
    assert_eq!(daemon.verb(Verb::ConfirmRestart), None);
    // No CLI: no update confirmation either.
    daemon.told(local(true, None));
    assert_eq!(daemon.verb(Verb::ConfirmUpdate(Source::PublicStable)), None);
    assert_eq!(daemon.restarted(Err("x".to_owned())), Some("x".to_owned()));
    assert_eq!(daemon.restarted(Ok(())), None);
}

// ----------------------------------------------------------------- /subagents

fn thread(id: &str, parent: Option<&str>, nickname: Option<&str>, role: Option<&str>) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": parent,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1, "updatedAt": 1,
        "recencyAt": 1, "status": {"type": "notLoaded"}, "path": null, "cwd": "/tmp",
        "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": nickname, "agentRole": role, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn id(text: &str) -> ThreadId {
    ThreadId::new(text.to_owned())
}

#[test]
fn the_picker_lists_the_primary_then_its_agents_and_marks_the_current() {
    let threads = vec![
        thread("main", None, None, None),
        thread("a", Some("main"), Some("Robie"), Some("explorer")),
        thread("b", Some("a"), None, Some("worker")),
        thread("other", None, None, None),
        thread("c", Some("other"), Some("Stray"), None),
    ];
    let none = BTreeSet::new();
    let rows = agent_picker::rows(&threads, &none, &none, Some(&id("b")), &[], |_| false);
    let names: Vec<String> = rows.iter().map(|r| r.name.label()).collect();
    assert_eq!(names, ["Main [default]", "Robie [explorer]", "[worker]"]);
    assert_eq!(
        rows.iter().map(|r| r.current).collect::<Vec<_>>(),
        [false, false, true]
    );
    assert!(rows.iter().all(|r| r.closed), "not loaded and not running");
    // A thread the app runs is not closed.
    let rows = agent_picker::rows(&threads, &none, &none, Some(&id("b")), &[], |t| {
        t == &id("a")
    });
    assert_eq!(
        rows.iter().map(|r| r.closed).collect::<Vec<_>>(),
        [true, false, true]
    );
}

#[test]
fn a_thread_waiting_for_approval_is_listed_first_and_hidden_ones_are_not_listed() {
    let threads = vec![
        thread("main", None, None, None),
        thread("a", Some("main"), Some("Robie"), None),
        thread("b", Some("main"), Some("Bo"), None),
        thread("gone", Some("main"), Some("Gone"), None),
    ];
    let hidden: BTreeSet<String> = ["gone".to_owned()].into();
    let rows = agent_picker::rows(
        &threads,
        &hidden,
        &BTreeSet::new(),
        Some(&id("main")),
        &[id("b")],
        |_| true,
    );
    let order: Vec<_> = rows.iter().map(|r| r.thread.as_str().to_owned()).collect();
    assert_eq!(order, ["b", "main", "a"]);
    assert!(rows[0].needs_approval);
    assert!(agent_picker::has_subagents(&rows));
}

#[test]
fn nothing_known_is_no_agents_and_the_primary_alone_is_a_list() {
    let none = BTreeSet::new();
    let rows = agent_picker::rows(&[], &none, &none, None, &[], |_| true);
    assert_eq!(agent_picker::opened(rows), Opened::NoAgents);
    let rows = agent_picker::rows(&[], &none, &none, Some(&id("main")), &[], |_| true);
    assert_eq!(
        rows,
        vec![Row {
            thread: id("main"),
            name: Name::Main,
            current: true,
            needs_approval: false,
            closed: false,
        }]
    );
    assert!(!agent_picker::has_subagents(&rows));
    assert!(matches!(agent_picker::opened(rows), Opened::Rows(_)));
    assert_eq!(
        Name::Agent {
            nickname: None,
            role: None
        }
        .label(),
        "Agent"
    );
}
