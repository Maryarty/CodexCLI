//! The startup page's protocol mapping: the requests the host sends for the
//! page's facts and the trust write, and the pure reading of what came back
//! (the host does the IO, this decides). Oracle: `lib.rs`
//! (`get_login_status`, `should_show_login_screen`), `config_update.rs`
//! (`read_remote_project_trust`, `trusted_project_edit`,
//! `write_config_batch`), `updates.rs` (`get_upgrade_version_for_popup`) and
//! `update_versions.rs`.

use codex_app_server_protocol::Account;
use codex_app_server_protocol::CancelLoginAccountParams;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::ConfigReadParams;
use codex_app_server_protocol::GetAccountParams;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::LoginAccountParams;
use codex_app_server_protocol::LoginAccountResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;
use serde_json::Value;

use crate::rpc::RpcError;
use crate::screens::onboarding::UpdateOffer;
use crate::screens::onboarding::auth::provider;
use crate::screens::onboarding::auth::{DeviceCode, Error, Login, Offer, Outcome, Providers};

/// `account/read`: whether the sign-in step is due.
pub fn account_read(request_id: RequestId) -> ClientRequest {
    ClientRequest::GetAccount {
        request_id,
        params: GetAccountParams {
            refresh_token: false,
        },
    }
}

/// Whether the sign-in step is due, out of `account/read`'s result
/// (`should_show_login_screen`): the provider needs OpenAI auth and the
/// account is neither an API key nor ChatGPT.
pub fn sign_in_due(result: Value) -> Result<bool, RpcError> {
    let response: GetAccountResponse =
        serde_json::from_value(result).map_err(|error| RpcError::Malformed(error.to_string()))?;
    let signed_in = matches!(
        response.account,
        Some(Account::ApiKey {} | Account::Chatgpt { .. })
    );
    Ok(response.requires_openai_auth && !signed_in)
}

/// What the host read of the login policy and the provider wizards, for
/// [`sign_in_offer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoginPolicy {
    /// `AuthConfig::is_login_method_allowed(ForcedLoginMethod::Chatgpt)`.
    pub chatgpt: bool,
    /// `AuthConfig::is_login_method_allowed(ForcedLoginMethod::Api)`.
    pub api_key: bool,
    /// The wizards the configuration and the server allow, before the
    /// sign-in and the API login are weighed (`should_show_provider_setup_wizard`
    /// but its `should_show_login_screen` and API-login terms).
    pub wizards: Providers,
    /// `read_openai_api_key_from_env`.
    pub env_api_key: Option<String>,
}

/// The sign-in step's offer, out of `account/read`'s result: none when the
/// sign-in is not due ([`sign_in_due`]); the wizards only where API login
/// is allowed (`should_show_provider_setup_wizard`).
pub fn sign_in_offer(result: Value, policy: LoginPolicy) -> Result<Option<Offer>, RpcError> {
    if !sign_in_due(result)? {
        return Ok(None);
    }
    let wizards = if policy.api_key {
        policy.wizards
    } else {
        Providers::default()
    };
    Ok(Some(Offer {
        chatgpt: policy.chatgpt,
        api_key: policy.api_key,
        providers: wizards,
        env_api_key: policy.env_api_key,
    }))
}

/// `account/login/start` for a login (`start_chatgpt_login`,
/// `start_headless_chatgpt_login`, `save_api_key`).
pub fn login_start(login: &Login, request_id: RequestId) -> ClientRequest {
    let params = match login {
        // A provider wizard's setup (S03): its own mapping.
        Login::Provider(setup) => return provider::wire::request(setup, request_id),
        Login::ChatGpt => LoginAccountParams::Chatgpt {
            codex_streamlined_login: false,
            use_hosted_login_success_page: false,
            app_brand: None,
        },
        Login::DeviceCode => LoginAccountParams::ChatgptDeviceCode,
        Login::ApiKey(key) => LoginAccountParams::ApiKey {
            api_key: key.clone(),
        },
    };
    ClientRequest::LoginAccount { request_id, params }
}

