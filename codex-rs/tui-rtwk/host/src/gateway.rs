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
use codex_tui_core::screens::onboarding::auth::DeviceCode;
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
    // The text the oracle shows when the answer does not decode.
    let state = wire::gateway_oauth_state(value)
        .map_err(|error| format!("account/gatewayOAuth/read response decode error: {error}"))?;
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

/// `device_login::start_if_needed`: whether the gateway the session will use
/// needs a sign-in and a device code is the way; the code to show, none when
/// startup goes on without one (a server without the sign-in, a provider
/// that is signed in or has no device endpoint, a browser that can open).
/// `provider` is the one the launch names (`explicit_model_provider`), none
/// for the server's own. The text of an error is the client's, as the oracle
/// ends the launch with it (`io::Error::other(error)`). The resumed thread's
/// own provider is not read: the page has no resume selection.
pub async fn start_device<F, Fut>(
    provider: Option<&str>,
    ids: &Ids,
    send: &F,
    prefer_device: impl Future<Output = bool>,
) -> Result<Option<DeviceCode>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let value = match send(wire::gateway_oauth_read_of(provider, ids.next())).await {
        Ok(value) => value,
        Err(error) if wire::gateway_read_missing(&error) => return Ok(None),
        Err(error) => {
            // Servers with the device login always include
            // `supportsDeviceAuth`, even when false: ask without a provider
            // to recognise older servers that reject named reads.
            if provider.is_some()
                && wire::gateway_named_read_refused(&error)
                && let Ok(probe) = send(wire::gateway_oauth_read_of(None, ids.next())).await
                && wire::gateway_server_without_device(&probe)
            {
                return Ok(None);
            }
            return Err(error.message);
        }
    };
    let state = wire::gateway_oauth_state(value)
        .map_err(|error| format!("account/gatewayOAuth/read response decode error: {error}"))?;
    if provider.is_some_and(|id| id != state.provider_id)
        || !(wire::device_login_possible(&state) && prefer_device.await)
    {
        return Ok(None);
    }
    let value = send(wire::gateway_device_login(&state.provider_id, ids.next()))
        .await
        .map_err(|error| error.message)?;
    wire::gateway_device_prompt(value)
        .map_err(|error| format!("account/gatewayOAuth/login response decode error: {error}"))
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
    !codex_tui_rtwk::services::startup::browser_available(webbrowser::Browser::is_available).await
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

    #[tokio::test]
    async fn an_answer_that_does_not_decode_is_framed_as_the_oracle_does() {
        let server = Server::answering(json!({"providerId": "office"}));
        let Err(text) = run(&server, false).await else {
            panic!("an incomplete answer decoded");
        };
        assert!(
            text.starts_with("account/gatewayOAuth/read response decode error: missing field"),
            "{text}"
        );
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

    /// A server for the start of the device code: what it answers a read
    /// with a provider and without, and the login; what it was asked.
    struct Device {
        named: Result<serde_json::Value, ServerError>,
        plain: Result<serde_json::Value, ServerError>,
        login: Result<serde_json::Value, ServerError>,
        asked: Mutex<Vec<String>>,
    }

    impl Device {
        fn answering(named: serde_json::Value) -> Device {
            Device {
                plain: Ok(named.clone()),
                named: Ok(named),
                login: Ok(prompt()),
                asked: Mutex::new(Vec::new()),
            }
        }

        async fn answer(&self, request: ClientRequest) -> Result<serde_json::Value, ServerError> {
            let mut asked = self.asked.lock().unwrap();
            match request {
                ClientRequest::GatewayOAuthRead { params, .. } => {
                    let provider = params.and_then(|params| params.model_provider);
                    asked.push(format!("read:{}", provider.as_deref().unwrap_or("-")));
                    if provider.is_some() {
                        self.named.clone()
                    } else {
                        self.plain.clone()
                    }
                }
                ClientRequest::GatewayOAuthLogin { params, .. } => {
                    let params = params.unwrap();
                    asked.push(format!(
                        "login:{}:{}",
                        params.model_provider.unwrap_or_default(),
                        params.device_auth
                    ));
                    self.login.clone()
                }
                other => panic!("unexpected request {other:?}"),
            }
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    fn prompt() -> serde_json::Value {
        json!({"prompt": {
            "loginId": "device",
            "verificationUrl": "https://login.example/device",
            "userCode": "ABCD-EFGH",
        }})
    }

    fn code() -> DeviceCode {
        DeviceCode {
            login_id: "device".into(),
            url: "https://login.example/device".into(),
            code: "ABCD-EFGH".into(),
        }
    }

    fn refusal(code: i64, message: &str) -> ServerError {
        ServerError {
            code: Some(code),
            message: message.into(),
        }
    }

    async fn start(
        server: &Device,
        provider: Option<&str>,
        prefer: bool,
    ) -> Result<Option<DeviceCode>, String> {
        start_device(
            provider,
            &Ids::default(),
            &|request| server.answer(request),
            async move { prefer },
        )
        .await
    }

    // `start_if_needed`: a provider that needs the sign-in, has the device
    // endpoint and is not signed in gets the device login, whichever of the
    // two ways of naming it.
    #[tokio::test]
    async fn a_gateway_that_needs_the_sign_in_is_asked_for_its_code() {
        for provider in [Some("office"), None] {
            let server = Device::answering(read(true, true, Some("notReady")));
            assert_eq!(start(&server, provider, true).await, Ok(Some(code())));
            let named = provider.unwrap_or("-");
            assert_eq!(
                server.asked(),
                [format!("read:{named}"), "login:office:true".to_owned()]
            );
        }
        let server = Device::answering(read(true, true, Some("failed")));
        assert_eq!(start(&server, None, true).await, Ok(Some(code())));
    }

    #[tokio::test]
    async fn nothing_to_show_is_none_without_the_login() {
        for (read, prefer) in [
            (read(true, true, Some("succeeded")), true),
            (read(false, true, None), true),
            (read(true, false, Some("notReady")), true),
            // The browser can open.
            (read(true, true, Some("notReady")), false),
        ] {
            let server = Device::answering(read);
            assert_eq!(start(&server, Some("office"), prefer).await, Ok(None));
            assert_eq!(server.asked(), ["read:office"]);
        }
    }

    #[tokio::test]
    async fn the_preference_is_asked_only_when_it_can_matter() {
        let server = Device::answering(read(true, true, Some("succeeded")));
        let asked = std::cell::Cell::new(false);
        let result = start_device(
            Some("office"),
            &Ids::default(),
            &|request| server.answer(request),
            async {
                asked.set(true);
                true
            },
        )
        .await;
        assert_eq!(result, Ok(None));
        assert!(!asked.get());
    }

    #[tokio::test]
    async fn another_provider_than_the_named_one_is_not_signed_in() {
        let mut other = read(true, true, Some("notReady"));
        other["providerId"] = json!("elsewhere");
        let server = Device::answering(other);
        assert_eq!(start(&server, Some("office"), true).await, Ok(None));
        assert_eq!(server.asked(), ["read:office"]);
    }

    // A server that does not have the gateway sign-in has nothing to show.
    #[tokio::test]
    async fn a_server_without_the_sign_in_goes_on() {
        for error in [
            refusal(-32601, "account/gatewayOAuth/read failed: Method not found (code -32601)"),
            refusal(
                -32600,
                "account/gatewayOAuth/read failed: Invalid request: unknown variant `account/gatewayOAuth/read` (code -32600)",
            ),
        ] {
            let mut server = Device::answering(json!(null));
            server.named = Err(error);
            assert_eq!(start(&server, Some("office"), true).await, Ok(None));
            assert_eq!(server.asked(), ["read:office"]);
        }
    }

    // An older server that rejects a named read: asked without the name, it
    // names its provider and lacks `supportsDeviceAuth`.
    #[tokio::test]
    async fn an_older_server_that_rejects_the_named_read_goes_on() {
        let refused = refusal(
            -32600,
            "account/gatewayOAuth/read failed: Invalid request: invalid type (code -32600)",
        );
        let mut server = Device::answering(json!(null));
        server.named = Err(refused.clone());
        server.plain = Ok(json!({"providerId": "office", "required": true}));
        assert_eq!(start(&server, Some("office"), true).await, Ok(None));
        assert_eq!(server.asked(), ["read:office", "read:-"]);
        // A server that includes the field refuses for real: the client's text.
        server.plain = Ok(read(true, false, None));
        assert_eq!(
            start(&server, Some("office"), true).await,
            Err(refused.message.clone())
        );
        // Without a name there is nothing to retry.
        let mut server = Device::answering(json!(null));
        server.plain = Err(refused.clone());
        assert_eq!(start(&server, None, true).await, Err(refused.message));
        assert_eq!(server.asked(), ["read:-"]);
    }

    #[tokio::test]
    async fn another_failure_is_the_clients_text() {
        let mut server = Device::answering(json!(null));
        server.named = Err(refusal(
            -32603,
            "account/gatewayOAuth/read failed: boom (code -32603)",
        ));
        assert_eq!(
            start(&server, Some("office"), true).await,
            Err("account/gatewayOAuth/read failed: boom (code -32603)".to_owned())
        );
        let mut server = Device::answering(read(true, true, Some("notReady")));
        server.login = Err(refusal(
            -32600,
            "account/gatewayOAuth/login failed: no (code -32600)",
        ));
        assert_eq!(
            start(&server, Some("office"), true).await,
            Err("account/gatewayOAuth/login failed: no (code -32600)".to_owned())
        );
        let mut server = Device::answering(read(true, true, Some("notReady")));
        server.login = Ok(json!({"prompt": 3}));
        let Err(text) = start(&server, Some("office"), true).await else {
            panic!("an unreadable prompt decoded");
        };
        assert!(
            text.starts_with("account/gatewayOAuth/login response decode error: "),
            "{text}"
        );
    }

    // A login that signed in without a prompt (a refreshed credential).
    #[tokio::test]
    async fn a_login_without_a_prompt_goes_on() {
        let mut server = Device::answering(read(true, true, Some("notReady")));
        server.login = Ok(json!({"prompt": null}));
        assert_eq!(start(&server, Some("office"), true).await, Ok(None));
    }

    #[test]
    fn the_environment_prefers_the_device_code_over_ssh_or_without_a_display() {
        assert!(prefers_device(true, false, true));
        assert!(prefers_device(false, true, false));
        assert!(!prefers_device(false, true, true));
        assert!(!prefers_device(false, false, false));
    }
}
