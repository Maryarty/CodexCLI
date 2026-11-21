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
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
