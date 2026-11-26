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
use codex_tui_core::screens::agents::{Ev, Page};
use codex_tui_core::screens::onboarding::consent::{self, Checked, Target};
use codex_tui_core::screens::{
    ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, agents, onboarding,
};
use codex_tui_core::threads::{ThreadAsk, UserMessage};

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
            message: UserMessage::text("hello"),
        }),
    );
    assert_eq!(app.cycle_label(), None);
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
    assert!(fx.commands().is_empty());
}

fn cycle(app: &mut CodexApp) -> AppFx {
    run(app, AppEv::Intent(Intent::CycleBackend))
}

fn arrive_on_cloud(app: &mut CodexApp, fx: &AppFx) {
    let (ticket, backend) = start_ticket(fx);
    assert_eq!(backend, CLOUD);
    apply_app_from(
        app,
        CLOUD,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
        &mut AppFx::default(),
    );
    assert_eq!(app.screens().agents.displayed, CLOUD);
}

fn consent_request(fx: &AppFx, to: &Target) -> u64 {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Startup {
                ask:
                    onboarding::Ask::CheckConsent {
                        request, target, ..
                    },
                ..
            } if target == to => Some(*request),
            _ => None,
        })
        .expect("a folder trust check")
}

fn destination(request: u64, ask: Option<onboarding::TrustAsk>) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(onboarding::Ev::ConsentAt {
        request,
        ask,
        checked: Checked::Destination("/workspace/destination".into()),
    }))
}

fn no_start(fx: &AppFx) -> bool {
    !fx.commands().iter().any(|command| {
        matches!(
            command,
            Command::StartThread { .. }
                | Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(
                        codex_tui_core::threads::ThreadAsk::Start { .. }
                    )),
                    ..
                }
        )
    })
}

#[test]
fn the_destination_check_is_routed_to_the_primary_backend_and_pins_the_wire_cwd() {
    let mut app = two_backends().with_folder_consent(true);
    let cloud = cycle(&mut app);
    arrive_on_cloud(&mut app, &cloud);
    let check = cycle(&mut app);
    assert!(no_start(&check), "check before thread/start");
    assert_eq!(app.screens().agents.displayed, CLOUD);
    let request = consent_request(&check, &Target::BackendSwitch);
    assert!(
        check
            .commands()
            .iter()
            .all(|command| app.backend_of(command) == LOCAL)
    );

    let mut foreign = AppFx::default();
    apply_app_from(&mut app, CLOUD, destination(request, None), &mut foreign);
    assert!(no_start(&foreign));
    assert!(consent::pending(&app.screens().onboarding));
    let start = run(&mut app, destination(request, None));
    let (_, backend) = start_ticket(&start);
    assert_eq!(backend, LOCAL);
    let command = start
        .commands()
        .iter()
        .find(|command| matches!(command, Command::StartThread { .. }))
        .expect("the start");
    let request_message = codex_tui_core::protocol::client_request(
        command,
        codex_app_server_protocol::RequestId::Integer(1),
        None,
    )
    .expect("the start request");
    let codex_app_server_protocol::ClientRequest::ThreadStart { params, .. } = request_message
    else {
        panic!("expected thread/start");
    };
    assert_eq!(params.cwd.as_deref(), Some("/workspace/destination"));
    assert_eq!(
        app.screens().agents.displayed,
        CLOUD,
        "wait for thread/start reply"
    );
    assert!(no_start(&run(&mut app, destination(request, None))));
}

#[test]
fn declining_the_destination_keeps_the_current_task_and_ignores_its_old_answer() {
    let mut app = two_backends().with_folder_consent(true);
    let cloud = cycle(&mut app);
    arrive_on_cloud(&mut app, &cloud);
    let request = consent_request(&cycle(&mut app), &Target::BackendSwitch);
    run(
        &mut app,
        destination(
            request,
            Some(onboarding::TrustAsk {
                cwd: "/workspace/destination".into(),
                target: "/workspace/destination".into(),
                cancel: onboarding::TrustCancel::CurrentTask,
                ..onboarding::TrustAsk::default()
            }),
        ),
    );
    assert!(consent::open(&app.screens().onboarding).is_some());
    assert!(no_start(&run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            onboarding::Intent::CancelTrust
        ),))
    )));
    assert!(no_start(&run(&mut app, destination(request, None))));
    assert_eq!(app.screens().agents.displayed, CLOUD);
    assert_eq!(app.session().active, Some(ThreadId::new("cloud-1")));
    assert_ne!(
        consent_request(&cycle(&mut app), &Target::BackendSwitch),
        request
    );
}

