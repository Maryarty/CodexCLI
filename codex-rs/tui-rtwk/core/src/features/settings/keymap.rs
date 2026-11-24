//! The catalogue of the `/keymap` picker (oracle `keymap_setup/picker.rs`, `actions.rs`): the
//! actions of the keymap by tab (All, Common, Customized, Unbound, one per group of contexts,
//! Debug), each with its binding as the config writes it and the text a search matches. Built
//! from the resolved `Keymap` the host holds, with no IO; the host offers it (`Offer::Keymap`)
//! and sends it again when the config changed. What the picker draws (cursor, tab, search) is
//! the sheet's own.

use std::collections::BTreeSet;

use super::catalog::{ActionItem, KeymapPicker, KeymapTab};
use super::keymap_actions::ACTIONS;
use crate::keymap::Keymap;

/// The feature an action needs before `/keymap` lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    FastMode,
    MultiBackend,
    Workspace,
}

/// What the session has on (`KeymapActionFilter`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub fast_mode: bool,
    pub multi_backend: bool,
    pub workspace: bool,
}

/// One action of the oracle's catalogue.
#[derive(Clone, Copy, Debug)]
pub struct Action {
    pub context: &'static str,
    pub context_label: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub gate: Option<Gate>,
}

impl Action {
    fn visible(&self, filter: Filter) -> bool {
        match self.gate {
            None => true,
            Some(Gate::FastMode) => filter.fast_mode,
            Some(Gate::MultiBackend) => filter.multi_backend,
            Some(Gate::Workspace) => filter.workspace,
        }
    }
}

/// The actions the picker lists first (`KEYMAP_COMMON_ACTIONS`).
const COMMON: &[(&str, &str)] = &[
    ("composer", "submit"),
    ("chat", "interrupt_turn"),
    ("editor", "insert_newline"),
    ("composer", "queue"),
    ("global", "toggle_fast_mode"),
    ("global", "open_external_editor"),
    ("global", "copy"),
    ("global", "toggle_vim_mode"),
    ("editor", "delete_backward_word"),
    ("editor", "delete_forward_word"),
    ("editor", "move_word_left"),
    ("editor", "move_word_right"),
    ("global", "open_transcript"),
    ("pager", "close"),
    ("pager", "page_up"),
    ("pager", "page_down"),
    ("approval", "open_fullscreen"),
    ("approval", "approve"),
    ("approval", "approve_for_session"),
    ("approval", "decline"),
    ("approval", "cancel"),
];

/// The tabs of the groups of contexts (`KEYMAP_CONTEXT_TABS`): id, label, contexts.
const GROUPS: &[(&str, &str, &[&str])] = &[
    ("app", "App", &["global", "chat"]),
    ("composer", "Composer", &["composer"]),
    ("editor", "Editor", &["editor"]),
    (
        "vim",
        "Vim",
        &[
            "vim_normal",
            "vim_operator",
            "vim_search",
            "vim_text_object",
        ],
    ),
    ("navigation", "Navigation", &["pager", "list"]),
    ("agents", "Agents", &["agents"]),
    ("approval", "Approval", &["approval"]),
];

/// The id of the debug tab, whose one row opens the key inspector.
pub const DEBUG_TAB: &str = "debug";

/// A stable action identifier as a label (`action_label`): presentation only.
pub fn action_label(action: &str) -> String {
    if action == "change_host" {
        return "Change Where to Run".to_owned();
    }
    action
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The action of the catalogue named `context.name`.
pub fn action(context: &str, name: &str) -> Option<&'static Action> {
    ACTIONS
        .iter()
        .find(|action| action.context == context && action.name == name)
}

/// The id the keymap knows an action of the catalogue by.
fn id_of(keymap: &Keymap, context: &str, name: &str) -> Option<crate::keymap::ActionId> {
    keymap
        .actions()
        .map(|(id, _)| id)
        .find(|id| id.context.config_name() == context && id.name == name)
}

/// The bindings the action has now as the config writes them, each once; none for an action
/// the keymap does not hold (`active_binding_specs`).
pub fn active_specs(keymap: &Keymap, context: &str, name: &str) -> Option<Vec<String>> {
    if (context, name) == ("global", "leader") {
        let mut seen = BTreeSet::new();
        return Some(
            keymap
                .leaders()
                .iter()
                .map(crate::keymap::Stroke::spec)
                .filter(|spec| seen.insert(spec.clone()))
                .collect(),
        );
    }
    id_of(keymap, context, name).map(|id| keymap.specs(id))
}

/// The active bindings of an action as the config writes them, each once, `unbound` when it
/// has none; a `leader x` binding names the chords the leader keys make of it
/// (`format_action_binding_summary`).
pub fn summary(keymap: &Keymap, context: &str, name: &str) -> String {
    let specs: Vec<String> = active_specs(keymap, context, name)
        .unwrap_or_default()
        .into_iter()
        .map(|spec| {
            let Some(completion) = spec.strip_prefix("leader ") else {
                return spec;
            };
            let resolved: Vec<String> = keymap
                .leaders()
                .iter()
                .map(|leader| format!("{} {completion}", leader.spec()))
                .collect();
            if resolved.is_empty() {
                [spec.as_str(), " (inactive)"].concat()
            } else {
                format!("{spec} ({})", resolved.join(", "))
            }
        })
        .collect();
    if specs.is_empty() {
        "unbound".to_owned()
    } else {
        specs.join(", ")
    }
}

