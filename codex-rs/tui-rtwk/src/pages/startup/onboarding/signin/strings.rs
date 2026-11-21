//! The strings of the startup page's sign-in step (S02): a table of
//! `&'static str` fields and typed messages per locale, read through
//! `of(cx)`. `EN` is the oracle's text byte for byte (`onboarding/auth.rs`,
//! `auth/headless_chatgpt_login.rs`, `onboarding_screen.rs`'s copy of the
//! link). Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of the sign-in step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The indent of the step's lines.
    pub indent: &'static str,
    /// The menu's headings: "another way", with providers, the plain one
    /// (two lines).
    pub heading_other: &'static str,
    pub heading_providers: &'static str,
    pub heading_plain: [&'static str; 2],
    /// An option's number: the cursor's (`> 1. `) and the others' (`  1. `).
    option_cursor: Msg<1>,
    option_other: Msg<1>,
    /// The description's indent under an option.
    pub description_indent: &'static str,
    pub chatgpt: &'static str,
    pub chatgpt_description: &'static str,
    pub chatgpt_disabled: &'static str,
    pub device_code: &'static str,
    pub device_code_description: &'static str,
    pub other: &'static str,
    pub other_description_providers: &'static str,
    pub other_description_plain: &'static str,
    pub api_key: &'static str,
    pub api_key_description: &'static str,
    pub bedrock: &'static str,
    pub bedrock_description: &'static str,
    pub gcp: &'static str,
    pub gcp_description: &'static str,
    pub gateway: &'static str,
    pub gateway_description: &'static str,
    pub api_login_disabled_note: &'static str,
    /// The key hints: `  Press `, `enter`, ` to continue`, `esc`,
    /// ` to go back`, ` to cancel`, ` to save`.
    pub press: &'static str,
    pub enter: &'static str,
    pub esc: &'static str,
    pub to_continue: &'static str,
    pub to_go_back: &'static str,
    pub to_cancel: &'static str,
    pub to_save: &'static str,
    /// The browser login.
    pub finish_in_browser: &'static str,
    pub copy_link: [&'static str; 3],
    pub headless: [&'static str; 4],
    /// The copy of the link: the clipboard is written (OSC 52), unconfirmed.
    pub copy_requested: &'static str,
    /// The device code login.
    pub preparing_device_code: &'static str,
    pub device_code_steps: [&'static str; 2],
    pub device_code_warning: &'static str,
    pub requesting_code: &'static str,
    /// Signed in.
    pub signed_in: &'static str,
    pub in_control: &'static str,
    pub learn_permissions: &'static str,
    pub codex_docs: &'static str,
    pub codex_docs_url: &'static str,
    pub mistakes: &'static str,
    pub review_code: &'static str,
    pub included: &'static str,
    pub plan_limits: &'static str,
    pub training_preferences: &'static str,
    pub training_preferences_url: &'static str,
    pub apply: &'static str,
    /// The API key.
    pub api_key_intro: [&'static str; 2],
    pub api_key_paste: &'static str,
    pub env_detected: &'static str,
    pub env_other: &'static str,
    pub api_key_title: &'static str,
    pub api_key_placeholder: &'static str,
    pub api_key_configured: &'static str,
    pub api_key_billing: &'static str,
    /// The errors (core's `auth::Error`).
    pub api_key_disabled: &'static str,
    pub empty_api_key: &'static str,
    save_failed: Msg<1>,
    unexpected: Msg<1>,
}

impl Strings {
    /// `> 1. ` (the cursor's) or `  1. `.
    pub fn option_number(&self, number: usize, cursor: bool, num: &NumSyms) -> String {
        let msg = if cursor {
            self.option_cursor
        } else {
            self.option_other
        };
        Fmt::new(msg, [Arg::Int(number as u64)], num).to_string()
    }

    /// `Failed to save API key: <error>`.
    pub fn save_failed(&self, error: &str, num: &NumSyms) -> String {
        Fmt::new(self.save_failed, [Arg::Text(error)], num).to_string()
    }

    /// `Unexpected account/login/start response: <response>`.
    pub fn unexpected(&self, response: &str, num: &NumSyms) -> String {
        Fmt::new(self.unexpected, [Arg::Text(response)], num).to_string()
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    indent: "  ",
    heading_other: "Sign in another way",
    heading_providers: "Choose how you want to use Codex.",
    heading_plain: [
        "Sign in with ChatGPT to use Codex as part of your paid plan",
        "or connect an API key for usage-based billing",
    ],
    option_cursor: msg!("> {0}. "),
    option_other: msg!("  {0}. "),
    description_indent: "     ",
    chatgpt: "Sign in with ChatGPT",
    chatgpt_description: "Usage included with Plus, Pro, Business, and Enterprise plans",
    chatgpt_disabled: "ChatGPT login is disabled",
    device_code: "Sign in with Device Code",
    device_code_description: "Sign in from another device with a one-time code",
    other: "Sign in another way",
    other_description_providers: "Use an API key or another provider",
    other_description_plain: "Use an OpenAI API key",
    api_key: "Use an OpenAI API key",
    api_key_description: "Pay for what you use",
    bedrock: "Use Amazon Bedrock",
    bedrock_description: "Connect using your AWS credentials",
    gcp: "Use Google Cloud Vertex AI",
    gcp_description: "Connect using your Google Cloud credentials",
    gateway: "Use a custom provider or gateway",
    gateway_description: "Choose a saved configuration or create a new one",
    api_login_disabled_note: "  API key login is disabled by this workspace. Sign in with ChatGPT to continue.",
    press: "  Press ",
    enter: "enter",
    esc: "esc",
    to_continue: " to continue",
    to_go_back: " to go back",
    to_cancel: " to cancel",
    to_save: " to save",
    finish_in_browser: "Finish signing in via your browser",
    copy_link: [
        "  If the link doesn't open automatically, press ",
        "c",
        " to copy it:",
    ],
    headless: [
        "  On a remote or headless machine? Press ",
        " and choose ",
        "Sign in with Device Code",
        ".",
    ],
    copy_requested: "Copy requested; check your clipboard",
    preparing_device_code: "Preparing device code login",
    device_code_steps: [
        "  1. Open this link in your browser and sign in",
        "  2. Enter this one-time code after you are signed in (expires in 15 minutes)",
    ],
    device_code_warning: "  Continue only if you started this login in Codex. If a website or another person gave you this code, cancel.",
    requesting_code: "  Requesting a one-time code...",
    signed_in: "\u{2713} Signed in with your ChatGPT account",
    in_control: "  You're in control",
    learn_permissions: "  Learn about permissions and approvals in the ",
    codex_docs: "Codex docs",
    codex_docs_url: "https://developers.openai.com/codex/security",
    mistakes: "  Codex can make mistakes",
    review_code: "  Review the code it writes and commands it runs",
    included: "  Included with your ChatGPT plan",
    plan_limits: "  Your plan's rate limits and ",
    training_preferences: "training data preferences",
    training_preferences_url: "https://chatgpt.com/#settings",
    apply: " apply",
    api_key_intro: ["> ", "Use your own OpenAI API key for usage-based billing"],
    api_key_paste: "  Paste or type your API key below.",
    env_detected: "  Detected OPENAI_API_KEY environment variable.",
    env_other: "  Paste a different key if you prefer to use another account.",
    api_key_title: "API key",
    api_key_placeholder: "Paste or type your API key",
    api_key_configured: "\u{2713} API key configured",
    api_key_billing: "  Codex will use usage-based billing with your API key.",
    api_key_disabled: "API key login is disabled.",
    empty_api_key: "API key cannot be empty",
    save_failed: msg!("Failed to save API key: {0}"),
    unexpected: msg!("Unexpected account/login/start response: {0}"),
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
