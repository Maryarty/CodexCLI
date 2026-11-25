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
use serde_json::Value;

use crate::rpc::{RpcError, ServerError};
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
pub fn login_outcome(login: &Login, result: Result<Value, ServerError>) -> Outcome {
    if let Login::Provider(setup) = login {
        return Outcome::Provider(provider::wire::outcome(setup, result));
    }
    let result = result.map_err(|error| error.message);
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

/// `account/gatewayOAuth/read` for the provider (`start_gateway_oauth`).
pub fn gateway_oauth_read(provider_id: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::GatewayOAuthRead {
        request_id,
        params: Some(GatewayOAuthReadParams {
            model_provider: Some(provider_id.to_owned()),
        }),
    }
}

/// `account/gatewayOAuth/read` as `device_login::start_if_needed` makes it:
/// for the provider the launch names, or for none (the server's own).
pub fn gateway_oauth_read_of(provider_id: Option<&str>, request_id: RequestId) -> ClientRequest {
    ClientRequest::GatewayOAuthRead {
        request_id,
        params: provider_id.map(|id| GatewayOAuthReadParams {
            model_provider: Some(id.to_owned()),
        }),
    }
}

/// An answer of a server that has the gateway OAuth read but not the device
/// code (`start_if_needed`'s probe without a provider): it names the
/// provider and lacks `supportsDeviceAuth`, which servers with the device
/// login always include.
pub fn gateway_server_without_device(probe: &Value) -> bool {
    probe.as_object().is_some_and(|state| {
        state.contains_key("providerId") && !state.contains_key("supportsDeviceAuth")
    })
}

const GATEWAY_READ: &str = "account/gatewayOAuth/read";

/// The server's own words of a refusal: the client's text is
/// `<method> failed: <source.message> (code n)`.
fn gateway_read_source(error: &ServerError) -> &str {
    let text = error
        .message
        .strip_prefix(GATEWAY_READ)
        .and_then(|rest| rest.strip_prefix(" failed: "))
        .unwrap_or(&error.message);
    text.rsplit_once(" (code ")
        .map_or(text, |(source, _)| source)
}

/// `is_method_not_found` of `account/gatewayOAuth/read`: a server without the
/// gateway sign-in, which has nothing to show (older ones report an unknown
/// method as an invalid request, naming it).
pub fn gateway_read_missing(error: &ServerError) -> bool {
    error.code == Some(-32601)
        || (error.code == Some(-32600) && {
            let source = gateway_read_source(error);
            source.contains(GATEWAY_READ)
                && (source.contains("unknown variant") || source.contains("unknown method"))
        })
}

/// A read for a named provider the server refused as an invalid request
/// (`start_if_needed` then asks without a provider to recognise a server
/// that does not take the name). The oracle also requires that the refusal
/// carry no `data`; the client's text does not carry it.
pub fn gateway_named_read_refused(error: &ServerError) -> bool {
    error.code == Some(-32600)
}

/// `account/gatewayOAuth/login` asking for the device code of the provider
/// (`start_if_needed`).
pub fn gateway_device_login(provider_id: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::GatewayOAuthLogin {
        request_id,
        params: Some(GatewayOAuthLoginParams {
            device_auth: true,
            model_provider: Some(provider_id.to_owned()),
        }),
    }
}

/// The one-time code the login answered with, none when the server signed
/// in without one; the decoder's own text of the error (the host frames
/// it).
pub fn gateway_device_prompt(value: Value) -> Result<Option<DeviceCode>, String> {
    let response: GatewayOAuthLoginResponse =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    Ok(response.prompt.map(|prompt| DeviceCode {
        login_id: prompt.login_id,
        url: prompt.verification_url,
        code: prompt.user_code,
    }))
}

/// `account/gatewayOAuth/login` for the provider: the browser way (the
/// device code is the startup's own, `device_login::start_if_needed`).
pub fn gateway_oauth_login(provider_id: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::GatewayOAuthLogin {
        request_id,
        params: Some(GatewayOAuthLoginParams {
            model_provider: Some(provider_id.to_owned()),
            ..GatewayOAuthLoginParams::default()
        }),
    }
}

/// `account/gatewayOAuth/cancel` (`GatewayOAuthCancel`).
pub fn gateway_oauth_cancel(request_id: RequestId) -> ClientRequest {
    ClientRequest::GatewayOAuthCancel {
        request_id,
        params: None,
    }
}

/// Where the provider's sign-in stands, out of `account/gatewayOAuth/read`'s
/// result; the decoder's own text of the error (the host frames it).
pub fn gateway_oauth_state(value: Value) -> Result<GatewayOAuthReadResponse, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// `needs_device_login` but for the preference, which is asked of the
/// machine only when this holds: the provider uses OAuth, the server has a
/// device endpoint and the credential is missing or failed.
pub fn device_login_possible(state: &GatewayOAuthReadResponse) -> bool {
    state.required
        && state.supports_device_auth
        && matches!(
            state.status,
            Some(GatewayOAuthStatus::NotReady | GatewayOAuthStatus::Failed)
        )
}

/// Whether `start_gateway_oauth` asks the server to log in: the provider
/// uses OAuth, is not signed in yet, and the device code is not the way
/// (`device` is `needs_device_login`).
pub fn gateway_oauth_login_due(state: &GatewayOAuthReadResponse, device: bool) -> bool {
    state.required && state.status != Some(GatewayOAuthStatus::Succeeded) && !device
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
    if is_source_build_version(current) {
        return None;
    }
    let cache = cache?;
    let newer = is_newer(&cache.latest, current) == Some(true);
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

/// `update_versions.rs::is_newer`: whether `latest` is a later release than `current`; `None`
/// when either is not a plain `major.minor.patch` (a prerelease is not compared).
pub fn is_newer(latest: &str, current: &str) -> Option<bool> {
    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => Some(latest > current),
        _ => None,
    }
}

/// `update_versions.rs::is_source_build_version`: no release is compared against a source
/// build (`0.0.0`).
pub fn is_source_build_version(version: &str) -> bool {
    parse_version(version) == Some((0, 0, 0))
}

/// `update_versions.rs::parse_version`: the first three dot-separated numbers, the rest of the
/// text (a fourth component) is not read; surrounding whitespace is ignored.
pub fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major, minor, patch))
}
