//! The server's collaboration-mode catalog (`collaborationMode/list`) and the choice of a
//! mask out of it: the oracle's `collaboration_modes.rs`. The catalog is the app's, read at
//! connect and kept as the list the server named; a server that does not answer leaves it
//! empty (discovery is optional), and an empty catalog means no mode can be switched to.
//!
//! The mode a thread is in is its base settings (what Default mode runs with) with the
//! chosen mask over them (`CollaborationMode::apply_mask`): `effective` is that value, the
//! one the server is told in `thread/settings/update`.

use codex_app_server_protocol::{
    ClientRequest, CollaborationModeListParams, CollaborationModeListResponse, RequestId,
};
use codex_protocol::config_types::{CollaborationMode, ModeKind, Settings};
use codex_protocol::openai_models::ReasoningEffort;

use crate::ids::{BackendId, ThreadId, Ticket};
use crate::threads::queue::QueueOperation;

use super::models::{PlanSource, Selection};

pub use codex_protocol::config_types::CollaborationModeMask as Mask;

/// `collaborationMode/list`.
pub fn list_request(request_id: RequestId) -> ClientRequest {
    ClientRequest::CollaborationModeList {
        request_id,
        params: CollaborationModeListParams::default(),
    }
}

/// The masks a `collaborationMode/list` answer holds. The RPC omits the prompts, so the
/// mask clears any restored override and the server supplies its current instructions when
/// the mode is selected (`developer_instructions: Some(None)`).
pub fn masks(result: serde_json::Value) -> Result<Vec<Mask>, String> {
    serde_json::from_value::<CollaborationModeListResponse>(result)
        .map(|response| {
            response
                .data
                .into_iter()
                .map(|mask| Mask {
                    name: mask.name,
                    mode: mask.mode,
                    model: mask.model,
                    reasoning_effort: mask.reasoning_effort,
                    developer_instructions: Some(None),
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

/// The presets the TUI shows (`filtered_presets`): those of a visible mode kind.
pub fn visible(catalog: &[Mask]) -> impl Iterator<Item = &Mask> {
    catalog
        .iter()
        .filter(|mask| mask.mode.is_some_and(ModeKind::is_tui_visible))
}

/// The mask a thread starts in: the Default preset, else the first one.
pub fn default_mask(catalog: &[Mask]) -> Option<Mask> {
    visible(catalog)
        .find(|mask| mask.mode == Some(ModeKind::Default))
        .or_else(|| visible(catalog).next())
        .cloned()
}

/// The preset of `kind`, when that kind is one the TUI shows.
pub fn mask_for_kind(catalog: &[Mask], kind: ModeKind) -> Option<Mask> {
    if !kind.is_tui_visible() {
        return None;
    }
    visible(catalog)
        .find(|mask| mask.mode == Some(kind))
        .cloned()
}

/// The Default preset, none when the catalog has no Default (`default_mode_mask`).
pub fn default_mode_mask(catalog: &[Mask]) -> Option<Mask> {
    mask_for_kind(catalog, ModeKind::Default)
}

/// The Plan preset (`plan_mask`).
pub fn plan_mask(catalog: &[Mask]) -> Option<Mask> {
    mask_for_kind(catalog, ModeKind::Plan)
}

/// What can change the thread's mode settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Preset(Mask),
    PlanEffort {
        effort: Option<ReasoningEffort>,
        scope: ReasoningScope,
        operation: Option<QueueOperation>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReasoningScope {
    Session,
    PlanOnly,
    AllModes,
}

/// An accepted scope choice awaiting its backend's configuration write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanWrite {
    pub(crate) thread: ThreadId,
    pub(crate) backend: BackendId,
    pub(crate) issued: Ticket,
    pub(crate) selection: Selection,
    pub(crate) operation: Option<QueueOperation>,
}

/// The accepted Plan override or the catalog preset underneath it.
pub(crate) fn plan_source(
    override_effort: Option<&ReasoningEffort>,
    mask: Option<&Mask>,
) -> PlanSource {
    if let Some(effort) = override_effort {
        return PlanSource::Override(effort.clone());
    }
    mask.map_or(PlanSource::Unknown, |mask| {
        PlanSource::Default(mask.reasoning_effort.clone().flatten())
    })
}

/// The preset after the one of `current`, in list order (Shift+Tab); the first one when
/// `current` is not in the list, none for an empty list.
pub fn next_mask(catalog: &[Mask], current: Option<ModeKind>) -> Option<Mask> {
    let presets: Vec<&Mask> = visible(catalog).collect();
    if presets.is_empty() {
        return None;
    }
    let next = presets
        .iter()
        .position(|mask| mask.mode == current)
        .map_or(0, |at| (at + 1) % presets.len());
    presets.get(next).map(|mask| (*mask).clone())
}

/// What a thread runs with outside of any mask: the settings it was last seen with in
/// Default mode (`current_collaboration_mode`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Base {
    pub model: String,
    pub effort: Option<ReasoningEffort>,
}

impl Base {
    /// The base of a model and an effort as the server names them (the effort by its wire
    /// word; one this client does not know is kept as a custom effort).
    pub fn of(model: Option<&str>, effort: Option<&str>) -> Self {
        Base {
            model: model.unwrap_or_default().to_owned(),
            effort: effort.and_then(|effort| {
                serde_json::from_value(serde_json::Value::String(effort.to_owned())).ok()
            }),
        }
    }
}

/// What a switch of the mode changed of the model or the effort
/// (`Model changed to {model} {effort} for {mode} mode.`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelChange {
    pub model: String,
    /// The word after the model: `default` for no effort or `none`, else the effort's own;
    /// none for the automatic models (`codex-auto-*`), which say none.
    pub effort: Option<String>,
    pub mode: ModeKind,
}

/// The effort word of the line a switch says (`set_collaboration_mask`).
pub fn effort_label(model: &str, effort: Option<&ReasoningEffort>) -> Option<String> {
    if model.starts_with("codex-auto-") {
        return None;
    }
    Some(match effort {
        None | Some(ReasoningEffort::None) => "default".to_owned(),
        Some(effort) => effort.as_str().to_owned(),
    })
}

/// The collaboration mode `mask` makes over `base` (`CollaborationMode::apply_mask` with the
/// Plan override of `plan_mode_reasoning_effort` set on the mask first,
/// `set_collaboration_mask`). The instructions are the mask's: the list clears them, so the
/// server supplies its own.
pub fn effective(
    base: &Base,
    mask: &Mask,
    plan_effort: Option<&ReasoningEffort>,
) -> CollaborationMode {
    let mut mask = mask.clone();
    if mask.mode == Some(ModeKind::Plan)
        && let Some(effort) = plan_effort
    {
        mask.reasoning_effort = Some(Some(effort.clone()));
    }
    CollaborationMode {
        mode: mask.mode.unwrap_or(ModeKind::Default),
        settings: Settings {
            model: mask.model.unwrap_or_else(|| base.model.clone()),
            reasoning_effort: mask.reasoning_effort.unwrap_or_else(|| base.effort.clone()),
            developer_instructions: mask.developer_instructions.unwrap_or(None),
        },
    }
}
