//! Holds session actions until the destination folder's trust is resolved.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, lifecycle, session};
use crate::features::{FeatureCtx, Refusal};
use crate::ids::{BackendId, ConnEpoch, ThreadId};
use crate::screens::agents;
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::{self, Checked, Outcome, Target};
use crate::threads::{CdRefusal, Note, NoteKey, ThreadAsk};

/// The session action the app can dispatch once its folder has been decided.
#[derive(Debug)]
pub(super) enum Action {
    Cd {
        source: Option<ThreadId>,
        ask: ThreadAsk,
    },
    Open(ThreadId),
    New {
        cwd: Option<String>,
    },
    Fresh {
        feature: Option<FeatureCtx>,
    },
    Switch {
        destination: BackendId,
    },
}

/// The app lifecycle owns the action. Onboarding owns the currently numbered host check or write.
#[derive(Debug)]
pub(super) struct Pending {
    action: Action,
    active: Option<ThreadId>,
    displayed: BackendId,
    primary_epoch: ConnEpoch,
    checked_cwds: Vec<String>,
}

impl Pending {
    fn is_current(&self, app: &CodexApp) -> bool {
        self.active == app.session.active
            && self.displayed == app.screens.agents.displayed
            && self.primary_epoch == app.epoch_of(BackendId::default())
            && match &self.action {
                Action::Open(id) => {
                    app.screens.agents.owner_of(id.as_str()) == BackendId::default()
                }
                Action::Switch { .. } => app.cycle_label().is_some(),
                _ => true,
            }
    }
}

pub(super) fn begin(
    app: &mut CodexApp,
    action: Action,
    target: Target,
    cancel: TrustCancel,
    fx: &mut AppFx,
) {
    if app.consent.is_some() {
        return;
    }
    if consent::begin(
        &mut Rc::make_mut(&mut app.screens).onboarding,
        target,
        cancel,
        matches!(action, Action::Fresh { .. }),
        fx,
    ) {
        app.consent = Some(Pending {
            action,
            active: app.session.active.clone(),
            displayed: app.screens.agents.displayed,
            primary_epoch: app.epoch_of(BackendId::default()),
            checked_cwds: Vec::new(),
        });
    } else {
        go(app, action, fx);
    }
}

/// Invalidate navigation when its source changes, and advance the current action once.
pub(super) fn follow(app: &mut CodexApp, fx: &mut AppFx) {
    let Some(pending) = &app.consent else {
        return;
    };
    if !pending.is_current(app) || !consent::pending(&app.screens.onboarding) {
        abandon(app, fx);
        return;
    }
    if !consent::settled(&app.screens.onboarding) || switch_in_flight(app) {
        return;
    }
    let (Some(mut pending), Some(outcome)) = (
        app.consent.take(),
        consent::take(&mut Rc::make_mut(&mut app.screens).onboarding),
    ) else {
        return;
    };
    match outcome {
        Outcome::Go(checked) => {
            if let (Action::Open(thread), Some(Checked::Task(cwd))) = (&pending.action, &checked) {
                pending.checked_cwds.push(cwd.clone());
                let target = Target::TaskRecheck {
                    thread: thread.clone(),
                    checked: pending.checked_cwds.clone(),
                };
                consent::begin(
                    &mut Rc::make_mut(&mut app.screens).onboarding,
                    target,
                    TrustCancel::AgentsOverview,
                    false,
                    fx,
                );
                app.consent = Some(pending);
            } else {
                finish(app, pending.action, checked, fx);
            }
        }
        Outcome::Cancelled => refuse_feature(app, pending.action, fx),
        Outcome::Failed(message) => {
            if !matches!(pending.action, Action::Cd { .. } | Action::Switch { .. }) {
                Rc::make_mut(&mut app.screens).agents.notice =
                    Some(agents::Notice::TrustCheckFailed(message.clone()));
            }
            let source = match &pending.action {
                Action::Cd { source, .. } => source.clone(),
                _ => pending.active,
            };
            session::session_note(
                app,
                source,
                Note::from(NoteKey::CdRefused(CdRefusal::TrustCheck(message))),
                fx,
            );
            refuse_feature(app, pending.action, fx);
        }
    }
}

fn finish(app: &mut CodexApp, action: Action, checked: Option<Checked>, fx: &mut AppFx) {
    match (action, checked) {
        (Action::Switch { destination }, Some(Checked::Destination(cwd))) => {
            start_switch(app, destination, Some(cwd), fx);
        }
        (Action::Open(id), Some(Checked::ResumeIn(cwd))) => {
            super::open_verified_at(app, &id, Some(cwd), fx);
        }
        (action, _) => go(app, action, fx),
    }
}

pub(super) fn cancel_center_action(app: &mut CodexApp, fx: &mut AppFx) {
    if matches!(
        &app.consent,
        Some(Pending {
            action: Action::Open(_) | Action::New { .. },
            ..
        })
    ) {
        abandon(app, fx);
    }
}

fn abandon(app: &mut CodexApp, fx: &mut AppFx) {
    consent::abandon(&mut Rc::make_mut(&mut app.screens).onboarding);
    if let Some(pending) = app.consent.take() {
        refuse_feature(app, pending.action, fx);
    }
}

fn go(app: &mut CodexApp, action: Action, fx: &mut AppFx) {
    match action {
        Action::Cd { source, ask } => session::start_in(app, source, ask, fx),
        Action::Open(id) => {
            super::open_verified(app, &id, fx);
        }
        Action::New { cwd } => super::send_agent_command(
            app,
            agents::Out {
                command: agents::Command::Thread(ThreadAsk::Start { cwd }),
                ctx: agents::Ctx::Start,
            },
            fx,
        ),
        Action::Fresh { feature: None } => super::start_first_thread(app, fx),
        Action::Fresh { feature: Some(ctx) } => lifecycle::start_after_consent(app, ctx, fx),
        Action::Switch { destination } => start_switch(app, destination, None, fx),
    }
}

fn refuse_feature(app: &mut CodexApp, action: Action, fx: &mut AppFx) {
    if let Action::Fresh { feature: Some(ctx) } = action {
        super::features::thread_opened(app, ctx, Err(Refusal::Busy), fx);
    }
}

pub(super) fn switch(app: &mut CodexApp, fx: &mut AppFx) {
    if app.cycle_label().is_none() || switch_in_flight(app) {
        return;
    }
    let Some(destination) = app.screens.agents.next_backend() else {
        return;
    };
    if destination == BackendId::default() {
        begin(
            app,
            Action::Switch { destination },
            Target::BackendSwitch,
            TrustCancel::CurrentTask,
            fx,
        );
    } else {
        // Cloud destinations have no startup directory to check.
        start_switch(app, destination, None, fx);
    }
}

fn switch_in_flight(app: &CodexApp) -> bool {
    app.outgoing
        .any(|ctx| matches!(ctx, AppCtx::StartThread { switch: Some(_) }))
}

fn start_switch(app: &mut CodexApp, destination: BackendId, cwd: Option<String>, fx: &mut AppFx) {
    fx.ask(
        app,
        AppCtx::StartThread {
            switch: Some(destination),
        },
        |app, ticket| super::start_thread_in(app, ticket, destination, cwd),
    );
}
