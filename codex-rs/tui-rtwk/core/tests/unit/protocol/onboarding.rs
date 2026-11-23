use codex_app_server_protocol::Account;
use codex_app_server_protocol::CancelLoginAccountParams;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::ConfigReadParams;
use codex_app_server_protocol::GatewayOAuthLoginParams;
use codex_app_server_protocol::GatewayOAuthLoginResponse;
use codex_app_server_protocol::GatewayOAuthReadParams;
use codex_app_server_protocol::GatewayOAuthReadResponse;
use codex_app_server_protocol::GatewayOAuthStatus;
use codex_app_server_protocol::GetAccountParams;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::LoginAccountParams;
use codex_app_server_protocol::LoginAccountResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;
use codex_tui_core::protocol::onboarding::*;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::onboarding::auth::DeviceCode;
use codex_tui_core::screens::onboarding::auth::Error;
use codex_tui_core::screens::onboarding::auth::Login;
use codex_tui_core::screens::onboarding::auth::Outcome;
use codex_tui_core::screens::onboarding::auth::Providers;
use serde_json::Value;
use serde_json::json;

fn refusal(code: i64, message: &str) -> ServerError {
    ServerError {
        code: Some(code),
        data: None,
        message: message.into(),
    }
}

#[test]
fn a_gateway_read_the_server_does_not_know_is_missing() {
    assert!(gateway_read_missing(&refusal(
        -32601,
        "account/gatewayOAuth/read failed: Method not found (code -32601)"
    )));
    assert!(gateway_read_missing(&refusal(
        -32600,
        "account/gatewayOAuth/read failed: Invalid request: unknown variant `account/gatewayOAuth/read`, expected one of `initialize` (code -32600)"
    )));
    assert!(gateway_read_missing(&refusal(
        -32600,
        "account/gatewayOAuth/read failed: unknown method account/gatewayOAuth/read (code -32600)"
    )));
}

#[test]
fn another_refusal_is_not_a_missing_method() {
    // The method named in the client's frame is not the server's.
    assert!(!gateway_read_missing(&refusal(
        -32600,
        "account/gatewayOAuth/read failed: Invalid request: missing field `x` (code -32600)"
    )));
    assert!(!gateway_read_missing(&refusal(
        -32600,
        "account/gatewayOAuth/read failed: unknown variant `other/method` (code -32600)"
    )));
    assert!(!gateway_read_missing(&refusal(
        -32603,
        "account/gatewayOAuth/read failed: boom (code -32603)"
    )));
    assert!(gateway_named_read_refused(&refusal(-32600, "x")));
    assert!(!gateway_named_read_refused(&refusal(-32603, "x")));
}

#[test]
fn the_device_login_asks_for_the_code_and_reads_it_back() {
    let ClientRequest::GatewayOAuthLogin { params, .. } =
        gateway_device_login("office", RequestId::Integer(1))
    else {
        panic!("a login");
    };
    let params = params.unwrap();
    assert!(params.device_auth);
    assert_eq!(params.model_provider.as_deref(), Some("office"));
    let prompt =
        json!({"prompt": {"loginId": "l1", "verificationUrl": "https://v", "userCode": "AB-CD"}});
    assert_eq!(
        gateway_device_prompt(prompt),
        Ok(Some(DeviceCode {
            login_id: "l1".into(),
            url: "https://v".into(),
            code: "AB-CD".into(),
        }))
    );
    assert_eq!(gateway_device_prompt(json!({"prompt": null})), Ok(None));
    assert!(gateway_device_prompt(json!({"prompt": 3})).is_err());
}

#[test]
fn only_a_server_naming_a_provider_without_device_support_is_older() {
    assert!(gateway_server_without_device(&json!({"providerId": "x"})));
    assert!(!gateway_server_without_device(
        &json!({"providerId": "x", "supportsDeviceAuth": false})
    ));
    assert!(!gateway_server_without_device(&json!({})));
    assert!(!gateway_server_without_device(&json!("x")));
}

fn roots(projectless: bool) -> LocalRoots {
    LocalRoots {
        project_root: vec!["/repo".into()],
        git_root: Some(vec!["/repo".into()]),
        projectless,
    }
}

#[test]
fn a_trusted_folder_is_not_asked() {
    let response =
        json!({"config": {"projects": {"/repo": {"trust_level": "trusted"}}}, "layers": []});
    let keys = ["/repo/sub".to_owned()];
    assert_eq!(
        trust_lookup(&response, "/repo/sub", &keys, Some(&roots(false)), false),
        None
    );
}

#[test]
fn an_unknown_folder_in_a_repository_asks_for_the_root() {
    let response = json!({"config": {"projects": {}}, "layers": []});
    let keys = ["/repo/sub".to_owned()];
    assert_eq!(
        trust_lookup(&response, "/repo/sub", &keys, Some(&roots(false)), false),
        Some(Lookup {
            restricted: false,
            cwd: "/repo/sub".into(),
            target: "/repo".into(),
        })
    );
}

#[test]
fn a_projectless_folder_is_not_asked() {
    let response = json!({"config": {}, "layers": []});
    let keys = ["/tmp/x".to_owned()];
    assert_eq!(
        trust_lookup(&response, "/tmp/x", &keys, Some(&roots(true)), false),
        None
    );
}

