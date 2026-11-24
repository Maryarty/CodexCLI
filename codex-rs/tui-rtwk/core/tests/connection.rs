#![allow(clippy::expect_used, clippy::unwrap_used)]

//! A lost connection (`app/connection.rs`; oracle: a request on a lost connection errors, the
//! overview refreshes after `finish_reconnect`): a remote server's `Reconnecting` answers every
//! app-scoped ask in flight on it, so no list waits for an answer that will not come, and the
//! `Reconnected` that follows asks again for what the surfaces still want. The loss as the
//! embedded server tells it (`Disconnected`) takes the same path.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::plugins::{self, Configured, Fact, Interest, Want};
use codex_tui_core::features::{FeatureCommand, FeatureFact, FeatureInterest, InterestStep};
use codex_tui_core::ids::{Cwd, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenEv, agents};
use pretty_assertions::assert_eq;

const CWD: &str = "/work/project";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn agents_ev(app: &mut CodexApp, ev: agents::Ev) -> AppFx {
    run(app, AppEv::Screen(ScreenEv::Agents(ev)))
}

/// An app with a first thread and the plugins feature on.
fn app() -> CodexApp {
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
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
    );
    let config = plugins::Config {
        enabled: true,
        remote: false,
        sharing: false,
        home: Some("/home/me".to_string()),
        configured: vec![Configured {
            name: "team".to_string(),
            git: true,
        }],
    };
    run(
        &mut app,
        AppEv::Feature(FeatureFact::from(Fact::Config(config))),
    );
    app
}

fn interested() -> FeatureInterest {
    FeatureInterest::from(Interest::Listing { cwd: Cwd::new(CWD) })
}

/// The list requests of the feature a step left.
fn list_asks(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::Feature {
                    command: FeatureCommand::Plugins(wanted),
                    ..
                } if wanted.want == Want::List
            )
        })
        .count()
}

fn loading(app: &CodexApp) -> bool {
    app.features()
        .plugins
        .catalog(&Cwd::new(CWD))
        .is_some_and(codex_tui_core::features::Remote::is_loading)
}

/// `/plugins` opened, its list asked, and the server dropped before it answered: the list is not
/// loading any more (it failed, a second open asks again), and the new connection asks for it.
#[test]
fn a_reconnect_answers_the_list_in_flight_and_asks_it_again() {
    let mut app = app();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(interested()))),
    );
    assert_eq!(list_asks(&fx), 1);
    assert!(loading(&app));

    agents_ev(&mut app, agents::Ev::Reconnecting);
    assert!(!loading(&app), "the ask of the lost connection failed");

    let fx = agents_ev(&mut app, agents::Ev::Reconnected);
    assert_eq!(
        list_asks(&fx),
        1,
        "the new connection is asked for the list"
    );
    assert!(loading(&app));
}

/// The embedded server's loss answers the same asks.
#[test]
fn a_disconnect_answers_the_list_in_flight() {
    let mut app = app();
    run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(interested()))),
    );
    assert!(loading(&app));
    run(&mut app, AppEv::Disconnected);
    assert!(!loading(&app));
}

/// The model catalog read at connect (`AppCtx::Select`) is answered too: the pickers are not left
/// loading by a reconnect.
#[test]
fn a_reconnect_answers_the_model_list_in_flight() {
    let mut app = CodexApp::new();
    run(&mut app, AppEv::Connected);
    assert!(
        app.screens().select.catalog.pending.is_some(),
        "the catalog is read at connect"
    );
    agents_ev(&mut app, agents::Ev::Reconnecting);
    assert_eq!(app.screens().select.catalog.pending, None);
}
