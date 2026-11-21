//! S02, the sign-in step of the startup page: what may be chosen, where the
//! attempt stands and its error, changed only by `apply` (through the
//! startup page's `Ev`, `Intent` and `Ask`). Oracle:
//! `onboarding/auth.rs` (`AuthModeWidget`'s `SignInState`, its error, and
//! the state machine of `handle_sign_in_option`, `save_api_key`,
//! `cancel_active_attempt`, `on_account_login_completed`) and
//! `onboarding/auth/headless_chatgpt_login.rs` (the device code attempt,
//! guarded by its request).
//!
//! What the view keeps (widget state, R14): the menu shown (the first or
//! "another way"), its highlighted row, the API key typed. The login itself
//! is the host's (`Ask::Login`, `Ask::CancelLogin`), answered with
//! `Ev::Auth(AuthResult { request, .. })`; an answer to an ask no longer
//! awaited is dropped (an orphaned device code login is cancelled, as the
//! oracle does).

use super::{Ask, Request};

/// What the sign-in step offers, read by the host at launch
/// (`OnboardingScreenArgs`, `AuthConfig::is_login_method_allowed`,
/// `should_show_provider_setup_wizard`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Offer {
    /// ChatGPT login is allowed (`ForcedLoginMethod::Chatgpt` or none).
    pub chatgpt: bool,
    /// API key login is allowed (`ForcedLoginMethod::Api` or none).
    pub api_key: bool,
    /// The provider wizards offered under "Sign in another way" (S03).
    pub providers: Providers,
    /// `OPENAI_API_KEY`, which the key entry starts from
    /// (`read_openai_api_key_from_env`).
    pub env_api_key: Option<String>,
}

/// The provider setup wizards offered (`bedrock_setup_enabled`,
/// `gcp_setup_enabled`, `gateway_setup_enabled`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Providers {
    pub bedrock: bool,
    pub gcp: bool,
    pub gateway: bool,
}

impl Providers {
    pub fn any(self) -> bool {
        self.bedrock || self.gcp || self.gateway
    }

    pub fn offers(self, provider: Provider) -> bool {
        match provider {
            Provider::Bedrock => self.bedrock,
            Provider::Gcp => self.gcp,
            Provider::Gateway => self.gateway,
        }
    }
}

/// A provider setup (S03).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Bedrock,
    Gcp,
    Gateway,
}

/// A way to sign in (`SignInOption` but the menu's "another way", which is
/// the view's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    ChatGpt,
    DeviceCode,
    ApiKey,
    Provider(Provider),
}

/// Where the sign-in stands (`SignInState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The menu.
    #[default]
    Pick,
    /// ChatGPT login started: finish it in the browser.
    Browser { login_id: String, url: String },
    /// The device code login: asked (`code` none) or shown.
    DeviceCode {
        request: Request,
        code: Option<DeviceCode>,
    },
    /// Signed in with ChatGPT: the message Enter leaves.
    SuccessMessage,
    /// Signed in with ChatGPT.
    Success,
    /// The API key entry; `prefill` is the key it starts from.
    ApiKey { prefill: Option<String> },
    /// The API key is saved.
    ApiKeyConfigured,
    /// A provider's setup is chosen: the provider step (S03) runs it.
    Provider(Provider),
}

/// A device code to enter (`ContinueWithDeviceCodeState::ready`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCode {
    pub login_id: String,
    pub url: String,
    pub code: String,
}

/// What went wrong; the view says it (no prose in core).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// "API key login is disabled."
    ApiKeyDisabled,
    /// "API key cannot be empty"
    EmptyApiKey,
    /// "Failed to save API key: {0}"
    SaveFailed(String),
    /// "Unexpected account/login/start response: {0}"
    Unexpected(String),
    /// The server's own message.
    Server(String),
}

/// The sign-in step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignIn {
    pub offer: Offer,
    pub state: State,
    pub error: Option<Error>,
    /// The account is a ChatGPT one already (`LoginStatus::AuthMode` with
    /// `has_chatgpt_account`): a ChatGPT choice is done at once.
    pub chatgpt_account: bool,
    /// The browser login or API key save awaited.
    awaiting: Option<(Request, Login)>,
}

/// What the host is asked to log in with (`LoginAccountParams`).
#[derive(Clone, PartialEq, Eq)]
pub enum Login {
    ChatGpt,
    DeviceCode,
    ApiKey(String),
}

