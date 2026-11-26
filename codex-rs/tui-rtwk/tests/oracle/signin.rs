//! Shard `signin` (screens S02-S03, owner onboarding): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Each screen is the startup page's sign-in step opened from a debug entry
//! of the `onboarding` directory, over the fixture script that makes the
//! oracle test's widget state (`src/pages/startup/onboarding/fixtures/`).
//! The provider wizards (S03): Bedrock, Google Cloud and the gateway wizard
//! are built, the states after a server's answer are compared in
//! tests/pages/signin.rs and tests/pages/signin_wizards.rs; the GovCloud
//! guidance's launch step is startup-finish's.

use crate::kit::{self, Owner, Screen};

fn open(entry: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::Onboarding)
        .size(width, height)
        .entry("onboarding", entry)
        .open()
}

/// The URL of the oracle's `PRODUCTION_LENGTH_AUTH_URL` (fixture
/// `browser_long_url`).
const PRODUCTION_LENGTH_AUTH_URL: &str = concat!(
    "https://auth.openai.com/oauth/authorize?",
    "response_type=code&",
    "client_id=app_EMoamEEZ73f0CkXaXp7hrann&",
    "redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback&",
    "scope=openid%20profile%20email%20offline_access%20",
    "api.connectors.read%20api.connectors.invoke&",
    "code_challenge=1YM3Z8QbrLbdt9C3eX3j7UQ4GmFRmKz4OeVYwD6s5xA&",
    "code_challenge_method=S256&",
    "id_token_add_organizations=true&",
    "codex_cli_simplified_flow=true&",
    "state=8cHjQ4nVx2Yp7Lm9Rk3Wf6Ta1Bs5Du0Ei4Go7Nz2PqM&",
    "originator=codex_cli_rs"
);

// ---- oracle: onboarding/auth.rs (6 screen-level tests here; 10 state tests left to onboarding)
// ORACLE onboarding/auth.rs:1502 api_key_entry_snapshots kind=snap owner=onboarding snap=api_key_entry snap=api_key_entry_narrow
#[test]
fn api_key_entry_snapshots() {
    let s = open("api_key_entry", 80, 14);
    s.assert_snap("api_key_entry");
    let s = open("api_key_entry", 40, 14);
    kit::pending(
        Owner::Onboarding,
        "G48: rtwk's `TextWrap::Word` breaks after the hyphen of `usage-based` (`> Use your own OpenAI API key for usage-` / `based billing`); the oracle's `Paragraph` wraps at blanks only (`... for` / `usage-based billing`), and no `TextWrap` breaks at blanks only",
    );
    s.assert_snap("api_key_entry_narrow");
}

// ORACLE onboarding/auth.rs:1541 bedrock_option_requires_feature_and_api_login_permission kind=snap owner=onboarding
#[test]
fn bedrock_option_requires_feature_and_api_login_permission() {
    // Without the wizard's feature, "another way" offers the API key alone.
    let mut s = open("sign_in_menu", 76, 19);
    s.keys("3");
    s.assert_contains("1. Use an OpenAI API key");
    s.assert_absent("Use Amazon Bedrock");
    // With it, Bedrock follows; Esc goes back to "another way" and Down
    // wraps to the first option.
    let mut s = open("sign_in_bedrock", 76, 19);
    s.keys("3");
    s.assert_contains("1. Use an OpenAI API key");
    s.assert_contains("2. Use Amazon Bedrock");
    s.keys("esc down");
    let want = [
        "  Choose how you want to use Codex.",
        "",
        "> 1. Sign in with ChatGPT",
        "     Usage included with Plus, Pro, Business, and Enterprise plans",
        "",
        "  2. Sign in with Device Code",
        "     Sign in from another device with a one-time code",
        "",
        "  3. Sign in another way",
        "     Use an API key or another provider",
        "",
        "  Press enter to continue",
    ];
    for (row, text) in want.iter().enumerate() {
        s.assert_row(row, text);
    }
    // ChatGPT forced: the menu is ChatGPT and the device code.
    let s = open("sign_in_forced_chatgpt", 76, 19);
    s.assert_contains("1. Sign in with ChatGPT");
    s.assert_contains("2. Sign in with Device Code");
    s.assert_absent("Sign in another way");
}

