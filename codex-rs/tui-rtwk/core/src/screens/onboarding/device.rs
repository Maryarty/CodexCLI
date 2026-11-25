//! The gateway's device code, at the end of the launch (the oracle's
//! `device_login::start_if_needed` and `run_gateway_device_login`, run by
//! `lib.rs` after the onboarding screen and before it reads the models): the
//! server of the session is asked whether the gateway it will use needs a
//! sign-in and a device code is the way; when it does, the page shows the
//! link and the one-time code until `account/gatewayOAuth/changed` says that
//! attempt ended.
//!
//! IO is the host's, through `Ask::GatewayDevice` (the reads and the login)
//! and `Ask::FailLaunch` (the oracle ends the launch with the error); the
//! changes come in as `Ev::GatewayDeviceChanged`. A change that arrives
//! before the login's answer is kept and read with it
//! (`login_success_is_not_lost_while_waiting_for_the_device_code`).

use super::auth::DeviceCode;
use super::{Ask, Model, Request, ask, mint};
use crate::app::AppFx;

/// What `account/gatewayOAuth/changed` said of an attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// The sign-in is done.
    Succeeded,
    /// It failed (or the provider is not ready): the server's message.
    Failed(String),
}

/// Where the device code stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Not asked yet.
    #[default]
    Idle,
    /// The host reads the provider and asks for the code; the changes that
    /// came meanwhile are kept.
    Asking {
        request: Request,
        seen: Vec<(String, Change)>,
    },
    /// The code is on screen; the attempt's end is awaited.
    Waiting(DeviceCode),
    /// The attempt failed (the launch ends with the message, asked of the
    /// host) or was cancelled on the way out: nothing more is asked, the code
    /// stays until the app ends.
    Failed(DeviceCode),
    /// Nothing (more) to sign in to: the launch goes on.
    Done,
}

impl Phase {
    /// The code to show.
    pub fn code(&self) -> Option<&DeviceCode> {
        match self {
            Phase::Waiting(code) | Phase::Failed(code) => Some(code),
            Phase::Idle | Phase::Asking { .. } | Phase::Done => None,
        }
    }

    /// The launch waits: the code is asked for or on screen.
    pub fn blocks(&self) -> bool {
        !matches!(self, Phase::Idle | Phase::Done)
    }
}

/// After every change, once the steps are over: asks for the code, once;
/// whether the launch may go on.
pub(super) fn follow(model: &mut Model, fx: &mut AppFx) -> bool {
    match model.device {
        Phase::Idle => {
            // The oracle reads a resumed thread first: its provider decides.
            let thread = match super::session::provider(model) {
                super::session::Provider::Wait => return false,
                super::session::Provider::Ask(thread) => thread,
            };
            let request = mint(model);
            model.device = Phase::Asking {
                request,
                seen: Vec::new(),
            };
            ask(fx, Ask::GatewayDevice { request, thread });
            false
        }
        Phase::Done => true,
        Phase::Asking { .. } | Phase::Waiting(_) | Phase::Failed(_) => false,
    }
}

/// `Ev::GatewayDevice`: the code to show, or none when startup goes on
/// without a prompt.
pub(super) fn answered(
    model: &mut Model,
    request: Request,
    code: Option<DeviceCode>,
    fx: &mut AppFx,
) {
    let Phase::Asking {
        request: awaited,
        seen,
    } = model.device.clone()
    else {
        return;
    };
    if awaited != request {
        return;
    }
    let Some(code) = code else {
        model.device = Phase::Done;
        return;
    };
    let ended = seen
        .into_iter()
        .find(|(login_id, _)| *login_id == code.login_id)
        .map(|(_, change)| change);
    model.device = Phase::Waiting(code);
    if let Some(change) = ended {
        settle(model, change, fx);
    }
}

/// `handle_event_during_device_login`: the attempt shown ended.
pub(super) fn changed(model: &mut Model, login_id: String, change: Change, fx: &mut AppFx) {
    match &mut model.device {
        Phase::Asking { seen, .. } => seen.push((login_id, change)),
        Phase::Waiting(code) if code.login_id == login_id => settle(model, change, fx),
        _ => {}
    }
}

/// The attempt on screen ended: the launch goes on, or ends with the error.
fn settle(model: &mut Model, change: Change, fx: &mut AppFx) {
    let Phase::Waiting(code) = model.device.clone() else {
        return;
    };
    match change {
        Change::Succeeded => model.device = Phase::Done,
        Change::Failed(message) => {
            model.device = Phase::Failed(code);
            ask(fx, Ask::FailLaunch { message });
        }
    }
}

/// Quitting with the code on screen cancels the attempt
/// (`run_gateway_device_login`: `device_login::cancel`).
pub(super) fn exit(model: &mut Model, fx: &mut AppFx) {
    if let Phase::Waiting(code) = model.device.clone() {
        model.device = Phase::Failed(code);
        ask(fx, Ask::CancelGatewayOAuth);
    }
}

/// The connection was lost: what the host was asked for will not answer, and
/// the code of an attempt on that connection is not waited for; the start is
/// asked again with the next connection.
pub(super) fn lost(model: &mut Model) {
    if matches!(model.device, Phase::Asking { .. } | Phase::Waiting(_)) {
        model.device = Phase::Idle;
    }
}