/// `account/login/cancel` (`cancel_login_attempt`).
pub fn login_cancel(login_id: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::CancelLoginAccount {
        request_id,
        params: CancelLoginAccountParams {
            login_id: login_id.to_owned(),
        },
    }
}

/// What a login's `account/login/start` answer means; `Err` is the error
/// the request failed with, as the oracle shows it.
pub fn login_outcome(login: &Login, result: Result<Value, String>) -> Outcome {
    if let Login::Provider(setup) = login {
        return Outcome::Provider(provider::wire::outcome(setup, result));
    }
    let response = match result {
        Ok(value) => serde_json::from_value::<LoginAccountResponse>(value),
        Err(message) => {
            return Outcome::Failed(match login {
                Login::ApiKey(_) => Error::SaveFailed(message),
                Login::ChatGpt | Login::DeviceCode | Login::Provider(_) => Error::Server(message),
            });
        }
    };
    match (login, response) {
        (Login::ChatGpt, Ok(LoginAccountResponse::Chatgpt { login_id, auth_url })) => {
            Outcome::Browser {
                login_id,
                url: auth_url,
            }
        }
        (
            Login::DeviceCode,
            Ok(LoginAccountResponse::ChatgptDeviceCode {
                login_id,
                verification_url,
                user_code,
            }),
        ) => Outcome::DeviceCode(DeviceCode {
            login_id,
            url: verification_url,
            code: user_code,
        }),
        (Login::ApiKey(_), Ok(LoginAccountResponse::ApiKey {})) => Outcome::ApiKeySaved,
        (Login::ApiKey(_), Err(error)) => Outcome::Failed(Error::SaveFailed(error.to_string())),
        (_, Ok(other)) => Outcome::Failed(Error::Unexpected(format!("{other:?}"))),
        (_, Err(error)) => Outcome::Failed(Error::Server(error.to_string())),
    }
}

/// `config/read` with the layers for `cwd`: what the folder trust is read from.
pub fn trust_read(cwd: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::ConfigRead {
        request_id,
        params: ConfigReadParams {
            include_layers: true,
            cwd: Some(cwd.to_owned()),
        },
    }
}

/// The roots the host found on disk for a local folder, as trust keys
/// (`normalized_project_trust_keys`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalRoots {
    /// The project root's keys, or the folder's when it has no project root.
    pub project_root: Vec<String>,
    /// The repository root's keys (`resolve_root_git_project_for_trust`).
    pub git_root: Option<Vec<String>>,
    /// Neither the configured markers nor the default ones find a project.
    pub projectless: bool,
}

/// A folder whose trust the startup page asks about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lookup {
    /// The folder is saved as untrusted: it opens restricted.
    pub restricted: bool,
    pub cwd: String,
    /// The folder that receives the trust (a saved project, the repository
    /// root, or the folder itself).
    pub target: String,
}

