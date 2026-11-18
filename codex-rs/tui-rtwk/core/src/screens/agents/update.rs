//! The intents of the screen (`update`): what a key or a click means, the
//! state it changes and the RPCs it asks for.
//! Oracle: `agents_overview_view.rs` (`handle_key_event` arms),
//! `agents_overview_actions.rs` (archive, delete, hide, rename, pin).

use super::Action;
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
use codex_app_server_protocol::ThreadStatus;

use super::refresh::Out;
use super::refresh::list;
use super::refresh::out;

/// The intent's change and the RPCs it asks for.
pub fn update(model: &mut Model, intent: Intent) -> Vec<Out> {
    let mut outs = Vec::new();
    if model.failure.is_some() {
        // The failure dialog holds the screen until `Back to agents`.
        if intent == Intent::DismissFailure {
            model.failure = None;
        }
        return outs;
    }
    if blocked(model, &intent) {
        return outs;
    }
    match intent {
        Intent::Open => {
            model.paging.initialized = false;
            model.removed.clear();
            outs.push(list(model, None, false));
        }
        Intent::Refresh => outs.push(list(model, None, false)),
        Intent::CycleFilter { back } => {
            let count = FILTER_GROUPS.len();
            let step = if back { count - 1 } else { 1 };
            model.filter = (model.filter + step) % count;
        }
        Intent::SetFilter(filter) => {
            model.filter = filter.min(FILTER_GROUPS.len() - 1);
        }
        Intent::CycleGrouping => model.grouping = model.grouping.next(),
        Intent::SetGrouping(grouping) => model.grouping = grouping,
        Intent::ToggleHelp => model.help = !model.help,
        Intent::CloseHelp => model.help = false,
        Intent::BeginSearch => {
            if model.rename.is_none() {
                model.searching = true;
            }
        }
        Intent::SetSearch(text) => outs.extend(set_search(model, text)),
        Intent::CancelEditing => cancel_editing(model),
        Intent::BeginRename(thread) => {
            if model.rename.is_none() && model.row(&thread).is_some() {
                model.rename = Some(Rename { target: thread });
                model.searching = false;
                model.search.clear();
                model.history = None;
                model.search_matches.clear();
            }
        }
        Intent::Accept(at) => accept(model, at, &mut outs),
        Intent::Right(at) => {
            if !editing(model) {
                accept(model, at, &mut outs);
            }
        }
        Intent::Commit(text) => commit_rename(model, text, &mut outs),
        Intent::Stop(thread) => {
            if let Some(row) = model.row(&thread)
                && matches!(row.thread.status, ThreadStatus::Active { .. })
            {
                outs.push(out(
                    Command::ActiveTurn {
                        thread: thread.clone(),
                    },
                    Ctx::Stop { thread },
                ));
            }
        }
        // Handled above, while a failure holds the screen.
        Intent::DismissFailure => {}
        Intent::Archive(thread) => confirm(model, thread, Action::Archive),
        Intent::Delete(thread) => confirm(model, thread, Action::Delete),
        Intent::Confirmed(yes) => {
            if let Some(confirm) = model.confirm.take()
                && yes
            {
                model.notice = Some(Notice::Removing(confirm.action));
                let command = match confirm.action {
                    Action::Archive => Command::Archive {
                        thread: confirm.thread.clone(),
                    },
                    Action::Delete => Command::Delete {
                        thread: confirm.thread.clone(),
                    },
                };
                // Hidden at once, until the request ends (oracle:
                // `hidden_threads` in `run_agents_overview_action`).
                model.hidden.insert(confirm.thread.clone());
                outs.push(out(
                    command,
                    Ctx::Action {
                        thread: confirm.thread,
                        action: Some(confirm.action),
                    },
                ));
            }
        }
        Intent::Hide(id) => {
            if model.row(&id).is_some() {
                if model
                    .rename
                    .as_ref()
                    .is_some_and(|rename| rename.target == id)
                {
                    model.rename = None;
                }
                model.hidden.insert(id);
            }
        }
        Intent::TogglePin(thread) => {
            if model.pinned.is_some()
                && !model.pin_pending
                && let Some(row) = model.row(&thread)
                && supports_shared_pinning(&row.thread.source)
            {
                let pinned = !row.pinned;
                model.pin_pending = true;
                outs.push(out(
                    Command::Pin {
                        thread: thread.clone(),
                        pinned,
                    },
                    Ctx::Pin {
                        thread,
                        pinned,
                    },
                ));
            }
        }
        Intent::New(at) => {
            let cwd = cwd_of(model, at);
            outs.push(out(Command::Start { cwd }, Ctx::Start));
        }
        Intent::NewWorktree(at) => {
            if model.worktrees {
                let cwd = cwd_of(model, at);
                model.notice = Some(Notice::CreatingWorktree);
                outs.push(out(Command::NewWorktree { cwd }, Ctx::NewWorktree));
            }
        }
        Intent::Fork(thread) => {
            if model.row(&thread).is_some() {
                outs.push(out(
                    Command::Fork { thread },
                    Ctx::Fork,
                ));
            }
        }
        Intent::NeedUsage(at) => outs.extend(super::usage::refresh_usage(model, at)),
    }
    outs
}