impl std::fmt::Debug for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Login::ChatGpt => f.write_str("ChatGpt"),
            Login::DeviceCode => f.write_str("DeviceCode"),
            // The key stays out of logs.
            Login::ApiKey(_) => f.write_str("ApiKey(..)"),
        }
    }
}

/// The answer to `Ask::Login { request, .. }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthResult {
    pub request: Request,
    pub outcome: Outcome,
}

/// What `account/login/start` answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// ChatGPT: the browser login started (the host opened the browser).
    Browser { login_id: String, url: String },
    /// The device code to show.
    DeviceCode(DeviceCode),
    /// The API key is saved.
    ApiKeySaved,
    /// The login could not start or the key not be saved.
    Failed(Error),
}

/// The sign-in intents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A way to sign in, from the menu.
    Choose(Method),
    /// The menu changed ("another way" opened or left): the error goes.
    Menu,
    /// Esc on an attempt: cancel it, back to the menu.
    Cancel,
    /// Enter on the API key entry, with the text typed.
    SaveApiKey(String),
    /// Enter on the success message.
    Continue,
}

impl SignIn {
    /// A sign-in step offering `offer`.
    pub fn new(offer: Offer) -> SignIn {
        SignIn {
            offer,
            ..SignIn::default()
        }
    }

    /// Whether the step is finished (`StepState::Complete`); a provider
    /// chosen leaves it for the provider step.
    pub fn done(&self) -> bool {
        matches!(
            self.state,
            State::Success | State::ApiKeyConfigured | State::Provider(_)
        )
    }

    /// The login awaited, if any (what the view may show as starting).
    pub fn awaiting(&self) -> Option<&Login> {
        self.awaiting.as_ref().map(|(_, login)| login)
    }

    /// `should_suppress_animations`: copyable login material is shown.
    pub fn freezes_animations(&self) -> bool {
        matches!(self.state, State::Browser { .. } | State::DeviceCode { .. })
    }

    /// The API key entry is the step's body (`is_text_entry_active`).
    pub fn is_text_entry(&self) -> bool {
        matches!(self.state, State::ApiKey { .. })
    }

    /// `disallow_api_login`.
    fn disallow_api_login(&mut self) {
        self.error = Some(Error::ApiKeyDisabled);
        self.state = State::Pick;
    }
}

/// An intent of the sign-in step; `mint` numbers a new ask.
pub fn apply_intent(
    sign_in: &mut SignIn,
    intent: Intent,
    mint: &mut dyn FnMut() -> Request,
    asks: &mut Vec<Ask>,
) {
    match intent {
        Intent::Menu => sign_in.error = None,
        Intent::Choose(method) => choose(sign_in, method, mint, asks),
        Intent::Cancel => cancel(sign_in, asks),
        // `handle_api_key_entry_key_event` and `save_api_key`.
        Intent::SaveApiKey(text) => {
            if !matches!(sign_in.state, State::ApiKey { .. }) {
                return;
            }
            if !sign_in.offer.api_key {
                sign_in.disallow_api_login();
                return;
            }
            let key = text.trim();
            if key.is_empty() {
                sign_in.error = Some(Error::EmptyApiKey);
                return;
            }
            sign_in.error = None;
            let request = mint();
            let login = Login::ApiKey(key.to_owned());
            sign_in.awaiting = Some((request, login.clone()));
            asks.push(Ask::Login { request, login });
        }
        Intent::Continue => {
            if sign_in.state == State::SuccessMessage {
                sign_in.state = State::Success;
            }
        }
    }
}

/// `handle_sign_in_option`.
fn choose(
    sign_in: &mut SignIn,
    method: Method,
    mint: &mut dyn FnMut() -> Request,
    asks: &mut Vec<Ask>,
) {
    if sign_in.state != State::Pick {
        return;
    }
    match method {
        Method::ChatGpt | Method::DeviceCode => {
            if !sign_in.offer.chatgpt {
                return;
            }
            // `handle_existing_chatgpt_login`.
            if sign_in.chatgpt_account {
                sign_in.state = State::Success;
                return;
            }
            sign_in.error = None;
            let request = mint();
            if method == Method::ChatGpt {
                sign_in.awaiting = Some((request, Login::ChatGpt));
                asks.push(Ask::Login {
                    request,
                    login: Login::ChatGpt,
                });
            } else {
                sign_in.state = State::DeviceCode {
                    request,
                    code: None,
                };
                asks.push(Ask::Login {
                    request,
                    login: Login::DeviceCode,
                });
            }
        }
        Method::ApiKey => {
            if !sign_in.offer.api_key {
                sign_in.disallow_api_login();
                return;
            }
            sign_in.error = None;
            sign_in.state = State::ApiKey {
                prefill: sign_in.offer.env_api_key.clone(),
            };
        }
        Method::Provider(provider) => {
            if !sign_in.offer.api_key {
                sign_in.disallow_api_login();
            } else if sign_in.offer.providers.offers(provider) {
                sign_in.error = None;
                sign_in.state = State::Provider(provider);
            }
        }
    }
}

