//! What the edits of the settings dialogs write (oracle `config_update.rs` and
//! `app/event_dispatch.rs`: `config/batchWrite` through the app server): the key paths and
//! values an `Edit` stands for (the wire request of a batch is `protocol::config`). The dialogs
//! never name a key; the feature turns the edits they recorded into one write each.

use super::{Edit, Enable, MemoryFacts, SettingId, SetupKind};
use crate::app::effects::ConfigEdit;

/// What a write is for, with the values it saves: the words of its failure line are the
/// view's; what it changes in the session follows once the file has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    Theme(String),
    StatusLine {
        items: Vec<String>,
        use_theme_colors: bool,
    },
    Title(Vec<String>),
    Memories(MemoryFacts),
    /// `/tui`: fullscreen transcript on or off.
    TuiMode(bool),
    SidebarWidth(u8),
    Keymap,
    /// `/pets`: the pet picked (or the selector that turns pets off), `[tui].pet`.
    Pet(String),
    /// The rate limit prompt is not shown again, `[notice].hide_rate_limit_model_nudge`.
    RateLimitPrompt,
    /// A feature is turned on for new threads (`enable_feature_for_new_threads`).
    Enable(Enable),
}

impl Subject {
    /// The setting the write is for: what the words of its failure line name.
    pub fn setting(&self) -> Setting {
        match self {
            Subject::Theme(_) => Setting::Theme,
            Subject::StatusLine { .. } => Setting::StatusLine,
            Subject::Title(_) => Setting::Title,
            Subject::Memories(_) => Setting::Memories,
            Subject::TuiMode(_) => Setting::TuiMode,
            Subject::SidebarWidth(_) => Setting::SidebarWidth,
            Subject::Keymap => Setting::Keymap,
            Subject::Pet(selector) if selector == crate::screens::pets::DISABLED => {
                Setting::PetsDisabled
            }
            Subject::Pet(_) => Setting::PetSelection,
            Subject::RateLimitPrompt => Setting::RateLimitReminder,
            Subject::Enable(Enable::Memories) => Setting::MemoriesFeature,
            Subject::Enable(Enable::Subagents) => Setting::SubagentsFeature,
        }
    }
}

/// The settings a dialog writes, by name (`Failed to save {setting}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Theme,
    StatusLine,
    Title,
    Memories,
    TuiMode,
    SidebarWidth,
    Keymap,
    PetSelection,
    PetsDisabled,
    RateLimitReminder,
    MemoriesFeature,
    SubagentsFeature,
}

/// One batch of the config file and what it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    pub subject: Subject,
    pub edits: Vec<ConfigEdit>,
}

fn replace(key_path: &str, value: serde_json::Value) -> ConfigEdit {
    ConfigEdit {
        key_path: key_path.to_owned(),
        value,
    }
}

