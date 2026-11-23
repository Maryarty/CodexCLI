//! The provider wizards' protocol mapping: the request each `Setup` sends
//! and the reading of its answer (the host does the IO, this decides).
//! Oracle: `onboarding/gcp.rs`, `bedrock.rs` (`start_bedrock_discovery`,
//! `start_bedrock_setup`, `check_gov_cloud`), `provider_setup.rs`
//! (`spawn_provider_setup`), `app/provider/rpc.rs` (the gateway wizard).

use codex_app_server_protocol::AwsCredentialType;
use codex_app_server_protocol::BedrockCheckGovCloudRequirementsParams;
use codex_app_server_protocol::BedrockCheckGovCloudRequirementsResponse;
use codex_app_server_protocol::BedrockDiscoverParams;
use codex_app_server_protocol::BedrockDiscoverResponse;
use codex_app_server_protocol::BedrockSetupParams;
use codex_app_server_protocol::BedrockSetupResponse;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigValueWriteParams;
use codex_app_server_protocol::ConfigWriteResponse;
use codex_app_server_protocol::CustomGatewaySettingsParams;
use codex_app_server_protocol::GatewayAuth;
use codex_app_server_protocol::GcpSetupParams;
use codex_app_server_protocol::GcpSetupResponse;
use codex_app_server_protocol::LoginAccountParams;
use codex_app_server_protocol::LoginAccountResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::ModelProviderConfigWriteParams;
use codex_app_server_protocol::ModelProviderConfigWriteResponse;
use codex_app_server_protocol::ModelProviderListParams;
use codex_app_server_protocol::ModelProviderListResponse;
use codex_app_server_protocol::ModelProviderSummary;
use codex_app_server_protocol::ProviderSettings;
use codex_app_server_protocol::ProviderSettingsPatch;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::WriteStatus;
use serde_json::Value;

use super::bedrock::{Credential, CredentialKind, Discovery, EnvCredential, Profile};
use super::gateway::{POLICY, Page, Saved};
use super::{Answer, Outcome, Setup};
use crate::rpc::ServerError;

/// The request of a wizard's setup.
pub fn request(setup: &Setup, request_id: RequestId) -> ClientRequest {
    match setup {
        // `GcpState`'s Enter on the credentials: the app server validates
        // and writes the configuration (`account/gcp/setup`).
        Setup::Gcp(gcp) => ClientRequest::GcpSetup {
            request_id,
            params: GcpSetupParams {
                project_id: gcp.project_id.clone(),
                location: gcp.location.clone(),
                credential_file: gcp.credential_file.clone(),
            },
        },
        Setup::BedrockDiscover => ClientRequest::BedrockDiscover {
            request_id,
            params: BedrockDiscoverParams {},
        },
        Setup::GatewayList => ClientRequest::ModelProviderList {
            request_id,
            params: ModelProviderListParams {},
        },
        // A new gateway: the fields written are the ones filled in.
        Setup::GatewayWrite(patch) => ClientRequest::ModelProviderConfigWrite {
            request_id,
            params: ModelProviderConfigWriteParams {
                settings: ProviderSettingsPatch::CustomGateway(CustomGatewaySettingsParams {
                    provider_id: None,
                    name: Some(patch.name.clone()),
                    base_url: Some(Some(patch.url.clone())),
                    env_key: patch.variable.clone().map(Some),
                }),
            },
        },
        // `rpc::set_default_provider`.
        Setup::GatewaySelect(id) => ClientRequest::ConfigValueWrite {
            request_id,
            params: ConfigValueWriteParams {
                key_path: "model_provider".into(),
                value: id.as_str().into(),
                merge_strategy: MergeStrategy::Replace,
                file_path: None,
                expected_version: None,
            },
        },
        Setup::Bedrock(params) => {
            let region = params.region.clone();
            match &params.credential {
                Credential::ApiKey(api_key) => ClientRequest::LoginAccount {
                    request_id,
                    params: LoginAccountParams::AmazonBedrock {
                        api_key: api_key.clone(),
                        region,
                    },
                },
                Credential::AccessKeys {
                    access_key_id,
                    secret_access_key,
                    session_token,
                } => ClientRequest::LoginAccount {
                    request_id,
                    params: LoginAccountParams::AmazonBedrockAccessKeys {
                        access_key_id: access_key_id.clone(),
                        secret_access_key: secret_access_key.clone(),
                        session_token: session_token.clone(),
                        region,
                    },
                },
                Credential::Profile(profile) => ClientRequest::BedrockSetup {
                    request_id,
                    params: BedrockSetupParams::Profile {
                        profile: profile.clone(),
                        region,
                    },
                },
                Credential::Environment => ClientRequest::BedrockSetup {
                    request_id,
                    params: BedrockSetupParams::Environment { region },
                },
            }
        }
    }
}

/// Whether a server's refusal of this request is shown as the server's own
/// message, without the client's `<method> failed: ... (code n)` frame: the
/// oracle reads `source.message` for the GCP setup (`gcp.rs`) and the gateway
/// write (`app/provider/rpc.rs` `write_provider`); the list, the selection
/// (`set_default_provider`), Bedrock's and every other request show the
/// client's text (`error.to_string()`).
pub fn reads_message_only(request: &ClientRequest) -> bool {
    matches!(
        request,
        ClientRequest::GcpSetup { .. } | ClientRequest::ModelProviderConfigWrite { .. }
    )
}

fn read<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|err| err.to_string())
}

