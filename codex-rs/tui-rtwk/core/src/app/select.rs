//! The RPCs of the select screen (S14) the app sends and reads: `model/list`
//! for `/model`, and `configRequirements/read` followed by the pages of
//! `permissionProfile/list` for `/permissions`. The screen's model decides
//! what to ask (`Catalog::pending`, `Profiles::pending`: the request an
//! answer must belong to); the app gives each a ticket, sends it as a screen
//! command with no thread and turns the answer into the screen's events.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::{Answer, RpcError};
use crate::screens::select::collaboration::{Change, PlanWrite, ReasoningScope};
use crate::screens::select::models::{Apply, Current, Effort, Scope};
use crate::screens::select::{self, apps, permissions, review};
use crate::screens::{ScreenCommand, ScreenReply, ScreenThreadIntent, agents, chat, startup};
use crate::threads::ThreadEv;
use crate::threads::queue::{
    self, ModalKind, ModalOperation, ModalToken, QueueEvent, QueueOperation,
};

use super::features::{
    acknowledge_queue_modal, cancel_queue_modal, modal_address, queue_modal_can_reopen,
    queue_modal_is_current, register_queue_modal,
};

pub(super) mod presets;

/// An intent of the select screen, and the asks it left open.
pub(super) fn intent(app: &mut CodexApp, mut intent: select::Intent, fx: &mut AppFx) {
    if let select::Intent::ActInApps { token, action } = intent {
        let operation = token.map(ModalOperation::Apps);
        if operation
            .as_ref()
            .is_none_or(|operation| queue_modal_is_current(app, operation))
        {
            act_in_apps(
                app,
                operation.as_ref().map(ModalOperation::thread),
                action,
                fx,
            );
        }
        return;
    }
    if let select::Intent::RetryOwnedPermissions(token) = intent {
        if queue_modal_is_current(app, &ModalOperation::Permissions(token)) {
            self::intent(app, select::Intent::LoadProfiles, fx);
        }
        return;
    }
    if let select::Intent::PlanAnswer(answer) = intent {
        super::session::plan_answer(app, answer, fx);
        return;
    }
    match &mut intent {
        select::Intent::Reasoning { current, plan, .. } if current.scope.is_some() => {
            let Some(live) = live_current(app, current) else {
                return;
            };
            *plan = live.plan.is_some();
            *current = live;
        }
        select::Intent::Settle(select::ModelCompletion::Inline {
            selection,
            scope: Some(scope),
        }) => {
            if !accepts_model_scope(app, scope, selection) {
                return;
            }
            retire_model_picker(app);
        }
        _ => {}
    }
    match intent {
        select::Intent::Settle(select::ModelCompletion::Queued { operation, result }) => {
            match result {
                select::ModelResult::Picked(selection) => {
                    settle_queued_picker(app, operation, selection, fx)
                }
                select::ModelResult::GaveUp(why) => {
                    model_picker_gave_up(app, &operation, why, fx);
                    queued_picker_event(app, &operation, QueueEvent::PickerClosed, fx);
                }
            }
            return;
        }
        select::Intent::Settle(select::ModelCompletion::QueuedPageGaveUp { operation, why }) => {
            model_picker_gave_up(app, &operation, why, fx);
            return;
        }
        _ => {}
    }
    let queued = if let select::Intent::OpenModels(request) = &intent
        && let Some(operation) = &request.operation
    {
        if !begin_queued_picker(app, operation, fx) {
            return;
        }
        if app.session.active.as_ref() != Some(operation.thread())
            || !app
                .threads
                .get(operation.thread())
                .is_some_and(|thread| thread.has_writer())
        {
            queued_picker_event(app, operation, QueueEvent::PickerClosed, fx);
            return;
        }
        Some(operation.clone())
    } else {
        None
    };
    if let select::Intent::OpenModels(request) = &mut intent
        && request.current.scope.is_some()
    {
        let Some(mut current) = live_current(app, &request.current) else {
            if let Some(operation) = queued {
                queued_picker_event(app, &operation, QueueEvent::PickerClosed, fx);
            }
            return;
        };
        retire_model_picker(app);
        if let Some(scope) = &mut current.scope {
            scope.visit = app.screens.select.model_visit;
            scope.address = Some(app.address(app.screens.agents.owner_of(scope.thread.as_str())));
        }
        request.current = current;
    }
    let modal = if let select::Intent::OpenQueueDialog(open) = &intent {
        let kind = match open.dialog {
            select::QueueDialog::Permissions(_) => ModalKind::Permissions,
            select::QueueDialog::Apps { .. } => ModalKind::Apps,
            select::QueueDialog::Preset { .. } => ModalKind::Preset,
        };
        let Some(operation) =
            register_queue_modal(app, &open.thread, open.issued.clone(), kind, fx)
        else {
            return;
        };
        Some(operation)
    } else {
        None
    };
    let screens = Rc::make_mut(&mut app.screens);
    let before = (
        screens.select.catalog.pending,
        screens.select.profiles.pending,
    );
    let apps = screens.select.apps.pending;
    if let (select::Intent::OpenQueueDialog(open), Some(operation)) = (&intent, &modal) {
        screens
            .select
            .open_queue_dialog(open.dialog.clone(), operation.clone());
    } else {
        select::apply_intent(&mut screens.select, intent, fx);
    }
    let declined_modal = modal.filter(|operation| {
        !screens
            .select
            .shows
            .pending()
            .any(|show| show.queue_modal.as_ref() == Some(operation))
    });
    let declined = queued.filter(|operation| {
        !screens.select.shows.pending().any(|show| {
            show.queued_model
                .as_ref()
                .is_some_and(|queued| queued.operation() == operation)
        })
    });
    let after = (
        screens.select.catalog.pending,
        screens.select.profiles.pending,
    );
    asked_apps(app, apps, fx);
    if let Some(request) = after.0.filter(|_| after.0 != before.0) {
        ask(
            app,
            select::Command::Models { request },
            select::Ctx::Models { request },
            fx,
        );
    }
    if let Some(request) = after.1.filter(|_| after.1 != before.1) {
        ask(
            app,
            select::Command::Requirements { request },
            select::Ctx::Requirements { request },
            fx,
        );
    }
    review_asked(app, fx);
    worktree_asked(app, fx);
    quick_asked(app, fx);
    picks(app, fx);
    drain(app, fx);
    if let Some(operation) = declined {
        queued_picker_event(app, &operation, QueueEvent::PickerClosed, fx);
    }
    if let Some(operation) = declined_modal {
        cancel_queue_modal(app, operation, fx);
    }
}

