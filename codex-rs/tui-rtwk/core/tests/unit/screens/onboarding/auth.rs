use codex_tui_core::screens::onboarding::Ask;
use codex_tui_core::screens::onboarding::Request;
use codex_tui_core::screens::onboarding::auth::*;
use pretty_assertions::assert_eq;

fn offer() -> Offer {
    Offer {
        chatgpt: true,
        api_key: true,
        ..Offer::default()
    }
}

/// `widget_forced_chatgpt`: ChatGPT login forced, no API key.
fn forced_chatgpt() -> SignIn {
    SignIn::new(Offer {
        chatgpt: true,
        api_key: false,
        ..Offer::default()
    })
}

struct Run {
    next: Request,
    asks: Vec<Ask>,
}

impl Run {
    fn new() -> Run {
        Run {
            next: 0,
            asks: Vec::new(),
        }
    }

    fn intent(&mut self, sign_in: &mut SignIn, intent: Intent) {
        let mut next = self.next;
        let mut mint = || {
            next += 1;
            next
        };
        apply_intent(sign_in, intent, &mut mint, &mut self.asks);
        self.next = next;
    }

    fn result(&mut self, sign_in: &mut SignIn, request: Request, outcome: Outcome) {
        let mut next = self.next;
        let mut mint = || {
            next += 1;
            next
        };
        apply_result(
            sign_in,
            AuthResult { request, outcome },
            &mut mint,
            &mut self.asks,
        );
        self.next = next;
    }
}

fn device_code(login_id: &str) -> DeviceCode {
    DeviceCode {
        login_id: login_id.into(),
        url: "https://chatgpt.com/device".into(),
        code: "ABCD-EFGH".into(),
    }
}

// auth.rs:1525
#[test]
fn api_key_flow_disabled_when_chatgpt_forced() {
    let mut sign_in = forced_chatgpt();
    Run::new().intent(&mut sign_in, Intent::UseMethod(Method::ApiKey));
    assert_eq!(sign_in.error, Some(Error::ApiKeyDisabled));
    assert_eq!(sign_in.state, State::Pick);
}

// auth.rs:1616
#[test]
fn saving_api_key_is_blocked_when_chatgpt_forced() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::ApiKey { prefill: None };
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::SaveApiKey("sk-test".into()));
    assert_eq!(sign_in.error, Some(Error::ApiKeyDisabled));
    assert_eq!(sign_in.state, State::Pick);
    assert!(run.asks.is_empty());
    assert!(!sign_in.chatgpt_account);
}

// auth.rs:1633: a ChatGPT account of any kind (tokens, personal access
// token) is signed in already.
#[test]
fn existing_non_oauth_chatgpt_login_counts_as_signed_in() {
    for method in [Method::ChatGpt, Method::DeviceCode] {
        let mut sign_in = forced_chatgpt();
        sign_in.chatgpt_account = true;
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::UseMethod(method));
        assert_eq!(sign_in.state, State::Success);
        assert!(run.asks.is_empty());
    }
}

// auth.rs:1649
#[test]
fn cancel_active_attempt_resets_browser_login_state() {
    let mut sign_in = forced_chatgpt();
    sign_in.error = Some(Error::Server("still logging in".into()));
    sign_in.state = State::Browser {
        login_id: "login-1".into(),
        url: "https://auth.example.com".into(),
    };
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::Cancel);
    assert_eq!(sign_in.error, None);
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(
        run.asks,
        vec![Ask::CancelLogin {
            login_id: "login-1".into()
        }]
    );
}

// auth.rs:1668
#[test]
fn cancel_active_attempt_notifies_device_code_login() {
    let mut sign_in = forced_chatgpt();
    sign_in.error = Some(Error::Server("still logging in".into()));
    sign_in.state = State::DeviceCode {
        request: 1,
        code: Some(device_code("login-1")),
    };
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::Cancel);
    assert_eq!(sign_in.error, None);
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(
        run.asks,
        vec![Ask::CancelLogin {
            login_id: "login-1".into()
        }]
    );
}

