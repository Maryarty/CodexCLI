//! The selected gateway's own sign-in, which runs once the steps of the
//! startup page are over (the oracle's `authenticate_gateway`, run by
//! `lib.rs` after `OnboardingScreen` returned `selected_gateway`): the
//! configuration is read again and the embedded server started with it, the
//! primary login (ChatGPT or an API key) runs first when the account needs
//! it, then the gateway's OAuth when the provider asks for it. The states
//! the user sees are the sign-in step's (`State::GatewayOAuth`,
//! `GatewayOAuthFailed`, `GatewayConfigured`, the ChatGPT and API key ones);
//! this module only sequences them and holds the requests awaited.
//!
//! IO is the host's, through `Ask::{SelectGateway, GatewayLogins,
//! GatewayOAuth, CancelGatewayOAuth}`; every answer carries its request and
//! one no longer awaited is dropped.

use super::{Ask, Model, Request, ask, auth};
use crate::app::AppFx;

/// Where the gateway's sign-in stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Not started: no gateway was selected, or the steps are not over.
    #[default]
    Idle,
    /// The configuration is read again and the server started with the
    /// selected gateway (`Ask::SelectGateway`).
    Starting { request: Request, id: String },
    /// The account and the provider are read on that server
    /// (`Ask::GatewayLogins`).
    Reading { request: Request, id: String },
    /// The primary login runs on the sign-in step's own states; the gateway's
    /// OAuth follows when `oauth`.
    Primary { id: String, oauth: bool },
    /// The gateway's OAuth runs (or failed and waits for Enter).
    OAuth { request: Request, id: String },
    /// Everything the gateway needed is done.
    Done,
}

impl Phase {
    /// The sign-in step is not over: it shows the login being run.
    pub fn active(&self) -> bool {
        !matches!(self, Phase::Idle | Phase::Done)
    }
}

/// What the host read on the server of the selected gateway
/// (`authenticate_gateway`'s `login_status`, `should_show_login_screen`, and
/// `config.model_provider.gateway_oauth`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Logins {
    /// The account needs the primary login first.
    pub primary: bool,
    /// The provider signs in through gateway OAuth.
    pub oauth: bool,
    /// What the primary login offers (the methods allowed, no wizards).
    pub offer: auth::Offer,
}

/// The gateway selected in the wizard, if the sign-in ended on it.
fn selected(model: &Model) -> Option<String> {
    match &model.sign_in.as_ref()?.state {
        auth::State::GatewaySelected(id) => Some(id.clone()),
        _ => None,
    }
}

/// After every change: starts the sign-in once the steps are over, and goes
/// on when the primary login is finished.
pub(super) fn follow(model: &mut Model, fx: &mut AppFx) {
    if model.exit || model.ends() {
        return;
    }
    match model.gateway.clone() {
        Phase::Idle => {
            let Some(id) = selected(model) else {
                return;
            };
            if !model.read || model.steps().into_iter().any(|step| !model.done(step)) {
                return;
            }
            let request = fx.host_request_number();
            model.gateway = Phase::Starting {
                request,
                id: id.clone(),
            };
            ask(
                fx,
                Ask::SelectGateway {
                    request,
                    provider_id: id,
                },
            );
        }
        Phase::Primary { id, oauth } => {
            if model.sign_in.as_ref().is_some_and(auth::SignIn::done) {
                next(model, id, oauth, fx);
            }
        }
        Phase::Starting { .. } | Phase::Reading { .. } | Phase::OAuth { .. } | Phase::Done => {}
    }
}

/// `Ev::GatewayStarted`: the server runs with the selected gateway; its
/// account and provider are read next.
pub(super) fn started(model: &mut Model, request: Request, fx: &mut AppFx) {
    let Phase::Starting {
        request: awaited,
        id,
    } = model.gateway.clone()
    else {
        return;
    };
    if awaited != request {
        return;
    }
    read_logins(model, id, fx);
}

fn read_logins(model: &mut Model, id: String, fx: &mut AppFx) {
    let request = fx.host_request_number();
    model.gateway = Phase::Reading {
        request,
        id: id.clone(),
    };
    ask(
        fx,
        Ask::GatewayLogins {
            request,
            provider_id: id,
        },
    );
}

