//! The startup page through the app (ADR-003: it changes only through
//! `apply_app`): a connection lost while the trust is being saved, or while
//! the update to run is being stored.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Facts, Intent as StartupIntent, Stage, TrustAsk, TrustDecision, UpdateChoice,
    UpdateOffer,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};

// `all.rs` declares this file by `#[path]` (`test-mods`), which makes it a module file of its
// directory: its children name their own.
#[path = "onboarding/consent.rs"]
mod consent;
#[path = "onboarding/device.rs"]
mod device;
#[path = "onboarding/gateway.rs"]
mod gateway;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn send(app: &mut CodexApp, ev: AppEv) {
    run(app, ev);
}

fn choose(app: &mut CodexApp, intent: StartupIntent) {
    send(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(intent))),
    );
}

fn decision(app: &CodexApp) -> Option<TrustDecision> {
    app.screens()
        .onboarding
        .trust
        .as_ref()
        .and_then(|t| t.decision)
}

fn saving() -> (CodexApp, u64) {
    let mut app = CodexApp::new();
    send(&mut app, AppEv::Connected);
    let facts = Facts {
        trust: Some(TrustAsk {
            cwd: "/work".into(),
            target: "/work".into(),
            ..TrustAsk::default()
        }),
        ..Facts::default()
    };
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { request: 1, facts })),
    );
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            StartupIntent::Trust,
        ))),
    );
    let request = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::WriteTrust { request, .. },
                        ..
                    },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("trust write");
    assert_eq!(decision(&app), Some(TrustDecision::Saving));
    (app, request)
}

// A `TrustSaved` that never arrives (the connection dropped while saving)
// must not leave the step stuck: the choice is open again, and a late answer
// to the old write is dropped.
#[test]
fn a_connection_lost_while_saving_trust_opens_the_choice_again() {
    let (mut app, _) = saving();
    send(&mut app, AppEv::Disconnected);
    assert_eq!(decision(&app), None);
    choose(&mut app, StartupIntent::CancelTrust);
    assert_eq!(decision(&app), Some(TrustDecision::Cancelled));
}

#[test]
fn a_late_answer_to_the_write_of_a_lost_connection_is_dropped() {
    let (mut app, request) = saving();
    send(&mut app, AppEv::Disconnected);
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request })),
    );
    assert_eq!(decision(&app), None);
}

// Without a write in flight, a lost connection changes nothing.
#[test]
fn a_lost_connection_leaves_a_made_choice_alone() {
    let (mut app, request) = saving();
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request })),
    );
    send(&mut app, AppEv::Disconnected);
    assert_eq!(decision(&app), Some(TrustDecision::Trusted));
}

fn update_running() -> (CodexApp, u64) {
    let mut app = CodexApp::new();
    send(&mut app, AppEv::Connected);
    let facts = Facts {
        update: Some(UpdateOffer {
            current: "0.0.0".into(),
            latest: "9.9.9".into(),
            command: "npm install -g @openai/codex".into(),
            choice: None,
        }),
        ..Facts::default()
    };
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { request: 1, facts })),
    );
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            StartupIntent::ChooseUpdate(UpdateChoice::UpdateNow),
        ))),
    );
    let request = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::RunUpdate { request },
                        ..
                    },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("stored update");
    assert!(!app.screens().onboarding.done(Stage::Update));
    (app, request)
}

fn update_choice(app: &CodexApp) -> Option<UpdateChoice> {
    app.screens()
        .onboarding
        .update
        .as_ref()
        .and_then(|u| u.choice)
}

// Delta 5: the page leaves the update prompt only after the host stored the
// update (`UpdateQueued`). An ack lost with the connection must not leave the
// page on the prompt for good: the choice is open again, as the trust write's
// is, and a late answer to the old run is dropped.
#[test]
fn a_connection_lost_while_the_update_is_stored_opens_the_prompt_again() {
    let (mut app, _) = update_running();
    send(&mut app, AppEv::Disconnected);
    assert_eq!(update_choice(&app), None);
    choose(&mut app, StartupIntent::ChooseUpdate(UpdateChoice::Skip));
    assert!(app.screens().onboarding.done(Stage::Update));
}

