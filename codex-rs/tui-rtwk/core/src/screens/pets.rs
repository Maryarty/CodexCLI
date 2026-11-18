//! `pets` (S24, pets overlay): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/pets/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "pets";
pub const ID: &str = "S24";
pub const KIND: Kind = Kind::Part;

/// The pet that means "no pet" (`pets::DISABLED_PET_ID`).
pub const DISABLED: &str = "disabled";
/// The pet the picker preselects when none is configured (`DEFAULT_PET_ID`).
pub const DEFAULT: &str = "codex";

/// One row of the picker (`PetPickerEntry`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Pet {
    pub selector: String,
    /// The name shown; empty for the "disable" row, which the view words.
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// The id older configs stored for a custom pet.
    #[serde(default)]
    pub legacy_selector: Option<String>,
}

/// What the preview beside the list is doing (`PetPickerPreviewStatus`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    Loading,
    Ready,
}

/// The `/pets` picker (`pets/picker.rs`: `build_pet_picker_params`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picker {
    /// Disabled row first, the rest by name.
    pub pets: Vec<Pet>,
    /// The pet in the config, if any.
    pub current: Option<String>,
    pub cursor: usize,
    pub query: String,
    /// The pet whose preview is shown, and its state.
    pub preview: Option<(String, Preview)>,
}

impl Picker {
    /// The picker for `pets` (the built-in and custom ones, without the
    /// disable row) with `current` configured.
    pub fn build(current: Option<&str>, pets: Vec<Pet>) -> Self {
        let _ = (current, &pets);
        Self {
            pets,
            current: current.map(str::to_owned),
            cursor: 0,
            query: String::new(),
            preview: None,
        }
    }

    /// The rows shown, as indices into `pets`.
    pub fn visible(&self) -> Vec<usize> {
        (0..self.pets.len()).collect()
    }

    /// The row is the configured pet.
    pub fn is_current(&self, pet: &Pet) -> bool {
        let _ = pet;
        false
    }
}

/// What the picker's owner was asked (`AppEvent`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// `PetSelected`.
    Selected(String),
    /// `PetDisabled`.
    Disabled,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub open: Option<Picker>,
    /// `PetPreviewRequested`, in order.
    pub previews: Vec<String>,
    pub chosen: Vec<Choice>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Opened(Picker),
    /// The preview of a pet finished loading.
    PreviewReady(String),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Up / Down; wraps.
    Move(i32),
    Search(String),
    /// Enter.
    Pick,
    /// Esc: clears the search, then closes.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = (model, fx);
    match ev {
        Ev::Opened(_) | Ev::PreviewReady(_) => {}
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {
        Intent::Move(_) | Intent::Search(_) | Intent::Pick | Intent::Close => {}
    }
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
    result: Result<Reply, RpcError>,
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
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let _ = value;
    Err(crate::screens::SeedError::NoSeeds)
}
