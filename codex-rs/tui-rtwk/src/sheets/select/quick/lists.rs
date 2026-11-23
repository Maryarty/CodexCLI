//! The lists of `/presets` (oracle `chatwidget/quick_dials.rs`): the slot pickers, the sources and
//! the confirmation are lists of the shared picker on the oracle's `SelectionViewParams::picker()`
//! defaults (panel, stable columns, descriptions hidden when narrow, the standard hint). The rows
//! are core's (`Menu::page`, `Menu::Sources`, `Menu::Confirm`), the words are `strings.rs`. In
//! Flutter they are the `showModalBottomSheet` builders over a `ListView`.

use codex_tui_core::screens::select::quick_dials::{
    Action, CONFIRM, Dial, Menu, PickerMode, Quick, SlotRow,
};

use super::strings::Strings;
use crate::widgets::picker::list::{Columns, Item, List, Role, Span};

/// The list a menu of the presets is shown as; none for the editor (a prompt, `editor.rs`).
pub fn list(menu: &Menu, quick: &Quick, s: &Strings) -> Option<List> {
    let picker = List {
        standard_hint: true,
        panel: true,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        ..List::default()
    };
    Some(match menu {
        Menu::Slots { mode, dial } => {
            let page = menu.page(quick)?;
            List {
                title: Some(
                    match mode {
                        PickerMode::Save => s.save_title,
                        PickerMode::Edit => s.edit_title,
                        PickerMode::Clear => s.clear_title,
                    }
                    .to_string(),
                ),
                // `Save preset to slot` names the preset being saved.
                subtitle: dial
                    .as_ref()
                    .filter(|_| *mode == PickerMode::Save)
                    .map(Dial::label_line),
                items: page.rows.iter().map(|row| slot(row, s)).collect(),
                initial: page.initial,
                max_rows: Some(10),
                // A slot is picked by its digit, which is its number in the list (`display_shortcut`).
                numbered: false,
                ..picker
            }
        }
        Menu::Sources(rows) => List {
            title: Some(s.sources_title.to_string()),
            items: rows
                .iter()
                .map(|row| Item {
                    id: row.id().to_string(),
                    name: s.source(row.source).to_string(),
                    description: Some(match &row.dial {
                        Ok(dial) => dial.input.clone(),
                        Err(why) => s.unavailable(*why).to_string(),
                    }),
                    disabled: row.dial.is_err(),
                    ..Item::default()
                })
                .collect(),
            max_rows: Some(4),
            ..picker
        },
        Menu::Confirm { slot, old, new } => {
            let action = menu.action()?;
            List {
                header: confirmation(action, *slot, old.as_ref(), new.as_ref(), s),
                items: vec![Item {
                    id: CONFIRM.to_string(),
                    name: s.confirm_row(action, *slot),
                    // The row is numbered 1 in the oracle, but its digit only moves the cursor
                    // (`require_explicit_confirmation`: Enter confirms). The picker has no such
                    // row, so the number is drawn, not a shortcut.
                    prefix_spans: vec![Span {
                        text: "1. ".to_string(),
                        role: Role::Plain,
                    }],
                    ..Item::default()
                }],
                numbered: false,
                ..picker
            }
        }
        Menu::Editor { .. } => return None,
    })
}

/// A row of a slot picker: the label of the preset in the slot and its input under a label, or
/// `empty` (dim) for a slot with none.
fn slot(row: &SlotRow, s: &Strings) -> Item {
    Item {
        id: row.id.clone(),
        name: row.saved.as_ref().map(Dial::label_line).unwrap_or_default(),
        prefix_spans: if row.saved.is_none() {
            vec![Span {
                text: s.empty_slot.to_string(),
                role: Role::Dim,
            }]
        } else {
            Vec::new()
        },
        description: row
            .saved
            .as_ref()
            .filter(|dial| dial.has_label())
            .map(|dial| dial.input.clone()),
        shortcut: char::from_digit(u32::from(row.slot), 10),
        disabled: row.disabled,
        ..Item::default()
    }
}

/// The header of the confirmation (`QuickDialConfirmationHeader`): the question in bold, then what
/// the change is, dim, a line to a paragraph.
fn confirmation(
    action: Action,
    slot: u8,
    old: Option<&Dial>,
    new: Option<&Dial>,
    s: &Strings,
) -> Vec<Vec<Span>> {
    let what = match (action, old, new) {
        (Action::Replace, Some(old), Some(new)) => {
            format!("{}\n{}{}", old.input.trim(), s.arrow, new.input.trim())
        }
        (Action::Delete, Some(old), _) => old.label_line(),
        (_, _, Some(new)) => new.input.trim().to_string(),
        _ => String::new(),
    };
    let run = |text: String, role| {
        vec![Span {
            text,
            role,
        }]
    };
    std::iter::once(run(s.confirm_title(action, slot), Role::Strong))
        .chain(what.lines().map(|line| run(line.to_string(), Role::Dim)))
        .collect()
}
