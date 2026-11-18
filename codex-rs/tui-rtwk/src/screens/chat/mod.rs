//! `chat`: the slot of S10 (main chat screen), the `mock-chat` task's to fill. It holds
//! the empty form the skeleton registers: a screen widget, its opener, its
//! sub-enums of the root event and intent enums, its model part, and the
//! debug-menu entries. Nothing outside this directory (bar its test file)
//! needs an edit when it grows.

mod home;

use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::threads::ThreadFx;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::Kind;

pub const NAME: &str = "chat";
pub const ID: &str = "S10";
pub const KIND: Kind = Kind::Home;
pub const TITLE: &str = "main chat screen";

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = (model, fx);
    match ev {}
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, String>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, String> {
    let _ = result;
    match *command {}
}

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

/// The `Navigator`'s home: the chat of the active thread.
pub fn home(app: &crate::app::CodexApp) -> AnyWidget {
    home::Chat {
        ui: app.thread_ui.get(app.session.active.as_ref()),
    }
    .into()
}

/// The route of this screen. The chat screen is the `Navigator`'s home; the
/// route exists for the debug menu and for tests.
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen.into())
}

/// Opens the empty form of this slot (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|| Screen.into())
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

    fn build(&self, _: &Screen, _cx: &mut Cx<'_, Self>) -> AnyWidget {
        Text::new(format!("{NAME} ({ID}): empty slot")).into()
    }
}