/// `Ev::GatewayLogins`: the logins the gateway needs, in the oracle's order
/// (the primary one first).
pub(super) fn logins(model: &mut Model, request: Request, logins: Logins, fx: &mut AppFx) {
    let Phase::Reading {
        request: awaited,
        id,
    } = model.gateway.clone()
    else {
        return;
    };
    if awaited != request {
        return;
    }
    if !logins.primary {
        next(model, id, logins.oauth, fx);
        return;
    }
    let Some(sign_in) = model.sign_in.as_mut() else {
        return;
    };
    // A new `OnboardingScreen` with the sign-in step only: `Login::Primary`
    // starts ChatGPT when it is allowed, else the API key entry.
    let method = if logins.offer.chatgpt {
        auth::Method::ChatGpt
    } else {
        auth::Method::ApiKey
    };
    sign_in.offer = logins.offer;
    sign_in.state = auth::State::Pick;
    sign_in.error = None;
    sign_in.chatgpt_account = false;
    sign_in.wizard = None;
    let mut asks = Vec::new();
    auth::apply_intent(
        sign_in,
        auth::Intent::UseMethod(method),
        &mut || fx.host_request_number(),
        &mut asks,
    );
    model.gateway = Phase::Primary {
        id,
        oauth: logins.oauth,
    };
    asks.into_iter().for_each(|a| ask(fx, a));
}

/// The primary login is over: the gateway's OAuth, or the end.
fn next(model: &mut Model, id: String, oauth: bool, fx: &mut AppFx) {
    if oauth {
        start_oauth(model, id, fx);
    } else {
        model.gateway = Phase::Done;
    }
}

/// `start_gateway_oauth`: "Starting gateway sign-in…" until the host
/// answers.
fn start_oauth(model: &mut Model, id: String, fx: &mut AppFx) {
    let Some(sign_in) = model.sign_in.as_mut() else {
        return;
    };
    sign_in.error = None;
    sign_in.state = auth::State::GatewayOAuth {
        provider_id: id.clone(),
        url: None,
    };
    let request = fx.host_request_number();
    model.gateway = Phase::OAuth {
        request,
        id: id.clone(),
    };
    ask(
        fx,
        Ask::GatewayOAuth {
            request,
            provider_id: id,
        },
    );
}

/// `Ev::GatewayOAuth`: the gateway is signed in, or the failure with Enter
/// to retry.
pub(super) fn oauth(model: &mut Model, request: Request, result: Result<(), String>) {
    let Phase::OAuth {
        request: awaited,
        id,
    } = model.gateway.clone()
    else {
        return;
    };
    if awaited != request {
        return;
    }
    let Some(sign_in) = model.sign_in.as_mut() else {
        return;
    };
    match result {
        Ok(()) => {
            sign_in.error = None;
            sign_in.state = auth::State::GatewayConfigured;
            model.gateway = Phase::Done;
        }
        Err(message) => {
            sign_in.state = auth::State::GatewayOAuthFailed {
                provider_id: id,
                message,
            };
        }
    }
}

/// Enter on the failed state: the sign-in starts again.
pub(super) fn retry(model: &mut Model, fx: &mut AppFx) {
    if !matches!(model.gateway, Phase::OAuth { .. }) {
        return;
    }
    let Some(auth::State::GatewayOAuthFailed { provider_id, .. }) =
        model.sign_in.as_ref().map(|s| s.state.clone())
    else {
        return;
    };
    start_oauth(model, provider_id, fx);
}

/// `on_gateway_oauth_changed`: the page the browser opens, for the attempt
/// shown; once for each address.
pub(super) fn changed(
    model: &mut Model,
    provider_id: &str,
    auth_url: Option<String>,
    fx: &mut AppFx,
) {
    let Some(auth_url) = auth_url else {
        return;
    };
    let Some(sign_in) = model.sign_in.as_mut() else {
        return;
    };
    if let auth::State::GatewayOAuth {
        provider_id: shown,
        url,
    } = &mut sign_in.state
        && shown == provider_id
        && url.as_ref() != Some(&auth_url)
    {
        *url = Some(auth_url.clone());
        ask(fx, Ask::OpenBrowser { url: auth_url });
    }
}

/// Quitting from the gateway's OAuth cancels it on the server
/// (`authenticate_gateway`'s `GatewayOAuthCancel`).
pub(super) fn exit(model: &Model, fx: &mut AppFx) {
    if matches!(model.gateway, Phase::OAuth { .. }) {
        ask(fx, Ask::CancelGatewayOAuth);
    }
}

/// The connection was lost: what the host was asked for will not answer, the
/// start is asked again with the next connection.
pub(super) fn lost(model: &mut Model) {
    match model.gateway {
        Phase::Starting { .. } | Phase::Reading { .. } => model.gateway = Phase::Idle,
        Phase::OAuth { request, .. } => {
            oauth(model, request, Err(crate::app::CONNECTION_LOST.to_owned()));
        }
        _ => {}
    }
}

pub(super) fn transport_replaced(model: &mut Model, fx: &mut AppFx) {
    match &model.gateway {
        Phase::Reading { id, .. } => read_logins(model, id.clone(), fx),
        Phase::OAuth { request, .. } => {
            oauth(model, *request, Err(crate::app::CONNECTION_LOST.to_owned()));
        }
        _ => {}
    }
}