// ORACLE onboarding/auth.rs:1708 continue_in_browser_preserves_long_link_and_footer_at_narrow_width kind=snap owner=onboarding snap=continue_in_browser_narrow_long_url
#[test]
fn continue_in_browser_preserves_long_link_and_footer_at_narrow_width() {
    let s = open("browser_long_url", 44, 30);
    s.assert_link(
        "https://auth.openai.com/oauth/authorize?re",
        PRODUCTION_LENGTH_AUTH_URL,
    );
    s.assert_snap("continue_in_browser_narrow_long_url");
    s.assert_contains("On a remote or headless machine?");
    s.assert_contains("Press esc to cancel");
}

// ORACLE onboarding/auth.rs:1747 chatgpt_success_message_renders_osc8_hyperlinks kind=snap owner=onboarding
#[test]
fn chatgpt_success_message_renders_osc8_hyperlinks() {
    let s = open("success_message", 80, 12);
    s.assert_link("Codex docs", "https://developers.openai.com/codex/security");
    s.assert_link("training data preferences", "https://chatgpt.com/#settings");
    // The oracle's cells 0..47 of row 3 and 0..30 of row 9 are dim, the
    // indent's two blanks included; a terminal shows no dim on a blank, and
    // rtwk's presenter writes such blanks as erased cells
    // (`Attrs::VISIBLE_ON_BLANK`, `StyleTable::erasable`), so the screen's
    // blanks before the text carry none: the text's cells are checked.
    s.assert_text_style(
        "Learn about permissions and approvals in the",
        "dim !underlined",
    );
    s.assert_text_style("Codex docs", "dim underlined");
    s.assert_text_style("Your plan's rate limits and", "dim !underlined");
    s.assert_text_style("training data preferences", "dim underlined");
    s.assert_text_style("apply", "dim !underlined");
    let want = [
        "✓ Signed in with your ChatGPT account",
        "",
        "  You're in control",
        "  Learn about permissions and approvals in the Codex docs",
        "",
        "  Codex can make mistakes",
        "  Review the code it writes and commands it runs",
        "",
        "  Included with your ChatGPT plan",
        "  Your plan's rate limits and training data preferences apply",
        "",
        "  Press enter to continue",
    ];
    for (row, text) in want.iter().enumerate() {
        s.assert_row(row, text);
    }
}

// ORACLE onboarding/auth.rs:1859 mark_url_hyperlink_wraps_cyan_underlined_cells kind=ui owner=onboarding
#[test]
fn mark_url_hyperlink_wraps_cyan_underlined_cells() {
    kit::na(
        Owner::Onboarding,
        "a ratatui buffer pass that wraps cyan underlined cells in OSC 8 after painting: rtwk paints the link as the text's own (`Hyperlink`); the link over the URL is held by continue_in_browser_preserves_long_link_and_footer_at_narrow_width",
    );
    let s = open("browser_long_url", 44, 30);
    s.assert_link(
        "https://auth.openai.com/oauth/authorize?re",
        PRODUCTION_LENGTH_AUTH_URL,
    );
    s.assert_no_link("Finish signing in via your browser");
}

// ORACLE onboarding/auth.rs:1885 mark_url_hyperlink_sanitizes_control_chars kind=ui owner=onboarding
#[test]
fn mark_url_hyperlink_sanitizes_control_chars() {
    kit::na(
        Owner::Onboarding,
        "the OSC 8 escape is rtwk's presenter's, which writes a link's URL sanitized; the screen has no URL with control characters",
    );
}