fn set_app(
    app: &mut CodexApp,
    id: String,
    enabled: bool,
    owner: Option<apps::ModalWriteOwner>,
    fx: &mut AppFx,
) {
    ask(
        app,
        select::Command::SetApp {
            id: id.clone(),
            enabled,
        },
        select::Ctx::App { id, enabled, owner },
        fx,
    );
}

fn act_in_apps(
    app: &mut CodexApp,
    thread: Option<&ThreadId>,
    action: apps::ModalAction,
    fx: &mut AppFx,
) {
    match action {
        apps::ModalAction::Retry => intent(app, select::Intent::RefreshApps, fx),
        apps::ModalAction::Visit(id) => {
            if let Some(url) = app
                .screens
                .select
                .apps
                .app(&id)
                .and_then(|found| found.install_url.clone())
            {
                fx.host(crate::app::effects::Ask::open_url_or_report(url));
            }
        }
        apps::ModalAction::Enable { id, enabled } => {
            if thread.is_some() && !app.screens.select.app_installed(&id) {
                return;
            }
            let owner = thread.map(|thread| apps::ModalWriteOwner {
                thread: thread.clone(),
                scope: app.screens.select.apps.scope(),
            });
            set_app(app, id, enabled, owner, fx);
        }
        apps::ModalAction::Unlinked(id) => {
            Rc::make_mut(&mut app.screens).select.app_unlinked(&id);
            drain(app, fx);
        }
    }
}

pub(super) fn finish_owned_apps(app: &mut CodexApp, choice: apps::ModalChoice, fx: &mut AppFx) {
    let action = match choice {
        apps::ModalChoice::Unlinked(id) => apps::ModalAction::Unlinked(id),
        apps::ModalChoice::RefreshAfterVisit => apps::ModalAction::Retry,
    };
    act_in_apps(app, None, action, fx);
}

fn live_current(app: &CodexApp, asked: &Current) -> Option<Current> {
    let origin = asked.scope.as_ref()?;
    if app.session.active.as_ref() != Some(&origin.thread) {
        return None;
    }
    let thread = app.threads.get(&origin.thread)?;
    let owner = app.screens.agents.owner_of(origin.thread.as_str());
    Some(Current {
        reserve: asked.reserve.clone(),
        scope: asked.scope.clone(),
        ..Current::from_thread(
            &thread.screen_slots().chat,
            &app.screens.chat.status,
            app.screens.chat.flags.collaboration_modes_enabled,
            app.screens.select.plan_defaults.get(&owner),
            &app.modes,
        )
    })
}

fn accepts_model_scope(app: &CodexApp, scope: &Scope, selection: &select::Selection) -> bool {
    if scope.visit != app.screens.select.model_visit
        || app.session.active.as_ref() != Some(&scope.thread)
        || scope.address != Some(app.address(app.screens.agents.owner_of(scope.thread.as_str())))
    {
        return false;
    }
    if matches!(selection.apply, Apply::PlanOnly | Apply::AllModes) {
        return plan_thread(app, &scope.thread, &selection.model);
    }
    true
}

pub(super) fn retire_model_picker(app: &mut CodexApp) {
    let select = &mut Rc::make_mut(&mut app.screens).select;
    select.model_visit = select.model_visit.wrapping_add(1);
}

pub(super) fn configured_plan(app: &mut CodexApp, backend: BackendId, effort: Option<Effort>) {
    if app.screens.select.plan_defaults.get(&backend) == effort.as_ref() {
        return;
    }
    let defaults = &mut Rc::make_mut(&mut app.screens).select.plan_defaults;
    if let Some(effort) = effort {
        defaults.insert(backend, effort);
    } else {
        defaults.remove(&backend);
    }
}

/// The command must still be issued for this thread. Keep its continuation before asking the
/// catalog or handing the picker to the presenter.
fn begin_queued_picker(app: &mut CodexApp, operation: &QueueOperation, fx: &mut AppFx) -> bool {
    let report = app
        .threads
        .get(operation.thread())
        .and_then(|thread| thread.due_command())
        .filter(|due| due.operation() == operation)
        .map(crate::threads::DueCommand::opening_model_picker);
    let Some(report) = report else {
        return false;
    };
    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(report));
    super::apply_thread(app, operation.thread(), ThreadEv::ScreenIntent(intent), fx);
    true
}

fn queued_picker_event(
    app: &mut CodexApp,
    operation: &QueueOperation,
    event: impl FnOnce(QueueOperation) -> QueueEvent,
    fx: &mut AppFx,
) {
    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(event(operation.clone())));
    super::apply_thread(app, operation.thread(), ThreadEv::ScreenIntent(intent), fx);
}

fn settle_queued_picker(
    app: &mut CodexApp,
    operation: QueueOperation,
    selection: select::Selection,
    fx: &mut AppFx,
) {
    let applicable = app.session.active.as_ref() == Some(operation.thread())
        && app
            .threads
            .get(operation.thread())
            .is_some_and(|thread| queue::can_apply_model(thread, &operation));
    if !applicable {
        queued_picker_event(app, &operation, QueueEvent::PickerClosed, fx);
        return;
    }
    queued_picker_event(app, &operation, QueueEvent::PickerAccepted, fx);
    model_pick(app, selection, Some(operation), fx);
    drain(app, fx);
}

fn model_picker_gave_up(
    app: &mut CodexApp,
    operation: &QueueOperation,
    why: select::models::GaveUp,
    fx: &mut AppFx,
) {
    if app
        .threads
        .get(operation.thread())
        .is_some_and(|thread| queue::has_model_picker(thread, operation))
    {
        write_notes(app, Some(operation), [select::model_picker_note(why)], fx);
    }
}

