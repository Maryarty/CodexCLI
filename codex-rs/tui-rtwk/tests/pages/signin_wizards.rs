//! Screen tests of the sign-in step's provider wizards (S03) over the app
//! and the fake host: the Bedrock wizard asks the host to discover the
//! credentials and to set one up. The states after the fake host answers are
//! compared here with the oracle's inline snapshots (`onboarding/bedrock_tests.rs`),
//! drawn at 72x24 from the screen's top row.

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
    launch_offering(Providers {
        bedrock: true,
        ..Providers::default()
    })
}

/// The same, offering these wizards. On a driven harness: every step here
/// settles, and a settle on the real clock is 100 ms of sleeping (the 31
/// tests were 45 s; driven, 1.6 s).
fn launch_offering(providers: Providers) -> (Harness, ConnEpoch) {
    let mut h = crate::common::start_driven(false, crate::common::WIDTH, crate::common::HEIGHT);
    h.settle();
    let Some(epoch) = h.fake.epoch() else {
        panic!("the app attached no backend");
    };
    h.answer_startup_facts(Facts {
        sign_in: Some(Offer {
            chatgpt: true,
            api_key: true,
            providers,
            ..Offer::default()
        }),
        ..Facts::default()
    });
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

#[test]
fn pasted_bedrock_credentials_stay_masked_on_focus_return_and_reach_setup_intact() {
    // Synthetic credentials with one extended grapheme at the end of each value.
    let secret = "fixture-e\u{301}";
    let token = "fixture-👩‍💻";
    let (mut h, epoch) = launch();
    discovered(&mut h, epoch, Discovery::default());
    h.press("down");
    h.wait_for("the access-key method", |screen| {
        screen.contains("> 2. AWS access keys")
    });
    h.press("enter");
    h.wait_for("the access-key form", |screen| {
        screen.contains("> AWS access key ID:")
    });

    let assert_rows = |h: &Harness, rows: &[&str]| {
        let screen = h.t.screen();
        for row in rows {
            assert!(
                screen.lines().any(|line| line.trim() == *row),
                "expected masked row: {row}"
            );
        }
        assert!(
            !screen.contains(secret) && !screen.contains(token),
            "credential became visible"
        );
    };
    h.t.paste("AKIAEXAMPLE");
    h.wait_for("the pasted ID", |screen| {
        screen.contains("AWS access key ID: AKIAEXAMPLE")
    });
    h.press("down");
    h.wait_for("the focused secret", |screen| {
        screen.contains("> AWS secret access key:")
    });
    h.t.paste(secret);
    h.t.pump();
    assert_rows(&h, &["> AWS secret access key: •••••••••"]);

    h.press("down");
    h.wait_for("the focused token", |screen| {
        screen.contains("> AWS session token (optional):")
    });
    h.t.paste(token);
    h.t.pump();
    let token_selected = [
        "AWS secret access key: •••••••••",
        "> AWS session token (optional): •••••••••",
    ];
    assert_rows(&h, &token_selected);

    for (key, rows) in [
        (
            "up",
            [
                "> AWS secret access key: •••••••••",
                "AWS session token (optional): •••••••••",
            ],
        ),
        ("down", token_selected),
        (
            "up up",
            [
                "AWS secret access key: •••••••••",
                "AWS session token (optional): •••••••••",
            ],
        ),
    ] {
        for motion in key.split_whitespace() {
            h.press(motion);
        }
        assert_rows(&h, &rows);
    }

    h.press("down");
    h.press("down");
    h.press("enter");
    h.wait_for("the access-key region", |screen| {
        screen.contains("AWS Region:")
    });
    h.t.paste("us-west-2");
    h.wait_for("the pasted region", |screen| {
        screen.contains("AWS Region: us-west-2")
    });
    h.press("enter");
    h.wait_until("the Bedrock setup request", |h| {
        logins(h)
            .iter()
            .any(|(_, login)| matches!(login, Login::Provider(Setup::Bedrock(_))))
    });
    let asks = logins(&h);
    let Some((
        _,
        Login::Provider(Setup::Bedrock(Params {
            credential:
                Credential::AccessKeys {
                    access_key_id,
                    secret_access_key,
                    session_token,
                },
            region,
        })),
    )) = asks.last()
    else {
        panic!("the wizard did not ask to configure the access keys");
    };
    assert_eq!(region, "us-west-2");
    assert_eq!(access_key_id, "AKIAEXAMPLE");
    assert!(
        secret_access_key == secret && session_token.as_deref() == Some(token),
        "focus navigation changed the submitted credentials"
    );
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
/// and the environment's instructions. The focused secret shows its last
/// character it was typed with (`obscure_reveal_last`); the one a field
/// shows on arrival with text already in it is G55's rest
/// (`tests/oracle/signin.rs`).
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
    h.settle();
    assert!(
        h.t.screen()
            .contains("AWS secret access key: \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}e"),
        "{}",
        h.t.screen()
    );
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
    // The oracle's `{Bedrock,Gcp}Configured` completes the step: the screen
    // ends and the launch goes on, the gateway device code first (its
    // configured line is not kept).
    h.wait_until("the device code is asked", |h| {
        h.fake.commands().iter().any(|command| {
            matches!(
                command,
                Command::Startup {
                    ask: Ask::GatewayDevice { .. },
                    ..
                }
            )
        })
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

/// The GovCloud guidance (`onboarding/bedrock_tests.rs`): the widget alone
/// over an app that counts its acknowledgments, as the launch step that
/// shows it is the startup page's. The oracle's snapshots are written out
/// (`bedrock__tests__gov_cloud_warning`, `..._short`).
mod gov_cloud {
    use codex_tui_rtwk::app::run::localized;
    use codex_tui_rtwk::l10n::Lang;
    use codex_tui_rtwk::pages::startup::onboarding::GovCloudGuidance;
    use rtwk::AnyWidget;
    use rtwk::app::{AppView, ViewCx};
    use rtwk_core::input::{Mods, MouseKind};
    use rtwk_test::{Options, Tester};

    const URL: &str = "https://learn.chatgpt.com/docs/enterprise/govcloud-configuration";

    #[derive(Default)]
    struct Acknowledged(u32);

    fn view(_: &Acknowledged, cx: &mut ViewCx<Acknowledged>) -> AnyWidget {
        localized(
            GovCloudGuidance {
                acknowledge: cx.on(|app, _| app.0 += 1),
            },
            Lang::En,
        )
        .into()
    }

    fn open(width: u16, height: u16) -> Tester {
        // The kitty keyboard protocol reports repeats.
        let mut t = Tester::with(Options::new().size(width, height).kitty_keyboard(true));
        t.pump_app(AppView::new(Acknowledged::default(), view));
        t.pump();
        t
    }

    /// A key, and the frame it leads to.
    fn key(t: &mut Tester, keys: &str) {
        t.press(keys);
        t.pump_for(std::time::Duration::from_millis(50));
    }

    fn count(t: &Tester) -> u32 {
        t.read_app(|app: &Acknowledged| app.0)
    }

    /// The screen's rows, trailing blanks and blank rows at the bottom cut.
    fn visible(t: &Tester) -> String {
        let mut rows: Vec<String> = t
            .lines()
            .iter()
            .map(|row| row.trim_end().to_owned())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows.join("\n")
    }

    /// The link targets on the screen.
    fn links(t: &Tester) -> Vec<String> {
        let (width, height) = (
            t.lines().first().map_or(0, |row| row.chars().count()),
            t.lines().len(),
        );
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter_map(|(x, y)| t.cell(x, y).link)
            .collect()
    }

    /// Oracle `gov_cloud_guidance_requires_acknowledgement`.
    #[test]
    fn gov_cloud_guidance_requires_acknowledgement() {
        let mut t = open(72, 24);
        assert_eq!(
            visible(&t),
            "\
Using Codex with AWS GovCloud

You are using Codex with AWS GovCloud. Please ensure you or your
administrator have read the application configuration and security
guidance before proceeding.

> Acknowledge

  Press enter to continue"
        );
        // A held Enter (a repeat) does not acknowledge; a press does.
        t.input(b"\x1b[13;1:2u");
        t.pump();
        assert_eq!(count(&t), 0);
        t.press("enter");
        assert_eq!(count(&t), 1);
        // The link to the guidance is on the screen.
        assert!(links(&t).iter().any(|link| link == URL));
    }

    /// A tap on the row acknowledges as Enter does (ADR-011).
    #[test]
    fn a_tap_on_the_acknowledge_row_acknowledges() {
        let mut t = open(72, 24);
        t.click(3, 6);
        t.pump();
        assert_eq!(count(&t), 1);
        // Elsewhere nothing happens.
        t.click(3, 1);
        t.pump();
        assert_eq!(count(&t), 1);
    }

    /// Oracle `gov_cloud_guidance_scrolls_in_short_panes`.
    #[test]
    fn gov_cloud_guidance_scrolls_in_short_panes() {
        let mut t = open(24, 6);
        let initial = visible(&t);
        assert_eq!(
            initial,
            "\
Using Codex with AWS
GovCloud

You are using Codex with
> Acknowledge
\u{2191}/\u{2193} scroll \u{b7} enter"
        );
        // Even a one-row viewport exposes every line and the link.
        for height in [6, 2] {
            // (A resize to the same size clears the tester's emulator.)
            if height != 6 {
                t.resize(24, height);
                t.pump();
            }
            let mut seen = String::new();
            let mut linked = false;
            for _ in 0..20 {
                seen.push_str(&t.screen());
                linked |= links(&t).iter().any(|link| link == URL);
                assert!(visible(&t).contains("Acknowledge"), "\n{}", t.screen());
                key(&mut t, "down");
            }
            assert!(seen.contains("Using Codex with AWS"));
            assert!(seen.contains("proceeding."));
            assert!(linked);
            for _ in 0..20 {
                key(&mut t, "up");
            }
            assert!(visible(&t).starts_with("Using Codex with AWS"));
        }
        t.resize(24, 6);
        t.pump();
        assert_eq!(visible(&t), initial);
        // Down on a pane that fits scrolls nothing.
        t.resize(72, 24);
        t.pump();
        let full = visible(&t);
        key(&mut t, "down");
        assert_eq!(visible(&t), full);
        assert_eq!(count(&t), 0);
    }

    /// The wheel scrolls the text of a short pane (ADR-011).
    #[test]
    fn the_wheel_scrolls_a_short_pane() {
        let mut t = open(24, 6);
        t.mouse(MouseKind::WheelDown, 3, 1, 0, Mods::NONE);
        t.pump();
        assert!(!visible(&t).starts_with("Using Codex with AWS"));
        t.mouse(MouseKind::WheelUp, 3, 1, 0, Mods::NONE);
        t.pump();
        assert!(visible(&t).starts_with("Using Codex with AWS"));
    }
}

/// The gateway wizard (`onboarding/gateway_tests.rs`): the picker, the form
/// and the states after an answer of the server, drawn as the oracle's
/// snapshots are (72 wide, the screen from its top row).
mod gateway {
    use super::*;
    use codex_tui_core::screens::onboarding::auth::provider::gateway::{Page, Patch, Saved};

    const POLICY: &str =
        "The default provider is set by configuration policy or a launch override.";

    fn launch() -> (Harness, ConnEpoch) {
        let (mut h, epoch) = super::launch_offering(Providers {
            bedrock: true,
            gcp: true,
            gateway: true,
        });
        h.t.resize(72, 8);
        h.settle();
        (h, epoch)
    }

    /// "Sign in another way" (unless the menu of the other ways is shown,
    /// where Esc from the wizard comes back to), "Use a custom provider or
    /// gateway": the wizard asks the host for the saved gateways; the
    /// request's number.
    fn open(h: &mut Harness) -> u64 {
        if h.t.screen().contains("Sign in with ChatGPT") {
            step(h, "3");
        }
        step(h, "4");
        h.wait_for("the loading line", |s| s.contains("Loading gateways"));
        asked_request(h)
    }

    /// The last ask of the host, a gateway one.
    fn asked_request(h: &Harness) -> u64 {
        match logins(h).last() {
            Some((request, Login::Provider(_))) => *request,
            other => panic!("no gateway request asked: {other:?}"),
        }
    }

    fn setup(h: &Harness) -> Setup {
        match logins(h).last() {
            Some((_, Login::Provider(setup))) => setup.clone(),
            other => panic!("no gateway request asked: {other:?}"),
        }
    }

    fn saved(id: &str, name: &str, variable: Option<&str>) -> Saved {
        Saved {
            id: id.into(),
            name: name.into(),
            variable: variable.map(str::to_owned),
        }
    }

    fn page(gateways: Vec<Saved>) -> Page {
        Page {
            gateways,
            default_id: "openai".into(),
        }
    }

    /// The picker over these saved gateways.
    fn picker(h: &mut Harness, epoch: ConnEpoch, gateways: Vec<Saved>) {
        let request = open(h);
        answer(h, epoch, request, Ok(Answer::Gateways(page(gateways))));
    }

    #[test]
    fn select_a_saved_gateway_during_sign_in() {
        let (mut h, epoch) = launch();
        let request = open(&mut h);
        assert_eq!(setup(&h), Setup::GatewayList);
        answer(&mut h, epoch, request, Ok(Answer::Gateways(page(vec![]))));
        assert_visible(
            &h,
            "  Use a custom provider or gateway


› Create a custom provider or gateway



  Press enter to create or esc to go back",
        );
        step(&mut h, "esc");

        // A refusal of the server: the message wrapped, "Back".
        let request = open(&mut h);
        answer(&mut h, epoch, request, Err(POLICY.into()));
        assert_visible(
            &h,
            "  Use a custom provider or gateway
  The default provider is set by configuration policy or a launch
  override.


› 1. Back",
        );
        step(&mut h, "esc");

        // A saved gateway: Enter makes it the default.
        picker(
            &mut h,
            epoch,
            vec![saved("office", "Office", Some("office"))],
        );
        assert_visible(
            &h,
            "  Use a custom provider or gateway


› ○ Office                             office
  Create a custom provider or gateway


  Press enter to use as default or esc to go back",
        );
        step(&mut h, "enter");
        assert_eq!(setup(&h), Setup::GatewaySelect("office".into()));
        let request = asked_request(&h);
        answer(&mut h, epoch, request, Ok(Answer::GatewaySelected));
        h.wait_for("the configured line", |s| {
            s.contains("\u{2713} Custom provider configured")
        });
        assert_visible(&h, "✓ Custom provider configured");
        // The wizard's answer ends the steps: the gateway's server is asked
        // for (the answer of the wizard, not a state set by the test).
        assert!(
            h.fake.commands().iter().any(|command| matches!(
                command,
                Command::Startup {
                    ask: Ask::SelectGateway { provider_id, .. },
                    ..
                } if provider_id == "office"
            )),
            "{:?}",
            h.fake.commands()
        );
    }

    /// The screen drawn at `height` rows (the oracle draws at the height it
    /// names), from its top row.
    fn at(h: &mut Harness, height: u16) -> String {
        h.t.resize(72, height);
        h.settle();
        visible(h)
    }

    const REPEAT_ENTER: &[u8] = b"\x1b[13;1:2u";
    const RELEASE_ENTER: &[u8] = b"\x1b[13;1:3u";
    const SHIFT_ENTER: &[u8] = b"\x1b[13;2u";

    fn input(h: &mut Harness, bytes: &[u8]) {
        h.t.input(bytes);
        h.t.pump_for(Duration::from_millis(20));
    }

    /// A key in a field or on Save: the mark is the model's, so the host
    /// runs before the screen is read.
    fn key(h: &mut Harness, keys: &str) {
        h.press(keys);
        h.settle();
    }

    fn paste(h: &mut Harness, text: &str) {
        h.t.paste(text);
        h.settle();
    }

    /// The picker over `Office`, the create row opened: the form.
    fn form(h: &mut Harness, epoch: ConnEpoch) {
        picker(
            h,
            epoch,
            vec![saved("office", "Office", Some("OFFICE_KEY"))],
        );
        step(h, "down");
        step(h, "enter");
    }

    #[test]
    fn create_highlights_the_saved_gateway_until_explicitly_selected() {
        let (mut h, epoch) = launch();
        form(&mut h, epoch);
        assert_eq!(
            at(&mut h, 12),
            "Create a custom provider or gateway

\u{203a} Name*  Team gateway
  Base URL*  https://gateway.example/v1
  API key variable  $TEAM_API_KEY
  Save

* Required
Enter the environment variable name on the app server.
tab move \u{b7} enter next/save \u{b7} esc back"
        );
        paste(&mut h, "Team gateway");
        key(&mut h, "down");
        paste(&mut h, "https://team.example.test/v1?");
        key(&mut h, "down");
        paste(&mut h, "TEAM_KEY-");
        key(&mut h, "down");
        // Save marked, the pane four rows: the feedback follows it.
        assert_eq!(
            at(&mut h, 4),
            "\u{203a} Save

* Required
Enter the environment variable name on the app server."
        );
        // A variable that is not one: the form says so and sends nothing.
        let asked = logins(&h).len();
        step(&mut h, "enter");
        assert_eq!(logins(&h).len(), asked);
        assert_eq!(
            visible(&h),
            "\u{203a} Save
Enter an environment variable name such as TEAM_API_KEY.

* Required"
        );
        key(&mut h, "up");
        key(&mut h, "backspace");
        key(&mut h, "down");
        step(&mut h, "enter");
        assert_eq!(
            setup(&h),
            Setup::GatewayWrite(Patch {
                name: "Team gateway".into(),
                url: "https://team.example.test/v1?".into(),
                variable: Some("TEAM_KEY".into()),
            })
        );
        // The server's refusal: the form again, the fields kept, the
        // message under Save.
        let request = asked_request(&h);
        answer(
            &mut h,
            epoch,
            request,
            Err("Enter an HTTP(S) base URL without credentials, a query, or a fragment.".into()),
        );
        assert_eq!(
            at(&mut h, 8),
            "  Base URL*  https://team.example.test/v1?
  API key variable  $TEAM_KEY
\u{203a} Save
Enter an HTTP(S) base URL without credentials, a query, or a fragment.

* Required
Enter the environment variable name on the app server.
tab move \u{b7} enter next/save \u{b7} esc back"
        );

        // Correct the URL, keep the other fields.
        key(&mut h, "up");
        key(&mut h, "up");
        key(&mut h, "backspace");
        key(&mut h, "down");
        key(&mut h, "down");
        step(&mut h, "enter");
        assert_eq!(
            setup(&h),
            Setup::GatewayWrite(Patch {
                name: "Team gateway".into(),
                url: "https://team.example.test/v1".into(),
                variable: Some("TEAM_KEY".into()),
            })
        );
        let request = asked_request(&h);
        answer(
            &mut h,
            epoch,
            request,
            Ok(Answer::GatewaySaved(saved(
                "team-gateway",
                "Team gateway",
                Some("TEAM_KEY"),
            ))),
        );
        // The created gateway is marked, not the default: Enter on a held
        // key selects nothing.
        let asked = logins(&h).len();
        input(&mut h, REPEAT_ENTER);
        assert_eq!(logins(&h).len(), asked);
        assert_eq!(
            visible(&h),
            "  Use a custom provider or gateway


  \u{25cb} Office                             office
\u{203a} \u{25cb} Team gateway                       team-gateway
  Create a custom provider or gateway

  Press enter to use as default or esc to go back"
        );
        input(&mut h, RELEASE_ENTER);
        step(&mut h, "enter");
        assert_eq!(setup(&h), Setup::GatewaySelect("team-gateway".into()));
        let request = asked_request(&h);
        answer(&mut h, epoch, request, Ok(Answer::GatewaySelected));
        h.wait_for("the configured line", |s| {
            s.contains("\u{2713} Custom provider configured")
        });
    }

    /// The provider forms take plain keys only (`key_hint::plain`): Shift+Enter
    /// on Save sends nothing, plain Enter does.
    #[test]
    fn shift_enter_on_save_does_not_save_but_enter_does() {
        let (mut h, epoch) = launch();
        form(&mut h, epoch);
        paste(&mut h, "Team gateway");
        key(&mut h, "down");
        paste(&mut h, "https://team.example.test/v1");
        key(&mut h, "down");
        paste(&mut h, "TEAM_KEY");
        key(&mut h, "down");
        let asked = logins(&h).len();
        h.t.pump_for(Duration::from_millis(1100));
        input(&mut h, SHIFT_ENTER);
        h.settle();
        assert_eq!(logins(&h).len(), asked, "Shift+Enter saved");
        step(&mut h, "enter");
        assert_eq!(logins(&h).len(), asked + 1, "Enter did not save");
    }

    /// The row of the screen the text is on (the first), by its start.
    fn row_of(h: &Harness, start: &str) -> u16 {
        let screen = h.t.screen();
        let at = screen
            .lines()
            .position(|row| row.trim_start_matches(['\u{203a}', ' ']).starts_with(start));
        match at {
            Some(at) => u16::try_from(at).unwrap_or(0),
            None => panic!("no row starts with {start:?}:\n{screen}"),
        }
    }

    /// Esc goes one stage back: the form to the picker with the create row
    /// marked, the picker to the menu; while the list loads it leaves too,
    /// and a late answer is dropped.
    #[test]
    fn esc_goes_one_stage_back() {
        let (mut h, epoch) = launch();
        form(&mut h, epoch);
        h.t.resize(72, 12);
        step(&mut h, "esc");
        assert_eq!(
            at(&mut h, 8),
            "  Use a custom provider or gateway


  \u{25cb} Office                             office
\u{203a} Create a custom provider or gateway


  Press enter to create or esc to go back"
        );
        step(&mut h, "esc");
        assert!(h.t.screen().contains("Use a custom provider or gateway"));
        assert!(h.t.screen().contains("Sign in another way"));
        let request = open(&mut h);
        step(&mut h, "esc");
        assert!(h.t.screen().contains("Sign in another way"));
        answer(&mut h, epoch, request, Ok(Answer::Gateways(page(vec![]))));
        assert!(
            h.t.screen().contains("Sign in another way"),
            "a late answer shows nothing:\n{}",
            h.t.screen()
        );
    }

    /// The load error leaves by its one row (its number, Enter) or Esc.
    #[test]
    fn the_load_error_leaves_by_back() {
        for keys in ["1", "enter", "esc"] {
            let (mut h, epoch) = launch();
            let request = open(&mut h);
            answer(&mut h, epoch, request, Err(POLICY.into()));
            assert!(h.t.screen().contains("1. Back"));
            step(&mut h, keys);
            assert!(
                h.t.screen().contains("Sign in another way"),
                "{keys}:\n{}",
                h.t.screen()
            );
        }
    }

    /// A save whose reply was lost: the list is read again and the picker
    /// says the save is unconfirmed (`UNCONFIRMED_CREATE`).
    #[test]
    fn an_unconfirmed_save_reads_the_list_again_and_says_so() {
        let (mut h, epoch) = launch();
        form(&mut h, epoch);
        paste(&mut h, "Team gateway");
        key(&mut h, "down");
        paste(&mut h, "https://team.example.test/v1");
        key(&mut h, "down");
        key(&mut h, "down");
        step(&mut h, "enter");
        let request = asked_request(&h);
        answer(&mut h, epoch, request, Ok(Answer::GatewayUnconfirmed));
        assert_eq!(setup(&h), Setup::GatewayList);
        let request = asked_request(&h);
        answer(
            &mut h,
            epoch,
            request,
            Ok(Answer::Gateways(page(vec![
                saved("office", "Office", None),
                saved("team-gateway", "Team gateway", None),
            ]))),
        );
        assert_eq!(
            at(&mut h, 12),
            "  Use a custom provider or gateway


\u{203a} \u{25cb} Office                             office
  \u{25cb} Team gateway                       team-gateway
  Create a custom provider or gateway



  Could not confirm the save. Check the provider list before adding it
  again.
  Press enter to use as default or esc to go back"
        );
    }

    /// The server refuses to make the gateway the default: the picker stays
    /// on the row with the message.
    #[test]
    fn a_refused_selection_keeps_the_picker_with_the_message() {
        let (mut h, epoch) = launch();
        picker(&mut h, epoch, vec![saved("office", "Office", None)]);
        step(&mut h, "enter");
        let request = asked_request(&h);
        answer(
            &mut h,
            epoch,
            request,
            Err("The default cannot change.".into()),
        );
        let screen = h.t.screen();
        assert!(
            screen.contains("\u{203a} \u{25cb} Office")
                && screen.contains("The default cannot change."),
            "{screen}"
        );
        assert!(!screen.contains("Custom provider configured"));
    }

    /// Save asks for what is missing before it asks the server.
    #[test]
    fn save_names_the_field_that_is_missing() {
        let (mut h, epoch) = launch();
        picker(&mut h, epoch, vec![]);
        step(&mut h, "enter");
        h.t.resize(72, 12);
        key(&mut h, "down");
        key(&mut h, "down");
        key(&mut h, "down");
        let asked = logins(&h).len();
        step(&mut h, "enter");
        assert!(
            h.t.screen().contains("Enter a provider name."),
            "{}",
            h.t.screen()
        );
        paste_into(&mut h, 0, "Team");
        step(&mut h, "enter");
        assert!(
            h.t.screen().contains("Enter a base URL."),
            "{}",
            h.t.screen()
        );
        assert_eq!(logins(&h).len(), asked, "nothing is asked of the server");
    }

    fn paste_into(h: &mut Harness, field: usize, text: &str) {
        // Up or Down to the field from where the mark is (Save).
        for _ in 0..3usize.saturating_sub(field) {
            key(h, "up");
        }
        paste(h, text);
        for _ in 0..3usize.saturating_sub(field) {
            key(h, "down");
        }
    }

    /// Ctrl+Space completes the variable from the saved gateways' variables
    /// (`ProviderForm::suggestion`); the keys line says so while one is
    /// offered.
    #[test]
    fn ctrl_space_completes_a_saved_variable() {
        let (mut h, epoch) = launch();
        form(&mut h, epoch);
        h.t.resize(72, 12);
        key(&mut h, "down");
        key(&mut h, "down");
        paste(&mut h, "OFF");
        assert!(
            h.t.screen().contains("API key variable  $OFF"),
            "{}",
            h.t.screen()
        );
        assert!(h.t.screen().contains("space complete"), "{}", h.t.screen());
        key(&mut h, "ctrl+space");
        assert!(
            h.t.screen().contains("API key variable  $OFFICE_KEY"),
            "{}",
            h.t.screen()
        );
        assert!(!h.t.screen().contains("space complete"), "{}", h.t.screen());
        // Text that no saved variable starts with offers nothing.
        paste(&mut h, "X");
        assert!(!h.t.screen().contains("space complete"), "{}", h.t.screen());
    }

    /// Tab and Shift+Tab move the mark over the fields and Save; Enter in
    /// a field goes on to the next.
    #[test]
    fn tab_moves_the_mark_and_enter_goes_on() {
        let (mut h, epoch) = launch();
        picker(&mut h, epoch, vec![]);
        step(&mut h, "enter");
        h.t.resize(72, 12);
        let marked = |h: &Harness| -> String {
            h.t.screen()
                .lines()
                .find(|row| row.starts_with('\u{203a}'))
                .unwrap_or_default()
                .trim_end()
                .to_owned()
        };
        assert!(marked(&h).starts_with("\u{203a} Name*"));
        key(&mut h, "tab");
        assert!(
            marked(&h).starts_with("\u{203a} Base URL*"),
            "{}",
            h.t.screen()
        );
        key(&mut h, "tab");
        assert!(marked(&h).starts_with("\u{203a} API key variable"));
        key(&mut h, "tab");
        assert_eq!(marked(&h), "\u{203a} Save");
        key(&mut h, "shift+tab");
        assert!(marked(&h).starts_with("\u{203a} API key variable"));
        key(&mut h, "shift+tab");
        key(&mut h, "shift+tab");
        assert!(marked(&h).starts_with("\u{203a} Name*"));
        // Enter in a field: the next field.
        step(&mut h, "enter");
        assert!(
            marked(&h).starts_with("\u{203a} Base URL*"),
            "{}",
            h.t.screen()
        );
    }

    /// A tap on a row of the picker moves the cursor there; a tap on the
    /// row the cursor is on does what Enter does (ADR-011).
    #[test]
    fn a_tap_on_a_picker_row_moves_the_cursor_then_accepts() {
        let (mut h, epoch) = launch();
        picker(&mut h, epoch, vec![saved("office", "Office", None)]);
        let create = row_of(&h, "Create a custom");
        h.t.click(6, create);
        h.settle();
        assert!(
            h.t.screen()
                .contains("\u{203a} Create a custom provider or gateway")
                && h.t.screen().contains("Press enter to create"),
            "{}",
            h.t.screen()
        );
        // The cursor was on the create row: the next tap opens the form.
        h.t.click(6, create);
        h.settle();
        assert!(h.t.screen().contains("* Required"), "{}", h.t.screen());
        step(&mut h, "esc");
        // A tap on the saved gateway (the cursor is on the create row): it
        // is marked, the second tap selects it.
        let office = row_of(&h, "\u{25cb} Office");
        h.t.click(6, office);
        h.settle();
        assert!(
            h.t.screen().contains("\u{203a} \u{25cb} Office"),
            "{}",
            h.t.screen()
        );
        assert_eq!(setup(&h), Setup::GatewayList);
        h.t.click(6, office);
        h.settle();
        assert_eq!(setup(&h), Setup::GatewaySelect("office".into()));
    }

    /// A tap on a field marks it and types there; a tap on Save marks it, a
    /// tap on the marked Save saves (ADR-011).
    #[test]
    fn a_tap_marks_a_field_and_save_saves_when_marked() {
        let (mut h, epoch) = launch();
        picker(&mut h, epoch, vec![]);
        step(&mut h, "enter");
        h.t.resize(72, 12);
        paste(&mut h, "Team");
        let url = row_of(&h, "Base URL*");
        h.t.click(24, url);
        h.settle();
        assert!(
            h.t.screen().contains("\u{203a} Base URL*"),
            "{}",
            h.t.screen()
        );
        paste(&mut h, "https://team.example.test/v1");
        let asked = logins(&h).len();
        let save = row_of(&h, "Save");
        h.t.click(4, save);
        h.settle();
        assert!(h.t.screen().contains("\u{203a} Save"), "{}", h.t.screen());
        assert_eq!(logins(&h).len(), asked, "the first tap marks");
        h.t.click(4, save);
        h.settle();
        assert_eq!(
            setup(&h),
            Setup::GatewayWrite(Patch {
                name: "Team".into(),
                url: "https://team.example.test/v1".into(),
                variable: None,
            })
        );
    }

    /// A tap on the load error's Back row goes back to the menu (ADR-011).
    #[test]
    fn a_tap_on_back_leaves_the_load_error() {
        let (mut h, epoch) = launch();
        let request = open(&mut h);
        answer(&mut h, epoch, request, Err(POLICY.into()));
        let back = row_of(&h, "1. Back");
        h.t.click(6, back);
        h.settle();
        assert!(
            h.t.screen().contains("Sign in another way"),
            "{}",
            h.t.screen()
        );
    }
}

/// The selected gateway's own sign-in (`onboarding/gateway_login.rs`): after
/// the sign-in step ends on a gateway, the host starts its server and signs
/// in (`Ask::SelectGateway`, `GatewayLogins`, `GatewayOAuth`) while the
/// step's states show where it stands; Enter or a tap retries a failure.
mod gateway_oauth {
    use super::*;
    use codex_tui_core::screens::onboarding::auth::State;
    use codex_tui_core::screens::onboarding::gateway::Logins;

    fn asks(h: &Harness) -> Vec<Ask> {
        h.fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Startup { ask, .. } => Some(ask),
                _ => None,
            })
            .collect()
    }

    /// The numbers of the asks of one kind, in order.
    fn requests(h: &Harness, kind: fn(&Ask) -> Option<u64>) -> Vec<u64> {
        asks(h).iter().filter_map(kind).collect()
    }

    /// The number of the last ask of one kind.
    #[track_caller]
    fn last(h: &Harness, kind: fn(&Ask) -> Option<u64>, what: &str) -> u64 {
        match requests(h, kind).last() {
            Some(request) => *request,
            None => panic!("{what}"),
        }
    }

    fn select(ask: &Ask) -> Option<u64> {
        match ask {
            Ask::SelectGateway { request, .. } => Some(*request),
            _ => None,
        }
    }

    fn logins_read(ask: &Ask) -> Option<u64> {
        match ask {
            Ask::GatewayLogins { request, .. } => Some(*request),
            _ => None,
        }
    }

    fn sign_in(ask: &Ask) -> Option<u64> {
        match ask {
            Ask::GatewayOAuth { request, .. } => Some(*request),
            _ => None,
        }
    }

    fn cancelled(h: &Harness) -> usize {
        asks(h)
            .iter()
            .filter(|ask| matches!(ask, Ask::CancelGatewayOAuth))
            .count()
    }

    fn inject(h: &mut Harness, epoch: ConnEpoch, event: Ev) {
        h.fake.inject(epoch, ev(event));
        h.settle();
    }

    /// The sign-in step ended on the gateway `office` and the host's server
    /// runs with it: its account and provider are read, the gateway has an
    /// OAuth; the number of the sign-in's ask.
    fn signing_in() -> (Harness, ConnEpoch, u64) {
        let (mut h, epoch) = launch();
        h.t.resize(72, 8);
        inject(
            &mut h,
            epoch,
            Ev::SignInState(State::GatewaySelected("office".into())),
        );
        let request = last(&h, select, "the gateway's start is asked");
        inject(&mut h, epoch, Ev::GatewayStarted { request });
        let request = last(&h, logins_read, "the logins are read");
        inject(
            &mut h,
            epoch,
            Ev::GatewayLogins {
                request,
                logins: Logins {
                    primary: false,
                    oauth: true,
                    offer: Offer::default(),
                },
            },
        );
        let request = last(&h, sign_in, "the sign-in is asked");
        (h, epoch, request)
    }

    const STARTING: &str = "Starting gateway sign-in\u{2026}\nPress Ctrl+C to quit.";
    const FAILED: &str = "Gateway sign-in expired.\nPress Enter to retry or Ctrl+C to quit.";

    // `start_gateway_oauth` is under way: the title, and how to quit.
    #[test]
    fn the_step_shows_the_sign_in_starting() {
        let (h, _, _) = signing_in();
        assert_visible(&h, STARTING);
    }

    // `account/gatewayOAuth/changed`: the page to finish in, a link.
    #[test]
    fn the_page_to_sign_in_on_is_shown_when_the_server_names_it() {
        let (mut h, epoch, _) = signing_in();
        inject(
            &mut h,
            epoch,
            Ev::GatewayOAuthChanged {
                provider_id: "office".into(),
                auth_url: Some("https://gateway.example/login".into()),
            },
        );
        assert_visible(
            &h,
            "Finish signing in to your gateway in the browser.\nhttps://gateway.example/login\nPress Ctrl+C to quit.",
        );
    }

    fn failed(h: &mut Harness, epoch: ConnEpoch, request: u64) {
        inject(
            h,
            epoch,
            Ev::GatewayOAuth {
                request,
                result: Err("Gateway sign-in expired.".into()),
            },
        );
        assert_visible(h, FAILED);
    }

    // Enter on the failure asks again, with a new number: the answer to the
    // first is not the second's.
    #[test]
    fn enter_retries_a_failed_sign_in() {
        let (mut h, epoch, first) = signing_in();
        failed(&mut h, epoch, first);
        step(&mut h, "enter");
        let all = requests(&h, sign_in);
        assert_eq!(all.len(), 2, "{all:?}");
        assert_ne!(all[0], all[1]);
        assert_visible(&h, STARTING);
        // A late answer of the first attempt changes nothing.
        inject(
            &mut h,
            epoch,
            Ev::GatewayOAuth {
                request: first,
                result: Err("late".into()),
            },
        );
        assert_visible(&h, STARTING);
    }

    // ADR-011: the retry line is a tap target, as Enter is; the message is
    // not.
    #[test]
    fn a_tap_on_the_retry_line_retries() {
        let (mut h, epoch, first) = signing_in();
        failed(&mut h, epoch, first);
        h.t.click(3, 0);
        h.settle();
        assert_eq!(requests(&h, sign_in).len(), 1, "the message is no button");
        h.t.click(3, 1);
        h.settle();
        assert_eq!(requests(&h, sign_in).len(), 2);
        assert_visible(&h, STARTING);
    }

    // The sign-in succeeded: the page leaves for the launch.
    #[test]
    fn a_signed_in_gateway_goes_on_to_the_launch() {
        let (mut h, epoch, request) = signing_in();
        inject(
            &mut h,
            epoch,
            Ev::GatewayOAuth {
                request,
                result: Ok(()),
            },
        );
        let all = asks(&h);
        assert!(all.contains(&Ask::ReloadConfig), "{all:?}");
        assert!(
            all.iter()
                .any(|ask| matches!(ask, Ask::GatewayDevice { .. })),
            "{all:?}"
        );
        assert_eq!(cancelled(&h), 0);
    }

    // Ctrl+C while the sign-in runs cancels it on the server (the oracle's
    // `GatewayOAuthCancel`) before the app ends.
    #[test]
    fn quitting_cancels_the_sign_in() {
        let (mut h, _, _) = signing_in();
        h.press("ctrl+c");
        // The cancel ask reaches the fake on the runtime thread: settle first.
        h.settle();
        assert_eq!(cancelled(&h), 1);
    }
}

/// The gateway's device code at the end of the launch
/// (`device_login::start_if_needed`, `run_gateway_device_login`): the screen
/// the oracle draws (`device_login_tests.rs`, 80x16), the asks it makes, and
/// how it ends: the attempt's end, Esc / Ctrl+C / `q`, a tap on the cancel
/// line (ADR-011).
mod gateway_device {
    use super::*;
    use codex_app_server_protocol::{
        GatewayOAuthChangedNotification, GatewayOAuthStatus, ServerNotification,
    };
    use codex_tui_core::app::BackendEv;
    use codex_tui_core::screens::onboarding::auth::DeviceCode;

    fn asks(h: &Harness) -> Vec<Ask> {
        h.fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Startup { ask, .. } => Some(ask),
                _ => None,
            })
            .collect()
    }

    fn cancelled(h: &Harness) -> usize {
        asks(h)
            .iter()
            .filter(|ask| matches!(ask, Ask::CancelGatewayOAuth))
            .count()
    }

    fn inject(h: &mut Harness, epoch: ConnEpoch, event: AppEv) {
        h.fake.inject(epoch, event);
        h.settle();
    }

    fn changed(status: GatewayOAuthStatus, error: Option<&str>) -> AppEv {
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::GatewayOAuthChanged(GatewayOAuthChangedNotification {
                login_id: Some("attempt".into()),
                auth_url: None,
                provider_id: "office".into(),
                status,
                error: error.map(str::to_owned),
            }),
        )))
    }

    /// The app at the oracle's 80x16 with the launch's facts read (no step
    /// due): the host is asked for the gateway's device code.
    fn asked() -> (Harness, ConnEpoch, u64) {
        let mut h = crate::common::start_driven(false, crate::common::WIDTH, crate::common::HEIGHT);
        h.settle();
        let Some(epoch) = h.fake.epoch() else {
            panic!("the app attached no backend");
        };
        h.t.resize(80, 16);
        h.answer_startup_facts(Facts::default());
        let request = h.startup_request("the gateway device ask", |ask| match ask {
            Ask::GatewayDevice {
                request,
                thread: None,
            } => Some(*request),
            _ => None,
        });
        (h, epoch, request)
    }

    /// The same with the code on screen.
    fn showing() -> (Harness, ConnEpoch) {
        let (mut h, epoch, request) = asked();
        inject(
            &mut h,
            epoch,
            ev(Ev::GatewayDevice {
                request,
                code: Some(DeviceCode {
                    login_id: "attempt".into(),
                    url: "https://login.example.com/device".into(),
                    code: "ABCD-EFGH".into(),
                }),
            }),
        );
        h.wait_for("the device code", |s| s.contains("ABCD-EFGH"));
        (h, epoch)
    }

    /// The oracle's snapshot
    /// (`device_prompt_shows_where_to_enter_the_code_and_how_to_cancel`): the
    /// warning wraps without the indent the other rows keep.
    const SCREEN: &str = "  Sign in to your gateway

  1. Open this link in a browser

  https://login.example.com/device

  2. Enter this one-time code when prompted

  ABCD-EFGH

  Continue only if you started this login in Codex. If a website or another