// auth.rs:1408: the login completes while the browser is opening: the
// host answers the start, then the completion arrives.
#[test]
fn login_can_complete_while_browser_is_opening() {
    let mut sign_in = forced_chatgpt();
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::UseMethod(Method::ChatGpt));
    assert_eq!(
        run.asks,
        vec![Ask::Login {
            request: 1,
            login: Login::ChatGpt
        }]
    );
    run.result(
        &mut sign_in,
        1,
        Outcome::Browser {
            login_id: "login-1".into(),
            url: "https://example.test/authorize".into(),
        },
    );
    login_completed(&mut sign_in, Some("login-1"), true, None);
    assert_eq!(sign_in.state, State::SuccessMessage);
}

// auth.rs:1809, 1824
#[test]
fn auth_widget_suppresses_animations_when_device_code_is_visible() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 1,
        code: Some(device_code("login-1")),
    };
    assert!(sign_in.freezes_animations());
}

#[test]
fn auth_widget_suppresses_animations_while_requesting_device_code() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 1,
        code: None,
    };
    assert!(sign_in.freezes_animations());
}

// auth.rs:1835
#[test]
fn device_code_login_completion_advances_to_success_message() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 1,
        code: Some(device_code("login-1")),
    };
    login_completed(&mut sign_in, Some("login-1"), true, None);
    assert_eq!(sign_in.state, State::SuccessMessage);
}

// headless_chatgpt_login.rs:224 device_code_attempt_matches_only_for_matching_request_id
// and :238 set_device_code_state_for_active_attempt_updates_only_when_active:
// the code of the attempt shown is taken; one for another attempt is
// dropped and its login cancelled.
#[test]
fn set_device_code_state_for_active_attempt_updates_only_when_active() {
    let mut sign_in = forced_chatgpt();
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::UseMethod(Method::DeviceCode));
    run.result(&mut sign_in, 1, Outcome::DeviceCode(device_code("login-1")));
    assert_eq!(
        sign_in.state,
        State::DeviceCode {
            request: 1,
            code: Some(device_code("login-1"))
        }
    );

    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 2,
        code: None,
    };
    let mut run = Run::new();
    run.result(&mut sign_in, 1, Outcome::DeviceCode(device_code("login-1")));
    assert_eq!(
        sign_in.state,
        State::DeviceCode {
            request: 2,
            code: None
        }
    );
    assert_eq!(
        run.asks,
        vec![Ask::CancelLogin {
            login_id: "login-1".into()
        }]
    );
}

// headless_chatgpt_login.rs:286
#[test]
fn set_device_code_error_for_active_attempt_updates_only_when_active() {
    let failed = || Outcome::Failed(Error::Server("device code unavailable".into()));
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 1,
        code: None,
    };
    let mut run = Run::new();
    run.result(&mut sign_in, 1, failed());
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(
        sign_in.error,
        Some(Error::Server("device code unavailable".into()))
    );

    let mut sign_in = forced_chatgpt();
    sign_in.state = State::DeviceCode {
        request: 2,
        code: None,
    };
    run.result(&mut sign_in, 1, failed());
    assert!(matches!(sign_in.state, State::DeviceCode { .. }));
    assert_eq!(sign_in.error, None);
}

/// research/off-thread-state.md (a) item 1: a sign-in answer under a
/// request no longer awaited (cancelled, superseded) is dropped.
#[test]
fn stale_auth_results_are_dropped() {
    let mut sign_in = SignIn::new(offer());
    let mut run = Run::new();
    // A browser login started; an answer under another request is stale.
    run.intent(&mut sign_in, Intent::UseMethod(Method::ChatGpt));
    run.result(
        &mut sign_in,
        9,
        Outcome::Browser {
            login_id: "old".into(),
            url: "https://old".into(),
        },
    );
    assert_eq!(sign_in.state, State::Pick);
    run.result(&mut sign_in, 1, Outcome::Failed(Error::Server("x".into())));
    assert_eq!(sign_in.error, Some(Error::Server("x".into())));
    // An API key answer after the step moved on is dropped too.
    sign_in.error = None;
    run.result(&mut sign_in, 7, Outcome::ApiKeySaved);
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(sign_in.error, None);
}

