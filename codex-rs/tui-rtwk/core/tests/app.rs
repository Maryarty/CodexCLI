//! When the chat thread starts (oracle: after the startup screens, `App::run`
//! creates the session once onboarding, trust and the launch prompts are
//! done). A host that answers the startup facts has the app wait for
//! `Ev::Launched`; without one the thread starts on connect.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, Session, apply_app};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Facts, Intent as StartupIntent, TrustAsk, TrustDecision,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use codex_tui_core::threads::UserMessage;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn starts(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|c| matches!(c, Command::StartThread { .. }))
        .count()
}

fn launch_request(fx: &AppFx) -> Option<u64> {
    fx.commands().iter().find_map(|c| match c {
        Command::Startup {
            ask: Ask::Launch { request },
            ..
        } => Some(*request),
        _ => None,
    })
}

/// The launch ask the host gets once the steps are over: after the gateway's
/// device code (none here) is asked for.
fn launch_ask(app: &mut CodexApp, fx: &AppFx) -> u64 {
    if let Some(request) = launch_request(fx) {
        return request;
    }
    let device = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::Startup {
                ask: Ask::GatewayDevice { request },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("the device code is asked before the launch");
    let fx = run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
            request: device,
            code: None,
        })),
    );
    assert_eq!(starts(&fx), 0);
    launch_request(&fx).expect("then the launch")
}

fn facts(app: &mut CodexApp, facts: Facts) -> AppFx {
    run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { request: 1, facts })),
    )
}

fn trust_facts() -> Facts {
    Facts {
        trust: Some(TrustAsk {
            cwd: "/work".into(),
            target: "/work".into(),
            ..TrustAsk::default()
        }),
        ..Facts::default()
    }
}

fn launched(app: &mut CodexApp, request: u64) -> AppFx {
    run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Launched {
            request,
            prompts: Vec::new(),
        })),
    )
}

fn intent(app: &mut CodexApp, intent: StartupIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(intent))),
    )
}

fn gated() -> CodexApp {
    CodexApp::new().with_start_after_launch(true)
}

#[test]
fn without_a_startup_host_the_thread_starts_on_connect() {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 1);
}

#[test]
fn a_startup_screen_holds_the_thread_until_the_launch_is_known() {
    let mut app = gated();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 0, "{:?}", fx.commands());
    assert_eq!(app.session().status, Session::Starting);
    let fx = facts(&mut app, trust_facts());
    assert_eq!(starts(&fx), 0);
    intent(&mut app, StartupIntent::Trust);
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: 2 })),
    );
    assert_eq!(starts(&fx), 0, "the prompts are still to be read");
    let request = launch_ask(&mut app, &fx);
    let fx = launched(&mut app, request);
    assert_eq!(starts(&fx), 1, "{:?}", fx.commands());
}

#[test]
fn with_no_step_to_show_the_thread_starts_when_the_launch_is_known() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    assert_eq!(starts(&fx), 0);
    let request = launch_ask(&mut app, &fx);
    assert_eq!(starts(&launched(&mut app, request)), 1);
}

#[test]
fn a_stale_launch_answer_starts_nothing() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    assert_eq!(starts(&launched(&mut app, request + 7)), 0);
}

#[test]
fn a_cancelled_trust_never_opens_a_thread() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    facts(&mut app, trust_facts());
    let fx = intent(&mut app, StartupIntent::CancelTrust);
    assert_eq!(starts(&fx), 0);
    assert_eq!(
        app.screens().onboarding.trust.as_ref().unwrap().decision,
        Some(TrustDecision::Cancelled)
    );
    assert!(launch_request(&fx).is_none());
}

// The composer's asks before the first thread (BeforeThread) wait in order
// and go out when the held thread starts.
#[test]
fn asks_made_during_the_startup_steps_drain_in_order_when_the_thread_starts() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    for text in ["first", "second"] {
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Submit {
                thread: None,
                message: UserMessage::text(text),
            }),
        );
        assert!(fx.commands().is_empty());
    }
    let fx = launched(&mut app, request);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("the thread starts");
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("t-1"))),
    );
    let sent: Vec<String> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(sent.first().map(String::as_str), Some("first"), "{sent:?}");
}

// A new connection after the launch is known has nothing left to wait for.
#[test]
fn a_reconnect_after_the_launch_starts_the_thread_at_once() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    launched(&mut app, request);
    run(&mut app, AppEv::Disconnected);
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 1, "{:?}", fx.commands());
}