// ---- oracle: onboarding/auth/activation_key_tests.rs (1 screen-level tests here; 0 state tests left to onboarding)
// ORACLE onboarding/auth/activation_key_tests.rs:9 held_enter_requires_a_fresh_confirmation_to_save_a_prefilled_api_key kind=ui owner=onboarding
#[test]
fn held_enter_requires_a_fresh_confirmation_to_save_a_prefilled_api_key() {
    // Up wraps to "another way"; Enter opens it. The held Enter (a repeat,
    // then a press within the guard) opens nothing more; Down ends the
    // guard and a repeat does not activate.
    let mut s = open("sign_in_env_key", 80, 20);
    s.keys("up enter");
    s.key_repeat("enter");
    s.keys("enter down");
    s.key_repeat("enter");
    s.assert_contains("> 1. Use an OpenAI API key");
    s.assert_absent("Press enter to save");
    // A fresh Enter opens the entry, prefilled from the environment.
    s.keys("enter");
    s.assert_contains("Detected OPENAI_API_KEY environment variable.");
    // The Enter that opened it, held, saves nothing; nor a repeat after Left.
    // The fresh Enter's save and its confirmation: tests/pages/signin.rs
    // `a_held_enter_saves_the_prefilled_key_only_when_fresh`.
    s.keys("enter");
    s.key_repeat("enter");
    s.keys("left");
    s.key_repeat("enter");
    s.assert_contains("Press enter to save");
    s.assert_absent("API key configured");
}

// ---- oracle: onboarding/bedrock_tests.rs (4 screen-level tests here; 0 state tests left to onboarding)
// ORACLE onboarding/bedrock_tests.rs:6 gov_cloud_guidance_requires_acknowledgement kind=snap owner=onboarding snap=gov_cloud_warning
#[test]
fn gov_cloud_guidance_requires_acknowledgement() {
    let s = open("bedrock_gov_cloud", 72, 24);
    s.assert_snap("gov_cloud_warning");
}

// ORACLE onboarding/bedrock_tests.rs:28 gov_cloud_guidance_scrolls_in_short_panes kind=snap owner=onboarding snap=gov_cloud_warning_short
#[test]
fn gov_cloud_guidance_scrolls_in_short_panes() {
    let s = open("bedrock_gov_cloud", 24, 6);
    s.assert_snap("gov_cloud_warning_short");
}

// ORACLE onboarding/bedrock_tests.rs:87 discovery_prioritizes_profiles_and_groups_other_sign_in_methods kind=snap owner=onboarding
#[test]
fn discovery_prioritizes_profiles_and_groups_other_sign_in_methods() {
    // The wizard opens on the discovery it asks of the server; the lists
    // its answers show are compared in tests/pages/signin_wizards.rs (same
    // name): a fixture cannot answer a request.
    let mut s = open("sign_in_bedrock", 72, 24);
    s.keys("3 2");
    for (row, text) in [
        "> Set up Amazon Bedrock",
        "",
        "  Checking for existing AWS credentials...",
        "",
    ]
    .iter()
    .enumerate()
    {
        s.assert_row(row, text);
    }
}

