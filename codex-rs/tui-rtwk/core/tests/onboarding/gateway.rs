//! The selected gateway's own sign-in (`authenticate_gateway`): it runs after
//! the steps, on the sign-in step's states, through the host's asks.

use super::*;
use codex_tui_core::app::Command;
use codex_tui_core::screens::onboarding::auth::{
    AuthResult, Intent as AuthIntent, Login, Offer, Outcome, State,
};
use codex_tui_core::screens::onboarding::gateway::{Logins, Phase};
use codex_tui_core::screens::onboarding::{Ask, Model};

/// Applies `ev`; the asks of the startup page it made.
fn asks_of(app: &mut CodexApp, ev: AppEv) -> Vec<Ask> {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx.into_commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: codex_tui_core::app::effects::Ask::Startup { ask, .. },
                ..
            } => Some(ask),
            _ => None,
        })
        .collect()
}

fn ev(ev: Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(ev))
}

fn intent(app: &mut CodexApp, intent: StartupIntent) -> Vec<Ask> {
    asks_of(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(intent))),
    )
}

fn onboarding(app: &CodexApp) -> &Model {
    &app.screens().onboarding
}

fn state(app: &CodexApp) -> State {
    onboarding(app).sign_in.as_ref().unwrap().state.clone()
}

fn offer(chatgpt: bool) -> Offer {
    Offer {
        chatgpt,
        api_key: true,
        ..Offer::default()
    }
}

/// The sign-in step ended on the gateway `office`; `trust` also asks.
fn selected(trust: bool) -> (CodexApp, Vec<Ask>) {
    let mut app = CodexApp::new();
    send(&mut app, AppEv::Connected);
    let facts = Facts {
        sign_in: Some(offer(true)),
        trust: trust.then(|| TrustAsk {
            cwd: "/work".into(),
            target: "/work".into(),
            ..TrustAsk::default()
        }),
        ..Facts::default()
    };
    asks_of(&mut app, ev(Ev::Facts { request: 1, facts }));
    let asks = asks_of(
        &mut app,
        ev(Ev::SignInState(State::GatewaySelected("office".into()))),
    );
    (app, asks)
}

fn select_request(asks: &[Ask]) -> u64 {
    match asks {
        [
            Ask::SelectGateway {
                request,
                provider_id,
            },
        ] if provider_id == "office" => *request,
        other => panic!("not the gateway's start: {other:?}"),
    }
}

/// The server started: its account and provider are read.
fn started(app: &mut CodexApp, request: u64) -> u64 {
    match asks_of(app, ev(Ev::GatewayStarted { request })).as_slice() {
        [Ask::GatewayLogins { request, .. }] => *request,
        other => panic!("not the logins' read: {other:?}"),
    }
}

fn logins(primary: bool, oauth: bool, chatgpt: bool) -> Logins {
    Logins {
        primary,
        oauth,
        offer: offer(chatgpt),
    }
}

fn oauth_request(asks: &[Ask]) -> u64 {
    match asks {
        [
            Ask::GatewayOAuth {
                request,
                provider_id,
            },
        ] if provider_id == "office" => *request,
        other => panic!("not the gateway's sign-in: {other:?}"),
    }
}

/// Up to the gateway's OAuth, asked.
fn signing_in() -> (CodexApp, u64) {
    let (mut app, asks) = selected(false);
    let request = started(&mut app, select_request(&asks));
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayLogins {
            request,
            logins: logins(false, true, true),
        }),
    );
    let request = oauth_request(&asks);
    (app, request)
}

