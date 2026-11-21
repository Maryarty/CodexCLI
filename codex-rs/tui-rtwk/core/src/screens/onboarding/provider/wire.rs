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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::onboarding::auth::provider::bedrock::Params;
    use serde_json::json;

    fn bedrock(credential: Credential) -> Setup {
        Setup::Bedrock(Params {
            credential,
            region: "us-west-2".into(),
        })
    }

    #[test]
    fn each_credential_goes_to_its_request() {
        let id = || RequestId::Integer(1);
        assert!(matches!(
            request(&bedrock(Credential::ApiKey("k".into())), id()),
            ClientRequest::LoginAccount {
                params: LoginAccountParams::AmazonBedrock { .. },
                ..
            }
        ));
        assert!(matches!(
            request(&bedrock(Credential::Profile("p".into())), id()),
            ClientRequest::BedrockSetup {
                params: BedrockSetupParams::Profile { .. },
                ..
            }
        ));
        assert!(matches!(
            request(&bedrock(Credential::Environment), id()),
            ClientRequest::BedrockSetup {
                params: BedrockSetupParams::Environment { .. },
                ..
            }
        ));
        assert!(matches!(
            request(&Setup::BedrockDiscover, id()),
            ClientRequest::BedrockDiscover { .. }
        ));
    }

    // The oracle's GCP setup and gateway write show `source.message`; the
    // list, the selection, Bedrock's and any other request show the client's
    // whole text (`rpc::request_error`, `set_default_provider`).
    #[test]
    fn only_the_gcp_setup_and_the_gateway_write_read_the_servers_message() {
        let id = || RequestId::Integer(1);
        let write = Setup::GatewayWrite(super::super::gateway::Patch {
            name: "Team".into(),
            url: "https://t/v1".into(),
            variable: None,
        });
        assert!(reads_message_only(&request(&write, id())));
        for setup in [
            Setup::GatewayList,
            Setup::GatewaySelect("office".into()),
            Setup::BedrockDiscover,
            bedrock(Credential::ApiKey("k".into())),
            bedrock(Credential::Profile("p".into())),
        ] {
            assert!(!reads_message_only(&request(&setup, id())), "{setup:?}");
        }
        assert!(!reads_message_only(&crate::protocol::onboarding::account_read(id())));
    }

    #[test]
    fn answers_are_read_by_their_setup() {
        let discovered = outcome(
            &Setup::BedrockDiscover,
            Ok(json!({
                "profiles": [{"name": "engineering", "region": "us-east-2"}],
                "environmentCredentials": [{"type": "bedrockApiKey", "region": null}],
            })),
        );
        assert_eq!(
            discovered,
            Ok(Answer::Discovered(Discovery {
                profiles: vec![Profile {
                    name: "engineering".into(),
                    region: Some("us-east-2".into()),
                }],
                environment: vec![EnvCredential {
                    kind: CredentialKind::BedrockApiKey,
                    region: None,
                }],
            }))
        );
        let key = bedrock(Credential::ApiKey("k".into()));
        assert_eq!(
            outcome(&key, Ok(json!({"type": "amazonBedrock"}))),
            Ok(Answer::Written)
        );
        assert!(
            outcome(&key, Ok(json!({"type": "apiKey"})))
                .is_err_and(|e| e.starts_with("Unexpected account/login/start response"))
        );
        assert_eq!(outcome(&key, Err("denied".into())), Err("denied".into()));
        assert!(is_gov_cloud(Ok(
            json!({"isGovCloud": true, "shouldWarn": false})
        )));
        assert!(!is_gov_cloud(Err("timeout".into())));
    }

    fn summary(id: &str, settings: Value) -> Value {
        json!({
            "id": id, "name": id, "canEdit": true,
            "baseUrl": "https://office.example.test/v1", "settings": settings,
        })
    }

    fn gateway_auth(mode: Value) -> Value {
        json!({"type": "customGateway", "auth": mode})
    }

    // gateway.rs `GatewayStep::load`
    #[test]
    fn the_list_keeps_the_custom_gateways_and_says_when_the_default_is_fixed() {
        let list = |can_change: bool| {
            json!({
                "data": [
                    summary("openai", json!({"type": "openai"})),
                    summary("a", gateway_auth(json!({"mode": "none"}))),
                    summary("b", gateway_auth(json!({"mode": "environmentVariable", "envKey": "B_KEY"}))),
                    summary("c", gateway_auth(json!({"mode": "externallyConfigured", "envKey": null}))),
                ],
                "defaultProviderId": "openai",
                "canChangeDefault": can_change,
            })
        };
        let Ok(Answer::Gateways(page)) = outcome(&Setup::GatewayList, Ok(list(true))) else {
            panic!("no page");
        };
        let ids: Vec<_> = page.gateways.iter().map(|g| (g.id.as_str(), g.variable.as_deref())).collect();
        assert_eq!(ids, [("a", None), ("b", Some("B_KEY")), ("c", None)]);
        assert_eq!(page.default_id, "openai");
        assert_eq!(outcome(&Setup::GatewayList, Ok(list(false))), Err(POLICY.into()));
        let old = ServerError {
            code: Some(-32601),
            message: "no such method".into(),
        };
        assert_eq!(
            outcome(&Setup::GatewayList, Err(old)),
            Err("Update the connected app server to browse model providers.".into())
        );
        assert_eq!(
            outcome(&Setup::GatewayList, Err("down".into())),
            Err("Could not load model providers: down".into())
        );
        // The client's frame stays: the refusal of the list is not the bare
        // server message.
        let refused = ServerError {
            code: Some(-32602),
            message: "modelProvider/list failed: Bad cursor (code -32602)".into(),
        };
        assert_eq!(
            outcome(&Setup::GatewayList, Err(refused)),
            Err("Could not load model providers: modelProvider/list failed: Bad cursor (code -32602)".into())
        );
        // An older server rejects the method as an unknown variant of the
        // request, under the method's real name.
        let unknown = ServerError {
            code: Some(-32600),
            message: "modelProvider/list failed: Invalid request: unknown variant `modelProvider/list`, expected one of `initialize` (code -32600)".into(),
        };
        assert_eq!(
            outcome(&Setup::GatewayList, Err(unknown)),
            Err("Update the connected app server to browse model providers.".into())
        );
    }

    // rpc.rs `write_provider`
    #[test]
    fn a_create_without_a_confirmed_reply_is_unconfirmed_a_refusal_is_not() {
        let setup = Setup::GatewayWrite(super::super::gateway::Patch {
            name: "Team".into(),
            url: "https://t/v1".into(),
            variable: None,
        });
        let saved = outcome(
            &setup,
            Ok(json!({"provider": summary("team", gateway_auth(json!({"mode": "none"})))})),
        );
        assert!(matches!(saved, Ok(Answer::GatewaySaved(s)) if s.id == "team"));
        let internal = ServerError {
            code: Some(-32603),
            message: "read-back failed".into(),
        };
        assert_eq!(outcome(&setup, Err(internal)), Ok(Answer::GatewayUnconfirmed));
        assert_eq!(outcome(&setup, Err("lost".into())), Ok(Answer::GatewayUnconfirmed));
        assert_eq!(outcome(&setup, Ok(json!({"x": 1}))), Ok(Answer::GatewayUnconfirmed));
        let refused = ServerError {
            code: Some(-32602),
            message: "Enter an HTTP(S) base URL.".into(),
        };
        assert_eq!(outcome(&setup, Err(refused)), Err("Enter an HTTP(S) base URL.".into()));
    }

    // rpc.rs `set_default_provider`, `override_reason`
    #[test]
    fn a_selection_that_a_higher_setting_overrides_is_a_refusal_with_its_reason() {
        let setup = Setup::GatewaySelect("team".into());
        let done = |status: &str, metadata: Value| {
            json!({"status": status, "version": "v1", "filePath": "/c.toml", "overriddenMetadata": metadata})
        };
        assert_eq!(
            outcome(&setup, Ok(done("ok", Value::Null))),
            Ok(Answer::GatewaySelected)
        );
        assert_eq!(
            outcome(&setup, Ok(done("okOverridden", Value::Null))),
            Err("A higher-priority setting overrides this choice.".into())
        );
        assert_eq!(
            outcome(
                &setup,
                Ok(done(
                    "okOverridden",
                    json!({"message": "Managed.", "overridingLayer": {"name": {"type": "system", "file": "/m"}, "version": "v"}, "effectiveValue": "x"})
                ))
            ),
            Err("Managed.".into())
        );
        // The selection is a refusal with the client's frame
        // (`set_default_provider`: `err.to_string()`).
        let request = request(&setup, RequestId::Integer(1));
        assert!(!reads_message_only(&request));
        assert!(matches!(
            request,
            ClientRequest::ConfigValueWrite { params, .. } if params.key_path == "model_provider"
        ));
    }
}