/// The preset writes the menus settled on, asked of the config (one `config/batchWrite` each).
fn quick_asked(app: &mut CodexApp, fx: &mut AppFx) {
    let wanted = Rc::make_mut(&mut app.screens).select.quick.take_wanted();
    for write in wanted {
        let select::quick_dials::Write { slot, dial } = write;
        ask(
            app,
            select::Command::Preset {
                slot,
                dial: dial.clone(),
            },
            select::Ctx::Preset { slot, dial },
            fx,
        );
    }
}

/// The git list a `/review` menu waits for, asked of the host (once per request).
fn review_asked(app: &mut CodexApp, fx: &mut AppFx) {
    use super::effects::Ask;
    let Some(wanted) = Rc::make_mut(&mut app.screens).select.review.take_wanted() else {
        return;
    };
    let ask = match wanted.git {
        review::Git::Branches => Ask::GitBranches { cwd: wanted.cwd },
        review::Git::Log => Ask::GitLog {
            cwd: wanted.cwd,
            limit: review::RECENT_COMMITS,
        },
    };
    fx.commands
        .push(super::effects::command(wanted.request, ask));
}

/// The worktree list or removal the menus want, asked of the host.
fn worktree_asked(app: &mut CodexApp, fx: &mut AppFx) {
    use super::effects::Ask;
    use select::worktree::Want;
    let wanted = Rc::make_mut(&mut app.screens).select.worktree.take_wanted();
    for wanted in wanted {
        let cwd = wanted.cwd;
        let ask = match wanted.want {
            Want::List => Ask::Worktrees { cwd },
            Want::Remove { root } => Ask::WorktreeRemove { cwd, root },
        };
        fx.commands
            .push(super::effects::command(wanted.request, ask));
    }
}

/// The host's list for a `/review` menu: the child menu opens over the presets. False when
/// `request` is not the one the menus wait for (the answer is the UI's own).
pub(super) fn reviewed(
    app: &mut CodexApp,
    request: u64,
    done: &super::effects::Done,
    fx: &mut AppFx,
) -> bool {
    use super::effects::Done;
    if app.screens.select.worktree.waits_for(request) {
        let ev = match done {
            Done::Worktrees(result) => select::Ev::Worktrees {
                request,
                result: result.clone(),
            },
            Done::WorktreeRemoved(result) => select::Ev::WorktreeRemoved {
                request,
                result: result.clone(),
            },
            _ => return false,
        };
        event(app, ev, fx);
        return true;
    }
    if !app.screens.select.review.waits_for(request) {
        return false;
    }
    let listing = match done {
        Done::GitBranches { current, branches } => review::Listing::Branches {
            current: current.clone(),
            branches: branches.clone(),
        },
        Done::GitLog(commits) => review::Listing::Log(commits.clone()),
        _ => return false,
    };
    event(app, select::Ev::Reviewed { request, listing }, fx);
    true
}

/// What the screen said (`add_info_message` of the pickers) goes into the
/// transcript of the thread in use, as typed notes: the screen keeps no
/// history of its own.
fn drain(app: &mut CodexApp, fx: &mut AppFx) {
    if app.screens.select.hold_queue {
        Rc::make_mut(&mut app.screens).select.hold_queue = false;
        if let Some(thread) = app.session.active.clone() {
            let intent = crate::screens::chat::ThreadIntent::HoldQueue;
            let ev = crate::threads::ThreadEv::ScreenIntent(
                crate::screens::ScreenThreadIntent::Chat(intent),
            );
            super::apply_thread(app, &thread, ev, fx);
        }
    }
    if app.screens.select.notes.is_empty() {
        return;
    }
    let said = std::mem::take(&mut Rc::make_mut(&mut app.screens).select.notes);
    let thread = app.session.active.clone();
    for note in said.into_iter().filter(select::Note::in_transcript) {
        let note = crate::threads::Note::from(crate::threads::NoteKey::Select(note));
        super::session_note(app, thread.clone(), note, fx);
    }
}

/// The picks the screen holds, applied: the thread's settings change for its
/// next turns, a default pick is saved, and the history line follows what the
/// server accepted (`select_session_model`, `persist_default_model_selection`,
/// `apply_permission_profile_selection`).
fn picks(app: &mut CodexApp, fx: &mut AppFx) {
    use select::Picked::{Effort, Permissions, Selection, Tier};
    let ours = |pick: &select::Picked| {
        matches!(
            pick,
            Selection(_)
                | Permissions(_)
                | Tier(_)
                | Effort { .. }
                | select::Picked::Review(_)
                | select::Picked::Worktree(_)
        )
    };
    if !app.screens.select.picked.iter().any(ours) {
        return;
    }
    // The picks of the generic lists and the confirmations belong to the
    // screens that opened them: they stay.
    let mut picked = Vec::new();
    Rc::make_mut(&mut app.screens).select.picked.retain(|pick| {
        if ours(pick) {
            picked.push(pick.clone());
            false
        } else {
            true
        }
    });
    for pick in picked {
        match pick {
            Selection(selection) => model_pick(app, selection, None, fx),
            Permissions(selection) => permissions_pick(app, selection, fx),
            Tier(tier) => tier_pick(app, &tier, fx),
            Effort {
                model,
                effort,
                plan,
            } => effort_pick(app, &model, effort, plan, fx),
            // `AppEvent::Review`: `review/start`, inline on the thread in use.
            select::Picked::Review(target) => {
                let thread = app.session.active.clone();
                super::session::start_review(app, thread, target, fx);
            }
            select::Picked::Worktree(pick) => worktree_pick(app, pick, fx),
            _ => {}
        }
    }
}

