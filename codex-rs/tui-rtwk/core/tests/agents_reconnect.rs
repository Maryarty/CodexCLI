//! The connection of the command center as the oracle's reconnect keeps it
//! (`app/reconnect.rs`: `begin_reconnect`, `finish_reconnect`;
//! `app/server_version_notice.rs`): the stale reads are dropped and the
//! footer says so, a new connection ends the notice and reads the list again,
//! and the older-server words wait behind every other notice.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, apply_app};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Command as AgentsCommand, Ev, Notice};

fn app() -> CodexApp {
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    build(&format!("{script}\n")).expect("plays").app
}

fn run(app: &mut CodexApp, ev: Ev) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, AppEv::Screen(ScreenEv::Agents(ev)), &mut fx);
    fx
}

fn lists(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|command| {
            matches!(command, Command::Screen { command, .. }
                if matches!(command, codex_tui_core::screens::ScreenCommand::Agents(AgentsCommand::List { .. })))
        })
        .count()
}

/// `begin_reconnect`: the footer says Reconnecting, what was read from the
/// lost connection is dropped, the older-server words are judged again.
#[test]
fn reconnecting_drops_the_stale_reads_and_says_so() {
    let mut app = app();
    run(
        &mut app,
        Ev::ServerVersion(Some("Service v0.152.1 < Codex CLI v0.153.0".into())),
    );
    std::rc::Rc::make_mut(&mut app.screens)
        .agents
        .reading
        .insert("t".into());
    run(&mut app, Ev::Reconnecting);
    let model = &app.screens.agents;
    assert_eq!(model.notice, Some(Notice::Reconnecting));
    assert_eq!(model.server_version, None);
    assert!(model.reading.is_empty());
    assert!(!model.paging.initialized);
}

/// `finish_reconnect`: the notice ends and the list is read again.
#[test]
fn a_new_connection_ends_the_notice_and_reads_the_list() {
    let mut app = app();
    run(&mut app, Ev::Reconnecting);
    let fx = run(&mut app, Ev::Reconnected);
    assert_eq!(app.screens.agents.notice, None);
    assert_eq!(lists(&fx), 1, "one list request");
}

/// A failed reconnect keeps its notice (the intents stay blocked) and asks
/// for nothing.
#[test]
fn a_failed_reconnect_asks_for_nothing() {
    let mut app = app();
    run(&mut app, Ev::Reconnecting);
    let fx = run(&mut app, Ev::Notice(Some(Notice::ReconnectFailed)));
    assert_eq!(app.screens.agents.notice, Some(Notice::ReconnectFailed));
    assert_eq!(lists(&fx), 0);
}
