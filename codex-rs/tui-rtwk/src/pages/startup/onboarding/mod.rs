//! `onboarding`: the view half of the slot of S02-S04 (onboarding routes), the `mock-pre` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::onboarding`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::onboarding::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;

pub mod strings;

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

/// The steps of the startup page (architecture §1, amendment 2026-11-19b):
/// sign-in, trust, model migration, import.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    SignIn,
    Trust,
    Migration,
    Import,
}

impl Step {
    /// The position of the step in the page's sequence.
    pub fn index(self) -> usize {
        match self {
            Step::SignIn => 0,
            Step::Trust => 1,
            Step::Migration => 2,
            Step::Import => 3,
        }
    }
}

/// The startup page: one page whose body is the step the controller (the
/// page's state) is on. Pushed as a route by whoever opens it, on the step
/// it opens at.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupPage {
    pub step: Step,
}

/// The page's step controller (R14: widget state only).
#[derive(Debug)]
pub struct StartupPageState {
    step: Step,
}

impl StatefulWidget for StartupPage {
    type State = StartupPageState;

    fn create_state(&self) -> StartupPageState {
        StartupPageState { step: self.step }
    }
}

impl_widget!(stateful eq StartupPage);

impl State for StartupPageState {
    type Widget = StartupPage;
    type Msg = ();

    fn build(&self, _: &StartupPage, _: &mut Cx<'_, Self>) -> AnyWidget {
        match self.step {
            Step::Import => crate::pages::startup::import::Screen.into(),
            Step::SignIn | Step::Trust | Step::Migration => Screen.into(),
        }
    }
}

/// The route of the startup page on `step`, pushed by the widget that opens
/// it (`Navigator::of(cx).push`).
pub fn route(step: Step) -> PageRoute<()> {
    PageRoute::new(move |_| StartupPage { step }.into())
}

/// Opens the empty form of this slot (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| StartupPage { step: Step::SignIn }.into())
}

/// The empty screen widget of this slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids, drafts and controllers only (R14).
#[derive(Debug)]
pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let shared = crate::l10n::strings::of(cx);
        Text::new(shared.empty_slot(NAME, ID).to_string()).into()
    }
}
