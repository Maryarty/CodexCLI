//! The words and rows of the model providers sheet (oracle `app/provider.rs`,
//! `provider/editor.rs`, `provider/write.rs`): the list, its loading and failure pages, the details
//! document and the form's labels. Core says what each state is (`features::providers`), the rows
//! are made here. The wording is the oracle's English until the catalog has these entries.

use crate::l10n::Lang;
use codex_tui_core::features::providers::form::{Field, Invalid, provider_fields};
use codex_tui_core::features::providers::{
    Failure, ModelProviderSummary, Note, Outcome, Page, ProviderSettings, Saved, Trouble,
};

use crate::widgets::picker::list::{Columns, Item, List, Role, Secondary, Span};

/// The row ids of the pages.
pub const BACK: &str = "providers.back";
pub const CREATE: &str = "providers.create";
/// `provider.{n}` is the n-th provider of the catalog.
pub const PROVIDER: &str = "provider.";

pub const TITLE: &str = "Model providers";
pub const DETAILS_TITLE: &str = "Model provider details";
pub const CREATE_CUSTOM_PROVIDER: &str = "Create a custom provider or gateway";
pub const EDIT_CONFIGURATION: &str = "Edit configuration";

/// The form: the save row, the required mark, the key hints and the environment variable's mark.
pub const SAVE: &str = "  Save";
pub const SAVE_SELECTED: &str = "› Save";
pub const REQUIRED: &str = "* Required";
pub const KEY_TAB: &str = "tab";
pub const KEY_TAB_DOES: &str = " move · ";
pub const KEY_ENTER: &str = "enter";
pub const KEY_ENTER_DOES: &str = " next/save · ";
pub const KEY_ESC: &str = "esc";
pub const KEY_ESC_DOES: &str = " back";
pub const KEY_COMPLETE: &str = "ctrl + space";
pub const VARIABLE_PREFIX: &str = "$";

/// The title of the form that edits `name`.
pub fn edit_title(name: &str) -> String {
    format!("Edit {name}")
}

/// What `ctrl + space` does at the environment variable's field: completes `name`.
pub fn complete_does(name: &str) -> String {
    format!(" complete {name}")
}

/// The title of the directory in the debug menu.
pub fn title(_lang: Lang) -> &'static str {
    TITLE
}

fn back() -> Item {
    Item {
        id: BACK.to_owned(),
        name: "Back".to_owned(),
        ..Item::default()
    }
}

/// A page that waits or failed: the title, what it says under it, and `Back`.
fn page(title: &str, subtitle: Option<String>) -> List {
    List {
        title: Some(title.to_owned()),
        subtitle,
        items: vec![back()],
        panel: true,
        ..List::default()
    }
}

/// `Loading model providers…` (the read under way).
pub fn loading() -> List {
    page("Loading model providers…", None)
}

/// `Saving model provider…` (the write under way).
pub fn saving() -> List {
    page("Saving model provider…", None)
}

/// The list could not be read (`load_error`).
pub fn failed(failure: &Failure) -> List {
    let error = match failure {
        Failure::Offline => "Reconnect to the app server to browse model providers.".to_owned(),
        Failure::Unsupported => {
            "Update the connected app server to browse model providers.".to_owned()
        }
        Failure::Server(error) => format!("Could not load model providers: {error}"),
    };
    page(TITLE, Some(error))
}

/// What a write ended with, as the user reads it.
pub fn outcome(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Saved(Saved::Settings) => "Saved provider configuration.".to_owned(),
        Outcome::Saved(Saved::Default { id }) => {
            format!("/new now uses {id}. Restart the app server and TUI to refresh /model.")
        }
        Outcome::Saved(Saved::Overridden { id, reason }) => format!(
            "Saved {id}. {}",
            reason
                .as_deref()
                .unwrap_or("A higher-priority setting overrides this choice.")
        ),
        Outcome::Trouble(trouble) => trouble_words(trouble),
    }
}

/// Why a write did not go through.
pub fn trouble_words(trouble: &Trouble) -> String {
    match trouble {
        Trouble::Offline => {
            "Could not save provider: Reconnect to the app server and try again.".to_owned()
        }
        Trouble::Message(message) => message.clone(),
        Trouble::Rejected(error) => format!("Could not save provider: {error}"),
        Trouble::Lost { id, error } => {
            format!("Could not confirm whether {id} was saved: {error}")
        }
        Trouble::UnconfirmedCreate => {
            "Could not confirm the save. Check the provider list before adding it again.".to_owned()
        }
    }
}

/// The line the transcript gets (the oracle's `add_info_message` and `add_error_message`).
pub fn note(note: &Note) -> String {
    match note {
        Note::Busy => "A provider change is already saving.".to_owned(),
        Note::Outcome(outcome_) => outcome(outcome_),
    }
}

/// Why a form cannot be saved.
pub fn invalid(invalid: Invalid) -> &'static str {
    match invalid {
        Invalid::ReadOnly => "This provider configuration is read-only.",
        Invalid::EnterName => "Enter a provider name.",
        Invalid::EnterUrl => "Enter a base URL.",
        Invalid::KeyNeedsUrl => "Enter a base URL to use this API key variable.",
        Invalid::BadVariable => "Enter an environment variable name such as TEAM_API_KEY.",
        Invalid::UnknownBedrock => "Unrecognized Bedrock provider.",
    }
}

