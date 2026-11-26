//! Screen tests of the startup page's sign-in step (S02) over the app and
//! the fake host: what its oracle shard (`tests/oracle/signin.rs`, entry
//! screens playing fixtures) cannot name: the logins asked of the host,
//! their answers by number, the held key's guard on a terminal reporting
//! repeats and releases (kitty keyboard).

use std::time::Duration;

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::auth::provider::{Answer, Setup, gcp};
use codex_tui_core::screens::onboarding::auth::{AuthResult, Login, Offer, Outcome, Providers};
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, Welcome};

use crate::common::Harness;

const PRESS_ENTER: &[u8] = b"\x1b[13u";
const REPEAT_ENTER: &[u8] = b"\x1b[13;1:2u";
const RELEASE_ENTER: &[u8] = b"\x1b[13;1:3u";

fn logins(h: &Harness) -> Vec<Ask> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: ask @ (Ask::Login { .. } | Ask::CancelLogin { .. }),
                        ..
                    },
                ..
            } => Some(ask),
            _ => None,
        })
        .collect()
}

fn login_request(asks: &[Ask], expected: Login) -> u64 {
    match asks {
        [Ask::Login { request, login }] if *login == expected => *request,
        other => panic!("expected one {expected:?} login, got {other:?}"),
    }
}

fn ev(ev: Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(ev))
}

/// The app launched on a sign-in due, offering `offer`, on a driven harness
/// (a settle on the real clock sleeps 100 ms).
fn launch(offer: Offer) -> (Harness, ConnEpoch) {
    let mut h = crate::common::start_driven(false, crate::common::WIDTH, crate::common::HEIGHT);
    h.settle();
    let Some(epoch) = h.fake.epoch() else {
        panic!("the app attached no backend");
    };
    h.answer_startup_facts(Facts {
        sign_in: Some(offer),
        ..Facts::default()
    });
    h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
    (h, epoch)
}

fn offer() -> Offer {
    Offer {
        chatgpt: true,
        api_key: true,
        ..Offer::default()
    }
}

fn input(h: &mut Harness, bytes: &[u8]) {
    h.t.input(bytes);
    h.t.pump_for(Duration::from_millis(20));
}

/// ChatGPT: the login is asked of the host under a number; an answer under
/// another number is dropped; the browser link shows, Esc cancels the
/// login by its id, and the completion signs in (oracle
/// `start_chatgpt_login`, `handle_key_event` Esc, `on_account_login_completed`).
#[test]
fn chatgpt_sign_in_runs_through_the_host_and_drops_stale_results() {
    let (mut h, epoch) = launch(offer());
    h.press("enter");
    h.settle();
    let request = login_request(&logins(&h), Login::ChatGpt);
    let browser = |request| {
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Browser {
                login_id: "login-1".into(),
                url: "https://auth.example/login".into(),
            },
        }))
    };
    h.fake.inject(epoch, browser(request + 1));
    h.settle();
    assert!(
        !h.t.screen().contains("Finish signing in"),
        "a stale result opens nothing:\n{}",
        h.t.screen()
    );
    // A reply for an unissued request has no login to cancel. A late result for an issued,
    // cancelled attempt is covered by the core's registered-login cleanup regression.
    assert_eq!(login_request(&logins(&h), Login::ChatGpt), request);
    h.fake.inject(epoch, browser(request));
    h.wait_for("the browser link", |s| {
        s.contains("Finish signing in via your browser") && s.contains("https://auth.example/login")
    });
    h.press("esc");
    h.wait_for("the menu again", |s| s.contains("Sign in with ChatGPT"));
    h.settle();
    assert_eq!(
        logins(&h)[1..],
        [Ask::CancelLogin {
            login_id: "login-1".into(),
        }]
    );
    h.press("enter");
    h.settle();
    let fresh = login_request(&logins(&h)[2..], Login::ChatGpt);
    assert_ne!(fresh, request);
    h.fake.inject(epoch, browser(fresh));
    h.wait_for("the browser link", |s| s.contains("Finish signing in"));
    h.fake.inject(
        epoch,
        ev(Ev::LoginCompleted {
            login_id: Some("login-1".into()),
            success: true,
            error: None,
        }),
    );
    h.wait_for("the success message", |s| {
        s.contains("Signed in with your ChatGPT account") && s.contains("You're in control")
    });
}