/// The batch `edit` is written as; none for an edit that is no write of the config file
/// (a test notification, a skill, a hook, a reset: their own requests).
pub fn write_of(edit: &Edit) -> Option<Write> {
    Some(match edit {
        // `syntax_theme_edit`: `[tui].theme`.
        Edit::ThemeSelected(name) => Write {
            subject: Subject::Theme(name.clone()),
            edits: vec![replace("tui.theme", serde_json::json!(name))],
        },
        // `status_line_items_edit` + `status_line_use_colors_edit`; `terminal_title_items_edit`:
        // an empty list is written too (it is "no title", not "unset").
        Edit::SetupSaved {
            kind: SetupKind::StatusLine,
            items,
            use_theme_colors,
        } => Write {
            subject: Subject::StatusLine {
                items: items.clone(),
                use_theme_colors: *use_theme_colors,
            },
            edits: vec![
                replace("tui.status_line", serde_json::json!(items)),
                replace(
                    "tui.status_line_use_colors",
                    serde_json::json!(use_theme_colors),
                ),
            ],
        },
        Edit::SetupSaved {
            kind: SetupKind::Title,
            items,
            ..
        } => Write {
            subject: Subject::Title(items.clone()),
            edits: vec![replace("tui.terminal_title", serde_json::json!(items))],
        },
        // `save_fullscreen_transcript`: `[tui].fullscreen_transcript`.
        Edit::TuiMode(enabled) => Write {
            subject: Subject::TuiMode(*enabled),
            edits: vec![replace(
                "tui.fullscreen_transcript",
                serde_json::json!(enabled),
            )],
        },
        Edit::SidebarWidth(percent) => Write {
            subject: Subject::SidebarWidth(*percent),
            edits: vec![replace(
                "tui.sidebar_width_percent",
                serde_json::json!(percent),
            )],
        },
        // `PetSelected` / `PetDisabled` (`app/pets.rs`): `[tui].pet` is the selector.
        Edit::PetSelected(selector) => Write {
            subject: Subject::Pet(selector.clone()),
            edits: vec![replace("tui.pet", serde_json::json!(selector))],
        },
        // `PersistRateLimitSwitchPromptHidden` (`set_hide_rate_limit_model_nudge(true)`).
        Edit::RateLimitPromptHidden => Write {
            subject: Subject::RateLimitPrompt,
            edits: vec![replace(
                "notice.hide_rate_limit_model_nudge",
                serde_json::json!(true),
            )],
        },
        // The keymap: `[tui.keymap.<context>] <action> = [keys]`.
        Edit::KeymapBound {
            context,
            action,
            keys,
        } => Write {
            subject: Subject::Keymap,
            edits: vec![replace(
                &["tui.keymap.", context.as_str(), ".", action.as_str()].concat(),
                serde_json::json!(keys),
            )],
        },
        // `keymap_binding_clear_edit`: the key is removed (a null value clears the path).
        Edit::KeymapCleared { context, action } => Write {
            subject: Subject::Keymap,
            edits: vec![replace(
                &["tui.keymap.", context.as_str(), ".", action.as_str()].concat(),
                serde_json::Value::Null,
            )],
        },
        // `build_memory_settings_edits`.
        Edit::MemoriesSaved {
            use_memories,
            generate_memories,
        } => Write {
            subject: Subject::Memories(MemoryFacts {
                use_memories: *use_memories,
                generate_memories: *generate_memories,
            }),
            edits: vec![
                replace("memories.use_memories", serde_json::json!(use_memories)),
                replace(
                    "memories.generate_memories",
                    serde_json::json!(generate_memories),
                ),
            ],
        },
        // `enable_feature_for_new_threads`: the feature's key, and the memory tool's legacy one.
        Edit::FeatureEnabled(feature) => Write {
            subject: Subject::Enable(*feature),
            edits: feature
                .keys()
                .iter()
                .map(|key| replace(&["features.", key].concat(), serde_json::json!(true)))
                .collect(),
        },
        _ => return None,
    })
}

/// The config key a preference of the panel is, and the value it takes (`ConfigPreference::edit`).
/// A toggle is a boolean; a choice is the text the config reads (none unsets the key).
pub fn preference_write(edit: &Edit) -> Option<ConfigEdit> {
    let key = |setting: SettingId| -> &'static str {
        match setting {
            SettingId::Tips => "tui.show_tooltips",
            SettingId::StatusLineColors => "tui.status_line_use_colors",
            SettingId::Notifications => "tui.notifications",
            SettingId::StartInVimMode => "tui.vim_mode_default",
            SettingId::PasteDetection => "tui.disable_paste_burst",
            SettingId::AutomaticRecaps => "tui.auto_recap",
            SettingId::CollapseTurnDetails => "tui.collapse_turn_details",
            SettingId::DeselectAfterCopy => "tui.copy_on_select_clear_selection",
            SettingId::Animations => "tui.animations",
            SettingId::ComposerStars => "tui.effects.starfield",
            SettingId::CopyOnSelect => "tui.copy_on_select",
            SettingId::NotifyWhen => "tui.notification_condition",
            SettingId::RightClickPaste => "tui.right_click_paste",
            SettingId::SessionPickerLayout => "tui.session_picker_view",
            SettingId::ResumeDirectory => "tui.resume_cwd",
            _ => "",
        }
    };
    match edit {
        Edit::Toggled(setting, enabled) => {
            let path = key(*setting);
            (!path.is_empty()).then(|| {
                // "Paste detection" is the inverse of the key (`disable_paste_burst`).
                let value = if *setting == SettingId::PasteDetection {
                    !enabled
                } else {
                    *enabled
                };
                replace(path, serde_json::json!(value))
            })
        }
        Edit::Chosen(setting, choice) => {
            let path = key(*setting);
            (!path.is_empty()).then(|| replace(path, serde_json::json!(choice)))
        }
        _ => None,
    }
}
