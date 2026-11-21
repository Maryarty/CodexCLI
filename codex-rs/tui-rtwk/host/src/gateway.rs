//! The selected gateway's own sign-in, host side: the reads and the login
//! the oracle's `start_gateway_oauth` makes on the embedded server, and
//! whether a device code is the way (`device_login::prefer_device_login`).
//! The sequence the page runs (the server's start, the primary login, the
//! retry) is core's `screens::onboarding::gateway`; the requests and the
//! reading of their answers are `protocol::onboarding`.

use std::future::Future;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::protocol::onboarding as wire;
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

/// `start_gateway_oauth`: reads where the provider's sign-in stands and asks
/// the server to log in unless it is signed in already or a device code is
/// the way (the startup shows that one before it asks for models). The text
/// of the error is the client's, as the oracle shows it
/// (`err.to_string()`).
pub async fn authenticate<F, Fut>(
    provider_id: &str,
    ids: &Ids,
    send: &F,
    prefer_device: impl Future<Output = bool>,
) -> Result<(), String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let value = send(wire::gateway_oauth_read(provider_id, ids.next()))
        .await
        .map_err(|error| error.message)?;
    let state = wire::gateway_oauth_state(value)?;
    if state.provider_id != provider_id {
        return Err("The selected gateway changed during sign-in.".to_owned());
    }
    // The machine is asked only when the answer can matter
    // (`needs_device_login`'s short circuit).
    let device = wire::device_login_possible(&state) && prefer_device.await;
    if wire::gateway_oauth_login_due(&state, device) {
        send(wire::gateway_oauth_login(provider_id, ids.next()))
            .await
            .map_err(|error| error.message)?;
    }
    Ok(())
}

/// `device_login::prefer_device_login`: no browser can open here, so the
/// device code is the way.
pub async fn prefer_device_login() -> bool {
    let env = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    if prefers_device(
        std::env::var_os("SSH_TTY").is_some() || std::env::var_os("SSH_CONNECTION").is_some(),
        cfg!(target_os = "linux"),
        env("DISPLAY") || env("WAYLAND_DISPLAY"),
    ) {
        return true;
    }
    !tokio::task::spawn_blocking(webbrowser::Browser::is_available)
        .await
        .unwrap_or(false)
}

/// The environment's part of the preference: an SSH session, or Linux
/// without a display.
fn prefers_device(ssh: bool, linux: bool, display: bool) -> bool {
    ssh || (linux && !display)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use codex_app_server_protocol::ClientRequest;
    use serde_json::json;

    /// A server that answers the read with `read` and records the methods
    /// it was asked.
    struct Server {
        read: serde_json::Value,
        login: Result<serde_json::Value, ServerError>,
        asked: Mutex<Vec<String>>,
    }

    impl Server {
        fn answering(read: serde_json::Value) -> Server {
            Server {
                read,
                login: Ok(json!({"prompt": null})),
                asked: Mutex::new(Vec::new()),
            }
        }

        async fn answer(&self, request: ClientRequest) -> Result<serde_json::Value, ServerError> {
            let method = request.method_name().to_owned();
            self.asked.lock().unwrap().push(method);
            match request {
                ClientRequest::GatewayOAuthRead { .. } => Ok(self.read.clone()),
                ClientRequest::GatewayOAuthLogin { .. } => self.login.clone(),
                other => panic!("unexpected request {other:?}"),
            }
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    fn read(required: bool, device: bool, status: Option<&str>) -> serde_json::Value {
        json!({
            "providerId": "office",
            "providerName": "Office",
            "required": required,
            "supportsDeviceAuth": device,
            "status": status,
            "error": null,
        })
    }

    async fn run(server: &Server, prefer: bool) -> Result<(), String> {
        authenticate(
            "office",
            &Ids::default(),
            &|request| server.answer(request),
            async move { prefer },
        )
        .await
    }

    #[tokio::test]
    async fn a_provider_that_needs_the_login_is_asked_to_log_in() {
        let server = Server::answering(read(true, false, Some("notReady")));
        assert_eq!(run(&server, false).await, Ok(()));
        assert_eq!(
            server.asked(),
            ["account/gatewayOAuth/read", "account/gatewayOAuth/login"]
        );
    }

    #[tokio::test]
    async fn a_signed_in_or_unneeded_provider_is_only_read() {
        for read in [
            read(true, false, Some("succeeded")),
            read(false, false, None),
        ] {
            let server = Server::answering(read);
            assert_eq!(run(&server, false).await, Ok(()));
            assert_eq!(server.asked(), ["account/gatewayOAuth/read"]);
        }
    }

    // The device code is the startup's own: no browser login is asked when
    // the machine prefers it and the server has the endpoint.
    #[tokio::test]
    async fn the_device_code_replaces_the_browser_login_only_when_preferred() {
        let server = Server::answering(read(true, true, Some("failed")));
        assert_eq!(run(&server, true).await, Ok(()));
        assert_eq!(server.asked(), ["account/gatewayOAuth/read"]);
        let server = Server::answering(read(true, true, Some("failed")));
        assert_eq!(run(&server, false).await, Ok(()));
        assert_eq!(server.asked().len(), 2);
        // A server without the endpoint logs in whatever the preference.
        let server = Server::answering(read(true, false, Some("failed")));
        assert_eq!(run(&server, true).await, Ok(()));
        assert_eq!(server.asked().len(), 2);
    }

    #[tokio::test]
    async fn the_preference_is_not_asked_when_it_cannot_matter() {
        let server = Server::answering(read(true, true, Some("succeeded")));
        let asked = std::cell::Cell::new(false);
        let result = authenticate(
            "office",
            &Ids::default(),
            &|request| server.answer(request),
            async {
                asked.set(true);
                true
            },
        )
        .await;
        assert_eq!(result, Ok(()));
        assert!(!asked.get());
    }

    #[tokio::test]
    async fn another_provider_in_the_answer_is_an_error() {
        let mut other = read(true, false, Some("notReady"));
        other["providerId"] = json!("elsewhere");
        let server = Server::answering(other);
        assert_eq!(
            run(&server, false).await,
            Err("The selected gateway changed during sign-in.".to_owned())
        );
        assert_eq!(server.asked(), ["account/gatewayOAuth/read"]);
    }

    // The failure text is the client's own (`err.to_string()`), as the
    // server's refusal came through the sender.
    #[tokio::test]
    async fn a_refusal_of_the_login_is_the_text_of_the_failure() {
        let mut server = Server::answering(read(true, false, Some("notReady")));
        server.login = Err(ServerError {
            code: Some(-32600),
            message: "account/gatewayOAuth/login failed: Gateway refused (code -32600)".into(),
        });
        assert_eq!(
            run(&server, false).await,
            Err("account/gatewayOAuth/login failed: Gateway refused (code -32600)".to_owned())
        );
    }

    #[test]
    fn the_environment_prefers_the_device_code_over_ssh_or_without_a_display() {
        assert!(prefers_device(true, false, true));
        assert!(prefers_device(false, true, false));
        assert!(!prefers_device(false, true, true));
        assert!(!prefers_device(false, false, false));
    }
}