/// Review O1: Enter twice within the round trip of the ChatGPT login
/// (state still `Pick`, the ask out) starts one login, not two.
#[test]
fn second_choose_while_a_login_is_awaited_is_ignored() {
    let mut sign_in = SignIn::new(offer());
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::UseMethod(Method::ChatGpt));
    run.intent(&mut sign_in, Intent::UseMethod(Method::ChatGpt));
    assert_eq!(
        run.asks,
        vec![Ask::Login {
            request: 1,
            login: Login::ChatGpt
        }]
    );
    // The one answer is the one awaited.
    run.result(
        &mut sign_in,
        1,
        Outcome::Browser {
            login_id: "L".into(),
            url: "https://u".into(),
        },
    );
    assert_eq!(
        sign_in.state,
        State::Browser {
            login_id: "L".into(),
            url: "https://u".into()
        }
    );
    assert_eq!(run.asks.len(), 1);
}

/// Review O1: a browser login answered under a request no longer
/// awaited is cancelled, as a stale device code is.
#[test]
fn stale_browser_outcome_cancels_the_login() {
    let mut sign_in = SignIn::new(offer());
    let mut run = Run::new();
    run.result(
        &mut sign_in,
        4,
        Outcome::Browser {
            login_id: "orphan".into(),
            url: "https://o".into(),
        },
    );
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(
        run.asks,
        vec![Ask::CancelLogin {
            login_id: "orphan".into()
        }]
    );
}

/// Review O1 (secrets): Esc on the API key entry while the save is out
/// forgets the pending key; its late answer is dropped.
#[test]
fn cancel_forgets_the_pending_api_key() {
    let mut sign_in = SignIn::new(offer());
    sign_in.state = State::ApiKey { prefill: None };
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::SaveApiKey("sk-secret".into()));
    assert!(sign_in.awaiting().is_some());
    run.intent(&mut sign_in, Intent::Cancel);
    assert_eq!(sign_in.awaiting(), None);
    assert_eq!(sign_in.state, State::Pick);
    run.result(&mut sign_in, 1, Outcome::ApiKeySaved);
    assert_eq!(sign_in.state, State::Pick);
}

/// `handle_api_key_entry_key_event`: an empty key is refused; a key is
/// asked to be saved, and the save's answer ends the step.
#[test]
fn api_key_save_asks_and_confirms() {
    let mut sign_in = SignIn::new(Offer {
        env_api_key: Some("sk-env".into()),
        ..offer()
    });
    let mut run = Run::new();
    run.intent(&mut sign_in, Intent::UseMethod(Method::ApiKey));
    assert_eq!(
        sign_in.state,
        State::ApiKey {
            prefill: Some("sk-env".into())
        }
    );
    run.intent(&mut sign_in, Intent::SaveApiKey("   ".into()));
    assert_eq!(sign_in.error, Some(Error::EmptyApiKey));
    run.intent(&mut sign_in, Intent::SaveApiKey(" sk-test ".into()));
    assert_eq!(sign_in.error, None);
    assert_eq!(
        run.asks,
        vec![Ask::Login {
            request: 1,
            login: Login::ApiKey("sk-test".into())
        }]
    );
    run.result(
        &mut sign_in,
        1,
        Outcome::Failed(Error::SaveFailed("denied".into())),
    );
    assert_eq!(sign_in.state, State::ApiKey { prefill: None });
    assert_eq!(sign_in.error, Some(Error::SaveFailed("denied".into())));
    run.intent(&mut sign_in, Intent::SaveApiKey("sk-test".into()));
    run.result(&mut sign_in, 2, Outcome::ApiKeySaved);
    assert_eq!(sign_in.state, State::ApiKeyConfigured);
    assert!(sign_in.done());
}

/// A failed ChatGPT login (`on_account_login_completed` with an error)
/// returns to the menu with the server's message; one for another login
/// is not this attempt's.
#[test]
fn failed_login_completion_returns_to_the_menu() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::Browser {
        login_id: "login-1".into(),
        url: "https://example.test".into(),
    };
    login_completed(&mut sign_in, Some("login-2"), true, None);
    assert!(matches!(sign_in.state, State::Browser { .. }));
    login_completed(&mut sign_in, None, true, None);
    assert!(matches!(sign_in.state, State::Browser { .. }));
    login_completed(&mut sign_in, Some("login-1"), false, Some("denied".into()));
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(sign_in.error, Some(Error::Server("denied".into())));
}

