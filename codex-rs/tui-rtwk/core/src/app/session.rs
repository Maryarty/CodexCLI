//! App-scoped session commands and account status.

use std::rc::Rc;
use std::sync::Arc;

use super::{AppCtx, AppFx, CodexApp, Command, apply_thread};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::Answer;
use crate::screens::select::{Confirm, Menu, Picked};
use crate::screens::{
    ScreenCommand, ScreenReply, ScreenThreadIntent, agents, chat, select, status,
};
use crate::threads::{
    CdRefusal, Item, Note, NoteKey, RemoveNote, Stub, ThreadAsk, ThreadEv, ThreadModel, UserMessage,
};

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

/// `/status` (oracle `SlashCommand::Status`, `add_status_output`): the card is an
/// entry of the transcript of the thread the command ran in, appended through that
/// thread's `apply` (ADR-003); the app keeps no copy of it. With no thread yet
/// there is no transcript to put it in.
fn status_command(app: &mut CodexApp, source: Option<ThreadId>, fx: &mut AppFx) {
    let Some(id) = source.or_else(|| app.session.active.clone()) else {
        return;
    };
    let Some(thread) = app.threads.get(&id).cloned() else {
        return;
    };
    let backend = app.screens.agents.owner_of(id.as_str());
    let card = status_card(app, &thread, backend);
    let item = Item::Status(Arc::new(card));
    apply_thread(app, &id, ThreadEv::Appended { turn: None, item }, fx);
    // The limits the next card shows.
    refresh_rates(app, backend, fx);
}

