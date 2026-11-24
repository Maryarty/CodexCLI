//! `pets` (S24, pets overlay): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/pets/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::app::Command as AppCommand;
use crate::app::effects::{Ask, ConfigEdit, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "pets";
pub const ID: &str = "S24";
pub const KIND: Kind = Kind::Sheet;

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

/// What the `/pets` picker lists (`pets/picker.rs`: `build_pet_picker_params`): the host's
/// pets and the configured one, and the preview the model asked the host for. Where the cursor
/// is and what was searched are the sheet's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picker {
    /// Disabled row first, the rest by name.
    pub pets: Vec<Pet>,
    /// The pet in the config, if any.
    pub current: Option<String>,
    /// The pet whose preview is shown, and its state.
    pub preview: Option<(String, Preview)>,
}

impl Picker {
    /// The picker for `pets` (the built-in and custom ones, without the
    /// disable row) with `current` configured.
    pub fn build(current: Option<&str>, mut pets: Vec<Pet>) -> Self {
        pets.sort_by(|left, right| left.name.cmp(&right.name));
        pets.insert(
            0,
            Pet {
                selector: DISABLED.to_string(),
                name: String::new(),
                description: None,
                legacy_selector: None,
            },
        );
        Self {
            pets,
            current: current.map(str::to_owned),
            preview: None,
        }
    }

    /// The row the picker opens on, as an index into `pets`: the configured
    /// pet (or the default one when none is), else the first row.
    pub fn initial(&self) -> usize {
        let preferred = self.current.as_deref().unwrap_or(DEFAULT);
        self.pets
            .iter()
            .position(|pet| pet.selector == preferred || pet.legacy_selector.as_deref() == Some(preferred))
            .unwrap_or(0)
    }

    /// The rows shown, as indices into `pets`: those whose search value
    /// (the selector; the disable row answers to its own words) has the
    /// query in it.
    pub fn visible(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.pets
            .iter()
            .enumerate()
            .filter(|(_, pet)| {
                let value = if pet.selector == DISABLED {
                    "disable disabled hide hidden off none"
                } else {
                    pet.selector.as_str()
                };
                value.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// The row is the configured pet.
    pub fn is_current(&self, pet: &Pet) -> bool {
        self.current.as_deref().is_some_and(|current| {
            current == pet.selector || pet.legacy_selector.as_deref() == Some(current)
        })
    }

    /// Asks for the preview of a pet (`on_selection_changed`): its selector, if the preview
    /// shown is another one.
    fn load_preview(&mut self, selector: &str) -> Option<String> {
        if self.preview.as_ref().map(|(shown, _)| shown.as_str()) == Some(selector) {
            return None;
        }
        self.preview = Some((selector.to_owned(), Preview::Loading));
        Some(selector.to_owned())
    }
}

/// The bit of the request numbers of the config writes a pick makes (the other screens' are
/// listed in `core/src/app/effects.rs`: `1 << 61` is the worktree menus', `1 << 62` the review's).
pub const REQUEST_BIT: u64 = 1 << 60;

/// `request` is the number of a pick's config write.
pub fn owns(request: u64) -> bool {
    request & REQUEST_BIT != 0
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The pets the host listed (the picker's data, not whether a picker is up).
    pub catalog: Option<Picker>,
    /// `PetPreviewRequested`, in order.
    pub previews: Vec<String>,
    /// The picks being written to the config (`tui.pet`), by request number: the selector the
    /// picker shows as configured once the host confirms.
    saving: std::collections::BTreeMap<u64, String>,
    /// How many writes were asked: the numbers of their requests.
    stamp: u64,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The host listed the pets (and the configured one).
    Listed(Picker),
    /// The preview of a pet finished loading.
    PreviewReady(String),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The picker wants the preview of the pet with this selector.
    LoadPreview(String),
    /// The picker was left: the preview is not wanted any more.
    ReleasePreview,
    /// Enter on the row with this selector.
    Pick(String),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Listed(mut picker) => {
            picker.preview = None;
            model.catalog = Some(picker);
        }
        Ev::PreviewReady(selector) => {
            if let Some(picker) = &mut model.catalog
                && let Some((shown, state)) = &mut picker.preview
                && *shown == selector
            {
                *state = Preview::Ready;
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::LoadPreview(selector) => {
            if let Some(picker) = &mut model.catalog
                && let Some(selector) = picker.load_preview(&selector)
            {
                model.previews.push(selector);
            }
        }
        Intent::ReleasePreview => {
            if let Some(picker) = &mut model.catalog {
                picker.preview = None;
            }
        }
        Intent::Pick(selector) => {
            // `PetSelected` / `PetDisabled` (oracle `app/pets.rs`): `tui.pet` is written; the
            // picker's configured pet follows once the host confirms.
            model.stamp += 1;
            let request = REQUEST_BIT | model.stamp;
            let edit = ConfigEdit {
                key_path: "tui.pet".to_owned(),
                value: serde_json::Value::String(selector.clone()),
            };
            model.saving.insert(request, selector);
            fx.commands.push(AppCommand::Effect {
                request,
                ask: Ask::ConfigWrite { edits: vec![edit] },
            });
            if let Some(picker) = &mut model.catalog {
                picker.preview = None;
            }
        }
    }
}

/// The host answered the config write of a pick (`request` is `owns`): the picker shows the pet
/// as configured, or the failure is returned worded as the oracle words it (`handle_pet_disabled`,
/// `handle_pet_selection_loaded`), for the conversation's error line.
pub fn written(model: &mut Model, request: u64, done: &Done) -> Option<String> {
    let selector = model.saving.remove(&request)?;
    match done {
        Done::Config(Ok(())) => {
            if let Some(picker) = &mut model.catalog {
                picker.current = Some(selector);
            }
            None
        }
        Done::Config(Err(error)) if selector == DISABLED => {
            Some(format!("Failed to disable pets: {error}"))
        }
        Done::Config(Err(error)) => Some(format!("Failed to save pet selection: {error}")),
        _ => None,
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

/// The seed form of this screen's events: `{"kind":"picker","current":..,
/// "pets":[{"selector":..,"name":..,"description":..,"legacy_selector":..}]}`
/// (the built-in and custom pets; the disable row is the picker's own).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Picker {
        #[serde(default)]
        current: Option<String>,
        pets: Vec<Pet>,
    },
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    let SeedEv::Picker { current, pets } = seed;
    Ok(Ev::Listed(Picker::build(current.as_deref(), pets)))
}
