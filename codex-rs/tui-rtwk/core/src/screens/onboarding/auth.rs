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

#[path = "provider/mod.rs"]
pub mod provider;

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
    /// A provider's setup is chosen: the provider step (S03) runs it
    /// (`SignIn::wizard`).
    Provider(Provider),
    /// A provider's setup is written (`BedrockConfigured`, `GcpConfigured`).
    ProviderConfigured(Provider),
    /// The gateway with this id is the default provider
    /// (`SignInState::GatewaySelected`): what the host signs in to next.
    GatewaySelected(String),
    /// The gateway `provider_id`'s own sign-in runs on the app server of
    /// the selected provider (`SignInState::GatewayOAuth`): `url` is the
    /// page it opens in the browser, once known.
    GatewayOAuth {
        provider_id: String,
        url: Option<String>,
    },
    /// The gateway's sign-in failed; Enter retries it.
    GatewayOAuthFailed {
        provider_id: String,
        message: String,
    },
    /// The gateway is signed in (`SignInState::GatewayConfigured`).
    GatewayConfigured,
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
    /// The provider wizard the sign-in is in (`State::Provider`); none for a
    /// provider whose wizard is not built.
    pub wizard: Option<provider::Wizard>,
}

/// What the host is asked to log in with (`LoginAccountParams`).
#[derive(Clone, PartialEq, Eq)]
pub enum Login {
    ChatGpt,
    DeviceCode,
    ApiKey(String),
    /// A provider wizard's setup: written by the app server.
    Provider(provider::Setup),
}

impl std::fmt::Debug for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Login::ChatGpt => f.write_str("ChatGpt"),
            Login::DeviceCode => f.write_str("DeviceCode"),
            // The key stays out of logs.
            Login::ApiKey(_) => f.write_str("ApiKey(..)"),
            // Its fields hold paths and names, no secrets for GCP.
            Login::Provider(setup) => write!(f, "Provider({setup:?})"),
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
    /// A provider setup was written, or refused with a message.
    Provider(provider::Outcome),
}

/// The sign-in intents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A way to sign in, from the menu.
    UseMethod(Method),
    /// The menu changed ("another way" opened or left): the error goes.
    Menu,
    /// Esc on an attempt: cancel it, back to the menu.
    Cancel,
    /// Enter on the API key entry, with the text typed.
    SaveApiKey(String),
    /// Enter on the success message.
    Continue,
    /// A key of the provider wizard.
    Provider(provider::Intent),
    /// Esc in the provider wizard: one stage back.
    ProviderBack,
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
            State::Success
                | State::ApiKeyConfigured
                | State::Provider(_)
                | State::ProviderConfigured(_)
                | State::GatewaySelected(_)
                | State::GatewayConfigured
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
            || self
                .wizard
                .as_ref()
                .is_some_and(provider::Wizard::is_text_entry)
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
        Intent::UseMethod(method) => choose(sign_in, method, mint, asks),
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
        Intent::Provider(intent) => {
            let Some(wizard) = sign_in.wizard.as_mut() else {
                return;
            };
            let effect = wizard.apply(intent, mint);
            wizard_effect(sign_in, effect, asks);
        }
        Intent::ProviderBack => {
            let Some(wizard) = sign_in.wizard.as_mut() else {
                return;
            };
            let effect = wizard.back();
            // Esc leaves what the wizard waited for: its answer is dropped
            // (the wizard's view no longer holds the number) and the menu
            // takes choices again; a wizard that holds Esc back while its
            // request is out (the oracle's `Configuring` ignores it) still
            // takes the answer.
            let held = effect == provider::Effect::None && wizard.waits();
            if !held && matches!(sign_in.awaiting, Some((_, Login::Provider(_)))) {
                sign_in.awaiting = None;
            }
            wizard_effect(sign_in, effect, asks);
        }
    }
}