/// What the worktree menus settled on: `/new` and `/fork` in the current checkout, a new
/// worktree for `/new`, the owner thread opened, the directory copied.
fn worktree_pick(app: &mut CodexApp, pick: select::worktree::Pick, fx: &mut AppFx) {
    use crate::commands::SlashCommand;
    use select::worktree::{Conversation, Pick};
    let source = app.session.active.clone();
    match pick {
        Pick::Current { mode, name } => {
            let command = match mode {
                Conversation::New => SlashCommand::New,
                Conversation::Fork => SlashCommand::Fork,
            };
            super::session::begin_conversation(app, source, command, name, fx);
        }
        // The creation chain (`worktree.rs`: create, start or fork, bind, show) serves `/new`,
        // `/fork` and `/worktree`'s continue and start; its checks and lines are the chat's.
        Pick::Start { mode, name } => {
            let fork = mode == Conversation::Fork;
            super::worktree::Step::create_for_chat(app, source, fork, name, fx);
        }
        Pick::Resume(thread) => super::apply_intent(app, super::Intent::OpenThread(thread), fx),
        Pick::Copy(text) => {
            let request = select::worktree::REQUEST_BIT | (1 << 40);
            fx.commands.push(super::effects::command(
                request,
                super::effects::Ask::Copy {
                    text,
                    format: super::effects::CopyFormat::PlainText,
                    destination: super::effects::CopyDestination::Clipboard,
                },
            ));
        }
        Pick::Remove { .. } => {}
    }
}

/// An event of the select screen, and the `app/list` it left to ask (a final
/// answer may start the refresh a notification or a forced ask waited for;
/// the app server's directory notification comes through here too).
pub(super) fn event(app: &mut CodexApp, ev: select::Ev, fx: &mut AppFx) {
    let before = app.screens.select.apps.pending;
    select::apply(&mut Rc::make_mut(&mut app.screens).select, ev, fx);
    asked_apps(app, before, fx);
    picks(app, fx);
    drain(app, fx);
}

/// The directory of apps is read (not forced): a surface that lists the apps without `/apps`.
pub(super) fn refresh_apps(app: &mut CodexApp, fx: &mut AppFx) {
    let before = app.screens.select.apps.pending;
    Rc::make_mut(&mut app.screens).select.apps.refresh(false);
    asked_apps(app, before, fx);
}

/// The `app/list` the screen left pending since `before`, asked for.
fn asked_apps(app: &mut CodexApp, before: Option<apps::Request>, fx: &mut AppFx) {
    let Some(pending) = app
        .screens
        .select
        .apps
        .pending
        .filter(|now| Some(*now) != before)
    else {
        return;
    };
    let thread = app.session.active.as_ref().map(ToString::to_string);
    ask(
        app,
        select::Command::Apps {
            request: pending.request,
            force: pending.force,
            thread,
        },
        select::Ctx::Apps {
            request: pending.request,
        },
        fx,
    );
}

fn notes(app: &mut CodexApp, notes: impl IntoIterator<Item = select::Note>) {
    Rc::make_mut(&mut app.screens).select.notes.extend(notes);
}

fn show_owned_note(app: &mut CodexApp, thread: ThreadId, note: select::Note, fx: &mut AppFx) {
    let note = crate::threads::Note::from(crate::threads::NoteKey::Select(note));
    super::session_note(app, Some(thread), note, fx);
}

fn write_notes(
    app: &mut CodexApp,
    operation: Option<&QueueOperation>,
    said: impl IntoIterator<Item = select::Note>,
    fx: &mut AppFx,
) {
    let Some(operation) = operation else {
        notes(app, said);
        return;
    };
    write_notes_on(app, operation.thread(), said, fx);
}

fn write_notes_on(
    app: &mut CodexApp,
    thread: &ThreadId,
    said: impl IntoIterator<Item = select::Note>,
    fx: &mut AppFx,
) {
    if app.threads.contains(thread) {
        for note in said.into_iter().filter(select::Note::in_transcript) {
            show_owned_note(app, thread.clone(), note, fx);
        }
    }
}

fn settings(
    app: &mut CodexApp,
    patch: crate::threads::SettingsPatch,
    setting: select::Setting,
    then: Option<select::Note>,
    operation: Option<QueueOperation>,
    fx: &mut AppFx,
) -> bool {
    let Some(thread) = operation
        .as_ref()
        .map(QueueOperation::thread)
        .or(app.session.active.as_ref())
        .map(ToString::to_string)
    else {
        return false;
    };
    ask(
        app,
        select::Command::Settings { thread, patch },
        select::Ctx::Settings {
            setting,
            then,
            operation,
        },
        fx,
    );
    true
}

/// `PublicVerb::OverrideTurn`: the thread's model and effort for its next turns, as a session
/// pick without the line (`override_turn_context`); with no thread there is nothing to override.
pub(super) fn turn_override(
    app: &mut CodexApp,
    model: String,
    effort: Option<select::models::Effort>,
    fx: &mut AppFx,
) {
    let patch = crate::threads::SettingsPatch {
        model: Some(model),
        effort,
        ..crate::threads::SettingsPatch::default()
    };
    settings(app, patch, select::Setting::Model, None, None, fx);
}

/// A model picked by a verb of a feature: no select step drains its line, so it does.
pub(super) fn model_verb(app: &mut CodexApp, selection: select::Selection, fx: &mut AppFx) {
    model_pick(app, selection, None, fx);
    drain(app, fx);
}

fn model_pick(
    app: &mut CodexApp,
    selection: select::Selection,
    operation: Option<QueueOperation>,
    fx: &mut AppFx,
) {
    let effects = select::persist::effects(&selection);
    if let Some(threads) = selection.warning {
        notes(app, [select::Note::UltraConcurrency(threads)]);
    }
    if matches!(selection.apply, Apply::PlanOnly | Apply::AllModes) {
        let thread = operation
            .as_ref()
            .map(QueueOperation::thread)
            .or(app.session.active.as_ref())
            .filter(|thread| plan_thread(app, thread, &selection.model))
            .cloned();
        let Some(thread) = thread else {
            if let Some(operation) = operation {
                queued_picker_event(app, &operation, QueueEvent::ModelApplied, fx);
            }
            return;
        };
        let backend = app.screens.agents.owner_of(thread.as_str());
        let issued = app.mint.next();
        ask(
            app,
            select::Command::Write {
                edits: effects.edits,
            },
            select::Ctx::PlanWrite(Box::new(PlanWrite {
                thread,
                backend,
                issued,
                selection,
                operation,
            })),
            fx,
        );
        return;
    }
    let session = selection.apply == Apply::Session;
    let line = select::Note::ModelChanged {
        model: selection.model,
        effort: selection.effort,
        session,
    };
    let has_thread_update = effects.patch.is_some();
    if let Some(patch) = effects.patch {
        settings(
            app,
            patch,
            select::Setting::Model,
            None,
            operation.clone(),
            fx,
        );
    }
    if effects.edits.is_empty() {
        if session {
            notes(app, [line]);
        }
        return;
    }
    let queued = operation.map(|operation| {
        if has_thread_update {
            select::QueuedModelWrite::SavingDefault(operation)
        } else {
            select::QueuedModelWrite::Applying(operation)
        }
    });
    ask(
        app,
        select::Command::Write {
            edits: effects.edits,
        },
        select::Ctx::Write {
            setting: select::Setting::Model,
            then: Some(line),
            queued,
        },
        fx,
    );
}