/// The card of a `/status` that runs now, from the facts the app holds: the
/// loaded config, the thread's own facts and the limits last read for its backend.
fn status_card(app: &CodexApp, thread: &ThreadModel, backend: BackendId) -> status::StatusCard {
    let facts = &app.screens.chat.status;
    let slots = thread.screen_slots();
    let (chat, own) = (&slots.chat, &slots.status);
    let screen = &app.screens.status;
    let approval = match facts.approval_policy.as_deref() {
        Some("never") => status::Approval::Never,
        Some("on-failure") => status::Approval::OnFailure,
        Some("untrusted") => status::Approval::UnlessTrusted,
        _ => status::Approval::OnRequest,
    };
    let sandbox = match facts.profile {
        status::Profile::ReadOnly => status::SandboxKind::ReadOnly,
        status::Profile::Workspace => status::SandboxKind::Workspace,
        status::Profile::Full => status::SandboxKind::DangerFullAccess,
    };
    let window = |rate: &status::RateWindow| status::Window {
        used_percent: rate.used_percent,
        window_minutes: rate.window_minutes,
        resets_at: rate.resets_at,
    };
    let snapshots = app
        .session
        .rate_limits
        .get(&backend)
        .filter(|rates| rates.primary.is_some() || rates.secondary.is_some())
        .map(|rates| status::Snapshot {
            name: "codex".to_string(),
            captured_at: screen.now,
            primary: rates.primary.as_ref().map(window),
            secondary: rates.secondary.as_ref().map(window),
            credits: None,
            individual_limit: None,
        })
        .into_iter()
        .collect();
    status::StatusCard {
        version: facts
            .version
            .clone()
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        remote: None,
        model: chat
            .model
            .clone()
            .or_else(|| facts.model.clone())
            .unwrap_or_else(|| "loading".to_string()),
        reasoning: chat
            .effort
            .clone()
            .or_else(|| facts.effort.clone())
            .map(|effort| status::Reasoning {
                effort,
                summary: Some("auto".to_string()),
            }),
        provider: chat.provider.clone().or_else(|| facts.provider.clone()),
        directory: chat
            .cwd
            .clone()
            .or_else(|| facts.cwd.clone())
            .unwrap_or_default(),
        home: facts.home_directory.clone(),
        permissions: status::Permissions {
            profile: facts.permission_profile.clone(),
            sandbox,
            network: false,
            disabled: facts.profile == status::Profile::Full,
            approval,
            auto_review: facts.auto_review,
            extra_roots: Vec::new(),
        },
        agents: Vec::new(),
        account: None,
        thread_name: chat.name.clone(),
        session_id: Some(thread.id().to_string()),
        forked_from: None,
        collaboration_mode: None,
        usage_link: facts.account_usage,
        tokens: own.tokens,
        context: own.context,
        limits: status::Limits {
            snapshots,
            now: screen.now,
            utc_offset_secs: screen.utc_offset_secs,
            refreshing: false,
        },
        hour24: false,
    }
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
            // `/new <name>`: the new thread is named once it has started (`features/rename`).
            let then = name.map(|name| crate::features::rename::ThreadIntent::Set(name).into());
            app.outgoing.insert(
                ticket,
                AppCtx::NewConversation {
                    source,
                    backend,
                    then,
                    plan: None,
                    draft: None,
                },
            );
            fx.commands.push(request);
        }
        // The provider-error notice's continue key: a fresh chat whose composer
        // holds the text (the view's words); only a refused chat that was saved.
        S::ContinueChat { message } => {
            let Some(thread) = source else {
                return;
            };
            let continues = app.threads.get(&thread).is_some_and(|model| {
                model.provider_error().is_some() && model.screen_slots().chat.path.is_some()
            });
            if !continues
                || app
                    .outgoing
                    .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
            {
                return;
            }
            let ticket = app.mint.next();
            let backend = app.screens.agents.owner_of(thread.as_str());
            app.outgoing.insert(
                ticket,
                AppCtx::NewConversation {
                    source: Some(thread),
                    backend,
                    then: None,
                    plan: None,
                    draft: Some(message),
                },
            );
            fx.commands.push(super::start_thread(app, ticket, backend));
        }
        S::Cd { path } => change_directory(app, source, &path, fx),
        // `slash_dispatch.rs`: `/review <text>` reviews the text as instructions.
        S::Review {
            args: Some(instructions),
        } => {
            let target = crate::threads::ReviewTarget::Custom {
                instructions: instructions.trim().to_owned(),
            };
            start_review(app, source, target, fx);
        }
        S::StartReview { target } => start_review(app, source, target, fx),
        S::Archive => ask_removal(app, source, agents::Action::Archive, fx),
        S::Delete => ask_removal(app, source, agents::Action::Delete, fx),
        // `/rename`: the name prompt, or the rename itself (`features/rename`).
        S::Rename { name } => {
            let intent = crate::features::rename::Intent::Command { source, name };
            super::features::intent(app, intent.into(), fx);
        }
        S::Status => status_command(app, source, fx),
        S::Compact => {
            if let Some(thread) = source {
                let ev =
                    ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::Compact));
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
        S::Plan { prompt } => plan(app, source, command, prompt, fx),
        // `/export` (the destination picker) and `/export <path>` (written there): `features/export`.
        S::Export { path } => {
            let intent = crate::features::export::Intent::Command { path };
            super::features::intent(app, intent.into(), fx);
        }
        // `/goal`: the goal shown, set, paused, resumed, cleared or edited (`features/goal`).
        S::Goal { args } => {
            let intent = crate::features::goal::Intent::Command { source, args };
            super::features::intent(app, intent.into(), fx);
        }
        // `/permissions <profile> ..`: the profile is picked without a menu (`select_permission_profile`);
        // the select screen holds the arguments while the catalog is read.
        S::Permissions { args: Some(args) } => {
            use crate::screens::select::{Intent, permissions::Current};
            let current = Current::from_facts(&app.screens.chat.status);
            super::select::intent(app, Intent::PermissionsArgs { args, current }, fx);
        }
        // What another chunk or a later step builds: until then the answer
        // that says so. Every variant is named, so a new one cannot slip by.
        S::Recap
        | S::Worktree
        | S::Side { .. }
        | S::Copy
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
        | S::Review { args: None }
        | S::Model { .. }
        | S::Permissions { args: None }
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

