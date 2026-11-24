//! The words of the `/subagents` picker (oracle `app/session_lifecycle.rs`
//! `agent_picker_selection_view_params`, `multi_agents.rs` `agent_picker_status_dot_spans`,
//! `agent_navigation.rs` `picker_subtitle`). The wording is the oracle's English until the catalog
//! has these entries.

use codex_tui_core::screens::select::agent_picker::Row;

use crate::widgets::picker::list::{Columns, Item, List, Role, Span};

const TITLE: &str = "Subagents";
/// The subtitle with the keys that move between agents (`alt(Left)`, `alt(Right)` as the oracle's
/// key hints print them).
const SUBTITLE: &str = "Select an agent to watch. alt + \u{2190} previous, alt + \u{2192} next.";
const NEEDS_APPROVAL: &str = " (needs approval)";

/// The list of the picker; `initial` is the row the cursor starts on.
pub fn list(rows: &[Row], initial: Option<usize>) -> List {
    let items = rows
        .iter()
        .map(|row| {
            let name = row.name.label();
            let uuid = row.thread.as_str().to_owned();
            Item {
                id: uuid.clone(),
                name: if row.needs_approval {
                    format!("{name}{NEEDS_APPROVAL}")
                } else {
                    name.clone()
                },
                prefix_spans: vec![
                    Span {
                        text: "\u{2022}".to_owned(),
                        role: if row.closed { Role::Plain } else { Role::Success },
                    },
                    Span {
                        text: " ".to_owned(),
                        role: Role::Plain,
                    },
                ],
                description: Some(uuid.clone()),
                current: row.current,
                search_value: Some(format!("{name} {uuid}")),
                ..Item::default()
            }
        })
        .collect();
    List {
        title: Some(TITLE.to_owned()),
        subtitle: Some(SUBTITLE.to_owned()),
        standard_hint: true,
        items,
        initial,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
