//! The list of the confirmation sheet (`confirm.rs`). Every family is a sheet of its own
//! (`models.rs`, `permissions.rs`, `apps.rs`, `quick.rs`, `worktree.rs`).

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::screens::select::Confirm;

use super::strings::Strings;

/// The two rows of a confirmation, No first (`SelectionViewParams::confirmation`:
/// the panel picker with the standard hint, its columns placed from all rows
/// and the description hidden when narrow).
pub fn confirmation(confirm: Confirm, s: &Strings) -> List {
    let (title, subtitle, no, yes) = match confirm {
        Confirm::Archive { running } => (
            s.archive_title,
            if running {
                s.archive_subtitle_running
            } else {
                s.archive_subtitle
            },
            (s.archive_no, s.archive_no_description),
            (s.archive_yes, s.archive_yes_description),
        ),
        Confirm::Delete => (
            s.delete_title,
            s.delete_subtitle,
            (s.delete_no, s.delete_no_description),
            (s.delete_yes, s.delete_yes_description),
        ),
    };
    let row = |id: &str, (name, description): (&str, &str)| Item {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        ..Item::default()
    };
    List {
        title: Some(title.to_string()),
        subtitle: Some(subtitle.to_string()),
        standard_hint: true,
        items: vec![row(Confirm::NO, no), row(Confirm::YES, yes)],
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