/// `/plan` (`apply_plan_slash_command`): the thread goes on in the Plan preset of the server's
/// catalog. With modes off, or no Plan preset, the note says so; with a prompt, the prompt is
/// the first message in Plan mode, or, when the switch could not be made, goes back to the
/// composer as typed (`/plan <text>`).
fn plan(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    command: crate::commands::SlashCommand,
    prompt: Option<String>,
    fx: &mut AppFx,
) {
    use crate::screens::select::collaboration::plan_mask;
    use crate::threads::ModeNote;
    let Some(thread) = source else {
        return session_note(
            app,
            None,
            Note::from(crate::commands::Notice::SessionStarting(command)),
            fx,
        );
    };
    let refusal = if !app.screens.chat.flags.collaboration_modes_enabled {
        Some(ModeNote::Disabled)
    } else if plan_mask(&app.modes).is_none() {
        Some(ModeNote::PlanUnavailable)
    } else {
        None
    };
    if let Some(note) = refusal {
        session_note(
            app,
            Some(thread.clone()),
            Note::from(NoteKey::Mode(note)),
            fx,
        );
        if let Some(prompt) = prompt {
            let draft = ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
                chat::ThreadIntent::Draft(format!("/{} {prompt}", command.command())),
            ));
            apply_thread(app, &thread, draft, fx);
        }
        return;
    }
    if let Some(mask) = plan_mask(&app.modes) {
        let set =
            ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::SetMask(mask)));
        apply_thread(app, &thread, set, fx);
    }
    if let Some(prompt) = prompt {
        super::submit(app, &thread, UserMessage::text(prompt), fx);
    }
}

/// `review/start` inline on the thread: the review runs as a turn of it.
pub(super) fn start_review(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    target: crate::threads::ReviewTarget,
    fx: &mut AppFx,
) {
    if let Some(thread) = source {
        let ev =
            ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::Review(target)));
        apply_thread(app, &thread, ev, fx);
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
            then: None,
            plan: None,
            draft: None,
        },
    );
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ask)),
    });
}

/// `/archive`, `/delete` (`slash_dispatch.rs`): the confirmation opens in the
/// select sheet, whatever the session is (the checks are the answer Yes's).
fn ask_removal(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    action: agents::Action,
    fx: &mut AppFx,
) {
    let confirm = match action {
        agents::Action::Archive => Confirm::Archive {
            running: source
                .and_then(|id| app.threads.get(&id).map(|model| model.is_running()))
                .unwrap_or(false),
        },
        agents::Action::Delete => Confirm::Delete,
    };
    select::apply_intent(
        &mut Rc::make_mut(&mut app.screens).select,
        select::Intent::Open(Menu::Confirm(confirm)),
        fx,
    );
}

/// The answer Yes of a confirmation the select sheet closed on: the pick is
/// taken off the screen's record and acted on (No picks nothing).
pub(super) fn confirmed(app: &mut CodexApp, fx: &mut AppFx) {
    let yes = |picked: &Picked| matches!(picked, Picked::Confirmed(_));
    if !app.screens.select.picked.iter().any(yes) {
        return;
    }
    let mut asked = Vec::new();
    Rc::make_mut(&mut app.screens)
        .select
        .picked
        .retain(|picked| match picked {
            Picked::Confirmed(confirm) => {
                asked.push(*confirm);
                false
            }
            _ => true,
        });
    for confirm in asked {
        let action = match confirm {
            Confirm::Archive { .. } => agents::Action::Archive,
            Confirm::Delete => agents::Action::Delete,
        };
        remove_current_thread(app, action, fx);
    }
}

