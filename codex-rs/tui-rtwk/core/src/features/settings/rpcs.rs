//! The server RPCs of the dialogs (oracle `experimental_features.rs`, `hooks_rpc.rs`,
//! `config_update.rs`, `app/background_requests.rs`): the list of what is asked and what comes
//! back (`rpc_list!`). The catalogues the dialogs show are built here from the
//! protocol types, so a sheet never reads one (the protocol's `HookMetadata` is not `Eq`).

use codex_app_server_protocol as wire;

use super::catalog;
use super::hooks::{Hook, HookIssue, Hooks};
use super::resume::{Listed, display_label};
use crate::ids::ThreadId;
use crate::threads::asks::{self, ListQuery};

/// The most pages of the experimental features one listing reads (`MAX_PAGES`).
pub const MAX_PAGES: u8 = 10;
/// The page size of that listing.
pub const PAGE: u32 = 100;

crate::rpc_list! {
    /// What the feature asks the server.
    pub enum Command => Reply, request thread: &ThreadId;
    /// `experimentalFeature/list`, one page.
    Features { cursor: Option<String> }, Self::Features { cursor }
        => ExperimentalFeatureList(wire::ExperimentalFeatureListParams {
            cursor: cursor.clone(),
            limit: Some(PAGE),
            thread_id: Some(thread.as_str().to_owned()).filter(|thread| !thread.is_empty()),
        })
        -> wire::ExperimentalFeatureListResponse, |response| features(response);
    /// `skills/list` of a directory, read again from disk.
    Skills { cwd: String }, Self::Skills { cwd }
        => SkillsList(wire::SkillsListParams {
            cwds: Some(cwd)
                .filter(|cwd| !cwd.is_empty())
                .map(|cwd| vec![cwd.into()])
                .unwrap_or_default(),
            force_reload: true,
        })
        -> wire::SkillsListResponse, |response| Reply::Skills(skills(response, cwd));
    /// `skills/config/write` of one skill.
    SkillWrite { path: String, enabled: bool }, Self::SkillWrite { path, enabled }
        => SkillsConfigWrite(wire::SkillsConfigWriteParams {
            path: serde_json::from_value(serde_json::Value::String(path.clone())).ok(),
            name: None,
            enabled: *enabled,
        })
        -> serde_json::Value, |_| Reply::Written;
    /// `hooks/list` of a directory.
    Hooks { cwd: String }, Self::Hooks { cwd }
        => HooksList(wire::HooksListParams {
            cwds: Some(cwd)
                .filter(|cwd| !cwd.is_empty())
                .map(|cwd| vec![cwd.into()])
                .unwrap_or_default(),
            ..Default::default()
        })
        -> wire::HooksListResponse, |response| Reply::Hooks(hooks(response, cwd));
    /// `config/batchWrite` of `hooks.state`: the value is merged into what the file has.
    HooksWrite { value: serde_json::Value }, Self::HooksWrite { value }
        => ConfigBatchWrite(wire::ConfigBatchWriteParams {
            edits: vec![wire::ConfigEdit {
                key_path: "hooks.state".to_owned(),
                value: value.clone(),
                merge_strategy: wire::MergeStrategy::Upsert,
            }],
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        })
        -> serde_json::Value, |_| Reply::Written;
    /// `thread/list`, one page of the sessions `/resume <name>` looks a label up in.
    Sessions { cursor: Option<String> }, Self::Sessions { cursor }
        => Reuse(ListQuery {
            current_account_only: false,
            cursor: cursor.clone(),
            limit: Some(PAGE),
            // Activity must not move an unread thread ahead of the cursor.
            sort_key: Some(wire::ThreadSortKey::CreatedAt),
            model_providers: Some(Vec::new()),
            source_kinds: Some(vec![wire::ThreadSourceKind::Cli, wire::ThreadSourceKind::VsCode]),
            archived: Some(false),
            use_state_db_only: true,
            ..ListQuery::default()
        })
        -> wire::ThreadListResponse as asks::listed, |page| Reply::Sessions {
            rows: page.data.iter().map(listed).collect(),
            next: page.next_cursor,
        };
}

/// A skill of the listing (`SkillMetadata`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub path: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub enabled: bool,
}

/// The decoded answer of a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Features {
        features: Vec<catalog::Feature>,
        next: Option<String>,
    },
    Skills(Vec<Skill>),
    /// A write the server accepted.
    Written,
    Hooks(Hooks),
    Sessions {
        rows: Vec<Listed>,
        next: Option<String>,
    },
}