#[test]
fn a_successful_switch_invalidates_the_original_center_news_folder_check() {
    let mut app = two_backends().with_folder_consent(true);
    let check = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::New(
            None,
        )))),
    );
    let request = consent_request(&check, &Target::Folder(None));
    let cloud = cycle(&mut app);
    arrive_on_cloud(&mut app, &cloud);
    assert!(!consent::pending(&app.screens().onboarding));
    let old = run(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(onboarding::Ev::Consent {
            request,
            ask: None,
        })),
    );
    assert!(no_start(&old));
    assert_eq!(app.screens().agents.displayed, CLOUD);
}

#[test]
fn a_settled_center_new_waits_for_the_switch_that_may_invalidate_it() {
    for succeeds in [true, false] {
        let mut app = two_backends().with_folder_consent(true);
        let check = run(
            &mut app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::New(
                None,
            )))),
        );
        let request = consent_request(&check, &Target::Folder(None));
        let cloud = cycle(&mut app);
        let (ticket, _) = start_ticket(&cloud);
        let answer = AppEv::Screen(ScreenEv::Onboarding(onboarding::Ev::Consent {
            request,
            ask: None,
        }));
        assert!(
            no_start(&run(&mut app, answer.clone())),
            "wait for the switch"
        );
        let reply = if succeeds {
            Reply::thread_started(ThreadId::new("cloud-1"))
        } else {
            Reply::failed("unavailable")
        };
        let mut effects = AppFx::default();
        apply_app_from(&mut app, CLOUD, AppEv::Reply(ticket, reply), &mut effects);
        assert_eq!(no_start(&effects), succeeds);
        if !succeeds {
            assert!(
                effects
                    .commands()
                    .iter()
                    .all(|command| app.backend_of(command) == LOCAL)
            );
            assert_eq!(app.screens().agents.displayed, LOCAL);
        }
        assert!(no_start(&run(&mut app, answer)), "one answer advances once");
    }
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

fn agents_new(app: &mut CodexApp) -> (Ticket, Command) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::New(
            None,
        )))),
    )
    .into_commands()
    .into_iter()
    .find_map(|command| match &command {
        Command::Screen {
            ticket,
            command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Start { .. })),
            ..
        } => Some((*ticket, command)),
        _ => None,
    })
    .expect("command-center New asks thread/start")
}

fn two_agents_news() -> (CodexApp, (Ticket, Command), Ticket) {
    let mut app = two_backends();
    let cloud_epoch = app.attach_backend(CLOUD);
    assert_eq!(app.epoch(), cloud_epoch);
    let a = agents_new(&mut app);
    assert_eq!(app.backend_of(&a.1), LOCAL);
    let cloud = cycle(&mut app);
    arrive_on_cloud(&mut app, &cloud);
    let (b, command) = agents_new(&mut app);
    assert_eq!(app.backend_of(&command), CLOUD);
    (app, a, b)
}

fn answer_new(
    app: &mut CodexApp,
    backend: BackendId,
    epoch: codex_tui_core::ids::ConnEpoch,
    ticket: Ticket,
    id: &str,
) -> Option<AppFx> {
    app.ingest(Ingress::of(
        backend,
        epoch,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Started(Box::new(
                thread(id, id),
            )))),
        ),
    ))
}

