//! Screen tests of the sign-in step's provider wizards (S03) over the app
//! and the fake host: the Bedrock wizard asks the host to discover the
//! credentials and to set one up; its oracle shard (`tests/oracle/signin.rs`)
//! cannot answer a request, so the states after an answer are compared here
//! with the oracle's inline snapshots (`onboarding/bedrock_tests.rs`, drawn
//! at 72x24), the screen drawn from its top row.

use std::time::Duration;

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::{
    Credential, CredentialKind, Discovery, EnvCredential, Params, Profile,
};
use codex_tui_core::screens::onboarding::auth::provider::{Answer, Setup};
use codex_tui_core::screens::onboarding::auth::{AuthResult, Login, Offer, Outcome, Providers};
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts};

use crate::common::Harness;

fn ev(ev: Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Onboarding(ev))
}

/// The logins asked of the host, in order.
fn logins(h: &Harness) -> Vec<(u64, Login)> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Startup {
                ask: Ask::Login { request, login },
                ..
            } => Some((request, login)),
            _ => None,
        })
        .collect()
}

/// The app launched on a sign-in due offering Amazon Bedrock, at the
/// oracle's 72x24.
fn launch() -> (Harness, ConnEpoch) {
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
                sign_in: Some(Offer {
                    chatgpt: true,
                    api_key: true,
                    providers: Providers {
                        bedrock: true,
                        ..Providers::default()
                    },
                    ..Offer::default()
                }),
                ..Facts::default()
            },
        }),
    );
    h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
    h.t.resize(72, 24);
    h.settle();
    (h, epoch)
}

/// "Sign in another way", "Use Amazon Bedrock": the wizard opens and asks
/// the host to discover; its number.
fn open(h: &mut Harness) -> u64 {
    step(h, "3");
    step(h, "2");
    h.wait_for("the discovery", |s| s.contains("Checking for existing"));
    match logins(h).last() {
        Some((request, Login::Provider(Setup::BedrockDiscover))) => *request,
        other => panic!("no discovery asked: {other:?}"),
    }
}

/// A key on a view just shown: the step holds keys back for a second after
/// a change (the held key's guard), so the clock moves on first.
fn step(h: &mut Harness, keys: &str) {
    h.t.pump_for(Duration::from_millis(1100));
    h.press(keys);
    h.settle();
}

fn answer(h: &mut Harness, epoch: ConnEpoch, request: u64, outcome: Result<Answer, String>) {
    h.fake.inject(
        epoch,
        ev(Ev::Auth(AuthResult {
            request,
            outcome: Outcome::Provider(outcome),
        })),
    );
    h.settle();
}

/// The wizard opened on what the server found.
fn discovered(h: &mut Harness, epoch: ConnEpoch, discovery: Discovery) {
    let request = open(h);
    answer(h, epoch, request, Ok(Answer::Discovered(discovery)));
}

fn profile(name: &str, region: Option<&str>) -> Profile {
    Profile {
        name: name.into(),
        region: region.map(str::to_owned),
    }
}

fn environment(kind: CredentialKind, region: Option<&str>) -> EnvCredential {
    EnvCredential {
        kind,
        region: region.map(str::to_owned),
    }
}

/// The screen's rows, trailing blanks and blank rows cut (`render_visible`).
fn visible(h: &Harness) -> String {
    let screen = h.t.screen();
    let mut rows: Vec<&str> = screen.lines().map(str::trim_end).collect();
    while rows.last().is_some_and(|row| row.is_empty()) {
        rows.pop();
    }
    rows.join("\n")
}

#[track_caller]
fn assert_visible(h: &Harness, want: &str) {
    assert_eq!(visible(h), want, "\n{}", h.t.screen());
}

/// Back to the sign-in menu from the wizard's first list.
fn leave(h: &mut Harness) {
    step(h, "esc");
    h.wait_for("the sign-in menu", |s| s.contains("Use an OpenAI API key"));
    step(h, "esc");
    h.wait_for("the first menu", |s| s.contains("Sign in with ChatGPT"));
}

/// Oracle `discovery_prioritizes_profiles_and_groups_other_sign_in_methods`.
#[test]
fn discovery_prioritizes_profiles_and_groups_other_sign_in_methods() {
    let (mut h, epoch) = launch();
    open(&mut h);
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Checking for existing AWS credentials...",
    );
    // A profile and a Bedrock API key in the environment.
    let request = logins(&h).last().map_or(0, |(request, _)| *request);
    let found = Discovery {
        profiles: vec![profile("engineering", Some("us-east-2"))],
        environment: vec![environment(
            CredentialKind::BedrockApiKey,
            Some("us-east-2"),
        )],
    };
    answer(&mut h, epoch, request, Ok(Answer::Discovered(found)));
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  AWS profile detected: engineering
  Region: us-east-2