#[test]
fn a_late_ack_of_the_update_of_a_lost_connection_is_dropped() {
    let (mut app, request) = update_running();
    send(&mut app, AppEv::Disconnected);
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request })),
    );
    assert!(!app.screens().onboarding.done(Stage::Update));
    assert_eq!(update_choice(&app), None);
}

// The update was stored: a lost connection afterwards changes nothing.
#[test]
fn a_lost_connection_leaves_a_stored_update_alone() {
    let (mut app, request) = update_running();
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request })),
    );
    send(&mut app, AppEv::Disconnected);
    assert!(app.screens().onboarding.done(Stage::Update));
}

#[test]
fn host_asks_update_is_retained_until_reconnect_has_failed() {
    use codex_tui_core::app::Ingress;
    use codex_tui_core::screens::agents;

    let (mut app, request) = update_running();
    let epoch = app.epoch();
    let reconnect = |ev| Ingress::new(epoch, AppEv::Screen(ScreenEv::Agents(ev)));
    app.ingest(reconnect(agents::Ev::Reconnecting))
        .expect("recoverable loss");
    assert_eq!(update_choice(&app), Some(UpdateChoice::UpdateNow));
    app.ingest(reconnect(agents::Ev::Notice(Some(
        agents::Notice::ReconnectFailed,
    ))))
    .expect("terminal failure");
    assert_eq!(update_choice(&app), None);
    let late = AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request }));
    assert!(app.ingest(Ingress::new(epoch, late)).is_none());
}

#[test]
fn host_asks_a_cancelled_login_still_cleans_up_its_late_server_attempt() {
    use codex_tui_core::app::Ingress;
    use codex_tui_core::screens::onboarding::auth::{
        AuthResult, DeviceCode, Intent as Auth, Method, Offer, Outcome,
    };

    let mut app = CodexApp::new();
    let epoch = app.attach();
    let facts = run(&mut app, AppEv::Connected);
    let request = facts
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::Facts { request },
                        ..
                    },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("launch facts request");
    app.ingest(Ingress::new(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request,
            facts: Facts {
                sign_in: Some(Offer {
                    chatgpt: true,
                    ..Offer::default()
                }),
                ..Facts::default()
            },
        })),
    ))
    .expect("launch facts");
    let login = |method| {
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            StartupIntent::SignIn(Auth::UseMethod(method)),
        )))
    };
    let request_of = |fx: AppFx| {
        fx.into_commands()
            .into_iter()
            .find_map(|command| match command {
                Command::Effect {
                    ask:
                        codex_tui_core::app::effects::Ask::Startup {
                            ask: Ask::Login { request, .. },
                            ..
                        },
                    ..
                } => Some(request),
                _ => None,
            })
            .expect("login request")
    };
    let old = request_of(
        app.ingest(Ingress::new(epoch, login(Method::DeviceCode)))
            .expect("first login"),
    );
    choose(&mut app, StartupIntent::SignIn(Auth::Cancel));
    let fresh = request_of(
        app.ingest(Ingress::new(epoch, login(Method::ChatGpt)))
            .expect("second login"),
    );
    let answer = |request, login_id: &str| {
        AppEv::Screen(ScreenEv::Onboarding(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Browser {
                login_id: login_id.to_owned(),
                url: "https://login.example".into(),
            },
        })))
    };
    let old_answer = AppEv::Screen(ScreenEv::Onboarding(Ev::Auth(AuthResult {
        request: old,
        outcome: Outcome::DeviceCode(DeviceCode {
            login_id: "old".into(),
            url: "https://login.example".into(),
            code: "ABCD-EFGH".into(),
        }),
    })));
    let stale = app
        .ingest(Ingress::new(epoch, old_answer.clone()))
        .expect("late login can require cleanup");
    assert!(
        matches!(stale.commands(), [Command::Effect { ask: codex_tui_core::app::effects::Ask::Startup { ask: Ask::CancelLogin { login_id }, .. }, .. }] if login_id == "old")
    );
    assert!(app.ingest(Ingress::new(epoch, old_answer)).is_none());
    let current = app
        .ingest(Ingress::new(epoch, answer(fresh, "fresh")))
        .expect("live attempt");
    assert!(matches!(
        current.commands(),
        [Command::Effect {
            ask: codex_tui_core::app::effects::Ask::Startup {
                ask: Ask::OpenBrowser { .. },
                ..
            },
            ..
        }]
    ));
}
