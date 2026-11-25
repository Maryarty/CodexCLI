//! The fields of a provider the server reports and the draft of an edit (oracle
//! `app/provider/fields.rs`, `provider/editor/form.rs`): which fields a provider has, which
//! are required, what a form with these changes asks the server to write, and why it may not.
//! The text typed is the view's (ADR-013); the draft keeps what the server reported and
//! reads the changes handed to it, so it never holds a cursor or a half-typed value.

use std::collections::{BTreeMap, BTreeSet};

use codex_app_server_protocol::{
    BedrockProviderId, BedrockSettingsParams, CustomGatewaySettingsParams, GatewayAuth,
    GcpProviderId, GcpSettingsParams, ModelProviderConfigWriteParams, ModelProviderSummary,
    ProviderSettings, ProviderSettingsPatch,
};

const AMAZON_BEDROCK_PROVIDER_ID: &str = "amazon-bedrock";
const AMAZON_BEDROCK_RUNTIME_PROVIDER_ID: &str = "amazon-bedrock-runtime";

/// A field a provider reports.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    Name,
    Url,
    /// The endpoint the server manages: shown, never edited.
    ManagedUrl,
    Variable,
    ExternalAuth,
    Profile,
    Region,
    Project,
    Location,
    Credentials,
}

/// The fields of `provider` as the server reports them, each with its value (none: not set).
pub fn provider_fields(provider: &ModelProviderSummary) -> Vec<(Field, Option<&str>)> {
    let url = provider.base_url.as_deref();
    let mut fields = match &provider.settings {
        ProviderSettings::OpenAi => Vec::new(),
        ProviderSettings::CustomGateway { auth } => {
            let auth_field = match auth {
                GatewayAuth::None => (Field::Variable, None),
                GatewayAuth::EnvironmentVariable { env_key } => {
                    (Field::Variable, Some(env_key.as_str()))
                }
                GatewayAuth::ExternallyConfigured { env_key } => {
                    (Field::ExternalAuth, env_key.as_deref())
                }
            };
            let endpoint = if auth_field.0 == Field::ExternalAuth && url.is_none() {
                Field::ManagedUrl
            } else {
                Field::Url
            };
            return vec![
                (Field::Name, Some(provider.name.as_str())),
                (endpoint, url),
                auth_field,
            ];
        }
        ProviderSettings::Bedrock { profile, region } => vec![
            (
                Field::Profile,
                profile.as_deref().filter(|p| !p.trim().is_empty()),
            ),
            (Field::Region, region.as_deref()),
        ],
        ProviderSettings::Gcp {
            project_id,
            location,
            credential_file,
        } => vec![
            (Field::Project, project_id.as_deref()),
            (Field::Location, location.as_deref()),
            (Field::Credentials, credential_file.as_deref()),
        ],
    };
    if let Some(url) = url {
        fields.push((Field::ManagedUrl, Some(url)));
    }
    fields
}

/// Why a form cannot be saved yet (the words are the view's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    ReadOnly,
    EnterName,
    EnterUrl,
    KeyNeedsUrl,
    BadVariable,
    UnknownBedrock,
}

/// What an edit starts from: the provider being edited (none: a new gateway) and the variable
/// names the catalog already uses, which the key field completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    provider: Option<ModelProviderSummary>,
    variables: BTreeSet<String>,
}

impl Draft {
    /// An edit of `provider` (none: a new custom gateway) among the providers of `catalog`.
    pub fn new(provider: Option<ModelProviderSummary>, catalog: &[ModelProviderSummary]) -> Draft {
        let variables = catalog
            .iter()
            .flat_map(provider_fields)
            .filter(|(field, _)| matches!(field, Field::Variable | Field::ExternalAuth))
            .filter_map(|(_, name)| name.map(str::to_owned))
            .collect();
        Draft {
            provider,
            variables,
        }
    }

    /// The provider being edited.
    pub fn provider(&self) -> Option<&ModelProviderSummary> {
        self.provider.as_ref()
    }

    /// The fields the form shows, with what the server reported for each.
    pub fn fields(&self) -> Vec<(Field, Option<&str>)> {
        let Some(p) = &self.provider else {
            return [Field::Name, Field::Url, Field::Variable]
                .map(|field| (field, None))
                .into();
        };
        provider_fields(p)
            .into_iter()
            .filter(|(field, value)| match field {
                // With no reported profile, preserve possible credential-export authentication.
                Field::Profile => value.is_some(),
                Field::ManagedUrl | Field::ExternalAuth => false,
                _ => true,
            })
            .collect()
    }

    /// The text a field starts with.
    pub fn initial(original: Option<&str>) -> String {
        // A single-line field strips line breaks from legacy values on display.
        original.unwrap_or_default().replace(['\r', '\n'], "")
    }