/// What a wizard's intent asks of the step: leave to the menu, or run its
/// setup (`spawn_provider_setup`); a key of the wizard clears the error.
fn wizard_effect(sign_in: &mut SignIn, effect: provider::Effect, asks: &mut Vec<Ask>) {
    match effect {
        provider::Effect::None => {}
        provider::Effect::Leave => {
            sign_in.wizard = None;
            sign_in.state = State::Pick;
            sign_in.error = None;
        }
        provider::Effect::Ask(request, setup) => {
            let login = Login::Provider(setup);
            sign_in.awaiting = Some((request, login.clone()));
            asks.push(Ask::Login { request, login });
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
    // A login asked and not answered yet keeps the state on the menu: a
    // second activation within the round trip starts nothing (an Enter
    // pressed twice must not start two logins).
    if sign_in.state != State::Pick || sign_in.awaiting.is_some() {
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
                let (wizard, effect) = provider::Wizard::of(provider, mint);
                sign_in.wizard = Some(wizard);
                sign_in.state = State::Provider(provider);
                wizard_effect(sign_in, effect, asks);
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
        // The key typed is forgotten with the attempt; the answer of a save
        // still out is dropped (`apply_result`).
        State::ApiKey { .. } => sign_in.awaiting = None,
        State::Provider(_) => sign_in.wizard = None,
        _ => return,
    }
    sign_in.state = State::Pick;
    sign_in.error = None;
}

/// The answer of a login ask. One no longer awaited is dropped; a device
/// code or browser login that came for an attempt left is cancelled.
pub fn apply_result(
    sign_in: &mut SignIn,
    result: AuthResult,
    mint: &mut dyn FnMut() -> Request,
    asks: &mut Vec<Ask>,
) {
    let AuthResult { request, outcome } = result;
    let device =
        matches!(sign_in.state, State::DeviceCode { request: r, code: None } if r == request);
    let awaited = sign_in
        .awaiting
        .as_ref()
        .is_some_and(|(r, _)| *r == request);
    if !device && !awaited {
        // A login started for an attempt no longer awaited is cancelled
        // (the browser one holds a local callback server).
        match outcome {
            Outcome::DeviceCode(code) => asks.push(Ask::CancelLogin {
                login_id: code.login_id,
            }),
            Outcome::Browser { login_id, .. } => asks.push(Ask::CancelLogin { login_id }),
            Outcome::ApiKeySaved | Outcome::Failed(_) | Outcome::Provider(_) => {}
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
        // `spawn_provider_setup`: written, or back where it can retry.
        (Some(Login::Provider(_)), Outcome::Provider(result)) => {
            let State::Provider(provider) = sign_in.state else {
                return;
            };
            let Some(wizard) = sign_in.wizard.as_mut() else {
                return;
            };
            match wizard.answer(request, result, mint) {
                provider::Settled::Waiting => {}
                provider::Settled::Configured => {
                    sign_in.wizard = None;
                    sign_in.state = State::ProviderConfigured(provider);
                }
                provider::Settled::Selected(id) => {
                    sign_in.wizard = None;
                    sign_in.state = State::GatewaySelected(id);
                }
                provider::Settled::Ask(request, setup) => {
                    wizard_effect(sign_in, provider::Effect::Ask(request, setup), asks);
                }
            }
        }
        (Some(Login::Provider(_)), other) => {
            sign_in.error = Some(Error::Unexpected(format!("{other:?}")));
        }
        (None, _) => {}
    }
}

/// Stop awaiting credentials or a browser/device attempt from the replaced server.
pub(super) fn transport_lost(sign_in: &mut SignIn) {
    let awaiting = sign_in.awaiting.take().is_some();
    match sign_in.state {
        State::Browser { .. } | State::DeviceCode { .. } => sign_in.state = State::Pick,
        State::Provider(_) if awaiting => {
            sign_in.wizard = None;
            sign_in.state = State::Pick;
        }
        _ => {}
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
