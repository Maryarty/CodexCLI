//! The gateway's device code at the end of the launch (oracle
//! `onboarding_screen.rs`: `run_gateway_device_login`, `Step::GatewayDevice`,
//! `render_gateway_device_prompt`): the link and the one-time code, shown on
//! a screen of their own (no welcome header) until the attempt ends.
//!
//! In Flutter: a `StatelessWidget` over the model's code, as a `Focus` whose
//! `Shortcuts` (Esc, Ctrl+C, Ctrl+D, `q`) say "cancel" and whose cancel line
//! is a `GestureDetector`; cancelling is an intent to core
//! (`Intent::Exit`: the attempt is cancelled on the server, the app ends).
//! The attempt's end comes from the model, not from here.

use rtwk::focus::Focus;
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::DeviceCode;

use super::signin::provider::strings as provider_strings;
use super::signin::{Look, device_lines, strings as signin_strings, tapping};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;

/// The gateway's device code on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct GatewayDevice {
    pub code: DeviceCode,
}

/// Widget state: none (the quit is an intent, the code is the model's).
#[derive(Debug)]
pub struct GatewayDeviceState;

impl StatefulWidget for GatewayDevice {
    type State = GatewayDeviceState;

    fn create_state(&self) -> GatewayDeviceState {
        GatewayDeviceState
    }
}

impl_widget!(stateful eq GatewayDevice);

impl State for GatewayDeviceState {
    type Widget = GatewayDevice;
    type Msg = ();

    fn update(&mut self, (): (), _: &GatewayDevice, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
        Rebuild::No
    }

    fn build(&self, w: &GatewayDevice, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = signin_strings::of(cx);
        let gateway = provider_strings::of(cx);
        let look = Look::of(&Tokens::of(cx));
        let lines = device_lines(
            strings,
            &look,
            gateway.gateway_device_heading,
            gateway.gateway_device_steps,
            Some(&w.code),
        );
        // The cancel line is the last one: a tap there cancels, as Esc does.
        let cancel = lines.len().saturating_sub(1);
        let mut focus = Focus::new(tapping(lines, Some((cancel, cx.map(|_| ()))))).autofocus();
        for id in [fixed::DISMISS, fixed::PICK_LEAVE, fixed::PICK_QUIT] {
            focus = bind_fixed(focus, id, cx.send(()));
        }
        focus.into()
    }
}
