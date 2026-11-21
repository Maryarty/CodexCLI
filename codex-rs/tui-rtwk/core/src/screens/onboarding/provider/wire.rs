//! The provider wizards' protocol mapping: the request each `Setup` sends
//! and the reading of its answer (the host does the IO, this decides).
//! Oracle: `onboarding/gcp.rs`, `bedrock.rs` (`start_bedrock_discovery`,
//! `start_bedrock_setup`, `check_gov_cloud`), `provider_setup.rs`
//! (`spawn_provider_setup`).

use codex_app_server_protocol::AwsCredentialType;
use codex_app_server_protocol::BedrockCheckGovCloudRequirementsParams;
use codex_app_server_protocol::BedrockCheckGovCloudRequirementsResponse;
use codex_app_server_protocol::BedrockDiscoverParams;
use codex_app_server_protocol::BedrockDiscoverResponse;
use codex_app_server_protocol::BedrockSetupParams;
use codex_app_server_protocol::BedrockSetupResponse;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GcpSetupParams;
use codex_app_server_protocol::GcpSetupResponse;
use codex_app_server_protocol::LoginAccountParams;
use codex_app_server_protocol::LoginAccountResponse;
use codex_app_server_protocol::RequestId;
use serde_json::Value;

use super::bedrock::{Credential, CredentialKind, Discovery, EnvCredential, Profile};
use super::{Answer, Outcome, Setup};

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

fn read<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|err| err.to_string())
}

/// What a setup's answer means; `Err` is the message it failed with.
pub fn outcome(setup: &Setup, result: Result<Value, String>) -> Outcome {
    let value = result?;
    match setup {
        Setup::Gcp(_) => read::<GcpSetupResponse>(value).map(|_| Answer::Written),
        Setup::BedrockDiscover => {
            let response = read::<BedrockDiscoverResponse>(value)?;
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
                match read::<LoginAccountResponse>(value)? {
                    LoginAccountResponse::AmazonBedrock {} => Ok(Answer::Written),
                    response => Err(format!(
                        "Unexpected account/login/start response: {response:?}"
                    )),
                }
            }
            Credential::Profile(_) | Credential::Environment => {
                read::<BedrockSetupResponse>(value).map(|_| Answer::Written)
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
}
