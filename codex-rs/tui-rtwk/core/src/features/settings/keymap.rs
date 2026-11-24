//! The catalogue of the `/keymap` picker (oracle `keymap_setup/picker.rs`, `actions.rs`): the
//! actions of the keymap by tab (All, Common, Customized, Unbound, one per group of contexts,
//! Debug), each with its binding as the config writes it and the text a search matches. Built
//! from the resolved `Keymap` the host holds, with no IO; the host offers it (`Offer::Keymap`)
//! and sends it again when the config changed. What the picker draws (cursor, tab, search) is
//! the sheet's own.

use std::collections::BTreeSet;

use super::catalog::{ActionItem, KeymapPicker, KeymapTab};
use super::keymap_actions::ACTIONS;
use crate::keymap::{Binding, Keymap};

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

/// A binding as the config writes it: `ctrl-t`, `ctrl-x b`.
pub fn binding_spec(binding: &Binding) -> String {
    match binding {
        Binding::Single(stroke) => stroke.spec(),
        Binding::Chord { prefix, completion } => {
            format!("{} {}", prefix.spec(), completion.spec())
        }
    }
}

/// The active bindings of an action as the config writes them, each once, `unbound` when it
/// has none (`format_action_binding_summary`).
fn summary(keymap: &Keymap, context: &str, name: &str) -> String {
    let specs: Vec<String> = if (context, name) == ("global", "leader") {
        keymap
            .leaders()
            .iter()
            .map(|stroke| stroke.spec())
            .collect()
    } else {
        keymap
            .actions()
            .find(|(id, _)| id.context.config_name() == context && id.name == name)
            .map(|(_, bindings)| bindings.iter().map(binding_spec).collect())
            .unwrap_or_default()
    };
    let mut seen = BTreeSet::new();
    let specs: Vec<String> = specs
        .into_iter()
        .filter(|spec| seen.insert(spec.clone()))
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
    keymap
        .actions()
        .map(|(id, _)| id)
        .find(|id| id.context.config_name() == context && id.name == name)
        .is_some_and(|id| keymap.is_configured(id))
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
                name: label,
                binding,
                search,
            },
            custom,
        }
    }
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
            format!("Customized ({})", customized.len()),
            &customized,
        ),
        tab("unbound", format!("Unbound ({})", unbound.len()), &unbound),
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
