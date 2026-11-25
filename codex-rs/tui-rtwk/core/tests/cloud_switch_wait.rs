//! The empty conversation's move to a Cloud backend while the draft stays editable (oracle
//! `cloud_switch_wait.rs`, `CloudSwitchProgress`): `SessionSlice::cloud_switch` says Connecting
//! while Cloud's start is unanswered and Failed for the apply that heard the refusal; only a
//! Cloud destination waits (`responsive_cloud_switch`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{
    AppEv, AppFx, CloudSwitch, CodexApp, Command, Intent, Now, Session, apply_app,
};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev;

const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn start_ticket(fx: &AppFx) -> (Ticket, BackendId) {
    let found: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::StartThread {
                ticket, backend, ..
            } => Some((*ticket, *backend)),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "one StartThread in {:?}", fx.commands());
    found[0]
}

/// A ready app on the local backend with an empty thread `local-1` and the cloud connected.
fn two_backends() -> CodexApp {
    let mut app = CodexApp::new();
    app.attach();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_ticket(&fx).0;
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("local-1"))),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::CloudBackend(CLOUD))),
    );
    app
}

#[test]
fn cloud_switch_is_connecting_until_the_start_answers() {
    let mut app = two_backends();
    assert_eq!(app.session().cloud_switch, None);
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, backend) = start_ticket(&fx);
    assert_eq!(backend, CLOUD);
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Connecting));
    // an unrelated event leaves the wait as it is
    run(&mut app, AppEv::Clock(Now::at_secs(5)));
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Connecting));
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    assert_eq!(app.session().cloud_switch, None);
    assert_eq!(app.session().active, Some(ThreadId::new("cloud-1")));
}

#[test]
fn a_refused_cloud_switch_says_failed_and_keeps_the_draft_thread() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, _) = start_ticket(&fx);
    let fx = run(&mut app, AppEv::Reply(ticket, Reply::failed("unavailable")));
    // the apply that heard the refusal shows it, and the app asks to be woken to take it away
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Failed));
    assert!(fx.wake().is_some(), "an idle app is woken to clear it");
    assert_eq!(app.session().active, Some(ThreadId::new("local-1")));
    assert_eq!(app.session().status, Session::Ready);
    assert_eq!(app.session().switches, 0);
    assert!(app.active_thread().last_error().cloned().is_some());
    // the next apply (the wake, or any event) ends it
    run(&mut app, AppEv::Clock(Now::at_secs(9)));
    assert_eq!(app.session().cloud_switch, None);
}

#[test]
fn a_refusal_ends_with_the_next_event_and_a_new_cycle_waits_again() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, _) = start_ticket(&fx);
    run(&mut app, AppEv::Reply(ticket, Reply::failed("unavailable")));
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Failed));
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, backend) = start_ticket(&fx);
    assert_eq!(backend, CLOUD);
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Connecting));
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    assert_eq!(app.session().cloud_switch, None);
}

#[test]
fn a_local_switch_has_no_cloud_switch_state() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, _) = start_ticket(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    // back to the local backend: the oracle waits only for a Cloud destination
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, backend) = start_ticket(&fx);
    assert_eq!(backend, LOCAL);
    assert_eq!(app.session().cloud_switch, None);
    run(&mut app, AppEv::Reply(ticket, Reply::failed("unavailable")));
    assert_eq!(app.session().cloud_switch, None, "a local refusal says nothing");
}

#[test]
fn a_second_cycle_while_connecting_starts_nothing() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, _) = start_ticket(&fx);
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    assert_eq!(app.session().cloud_switch, Some(CloudSwitch::Connecting));
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    assert_eq!(app.session().switches, 1);
}
