//! The gateway's device code at the end of the launch
//! (`device_login::start_if_needed`, `run_gateway_device_login`,
//! `handle_event_during_device_login`): asked once the steps are over and
//! before the launch's prompts, shown until `account/gatewayOAuth/changed`
//! ends that attempt, cancelled on the way out.

use super::*;
use codex_app_server_protocol::{
    GatewayOAuthChangedNotification, GatewayOAuthStatus, ServerNotification,
};
use codex_tui_core::app::{BackendEv, Command, Ingress};
use codex_tui_core::screens::onboarding::auth::DeviceCode;
use codex_tui_core::screens::onboarding::{Ask, Model};

/// Applies `ev`; the asks of the startup page it made.
fn asks_of(app: &mut CodexApp, ev: AppEv) -> Vec<Ask> {
    let fx = app
        .ingest(Ingress::new(app.epoch(), ev))
        .unwrap_or_default();
    app.assert_consistent();
    fx.into_commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Startup { ask, .. } => Some(ask),
            _ => None,
        })
        .collect()
}

fn ev(ev: Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(ev))
}

fn onboarding(app: &CodexApp) -> &Model {
    &app.screens().onboarding
}

fn changed(login_id: Option<&str>, status: GatewayOAuthStatus, error: Option<&str>) -> AppEv {
    AppEv::Backend(BackendEv::Notification(Box::new(
        ServerNotification::GatewayOAuthChanged(GatewayOAuthChangedNotification {
            login_id: login_id.map(str::to_owned),
            auth_url: None,
            provider_id: "company".into(),
            status,
            error: error.map(str::to_owned),
        }),
    )))
}

fn code() -> DeviceCode {
    DeviceCode {
        login_id: "device".into(),
        url: "https://login.example/device".into(),
        code: "ABCD-EFGH".into(),
    }
}

/// A launch whose facts are read, with no step due: the device code is asked.
fn launched() -> (CodexApp, u64) {
    let mut app = CodexApp::new();
    send(&mut app, AppEv::Connected);
    let asks = asks_of(
        &mut app,
        ev(Ev::Facts {
            request: 1,
            facts: Facts::default(),
        }),
    );
    let [
        Ask::GatewayDevice {
            request,
            thread: None,
        },
    ] = asks.as_slice()
    else {
        panic!("one launch device request: {asks:?}");
    };
    (app, *request)
}

/// The same, with the code on screen.
fn waiting() -> CodexApp {
    let (mut app, request) = launched();
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayDevice {
            request,
            code: Some(code()),
        }),
    );
    assert_eq!(asks, [], "the launch waits for the attempt");
    app
}

// `lib.rs`: the code is asked before the launch's prompts are read, and a
// server with nothing to sign in to goes straight on.
#[test]
fn no_code_goes_on_to_the_launch() {
    let (mut app, request) = launched();
    assert!(!onboarding(&app).due());
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayDevice {
            request,
            code: None,
        }),
    );
    assert!(matches!(asks.as_slice(), [Ask::Launch { request: launch }] if *launch != request));
    assert_eq!(onboarding(&app).device_code(), None);
}

// `run_gateway_device_login`: the screen shows the link and the code, and the
// launch waits.
#[test]
fn a_code_keeps_the_page_open_until_the_attempt_ends() {
    let app = waiting();
    assert_eq!(onboarding(&app).device_code(), Some(&code()));
    assert!(onboarding(&app).due(), "the page opens for the code");
    assert!(!onboarding(&app).launched());
}

#[test]
fn the_attempt_succeeding_goes_on_to_the_launch() {
    let mut app = waiting();
    let asks = asks_of(
        &mut app,
        changed(Some("device"), GatewayOAuthStatus::Succeeded, None),
    );
    assert!(matches!(asks.as_slice(), [Ask::Launch { .. }]));
    assert_eq!(onboarding(&app).device_code(), None);
}

// `handle_event_during_device_login`: only the attempt shown counts, and a
// start is not an end.
#[test]
fn other_attempts_and_a_start_change_nothing() {
    let mut app = waiting();
    for event in [
        changed(Some("old"), GatewayOAuthStatus::Failed, Some("Cancelled")),
        changed(Some("old"), GatewayOAuthStatus::Succeeded, None),
        changed(None, GatewayOAuthStatus::Succeeded, None),
        changed(Some("device"), GatewayOAuthStatus::Started, None),
    ] {
        assert_eq!(asks_of(&mut app, event), []);
    }
    assert_eq!(onboarding(&app).device_code(), Some(&code()));
}

// An attempt that fails (or is not ready) ends the launch with the server's
// message; the code stays until the app ends.
#[test]
fn a_failed_attempt_ends_the_launch_with_the_servers_message() {
    for (status, error, message) in [
        (GatewayOAuthStatus::Failed, Some("Denied"), "Denied"),
        (GatewayOAuthStatus::NotReady, None, "Gateway sign-in failed"),
    ] {
        let mut app = waiting();
        let asks = asks_of(&mut app, changed(Some("device"), status, error));
        assert_eq!(
            asks,
            [Ask::FailLaunch {
                message: message.into()
            }]
        );
        assert_eq!(onboarding(&app).device_code(), Some(&code()), "{message}");
        assert!(!onboarding(&app).launched());
    }
}

// `login_success_is_not_lost_while_waiting_for_the_device_code`: the end can
// come before the answer to the login.
#[test]
fn an_end_that_came_before_the_answer_is_not_lost() {
    let (mut app, request) = launched();
    assert_eq!(
        asks_of(
            &mut app,
            changed(Some("old"), GatewayOAuthStatus::Failed, Some("Cancelled"))
        ),
        []
    );
    assert_eq!(
        asks_of(
            &mut app,
            changed(Some("device"), GatewayOAuthStatus::Succeeded, None)
        ),
        []
    );
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayDevice {
            request,
            code: Some(code()),
        }),
    );
    assert!(matches!(asks.as_slice(), [Ask::Launch { request: launch }] if *launch != request));
}

// Quitting with the code shown cancels the attempt (`device_login::cancel`)
// and ends the launch.
#[test]
fn quitting_cancels_the_attempt() {
    let mut app = waiting();
    let asks = asks_of(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            StartupIntent::Exit,
        ))),
    );
    assert_eq!(asks, [Ask::CancelGatewayOAuth]);
    assert!(onboarding(&app).ends());
    assert_eq!(onboarding(&app).device_code(), None, "the page leaves");
    // A second quit (a held key) cancels nothing more.
    let asks = asks_of(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(
            StartupIntent::Exit,
        ))),
    );
    assert_eq!(asks, []);
}

// The answer of a ask no longer awaited is dropped; a connection lost asks
// again with the next one.
#[test]
fn a_stale_answer_is_dropped_and_a_lost_connection_asks_again() {
    let (mut app, request) = launched();
    assert_eq!(
        asks_of(
            &mut app,
            ev(Ev::GatewayDevice {
                request: request + 9,
                code: Some(code()),
            })
        ),
        []
    );
    assert_eq!(onboarding(&app).device_code(), None);
    send(&mut app, AppEv::Disconnected);
    let asks = asks_of(&mut app, AppEv::Connected);
    assert!(
        matches!(asks.as_slice(), [Ask::GatewayDevice { request: next, thread: None }] if *next != request)
    );
    // The answer of the lost connection's ask is not taken.
    assert_eq!(
        asks_of(
            &mut app,
            ev(Ev::GatewayDevice {
                request,
                code: Some(code()),
            })
        ),
        []
    );
    assert_eq!(onboarding(&app).device_code(), None);
}
