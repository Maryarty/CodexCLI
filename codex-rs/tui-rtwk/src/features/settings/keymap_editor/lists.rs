//! The lists of the `/keymap` sheet (oracle `keymap_setup/picker.rs` and `keymap_setup.rs`):
//! the picker of the actions by tab, the menu of an action, the menu of the bindings to
//! replace, and the conflict sheet. Pure functions of what the feature offers and the keymap
//! in force; nothing here is drawn.

use codex_tui_core::features::settings::keymap::{self, action_label};
use codex_tui_core::features::settings::{ActionItem, KeymapPicker};
use codex_tui_core::keymap::Keymap;

use super::strings::Strings;
use crate::features::settings::strings::Strings as Frame;
use crate::widgets::picker::list::{Columns, Item, List, Role, Span, Tab};
use crate::widgets::picker::{Extras, TabNote};

/// The width of the context column of a row (`KEYMAP_CONTEXT_LABEL_WIDTH`).
const CONTEXT_WIDTH: usize = 12;
/// What a row puts before its name: the context, the mark and a blank.
const PREFIX_WIDTH: usize = CONTEXT_WIDTH + 3;
/// The rows the picker shows at most.
const MAX_ROWS: usize = 24;

pub const DEBUG_ID: &str = "debug";

fn width_of(text: &str) -> usize {
    rtwk_core::unicode::string_width(text, rtwk_core::unicode::TextMode::Graphemes, 1)
}

/// An action's name as the pickers write it.
pub fn label_of(action: &str) -> String {
    action_label(action)
}

/// The id of a row: the action it stands for.
pub fn id_of(context: &str, action: &str) -> String {
    format!("{context}.{action}")
}

/// The action a row id names.
pub fn action_of(id: &str) -> Option<(&str, &str)> {
    id.split_once('.')
}

fn span(text: impl Into<String>, role: Role) -> Span {
    Span {
        text: text.into(),
        role,
    }
}

fn line(roles: Vec<(String, Role)>) -> Vec<Span> {
    roles
        .into_iter()
        .map(|(text, role)| span(text, role))
        .collect()
}

/// A row that is only said.
fn said(name: &str, description: &str) -> Item {
    Item {
        name: name.to_owned(),
        description: Some(description.to_owned()),
        disabled: true,
        ..Item::default()
    }
}

fn row_of(item: &ActionItem) -> Item {
    let indicator = if item.custom {
        span("*", Role::Info)
    } else if item.binding == "unbound" {
        span("-", Role::Dim)
    } else {
        span(" ", Role::Plain)
    };
    Item {
        id: id_of(&item.context, &item.action),
        name: item.name.clone(),
        description: Some(item.binding.clone()),
        search_value: Some(item.search.clone()),
        prefix_spans: vec![
            span(
                format!("{:<width$} ", item.context_label, width = CONTEXT_WIDTH),
                Role::Dim,
            ),
            indicator,
            span(" ", Role::Dim),
        ],
        ..Item::default()
    }
}

fn count_line(frame: &Frame, count: usize) -> String {
    if count == 1 {
        frame.keymap_actions_one.to_owned()
    } else {
        frame.actions_many(count).to_string()
    }
}

/// The picker: a tab per group of the offer, and what each says under the title.
pub fn picker(
    offer: &KeymapPicker,
    s: &Strings,
    frame: &Frame,
    tab: Option<&str>,
    selected: Option<&str>,
) -> (List, Extras) {
    let all = offer.tabs.first().map(|tab| tab.items.as_slice()).unwrap_or(&[]);
    let custom = all.iter().filter(|item| item.custom).count();
    let unbound = all.iter().filter(|item| item.binding == "unbound").count();
    let name_column_width = all
        .iter()
        .map(|item| PREFIX_WIDTH + width_of(&item.name))
        .max();
    let mut tabs = Vec::new();
    let mut notes = Vec::new();
    for shown in &offer.tabs {
        let words = s.tab(&shown.id);
        let debug = shown.id == "debug";
        let items = if debug {
            vec![Item {
                id: DEBUG_ID.to_owned(),
                name: s.debug_row.to_owned(),
                description: Some(s.debug_row_hint.to_owned()),
                selected_description: Some(s.debug_row_selected.to_owned()),
                search_value: Some(s.debug_row_search.to_owned()),
                ..Item::default()
            }]
        } else if shown.items.is_empty() {
            vec![said(
                words.map_or(frame.keymap_empty, |words| words.empty_name),
                words.map_or("", |words| words.empty_description),
            )]
        } else {
            shown.items.iter().map(row_of).collect()
        };
        let summary = if debug {
            s.debug_summary.to_owned()
        } else if shown.id == "all" {
            s.all_summary(all.len(), custom, unbound).to_string()
        } else {
            count_line(frame, shown.items.len())
        };
        notes.push(TabNote {
            header: vec![
                vec![span(words.map_or("", |words| words.description), Role::Dim)],
                vec![span(summary, Role::Dim)],
            ],
            hint: debug.then(|| s.debug_footer.to_owned()),
        });
        tabs.push(Tab {
            id: shown.id.clone(),
            label: shown.label.clone(),
            items,
        });
    }
    let initial_tab = tab
        .filter(|id| tabs.iter().any(|tab| tab.id == *id))
        .map_or_else(|| "all".to_owned(), str::to_owned);
    let initial = selected.and_then(|id| {
        tabs.iter()
            .find(|tab| tab.id == initial_tab)?
            .items
            .iter()
            .position(|item| item.id == id)
    });
    let list = List {
        title: Some(frame.keymap_title.to_owned()),
        footer_hint: Some(frame.keymap_hint.to_owned()),
        tabs,
        initial_tab: Some(initial_tab),
        initial,
        searchable: true,
        search_placeholder: Some(frame.keymap_search_placeholder.to_owned()),
        columns: Columns::AutoAllRows,
        single_line: true,
        name_column_width,
        max_rows: Some(MAX_ROWS),
        panel: true,
        ..List::default()
    };
    let extras = Extras {
        notes,
        reserve: true,
        keep_position: true,
        ..Extras::default()
    };
    (list, extras)
}