fn confirm(model: &mut Model, thread: String, action: Action) {
    if model.row(&thread).is_some() {
        model.confirm = Some(Confirm { thread, action });
    }
}

/// The directory of the task `at`, if there is one.
fn cwd_of(model: &Model, at: Option<String>) -> Option<String> {
    model.row(&at?).map(|row| row.thread.cwd.display().to_string())
}

fn editing(model: &Model) -> bool {
    model.searching || model.rename.is_some()
}

/// The text of the search line changed; the first match is highlighted
/// again (oracle: `edit_input`).
fn set_search(model: &mut Model, text: String) -> Vec<Out> {
    if model.searching && model.rename.is_none() {
        model.search = text;
        let outs = super::discovery::restart_history(model);
        return outs;
    }
    Vec::new()
}

/// Whether the server's lock or the connection holds the intent back: while
/// the center is offline, removing a task or creating a worktree, only moving,
/// searching, filtering and leaving work (oracle: the gate in
/// `handle_key_event`).
fn blocked(model: &Model, intent: &Intent) -> bool {
    let held = matches!(
        model.notice,
        Some(
            Notice::Reconnecting
                | Notice::ReconnectFailed
                | Notice::Removing(_)
                | Notice::CreatingWorktree
        )
    );
    held && matches!(
        intent,
        Intent::Accept(_)
            | Intent::Right(_)
            | Intent::CycleGrouping
            | Intent::SetGrouping(_)
            | Intent::TogglePin(_)
            | Intent::New(_)
            | Intent::NewWorktree(_)
            | Intent::Fork(_)
            | Intent::BeginRename(_)
            | Intent::Archive(_)
            | Intent::Delete(_)
            | Intent::Hide(_)
            | Intent::Stop(_)
    )
}

/// Oracle: `on_ctrl_c` of the view for the editing case.
fn cancel_editing(model: &mut Model) {
    if !editing(model) {
        return;
    }
    model.searching = false;
    model.rename = None;
    model.search.clear();
    model.history = None;
    model.search_matches.clear();
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
    }
}

fn accept(model: &mut Model, at: Sel, outs: &mut Vec<Out>) {
    if at == Sel::ShowMore {
        if model.searching {
            outs.extend(super::discovery::load_matches(
                model,
                crate::screens::agents::PAGE_SIZE as usize,
            ));
        } else {
            outs.extend(super::discovery::show_more(model));
        }
        return;
    }
    if model.rename.is_some() {
        return;
    }
    if let Sel::Task(thread) = at
        && model.row(&thread).is_some()
    {
        outs.push(out(
            Command::Resume {
                thread: thread.clone(),
            },
            Ctx::Resume { thread },
        ));
        if model.searching {
            model.searching = false;
            model.history = None;
            model.search_matches.clear();
            model.search.clear();
        }
    }
}

/// The `cwd`s of the center's tasks whose directory keys the shell has not
/// been asked for, marked as asked (oracle: `AgentsOverviewProjectGroup::
/// for_thread`, `directory_group_key`, read while building the rows). Only
/// with worktrees on: the keys are used by nothing else.
pub fn wants_dirs(model: &Model) -> bool {
    model.worktrees
        && model
            .threads
            .iter()
            .chain(&model.search_matches)
            .any(|thread| !model.dirs_asked.contains(thread.cwd.display().to_string().as_str()))
}

pub fn wanted_dirs(model: &mut Model) -> Vec<String> {
    if !model.worktrees {
        return Vec::new();
    }
    let cwds: Vec<String> = model
        .threads
        .iter()
        .chain(&model.search_matches)
        .map(|thread| thread.cwd.display().to_string())
        .collect();
    let mut wanted = Vec::new();
    for cwd in cwds {
        if model.dirs_asked.insert(cwd.clone()) {
            wanted.push(cwd);
        }
    }
    wanted
}
