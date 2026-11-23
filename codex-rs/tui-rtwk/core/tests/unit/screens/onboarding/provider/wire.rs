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
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::onboarding::auth::provider::Answer;
use codex_tui_core::screens::onboarding::auth::provider::Setup;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::Credential;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::CredentialKind;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::Discovery;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::EnvCredential;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::Params;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::Profile;
use codex_tui_core::screens::onboarding::auth::provider::gateway::POLICY;
use codex_tui_core::screens::onboarding::auth::provider::wire::*;
use serde_json::Value;
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
    let write = Setup::GatewayWrite(
        codex_tui_core::screens::onboarding::auth::provider::gateway::Patch {
            name: "Team".into(),
            url: "https://t/v1".into(),
            variable: None,
        },
    );
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
    assert!(!reads_message_only(
        &codex_tui_core::protocol::onboarding::account_read(id())
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
    let ids: Vec<_> = page
        .gateways
        .iter()
        .map(|g| (g.id.as_str(), g.variable.as_deref()))
        .collect();
    assert_eq!(ids, [("a", None), ("b", Some("B_KEY")), ("c", None)]);
    assert_eq!(page.default_id, "openai");
    assert_eq!(
        outcome(&Setup::GatewayList, Ok(list(false))),
        Err(POLICY.into())
    );
    let old = ServerError {
        code: Some(-32601),
        data: None,
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
        data: None,
        message: "modelProvider/list failed: Bad cursor (code -32602)".into(),
    };
    assert_eq!(
        outcome(&Setup::GatewayList, Err(refused)),
        Err(
            "Could not load model providers: modelProvider/list failed: Bad cursor (code -32602)"
                .into()
        )
    );
    // An older server rejects the method as an unknown variant of the
    // request, under the method's real name.
    let unknown = ServerError {
        code: Some(-32600),
        data: None,
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
    let setup = Setup::GatewayWrite(
        codex_tui_core::screens::onboarding::auth::provider::gateway::Patch {
            name: "Team".into(),
            url: "https://t/v1".into(),
            variable: None,
        },
    );
    let saved = outcome(
        &setup,
        Ok(json!({"provider": summary("team", gateway_auth(json!({"mode": "none"})))})),
    );
    assert!(matches!(saved, Ok(Answer::GatewaySaved(s)) if s.id == "team"));
    let internal = ServerError {
        code: Some(-32603),
        data: None,
        message: "read-back failed".into(),
    };
    assert_eq!(
        outcome(&setup, Err(internal)),
        Ok(Answer::GatewayUnconfirmed)
    );
    assert_eq!(
        outcome(&setup, Err("lost".into())),
        Ok(Answer::GatewayUnconfirmed)
    );
    assert_eq!(
        outcome(&setup, Ok(json!({"x": 1}))),
        Ok(Answer::GatewayUnconfirmed)
    );
    let refused = ServerError {
        code: Some(-32602),
        data: None,
        message: "Enter an HTTP(S) base URL.".into(),
    };
    assert_eq!(
        outcome(&setup, Err(refused)),
        Err("Enter an HTTP(S) base URL.".into())
    );
}

// rpc.rs `set_default_provider`, `override_reason`
#[test]
fn a_selection_that_a_higher_setting_overrides_is_a_refusal_with_its_reason() {
    let setup = Setup::GatewaySelect("team".into());
    let done = |status: &str, metadata: Value| json!({"status": status, "version": "v1", "filePath": "/c.toml", "overriddenMetadata": metadata});
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