/// What a row of the menu of an action does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Set,
    Replace,
    Add,
    ReplaceOne,
    ReplaceAll,
    Leader,
    ConfigureLeader,
    Chord,
    ReplaceChord,
    AddChord,
    Remove,
    Back,
}

impl Pick {
    pub const ALL: [Pick; 12] = [
        Pick::Set,
        Pick::Replace,
        Pick::Add,
        Pick::ReplaceOne,
        Pick::ReplaceAll,
        Pick::Leader,
        Pick::ConfigureLeader,
        Pick::Chord,
        Pick::ReplaceChord,
        Pick::AddChord,
        Pick::Remove,
        Pick::Back,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Pick::Set => "set",
            Pick::Replace => "replace",
            Pick::Add => "add",
            Pick::ReplaceOne => "replace_one",
            Pick::ReplaceAll => "replace_all",
            Pick::Leader => "leader",
            Pick::ConfigureLeader => "configure_leader",
            Pick::Chord => "chord",
            Pick::ReplaceChord => "replace_chord",
            Pick::AddChord => "add_chord",
            Pick::Remove => "remove",
            Pick::Back => "back",
        }
    }

    pub fn of(id: &str) -> Option<Pick> {
        Pick::ALL.into_iter().find(|pick| pick.id() == id)
    }
}

fn entry(pick: Pick, name: &str, description: String) -> Item {
    Item {
        id: pick.id().to_owned(),
        name: name.to_owned(),
        description: Some(description),
        ..Item::default()
    }
}

/// The menu of an action (`build_keymap_action_menu_params`): what its bindings are, where
/// they come from, and what can be done with them.
pub fn menu(
    context: &str,
    action: &str,
    keymap: &Keymap,
    custom: bool,
    s: &Strings,
) -> List {
    let bindings = keymap::active_specs(keymap, context, action).unwrap_or_default();
    let current = if bindings.is_empty() {
        s.unbound.to_owned()
    } else {
        bindings.join(", ")
    };
    let descriptor = keymap::action(context, action);
    let context_label = descriptor.map_or(context, |descriptor| descriptor.context_label);
    let description = descriptor.map_or(s.menu_fallback_description, |descriptor| {
        descriptor.description
    });
    let current_role = if bindings.is_empty() {
        Role::Dim
    } else {
        Role::Info
    };
    let (source, source_role) = if custom {
        (s.menu_source_custom, Role::Info)
    } else {
        (s.menu_source_default, Role::Dim)
    };
    let header = vec![
        vec![span(s.menu_title, Role::Strong)],
        line(vec![
            (label_of(action), Role::Strong),
            (" \u{b7} ".to_owned(), Role::Dim),
            (context_label.to_owned(), Role::Dim),
        ]),
        line(vec![
            (s.menu_current.to_owned(), Role::Dim),
            (current.clone(), current_role),
            (" \u{b7} ".to_owned(), Role::Dim),
            (source.to_owned(), source_role),
        ]),
        line(vec![
            (s.menu_config.to_owned(), Role::Dim),
            (format!("`tui.keymap.{context}.{action}`"), Role::Info),
        ]),
        vec![span(description, Role::Dim)],
    ];
    let mut items = Vec::new();
    match bindings.len() {
        0 => items.push(entry(Pick::Set, s.set_key, s.set_key_hint.to_owned())),
        1 => {
            items.push(entry(
                Pick::Replace,
                s.replace_binding,
                s.with(s.replace_binding_hint, &current).to_string(),
            ));
            items.push(entry(
                Pick::Add,
                s.add_alternate,
                s.with(s.add_alternate_hint, &current).to_string(),
            ));
        }
        _ => {
            items.push(entry(
                Pick::ReplaceOne,
                s.replace_one,
                s.replace_one_hint.to_owned(),
            ));
            items.push(entry(
                Pick::ReplaceAll,
                s.replace_all,
                s.with(s.replace_all_hint, &current).to_string(),
            ));
            items.push(entry(
                Pick::Add,
                s.add_alternate,
                s.with(s.add_alternate_hint, &current).to_string(),
            ));
        }
    }
    if action != "leader" {
        let available = !keymap.leaders().is_empty();
        let mut leader = entry(Pick::Leader, s.use_leader, s.use_leader_hint.to_owned());
        if !available {
            leader.disabled_reason = Some(s.leader_unbound.to_owned());
        }
        items.push(leader);
        if !available {
            items.push(entry(
                Pick::ConfigureLeader,
                s.configure_leader,
                s.configure_leader_hint.to_owned(),
            ));
        }
    }
    if action != "leader" && bindings.is_empty() {
        items.push(entry(Pick::Chord, s.set_chord, s.set_chord_hint.to_owned()));
    } else if action != "leader" {
        items.push(entry(
            Pick::ReplaceChord,
            s.replace_chord,
            s.with(s.replace_chord_hint, &current).to_string(),
        ));
        items.push(entry(
            Pick::AddChord,
            s.add_chord,
            s.with(s.add_chord_hint, &current).to_string(),
        ));
    }
    items.push(Item {
        id: Pick::Remove.id().to_owned(),
        name: s.remove_custom.to_owned(),
        description: custom.then(|| s.remove_custom_hint.to_owned()),
        disabled_reason: (!custom).then(|| s.remove_custom_disabled.to_owned()),
        ..Item::default()
    });
    items.push(entry(Pick::Back, s.back, s.back_hint.to_owned()));
    List {
        header,
        footer_note: Some(format!(
            "{}{}{}",
            s.menu_footer_note_open, s.menu_footer_note_path, s.menu_footer_note_close
        )),
        standard_hint: true,
        items,
        ..List::default()
    }
}

