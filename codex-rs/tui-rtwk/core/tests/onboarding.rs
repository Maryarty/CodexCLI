//! The startup page through the app (ADR-003: it changes only through
//! `apply_app`): a connection lost while the trust is being saved.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Intent, apply_app};
use codex_tui_core::screens::onboarding::{
    Ev, Facts, Intent as StartupIntent, TrustAsk, TrustDecision,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};

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
