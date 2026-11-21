//! The strings of the provider setup wizards (S03): the oracle's text byte
//! for byte (`onboarding/gcp.rs`, `bedrock.rs`, `gateway.rs`).

use rtwk::l10n::DependCx;

use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of the provider wizards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The wizard title's mark and words.
    pub gcp_title: [&'static str; 2],
    pub gcp_project_guidance: &'static str,
    pub gcp_project_label: &'static str,
    pub gcp_project_placeholder: &'static str,
    pub gcp_location_guidance: &'static str,
    pub gcp_location_label: &'static str,
    pub gcp_location_placeholder: &'static str,
    pub gcp_credentials_guidance: &'static str,
    pub gcp_credentials_label: &'static str,
    pub gcp_adc_hint: &'static str,
    pub gcp_adc_command: &'static str,
    pub gcp_configuring: &'static str,
    pub gcp_configured: &'static str,
    pub gcp_project_required: &'static str,
    /// The footer's hints: `  `, `enter`, ` continue`, ` set up`, ` · `,
    /// `esc`, ` back`.
    pub indent: &'static str,
    pub enter: &'static str,
    pub esc: &'static str,
    pub to_continue: &'static str,
    pub to_set_up: &'static str,
    pub separator: &'static str,
    pub back: &'static str,
    /// Amazon Bedrock (`onboarding/bedrock.rs`).
    pub bedrock_title: [&'static str; 2],
    pub bedrock_discovering: &'static str,
    pub bedrock_configuring: &'static str,
    pub bedrock_profile_detected: &'static str,
    pub bedrock_region: &'static str,
    pub bedrock_choose_profile: &'static str,
    pub bedrock_environment_detected: &'static str,
    pub bedrock_none_found: &'static str,
    pub bedrock_choose_method: &'static str,
    pub bedrock_profile_guidance: &'static str,
    pub bedrock_profile_label: &'static str,
    pub bedrock_api_key_guidance: &'static str,
    pub bedrock_api_key_label: &'static str,
    pub bedrock_region_guidance: &'static str,
    pub bedrock_region_label: &'static str,
    pub bedrock_access_keys_guidance: &'static str,
    /// The access key fields' labels, after the marker and a blank.
    pub bedrock_access_key_labels: [&'static str; 3],
    pub bedrock_environment_guidance: &'static str,
    pub bedrock_setup_guide: &'static str,
    pub bedrock_setup_guide_url: &'static str,
    /// `Continue with ` before a profile's name.
    pub bedrock_continue_with: &'static str,
    pub bedrock_existing_credentials: &'static str,
    pub bedrock_detected_credentials: &'static str,
    pub bedrock_environment_api_key: &'static str,
    pub bedrock_environment_access_keys: &'static str,
    pub bedrock_other_methods: &'static str,
    pub bedrock_other_methods_description: &'static str,
    pub bedrock_another_method: &'static str,
    /// The methods of the full list: title and description.
    pub bedrock_manual_profile: [&'static str; 2],
    pub bedrock_access_keys: [&'static str; 2],
    pub bedrock_environment_variables: [&'static str; 2],
    pub bedrock_api_key: [&'static str; 2],
    /// `  Press `, ` to continue`, ` to go back`.
    pub press: &'static str,
    pub press_to_continue: &'static str,
    pub press_to_go_back: &'static str,
    /// The errors, before the server's message.
    pub bedrock_discover_failed: &'static str,
    pub bedrock_setup_failed: &'static str,
    pub bedrock_configured: &'static str,
    /// The GovCloud guidance at launch.
    pub gov_cloud_title: &'static str,
    /// The text before the link, the link and the text after it.
    pub gov_cloud_text: [&'static str; 3],
    pub gov_cloud_url: &'static str,
    pub acknowledge: &'static str,
    /// `↑`, `/`, `↓`, ` scroll · `, `enter`.
    pub gov_cloud_scroll: [&'static str; 5],
    /// The masked character of a secret field.
    pub secret: &'static str,
}

pub static EN: Strings = Strings {
    gcp_title: ["> ", "Set up Google Cloud Vertex AI"],
    gcp_project_guidance: "  Enter the Google Cloud project ID to use with Vertex AI.",
    gcp_project_label: "  Project ID: ",
    gcp_project_placeholder: "my-project",
    gcp_location_guidance: "  Press Enter to use global, or enter a Vertex AI location.",
    gcp_location_label: "  Location (optional): ",
    gcp_location_placeholder: "global",
    gcp_credentials_guidance: "  Enter an absolute file path on the host running Codex, or press Enter to use Application Default Credentials (ADC).",
    gcp_credentials_label: "  Credential file (optional): ",
    gcp_adc_hint: "  To set up ADC with the Google Cloud CLI, run",
    gcp_adc_command: "  gcloud auth application-default login",
    gcp_configuring: "  Validating credentials and saving configuration\u{2026}",
    gcp_configured: "\u{2713} Google Cloud Vertex AI configured",
    gcp_project_required: "Enter a Google Cloud project ID.",
    indent: "  ",
    enter: "enter",
    esc: "esc",
    to_continue: " continue",
    to_set_up: " set up",
    separator: " \u{b7} ",
    back: " back",
    bedrock_title: ["> ", "Set up Amazon Bedrock"],
    bedrock_discovering: "  Checking for existing AWS credentials...",
    bedrock_configuring: "  Setting up Amazon Bedrock...",
    bedrock_profile_detected: "  AWS profile detected: ",
    bedrock_region: "  Region: ",
    bedrock_choose_profile: "  Choose an AWS profile.",
    bedrock_environment_detected: "  AWS credentials detected in the server environment.",
    bedrock_none_found: "  No AWS credentials found.",
    bedrock_choose_method: "  Choose how you authenticate with AWS.",
    bedrock_profile_guidance: "  Enter the name of your AWS profile.",
    bedrock_profile_label: "  AWS profile: ",
    bedrock_api_key_guidance: "  Enter your Amazon Bedrock API key.",
    bedrock_api_key_label: "  Bedrock API key: ",
    bedrock_region_guidance: "  Enter the AWS Region to use with Amazon Bedrock.",
    bedrock_region_label: "  AWS Region: ",
    bedrock_access_keys_guidance: "  Enter your AWS access keys.",
    bedrock_access_key_labels: [
        "AWS access key ID: ",
        "AWS secret access key: ",
        "AWS session token (optional): ",
    ],
    bedrock_environment_guidance: "  Export AWS credentials, then run `codex --no-daemon` in that shell.",
    bedrock_setup_guide: "  Setup guide: ",
    bedrock_setup_guide_url: "https://learn.chatgpt.com/docs/amazon-bedrock",
    bedrock_continue_with: "Continue with ",
    bedrock_existing_credentials: "Use your existing AWS credentials",
    bedrock_detected_credentials: "Continue with detected credentials",
    bedrock_environment_api_key: "Use the Bedrock API key from the server environment",
    bedrock_environment_access_keys: "Use AWS access keys from the server environment",
    bedrock_other_methods: "Other AWS sign-in methods",
    bedrock_other_methods_description: "Use another profile, access keys, environment variables, or a Bedrock API key",
    bedrock_another_method: "Choose another sign-in method",
    bedrock_manual_profile: ["AWS profile", "Use AWS SSO or a named profile"],
    bedrock_access_keys: [
        "AWS access keys",
        "Enter an access key ID and secret access key",
    ],
    bedrock_environment_variables: [
        "Environment variables",
        "Use AWS credentials exported in your shell",
    ],
    bedrock_api_key: ["Bedrock API key", "Enter a Bedrock API key"],
    press: "  Press ",
    press_to_continue: " to continue",
    press_to_go_back: " to go back",
    bedrock_discover_failed: "Unable to check AWS credentials: ",
    bedrock_setup_failed: "Unable to set up Amazon Bedrock: ",
    bedrock_configured: "\u{2713} Amazon Bedrock configured",
    gov_cloud_title: "Using Codex with AWS GovCloud",
    gov_cloud_text: [
        "You are using Codex with AWS GovCloud. Please ensure you or your administrator have read the ",
        "application configuration and security guidance",
        " before proceeding.",
    ],
    gov_cloud_url: "https://learn.chatgpt.com/docs/enterprise/govcloud-configuration",
    acknowledge: "> Acknowledge",
    gov_cloud_scroll: ["\u{2191}", "/", "\u{2193}", " scroll \u{b7} ", "enter"],
    secret: "\u{2022}",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
