//! Screen tests of the startup page's sign-in step (S02) over the app and
//! the fake host: what its oracle shard (`tests/oracle/signin.rs`, entry
//! screens playing fixtures) cannot name: the logins asked of the host,
//! their answers by number, the held key's guard on a terminal reporting
//! repeats and releases (kitty keyboard).

use std::time::Duration;

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::auth::{AuthResult, Login, Offer, Outcome};
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
            Command::Startup {
                ask: ask @ (Ask::Login { .. } | Ask::CancelLogin { .. }),
                ..
            } => Some(ask),
            _ => None,
        })
        .collect()
}

fn ev(ev: Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(ev))
}

/// The app launched on a sign-in due, offering `offer`.
fn launch(offer: Offer) -> (Harness, ConnEpoch) {
    let mut h = crate::common::start();
    h.settle();
    let Some(epoch) = h.fake.epoch() else {
        panic!("the app attached no backend");
    };
    h.fake.inject(
        epoch,
        ev(Ev::Facts {
            request: 1,
            facts: Facts {
                sign_in: Some(offer),
                ..Facts::default()
            },
        }),
    );
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
    assert_eq!(
        logins(&h),
        [Ask::Login {
            request: 2,
            login: Login::ChatGpt,
        }]
    );
    let browser = |request| {
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Browser {
                login_id: "login-1".into(),
                url: "https://auth.example/login".into(),
            },
        }))
    };
    h.fake.inject(epoch, browser(9));
    h.settle();
    assert!(
        !h.t.screen().contains("Finish signing in"),
        "a stale result opens nothing:\n{}",
        h.t.screen()
    );
    // ... and the login it started is cancelled (review O1).
    assert_eq!(
        logins(&h)[1..],
        [Ask::CancelLogin {
            login_id: "login-1".into(),
        }]
    );
    h.fake.inject(epoch, browser(2));
    h.wait_for("the browser link", |s| {
        s.contains("Finish signing in via your browser") && s.contains("https://auth.example/login")
    });
    h.press("esc");
    h.wait_for("the menu again", |s| s.contains("Sign in with ChatGPT"));
    h.settle();
    assert_eq!(
        logins(&h)[2..],
        [Ask::CancelLogin {
            login_id: "login-1".into(),
        }]
    );
    h.press("enter");
    h.settle();
    h.fake.inject(epoch, browser(3));
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
    assert_eq!(
        logins(&h),
        [Ask::Login {
            request: 2,
            login: Login::ApiKey("sk-env".into()),
        }]
    );
    h.fake.inject(
        epoch,
        ev(Ev::Auth(AuthResult {
            request: 2,
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
    h.fake.inject(
        epoch,
        ev(Ev::Facts {
            request: 1,
            facts: Facts {
                welcome: Some(Welcome { animations: true }),
                sign_in: Some(offer()),
                ..Facts::default()
            },
        }),
    );
    h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
    assert_eq!(welcome_row(&h.t.screen()), Some(22), "the logo above it");
    h.press("enter");
    h.settle();
    h.fake.inject(
        epoch,
        ev(Ev::Auth(AuthResult {
            request: 2,
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
