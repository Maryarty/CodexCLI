//! The host's side of the security-setup reminder (oracle `security_setup.rs` `prefetch`): the
//! server's notice for the signed-in account, read once at start and again whenever the account is
//! announced. Nothing blocks on it, nothing is reported of it, and every doubt is no notice: the
//! provider must be OpenAI, the workspace local, the notice valid (an `https://chatgpt.com` page
//! without credentials or port) and the identity it names the one the auth holds after the fetch.
//! The feature gets the notice as a fact (`features::security_setup`).

use std::time::Duration;

use codex_app_server_protocol::{ClientRequest, GetAccountSecuritySetupResponse};
use codex_core::config::Config;
use codex_tui_core::app::AppEv;
use codex_tui_core::features::security_setup::{Action, Fact, Identity, Notice};
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

/// The fact for the notice the server holds, or none.
pub(crate) async fn fetch<F, Fut>(config: &Config, ids: &Ids, send: F) -> Option<AppEv>
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
        let identity = Identity {
            account: fetched.account_id,
            user: fetched.user_id,
        };
        let notice = Notice {
            title: fetched.title,
            description: fetched.description,
            action: Action {
                label: fetched.action.label,
                url: fetched.action.url,
            },
        };
        if !notice.text_valid() || !address_valid(&notice.action.url) {
            return None;
        }
        let current = config
            .auth_config()
            .load_auth(/*enable_codex_api_key_env*/ false)
            .await
            .ok()
            .flatten()?;
        let (account, user) = (current.get_account_id()?, current.get_chatgpt_user_id()?);
        (account == identity.account && user == identity.user).then_some((identity, notice))
    };
    let (identity, notice) = tokio::time::timeout(WITHIN, asked).await.ok().flatten()?;
    Some(AppEv::Feature(Fact::Loaded { identity, notice }.into()))
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
