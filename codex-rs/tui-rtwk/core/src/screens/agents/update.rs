//! The intents of the screen (`update`): what a key or a click means, the
//! state it changes and the RPCs it asks for.
//! Oracle: `agents_overview_view.rs` (`handle_key_event` arms),
//! `agents_overview_actions.rs` (archive, delete, hide, rename, pin).

use codex_app_server_protocol::ThreadStatus;
use super::Action;
use super::CenterRow;
use super::Command;
use super::Confirm;
use super::Ctx;
use super::Intent;
use super::Model;
use super::Notice;
use super::Rename;
use super::Sel;
use super::carried::supports_shared_pinning;
use super::center::FILTER_GROUPS;

use super::refresh::Out;
use super::refresh::list;
use super::refresh::out;

/// The intent's change and the RPCs it asks for.
pub fn update(model: &mut Model, intent: Intent) -> Vec<Out> {
    let mut outs = Vec::new();
    match intent {
        Intent::Open | Intent::Refresh => outs.push(list(model, None, false)),
        Intent::Select(sel) => {
            if model.rename.is_none() {
                model.selected = Some(sel);
                model.reconcile();
            }
        }
        Intent::CycleFilter { back } => {
            let count = FILTER_GROUPS.len();
            let step = if back { count - 1 } else { 1 };
            model.filter = (model.filter + step) % count;
            model.reconcile();
        }
        Intent::SetFilter(filter) => {
            model.filter = filter.min(FILTER_GROUPS.len() - 1);
            model.reconcile();
        }
        Intent::CycleGrouping => model.grouping = model.grouping.next(),
        Intent::SetGrouping(grouping) => model.grouping = grouping,
        Intent::ToggleHelp => model.help = !model.help,
        Intent::CloseHelp => model.help = false,
        Intent::BeginSearch => {
            if model.rename.is_none() {
                model.browse = model.selected.clone();
                model.searching = true;
            }
        }
        Intent::SetSearch(text) => set_search(model, text),
        Intent::CancelEditing => cancel_editing(model),
        Intent::BeginRename => {
            let center = model.center();
            if model.rename.is_none()
                && let Some(row) = center.selected_row()
            {
                model.rename = Some(Rename {
                    target: row.id().to_owned(),
                });
                model.searching = false;
                model.search.clear();
            }
        }
        Intent::Accept => accept(model, &mut outs),
        Intent::Commit(text) => commit_rename(model, text, &mut outs),
        Intent::Stop => {
            if let Some(row) = model.center().selected_row()
                && matches!(row.thread.status, ThreadStatus::Active { .. })
            {
                let thread = row.id().to_owned();
                outs.push(out(
                    Command::ActiveTurn {
                        thread: thread.clone(),
                    },
                    Ctx::Stop { thread },
                ));
            }
        }
        Intent::Archive => confirm(model, Action::Archive),
        Intent::Delete => confirm(model, Action::Delete),
        Intent::Confirmed(yes) => {
            if let Some(confirm) = model.confirm.take()
                && yes
            {
                let next = model.selection_after_removal(std::slice::from_ref(&confirm.thread));
                model.notice = Some(Notice::Removing(confirm.action));
                let command = match confirm.action {
                    Action::Archive => Command::Archive {
                        thread: confirm.thread.clone(),
                    },
                    Action::Delete => Command::Delete {
                        thread: confirm.thread.clone(),
                    },
                };
                model.selected = next.map(Sel::Task);
                outs.push(out(
                    command,
                    Ctx::Action {
                        thread: confirm.thread,
                        action: Some(confirm.action),
                    },
                ));
            }
        }
        Intent::Hide => {
            let center = model.center();
            if let Some(row) = center.selected_row() {
                let id = row.id().to_owned();
                let next = model.selection_after_removal(std::slice::from_ref(&id));
                if model.rename.as_ref().is_some_and(|rename| rename.target == id) {
                    model.rename = None;
                }
                model.hidden.insert(id);
                model.selected = next.map(Sel::Task);
                model.reconcile();
            }
        }
        Intent::TogglePin => {
            let center = model.center();
            if model.pinned.is_some()
                && let Some(row) = center.selected_row()
                && supports_shared_pinning(&row.thread.source)
            {
                let thread = row.id().to_owned();
                let pinned = !row.pinned;
                outs.push(out(
                    Command::Pin {
                        thread: thread.clone(),
                        pinned,
                    },
                    Ctx::Pin { thread },
                ));
            }
        }
        Intent::New => {
            let center = model.center();
            let cwd = center
                .selected_row()
                .map(|row| row.thread.cwd.display().to_string());
            outs.push(out(Command::Start { cwd }, Ctx::Start));
        }
    }
    outs
}

fn confirm(model: &mut Model, action: Action) {
    let center = model.center();
    if let Some(row) = center.selected_row() {
        model.confirm = Some(Confirm {
            thread: row.id().to_owned(),
            action,
        });
    }
}

fn editing(model: &Model) -> bool {
    model.searching || model.rename.is_some()
}

/// The text of the search line changed; the first match is highlighted
/// again (oracle: `edit_input`).
fn set_search(model: &mut Model, text: String) {
    if model.searching && model.rename.is_none() {
        model.search = text;
        search_changed(model);
    }
}

/// The first match is highlighted again (oracle: `edit_input`).
fn search_changed(model: &mut Model) {
    let center = model.center();
    model.selected = center
        .entries
        .iter()
        .find_map(|entry| match entry {
            CenterRow::Task(index) => center.rows.get(*index).map(|row| Sel::Task(row.id().to_owned())),
            _ => None,
        });
}

/// Oracle: `on_ctrl_c` of the view for the editing case.
fn cancel_editing(model: &mut Model) {
    if !editing(model) {
        return;
    }
    let restore = model.searching.then(|| model.browse.take());
    model.searching = false;
    model.rename = None;
    model.search.clear();
    if let Some(selected) = restore {
        model.selected = selected;
    }
    model.reconcile();
}

/// Enter in the rename line: the task is named `text` (oracle: `activate`).
fn commit_rename(model: &mut Model, text: String, outs: &mut Vec<Out>) {
    let name = text.trim().to_owned();
    if name.is_empty() {
        return;
    }
    if let Some(rename) = model.rename.take() {
        outs.push(out(
            Command::Rename {
                thread: rename.target.clone(),
                name,
            },
            Ctx::Rename {
                thread: rename.target,
            },
        ));
        model.reconcile();
    }
}

fn accept(model: &mut Model, outs: &mut Vec<Out>) {
    let center = model.center();
    if center.show_more_selected {
        if !model.paging.loading {
            let cursor = model.paging.cursor.clone();
            outs.push(list(model, cursor, true));
        }
        return;
    }
    if model.rename.is_some() {
        return;
    }
    if let Some(row) = center.selected_row() {
        let thread = row.id().to_owned();
        outs.push(out(
            Command::Resume {
                thread: thread.clone(),
            },
            Ctx::Resume { thread },
        ));
        if model.searching {
            model.searching = false;
            model.search.clear();
        }
    }
}