/// The success message waits for Enter; a provider chosen hands the
/// step to the provider step, and Esc there comes back.
#[test]
fn success_message_and_provider_choice() {
    let mut sign_in = forced_chatgpt();
    sign_in.state = State::SuccessMessage;
    assert!(!sign_in.done());
    Run::new().intent(&mut sign_in, Intent::Continue);
    assert_eq!(sign_in.state, State::Success);
    assert!(sign_in.done());

    let mut sign_in = SignIn::new(Offer {
        providers: Providers {
            bedrock: true,
            ..Providers::default()
        },
        ..offer()
    });
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gcp)),
    );
    assert_eq!(sign_in.state, State::Pick);
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Bedrock)),
    );
    assert_eq!(sign_in.state, State::Provider(Provider::Bedrock));
    assert!(sign_in.done());
    run.intent(&mut sign_in, Intent::Cancel);
    assert_eq!(sign_in.state, State::Pick);
}

fn gcp_sign_in() -> SignIn {
    SignIn::new(Offer {
        providers: Providers {
            gcp: true,
            ..Providers::default()
        },
        ..offer()
    })
}

fn gcp_next(sign_in: &mut SignIn, run: &mut Run, project: &str) {
    let fields = provider::gcp::Fields {
        project: project.into(),
        ..provider::gcp::Fields::default()
    };
    run.intent(
        sign_in,
        Intent::Provider(provider::Intent::Gcp(provider::gcp::Intent::Next(fields))),
    );
}

/// gcp_tests.rs gcp_menu_gating_and_setup_flow: choosing Google Cloud
/// opens its wizard; the credentials stage asks the setup and waits;
/// the answer writes it.
#[test]
fn the_gcp_wizard_runs_its_setup_through_an_ask() {
    let mut sign_in = gcp_sign_in();
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gcp)),
    );
    assert_eq!(sign_in.state, State::Provider(Provider::Gcp));
    assert!(sign_in.is_text_entry());
    for _ in 0..3 {
        gcp_next(&mut sign_in, &mut run, "test-project");
    }
    let (request, setup) = match run.asks.last() {
        Some(Ask::Login {
            request,
            login: Login::Provider(setup),
        }) => (*request, setup.clone()),
        other => panic!("no setup asked: {other:?}"),
    };
    assert!(matches!(setup, provider::Setup::Gcp(ref gcp) if gcp.project_id == "test-project"));
    assert!(!sign_in.is_text_entry(), "no keys while it is configured");
    // A refusal returns to the credentials with its message; the retry
    // asks again and the answer under the new number configures it.
    run.result(
        &mut sign_in,
        request,
        Outcome::Provider(Err("must be absolute".into())),
    );
    assert_eq!(sign_in.state, State::Provider(Provider::Gcp));
    assert!(sign_in.is_text_entry());
    gcp_next(&mut sign_in, &mut run, "test-project");
    let Some(Ask::Login { request: retry, .. }) = run.asks.last().cloned() else {
        panic!("no retry asked");
    };
    assert_ne!(retry, request);
    run.result(
        &mut sign_in,
        request,
        Outcome::Provider(Ok(provider::Answer::Written)),
    );
    assert_eq!(sign_in.state, State::Provider(Provider::Gcp), "stale");
    run.result(
        &mut sign_in,
        retry,
        Outcome::Provider(Ok(provider::Answer::Written)),
    );
    assert_eq!(sign_in.state, State::ProviderConfigured(Provider::Gcp));
    assert!(sign_in.done());
}

/// Esc on the first stage leaves the wizard for the menu.
#[test]
fn esc_on_the_first_gcp_stage_returns_to_the_menu() {
    let mut sign_in = gcp_sign_in();
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gcp)),
    );
    run.intent(&mut sign_in, Intent::ProviderBack);
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(sign_in.wizard, None);
}

