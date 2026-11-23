//! The automatic model switch a usage banner asks for (oracle
//! `app/backend_banner_fallback.rs`, `chatwidget/backend_banners.rs`): when the account read says
//! the model in use is blocked, the thread moves to the banner's first usable fallback by itself;
//! Luna Reserve is entered the same way and left again once ordinary usage is back. The switch is
//! a `thread/settings/update` like a manual pick, with its own ticket: the model of the thread
//! changes when the server says so (`thread/settings/updated`), the note and the banner's
//! notice follow the accepted update.

use std::rc::Rc;

use codex_protocol::openai_models::ReasoningEffort;

use super::{AppCtx, AppFx, CodexApp, model_in_use, select::ask, session_note, with_banner};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::RpcError;
use crate::screens::chat::{AutomaticSwitch, BannerOccurrence, SwitchReason};
use crate::screens::select::models::{Preset, RESERVE_MODEL};
use crate::screens::select::{self, collaboration};
use crate::threads::{ModeNote, Note, NoteKey, SettingsPatch};

/// Where a thread returns to when Luna Reserve ends (`ReserveReturnModel`): saved before the
/// switch into Reserve, for the account the switch was read for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReserveReturn {
    account_id: String,
    model: String,
    effort: Option<ReasoningEffort>,
}

/// What the account read asks of the thread.
enum Want {
    /// Away from a blocked model, to the banner's fallback (or into Reserve).
    Away,
    /// Back from Reserve to the model it replaced.
    Back(ReserveReturn),
}

/// The switch the account read of `backend` wants for the thread on display
/// (`apply_backend_banner_fallback`): asks the catalog first when the app has none yet.
pub(super) fn fallback(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    let Some(thread) = app.session.active.clone() else {
        return;
    };
    // A backend that is not ChatGPT-backed, or a server that needs no login, has no usage
    // banner to follow (`has_chatgpt_account`, `requires_openai_auth`).
    if app.screens.agents.owner_of(thread.as_str()) != backend
        || !app.screens.agents.backend_auth
        || !app.screens.chat.status.account_usage
    {
        return;
    }
    let Some(current) = model_in_use(app, &thread) else {
        return;
    };
    // One switch of a thread at a time: its answer settles the next read.
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::Select(select::Ctx::Switch { thread: asked, .. }) if *asked == thread))
    {
        return;
    }
    let (recovered, account) = app
        .session
        .reserve
        .get(&backend)
        .map(|reserve| (reserve.recovered, reserve.account_id.clone()))
        .unwrap_or_default();
    // A return target saved for another account is not followed.
    if let (Some(previous), Some(account)) = (app.reserve_returns.get(&thread), &account)
        && previous.account_id != *account
    {
        app.reserve_returns.remove(&thread);
    }
    let want = if current == RESERVE_MODEL && recovered {
        match app.reserve_returns.get(&thread) {
            Some(previous) if Some(&previous.account_id) == account.as_ref() => {
                Want::Back(previous.clone())
            }
            _ => return,
        }
    } else if app
        .session
        .backend_banners
        .get(&backend)
        .is_some_and(|occurrence| occurrence.banner.wants_switch(&current))
    {
        Want::Away
    } else {
        return;
    };
    if app.screens.select.catalog.presets.is_empty() {
        ask_catalog(app, backend, fx);
        return;
    }
    let presets = &app.screens.select.catalog.presets;
    let target: Option<Preset> = match &want {
        Want::Back(previous) => presets
            .iter()
            .find(|preset| preset.show_in_picker && preset.model == previous.model)
            .cloned(),
        Want::Away => app
            .session
            .backend_banners
            .get(&backend)
            .and_then(|occurrence| occurrence.banner.fallback_target(&current, presets))
            .cloned(),
    };
    let entering_reserve = target
        .as_ref()
        .is_some_and(|target| target.model == RESERVE_MODEL);
    if entering_reserve && account.is_none() {
        with_banner(app, backend, BannerOccurrence::reopen);
        return;
    }
    let Some(target) = target else {
        return;
    };
    let current_effort = current_effort(app, &thread);
    let (wanted_effort, reason) = match &want {
        Want::Back(previous) => (previous.effort.clone(), SwitchReason::UsageRecovered),
        Want::Away => (current_effort.clone(), SwitchReason::UsageLimit),
    };
    let effort = wanted_effort
        .filter(|effort| target.efforts.iter().any(|option| option.effort == *effort))
        .unwrap_or_else(|| target.default_effort.clone());
    if let (true, Some(account_id)) = (entering_reserve, account) {
        // Saved before the server changes, so a later read can bring the thread back.
        app.reserve_returns.insert(
            thread.clone(),
            ReserveReturn {
                account_id,
                model: current,
                effort: current_effort,
            },
        );
    }
    // Automatic recovery must not apply the permission defaults a manual pick does.
    let patch = SettingsPatch {
        model: Some(target.model.clone()),
        effort: Some(effort.clone()),
        ..SettingsPatch::default()
    };
    ask(
        app,
        select::Command::Settings {
            thread: thread.to_string(),
            patch,
        },
        select::Ctx::Switch {
            backend,
            thread,
            model: target.model,
            effort,
            reason,
        },
        fx,
    );
}