/// The id of a row of the replace menu: the binding, and whether the chord is asked for.
pub fn replace_id(binding: &str, chord: bool) -> String {
    format!("{}:{binding}", if chord { "chord" } else { "key" })
}

/// The binding a row of the replace menu replaces, and whether it asks for a chord.
pub fn replace_of(id: &str) -> Option<(&str, bool)> {
    let (kind, binding) = id.split_once(':')?;
    Some((binding, kind == "chord"))
}

/// The menu of the bindings an action has, to say which one a new key takes the place of
/// (`build_keymap_replace_binding_menu_params`).
pub fn replace_menu(context: &str, action: &str, keymap: &Keymap, s: &Strings) -> List {
    let bindings = keymap::active_specs(keymap, context, action).unwrap_or_default();
    let header = vec![
        vec![span(s.replace_menu_title, Role::Strong)],
        line(vec![
            (label_of(action), Role::Strong),
            (" \u{b7} ".to_owned(), Role::Dim),
            (format!("{context}.{action}"), Role::Dim),
        ]),
        vec![span(s.replace_menu_pick, Role::Dim)],
    ];
    let items = bindings
        .iter()
        .flat_map(|binding| {
            [
                Item {
                    id: replace_id(binding, false),
                    name: binding.clone(),
                    description: Some(s.with(s.replace_one_key_hint, binding).to_string()),
                    ..Item::default()
                },
                Item {
                    id: replace_id(binding, true),
                    name: s.with(s.chord_suffix, binding).to_string(),
                    description: Some(s.with(s.replace_one_chord_hint, binding).to_string()),
                    ..Item::default()
                },
            ]
            .into_iter()
            .take(if action == "leader" { 1 } else { 2 })
        })
        .collect();
    List {
        header,
        standard_hint: true,
        items,
        ..List::default()
    }
}

/// The ids of the conflict sheet's two rows.
pub const AGAIN: &str = "again";
pub const CANCEL: &str = "cancel";

/// The sheet of a key the keymap cannot take (`build_keymap_conflict_params`).
pub fn conflict(context: &str, action: &str, key: &str, error: &str, s: &Strings) -> List {
    List {
        header: vec![
            vec![span(s.conflict_title, Role::Strong)],
            vec![span(
                s.conflict(context, action, key).to_string(),
                Role::Dim,
            )],
            vec![span(error, Role::Danger)],
        ],
        standard_hint: true,
        items: vec![
            Item {
                id: AGAIN.to_owned(),
                name: s.pick_another.to_owned(),
                description: Some(s.pick_another_hint.to_owned()),
                ..Item::default()
            },
            Item {
                id: CANCEL.to_owned(),
                name: s.cancel.to_owned(),
                description: Some(s.cancel_hint.to_owned()),
                ..Item::default()
            },
        ],
        ..List::default()
    }
}