fn plan_thread(app: &CodexApp, thread: &ThreadId, model: &str) -> bool {
    use codex_protocol::config_types::ModeKind;
    app.screens.chat.flags.collaboration_modes_enabled
        && app.threads.get(thread).is_some_and(|thread| {
            let chat = &thread.screen_slots().chat;
            thread.has_writer()
                && chat.mode() == ModeKind::Plan
                && chat.model.as_deref() == Some(model)
        })
}

fn plan_effort(
    app: &mut CodexApp,
    thread: &ThreadId,
    effort: Option<Effort>,
    scope: ReasoningScope,
    operation: Option<QueueOperation>,
    fx: &mut AppFx,
) {
    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::SetMask(Change::PlanEffort {
        effort,
        scope,
        operation,
    }));
    super::apply_thread(app, thread, ThreadEv::ScreenIntent(intent), fx);
}

fn plan_written(
    app: &mut CodexApp,
    write: PlanWrite,
    result: Result<select::Reply, RpcError>,
    fx: &mut AppFx,
) {
    let PlanWrite {
        thread,
        backend,
        issued,
        selection,
        operation,
    } = write;
    let all_modes = selection.apply == Apply::AllModes;
    let owns_thread =
        app.threads.contains(&thread) && app.screens.agents.owner_of(thread.as_str()) == backend;
    let setting = if all_modes {
        select::Setting::Model
    } else {
        select::Setting::PlanEffort
    };
    let note = match result {
        Ok(select::Reply::Written { overridden: false })
            if app
                .screens
                .select
                .last_plan_write
                .get(&backend)
                .is_none_or(|last| *last < issued) =>
        {
            Rc::make_mut(&mut app.screens)
                .select
                .last_plan_write
                .insert(backend, issued);
            configured_plan(app, backend, selection.effort.clone());
            if owns_thread && plan_thread(app, &thread, &selection.model) {
                let scope = if all_modes {
                    ReasoningScope::AllModes
                } else {
                    ReasoningScope::PlanOnly
                };
                plan_effort(app, &thread, selection.effort.clone(), scope, operation, fx);
                if all_modes {
                    let note = select::Note::ModelChanged {
                        model: selection.model,
                        effort: selection.effort,
                        session: false,
                    };
                    super::session_note(
                        app,
                        Some(thread),
                        crate::threads::NoteKey::Select(note).into(),
                        fx,
                    );
                }
                return;
            }
            None
        }
        Ok(select::Reply::Written { overridden: true }) => Some(select::Note::Overridden(setting)),
        Ok(_) => None,
        Err(error) => Some(select::Note::SaveFailed {
            setting,
            error: text(error),
        }),
    };
    if owns_thread {
        if let Some(note) = note {
            super::session_note(
                app,
                Some(thread),
                crate::threads::NoteKey::Select(note).into(),
                fx,
            );
        }
        if let Some(operation) = operation {
            queued_picker_event(app, &operation, QueueEvent::ModelApplied, fx);
        }
    }
}

/// A reasoning shortcut's step (`UpdateReasoningEffort`, `UpdatePlanModeReasoningEffort`): the
/// thread's effort for its next turns, saved nowhere; in Plan mode it rides on the mode
/// the thread is in.
fn effort_pick(
    app: &mut CodexApp,
    model: &str,
    effort: select::models::Effort,
    plan: bool,
    fx: &mut AppFx,
) {
    if plan {
        if let Some(thread) = app.session.active.clone()
            && plan_thread(app, &thread, model)
        {
            plan_effort(
                app,
                &thread,
                Some(effort),
                ReasoningScope::Session,
                None,
                fx,
            );
        }
        return;
    }
    let patch = crate::threads::SettingsPatch {
        effort: Some(effort),
        ..Default::default()
    };
    settings(app, patch, select::Setting::Model, None, None, fx);
}

/// A tier command of the model: the thread's tier changes for its next turns, the default is
/// saved, and the history says so once the write was taken.
fn tier_pick(app: &mut CodexApp, tier: &str, fx: &mut AppFx) {
    let effects = select::persist::tier_effects(tier);
    if let Some(patch) = effects.patch {
        settings(app, patch, select::Setting::ServiceTier, None, None, fx);
    }
    ask(
        app,
        select::Command::Write {
            edits: effects.edits,
        },
        select::Ctx::Write {
            setting: select::Setting::ServiceTier,
            then: Some(select::Note::ServiceTierSet(tier.to_owned())),
            queued: None,
        },
        fx,
    );
}

pub(super) fn permissions_pick(
    app: &mut CodexApp,
    selection: permissions::Selection,
    fx: &mut AppFx,
) {
    // The sandbox of a Windows host is the missing piece of an Auto pick: the enable prompt
    // opens and the pick waits for the setup (`approval_preset_actions`).
    if let Some(ask) = crate::features::setup_before_pick(&app.features, &selection) {
        super::features::intent(app, ask, fx);
        return;
    }
    permissions_apply(app, selection, fx);
}