// The oracle's onboarding screen ends first (the trust is asked on it), then
// the gateway's server starts and its sign-in runs: the step stays on screen
// meanwhile.
#[test]
fn the_gateway_signs_in_after_the_other_steps() {
    let (mut app, asks) = selected(true);
    assert_eq!(asks, vec![], "the trust is asked first");
    assert_eq!(onboarding(&app).gateway(), &Phase::Idle);
    let asks = intent(&mut app, StartupIntent::Trust);
    let [Ask::WriteTrust { request, .. }] = asks.as_slice() else {
        panic!("the trust is not written: {asks:?}");
    };
    let asks = asks_of(&mut app, ev(Ev::TrustSaved { request: *request }));
    let request = select_request(&asks);
    assert!(!onboarding(&app).done(Stage::SignIn));
    assert!(onboarding(&app).due(), "the page stays for the gateway");
    started(&mut app, request);
}

#[test]
fn host_asks_setup_ack_survives_a_new_transport_and_checks_the_full_continuation() {
    use codex_tui_core::app::Ingress;
    use codex_tui_core::ids::BackendId;

    let (mut app, asks) = selected(false);
    let request = select_request(&asks);
    let primary = BackendId::default();
    let cloud = BackendId(1);
    let epoch = app.epoch_of(primary);
    let cloud_epoch = app.attach_backend(cloud);
    let answer = || ev(Ev::GatewayStarted { request });
    let wrong = ev(Ev::Launched {
        request,
        prompts: Vec::new(),
    });
    assert!(
        app.ingest(Ingress::of(cloud, cloud_epoch, answer()))
            .is_none()
    );
    assert!(app.ingest(Ingress::new(epoch, wrong)).is_none());
    assert_eq!(
        onboarding(&app).gateway(),
        &Phase::Starting {
            request,
            id: "office".into()
        }
    );

    app.ingest(Ingress::transport_replaced(primary, epoch, epoch.next()))
        .expect("a new setup transport");
    let effects = app
        .ingest(Ingress::new(epoch, answer()))
        .expect("the setup acknowledgement");
    assert!(effects.commands().iter().any(|command| matches!(
        command,
        Command::Effect { ask: codex_tui_core::app::effects::Ask::Startup { ask: Ask::GatewayLogins { request: next, .. }, .. }, .. } if *next != request
    )));
    assert!(app.ingest(Ingress::new(epoch, answer())).is_none());
}

#[test]
fn host_asks_setup_ack_dies_with_its_connector() {
    use codex_tui_core::app::Ingress;

    let (mut app, asks) = selected(false);
    let request = select_request(&asks);
    let epoch = app.epoch();
    app.ingest(Ingress::new(epoch, AppEv::Disconnected))
        .expect("connector loss");
    let ack = ev(Ev::GatewayStarted { request });
    assert!(app.ingest(Ingress::new(epoch, ack.clone())).is_none());
    let new_epoch = app.attach();
    assert!(app.ingest(Ingress::new(new_epoch, ack)).is_none());
}

#[test]
fn host_asks_a_direct_connector_attach_reissues_the_old_setup_owner() {
    use codex_tui_core::app::Ingress;

    let (mut app, asks) = selected(false);
    let old = select_request(&asks);
    let epoch = app.attach();
    let fx = app
        .ingest(Ingress::new(epoch, AppEv::Connected))
        .expect("replacement connector");
    let asks = fx
        .into_commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: ask @ Ask::SelectGateway { .. },
                        ..
                    },
                ..
            } => Some(ask),
            _ => None,
        })
        .collect::<Vec<_>>();
    let new = select_request(&asks);
    assert_ne!(old, new);
    assert!(
        app.ingest(Ingress::new(epoch, ev(Ev::GatewayStarted { request: old })))
            .is_none()
    );
    assert!(
        app.ingest(Ingress::new(epoch, ev(Ev::GatewayStarted { request: new })))
            .is_some()
    );
}

#[test]
fn a_gateway_that_needs_no_login_goes_on_to_the_launch() {
    let (mut app, asks) = selected(false);
    let request = started(&mut app, select_request(&asks));
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayLogins {
            request,
            logins: logins(false, false, true),
        }),
    );
    assert!(matches!(
        asks.as_slice(),
        [Ask::ReloadConfig, Ask::GatewayDevice { .. }]
    ));
    assert!(onboarding(&app).done(Stage::SignIn));
}

