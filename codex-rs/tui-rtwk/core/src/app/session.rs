//! App-scoped session commands and account status.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command, apply_thread};
use crate::ids::{BackendId, ThreadId};
use crate::screens::{ScreenCommand, ScreenThreadIntent, agents, chat, status};
use crate::threads::{Item, Note, ThreadAsk, ThreadEv};

pub(super) fn active_backend(app: &CodexApp) -> BackendId {
    let agents = &app.screens.agents;
    app.session
        .active
        .as_ref()
        .map_or(agents.displayed, |thread| agents.owner_of(thread.as_str()))
}

pub(super) fn refresh_rates(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::RateLimits { backend: target } if *target == backend))
    {
        return;
    }
    let ticket = app.mint.next();
    app.outgoing.insert(ticket, AppCtx::RateLimits { backend });
    fx.commands
        .push(Command::ReadRateLimits { ticket, backend });
}

pub(super) fn open_status(app: &mut CodexApp, fx: &mut AppFx) {
    let backend = active_backend(app);
    let facts = &app.screens.chat.status;
    let thread = app.active_thread();
    let part = &thread.screen_slots().status;
    let card = status::Card {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: facts.model.clone().unwrap_or_default(),
        reasoning_effort: facts.effort.clone(),
        reasoning_summary: None,
        directory: facts.cwd.clone().unwrap_or_default(),
        profile: facts.profile,
        approval: match facts.approval_policy.as_deref() {
            Some("never") => status::Approval::Never,
            Some("on-failure") => status::Approval::OnFailure,
            Some("untrusted") => status::Approval::UnlessTrusted,
            _ => status::Approval::OnRequest,
        },
        auto_review: facts.auto_review,
        agents: Vec::new(),
        tokens: part.tokens,
        context: part.context,
        limits: app
            .session
            .rate_limits
            .get(&backend)
            .map_or_else(Vec::new, status::RateLimits::card_limits),
    };
    status::apply(
        &mut Rc::make_mut(&mut app.screens).status,
        status::Ev::Shown(card),
        fx,
    );
    refresh_rates(app, backend, fx);
}

pub(super) fn session_intent(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    command: crate::commands::SlashCommand,
    intent: crate::commands::SessionIntent,
    fx: &mut AppFx,
) {
    use crate::commands::SessionIntent as S;
    match intent {
        S::New { name } | S::Clear { name } | S::Fork { name } => {
            if app
                .outgoing
                .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
            {
                return;
            }
            let ticket = app.mint.next();
            let backend = source.as_ref().map_or(app.screens.agents.displayed, |id| {
                app.screens.agents.owner_of(id.as_str())
            });
            let request = if matches!(command, crate::commands::SlashCommand::Fork) {
                let Some(thread) = &source else {
                    return;
                };
                Command::Screen {
                    ticket,
                    thread: ThreadId::default(),
                    command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Fork {
                        thread: thread.to_string(),
                    })),
                }
            } else {
                super::start_thread(app, ticket, backend)
            };
            app.outgoing.insert(
                ticket,
                AppCtx::NewConversation {
                    source,
                    backend,
                    name,
                },
            );
            fx.commands.push(request);
        }
        S::Stop | S::Compact | S::Rename { name: Some(_) } => {
            if let Some(thread) = source {
                let ev = match intent {
                    S::Stop => ThreadEv::Interrupt,
                    S::Rename { name: Some(name) } => ThreadEv::ScreenIntent(
                        ScreenThreadIntent::Chat(chat::ThreadIntent::Rename(name)),
                    ),
                    _ => ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
                        chat::ThreadIntent::Compact,
                    )),
                };
                apply_thread(app, &thread, ev, fx);
            }
        }
        // Not built yet: the oracle-style "not available" key.
        _ => session_note(
            app,
            source,
            Note::from(crate::commands::Notice::NotAvailable(command)),
            fx,
        ),
    }
}

/// What a command line answered with (oracle `add_info_message` /
/// `add_error_message`): an entry of the thread it was submitted in, so it
/// stays in the transcript; with no thread yet, a note of the chat screen.
pub(super) fn session_note(
    app: &mut CodexApp,
    thread: Option<ThreadId>,
    note: Note,
    fx: &mut AppFx,
) {
    match thread.filter(|id| app.threads.get(id).is_some()) {
        Some(thread) => apply_thread(
            app,
            &thread,
            ThreadEv::Appended {
                turn: None,
                item: Item::Note(note),
            },
            fx,
        ),
        None => chat::apply_intent(
            &mut Rc::make_mut(&mut app.screens).chat,
            chat::Intent::Note { thread: None, note },
            fx,
        ),
    }
}