/// The setup the last ask of the run is.
fn last_setup(run: &Run) -> (Request, provider::Setup) {
    match run.asks.last() {
        Some(Ask::Login {
            request,
            login: Login::Provider(setup),
        }) => (*request, setup.clone()),
        other => panic!("no setup asked: {other:?}"),
    }
}

fn gateway_sign_in() -> SignIn {
    SignIn::new(Offer {
        providers: Providers {
            gateway: true,
            ..Providers::default()
        },
        ..offer()
    })
}

fn one_gateway() -> provider::Answer {
    provider::Answer::Gateways(provider::gateway::Page {
        gateways: vec![provider::gateway::Saved {
            id: "office".into(),
            name: "Office".into(),
            variable: None,
        }],
        default_id: "openai".into(),
    })
}

/// The gateway's own sign-in (`SignInState::GatewayOAuth`) keeps the
/// step open until the gateway is configured.
#[test]
fn the_gateway_sign_in_ends_the_step_only_when_configured() {
    let mut sign_in = gateway_sign_in();
    for (state, done) in [
        (
            State::GatewayOAuth {
                provider_id: "office".into(),
                url: None,
            },
            false,
        ),
        (
            State::GatewayOAuthFailed {
                provider_id: "office".into(),
                message: "Gateway sign-in expired.".into(),
            },
            false,
        ),
        (State::GatewayConfigured, true),
    ] {
        sign_in.state = state;
        assert_eq!(sign_in.done(), done, "{:?}", sign_in.state);
    }
}

/// gateway_tests.rs select_a_saved_gateway_during_sign_in: choosing the
/// gateway opens its wizard, which reads the saved gateways at once;
/// only picking one ends the step, with its id.
#[test]
fn the_gateway_wizard_reads_the_saved_ones_and_ends_with_the_one_picked() {
    let mut sign_in = gateway_sign_in();
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gateway)),
    );
    assert_eq!(sign_in.state, State::Provider(Provider::Gateway));
    let (list, setup) = last_setup(&run);
    assert_eq!(setup, provider::Setup::GatewayList);
    run.result(&mut sign_in, list, Outcome::Provider(Ok(one_gateway())));
    run.intent(
        &mut sign_in,
        Intent::Provider(provider::Intent::Gateway(provider::gateway::Intent::Use(0))),
    );
    let (select, setup) = last_setup(&run);
    assert_eq!(setup, provider::Setup::GatewaySelect("office".into()));
    run.result(
        &mut sign_in,
        select,
        Outcome::Provider(Ok(provider::Answer::GatewaySelected)),
    );
    assert_eq!(sign_in.state, State::GatewaySelected("office".into()));
    assert_eq!(sign_in.wizard, None);
    assert!(sign_in.done());
}

/// Esc while the gateways are read leaves for the menu and the answer
/// that comes later is dropped.
#[test]
fn esc_while_the_gateways_load_leaves_and_drops_the_answer() {
    let mut sign_in = gateway_sign_in();
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gateway)),
    );
    let (list, _) = last_setup(&run);
    run.intent(&mut sign_in, Intent::ProviderBack);
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(sign_in.awaiting(), None);
    run.result(&mut sign_in, list, Outcome::Provider(Ok(one_gateway())));
    assert_eq!(sign_in.state, State::Pick);
    assert_eq!(sign_in.wizard, None);
}

/// gcp.rs: Esc while the credentials are validated does nothing; the
/// answer still arrives and is taken (it is not dropped with the key).
#[test]
fn esc_while_a_wizard_waits_keeps_its_answer() {
    let mut sign_in = gcp_sign_in();
    let mut run = Run::new();
    run.intent(
        &mut sign_in,
        Intent::UseMethod(Method::Provider(Provider::Gcp)),
    );
    for _ in 0..3 {
        gcp_next(&mut sign_in, &mut run, "test-project");
    }
    let (request, _) = last_setup(&run);
    run.intent(&mut sign_in, Intent::ProviderBack);
    assert!(sign_in.awaiting().is_some());
    run.result(
        &mut sign_in,
        request,
        Outcome::Provider(Ok(provider::Answer::Written)),
    );
    assert_eq!(sign_in.state, State::ProviderConfigured(Provider::Gcp));
}