#[test]
fn addressed_rpc_keeps_its_dispatch_backend_after_display_switches() {
    let (app, (_, a), _) = two_agents_news();
    assert_eq!(app.screens().agents.displayed, CLOUD);
    assert_eq!(app.backend_of(&a), LOCAL);
    assert_eq!(
        app.dispatch_route(&a, None).map(|(backend, _)| backend),
        Some(LOCAL)
    );
}

#[test]
fn addressed_rpc_losing_a_drops_late_a_before_and_after_reattach_and_preserves_b() {
    let (mut app, (a, a_command), b) = two_agents_news();
    let old = app.epoch_of(LOCAL);
    let current_b = app.epoch_of(CLOUD);
    app.ingest(Ingress::of(LOCAL, old, AppEv::Disconnected));
    assert_eq!(app.dispatch_route(&a_command, None), None);
    assert_eq!(
        app.epoch_of(LOCAL),
        old,
        "the late reply can still carry the current epoch"
    );
    answer_new(&mut app, LOCAL, old, a, "late-a-before");
    assert!(app.thread(&ThreadId::new("late-a-before")).is_none());
    let new = app.attach_backend(LOCAL);
    assert_ne!(new, old);
    assert!(answer_new(&mut app, LOCAL, old, a, "late-a-after").is_none());
    answer_new(&mut app, LOCAL, new, a, "retired-a");
    assert!(app.thread(&ThreadId::new("retired-a")).is_none());
    answer_new(&mut app, CLOUD, current_b, b, "b-survived");
    assert_eq!(app.screens().agents.owner_of("b-survived"), CLOUD);
    assert!(app.thread(&ThreadId::new("b-survived")).is_some());
}

#[test]
fn addressed_rpc_losing_b_preserves_a_while_b_is_displayed() {
    let (mut app, (a, _), b) = two_agents_news();
    let current_a = app.epoch_of(LOCAL);
    let old = app.epoch_of(CLOUD);
    app.ingest(Ingress::of(CLOUD, old, AppEv::Disconnected));
    assert_eq!(app.screens().agents.displayed, CLOUD);
    answer_new(&mut app, CLOUD, old, b, "late-b");
    assert!(app.thread(&ThreadId::new("late-b")).is_none());
    answer_new(&mut app, LOCAL, current_a, a, "a-survived");
    assert!(app.thread(&ThreadId::new("a-survived")).is_some());
    assert_eq!(app.screens().agents.owner_of("a-survived"), LOCAL);
}

#[test]
fn addressed_rpc_drops_a_same_epoch_foreign_reply_and_a_duplicate() {
    let (mut app, (a, _), _) = two_agents_news();
    let epoch = app.epoch_of(CLOUD);
    assert_eq!(
        epoch,
        app.epoch_of(LOCAL),
        "matching epochs do not make a matching origin"
    );
    answer_new(&mut app, CLOUD, epoch, a, "wrong-owner");
    assert!(app.thread(&ThreadId::new("wrong-owner")).is_none());
    answer_new(&mut app, LOCAL, epoch, a, "correct-owner");
    assert!(app.thread(&ThreadId::new("correct-owner")).is_some());
    assert_eq!(app.screens().agents.owner_of("correct-owner"), LOCAL);
    answer_new(&mut app, LOCAL, epoch, a, "duplicate");
    assert!(app.thread(&ThreadId::new("duplicate")).is_none());
}