#[test]
fn an_untrusted_folder_opens_restricted() {
    let response =
        json!({"config": {"projects": {"/repo": {"trust_level": "untrusted"}}}, "layers": []});
    let keys = ["/repo".to_owned()];
    assert_eq!(
        trust_lookup(&response, "/repo", &keys, Some(&roots(false)), false),
        Some(Lookup {
            restricted: true,
            cwd: "/repo".into(),
            target: "/repo".into(),
        })
    );
}

#[test]
fn a_loaded_project_layer_is_trusted() {
    let response = json!({"config": {}, "layers": [{"name": {"type": "project"}}]});
    let keys = ["/repo".to_owned()];
    assert_eq!(
        trust_lookup(&response, "/repo", &keys, Some(&roots(false)), false),
        None
    );
}

#[test]
fn the_trust_write_quotes_the_key() {
    let ClientRequest::ConfigBatchWrite { params, .. } =
        trust_write("/a\"b", RequestId::Integer(1))
    else {
        panic!("a batch write");
    };
    assert_eq!(params.edits[0].key_path, "projects.\"/a\\\"b\".trust_level");
    assert_eq!(params.edits[0].value, json!("trusted"));
    assert!(params.reload_user_config);
}

#[test]
fn sign_in_is_due_only_without_an_account_where_auth_is_required() {
    let due = |value| sign_in_due(value).expect("an account");
    assert!(due(json!({"account": null, "requiresOpenaiAuth": true})));
    assert!(!due(json!({"account": null, "requiresOpenaiAuth": false})));
    assert!(!due(
        json!({"account": {"type": "apiKey"}, "requiresOpenaiAuth": true})
    ));
}

#[test]
fn the_sign_in_offer_weighs_the_login_policy() {
    let none = json!({"account": null, "requiresOpenaiAuth": true});
    let wizards = Providers {
        bedrock: true,
        gcp: false,
        gateway: true,
    };
    let policy = |api_key| LoginPolicy {
        chatgpt: true,
        api_key,
        wizards,
        env_api_key: Some("sk-env".into()),
    };
    let offer = sign_in_offer(none.clone(), policy(true))
        .expect("an account")
        .expect("due");
    assert_eq!(offer.providers, wizards);
    assert_eq!(offer.env_api_key.as_deref(), Some("sk-env"));
    // No API login: no wizard (`is_login_method_allowed(Api)`).
    let offer = sign_in_offer(none, policy(false))
        .expect("an account")
        .expect("due");
    assert_eq!(offer.providers, Providers::default());
    assert!(!offer.api_key);
    let signed_in = json!({"account": {"type": "apiKey"}, "requiresOpenaiAuth": true});
    assert!(
        sign_in_offer(signed_in, policy(true))
            .expect("an account")
            .is_none()
    );
}

#[test]
fn login_requests_and_their_answers() {
    let ClientRequest::LoginAccount { params, .. } =
        login_start(&Login::ApiKey("sk".into()), RequestId::Integer(1))
    else {
        panic!("a login");
    };
    assert_eq!(
        params,
        LoginAccountParams::ApiKey {
            api_key: "sk".into()
        }
    );
    assert_eq!(
        login_outcome(
            &Login::ChatGpt,
            Ok(json!({"type": "chatgpt", "loginId": "l", "authUrl": "https://a"}))
        ),
        Outcome::Browser {
            login_id: "l".into(),
            url: "https://a".into()
        }
    );
    assert_eq!(
        login_outcome(
            &Login::DeviceCode,
            Ok(
                json!({"type": "chatgptDeviceCode", "loginId": "l", "verificationUrl": "https://d", "userCode": "AB-CD"})
            )
        ),
        Outcome::DeviceCode(DeviceCode {
            login_id: "l".into(),
            url: "https://d".into(),
            code: "AB-CD".into()
        })
    );
    assert_eq!(
        login_outcome(&Login::ApiKey("sk".into()), Ok(json!({"type": "apiKey"}))),
        Outcome::ApiKeySaved
    );
    assert_eq!(
        login_outcome(&Login::ApiKey("sk".into()), Err("boom".into())),
        Outcome::Failed(Error::SaveFailed("boom".into()))
    );
    assert_eq!(
        login_outcome(&Login::ChatGpt, Ok(json!({"type": "apiKey"}))),
        Outcome::Failed(Error::Unexpected("ApiKey".into()))
    );
}

#[test]
fn the_update_offer_follows_the_cache() {
    let cache = |latest: &str, dismissed: Option<&str>| VersionCache {
        latest: latest.into(),
        dismissed: dismissed.map(str::to_owned),
    };
    let npm = || Some("npm install -g @openai/codex".to_owned());
    assert_eq!(
        update_offer("1.0.0", Some(&cache("1.1.0", None)), npm()).map(|o| o.latest),
        Some("1.1.0".to_owned())
    );
    assert_eq!(
        update_offer("0.0.0", Some(&cache("1.1.0", None)), npm()),
        None
    );
    assert_eq!(
        update_offer("1.1.0", Some(&cache("1.1.0", None)), npm()),
        None
    );
    assert_eq!(
        update_offer("1.0.0", Some(&cache("1.1.0", Some("1.1.0"))), npm()),
        None
    );
    assert_eq!(
        update_offer("1.0.0", Some(&cache("1.1.0", None)), None),
        None
    );
    assert_eq!(update_offer("1.0.0", None, npm()), None);
}