> 1. Continue with engineering
     Use your existing AWS credentials

  2. Continue with detected credentials
     Use the Bedrock API key from the server environment

  3. Other AWS sign-in methods
     Use another profile, access keys, environment variables, or a
Bedrock API key


  Press enter to continue
  Press esc to go back",
    );
    step(&mut h, "3");
    let every_method = "\
> 1. AWS profile
     Use AWS SSO or a named profile

  2. AWS access keys
     Enter an access key ID and secret access key

  3. Environment variables
     Use AWS credentials exported in your shell

  4. Bedrock API key
     Enter a Bedrock API key


  Press enter to continue
  Press esc to go back";
    assert_visible(
        &h,
        &format!(
            "\
> Set up Amazon Bedrock

  Choose how you authenticate with AWS.

{every_method}"
        ),
    );
    // Esc: the detected list; its profile has a region: set up at once.
    step(&mut h, "esc");
    step(&mut h, "1");
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Setting up Amazon Bedrock...

  Press esc to go back",
    );
    let Some((_, Login::Provider(setup))) = logins(&h).last().cloned() else {
        panic!("no setup asked");
    };
    assert_eq!(
        setup,
        Setup::Bedrock(Params {
            credential: Credential::Profile("engineering".into()),
            region: "us-east-2".into(),
        })
    );
    // Esc while setting up: back to the detected list.
    step(&mut h, "esc");
    h.wait_for("the detected list", |s| {
        s.contains("Continue with engineering")
    });
    leave(&mut h);

    discovered(
        &mut h,
        epoch,
        Discovery {
            profiles: vec![profile("engineering", Some("us-east-2"))],
            environment: Vec::new(),
        },
    );
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  AWS profile detected: engineering
  Region: us-east-2

> 1. Continue with engineering
     Use your existing AWS credentials

  2. Other AWS sign-in methods
     Use another profile, access keys, environment variables, or a
Bedrock API key


  Press enter to continue
  Press esc to go back",
    );
    leave(&mut h);

    discovered(
        &mut h,
        epoch,
        Discovery {
            profiles: vec![
                profile("engineering", Some("us-east-2")),
                profile("production", Some("us-west-2")),
            ],
            environment: Vec::new(),
        },
    );
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Choose an AWS profile.

> 1. engineering
     us-east-2

  2. production
     us-west-2

  3. Other AWS sign-in methods
     Use another profile, access keys, environment variables, or a
Bedrock API key


  Press enter to continue
  Press esc to go back",
    );
    leave(&mut h);

    discovered(
        &mut h,
        epoch,
        Discovery {
            profiles: Vec::new(),
            environment: vec![environment(CredentialKind::AccessKeys, Some("us-west-2"))],
        },
    );
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  AWS credentials detected in the server environment.

> 1. Continue with detected credentials

  2. Other AWS sign-in methods


  Press enter to continue
  Press esc to go back",
    );
    leave(&mut h);

    discovered(&mut h, epoch, Discovery::default());
    assert_visible(
        &h,
        &format!(
            "\
> Set up Amazon Bedrock

  No AWS credentials found.
  Choose how you authenticate with AWS.

{every_method}"
        ),
    );
    leave(&mut h);

    // Twelve profiles in ten rows: the marked last row, the footer kept.
    discovered(
        &mut h,
        epoch,
        Discovery {
            profiles: (0..12)
                .map(|index| profile(&format!("engineering-{index}"), Some("us-east-2")))
                .collect(),
            environment: Vec::new(),
        },
    );
    h.t.resize(72, 10);
    step(&mut h, "up");
    let screen = h.t.screen();
    for text in [
        "Other AWS sign-in methods",
        "Press enter to continue",
        "Press esc to go back",
    ] {
        assert!(screen.contains(text), "{text}:\n{screen}");
    }
    leave(&mut h);

    // A failed discovery at 24x12: every method, the footer and the
    // message wrapped.
    h.t.resize(24, 12);
    let request = open(&mut h);
    answer(
        &mut h,
        epoch,
        request,
        Err("AWS credentials unavailable in this terminal".into()),
    );
    let screen = h.t.screen();
    for text in [
        "Press enter to",
        "continue",
        "Press esc to go back",
        // The oracle's message wraps under its prefix
        // (`Unable to check AWS credentials: `, `start_bedrock_discovery`).
        "Unable to check AWS",
        "credentials unavailable",
    ] {
        assert!(screen.contains(text), "{text}:\n{screen}");
    }
}

