//! The RPCs of the select screen (S14) the app sends and reads: `model/list`
//! for `/model`, and `configRequirements/read` followed by the pages of
//! `permissionProfile/list` for `/permissions`. The screen's model decides
//! what to ask (`Catalog::pending`, `Profiles::pending`: the request an
//! answer must belong to); the app gives each a ticket, sends it as a screen
//! command with no thread and turns the answer into the screen's events.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::select::{self, apps, permissions, review};
use crate::screens::{ScreenCommand, ScreenReply};

/// An intent of the select screen, and the asks it left open.
pub(super) fn intent(app: &mut CodexApp, intent: select::Intent, fx: &mut AppFx) {
    // The dialog of the app shows its switch at once; the directory follows the saved write.
    if let select::Intent::SetApp { id, enabled } = intent {
        ask(
            app,
            select::Command::SetApp {
                id: id.clone(),
                enabled,
            },
            select::Ctx::App { id, enabled },
            fx,
        );
        return;
    }
    if let select::Intent::PlanAnswer(answer) = intent {
        super::session::plan_answer(app, answer, fx);
        return;
    }
    // `AppEvent::OpenUrlInBrowser`: the page of the app the link dialog shows.
    if let select::Intent::OpenAppPage(id) = &intent {
        if let Some(url) = app
            .screens
            .select
            .apps
            .app(id)
            .and_then(|found| found.install_url.clone())
        {
            fx.host(crate::app::effects::Ask::open_url_or_report(url));
        }
        return;
    }
    let screens = Rc::make_mut(&mut app.screens);
    let before = (
        screens.select.catalog.pending,
        screens.select.profiles.pending,
    );
    let apps = screens.select.apps.pending;
    select::apply_intent(&mut screens.select, intent, fx);
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
            Selection(selection) => model_pick(app, selection, fx),
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

fn settings(
    app: &mut CodexApp,
    patch: crate::threads::SettingsPatch,
    setting: select::Setting,
    then: Option<select::Note>,
    fx: &mut AppFx,
) -> bool {
    let Some(thread) = app.session.active.as_ref().map(ToString::to_string) else {
        return false;
    };
    ask(
        app,
        select::Command::Settings { thread, patch },
        select::Ctx::Settings { setting, then },
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
    settings(app, patch, select::Setting::Model, None, fx);
}

/// A model picked by a verb of a feature: no select step drains its line, so it does.
pub(super) fn model_verb(app: &mut CodexApp, selection: select::Selection, fx: &mut AppFx) {
    model_pick(app, selection, fx);
    drain(app, fx);
}

fn model_pick(app: &mut CodexApp, selection: select::Selection, fx: &mut AppFx) {
    use select::models::Apply;
    let effects = select::persist::effects(&selection);
    if let Some(threads) = selection.warning {
        notes(app, [select::Note::UltraConcurrency(threads)]);
    }
    let session = selection.apply == Apply::Session;
    let line = select::Note::ModelChanged {
        model: selection.model.clone(),
        effort: selection.effort.clone(),
        session,
    };
    if let Some(patch) = effects.patch {
        settings(app, patch, select::Setting::Model, None, fx);
    }
    if effects.edits.is_empty() {
        // Plan mode only saves a key and says nothing of the model.
        if session {
            notes(app, [line]);
        }
        return;
    }
    let setting = if selection.apply == Apply::PlanOnly {
        select::Setting::PlanEffort
    } else {
        select::Setting::Model
    };
    let then = (selection.apply != Apply::PlanOnly).then_some(line);
    ask(
        app,
        select::Command::Write {
            edits: effects.edits,
        },
        select::Ctx::Write { setting, then },
        fx,
    );
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
    let patch = if plan {
        let mut mode = crate::screens::chat::plan_mode::collaboration_mode(
            codex_protocol::config_types::ModeKind::Plan,
            Some(model),
        );
        mode.settings.reasoning_effort = Some(effort);
        crate::threads::SettingsPatch {
            collaboration_mode: Some(mode),
            ..Default::default()
        }
    } else {
        crate::threads::SettingsPatch {
            effort: Some(effort),
            ..Default::default()
        }
    };
    settings(app, patch, select::Setting::Model, None, fx);
}

/// A tier command of the model: the thread's tier changes for its next turns, the default is
/// saved, and the history says so once the write was taken.
fn tier_pick(app: &mut CodexApp, tier: &str, fx: &mut AppFx) {
    let effects = select::persist::tier_effects(tier);
    if let Some(patch) = effects.patch {
        settings(app, patch, select::Setting::ServiceTier, None, fx);
    }
    ask(
        app,
        select::Command::Write {
            edits: effects.edits,
        },
        select::Ctx::Write {
            setting: select::Setting::ServiceTier,
            then: Some(select::Note::ServiceTierSet(tier.to_owned())),
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
            if !settings(app, patch, select::Setting::Permissions, Some(then), fx) {
                Rc::make_mut(&mut app.screens).select.profiles.shortcut = false;
            }
        }
        // Without a thread the legacy list has nothing to update: the
        // oracle's config is the host's, and its line is still said.
        permissions::Selection::Preset { .. } => {
            settings(app, patch, select::Setting::Permissions, None, fx);
            notes(app, [select::Note::PermissionsUpdated(label)]);
            // a verb is no select step: nothing else drains the line
            drain(app, fx);
        }
    }
}

pub(in crate::app) fn ask(
    app: &mut CodexApp,
    command: select::Command,
    ctx: select::Ctx,
    fx: &mut AppFx,
) {
    fx.ask(app, AppCtx::Select(ctx), |_, ticket| Command::Screen {
        ticket,
        thread: ThreadId::default(),
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
    answered(app, ctx, answer, fx);
    // The catalog an inline `/permissions <profile>` waited for may have made its pick.
    picks(app, fx);
    drain(app, fx);
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
        (select::Ctx::Settings { setting, then }, result) => {
            // `complete_permission_shortcut`: the pick of a shortcut has its answer.
            if setting == select::Setting::Permissions {
                Rc::make_mut(&mut app.screens).select.profiles.shortcut = false;
            }
            match result {
                Ok(_) => notes(app, then),
                Err(error) => notes(
                    app,
                    [select::Note::SaveFailed {
                        setting,
                        error: text(error),
                    }],
                ),
            }
        }
        (select::Ctx::Write { setting, then }, result) => match result {
            Ok(select::Reply::Written { overridden }) => {
                if overridden {
                    notes(app, [select::Note::Overridden(setting)]);
                }
                notes(app, then);
            }
            Ok(_) => {}
            Err(error) => notes(
                app,
                [select::Note::SaveFailed {
                    setting,
                    error: text(error),
                }],
            ),
        },
        // `SetAppEnabled`: the directory takes the switch once the config has it; a refused
        // write leaves it as it was.
        (select::Ctx::App { id, enabled }, Ok(select::Reply::Written { .. })) => {
            event(app, select::Ev::AppSwitched { id, enabled }, fx);
        }
        // `SetAppEnabled` refused: said as an error line; the dialog keeps its switch (the oracle's
        // view does too).
        (select::Ctx::App { id, .. }, Err(error)) => notes(
            app,
            [select::Note::SaveFailed {
                setting: select::Setting::App(id),
                error: text(error),
            }],
        ),
        (select::Ctx::App { .. }, Ok(_)) => {}
        // `save_quick_dial`: the slot follows a saved write; a refused one is said, with the
        // editor shown again.
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
