//! Switching the displayed backend (oracle `AppEvent::CycleNewConversationBackend`,
//! `start_new_session_on_other_backend`, codex-tui 0.170.0): an empty
//! conversation moves to the next connected backend, the draft goes with
//! it; one connection epoch per backend; a notification carries the
//! backend it came from (`BackendRegistry::observe` drops a foreign id's).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadNameUpdatedNotification;
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, BackendLabel, CodexApp, Command, Ingress, Intent, Session, apply_app,
    apply_app_from,
};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev, Page};

const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

/// A ready app on the local backend with an empty thread `local-1` and the
/// cloud connected.
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

fn name_updated(thread: &str, name: &str) -> AppEv {
    AppEv::Backend(BackendEv::Notification(Box::new(
        ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
            thread_id: thread.to_owned(),
            thread_name: Some(name.to_owned()),
        }),
    )))
}

/// `key_chords.rs::empty_thread_backend_chord_preserves_draft_until_first_submission`
/// (the model's half): the control is offered on an empty conversation when
/// another backend is connected, labelled with the displayed one.
#[test]
fn the_backend_control_shows_on_an_empty_conversation_with_two_backends() {
    let mut app = CodexApp::new();
    app.attach();
    assert_eq!(app.cycle_label(), None, "nothing connected yet");
    let app = two_backends();
    assert_eq!(app.cycle_label(), Some(BackendLabel::Local));
}

#[test]
fn a_single_backend_offers_no_control_and_the_intent_does_nothing() {
    let mut app = CodexApp::new();
    app.attach();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_ticket(&fx).0;
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("local-1"))),
    );
    assert_eq!(app.cycle_label(), None);
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    assert!(fx.commands().is_empty());
}

/// `CycleNewConversationBackend`: a new conversation starts on the next
/// backend (the RPC goes there), its answer makes that backend displayed and
/// the new thread the chat's; the draft goes with it (the session says so).
#[test]
fn cycling_starts_a_conversation_on_the_next_backend() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, backend) = start_ticket(&fx);
    assert_eq!(backend, CLOUD);
    assert_eq!(
        app.backend_of(&fx.commands()[0]),
        CLOUD,
        "routed to the cloud"
    );
    // Not displayed before the cloud answers.
    assert_eq!(app.screens().agents.displayed, LOCAL);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    assert_eq!(app.screens().agents.displayed, CLOUD);
    assert_eq!(app.session().active, Some(ThreadId::new("cloud-1")));
    assert_eq!(app.session().status, Session::Ready);
    assert_eq!(app.session().switched_from, Some(ThreadId::new("local-1")));
    assert_eq!(app.session().switches, 1);
    assert_eq!(app.screens().agents.owner_of("cloud-1"), CLOUD);
    assert_eq!(app.cycle_label(), Some(BackendLabel::Cloud));
    // Round trip: the next backend after the cloud is the local one.
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, backend) = start_ticket(&fx);
    assert_eq!(backend, LOCAL);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("local-2"))),
    );
    assert_eq!(app.screens().agents.displayed, LOCAL);
    assert_eq!(app.cycle_label(), Some(BackendLabel::Local));
}

/// A failed start leaves the displayed backend and the chat as they were
/// (oracle: the error is shown, nothing switches).
#[test]
fn a_failed_switch_keeps_the_displayed_backend() {
    let mut app = two_backends();
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    let (ticket, _) = start_ticket(&fx);
    run(&mut app, AppEv::Reply(ticket, Reply::failed("unavailable")));
    assert_eq!(app.screens().agents.displayed, LOCAL);
    assert_eq!(app.session().active, Some(ThreadId::new("local-1")));
    assert_eq!(app.session().status, Session::Ready);
    assert_eq!(app.session().switches, 0);
    assert!(app.active_thread().last_error().cloned().is_some());
}

/// `can_cycle_conversation_backend`: only while the conversation has no turn
/// yet; a submit ends the offer.
#[test]
fn a_conversation_with_a_turn_cannot_cycle() {
    let mut app = two_backends();
    run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: None,
            text: "hello".into(),
        }),
    );
    assert_eq!(app.cycle_label(), None);
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    assert!(fx.commands().is_empty());
}

/// One connection epoch PER backend: a new connection of the first drops the
/// first's older events and never the cloud's; the cloud's own stale epoch
/// is dropped.
#[test]
fn each_backend_has_its_own_connection_epoch() {
    let mut app = CodexApp::new();
    let local_epoch = app.attach();
    let cloud_epoch = app.attach_backend(CLOUD);
    let ev = || AppEv::Screen(ScreenEv::Agents(Ev::Home(Some("/h".into()))));
    assert!(app.ingest(Ingress::of(CLOUD, cloud_epoch, ev())).is_some());
    // The first reconnects.
    let newer = app.attach();
    assert_ne!(newer, local_epoch);
    assert!(app.ingest(Ingress::of(LOCAL, local_epoch, ev())).is_none());
    assert!(app.ingest(Ingress::new(newer, ev())).is_some());
    assert!(
        app.ingest(Ingress::of(CLOUD, cloud_epoch, ev())).is_some(),
        "the cloud's events survive the first's reconnect"
    );
    // The cloud reconnects.
    let cloud_newer = app.attach_backend(CLOUD);
    assert!(app.ingest(Ingress::of(CLOUD, cloud_epoch, ev())).is_none());
    assert!(app.ingest(Ingress::of(CLOUD, cloud_newer, ev())).is_some());
    assert!(app.ingest(Ingress::new(newer, ev())).is_some());
}

/// The oracle's `overview_thread`: an idle root named `name`.
fn thread(id: &str, name: &str) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn name_of(app: &CodexApp, id: &str) -> Option<String> {
    app.screens()
        .agents
        .threads
        .iter()
        .find(|held| held.id == id)
        .and_then(|held| held.name.clone())
}

/// `overview_keeps_rows_and_routes_actions_to_the_owner`, the notification
/// part: `name_updated(remote_id, None)` from the displayed backend leaves
/// the remote row as it is (`BackendRegistry::observe` drops a foreign id's
/// notification); the owner's is handled; an unowned thread's notification
/// claims it for its origin.
#[test]
fn a_notification_of_a_foreign_thread_is_dropped_by_origin() {
    let mut app = two_backends();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::ListedBy(
            CLOUD,
            Page {
                threads: vec![thread("remote", "Remote task")],
                unread: vec![],
                next_cursor: None,
                append: false,
                pins: None,
            },
        ))),
    );
    assert_eq!(app.screens().agents.owner_of("remote"), CLOUD);
    let mut fx = AppFx::default();
    apply_app_from(
        &mut app,
        LOCAL,
        name_updated("remote", "From local"),
        &mut fx,
    );
    assert_eq!(name_of(&app, "remote").as_deref(), Some("Remote task"));
    apply_app_from(
        &mut app,
        CLOUD,
        name_updated("remote", "From cloud"),
        &mut fx,
    );
    assert_eq!(name_of(&app, "remote").as_deref(), Some("From cloud"));
    apply_app_from(&mut app, CLOUD, name_updated("fresh", "Named"), &mut fx);
    assert_eq!(app.screens().agents.owner_of("fresh"), CLOUD);
    apply_app_from(&mut app, LOCAL, name_updated("fresh", "Other"), &mut fx);
    assert_eq!(app.screens().agents.owner_of("fresh"), CLOUD);
}