/// Oracle `credential_entries_share_region_and_keep_aws_secrets_hidden`,
/// the part rtwk draws as the oracle does: a profile or the environment
/// without a region asks for it; the access keys move with Down and Up and
/// keep the secrets hidden; the API key, the profile, the region entries
/// and the environment's instructions. The focused secret's last character
/// shown is G55 (`tests/oracle/signin.rs`).
#[test]
fn credential_entries_share_region_and_keep_aws_secrets_hidden() {
    let (mut h, epoch) = launch();
    for (found, region) in [
        (
            Discovery {
                profiles: vec![profile("engineering", None)],
                environment: Vec::new(),
            },
            "us-west-1",
        ),
        (
            Discovery {
                profiles: Vec::new(),
                environment: vec![environment(CredentialKind::AccessKeys, None)],
            },
            "us-west-1",
        ),
    ] {
        discovered(&mut h, epoch, found);
        step(&mut h, "1");
        h.wait_for("the region entry", |s| s.contains("AWS Region:"));
        h.t.pump_for(Duration::from_millis(1100));
        h.t.input(region.as_bytes());
        h.settle();
        assert_visible(
            &h,
            "\
> Set up Amazon Bedrock

  Enter the AWS Region to use with Amazon Bedrock.

  AWS Region: us-west-1

  Press enter to continue
  Press esc to go back",
        );
        step(&mut h, "esc");
        leave(&mut h);
    }

    discovered(&mut h, epoch, Discovery::default());
    step(&mut h, "2");
    h.wait_for("the access keys", |s| s.contains("AWS access key ID"));
    h.t.pump_for(Duration::from_millis(1100));
    h.t.input(b"AKIAEXAMPLE");
    step(&mut h, "down");
    h.t.input(b"secret-value");
    step(&mut h, "down");
    h.t.input(b"session-value");
    step(&mut h, "up");
    step(&mut h, "up");
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Enter your AWS access keys.

> AWS access key ID: AKIAEXAMPLE
  AWS secret access key: \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}
  AWS session token (optional): \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}

  Press enter to continue
  Press esc to go back",
    );
    let screen = h.t.screen();
    assert!(!screen.contains("secret-value") && !screen.contains("session-value"));

    // Enter on the last field with the ID cleared returns to it.
    step(&mut h, "ctrl+u");
    step(&mut h, "down");
    step(&mut h, "down");
    step(&mut h, "enter");
    assert!(
        h.t.screen().contains("> AWS access key ID:"),
        "{}",
        h.t.screen()
    );
    step(&mut h, "esc");
    step(&mut h, "4");
    h.wait_for("the API key entry", |s| s.contains("Bedrock API key:"));
    h.t.pump_for(Duration::from_millis(1100));
    h.t.input(b"bedrock-api-key");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("bedrock-api-key"), "{screen}");
    step(&mut h, "esc");
    step(&mut h, "1");
    h.wait_for("the profile entry", |s| s.contains("AWS profile:"));
    h.t.pump_for(Duration::from_millis(1100));
    h.t.input(b"engineering");
    h.settle();
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Enter the name of your AWS profile.

  AWS profile: engineering

  Press enter to continue
  Press esc to go back",
    );
    // The profile is not one found: its region is asked, then the setup.
    step(&mut h, "enter");
    h.wait_for("the region entry", |s| s.contains("AWS Region:"));
    h.t.pump_for(Duration::from_millis(1100));
    h.t.input(b"us-west-1");
    step(&mut h, "enter");
    let Some((request, Login::Provider(setup))) = logins(&h).last().cloned() else {
        panic!("no setup asked");
    };
    assert_eq!(
        setup,
        Setup::Bedrock(Params {
            credential: Credential::Profile("engineering".into()),
            region: "us-west-1".into(),
        })
    );
    // Refused: the region entry again, prefilled, with the message.
    answer(&mut h, epoch, request, Err("no such profile".into()));
    let screen = h.t.screen();
    assert!(
        screen.contains("AWS Region: us-west-1") && screen.contains("no such profile"),
        "{screen}"
    );
    // Accepted: Amazon Bedrock configured.
    step(&mut h, "enter");
    let Some((request, _)) = logins(&h).last().cloned() else {
        panic!("no setup asked");
    };
    answer(&mut h, epoch, request, Ok(Answer::Written));
    h.wait_for("the configured line", |s| {
        s.contains("\u{2713} Amazon Bedrock configured")
    });

    let (mut h, epoch) = launch();
    discovered(&mut h, epoch, Discovery::default());
    step(&mut h, "3");
    assert_visible(
        &h,
        "\
> Set up Amazon Bedrock

  Export AWS credentials, then run `codex --no-daemon` in that shell.

  Setup guide: https://learn.chatgpt.com/docs/amazon-bedrock

> 1. Choose another sign-in method


  Press enter to continue
  Press esc to go back",
    );
}