person gave you this code, cancel.

  Press esc to cancel";

    // `lib.rs`: the code is asked once the facts are read, before the
    // launch's prompts.
    #[test]
    fn the_device_code_is_asked_once_the_facts_are_read() {
        let (h, _, request) = asked();
        assert!(
            asks(&h).contains(&Ask::GatewayDevice {
                request,
                thread: None
            }),
            "{:?}",
            asks(&h)
        );
    }

    #[test]
    fn the_screen_is_the_oracles() {
        let (h, _) = showing();
        assert_visible(&h, SCREEN);
    }

    // `handle_event_during_device_login`: the attempt that succeeded goes on
    // to the launch's prompts, the page leaves.
    #[test]
    fn the_attempt_succeeding_goes_on_to_the_launch() {
        let (mut h, epoch) = showing();
        inject(&mut h, epoch, changed(GatewayOAuthStatus::Succeeded, None));
        assert!(
            asks(&h).iter().any(|ask| matches!(ask, Ask::Launch { .. })),
            "{:?}",
            asks(&h)
        );
        assert!(!h.t.screen().contains("ABCD-EFGH"), "\n{}", h.t.screen());
        assert_eq!(cancelled(&h), 0);
    }

    #[test]
    fn the_attempt_failing_ends_the_launch_with_the_servers_message() {
        let (mut h, epoch) = showing();
        inject(
            &mut h,
            epoch,
            changed(GatewayOAuthStatus::Failed, Some("Denied")),
        );
        let all = asks(&h);
        assert!(
            all.contains(&Ask::FailLaunch {
                message: "Denied".into()
            }),
            "{all:?}"
        );
    }

    // Starting is not ending: the code stays.
    #[test]
    fn a_start_changes_nothing() {
        let (mut h, epoch) = showing();
        inject(&mut h, epoch, changed(GatewayOAuthStatus::Started, None));
        assert_visible(&h, SCREEN);
    }

    // `keys::CANCEL` and `keys::QUIT` quit this screen
    // (`run_gateway_device_login` cancels the attempt on the way out).
    #[test]
    fn esc_and_the_quit_keys_cancel_the_attempt() {
        for key in ["esc", "ctrl+c", "ctrl+d", "q"] {
            let (mut h, _) = showing();
            h.press(key);
            // the ask travels to the fake on the runtime thread
            h.settle();
            assert_eq!(cancelled(&h), 1, "{key}");
        }
    }

    // ADR-011: the cancel line is a tap target, as Esc is.
    #[test]
    fn a_tap_on_the_cancel_line_cancels_the_attempt() {
        let (mut h, _) = showing();
        // The code is not a target.
        h.t.click(4, 8);
        h.settle();
        assert_eq!(cancelled(&h), 0);
        h.t.click(4, 13);
        h.settle();
        assert_eq!(cancelled(&h), 1);
    }
}