/// The label of a field.
pub fn label(field: Field) -> &'static str {
    match field {
        Field::Name => "Name",
        Field::Url | Field::ManagedUrl => "Base URL",
        Field::Variable => "API key variable",
        Field::ExternalAuth => "External authentication",
        Field::Profile => "AWS profile",
        Field::Region => "AWS region",
        Field::Project => "Project ID",
        Field::Location => "Location",
        Field::Credentials => "Credential file (server)",
    }
}

/// What the field is called in the details when the server reported no value.
fn absent(field: Field) -> &'static str {
    match field {
        Field::Url | Field::ManagedUrl => "Not reported",
        Field::ExternalAuth => "Managed externally",
        Field::Variable => "None",
        _ => "Not set",
    }
}

/// What a field's text shows while it is empty (a hint of the shape of a value).
pub fn example(field: Field) -> &'static str {
    match field {
        Field::Name => "Team gateway",
        Field::Url => "https://gateway.example/v1",
        Field::Variable => "TEAM_API_KEY",
        _ => "",
    }
}

/// The document of the details pager (`details`): a bold name, a blank line, then `label  value`
/// rows with a bold label. The pager reads ANSI escapes.
pub fn details(provider: &ModelProviderSummary) -> String {
    const BOLD: &str = "\x1b[1m";
    const PLAIN: &str = "\x1b[0m";
    let mut lines = vec![format!("{BOLD}{}{PLAIN}", provider.name), String::new()];
    let mut field = |label: &str, value: &str| {
        lines.push(format!("{BOLD}{label}  {PLAIN}{value}"));
    };
    field("Provider ID", &provider.id);
    if matches!(provider.settings, ProviderSettings::OpenAi) {
        field("Type", "Built-in OpenAI provider");
    }
    for (kind, value) in provider_fields(provider) {
        field(label(kind), value.unwrap_or(absent(kind)));
    }
    lines.join("\n")
}

/// `enter details · space set default · esc back`: `accept` and `cancel` are the keys the keymap
/// names (an unbound action is left out).
pub fn provider_hint(accept: Option<&str>, select: bool, cancel: Option<&str>) -> String {
    [
        accept.map(|key| format!("{key} details")),
        select.then(|| "space set default".to_owned()),
        cancel.map(|key| format!("{key} back")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

/// `enter select · esc back` (the standard footer of a list).
pub fn picker_hint(accept: Option<&str>, cancel: Option<&str>) -> String {
    [
        accept.map(|key| format!("{key} select")),
        cancel.map(|key| format!("{key} back")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

/// The keys the list's footers name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    pub accept: Option<String>,
    pub cancel: Option<String>,
}

/// The list of the providers the server knows (`list`): `current` is the provider of the chat
/// the user is in, `client_override` what the launch settings select for new chats.
pub fn list(
    page: &Page,
    current: Option<&str>,
    client_override: Option<&str>,
    keys: &Keys,
) -> List {
    let list = &page.list;
    let current_label = current.unwrap_or("No active chat");
    let can_select = list.can_change_default && client_override.is_none();
    let default_label = if client_override.is_some() {
        "Server default"
    } else {
        "Default for new chats"
    };
    let note = match client_override {
        Some(id) => format!("This client's launch settings select {id} for new chats."),
        None if !list.can_change_default => {
            "The server configuration fixes the default provider.".to_owned()
        }
        None => "Use /new to start a chat with the selected provider.".to_owned(),
    };
    let (accept, cancel) = (keys.accept.as_deref(), keys.cancel.as_deref());
    let mut items: Vec<Item> = list
        .data
        .iter()
        .enumerate()
        .map(|(index, provider)| {
            let is_default = provider.id == list.default_provider_id;
            Item {
                id: format!("{PROVIDER}{index}"),
                name: provider.name.clone(),
                description: Some(provider.id.clone()),
                current: current == Some(provider.id.as_str()),
                prefix_spans: vec![if is_default {
                    Span::new("● ", Role::Good)
                } else {
                    Span::new("○ ", Role::Dim)
                }],
                secondary: (can_select && !is_default).then(|| Secondary {
                    key: ' ',
                    hint: provider_hint(accept, true, cancel),
                    accept: None,
                }),
                ..Item::default()
            }
        })
        .collect();
    items.push(Item {
        id: CREATE.to_owned(),
        name: CREATE_CUSTOM_PROVIDER.to_owned(),
        footer_hint: Some(picker_hint(accept, cancel)),
        ..Item::default()
    });
    List {
        title: Some(TITLE.to_owned()),
        subtitle: Some(format!(
            "Current chat: {current_label} · {default_label}: {}",
            list.default_provider_id
        )),
        footer_note: Some(page.notice.as_ref().map_or(note, outcome)),
        footer_hint: Some(provider_hint(accept, false, cancel)),
        items,
        columns: Columns::AutoVisible,
        panel: true,
        ..List::default()
    }
}