#[test]
fn addressed_rpc_transport_replacement_keeps_ingress_and_rejects_old_handles() {
    let mut app = two_backends();
    let ingress_epoch = app.epoch();
    let (old_ticket, old_command) = agents_new(&mut app);
    let transport_epoch = ingress_epoch.next();
    app.ingest(Ingress::new(
        ingress_epoch,
        AppEv::TransportReplaced { transport_epoch },
    ));
    assert_eq!(app.epoch(), ingress_epoch);
    assert_eq!(app.dispatch_route(&old_command, None), None);

    let (fresh, command) = agents_new(&mut app);
    let (_, address) = app.dispatch_route(&command, None).expect("new live ask");
    assert_eq!(address.expect("RPC address").epoch, transport_epoch);
    let answer = |ticket, epoch, id| {
        Ingress::new(
            ingress_epoch,
            AppEv::TransportReply {
                transport_epoch: epoch,
                ticket,
                reply: Reply::screen(ScreenReply::Agents(agents::Reply::Started(Box::new(
                    thread(id, id),
                )))),
            },
        )
    };
    assert!(app.ingest(answer(old_ticket, ingress_epoch, "old")).is_none());
    assert!(app.ingest(answer(fresh, ingress_epoch, "stale")).is_none());
    assert!(app.ingest(answer(fresh, transport_epoch, "fresh")).is_some());
    assert!(app.thread(&ThreadId::new("fresh")).is_some());
    assert!(app.ingest(answer(fresh, transport_epoch, "duplicate")).is_none());

    let now = codex_tui_core::app::Now::at_secs(125);
    assert!(app.ingest(Ingress::new(ingress_epoch, AppEv::Clock(now))).is_some());
    assert_eq!(app.now(), now);
}

#[test]
fn addressed_rpc_provider_interest_follows_only_the_primary_connection() {
    use codex_tui_core::features::{FeatureCommand, FeatureInterest, InterestStep, providers};

    let mut app = two_backends();
    let cloud_epoch = app.attach_backend(CLOUD);
    let cloud = cycle(&mut app);
    arrive_on_cloud(&mut app, &cloud);
    assert_eq!(app.screens().agents.displayed, CLOUD);
    let provider_read = |effects: &AppFx| {
        effects.commands().iter().find_map(|command| match command {
            Command::Feature {
                ticket,
                command: FeatureCommand::Providers(providers::Command::List),
                ..
            } => Some(*ticket),
            _ => None,
        })
    };
    let read = run(
        &mut app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(FeatureInterest::from(
            providers::Interest::List,
        )))),
    );
    let ticket = provider_read(&read).expect("primary provider list");
    let command = read
        .commands()
        .iter()
        .find(|command| matches!(command, Command::Feature { ticket: t, .. } if *t == ticket))
        .expect("provider command");
    assert_eq!(app.backend_of(command), LOCAL);

    app.ingest(Ingress::of(CLOUD, cloud_epoch, AppEv::Disconnected));
    assert!(app.dispatch_route(command, None).is_some());
    let primary = app.epoch_of(LOCAL);
    app.ingest(Ingress::of(
        LOCAL,
        primary,
        AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting)),
    ));
    assert!(app.dispatch_route(command, None).is_none());
    let new_cloud = app.ingest(Ingress::of(
        CLOUD,
        cloud_epoch,
        AppEv::Screen(ScreenEv::Agents(Ev::Reconnected)),
    )).expect("Cloud up");
    assert!(provider_read(&new_cloud).is_none());
    let new_primary = app.ingest(Ingress::of(
        LOCAL,
        primary,
        AppEv::Screen(ScreenEv::Agents(Ev::Reconnected)),
    )).expect("primary up");
    assert!(provider_read(&new_primary).is_some());
}

fn local_turn_waiting_for_a_reply() -> (CodexApp, Ticket, ThreadId) {
    let mut app = two_backends();
    let epoch = app.attach_backend(CLOUD);
    assert_eq!(epoch, app.epoch());
    let foreground = ThreadId::new("local-1");
    let effects = run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(foreground.clone()),
            message: UserMessage::text("one turn"),
        }),
    );
    let (ticket, command) = effects
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartTurn { ticket, .. } => Some((*ticket, command)),
            _ => None,
        })
        .expect("the local turn");
    assert_eq!(
        app.dispatch_route(command, None).map(|(backend, _)| backend),
        Some(LOCAL)
    );
    (app, ticket, foreground)
}