    /// Whether the form marks `field` as required, given the changes typed: the URL becomes
    /// required once an API key variable is typed for a provider that did not need one.
    pub fn marks_required(&self, field: Field, changes: &BTreeMap<Field, String>) -> bool {
        self.required(field)
            || (field == Field::Url
                && changes
                    .get(&Field::Variable)
                    .is_some_and(|value| !value.is_empty()))
    }

    fn required(&self, field: Field) -> bool {
        field == Field::Name
            || (field == Field::Url && self.provider.as_ref().is_none_or(|p| p.base_url.is_some()))
    }

    /// The changes among `values` (one per field of `fields`, in order): what differs from the
    /// field's start, trimmed.
    pub fn changes<S: AsRef<str>>(&self, values: &[S]) -> BTreeMap<Field, String> {
        self.fields()
            .into_iter()
            .zip(values)
            .filter_map(|((field, original), value)| {
                let value = value.as_ref().trim().to_owned();
                (value != Self::initial(original).trim()).then_some((field, value))
            })
            .collect()
    }

    /// The variable name that completes `text` (the one a provider of the catalog uses).
    pub fn suggestion(&self, text: &str) -> Option<&str> {
        if text.is_empty() {
            return None;
        }
        self.variables
            .iter()
            .map(String::as_str)
            .find(|name| *name != text && name.starts_with(text))
    }

    fn value<'a>(
        changes: &'a BTreeMap<Field, String>,
        field: Field,
        original: Option<&'a str>,
    ) -> &'a str {
        changes
            .get(&field)
            .map_or(original.unwrap_or_default(), String::as_str)
    }

    /// What the server is asked to write for `changes`, or why it cannot be asked.
    pub fn patch(
        &self,
        changes: &BTreeMap<Field, String>,
    ) -> Result<ModelProviderConfigWriteParams, Invalid> {
        if self.provider.as_ref().is_some_and(|p| !p.can_edit) {
            return Err(Invalid::ReadOnly);
        }
        for (field, original) in self.fields() {
            if self.required(field) && Self::value(changes, field, original).trim().is_empty() {
                return Err(if field == Field::Name {
                    Invalid::EnterName
                } else {
                    Invalid::EnterUrl
                });
            }
        }
        if let Some(key) = changes.get(&Field::Variable)
            && !key.is_empty()
        {
            let original_url = self.provider.as_ref().and_then(|p| p.base_url.as_deref());
            if Self::value(changes, Field::Url, original_url)
                .trim()
                .is_empty()
            {
                return Err(Invalid::KeyNeedsUrl);
            }
            if !(key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
            {
                return Err(Invalid::BadVariable);
            }
        }
        let changed = |field| {
            changes
                .get(&field)
                .map(|s| (!s.is_empty()).then(|| s.clone()))
        };
        let provider = self.provider.as_ref();
        let settings = match provider.map(|p| &p.settings) {
            None | Some(ProviderSettings::CustomGateway { .. }) => {
                ProviderSettingsPatch::CustomGateway(CustomGatewaySettingsParams {
                    provider_id: provider.map(|p| p.id.clone()),
                    name: changes.get(&Field::Name).cloned(),
                    base_url: changed(Field::Url),
                    env_key: changed(Field::Variable),
                })
            }
            Some(ProviderSettings::Bedrock { .. }) => {
                ProviderSettingsPatch::Bedrock(BedrockSettingsParams {
                    provider_id: match provider.map(|p| p.id.as_str()) {
                        Some(AMAZON_BEDROCK_PROVIDER_ID) => BedrockProviderId::AmazonBedrock,
                        Some(AMAZON_BEDROCK_RUNTIME_PROVIDER_ID) => {
                            BedrockProviderId::AmazonBedrockRuntime
                        }
                        _ => return Err(Invalid::UnknownBedrock),
                    },
                    profile: changed(Field::Profile),
                    region: changed(Field::Region),
                })
            }
            Some(ProviderSettings::Gcp { .. }) => ProviderSettingsPatch::Gcp(GcpSettingsParams {
                provider_id: GcpProviderId::Gcp,
                project_id: changed(Field::Project),
                location: changed(Field::Location),
                credential_file: changed(Field::Credentials),
            }),
            Some(ProviderSettings::OpenAi) => return Err(Invalid::ReadOnly),
        };
        Ok(ModelProviderConfigWriteParams { settings })
    }
}

/// Whether `params` creates a custom gateway (the write whose lost answer may follow a
/// committed save).
pub fn creates(params: &ModelProviderConfigWriteParams) -> bool {
    matches!(
        &params.settings,
        ProviderSettingsPatch::CustomGateway(gateway) if gateway.provider_id.is_none()
    )
}