// ORACLE onboarding/bedrock_tests.rs:385 credential_entries_share_region_and_keep_aws_secrets_hidden kind=snap owner=onboarding
#[test]
fn credential_entries_share_region_and_keep_aws_secrets_hidden() {
    kit::pending(
        Owner::Onboarding,
        "G55: the pinned TextField's typed-last reveal ends on blur. The oracle reveals the last grapheme of already-filled text when its field regains focus.",
    );

    let discover = |result| {
        let mut s = Screen::bedrock_sign_in(72, 24);
        s.keys("3").wait_for("Use Amazon Bedrock").keys("2");
        s.wait_for("Checking for existing AWS credentials...");
        s.answer_bedrock_discovery(result);
        s
    };
    for discovery in [
        serde_json::json!({
            "profiles": [{"name": "engineering", "region": null}],
            "environmentCredentials": []
        }),
        serde_json::json!({
            "profiles": [],
            "environmentCredentials": [{"type": "accessKeys", "region": null}]
        }),
    ] {
        let mut s = discover(discovery);
        s.wait_for("Press enter to continue")
            .keys("enter")
            .wait_for("AWS Region:");
        s.type_text("us-west-1");
        s.assert_contains("AWS Region: us-west-1");
    }

    let mut s = discover(serde_json::json!({"profiles": [], "environmentCredentials": []}));
    // Move to access keys rather than repeat the digit that opened the wizard (its key guard).
    s.wait_for("2. AWS access keys")
        .keys("down")
        .wait_for("> 2. AWS access keys")
        .keys("enter")
        .wait_for("> AWS access key ID:");
    s.type_text("AKIAEXAMPLE");
    s.keys("down")
        .wait_for("> AWS secret access key:")
        .type_text("secret-value");
    s.assert_contains("> AWS secret access key: •••••••••••e");
    s.keys("down")
        .wait_for("> AWS session token (optional):")
        .paste("session-value");
    s.assert_contains("AWS secret access key: ••••••••••••");
    s.keys("up").wait_for("> AWS secret access key:");
    s.assert_contains("AWS access key ID: AKIAEXAMPLE");
    s.assert_contains("AWS session token (optional): •••••••••••••");
    s.assert_absent("secret-value");
    s.assert_absent("session-value");
    s.assert_contains("> AWS secret access key: •••••••••••e");
}

// ---- oracle: onboarding/gateway_tests.rs (3 screen-level tests here; 0 state tests left to onboarding)
// ORACLE onboarding/gateway_tests.rs:33 select_a_saved_gateway_during_sign_in kind=snap owner=onboarding snap=codex_tui__onboarding__auth__gateway_tests__gateway_empty_picker snap=codex_tui__onboarding__auth__gateway_tests__gateway_load_error snap=codex_tui__onboarding__auth__gateway_tests__gateway_saved_picker snap=codex_tui__onboarding__auth__gateway_tests__gateway_sign_in_menu
#[test]
fn select_a_saved_gateway_during_sign_in() {
    // The menu, and the wizard that opens on the list it asks of the
    // server; the pickers its answers show (empty, saved, the load error)
    // and the selection are compared in tests/pages/signin_wizards.rs (same
    // name): a fixture cannot answer a request.
    let mut s = open("sign_in_gateway", 72, 8);
    s.keys("3");
    s.assert_snap("gateway_sign_in_menu");
    s.keys("4");
    s.assert_row(0, "Loading gateways…");
}

// ORACLE onboarding/gateway_tests.rs:79 create_highlights_the_saved_gateway_until_explicitly_selected kind=snap owner=onboarding snap=codex_tui__onboarding__auth__gateway_tests__gateway_created_picker snap=codex_tui__onboarding__auth__gateway_tests__gateway_save_error_short snap=codex_tui__onboarding__auth__gateway_tests__gateway_save_focused_short snap=codex_tui__onboarding__auth__gateway_tests__gateway_validation_retry
#[test]
fn create_highlights_the_saved_gateway_until_explicitly_selected() {
    // The picker, the form and its four snapshots (created_picker,
    // save_error_short, save_focused_short, validation_retry) need the
    // list and the save answered: tests/pages/signin_wizards.rs (same name).
    // Here the way to them: the menu row opens the wizard on its load.
    let mut s = open("sign_in_gateway", 72, 8);
    s.keys("3 4");
    s.assert_row(0, "Loading gateways…");
    s.assert_row(1, "");
}

// ORACLE onboarding/gateway_tests.rs:184 gateway_oauth_displays_the_browser_link_and_retry_action kind=snap owner=onboarding snap=codex_tui__onboarding__auth__gateway_tests__gateway_oauth_browser snap=codex_tui__onboarding__auth__gateway_tests__gateway_oauth_failed
#[test]
fn gateway_oauth_displays_the_browser_link_and_retry_action() {
    let s = open("gateway_oauth_starting", 72, 1);
    s.assert_row(0, "Starting gateway sign-in…");
    let s = open("gateway_oauth_browser", 72, 4);
    s.assert_snap("gateway_oauth_browser");
    let s = open("gateway_oauth_failed", 72, 4);
    s.assert_snap("gateway_oauth_failed");
}