#[test]
fn the_gateways_oauth_succeeds_and_the_launch_follows() {
    let (mut app, request) = signing_in();
    assert_eq!(
        state(&app),
        State::GatewayOAuth {
            provider_id: "office".into(),
            url: None
        }
    );
    assert!(!onboarding(&app).done(Stage::SignIn));
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayOAuth {
            request,
            result: Ok(()),
        }),
    );
    assert_eq!(state(&app), State::GatewayConfigured);
    assert!(matches!(
        asks.as_slice(),
        [Ask::ReloadConfig, Ask::GatewayDevice { .. }]
    ));
}

#[test]
fn a_failed_oauth_waits_for_enter_and_asks_again_under_a_new_number() {
    let (mut app, request) = signing_in();
    asks_of(
        &mut app,
        ev(Ev::GatewayOAuth {
            request,
            result: Err("Gateway sign-in expired.".into()),
        }),
    );
    assert_eq!(
        state(&app),
        State::GatewayOAuthFailed {
            provider_id: "office".into(),
            message: "Gateway sign-in expired.".into()
        }
    );
    assert!(!onboarding(&app).done(Stage::SignIn));
    let asks = intent(&mut app, StartupIntent::RetryGatewayOAuth);
    let again = oauth_request(&asks);
    assert_ne!(again, request);
    assert!(matches!(state(&app), State::GatewayOAuth { url: None, .. }));
    // The answer of the first attempt comes late: dropped.
    asks_of(
        &mut app,
        ev(Ev::GatewayOAuth {
            request,
            result: Ok(()),
        }),
    );
    assert!(matches!(state(&app), State::GatewayOAuth { .. }));
    asks_of(
        &mut app,
        ev(Ev::GatewayOAuth {
            request: again,
            result: Ok(()),
        }),
    );
    assert_eq!(state(&app), State::GatewayConfigured);
}

#[test]
fn enter_retries_only_a_failed_sign_in() {
    let (mut app, _) = signing_in();
    assert_eq!(intent(&mut app, StartupIntent::RetryGatewayOAuth), vec![]);
    let (mut app, _) = selected(false);
    assert_eq!(intent(&mut app, StartupIntent::RetryGatewayOAuth), vec![]);
}

// `on_gateway_oauth_changed`: the page is shown and opened once for the
// attempt shown, for its provider only.
#[test]
fn the_page_to_finish_in_opens_once_for_the_attempt_shown() {
    let (mut app, _) = signing_in();
    let changed = |provider: &str, url: Option<&str>| Ev::GatewayOAuthChanged {
        provider_id: provider.into(),
        auth_url: url.map(str::to_owned),
    };
    assert_eq!(asks_of(&mut app, ev(changed("office", None))), vec![]);
    assert_eq!(
        asks_of(&mut app, ev(changed("other", Some("https://other/a")))),
        vec![]
    );
    assert_eq!(
        asks_of(&mut app, ev(changed("office", Some("https://office/a")))),
        vec![Ask::OpenBrowser {
            url: "https://office/a".into()
        }]
    );
    assert_eq!(
        state(&app),
        State::GatewayOAuth {
            provider_id: "office".into(),
            url: Some("https://office/a".into())
        }
    );
    assert_eq!(
        asks_of(&mut app, ev(changed("office", Some("https://office/a")))),
        vec![]
    );
    assert_eq!(
        asks_of(&mut app, ev(changed("office", Some("https://office/b")))),
        vec![Ask::OpenBrowser {
            url: "https://office/b".into()
        }]
    );
}

#[test]
fn quitting_from_the_oauth_cancels_it_on_the_server() {
    let (mut app, _) = signing_in();
    let asks = intent(&mut app, StartupIntent::Exit);
    assert_eq!(asks, vec![Ask::CancelGatewayOAuth]);
    // A failed one is cancelled too (any sign-in of the gateway's).
    let (mut app, request) = signing_in();
    asks_of(
        &mut app,
        ev(Ev::GatewayOAuth {
            request,
            result: Err("no".into()),
        }),
    );
    assert_eq!(
        intent(&mut app, StartupIntent::Exit),
        vec![Ask::CancelGatewayOAuth]
    );
    // Quitting before it started cancels nothing.
    let (mut app, _) = selected(false);
    assert_eq!(intent(&mut app, StartupIntent::Exit), vec![]);
}

