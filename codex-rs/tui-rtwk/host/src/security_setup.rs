//! The host's side of the security-setup reminder (oracle `security_setup.rs` `prefetch`): the
//! server's notice for the signed-in account, read once at start and again whenever the account is
//! announced. Nothing blocks on it, nothing is reported of it, and every doubt is no notice: the
//! provider must be OpenAI, the workspace local, the notice valid (an `https://chatgpt.com` page
//! without credentials or port) and the identity it names the one the auth holds after the fetch.
//! The feature gets the notice as a fact (`features::security_setup`).

use std::time::Duration;

use codex_app_server_protocol::{ClientRequest, GetAccountSecuritySetupResponse};
use codex_core::config::Config;
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

/// How long the server may take (the oracle's budget).
const WITHIN: Duration = Duration::from_secs(3);

/// Whether this session may be reminded at all (`prefetch`'s first test).
pub(crate) fn eligible(config: &Config, remote_workspace: bool) -> bool {
    config.model_provider_id == "openai" && !remote_workspace
}

/// The address rule of the oracle's `Notice::valid`.
fn address_valid(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("chatgpt.com")
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
    })
}

/// A notice the server holds for the account, as the host checked it.
pub(crate) struct Fetched {
    pub account: String,
    pub user: String,
    pub title: String,
    pub description: String,
    pub label: String,
    pub url: String,
}

/// The notice the server holds, verified for the auth that is current now, or none (the feature
/// checks the text; the address and the identity are the host's).
pub(crate) async fn fetch<F, Fut>(config: &Config, ids: &Ids, send: F) -> Option<Fetched>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, ServerError>>,
{
    let asked = async {
        let request = ClientRequest::GetAccountSecuritySetup {
            request_id: ids.next(),
            params: None,
        };
        let value = send(request).await.ok()?;
        let response: GetAccountSecuritySetupResponse = serde_json::from_value(value).ok()?;
        let fetched = response.notice?;
        if !address_valid(&fetched.action.url) {
            return None;
        }
        let current = config
            .auth_config()
            .load_auth(/*enable_codex_api_key_env*/ false)
            .await
            .ok()
            .flatten()?;
        let (account, user) = (current.get_account_id()?, current.get_chatgpt_user_id()?);
        (account == fetched.account_id && user == fetched.user_id).then_some(Fetched {
            account,
            user,
            title: fetched.title,
            description: fetched.description,
            label: fetched.action.label,
            url: fetched.action.url,
        })
    };
    tokio::time::timeout(WITHIN, asked).await.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::address_valid;

    #[test]
    fn only_a_plain_chatgpt_page_is_a_notice_action() {
        assert!(address_valid("https://chatgpt.com/cyber"));
        for url in [
            "http://chatgpt.com/cyber",
            "https://chatgpt.com:8443/cyber",
            "https://user@chatgpt.com/cyber",
            "https://evil.example/cyber",
            "not a url",
        ] {
            assert!(!address_valid(url), "{url}");
        }
    }
}