/// A saved gateway as the wizard has it.
fn saved(provider: ModelProviderSummary) -> Saved {
    let variable = match provider.settings {
        ProviderSettings::CustomGateway { auth } => match auth {
            GatewayAuth::None => None,
            GatewayAuth::EnvironmentVariable { env_key } => Some(env_key),
            GatewayAuth::ExternallyConfigured { env_key } => env_key,
        },
        _ => None,
    };
    Saved {
        id: provider.id,
        name: provider.name,
        variable,
    }
}

/// What the list refusal and the unreadable list start with (the oracle's
/// words, named like `POLICY`: a message is a value of the answer).
const LIST_FAILED: &str = "Could not load model providers: ";

/// `rpc::request_error`.
fn list_failure(error: &ServerError) -> String {
    // The message is the client's text: `<method> failed: <source.message>
    // (code n)`.
    let unknown = error.code == Some(-32600)
        && error.message.starts_with(
            "modelProvider/list failed: Invalid request: unknown variant `modelProvider/list`",
        );
    if error.code == Some(-32601) || unknown {
        "Update the connected app server to browse model providers.".into()
    } else {
        format!("{LIST_FAILED}{}", error.message)
    }
}

/// `GatewayStep::load`: the custom gateways, when the default provider can
/// be changed.
fn gateways(result: Result<Value, ServerError>) -> Outcome {
    let value = result.map_err(|error| list_failure(&error))?;
    let page = serde_json::from_value::<ModelProviderListResponse>(value)
        .map_err(|error| format!("{LIST_FAILED}{error}"))?;
    if !page.can_change_default {
        return Err(POLICY.into());
    }
    Ok(Answer::Gateways(Page {
        gateways: page
            .data
            .into_iter()
            .filter(|provider| matches!(provider.settings, ProviderSettings::CustomGateway { .. }))
            .map(saved)
            .collect(),
        default_id: page.default_provider_id,
    }))
}

/// `rpc::write_provider` for a create: a lost reply or a failed read-back
/// can follow a committed write (a transport failure, a decode failure, an
/// internal error of the server), which is not an answer.
fn written(result: Result<Value, ServerError>) -> Outcome {
    match result {
        Ok(value) => Ok(
            match serde_json::from_value::<ModelProviderConfigWriteResponse>(value) {
                Ok(response) => Answer::GatewaySaved(saved(response.provider)),
                Err(_) => Answer::GatewayUnconfirmed,
            },
        ),
        Err(error) if error.code.is_none() || error.code == Some(-32603) => {
            Ok(Answer::GatewayUnconfirmed)
        }
        Err(error) => Err(error.message),
    }
}

/// `rpc::set_default_provider` and `override_reason`.
fn selected(result: Result<Value, ServerError>) -> Outcome {
    let value = result.map_err(|error| error.message)?;
    let response = read::<ConfigWriteResponse>(value)?;
    match response.status {
        WriteStatus::Ok => Ok(Answer::GatewaySelected),
        WriteStatus::OkOverridden => Err(response
            .overridden_metadata
            .map(|metadata| metadata.message)
            .unwrap_or_else(|| "A higher-priority setting overrides this choice.".into())),
    }
}

/// The result of a request as the message it failed with.
fn value(result: Result<Value, ServerError>) -> Result<Value, String> {
    result.map_err(|error| error.message)
}

/// What a setup's answer means; `Err` is the message it failed with.
pub fn outcome(setup: &Setup, result: Result<Value, ServerError>) -> Outcome {
    match setup {
        Setup::GatewayList => gateways(result),
        Setup::GatewayWrite(_) => written(result),
        Setup::GatewaySelect(_) => selected(result),
        Setup::Gcp(_) => read::<GcpSetupResponse>(value(result)?).map(|_| Answer::Written),
        Setup::BedrockDiscover => {
            let response = read::<BedrockDiscoverResponse>(value(result)?)?;
            Ok(Answer::Discovered(Discovery {
                profiles: response
                    .profiles
                    .into_iter()
                    .map(|profile| Profile {
                        name: profile.name,
                        region: profile.region,
                    })
                    .collect(),
                environment: response
                    .environment_credentials
                    .into_iter()
                    .map(|credential| EnvCredential {
                        kind: match credential.credential_type {
                            AwsCredentialType::AccessKeys => CredentialKind::AccessKeys,
                            AwsCredentialType::BedrockApiKey => CredentialKind::BedrockApiKey,
                        },
                        region: credential.region,
                    })
                    .collect(),
            }))
        }
        Setup::Bedrock(params) => match params.credential {
            Credential::ApiKey(_) | Credential::AccessKeys { .. } => {
                match read::<LoginAccountResponse>(value(result)?)? {
                    LoginAccountResponse::AmazonBedrock {} => Ok(Answer::Written),
                    response => Err(format!(
                        "Unexpected account/login/start response: {response:?}"
                    )),
                }
            }
            Credential::Profile(_) | Credential::Environment => {
                read::<BedrockSetupResponse>(value(result)?).map(|_| Answer::Written)
            }
        },
    }
}

/// `account/bedrock/checkGovCloudRequirements` (`check_gov_cloud`).
pub fn gov_cloud_check(request_id: RequestId) -> ClientRequest {
    ClientRequest::BedrockCheckGovCloudRequirements {
        request_id,
        params: BedrockCheckGovCloudRequirementsParams {},
    }
}

/// Whether the GovCloud guidance shows: the active Bedrock authentication
/// resolves to GovCloud; a failed check shows nothing.
pub fn is_gov_cloud(result: Result<Value, String>) -> bool {
    result
        .and_then(read::<BedrockCheckGovCloudRequirementsResponse>)
        .is_ok_and(|response| response.is_gov_cloud)
}