/// The trust question for the local folder `cwd`, out of `config/read`'s
/// `response` (`read_remote_project_trust` for a local host): `None` when the
/// folder needs no question (trusted, projectless, or a project config
/// already loaded). `cwd_keys` are the folder's trust keys, `roots` what the
/// host found on disk, `windows` whether paths compare without case.
pub fn trust_lookup(
    response: &Value,
    cwd: &str,
    cwd_keys: &[String],
    roots: Option<&LocalRoots>,
    windows: bool,
) -> Option<Lookup> {
    let project_layers: Vec<&Value> = response
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|layer| layer["name"]["type"] == "project")
        .collect();
    let disabled_project = project_layers.iter().rev().find(|layer| {
        layer
            .get("disabledReason")
            .and_then(Value::as_str)
            .is_some()
    });
    let disabled_reason = disabled_project
        .and_then(|layer| layer.get("disabledReason"))
        .and_then(Value::as_str);
    let projects = response["config"]["projects"].as_object();
    let project_entry = |key: &str| {
        let projects = projects?;
        projects
            .get_key_value(key)
            .filter(|(_, project)| project["trust_level"].as_str().is_some())
            .or_else(|| {
                projects
                    .iter()
                    .filter(|(path, project)| {
                        windows
                            && path.eq_ignore_ascii_case(key)
                            && project["trust_level"].as_str().is_some()
                    })
                    .min_by_key(|(path, _)| *path)
            })
    };
    // The disk's roots count only where no project layer was found.
    let local = roots.filter(|_| project_layers.is_empty());
    let projectless = local.is_some_and(|roots| roots.projectless);
    let saved = |keys: &[String]| {
        keys.iter()
            .find_map(|key| project_entry(key).map(|(path, _)| path.as_str()))
    };
    let cwd_target = saved(cwd_keys);
    let target = cwd_target
        .or_else(|| {
            disabled_reason
                .and_then(|reason| reason.split_once(", add "))
                .and_then(|(_, reason)| reason.rsplit_once(" as a trusted project in "))
                .map(|(target, _)| target)
                .or_else(|| {
                    disabled_project
                        .and_then(|layer| layer["name"]["dotCodexFolder"].as_str())
                        .and_then(|path| {
                            path.strip_suffix("/.codex")
                                .or_else(|| path.strip_suffix("\\.codex"))
                        })
                })
        })
        .or_else(|| {
            let roots = local?;
            saved(&roots.project_root)
                .or_else(|| roots.git_root.as_deref().and_then(saved))
                .or_else(|| {
                    roots
                        .git_root
                        .as_ref()
                        .and_then(|keys| keys.first())
                        .map(String::as_str)
                })
                .or_else(|| roots.project_root.first().map(String::as_str))
        })
        .unwrap_or(cwd);
    let level = project_entry(target)
        .and_then(|(_, project)| project.get("trust_level"))
        .and_then(Value::as_str);
    let explicitly_untrusted = cwd_target.is_none()
        && disabled_reason.is_some_and(|reason| {
            projects.into_iter().flatten().any(|(path, project)| {
                project.get("trust_level").and_then(Value::as_str) == Some("untrusted")
                    && reason
                        .strip_prefix(path.as_str())
                        .is_some_and(|suffix| suffix.starts_with(" is marked as untrusted"))
            })
        });
    let unknown = !matches!(level, Some("trusted" | "untrusted"));
    if !explicitly_untrusted
        && (level == Some("trusted")
            || unknown && projectless
            || (unknown
                && disabled_project.is_none()
                && project_layers
                    .iter()
                    .any(|layer| layer.get("disabledReason").is_none())))
    {
        return None;
    }
    Some(Lookup {
        restricted: explicitly_untrusted || level == Some("untrusted"),
        cwd: cwd.to_owned(),
        target: target.to_owned(),
    })
}

/// `config/batchWrite` saving the project `key` as trusted
/// (`trusted_project_edit`, `write_config_batch`).
pub fn trust_write(key: &str, request_id: RequestId) -> ClientRequest {
    let key = key.replace('\\', "\\\\").replace('"', "\\\"");
    ClientRequest::ConfigBatchWrite {
        request_id,
        params: ConfigBatchWriteParams {
            edits: vec![ConfigEdit {
                key_path: ["projects.\"", &key, "\".trust_level"].concat(),
                value: Value::String("trusted".to_owned()),
                merge_strategy: MergeStrategy::Replace,
            }],
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// What the host read of the update cache (`version.json`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionCache {
    pub latest: String,
    pub dismissed: Option<String>,
}

/// The update to offer (`get_upgrade_version_for_popup`): a release newer
/// than `current` in the cache, not dismissed, with a `command` to run it.
/// A source build (`0.0.0`) is never offered one.
pub fn update_offer(
    current: &str,
    cache: Option<&VersionCache>,
    command: Option<String>,
) -> Option<UpdateOffer> {
    if parse_version(current) == Some((0, 0, 0)) {
        return None;
    }
    let cache = cache?;
    let newer = matches!(
        (parse_version(&cache.latest), parse_version(current)),
        (Some(latest), Some(current)) if latest > current
    );
    if !newer || cache.dismissed.as_deref() == Some(cache.latest.as_str()) {
        return None;
    }
    Some(UpdateOffer {
        current: current.to_owned(),
        latest: cache.latest.clone(),
        command: command?,
        choice: None,
    })
}

/// `update_versions.rs::parse_version`.
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