/// The oracle's `held_enter_requires_a_fresh_confirmation_to_save_a_prefilled_api_key`
/// on a terminal reporting repeats and releases: the Enter that opened
/// "another way" and then the key entry is held back until released, and
/// the prefilled key is saved by a fresh press; a key of another kind ends
/// the guard too.
#[test]
fn a_held_enter_saves_the_prefilled_key_only_when_fresh() {
    let (mut h, epoch) = launch(Offer {
        env_api_key: Some("sk-env".into()),
        ..offer()
    });
    h.press("down down");
    input(&mut h, PRESS_ENTER);
    h.wait_for("another way", |s| s.contains("> 1. Use an OpenAI API key"));
    input(&mut h, REPEAT_ENTER);
    input(&mut h, PRESS_ENTER);
    assert!(
        !h.t.screen().contains("Press enter to save"),
        "a held Enter does not open the entry:\n{}",
        h.t.screen()
    );
    input(&mut h, RELEASE_ENTER);
    input(&mut h, PRESS_ENTER);
    h.wait_for("the key entry", |s| {
        s.contains("Detected OPENAI_API_KEY environment variable.")
    });
    input(&mut h, REPEAT_ENTER);
    input(&mut h, PRESS_ENTER);
    h.settle();
    assert_eq!(logins(&h), [], "a held Enter does not save");
    h.press("left");
    input(&mut h, PRESS_ENTER);
    h.settle();
    let request = login_request(&logins(&h), Login::ApiKey("sk-env".into()));
    h.fake.inject(
        epoch,
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::ApiKeySaved,
        })),
    );
    // The step is done: the page goes on (here past its last step).
    h.wait_for("the sign-in done", |s| !s.contains("Press enter to save"));
}

/// The row the welcome line is on.
fn welcome_row(screen: &str) -> Option<usize> {
    screen
        .lines()
        .position(|line| line.contains("Welcome to Codex"))
}

/// Under the welcome logo, the link of a login hides the logo
/// (`should_suppress_animations`), and a quit key cancels the login on the
/// way out (`cancel_auth_if_active`, `keys::QUIT`).
#[test]
fn a_login_link_hides_the_logo_and_a_quit_cancels_the_login() {
    let mut h = crate::common::start_sized(false, 80, 44);
    h.settle();
    let epoch = h.fake.epoch().expect("attached");
    h.answer_startup_facts(Facts {
        welcome: Some(Welcome { animations: true }),
        sign_in: Some(offer()),
        ..Facts::default()
    });
    h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
    assert_eq!(welcome_row(&h.t.screen()), Some(22), "the logo above it");
    h.press("enter");
    h.settle();
    let request = login_request(&logins(&h), Login::ChatGpt);
    h.fake.inject(
        epoch,
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Browser {
                login_id: "login-1".into(),
                url: "https://auth.example/login".into(),
            },
        })),
    );
    h.wait_for("the browser link", |s| s.contains("Finish signing in"));
    assert_eq!(
        welcome_row(&h.t.screen()),
        Some(0),
        "no logo:\n{}",
        h.t.screen()
    );
    h.press("ctrl+c");
    h.settle();
    assert_eq!(
        logins(&h)[1..],
        [Ask::CancelLogin {
            login_id: "login-1".into(),
        }]
    );
}

/// The logo rows of the screen (the stage above the welcome line).
fn logo_rows(screen: &str) -> String {
    screen.lines().take(21).collect::<Vec<_>>().join("\n")
}

/// While the API key is typed the welcome logo stops and fades
/// (`Presentation::Faded`, `text_entry_context().active`): it no longer
/// moves, and leaving the entry lets it play on. Review O3.
#[test]
fn the_welcome_logo_stops_and_fades_while_the_api_key_is_typed() {
    let mut h = crate::common::start_sized(false, 80, 44);
    h.settle();
    h.answer_startup_facts(Facts {
        welcome: Some(Welcome { animations: true }),
        sign_in: Some(offer()),
        ..Facts::default()
    });
    h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
    // The menu: the logo plays.
    h.t.pump_for(Duration::from_millis(500));
    let (before, after) = (logo_rows(&h.t.screen()), {
        h.t.pump_for(Duration::from_millis(500));
        logo_rows(&h.t.screen())
    });
    assert_ne!(before, after, "the logo spins on the menu");
    // The key entry: it fades (400 ms) and stops.
    h.press("3");
    h.settle();
    h.press("1");
    h.wait_for("the key entry", |s| s.contains("Press enter to save"));
    h.t.pump_for(Duration::from_millis(600));
    let (before, after) = (logo_rows(&h.t.screen()), {
        h.t.pump_for(Duration::from_millis(500));
        logo_rows(&h.t.screen())
    });
    assert_eq!(
        before, after,
        "the logo stands still while the key is typed"
    );
    // Esc: back on the menu, the logo plays on.
    h.press("esc");
    h.wait_for("the menu", |s| s.contains("Sign in another way"));
    h.t.pump_for(Duration::from_millis(600));
    let (before, after) = (logo_rows(&h.t.screen()), {
        h.t.pump_for(Duration::from_millis(500));
        logo_rows(&h.t.screen())
    });
    assert_ne!(before, after, "the logo plays again");
}