/// A permissions pick that is applied at once: the sheet's own, or the one the Windows sandbox
/// setup held back and now lets go (`PublicVerb::PickPermissions`).
pub(super) fn permissions_apply(
    app: &mut CodexApp,
    selection: permissions::Selection,
    fx: &mut AppFx,
) {
    let label = match &selection {
        permissions::Selection::Preset { preset, reviewer } => permissions::Label::Mode {
            preset: *preset,
            reviewer: *reviewer,
        },
        permissions::Selection::Profile { label, .. } => label.clone(),
    };
    let patch = select::persist::permissions_patch(&selection);
    match &selection {
        // A catalog pick of a thread: asked of the server, the line follows
        // its answer; a profile the thread has says nothing
        // (`apply_permission_profile_selection`, `already_selected`).
        permissions::Selection::Profile { id, unchanged, .. } => {
            if *unchanged && id.starts_with(':') {
                Rc::make_mut(&mut app.screens).select.profiles.shortcut = false;
                return;
            }
            let then = select::Note::PermissionSelectionRequested(label);
            if !settings(
                app,
                patch,
                select::Setting::Permissions,
                Some(then),
                None,
                fx,
            ) {
                Rc::make_mut(&mut app.screens).select.profiles.shortcut = false;
            }
        }
        // Without a thread the legacy list has nothing to update: the
        // oracle's config is the host's, and its line is still said.
        permissions::Selection::Preset { .. } => {
            settings(app, patch, select::Setting::Permissions, None, None, fx);
            notes(app, [select::Note::PermissionsUpdated(label)]);
            // a verb is no select step: nothing else drains the line
            drain(app, fx);
        }
    }
}

pub(super) fn apply_owned_permissions(
    app: &mut CodexApp,
    token: ModalToken<permissions::Selection>,
    selection: permissions::Selection,
    fx: &mut AppFx,
) {
    let thread = token.thread().clone();
    let label = match &selection {
        permissions::Selection::Preset { preset, reviewer } => permissions::Label::Mode {
            preset: *preset,
            reviewer: *reviewer,
        },
        permissions::Selection::Profile { label, .. } => label.clone(),
    };
    if !queue_modal_can_reopen(app, &ModalOperation::Permissions(token.clone()))
        || app
            .screens
            .select
            .permissions_applying
            .contains_key(&thread)
    {
        settle_owned_permission(app, token, true, fx);
        return;
    }
    let patch = select::persist::permissions_patch(&selection);
    let address = modal_address(app, &thread);
    let effective = app
        .screens
        .select
        .permissions_effective
        .get(&thread)
        .is_some_and(|effective| effective.address == address && effective.matches(&patch));
    let setup = crate::features::setup_before_owned_pick(&app.features, &token, &selection);
    if setup.is_none()
        && effective
        && patch
            .permissions
            .as_ref()
            .is_some_and(|id| id.starts_with(':'))
    {
        settle_owned_permission(app, token, false, fx);
        return;
    }
    Rc::make_mut(&mut app.screens)
        .select
        .permissions_applying
        .insert(
            thread.clone(),
            permissions::Applying {
                token: token.clone(),
                address,
                patch: patch.clone(),
                deferred: setup.is_some().then_some(selection),
                accepted: false,
                effective,
            },
        );
    if let Some(setup) = setup {
        super::features::intent(app, setup, fx);
        return;
    }
    ask(
        app,
        select::Command::Settings {
            thread: thread.to_string(),
            patch,
        },
        select::Ctx::OwnedPermissionWrite {
            token,
            note: select::Note::PermissionSelectionRequested(label),
        },
        fx,
    );
}

fn settle_owned_permission(
    app: &mut CodexApp,
    token: ModalToken<permissions::Selection>,
    refused: bool,
    fx: &mut AppFx,
) {
    if refused {
        let event = ScreenThreadIntent::Chat(chat::ThreadIntent::HoldQueue);
        super::apply_thread(app, token.thread(), ThreadEv::ScreenIntent(event), fx);
    }
    acknowledge_queue_modal(app, ModalOperation::Permissions(token), fx);
}

fn settle_effective_permission(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    if app
        .screens
        .select
        .permissions_applying
        .get(thread)
        .is_some_and(|pending| pending.accepted && pending.effective)
        && let Some(pending) = Rc::make_mut(&mut app.screens)
            .select
            .permissions_applying
            .remove(thread)
    {
        settle_owned_permission(app, pending.token, false, fx);
    }
}

pub(super) fn opening_permission_baseline(
    ctx: &AppCtx,
    answer: &Answer,
) -> Option<(ThreadId, permissions::Snapshot)> {
    let Answer::Thread(reply) = answer else {
        return None;
    };
    let (id, snapshot) = reply.permission_baseline()?;
    let matches = match ctx {
        AppCtx::OpenThread { thread, .. }
        | AppCtx::OpenSession {
            action: startup::Action::Resume,
            thread,
        } => thread.as_str() == id,
        AppCtx::Agents(agents::Ctx::Resume { thread }) => thread == id,
        AppCtx::StartThread { .. }
        | AppCtx::NewConversation { .. }
        | AppCtx::Agents(agents::Ctx::Start | agents::Ctx::Fork { .. })
        | AppCtx::OpenSession {
            action: startup::Action::Fork,
            ..
        } => true,
        _ => false,
    };
    matches.then(|| (ThreadId::new(id), snapshot.clone()))
}

pub(super) fn record_permission_settings(
    app: &mut CodexApp,
    thread: &ThreadId,
    observed: permissions::Effective,
    fx: &mut AppFx,
) {
    let backend = observed.address.backend;
    if !app.threads.contains(thread)
        || app.address(backend) != observed.address
        || app.screens.agents.owners.get(thread.as_str()) != Some(&backend)
    {
        return;
    }
    let select = &mut Rc::make_mut(&mut app.screens).select;
    // A settings notification since the opening request is newer than its snapshot.
    if select
        .permissions_effective
        .get(thread)
        .is_some_and(|prior| prior.address == observed.address && prior.at >= observed.at)
    {
        return;
    }
    if let Some(pending) = select.permissions_applying.get_mut(thread)
        && pending.address == observed.address
    {
        pending.effective = observed.matches(&pending.patch);
    }
    select
        .permissions_effective
        .insert(thread.clone(), observed);
    settle_effective_permission(app, thread, fx);
}