// `Login::Primary`: ChatGPT first (when allowed); the gateway's OAuth starts
// when it is signed in.
#[test]
fn the_primary_login_runs_before_the_gateways() {
    let (mut app, asks) = selected(false);
    let request = started(&mut app, select_request(&asks));
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayLogins {
            request,
            logins: logins(true, true, true),
        }),
    );
    let [Ask::Login { request, login }] = asks.as_slice() else {
        panic!("not the ChatGPT login: {asks:?}");
    };
    assert_eq!(*login, Login::ChatGpt);
    let asks = asks_of(
        &mut app,
        ev(Ev::Auth(AuthResult {
            request: *request,
            outcome: Outcome::Browser {
                login_id: "l1".into(),
                url: "https://chatgpt/auth".into(),
            },
        })),
    );
    assert_eq!(
        asks,
        vec![Ask::OpenBrowser {
            url: "https://chatgpt/auth".into()
        }]
    );
    assert!(!onboarding(&app).done(Stage::SignIn));
    asks_of(
        &mut app,
        ev(Ev::LoginCompleted {
            login_id: Some("l1".into()),
            success: true,
            error: None,
        }),
    );
    assert_eq!(state(&app), State::SuccessMessage);
    let asks = intent(&mut app, StartupIntent::SignIn(AuthIntent::Continue));
    oauth_request(&asks);
    assert!(matches!(state(&app), State::GatewayOAuth { .. }));
}

#[test]
fn an_api_key_is_asked_when_chatgpt_is_not_allowed() {
    let (mut app, asks) = selected(false);
    let request = started(&mut app, select_request(&asks));
    let asks = asks_of(
        &mut app,
        ev(Ev::GatewayLogins {
            request,
            logins: logins(true, false, false),
        }),
    );
    assert_eq!(asks, vec![]);
    assert!(matches!(state(&app), State::ApiKey { .. }));
    assert!(!onboarding(&app).done(Stage::SignIn));
    let asks = intent(
        &mut app,
        StartupIntent::SignIn(AuthIntent::SaveApiKey("sk-1".into())),
    );
    let [Ask::Login { request, .. }] = asks.as_slice() else {
        panic!("the key is not saved: {asks:?}");
    };
    let asks = asks_of(
        &mut app,
        ev(Ev::Auth(AuthResult {
            request: *request,
            outcome: Outcome::ApiKeySaved,
        })),
    );
    assert!(matches!(
        asks.as_slice(),
        [Ask::ReloadConfig, Ask::GatewayDevice { .. }]
    ));
}

#[test]
fn answers_under_another_number_are_dropped() {
    let (mut app, asks) = selected(false);
    let request = select_request(&asks);
    assert_eq!(
        asks_of(
            &mut app,
            ev(Ev::GatewayStarted {
                request: request + 9
            })
        ),
        vec![]
    );
    let reading = started(&mut app, request);
    assert_eq!(
        asks_of(
            &mut app,
            ev(Ev::GatewayLogins {
                request: reading + 9,
                logins: logins(false, true, true)
            })
        ),
        vec![]
    );
    assert!(matches!(state(&app), State::GatewaySelected(_)));
}

// The connection lost while the server starts: the ask is made again with
// the next connection.
#[test]
fn a_start_lost_with_the_connection_is_asked_again() {
    let (mut app, _) = selected(false);
    send(&mut app, AppEv::Disconnected);
    assert_eq!(onboarding(&app).gateway(), &Phase::Idle);
    let asks = asks_of(&mut app, AppEv::Connected);
    select_request(&asks);
}
