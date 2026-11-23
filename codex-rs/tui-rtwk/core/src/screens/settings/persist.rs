//! What the edits of the settings dialogs write (oracle `config_update.rs` and
//! `app/event_dispatch.rs`: `config/batchWrite` through the app server): the key paths and
//! values an `Edit` stands for, and the wire request of a batch. The dialogs never name a
//! key; the app turns the edits they recorded into one write each.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit as WireEdit;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;

use super::{Edit, MemoryFacts, SetupKind};
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
        _ => return None,
    })
}

/// `config/batchWrite` of `edits` (`write_config_batch`): the values replace, the user's
/// config is reloaded.
pub fn write_request(edits: &[ConfigEdit], id: RequestId) -> ClientRequest {
    ClientRequest::ConfigBatchWrite {
        request_id: id,
        params: ConfigBatchWriteParams {
            edits: edits
                .iter()
                .map(|edit| WireEdit {
                    key_path: edit.key_path.clone(),
                    value: edit.value.clone(),
                    merge_strategy: MergeStrategy::Replace,
                })
                .collect(),
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// How the write ended: saved (`false`), or saved under a layer that overrides it (`true`).
pub use crate::screens::select::persist::written;