/// `cancel_active_attempt` and the API key entry's Esc.
fn cancel(sign_in: &mut SignIn, asks: &mut Vec<Ask>) {
    match &sign_in.state {
        State::Browser { login_id, .. } => asks.push(Ask::CancelLogin {
            login_id: login_id.clone(),
        }),
        State::DeviceCode { code, .. } => {
            if let Some(code) = code {
                asks.push(Ask::CancelLogin {
                    login_id: code.login_id.clone(),
                });
            }
        }
        State::ApiKey { .. } | State::Provider(_) => {}
        _ => return,
    }
    sign_in.state = State::Pick;
    sign_in.error = None;
}

/// The answer of a login ask. One no longer awaited is dropped; a device
/// code that came for an attempt left is cancelled.
pub fn apply_result(sign_in: &mut SignIn, result: AuthResult, asks: &mut Vec<Ask>) {
    let AuthResult { request, outcome } = result;
    let device = matches!(sign_in.state, State::DeviceCode { request: r, code: None } if r == request);
    let awaited = sign_in
        .awaiting
        .as_ref()
        .is_some_and(|(r, _)| *r == request);
    if !device && !awaited {
        if let Outcome::DeviceCode(code) = outcome {
            asks.push(Ask::CancelLogin {
                login_id: code.login_id,
            });
        }
        return;
    }
    let login = if awaited {
        sign_in.awaiting.take().map(|(_, login)| login)
    } else {
        Some(Login::DeviceCode)
    };
    match (login, outcome) {
        (Some(Login::ChatGpt), Outcome::Browser { login_id, url }) => {
            sign_in.error = None;
            sign_in.state = State::Browser { login_id, url };
        }
        (Some(Login::DeviceCode), Outcome::DeviceCode(code)) => {
            sign_in.error = None;
            sign_in.state = State::DeviceCode {
                request,
                code: Some(code),
            };
        }
        (Some(Login::ApiKey(_)), Outcome::ApiKeySaved) => {
            sign_in.error = None;
            sign_in.state = State::ApiKeyConfigured;
        }
        // `save_api_key`: back on the entry with the key kept (the field
        // still holds it) and the error.
        (Some(Login::ApiKey(_)), Outcome::Failed(error)) => {
            sign_in.state = State::ApiKey { prefill: None };
            sign_in.error = Some(error);
        }
        (Some(Login::ApiKey(_)), other) => {
            sign_in.state = State::ApiKey { prefill: None };
            sign_in.error = Some(Error::Unexpected(format!("{other:?}")));
        }
        (Some(Login::ChatGpt | Login::DeviceCode), Outcome::Failed(error)) => {
            sign_in.state = State::Pick;
            sign_in.error = Some(error);
        }
        (Some(Login::ChatGpt | Login::DeviceCode), other) => {
            sign_in.state = State::Pick;
            sign_in.error = Some(Error::Unexpected(format!("{other:?}")));
        }
        (None, _) => {}
    }
}

