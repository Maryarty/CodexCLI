//! The startup page through the app (ADR-003: it changes only through
//! `apply_app`): a connection lost while the trust is being saved, or while
//! the update to run is being stored.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Intent, apply_app};
use codex_tui_core::screens::onboarding::{
    Ev, Facts, Intent as StartupIntent, Stage, TrustAsk, TrustDecision, UpdateChoice, UpdateOffer,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};

// `all.rs` declares this file by `#[path]` (`test-mods`), which makes it a module file of its
// directory: its children name their own.
#[path = "onboarding/device.rs"]
mod device;
#[path = "onboarding/gateway.rs"]
mod gateway;

fn send(app: &mut CodexApp, ev: AppEv) {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
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

fn saving() -> CodexApp {
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
    choose(&mut app, StartupIntent::Trust);
    assert_eq!(decision(&app), Some(TrustDecision::Saving));
    app
}

// A `TrustSaved` that never arrives (the connection dropped while saving)
// must not leave the step stuck: the choice is open again, and a late answer
// to the old write is dropped.
#[test]
fn a_connection_lost_while_saving_trust_opens_the_choice_again() {
    let mut app = saving();
    send(&mut app, AppEv::Disconnected);
    assert_eq!(decision(&app), None);
    choose(&mut app, StartupIntent::CancelTrust);
    assert_eq!(decision(&app), Some(TrustDecision::Cancelled));
}

#[test]
fn a_late_answer_to_the_write_of_a_lost_connection_is_dropped() {
    let mut app = saving();
    send(&mut app, AppEv::Disconnected);
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: 2 })),
    );
    assert_eq!(decision(&app), None);
}

// Without a write in flight, a lost connection changes nothing.
#[test]
fn a_lost_connection_leaves_a_made_choice_alone() {
    let mut app = saving();
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: 2 })),
    );
    send(&mut app, AppEv::Disconnected);
    assert_eq!(decision(&app), Some(TrustDecision::Trusted));
}

fn update_running() -> CodexApp {
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
    choose(
        &mut app,
        StartupIntent::ChooseUpdate(UpdateChoice::UpdateNow),
    );
    assert!(!app.screens().onboarding.done(Stage::Update));
    app
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
    let mut app = update_running();
    send(&mut app, AppEv::Disconnected);
    assert_eq!(update_choice(&app), None);
    choose(&mut app, StartupIntent::ChooseUpdate(UpdateChoice::Skip));
    assert!(app.screens().onboarding.done(Stage::Update));
}

#[test]
fn a_late_ack_of_the_update_of_a_lost_connection_is_dropped() {
    let mut app = update_running();
    send(&mut app, AppEv::Disconnected);
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request: 2 })),
    );
    assert!(!app.screens().onboarding.done(Stage::Update));
    assert_eq!(update_choice(&app), None);
}

// The update was stored: a lost connection afterwards changes nothing.
#[test]
fn a_lost_connection_leaves_a_stored_update_alone() {
    let mut app = update_running();
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request: 2 })),
    );
    send(&mut app, AppEv::Disconnected);
    assert!(app.screens().onboarding.done(Stage::Update));
}