/// `archive_current_thread`, `delete_current_thread` (app/event_dispatch.rs):
/// the server is asked to remove the thread the chat shows. Saved side chats
/// and the voice session of the thread are not carried (no `/side`, no voice
/// session in the port).
fn remove_current_thread(app: &mut CodexApp, action: agents::Action, fx: &mut AppFx) {
    let source = app.session.active.clone();
    let refuse = |app: &mut CodexApp, why, fx: &mut AppFx| {
        let note = Note::from(NoteKey::Remove(action, why));
        session_note(app, source.clone(), note, fx);
    };
    let Some(thread) = source.clone() else {
        return refuse(app, RemoveNote::NotStarted, fx);
    };
    if app.screens.chat.flags.side_conversation_active {
        return refuse(app, RemoveNote::SideConversation, fx);
    }
    let asked =
        |ctx: &AppCtx| matches!(ctx, AppCtx::Removal { thread: held, .. } if *held == thread);
    if app.outgoing.any(asked) {
        return;
    }
    let command = match action {
        agents::Action::Archive => agents::Command::Archive {
            thread: thread.to_string(),
        },
        agents::Action::Delete => agents::Command::Delete {
            thread: thread.to_string(),
        },
    };
    let ticket = app.mint.next();
    app.outgoing
        .insert(ticket, AppCtx::Removal { thread, action });
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(command),
    });
}

/// The server answered the removal of `thread`: true when it is gone (the
/// caller empties the chat when it was the shown one, and the command center
/// takes its place: `return_to_agents_after_removal`); refused, the thread
/// stays and the chat says so.
pub(super) fn removed(
    app: &mut CodexApp,
    thread: &ThreadId,
    action: agents::Action,
    answer: Answer,
    fx: &mut AppFx,
) -> bool {
    if !matches!(
        answer,
        Answer::Screen(ScreenReply::Agents(agents::Reply::Done))
    ) {
        let note = Note::from(NoteKey::Remove(action, RemoveNote::Failed));
        session_note(app, Some(thread.clone()), note, fx);
        return false;
    }
    app.threads.forget(thread);
    true
}

/// A prompt the user submitted: the app carries out what its kind asks for.
pub(super) fn prompt_submitted(app: &mut CodexApp, fx: &mut AppFx) {
    use crate::screens::dialogs::PromptKind;
    if app.screens.dialogs.submitted.is_none() {
        return;
    }
    let Some(submitted) = Rc::make_mut(&mut app.screens).dialogs.submitted.take() else {
        return;
    };
    match submitted.kind {
        PromptKind::Feature(prompt) => {
            super::features::prompt_submitted(app, prompt, submitted.text, fx);
        }
    }
}

/// `config.cwd`: the directory of the thread, else the one the host read for
/// the status line.
fn working_directory(app: &CodexApp, thread: Option<&ThreadId>) -> String {
    super::features::view(app).working_directory(thread)
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

/// The answer of the "Implement this plan?" question, for the thread the chat shows
/// (`plan_implementation::selection_view_params` actions).
pub(super) fn plan_answer(
    app: &mut CodexApp,
    answer: crate::screens::chat::plan_mode::Answer,
    fx: &mut AppFx,
) {
    use crate::screens::chat::plan_mode::Answer;
    let Some(thread) = app.session.active.clone() else {
        return;
    };
    match answer {
        Answer::Implement => {
            let ev =
                ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::ImplementPlan));
            apply_thread(app, &thread, ev, fx);
        }
        // `implement_plan_in_fresh_session`: a new thread, which takes the plan.
        Answer::ImplementFresh => {
            let Some(markdown) = app
                .threads
                .get(&thread)
                .and_then(|model| model.screen_slots().chat.plan.prompt()?.plan.clone())
            else {
                return;
            };
            if app
                .outgoing
                .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
            {
                return;
            }
            let ticket = app.mint.next();
            let backend = app.screens.agents.owner_of(thread.as_str());
            app.outgoing.insert(
                ticket,
                AppCtx::NewConversation {
                    source: Some(thread),
                    backend,
                    then: None,
                    plan: Some(markdown),
                    draft: None,
                },
            );
            fx.commands.push(super::start_thread(app, ticket, backend));
        }
    }
}