pub(super) fn forget_permission_settings(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    let select = &mut Rc::make_mut(&mut app.screens).select;
    select.permissions_effective.remove(thread);
    if let Some(pending) = select.permissions_applying.remove(thread) {
        discard_owned_permission_setup(app, &pending.token, fx);
    }
}

pub(super) fn retire_permission_writes(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    let mut lost = Vec::new();
    let select = &mut Rc::make_mut(&mut app.screens).select;
    select
        .permissions_effective
        .retain(|_, effective| effective.address.backend != backend);
    select.permissions_applying.retain(|_, pending| {
        if pending.address.backend != backend {
            return true;
        }
        lost.push(pending.token.clone());
        false
    });
    for token in lost {
        discard_owned_permission_setup(app, &token, fx);
        settle_owned_permission(app, token, true, fx);
    }
}

pub(super) fn start_owned_permission_setup(
    app: &mut CodexApp,
    token: ModalToken<permissions::Selection>,
    mode: codex_app_server_protocol::WindowsSandboxSetupMode,
    fx: &mut AppFx,
) {
    let deferred = app.screens.select.permissions_applying.get(token.thread());
    if deferred.is_some_and(|write| write.deferred.is_some() && write.token == token)
        && queue_modal_can_reopen(app, &ModalOperation::Permissions(token.clone()))
    {
        super::features::run(app, fx, |states, cx| {
            crate::features::start_owned_permission_setup(states, &token, mode, cx);
        });
    } else {
        finish_owned_permission_setup(app, token, false, fx);
    }
}

pub(super) fn finish_owned_permission_setup(
    app: &mut CodexApp,
    token: ModalToken<permissions::Selection>,
    success: bool,
    fx: &mut AppFx,
) {
    let pending = &mut Rc::make_mut(&mut app.screens).select.permissions_applying;
    let Some(selection) = pending
        .get_mut(token.thread())
        .filter(|write| write.token == token)
        .and_then(|write| write.deferred.take())
    else {
        return;
    };
    pending.remove(token.thread());
    discard_owned_permission_setup(app, &token, fx);
    if success {
        apply_owned_permissions(app, token, selection, fx);
    } else {
        settle_owned_permission(app, token, true, fx);
    }
}

fn discard_owned_permission_setup(
    app: &mut CodexApp,
    token: &ModalToken<permissions::Selection>,
    fx: &mut AppFx,
) {
    super::features::run(app, fx, |states, _| {
        crate::features::clear_owned_permission_setup(states, token)
    });
}

fn permission_write_answer(
    app: &mut CodexApp,
    token: ModalToken<permissions::Selection>,
    note: select::Note,
    result: Result<select::Reply, RpcError>,
    fx: &mut AppFx,
) {
    let thread = token.thread().clone();
    let pending = &mut Rc::make_mut(&mut app.screens).select.permissions_applying;
    let Some(write) = pending
        .get_mut(&thread)
        .filter(|write| write.token == token)
    else {
        return;
    };
    let error = match result {
        Ok(select::Reply::Updated) => {
            write.accepted = true;
            show_owned_note(app, thread.clone(), note, fx);
            settle_effective_permission(app, &thread, fx);
            return;
        }
        Err(error) => error,
        Ok(_) => RpcError::Mismatch,
    };
    pending.remove(&thread);
    let note = select::Note::SaveFailed {
        setting: select::Setting::Permissions,
        error: text(error),
    };
    show_owned_note(app, thread, note, fx);
    settle_owned_permission(app, token, true, fx);
}

pub(in crate::app) fn ask(
    app: &mut CodexApp,
    command: select::Command,
    ctx: select::Ctx,
    fx: &mut AppFx,
) {
    let thread = match &ctx {
        select::Ctx::Settings {
            operation: Some(operation),
            ..
        } => operation.thread().clone(),
        select::Ctx::Write {
            queued: Some(queued),
            ..
        } => queued.operation().thread().clone(),
        select::Ctx::PlanWrite(write) => write.thread.clone(),
        select::Ctx::OwnedPermissionWrite { token, .. } => token.thread().clone(),
        select::Ctx::OwnedPreset { token, .. } => token.thread().clone(),
        select::Ctx::App {
            owner: Some(owner), ..
        } => owner.thread.clone(),
        _ => ThreadId::default(),
    };
    fx.ask(app, AppCtx::Select(ctx), |_, ticket| Command::Screen {
        ticket,
        thread,
        command: ScreenCommand::Select(command),
    });
}

fn failure(error: RpcError) -> permissions::Failure {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) => permissions::Failure::Other(text),
        RpcError::Mismatch => permissions::Failure::Other(String::new()),
        RpcError::Prepare(text) => permissions::Failure::Other(text),
        RpcError::Provider(error) => permissions::Failure::Other(error.to_string()),
    }
}

fn text(error: RpcError) -> String {
    match failure(error) {
        permissions::Failure::Other(text) => text,
        _ => String::new(),
    }
}

/// The answer of one of those RPCs.
pub(super) fn answer(app: &mut CodexApp, ctx: select::Ctx, answer: Answer, fx: &mut AppFx) {
    let operation = match &ctx {
        select::Ctx::Settings { operation, .. } => operation.clone(),
        select::Ctx::Write {
            queued: Some(select::QueuedModelWrite::Applying(operation)),
            ..
        } => Some(operation.clone()),
        _ => None,
    };
    answered(app, ctx, answer, fx);
    // The catalog an inline `/permissions <profile>` waited for may have made its pick.
    picks(app, fx);
    drain(app, fx);
    if let Some(operation) = operation {
        queued_picker_event(app, &operation, QueueEvent::ModelApplied, fx);
    }
}