// ---- oracle: onboarding/gcp_tests.rs (1 screen-level tests here; 0 state tests left to onboarding)
// ORACLE onboarding/gcp_tests.rs:16 gcp_menu_gating_and_setup_flow kind=snap owner=onboarding snap=codex_tui__onboarding__auth__gcp_tests__gcp_configured snap=codex_tui__onboarding__auth__gcp_tests__gcp_configuring snap=codex_tui__onboarding__auth__gcp_tests__gcp_credentials_entry snap=codex_tui__onboarding__auth__gcp_tests__gcp_location_entry snap=codex_tui__onboarding__auth__gcp_tests__gcp_project_entry snap=codex_tui__onboarding__auth__gcp_tests__gcp_setup_error_narrow snap=codex_tui__onboarding__auth__gcp_tests__gcp_sign_in_menu snap=codex_tui__onboarding__auth__gcp_tests__gcp_sign_in_menu_short snap=codex_tui__onboarding__auth__gcp_tests__sign_in_menu_short_without_cloud_wizards
#[test]
fn gcp_menu_gating_and_setup_flow() {
    let s = open("sign_in_menu", 76, 8);
    s.assert_snap("sign_in_menu_short_without_cloud_wizards");
    let mut s = open("sign_in_cloud", 76, 21);
    s.keys("3");
    s.key_repeat("3");
    s.assert_snap("gcp_sign_in_menu");
    s.resize(76, 8);
    s.keys("up");
    s.assert_snap("gcp_sign_in_menu_short");
    s.keys("esc 3 1 esc 3 esc enter");
    s.assert_contains("Google Cloud project");
    // The wizard: an empty project is refused, the text is edited (a paste
    // drops its newline, the cursor moves, a second paste lands inside it).
    s.resize(76, 12);
    // The Enter that opened it is held back a second (a terminal that
    // reports no releases); the oracle's test sends the release.
    s.wait(1100);
    s.keys("enter");
    s.assert_contains("Enter a Google Cloud project ID.");
    s.keys("q backspace");
    s.paste("test-ject\n");
    s.keys("left left left left");
    s.paste("pro");
    s.assert_snap("gcp_project_entry");
    s.keys("enter");
    s.assert_snap("gcp_location_entry");
    // Location accepts text normally, and Esc keeps it for the next visit.
    s.paste("us-central1");
    s.keys("esc enter ctrl+u enter");
    s.assert_snap("gcp_credentials_entry");
    // A relative credential path is what the app server refuses (the
    // answer is the app test's: this screen has no host).
    s.paste("relative-adc.json");
    s.keys("enter");
    s.keys("esc enter");
    s.assert_snap("gcp_configuring");
}

// Part of ORACLE onboarding/gcp_tests.rs:16 and gateway_tests.rs:33: the
// sign-in menus their wizard flows start from, which the sign-in step draws
// already (the rest of both tests is pending above).
#[test]
fn provider_tests_sign_in_menus() {
    let s = open("sign_in_menu", 76, 8);
    s.assert_snap("sign_in_menu_short_without_cloud_wizards");
    // A held `3` (a repeat) selects nothing in the menu it opened.
    let mut s = open("sign_in_cloud", 76, 21);
    s.keys("3");
    s.key_repeat("3");
    s.assert_snap("gcp_sign_in_menu");
    s.resize(76, 8);
    s.keys("up");
    s.assert_snap("gcp_sign_in_menu_short");
    s.keys("esc");
    s.assert_contains("> 3. Sign in another way");
    let mut s = open("sign_in_gateway", 72, 8);
    s.keys("3");
    s.assert_snap("gateway_sign_in_menu");
}