/// The rows of the screen with their trailing blanks cut, as the oracle's
/// snapshots hold them.
fn rows(screen: &str) -> Vec<String> {
    screen
        .lines()
        .map(|line| line.trim_end().to_owned())
        .collect()
}

/// Google Cloud (oracle `gcp_menu_gating_and_setup_flow`, its host half):
/// the credentials' Enter asks the host to set it up under a number with
/// what the fields hold; the server's refusal returns to the credentials
/// with its message under the narrow screen's text (`gcp_setup_error_narrow`);
/// the answer under the new number configures it (`gcp_configured`).
#[test]
fn gcp_setup_is_asked_of_the_host_refused_and_retried() {
    let (mut h, epoch) = launch(Offer {
        providers: Providers {
            gcp: true,
            ..Providers::default()
        },
        ..offer()
    });
    h.press("3");
    h.settle();
    h.press("2");
    h.wait_for("the project entry", |s| {
        s.contains("Google Cloud project ID")
    });
    // The Enter that opened it is held back a second.
    h.t.pump_for(Duration::from_millis(1100));
    input(&mut h, b"test-project");
    h.press("enter enter");
    h.wait_for("the credentials", |s| s.contains("Credential file"));
    input(&mut h, b"relative-adc.json");
    h.t.resize(40, 18);
    h.press("enter");
    h.settle();
    let setup = |credential: Option<&str>| {
        Login::Provider(Setup::Gcp(gcp::Params {
            project_id: "test-project".into(),
            location: None,
            credential_file: credential.map(str::to_owned),
        }))
    };
    let request = login_request(&logins(&h), setup(Some("relative-adc.json")));
    assert!(
        h.t.screen().contains("Validating credentials and saving"),
        "{}",
        h.t.screen()
    );
    let answer = |request, outcome| {
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Provider(outcome),
        }))
    };
    h.fake.inject(
        epoch,
        answer(
            request,
            Err("provider credential_file must be an absolute path".into()),
        ),
    );
    h.wait_for("the refusal", |s| s.contains("absolute path"));
    // gcp_setup_error_narrow.snap
    let want = "\
> Set up Google Cloud Vertex AI

  Enter an absolute file path on the
host running Codex, or press Enter to
use Application Default Credentials
(ADC).

  Credential file (optional): -adc.json

  To set up ADC with the Google Cloud
CLI, run
  gcloud auth application-default login


  enter set up \u{b7} esc back

provider credential_file must be an
absolute path";
    let got = rows(&h.t.screen());
    let want: Vec<String> = want.lines().map(str::to_owned).collect();
    assert_eq!(got[..want.len()], want[..], "{}", h.t.screen());
    // Retry with the cleared field: a new number; the stale answer of the
    // first is dropped, the second's configures.
    h.press("ctrl+u");
    h.press("enter");
    h.settle();
    let fresh = login_request(&logins(&h)[1..], setup(None));
    assert_ne!(fresh, request);
    h.fake.inject(epoch, answer(request, Ok(Answer::Written)));
    h.settle();
    assert!(!h.t.screen().contains("configured"), "stale");
    h.fake.inject(epoch, answer(fresh, Ok(Answer::Written)));
    // The oracle's `{Bedrock,Gcp}Configured` completes the step: the screen
    // ends and the launch goes on, the gateway device code first (its
    // configured line is not kept).
    h.wait_until("the device code is asked", |h| {
        h.fake.commands().iter().any(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::GatewayDevice { .. },
                        ..
                    },
                    ..
                }
            )
        })
    });
}