fn answered(app: &mut CodexApp, ctx: select::Ctx, answer: Answer, fx: &mut AppFx) {
    let result = match answer {
        Answer::Screen(ScreenReply::Select(reply)) => Ok(reply),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    let event = |app: &mut CodexApp, ev: select::Ev, fx: &mut AppFx| {
        select::apply(&mut Rc::make_mut(&mut app.screens).select, ev, fx);
    };
    match (ctx, result) {
        (select::Ctx::PlanWrite(write), result) => plan_written(app, *write, result, fx),
        (select::Ctx::OwnedPermissionWrite { token, note }, result) => {
            permission_write_answer(app, token, note, result, fx);
        }
        (
            select::Ctx::Settings {
                setting,
                then,
                operation,
            },
            result,
        ) => {
            // `complete_permission_shortcut`: the pick of a shortcut has its answer.
            if setting == select::Setting::Permissions {
                Rc::make_mut(&mut app.screens).select.profiles.shortcut = false;
            }
            let note = match result {
                Ok(_) => then,
                Err(error) => Some(select::Note::SaveFailed {
                    setting,
                    error: text(error),
                }),
            };
            write_notes(app, operation.as_ref(), note, fx);
        }
        (
            select::Ctx::Write {
                setting,
                then,
                queued,
            },
            result,
        ) => {
            let operation = queued.as_ref().map(select::QueuedModelWrite::operation);
            match result {
                Ok(select::Reply::Written { overridden }) => {
                    let override_note = overridden.then_some(select::Note::Overridden(setting));
                    write_notes(app, operation, override_note.into_iter().chain(then), fx);
                }
                Ok(_) => {}
                Err(error) => write_notes(
                    app,
                    operation,
                    [select::Note::SaveFailed {
                        setting,
                        error: text(error),
                    }],
                    fx,
                ),
            }
        }
        // `SetAppEnabled`: the directory takes the switch once the config has it; a refused
        // write leaves it as it was.
        (select::Ctx::App { id, enabled, owner }, Ok(select::Reply::Written { .. })) => {
            if owner.is_none_or(|owner| {
                app.session.active.as_ref() == Some(&owner.thread)
                    && app.screens.select.apps.scope() == owner.scope
            }) {
                event(app, select::Ev::AppSwitched { id, enabled }, fx);
            }
        }
        // `SetAppEnabled` refused: said as an error line; the dialog keeps its switch (the oracle's
        // view does too).
        (select::Ctx::App { id, owner, .. }, Err(error)) => {
            let note = select::Note::SaveFailed {
                setting: select::Setting::App(id),
                error: text(error),
            };
            if let Some(owner) = owner {
                show_owned_note(app, owner.thread, note, fx);
            } else {
                notes(app, [note]);
            }
        }
        (select::Ctx::App { .. }, Ok(_)) => {}
        // `save_quick_dial`: the slot follows a saved write; a refused one is said, with the
        // editor shown again.
        (select::Ctx::OwnedPreset { token, write }, result) => {
            presets::answered(app, token, write, result, fx);
        }
        (select::Ctx::Preset { slot, dial }, result) => {
            let result = match result {
                Ok(_) => Ok(()),
                Err(error) => Err(text(error)),
            };
            event(app, select::Ev::PresetWritten { slot, dial, result }, fx);
        }
        (select::Ctx::Apps { request }, result) => {
            let result = match result {
                Ok(select::Reply::Apps(apps)) => Ok(apps),
                Ok(_) => Err(String::new()),
                Err(error) => Err(text(error)),
            };
            self::event(app, select::Ev::AppsLoaded { request, result }, fx);
        }
        // `collaboration_modes::list`: discovery is optional, a failed read leaves no presets
        (select::Ctx::Collaboration, result) => {
            app.modes = match result {
                Ok(select::Reply::Collaboration(masks)) => masks,
                _ => Vec::new(),
            };
        }
        (select::Ctx::Models { request }, result) => {
            let result = match result {
                Ok(select::Reply::Models(presets)) => Ok(presets),
                Ok(_) => Err(String::new()),
                Err(error) => Err(text(error)),
            };
            event(app, select::Ev::Fetched { request, result }, fx);
        }
        (select::Ctx::Fallback { backend }, result) => {
            super::banners::catalog_answer(app, backend, result, fx);
        }
        (
            select::Ctx::Switch {
                backend,
                thread,
                model,
                effort,
                reason,
            },
            result,
        ) => super::banners::switched(app, (backend, thread, model, effort, reason), result, fx),
        (select::Ctx::Requirements { request }, result) => match result {
            Ok(select::Reply::Requirements(requirements)) => {
                let ctx = select::Ctx::Profiles {
                    request,
                    requirements,
                    found: Vec::new(),
                    pages: 0,
                };
                ask_page(app, ctx, None, fx);
            }
            Ok(_) => discovered(
                app,
                request,
                Err(permissions::Failure::Other(String::new())),
                fx,
            ),
            Err(error) => discovered(app, request, Err(failure(error)), fx),
        },
        (
            select::Ctx::Profiles {
                request,
                requirements,
                mut found,
                pages,
            },
            result,
        ) => match result {
            Ok(select::Reply::Profiles { profiles, next }) => {
                found.extend(profiles);
                let pages = pages + 1;
                match next {
                    Some(_) if pages >= select::MAX_PAGES => {
                        discovered(app, request, Err(permissions::Failure::Pagination), fx);
                    }
                    Some(cursor) => {
                        let ctx = select::Ctx::Profiles {
                            request,
                            requirements,
                            found,
                            pages,
                        };
                        ask_page(app, ctx, Some(cursor), fx);
                    }
                    None => discovered(
                        app,
                        request,
                        permissions::Discovery::new(found, requirements),
                        fx,
                    ),
                }
            }
            Ok(_) => discovered(
                app,
                request,
                Err(permissions::Failure::Other(String::new())),
                fx,
            ),
            Err(error) => discovered(app, request, Err(failure(error)), fx),
        },
    }
}

fn ask_page(app: &mut CodexApp, ctx: select::Ctx, cursor: Option<String>, fx: &mut AppFx) {
    let select::Ctx::Profiles { request, .. } = ctx else {
        return;
    };
    let command = select::Command::Profiles {
        request,
        cursor,
        cwd: None,
    };
    ask(app, command, ctx, fx);
}

fn discovered(
    app: &mut CodexApp,
    request: u64,
    result: Result<permissions::Discovery, permissions::Failure>,
    fx: &mut AppFx,
) {
    select::apply(
        &mut Rc::make_mut(&mut app.screens).select,
        select::Ev::Discovered { request, result },
        fx,
    );
}