/// `camelCase` as the protocol writes an enum to `snake_case` as the models read it.
fn snake(camel: &str) -> String {
    let mut out = String::new();
    for ch in camel.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// The hook of the browser a server entry stands for.
fn hook(entry: &wire::HookMetadata) -> Option<Hook> {
    let wire = serde_json::to_value(entry).ok()?;
    let text = |name: &str| wire.get(name).and_then(|value| value.as_str());
    let mut object = serde_json::Map::new();
    let mut put = |name: &str, value: serde_json::Value| {
        object.insert(name.to_owned(), value);
    };
    put("key", entry.key.clone().into());
    put("event", snake(text("eventName")?).into());
    put("handler", snake(text("handlerType")?).into());
    for name in ["command", "async", "server", "tool"] {
        if let Some(value) = wire.get(name) {
            put(name, value.clone());
        }
    }
    put("matcher", wire.get("matcher").cloned().unwrap_or_default());
    put("timeout_sec", entry.timeout_sec.into());
    put(
        "status_message",
        wire.get("statusMessage").cloned().unwrap_or_default(),
    );
    put(
        "additional_context_limit",
        wire.get("additionalContextLimit")
            .cloned()
            .unwrap_or_default(),
    );
    put("source", snake(text("source")?).into());
    put(
        "source_path",
        wire.get("sourcePath").cloned().unwrap_or_default(),
    );
    put(
        "plugin_id",
        wire.get("pluginId").cloned().unwrap_or_default(),
    );
    put(
        "environment_id",
        wire.get("environmentId").cloned().unwrap_or_default(),
    );
    put("enabled", entry.enabled.into());
    put("is_managed", entry.is_managed.into());
    put("current_hash", entry.current_hash.clone().into());
    put("trust", snake(text("trustStatus")?).into());
    serde_json::from_value(serde_json::Value::Object(object)).ok()
}

/// The hooks of the entry of `cwd` (else the first), in the server's display order.
fn hooks(response: wire::HooksListResponse, cwd: &str) -> Hooks {
    let entry = response
        .data
        .iter()
        .position(|entry| entry.cwd == std::path::Path::new(cwd))
        .unwrap_or(0);
    let Some(entry) = response.data.into_iter().nth(entry) else {
        return Hooks::new(Vec::new(), Vec::new(), Vec::new());
    };
    let mut metadata = entry.hooks;
    metadata.sort_by_key(|hook| hook.display_order);
    Hooks::new(
        metadata.iter().filter_map(hook).collect(),
        entry.warnings,
        entry
            .errors
            .into_iter()
            .map(|issue| HookIssue {
                path: issue.path.to_string_lossy().into_owned(),
                message: issue.message,
            })
            .collect(),
    )
}

fn skills(response: wire::SkillsListResponse, cwd: &str) -> Vec<Skill> {
    let entry = response
        .data
        .iter()
        .position(|entry| entry.cwd == std::path::Path::new(cwd))
        .unwrap_or(0);
    let Some(entry) = response.data.into_iter().nth(entry) else {
        return Vec::new();
    };
    entry
        .skills
        .into_iter()
        .map(|skill| {
            let path = serde_json::to_value(&skill.path)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default();
            let display_name = skill
                .interface
                .as_ref()
                .and_then(|interface| interface.display_name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| skill.name.clone());
            let description = skill
                .interface
                .as_ref()
                .and_then(|interface| interface.short_description.clone())
                .or(skill.short_description.clone())
                .unwrap_or(skill.description.clone());
            Skill {
                path,
                name: skill.name,
                display_name,
                description,
                enabled: skill.enabled,
            }
        })
        .collect()
}

/// `ExperimentalFeature` as the dialog lists it: the beta features only (the rest have no
/// display name, so the dialog does not show them).
fn features(response: wire::ExperimentalFeatureListResponse) -> Reply {
    Reply::Features {
        features: response
            .data
            .into_iter()
            .filter(|feature| feature.stage == wire::ExperimentalFeatureStage::Beta)
            .filter_map(|feature| {
                Some(catalog::Feature {
                    name: feature.display_name?,
                    description: feature.description.unwrap_or_default(),
                    key: feature.name,
                    enabled: feature.enabled,
                    writable: true,
                })
            })
            .collect(),
        next: response.next_cursor,
    }
}

fn listed(thread: &wire::Thread) -> Listed {
    Listed {
        id: thread.id.clone(),
        label: display_label(thread.name.as_deref(), &thread.preview),
    }
}