#[test]
fn addressed_rpc_a_thread_reply_checks_its_origin_before_consuming_its_ticket() {
    let (mut app, ticket, foreground) = local_turn_waiting_for_a_reply();
    let epoch = app.epoch();
    let answer = || {
        AppEv::Reply(
            ticket,
            Reply::turn_started(
                foreground.clone(),
                codex_tui_core::ids::TurnId::new("one-turn"),
            ),
        )
    };
    assert!(app.ingest(Ingress::of(CLOUD, epoch, answer())).is_none());
    assert!(app.active_thread().outgoing().contains(ticket));
    assert!(app.ingest(Ingress::of(LOCAL, epoch, answer())).is_some());
    assert!(app.active_thread().is_running());
    assert!(app.ingest(Ingress::of(LOCAL, epoch, answer())).is_none());
}

#[test]
fn addressed_rpc_reattach_retires_old_thread_requests_and_holds_unconfirmed_input() {
    let (mut app, ticket, foreground) = local_turn_waiting_for_a_reply();
    let old = app.epoch();
    let new = app.attach_backend(LOCAL);
    assert_ne!(new, old);
    assert!(!app.active_thread().outgoing().contains(ticket));
    assert!(matches!(
        app.active_thread().queue().first(),
        Some(codex_tui_core::threads::Queued::Unconfirmed { .. })
    ));
    let answer = || {
        AppEv::Reply(
            ticket,
            Reply::turn_started(
                foreground.clone(),
                codex_tui_core::ids::TurnId::new("stale-turn"),
            ),
        )
    };
    assert!(app.ingest(Ingress::of(LOCAL, old, answer())).is_none());
    assert!(app.ingest(Ingress::of(LOCAL, new, answer())).is_none());
    assert!(!app.active_thread().is_running());
}

fn open_agents_list(app: &mut CodexApp) -> Ticket {
    run(app, AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::Open))))
        .commands().iter().find_map(|command| match command {
            Command::Screen { ticket, command: ScreenCommand::Agents(agents::Command::List { .. }), .. } => Some(*ticket),
            _ => None,
        }).expect("the list")
}

#[test]
fn addressed_rpc_a_parked_list_settles_only_its_owner_even_during_reconnect() {
    for loses_a in [false, true] {
        let mut app = two_backends();
        let a_epoch = app.epoch();
        app.attach_backend(CLOUD);
        let a = open_agents_list(&mut app);
        assert!(app.screens().agents.paging_of(LOCAL).loading);
        let cloud = cycle(&mut app);
        arrive_on_cloud(&mut app, &cloud);
        open_agents_list(&mut app);
        let b_paging = app.screens().agents.paging.clone();
        assert!(b_paging.loading);
        let answer = AppEv::Reply(a, Reply::screen(ScreenReply::Agents(agents::Reply::Listed(Page {
            threads: vec![thread("listed-on-a", "On A")], unread: Vec::new(),
            next_cursor: Some("next-a".into()), append: false, pins: None,
        }))));
        if loses_a {
            app.ingest(Ingress::of(LOCAL, a_epoch, AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting))));
            assert!(app.ingest(Ingress::of(LOCAL, a_epoch, answer)).is_none());
            app.ingest(Ingress::of(LOCAL, a_epoch, AppEv::TransportReplaced {
                transport_epoch: a_epoch.next(),
            }));
            let restored = app.ingest(Ingress::of(LOCAL, a_epoch, AppEv::Screen(ScreenEv::Agents(Ev::Reconnected))))
                .expect("the restored connection");
            assert!(restored.commands().iter().all(|c|
                !matches!(c, Command::Screen { command: ScreenCommand::Agents(agents::Command::List { .. }), .. })));
        } else {
            app.ingest(Ingress::of(LOCAL, a_epoch, answer));
            assert_eq!(app.screens().agents.owner_of("listed-on-a"), LOCAL);
        }
        assert_eq!(app.screens().agents.paging, b_paging);
        let a_paging = app.screens().agents.paging_of(LOCAL).clone();
        assert!(!a_paging.loading);
        assert_eq!(a_paging.failed, loses_a);
        let (ticket, backend) = start_ticket(&cycle(&mut app));
        assert_eq!(backend, LOCAL);
        run(&mut app, AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("local-again"))));
        assert_eq!(app.screens().agents.displayed, LOCAL);
        assert_eq!(app.screens().agents.paging, a_paging);
    }
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
