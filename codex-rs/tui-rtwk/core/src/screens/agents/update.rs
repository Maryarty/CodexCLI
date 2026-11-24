//! The intents of the screen (`update`): what a key or a click means, the
//! state it changes and the RPCs it asks for.
//! Oracle: `agents_overview_view.rs` (`handle_key_event` arms),
//! `agents_overview_actions.rs` (archive, delete, hide, rename, pin).

use crate::threads::ThreadAsk;
use super::Action;
use super::Command;
use super::Ctx;
use super::Intent;
use super::Model;
use super::Notice;
use super::carried::supports_shared_pinning;
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
        Intent::CycleGrouping => {
            let next = model.grouping.next();
            outs.extend(set_grouping(model, next));
        }
        Intent::SetGrouping(grouping) => outs.extend(set_grouping(model, grouping)),
        Intent::SetSearch { text, lens } => {
            model.search = text;
            outs.extend(super::discovery::restart_history(model, &lens));
        }
        Intent::ClearSearch => clear_search(model),
        Intent::ShowMore => outs.extend(super::discovery::show_more(model)),
        Intent::CancelOpen => {}
        Intent::MoreMatches(lens) => outs.extend(super::discovery::load_matches(
            model,
            crate::screens::agents::PAGE_SIZE as usize,
            &lens,
        )),
        Intent::Rename { thread, name } => rename(model, thread, name, &mut outs),
        Intent::Stop(thread) => {
            if let Some(task) = model.task(&thread)
                && matches!(task.status, ThreadStatus::Active { .. })
            {
                outs.push(out(
                    Command::Thread(ThreadAsk::ActiveTurn {
                        thread: thread.clone(),
                    }),
                    Ctx::Stop { thread },
                ));
            }
        }
        // Handled above, while a failure holds the screen.
        Intent::DismissFailure => {}
        Intent::Archive(thread) => remove(model, thread, Action::Archive, &mut outs),
        Intent::Delete(thread) => remove(model, thread, Action::Delete, &mut outs),
        Intent::Hide(id) => {
            if model.has_task(&id) {
                model.hidden.insert(id);
            }
        }
        Intent::TogglePin(thread) => {
            if model.pin_action_supported()
                && !model.pin_pending
                && let Some(task) = model.task(&thread)
                && supports_shared_pinning(&task.source)
            {
                let pinned = model.pin_rank(&thread).is_none();
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
            outs.push(out(Command::Thread(ThreadAsk::Start { cwd }), Ctx::Start));
        }
        Intent::NewWorktree(at) => {
            if model.worktrees {
                let cwd = cwd_of(model, at);
                model.notice = Some(Notice::CreatingWorktree);
                outs.push(out(Command::NewWorktree { cwd }, Ctx::NewWorktree));
            }
        }
        Intent::Fork(thread) => {
            if model.has_task(&thread) {
                outs.push(out(
                    Command::Thread(ThreadAsk::Fork {
                        thread: thread.clone(),
                    }),
                    Ctx::Fork { thread },
                ));
            }
        }
        Intent::NeedUsage(at) => outs.extend(super::usage::refresh_usage(model, at)),
    }
    outs
}

/// The grouping changes; `Directory` is view-local, the others are written to
/// the config (oracle: `PersistAgentsOverviewGrouping`).
fn set_grouping(model: &mut Model, grouping: super::Grouping) -> Option<Out> {
    model.grouping = grouping;
    grouping.persisted().map(|name| {
        out(
            Command::SaveGrouping {
                grouping: name.to_owned(),
            },
            Ctx::SaveGrouping,
        )
    })
}

/// Archive or delete the task (oracle: `run_agents_overview_action`, after
/// the confirmation the view asked).
fn remove(model: &mut Model, thread: String, action: Action, outs: &mut Vec<Out>) {
    if !model.has_task(&thread) {
        return;
    }
    model.notice = Some(Notice::Removing(action));
    let command = match action {
        Action::Archive => Command::Archive {
            thread: thread.clone(),
        },
        Action::Delete => Command::Delete {
            thread: thread.clone(),
        },
    };
    // Hidden at once, until the request ends (oracle: `hidden_threads` in
    // `run_agents_overview_action`).
    model.hidden.insert(thread.clone());
    outs.push(out(
        command,
        Ctx::Action {
            thread,
            action: Some(action),
        },
    ));
}

/// The directory of the task `at`, if there is one.
fn cwd_of(model: &Model, at: Option<String>) -> Option<String> {
    model.task(&at?).map(|task| task.cwd.display().to_string())
}

/// Whether the server's lock or the connection holds the intent back: while
/// the center is offline, removing a task or creating a worktree, only moving,
/// searching, filtering and leaving work (oracle: the gate in
/// `handle_key_event`).
fn blocked(model: &Model, intent: &Intent) -> bool {
    model.held() && matches!(
        intent,
        Intent::ShowMore
            | Intent::CycleGrouping
            | Intent::SetGrouping(_)
            | Intent::TogglePin(_)
            | Intent::New(_)
            | Intent::NewWorktree(_)
            | Intent::Fork(_)
            | Intent::Archive(_)
            | Intent::Delete(_)
            | Intent::Hide(_)
            | Intent::Stop(_)
    )
}

/// The search line closed (oracle: `on_ctrl_c` of the view for the editing
/// case, and a resume from a search).
fn clear_search(model: &mut Model) {
    model.search.clear();
    model.history = None;
    model.search_matches.clear();
}

/// The task is named `name` (oracle: `activate`).
fn rename(model: &Model, thread: String, name: String, outs: &mut Vec<Out>) {
    let name = name.trim().to_owned();
    if name.is_empty() || !model.has_task(&thread) {
        return;
    }
    outs.push(out(
        Command::Rename {
            thread: thread.clone(),
            name,
        },
        Ctx::Rename { thread },
    ));
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
            .any(|thread| !model.dirs_asked.contains(thread.cwd.to_string_lossy().as_ref()))
}

pub fn wanted_dirs(model: &mut Model) -> Vec<String> {
    if !model.worktrees {
        return Vec::new();
    }
    let mut wanted = Vec::new();
    for thread in model.threads.iter().chain(&model.search_matches) {
        let cwd = thread.cwd.to_string_lossy();
        if !model.dirs_asked.contains(cwd.as_ref()) {
            model.dirs_asked.insert(cwd.to_string());
            wanted.push(cwd.into_owned());
        }
    }
    wanted
}
