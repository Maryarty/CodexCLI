//! App-scoped session commands and account status.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command, apply_thread};
use crate::ids::{BackendId, ThreadId};
use crate::screens::{ScreenCommand, ScreenThreadIntent, agents, chat, status};
use crate::threads::{CdRefusal, Item, Note, NoteKey, Stub, ThreadAsk, ThreadEv, UserMessage};

/// The Desktop app's link to a thread (`desktop_thread_deeplink`), the thread's
/// id after it.
const DESKTOP_THREAD_URL: &str = "codex://threads/";

/// `/init`: the prompt the oracle sends as a user message
/// (`assets/prompt_for_init_command.md`).
const INIT_PROMPT: &str = include_str!("../../assets/prompt_for_init_command.md");

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
        S::Cd { path } => change_directory(app, source, &path, fx),
        S::Compact | S::Rename { name: Some(_) } => {
            if let Some(thread) = source {
                let ev = match intent {
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
        // `clean_background_terminals`: the op, then the line saying so.
        S::Stop => {
            if let Some(thread) = &source {
                let ev = ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
                    chat::ThreadIntent::CleanTerminals,
                ));
                apply_thread(app, thread, ev, fx);
            }
            session_note(app, source, Note::from(NoteKey::StoppingTerminals), fx);
        }
        S::MemoryMaintenance => session_note(
            app,
            source,
            Note::from(NoteKey::Stub(Stub::MemoryMaintenance)),
            fx,
        ),
        S::Pwd => {
            let directory = working_directory(app, source.as_ref());
            session_note(
                app,
                source,
                Note::from(NoteKey::WorkingDirectory(directory)),
                fx,
            );
        }
        S::Rollout => {
            let path = source
                .as_ref()
                .and_then(|id| app.threads.get(id))
                .and_then(|model| model.screen_slots().chat.path.clone());
            session_note(app, source, Note::from(NoteKey::Rollout(path)), fx);
        }
        // `AppEvent::OpenDesktopThread`: the Desktop app takes the thread by its link.
        S::App => match source {
            Some(thread) => {
                fx.commands.push(Command::OpenUrl {
                    url: [DESKTOP_THREAD_URL, thread.as_str()].concat(),
                });
                session_note(app, Some(thread), Note::from(NoteKey::DesktopOpened), fx);
            }
            None => session_note(
                app,
                None,
                Note::from(crate::commands::Notice::SessionStarting(command)),
                fx,
            ),
        },
        // The prompt goes as a message of the thread it was typed in.
        S::Init => {
            if let Some(thread) = source {
                super::submit(app, &thread, UserMessage::text(INIT_PROMPT), fx);
            }
        }
        // `toggle_raw_output_mode_and_notify`: the mode, then the line saying so.
        S::Raw { on } => {
            let enabled = on.unwrap_or(!app.screens.chat.raw_output);
            chat::apply_intent(
                &mut Rc::make_mut(&mut app.screens).chat,
                chat::Intent::RawOutput(enabled),
                fx,
            );
            session_note(app, source, Note::from(NoteKey::RawOutput(enabled)), fx);
        }
        // What another chunk or a later step builds: until then the answer
        // that says so. Every variant is named, so a new one cannot slip by.
        S::Recap
        | S::Archive
        | S::Delete
        | S::Worktree
        | S::Rename { name: None }
        | S::Plan { .. }
        | S::Goal { .. }
        | S::Side { .. }
        | S::Copy
        | S::Export { .. }
        | S::Tui
        | S::Mcp { .. }
        | S::Apps
        | S::Plugins
        | S::Logout
        | S::Vim
        | S::Ide { .. }
        | S::Daybreak
        | S::ElevateSandbox
        | S::Hooks
        | S::DebugConfig
        | S::Ps
        | S::Review { .. }
        | S::Model { .. }
        | S::Permissions { .. }
        | S::Title { .. }
        | S::Statusline { .. }
        | S::VoiceControl(_)
        | S::ResumeNamed { .. } => session_note(
            app,
            source,
            Note::from(crate::commands::Notice::NotAvailable(command)),
            fx,
        ),
    }
}

/// `/cd` (`request_working_directory_change`, `AppEvent::ChangeWorkingDirectory`):
/// the session goes on in the directory, as a fork of the conversation when it
/// has history to keep and as a fresh thread when it has none. The new thread
/// takes the screen as `/new` and `/fork` do (`AppCtx::NewConversation`).
fn change_directory(app: &mut CodexApp, source: Option<ThreadId>, path: &str, fx: &mut AppFx) {
    let refuse = |app: &mut CodexApp, source: Option<ThreadId>, why, fx: &mut AppFx| {
        session_note(app, source, Note::from(NoteKey::CdRefused(why)), fx);
    };
    let Some((thread, model)) = source.as_ref().and_then(|id| {
        app.threads
            .get(id)
            .map(|model| (id.clone(), Rc::clone(model)))
    }) else {
        return refuse(app, source, CdRefusal::NotStarted, fx);
    };
    let part = &model.screen_slots().chat;
    // `can_change_working_directory`: an idle primary session, nothing queued.
    let idle = !model.is_running()
        && !model.can_interrupt()
        && model.queue().is_empty()
        && model.pending_steers().is_empty();
    if !idle
        || app.screens.chat.flags.side_conversation_active
        || part.parent_owned
        || part.ephemeral
    {
        return refuse(app, source, CdRefusal::NotIdle, fx);
    }
    let has_history = model
        .transcript()
        .iter()
        .any(|(_, item)| !matches!(item, Item::Note(_)));
    if has_history && part.path.is_none() {
        return refuse(app, source, CdRefusal::HistoryNotSaved, fx);
    }
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
    {
        return;
    }
    let base = part
        .cwd
        .clone()
        .or_else(|| app.screens.chat.status.cwd.clone())
        .unwrap_or_default();
    let cwd = Some(crate::commands::resolve_directory(
        path,
        &base,
        app.screens.agents.home.as_deref(),
    ));
    let ask = if has_history {
        ThreadAsk::ForkAt {
            thread: thread.to_string(),
            cwd,
        }
    } else {
        ThreadAsk::Start { cwd }
    };
    let ticket = app.mint.next();
    let backend = app.screens.agents.owner_of(thread.as_str());
    app.outgoing.insert(
        ticket,
        AppCtx::NewConversation {
            source,
            backend,
            name: None,
        },
    );
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ask)),
    });
}

/// `config.cwd`: the directory of the thread, else the one the host read for
/// the status line.
fn working_directory(app: &CodexApp, thread: Option<&ThreadId>) -> String {
    thread
        .and_then(|id| app.threads.get(id))
        .and_then(|model| model.screen_slots().chat.cwd.clone())
        .or_else(|| app.screens.chat.status.cwd.clone())
        .unwrap_or_default()
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