/// Whether the user's config sets the action (an explicit unbind counts).
fn custom(keymap: &Keymap, context: &str, name: &str) -> bool {
    if (context, name) == ("global", "leader") {
        return false;
    }
    id_of(keymap, context, name).is_some_and(|id| keymap.is_configured(id))
}

struct Row {
    context: &'static str,
    action: &'static str,
    item: ActionItem,
    custom: bool,
    unbound: bool,
}

impl Row {
    fn new(action: &Action, label: String, binding: String, custom: bool, source: &str) -> Row {
        let search = format!(
            "{} {} {} {} {} {}",
            action.context_label, action.name, label, action.description, binding, source
        );
        Row {
            context: action.context,
            action: action.name,
            unbound: binding == "unbound",
            item: ActionItem {
                context: action.context.to_owned(),
                context_label: action.context_label.to_owned(),
                action: action.name.to_owned(),
                name: label,
                binding,
                custom,
                search,
            },
            custom,
        }
    }
}

/// A catalogue tab's name with its count, as the oracle's tab bar reads it.
fn counted(label: &str, count: usize) -> String {
    format!("{label} ({count})")
}

fn tab(id: &str, label: String, rows: &[&Row]) -> KeymapTab {
    KeymapTab {
        id: id.to_owned(),
        label,
        selectable: rows.len(),
        items: Vec::new(),
    }
}

/// The picker of `keymap`: the rows are in the catalogue's order; every tab says how many
/// actions it holds, and the All tab lists them with their bindings (the sheet reads the other
/// tabs' rows from it by context, the debug tab has the one row of the inspector).
pub fn picker(keymap: &Keymap, filter: Filter) -> KeymapPicker {
    let rows: Vec<Row> = ACTIONS
        .iter()
        .filter(|action| action.visible(filter))
        .map(|action| {
            let label = action_label(action.name);
            let binding = summary(keymap, action.context, action.name);
            let custom = custom(keymap, action.context, action.name);
            let source = if custom { "Custom" } else { "Default" };
            Row::new(action, label, binding, custom, source)
        })
        .collect();
    let all: Vec<&Row> = rows.iter().collect();
    let common: Vec<&Row> = COMMON
        .iter()
        .filter_map(|(context, action)| {
            rows.iter()
                .find(|row| row.context == *context && row.action == *action)
        })
        .collect();
    let customized: Vec<&Row> = rows.iter().filter(|row| row.custom).collect();
    let unbound: Vec<&Row> = rows.iter().filter(|row| row.unbound).collect();
    let mut tabs = vec![
        tab("all", "All".to_owned(), &all),
        tab("common", "Common".to_owned(), &common),
        tab(
            "customized",
            counted("Customized", customized.len()),
            &customized,
        ),
        tab("unbound", counted("Unbound", unbound.len()), &unbound),
    ];
    tabs[0].items = rows.iter().map(|row| row.item.clone()).collect();
    tabs[1].items = common.iter().map(|row| row.item.clone()).collect();
    tabs[2].items = customized.iter().map(|row| row.item.clone()).collect();
    tabs[3].items = unbound.iter().map(|row| row.item.clone()).collect();
    for (id, label, contexts) in GROUPS {
        let group: Vec<&Row> = rows
            .iter()
            .filter(|row| contexts.contains(&row.context))
            .collect();
        let mut tab = tab(id, (*label).to_owned(), &group);
        tab.items = group.iter().map(|row| row.item.clone()).collect();
        tabs.push(tab);
    }
    tabs.push(KeymapTab {
        id: DEBUG_TAB.to_owned(),
        label: "Debug".to_owned(),
        selectable: 1,
        items: Vec::new(),
    });
    KeymapPicker { tabs }
}

/// Where an action's binding on a key comes from (`KeymapDebugBindingSource`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The user's `tui.keymap.<context>.<action>`.
    Custom,
    /// The user's `tui.keymap.global.<action>` the composer's action falls back to.
    CustomGlobal,
    Default,
}

/// An action that has a key as one of its single keys (`KeymapDebugActionMatch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMatch {
    pub context: &'static str,
    pub action: &'static str,
    pub label: String,
    pub description: &'static str,
    pub source: Source,
}

/// The actions of the catalogue (every one, whatever feature gates it) that have `stroke` as a
/// single key under `keymap`, each with where its binding comes from in `spec`, the config the
/// user wrote (`matching_actions_for_key_event`, `debug_binding_source`).
pub fn matching(keymap: &Keymap, spec: &crate::keymap::KeymapSpec, stroke: &crate::keymap::Stroke) -> Vec<KeyMatch> {
    ACTIONS
        .iter()
        .filter(|action| {
            id_of(keymap, action.context, action.name).is_some_and(|id| {
                keymap
                    .singles(id)
                    .iter()
                    .any(|single| single.same_key(stroke))
            })
        })
        .map(|action| {
            let source = if spec.get(action.context, action.name).is_some() {
                Source::Custom
            } else if action.context == "composer"
                && matches!(action.name, "submit" | "queue" | "toggle_shortcuts")
                && spec.get("global", action.name).is_some()
            {
                Source::CustomGlobal
            } else {
                Source::Default
            };
            KeyMatch {
                context: action.context,
                action: action.name,
                label: action_label(action.name),
                description: action.description,
                source,
            }
        })
        .collect()
}