/// `account/login/completed` (`on_account_login_completed`): only for the
/// login shown.
pub fn login_completed(
    sign_in: &mut SignIn,
    login_id: Option<&str>,
    success: bool,
    error: Option<String>,
) {
    let Some(login_id) = login_id else {
        return;
    };
    let matching = match &sign_in.state {
        State::Browser { login_id: id, .. } => id == login_id,
        State::DeviceCode {
            code: Some(code), ..
        } => code.login_id == login_id,
        _ => false,
    };
    if !matching {
        return;
    }
    if success {
        sign_in.error = None;
        sign_in.state = State::SuccessMessage;
    } else {
        sign_in.error = error.map(Error::Server);
        sign_in.state = State::Pick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn offer() -> Offer {
        Offer {
            chatgpt: true,
            api_key: true,
            ..Offer::default()
        }
    }

    /// `widget_forced_chatgpt`: ChatGPT login forced, no API key.
    fn forced_chatgpt() -> SignIn {
        SignIn::new(Offer {
            chatgpt: true,
            api_key: false,
            ..Offer::default()
        })
    }

    struct Run {
        next: Request,
        asks: Vec<Ask>,
    }

    impl Run {
        fn new() -> Run {
            Run {
                next: 0,
                asks: Vec::new(),
            }
        }

        fn intent(&mut self, sign_in: &mut SignIn, intent: Intent) {
            let mut next = self.next;
            let mut mint = || {
                next += 1;
                next
            };
            apply_intent(sign_in, intent, &mut mint, &mut self.asks);
            self.next = next;
        }

        fn result(&mut self, sign_in: &mut SignIn, request: Request, outcome: Outcome) {
            apply_result(sign_in, AuthResult { request, outcome }, &mut self.asks);
        }
    }

    fn device_code(login_id: &str) -> DeviceCode {
        DeviceCode {
            login_id: login_id.into(),
            url: "https://chatgpt.com/device".into(),
            code: "ABCD-EFGH".into(),
        }
    }

    // auth.rs:1525
    #[test]
    fn api_key_flow_disabled_when_chatgpt_forced() {
        let mut sign_in = forced_chatgpt();
        Run::new().intent(&mut sign_in, Intent::Choose(Method::ApiKey));
        assert_eq!(sign_in.error, Some(Error::ApiKeyDisabled));
        assert_eq!(sign_in.state, State::Pick);
    }

    // auth.rs:1616
    #[test]
    fn saving_api_key_is_blocked_when_chatgpt_forced() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::ApiKey { prefill: None };
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::SaveApiKey("sk-test".into()));
        assert_eq!(sign_in.error, Some(Error::ApiKeyDisabled));
        assert_eq!(sign_in.state, State::Pick);
        assert!(run.asks.is_empty());
        assert!(!sign_in.chatgpt_account);
    }

    // auth.rs:1633: a ChatGPT account of any kind (tokens, personal access
    // token) is signed in already.
    #[test]
    fn existing_non_oauth_chatgpt_login_counts_as_signed_in() {
        for method in [Method::ChatGpt, Method::DeviceCode] {
            let mut sign_in = forced_chatgpt();
            sign_in.chatgpt_account = true;
            let mut run = Run::new();
            run.intent(&mut sign_in, Intent::Choose(method));
            assert_eq!(sign_in.state, State::Success);
            assert!(run.asks.is_empty());
        }
    }

    // auth.rs:1649
    #[test]
    fn cancel_active_attempt_resets_browser_login_state() {
        let mut sign_in = forced_chatgpt();
        sign_in.error = Some(Error::Server("still logging in".into()));
        sign_in.state = State::Browser {
            login_id: "login-1".into(),
            url: "https://auth.example.com".into(),
        };
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Cancel);
        assert_eq!(sign_in.error, None);
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(
            run.asks,
            vec![Ask::CancelLogin {
                login_id: "login-1".into()
            }]
        );
    }

    // auth.rs:1668
    #[test]
    fn cancel_active_attempt_notifies_device_code_login() {
        let mut sign_in = forced_chatgpt();
        sign_in.error = Some(Error::Server("still logging in".into()));
        sign_in.state = State::DeviceCode {
            request: 1,
            code: Some(device_code("login-1")),
        };
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Cancel);
        assert_eq!(sign_in.error, None);
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(
            run.asks,
            vec![Ask::CancelLogin {
                login_id: "login-1".into()
            }]
        );
    }

    // auth.rs:1408: the login completes while the browser is opening: the
    // host answers the start, then the completion arrives.
    #[test]
    fn login_can_complete_while_browser_is_opening() {
        let mut sign_in = forced_chatgpt();
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Choose(Method::ChatGpt));
        assert_eq!(
            run.asks,
            vec![Ask::Login {
                request: 1,
                login: Login::ChatGpt
            }]
        );
        run.result(
            &mut sign_in,
            1,
            Outcome::Browser {
                login_id: "login-1".into(),
                url: "https://example.test/authorize".into(),
            },
        );
        login_completed(&mut sign_in, Some("login-1"), true, None);
        assert_eq!(sign_in.state, State::SuccessMessage);
    }

    // auth.rs:1809, 1824
    #[test]
    fn auth_widget_suppresses_animations_when_device_code_is_visible() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 1,
            code: Some(device_code("login-1")),
        };
        assert!(sign_in.freezes_animations());
    }

    #[test]
    fn auth_widget_suppresses_animations_while_requesting_device_code() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 1,
            code: None,
        };
        assert!(sign_in.freezes_animations());
    }

    // auth.rs:1835
    #[test]
    fn device_code_login_completion_advances_to_success_message() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 1,
            code: Some(device_code("login-1")),
        };
        login_completed(&mut sign_in, Some("login-1"), true, None);
        assert_eq!(sign_in.state, State::SuccessMessage);
    }

    // headless_chatgpt_login.rs:224 device_code_attempt_matches_only_for_matching_request_id
    // and :238 set_device_code_state_for_active_attempt_updates_only_when_active:
    // the code of the attempt shown is taken; one for another attempt is
    // dropped and its login cancelled.
    #[test]
    fn set_device_code_state_for_active_attempt_updates_only_when_active() {
        let mut sign_in = forced_chatgpt();
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Choose(Method::DeviceCode));
        run.result(&mut sign_in, 1, Outcome::DeviceCode(device_code("login-1")));
        assert_eq!(
            sign_in.state,
            State::DeviceCode {
                request: 1,
                code: Some(device_code("login-1"))
            }
        );

        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 2,
            code: None,
        };
        let mut run = Run::new();
        run.result(&mut sign_in, 1, Outcome::DeviceCode(device_code("login-1")));
        assert_eq!(
            sign_in.state,
            State::DeviceCode {
                request: 2,
                code: None
            }
        );
        assert_eq!(
            run.asks,
            vec![Ask::CancelLogin {
                login_id: "login-1".into()
            }]
        );
    }

    // headless_chatgpt_login.rs:286
    #[test]
    fn set_device_code_error_for_active_attempt_updates_only_when_active() {
        let failed = || Outcome::Failed(Error::Server("device code unavailable".into()));
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 1,
            code: None,
        };
        let mut run = Run::new();
        run.result(&mut sign_in, 1, failed());
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(
            sign_in.error,
            Some(Error::Server("device code unavailable".into()))
        );

        let mut sign_in = forced_chatgpt();
        sign_in.state = State::DeviceCode {
            request: 2,
            code: None,
        };
        run.result(&mut sign_in, 1, failed());
        assert!(matches!(sign_in.state, State::DeviceCode { .. }));
        assert_eq!(sign_in.error, None);
    }

    /// research/off-thread-state.md (a) item 1: a sign-in answer under a
    /// request no longer awaited (cancelled, superseded) is dropped.
    #[test]
    fn stale_auth_results_are_dropped() {
        let mut sign_in = SignIn::new(offer());
        let mut run = Run::new();
        // A browser login started; an answer under another request is stale.
        run.intent(&mut sign_in, Intent::Choose(Method::ChatGpt));
        run.result(
            &mut sign_in,
            9,
            Outcome::Browser {
                login_id: "old".into(),
                url: "https://old".into(),
            },
        );
        assert_eq!(sign_in.state, State::Pick);
        run.result(&mut sign_in, 1, Outcome::Failed(Error::Server("x".into())));
        assert_eq!(sign_in.error, Some(Error::Server("x".into())));
        // An API key answer after the step moved on is dropped too.
        sign_in.error = None;
        run.result(&mut sign_in, 7, Outcome::ApiKeySaved);
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(sign_in.error, None);
    }

    /// Review O1: Enter twice within the round trip of the ChatGPT login
    /// (state still `Pick`, the ask out) starts one login, not two.
    #[test]
    fn second_choose_while_a_login_is_awaited_is_ignored() {
        let mut sign_in = SignIn::new(offer());
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Choose(Method::ChatGpt));
        run.intent(&mut sign_in, Intent::Choose(Method::ChatGpt));
        assert_eq!(
            run.asks,
            vec![Ask::Login {
                request: 1,
                login: Login::ChatGpt
            }]
        );
        // The one answer is the one awaited.
        run.result(
            &mut sign_in,
            1,
            Outcome::Browser {
                login_id: "L".into(),
                url: "https://u".into(),
            },
        );
        assert_eq!(
            sign_in.state,
            State::Browser {
                login_id: "L".into(),
                url: "https://u".into()
            }
        );
        assert_eq!(run.asks.len(), 1);
    }

    /// Review O1: a browser login answered under a request no longer
    /// awaited is cancelled, as a stale device code is.
    #[test]
    fn stale_browser_outcome_cancels_the_login() {
        let mut sign_in = SignIn::new(offer());
        let mut run = Run::new();
        run.result(
            &mut sign_in,
            4,
            Outcome::Browser {
                login_id: "orphan".into(),
                url: "https://o".into(),
            },
        );
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(
            run.asks,
            vec![Ask::CancelLogin {
                login_id: "orphan".into()
            }]
        );
    }

    /// Review O1 (secrets): Esc on the API key entry while the save is out
    /// forgets the pending key; its late answer is dropped.
    #[test]
    fn cancel_forgets_the_pending_api_key() {
        let mut sign_in = SignIn::new(offer());
        sign_in.state = State::ApiKey { prefill: None };
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::SaveApiKey("sk-secret".into()));
        assert!(sign_in.awaiting().is_some());
        run.intent(&mut sign_in, Intent::Cancel);
        assert_eq!(sign_in.awaiting(), None);
        assert_eq!(sign_in.state, State::Pick);
        run.result(&mut sign_in, 1, Outcome::ApiKeySaved);
        assert_eq!(sign_in.state, State::Pick);
    }

    /// `handle_api_key_entry_key_event`: an empty key is refused; a key is
    /// asked to be saved, and the save's answer ends the step.
    #[test]
    fn api_key_save_asks_and_confirms() {
        let mut sign_in = SignIn::new(Offer {
            env_api_key: Some("sk-env".into()),
            ..offer()
        });
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Choose(Method::ApiKey));
        assert_eq!(
            sign_in.state,
            State::ApiKey {
                prefill: Some("sk-env".into())
            }
        );
        run.intent(&mut sign_in, Intent::SaveApiKey("   ".into()));
        assert_eq!(sign_in.error, Some(Error::EmptyApiKey));
        run.intent(&mut sign_in, Intent::SaveApiKey(" sk-test ".into()));
        assert_eq!(sign_in.error, None);
        assert_eq!(
            run.asks,
            vec![Ask::Login {
                request: 1,
                login: Login::ApiKey("sk-test".into())
            }]
        );
        run.result(
            &mut sign_in,
            1,
            Outcome::Failed(Error::SaveFailed("denied".into())),
        );
        assert_eq!(sign_in.state, State::ApiKey { prefill: None });
        assert_eq!(sign_in.error, Some(Error::SaveFailed("denied".into())));
        run.intent(&mut sign_in, Intent::SaveApiKey("sk-test".into()));
        run.result(&mut sign_in, 2, Outcome::ApiKeySaved);
        assert_eq!(sign_in.state, State::ApiKeyConfigured);
        assert!(sign_in.done());
    }

    /// A failed ChatGPT login (`on_account_login_completed` with an error)
    /// returns to the menu with the server's message; one for another login
    /// is not this attempt's.
    #[test]
    fn failed_login_completion_returns_to_the_menu() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::Browser {
            login_id: "login-1".into(),
            url: "https://example.test".into(),
        };
        login_completed(&mut sign_in, Some("login-2"), true, None);
        assert!(matches!(sign_in.state, State::Browser { .. }));
        login_completed(&mut sign_in, None, true, None);
        assert!(matches!(sign_in.state, State::Browser { .. }));
        login_completed(&mut sign_in, Some("login-1"), false, Some("denied".into()));
        assert_eq!(sign_in.state, State::Pick);
        assert_eq!(sign_in.error, Some(Error::Server("denied".into())));
    }

    /// The success message waits for Enter; a provider chosen hands the
    /// step to the provider step, and Esc there comes back.
    #[test]
    fn success_message_and_provider_choice() {
        let mut sign_in = forced_chatgpt();
        sign_in.state = State::SuccessMessage;
        assert!(!sign_in.done());
        Run::new().intent(&mut sign_in, Intent::Continue);
        assert_eq!(sign_in.state, State::Success);
        assert!(sign_in.done());

        let mut sign_in = SignIn::new(Offer {
            providers: Providers {
                bedrock: true,
                ..Providers::default()
            },
            ..offer()
        });
        let mut run = Run::new();
        run.intent(&mut sign_in, Intent::Choose(Method::Provider(Provider::Gcp)));
        assert_eq!(sign_in.state, State::Pick);
        run.intent(
            &mut sign_in,
            Intent::Choose(Method::Provider(Provider::Bedrock)),
        );
        assert_eq!(sign_in.state, State::Provider(Provider::Bedrock));
        assert!(sign_in.done());
        run.intent(&mut sign_in, Intent::Cancel);
        assert_eq!(sign_in.state, State::Pick);
    }
}
