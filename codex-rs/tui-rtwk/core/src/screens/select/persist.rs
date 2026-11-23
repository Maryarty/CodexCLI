//! What a final pick of `/model` and `/permissions` stores: the settings of
//! the thread for the turns after it (`thread/settings/update`) and, unless
//! the pick is for the session only, the defaults in the user's config
//! (`config/batchWrite`). Values and the oracle's key names only; the words
//! of the history lines are the view's. Oracle: `config_update.rs`
//! (`build_model_selection_edits`), `app/model_defaults.rs`, `app/event_dispatch.rs`
//! (`PersistPlanModeReasoningEffort`).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::WriteStatus;

use super::models::{Apply, Effort, Selection};
use super::permissions;
use crate::threads::SettingsPatch;

/// One key of the config: a value, or `None` to clear it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub key: &'static str,
    pub value: Option<String>,
}

/// What a model pick changes: the thread's settings and the keys written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub patch: Option<SettingsPatch>,
    pub edits: Vec<Edit>,
}

/// `build_model_selection_edits`: the model and its effort (cleared when the
/// model's default is meant).
fn model_edits(model: &str, effort: Option<&Effort>) -> Vec<Edit> {
    vec![
        Edit { key: "model", value: Some(model.to_owned()) },
        Edit { key: "model_reasoning_effort", value: effort.map(ToString::to_string) },
    ]
}

/// The effects of a settled model and effort (`select_session_model`,
/// `persist_default_model_selection`, the plan scope's two answers).
pub fn effects(selection: &Selection) -> Effects {
    let Selection { model, effort, apply, .. } = selection;
    let patch = SettingsPatch {
        model: Some(model.clone()),
        effort: effort.clone(),
        ..SettingsPatch::default()
    };
    match apply {
        Apply::Selection | Apply::Advanced | Apply::AllModes => {
            let mut edits = model_edits(model, effort.as_ref());
            if matches!(apply, Apply::AllModes) {
                edits.push(Edit {
                    key: "plan_mode_reasoning_effort",
                    value: effort.as_ref().map(ToString::to_string),
                });
            }
            Effects { patch: Some(patch), edits }
        }
        Apply::Session => Effects { patch: Some(patch), edits: Vec::new() },
        // The Reserve keeps its model: only the effort of the task changes.
        Apply::Reserve => Effects {
            patch: Some(SettingsPatch { effort: effort.clone(), ..SettingsPatch::default() }),
            edits: Vec::new(),
        },
        // Plan mode only: the thread keeps its own model and effort.
        Apply::PlanOnly => Effects {
            patch: None,
            edits: vec![Edit {
                key: "plan_mode_reasoning_effort",
                value: effort.as_ref().map(ToString::to_string),
            }],
        },
    }
}

/// What a service-tier switch changes (`set_service_tier_selection`,
/// `build_service_tier_selection_edits`): the thread's tier for its next turns and the saved
/// default, where the fast tier is written as `fast`.
pub fn tier_effects(tier: &str) -> Effects {
    let saved = match tier {
        "priority" => "fast",
        other => other,
    };
    Effects {
        patch: Some(SettingsPatch {
            service_tier: Some(Some(tier.to_owned())),
            ..SettingsPatch::default()
        }),
        edits: vec![Edit { key: "service_tier", value: Some(saved.to_owned()) }],
    }
}

/// What a permission pick changes in the thread (`override_turn_context`
/// with the profile, the policy and the reviewer): the profile of a preset is
/// its built-in one; the session only, nothing is written to the config.
pub fn permissions_patch(selection: &permissions::Selection) -> SettingsPatch {
    match selection {
        permissions::Selection::Preset { preset, reviewer } => SettingsPatch {
            permissions: Some(preset.profile().to_owned()),
            approval_policy: Some(preset.approval()),
            approvals_reviewer: Some(*reviewer),
            ..SettingsPatch::default()
        },
        permissions::Selection::Profile { id, approval, reviewer, .. } => SettingsPatch {
            permissions: Some(id.clone()),
            approval_policy: *approval,
            approvals_reviewer: *reviewer,
            ..SettingsPatch::default()
        },
    }
}

/// `app_scoped_key_path`: `apps."<id>".<key>`, the id quoted as TOML needs it.
fn app_key(id: &str, key: &str) -> String {
    let quoted = serde_json::Value::String(id.to_owned()).to_string();
    ["apps", quoted.as_str(), key].join(".")
}

/// `SetAppEnabled`: an app switched on clears its `enabled` and `disabled_reason`
/// keys; one switched off writes `enabled = false` and `disabled_reason = "user"`.
pub fn app_enabled_edits(id: &str, enabled: bool) -> Vec<ConfigEdit> {
    let edit = |key: &str, value: serde_json::Value| ConfigEdit {
        key_path: app_key(id, key),
        value,
        merge_strategy: MergeStrategy::Replace,
    };
    if enabled {
        vec![
            edit("enabled", serde_json::Value::Null),
            edit("disabled_reason", serde_json::Value::Null),
        ]
    } else {
        vec![
            edit("enabled", serde_json::Value::Bool(false)),
            edit("disabled_reason", serde_json::Value::String("user".to_owned())),
        ]
    }
}

/// `config/batchWrite` of the app `id` switched (`SetAppEnabled`).
pub fn app_enabled_request(id: &str, enabled: bool, request: RequestId) -> ClientRequest {
    ClientRequest::ConfigBatchWrite {
        request_id: request,
        params: ConfigBatchWriteParams {
            edits: app_enabled_edits(id, enabled),
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// `save_quick_dial`: `tui.presets.<slot>` as a table of the input and the label (an explicit
/// empty label masks labels inherited from lower layers), or cleared.
pub fn preset_edit(slot: u8, dial: Option<&super::quick_dials::Dial>) -> ConfigEdit {
    ConfigEdit {
        key_path: ["tui.presets.", &slot.to_string()].concat(),
        value: dial.map_or(serde_json::Value::Null, |dial| {
            serde_json::json!({
                "input": dial.input,
                "label": dial.label.as_deref().unwrap_or_default(),
            })
        }),
        merge_strategy: MergeStrategy::Replace,
    }
}

/// `config/batchWrite` of the preset in `slot` (`save_quick_dial`).
pub fn preset_request(
    slot: u8,
    dial: Option<&super::quick_dials::Dial>,
    request: RequestId,
) -> ClientRequest {
    ClientRequest::ConfigBatchWrite {
        request_id: request,
        params: ConfigBatchWriteParams {
            edits: vec![preset_edit(slot, dial)],
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// `config/batchWrite` of `edits` (`write_config_batch`).
pub fn write_request(edits: &[Edit], id: RequestId) -> ClientRequest {
    ClientRequest::ConfigBatchWrite {
        request_id: id,
        params: ConfigBatchWriteParams {
            edits: edits
                .iter()
                .map(|edit| ConfigEdit {
                    key_path: edit.key.to_owned(),
                    value: edit.value.clone().map_or(serde_json::Value::Null, serde_json::Value::String),
                    merge_strategy: MergeStrategy::Replace,
                })
                .collect(),
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// How the write ended: saved, or saved under a layer that overrides it.
pub fn written(result: serde_json::Value) -> Result<bool, String> {
    serde_json::from_value::<codex_app_server_protocol::ConfigWriteResponse>(result)
        .map(|response| response.status == WriteStatus::OkOverridden)
        .map_err(|error| error.to_string())
}