/// The effort the thread runs on, as the server named it: its own settings, else the config's
/// (like `model_in_use`).
fn current_effort(app: &CodexApp, thread: &ThreadId) -> Option<ReasoningEffort> {
    app.threads
        .get(thread)
        .and_then(|model| model.screen_slots().chat.effort.as_deref())
        .or(app.screens.chat.status.effort.as_deref())
        .and_then(|effort| {
            serde_json::from_value(serde_json::Value::String(effort.to_owned())).ok()
        })
}

/// `model/list` for a switch that waits for the catalog: the pickers ask it only when opened,
/// so a thread that never opened `/model` has none.
fn ask_catalog(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::Select(select::Ctx::Fallback { .. })))
    {
        return;
    }
    ask(
        app,
        select::Command::Models { request: 0 },
        select::Ctx::Fallback { backend },
        fx,
    );
}

/// The catalog the switch waited for: kept like any other read, and the switch is decided again
/// with it (once: an empty catalog does not ask again).
pub(super) fn catalog_answer(
    app: &mut CodexApp,
    backend: BackendId,
    result: Result<select::Reply, RpcError>,
    fx: &mut AppFx,
) {
    let Ok(select::Reply::Models(presets)) = result else {
        return;
    };
    select::apply(
        &mut Rc::make_mut(&mut app.screens).select,
        select::Ev::Loaded(presets),
        fx,
    );
    if !app.screens.select.catalog.presets.is_empty() {
        fallback(app, backend, fx);
    }
}

/// The settings update of an automatic switch was answered. Older servers can decline it: the
/// existing recovery actions stay then, and nothing is changed locally.
pub(super) fn switched(
    app: &mut CodexApp,
    switch: (BackendId, ThreadId, String, ReasoningEffort, SwitchReason),
    result: Result<select::Reply, RpcError>,
    fx: &mut AppFx,
) {
    let (backend, thread, model, effort, reason) = switch;
    if !matches!(result, Ok(select::Reply::Updated)) {
        if model == RESERVE_MODEL {
            with_banner(app, backend, BannerOccurrence::reopen);
            app.reserve_returns.remove(&thread);
        }
        return;
    }
    // Reserve is left (or never entered): no return target remains.
    if model != RESERVE_MODEL {
        app.reserve_returns.remove(&thread);
    }
    with_banner(app, backend, |occurrence| occurrence.after_switch(&model));
    let note = AutomaticSwitch {
        effort: collaboration::effort_label(&model, Some(&effort)),
        model,
        reason,
    };
    session_note(
        app,
        Some(thread),
        Note::from(NoteKey::Mode(ModeNote::AutomaticSwitch(note))),
        fx,
    );
}
