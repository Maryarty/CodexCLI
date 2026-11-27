//! App-scoped session commands and account status.

use std::rc::Rc;
use std::sync::Arc;

use super::{AppCtx, AppFx, CodexApp, Command, apply_thread};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::Answer;
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::Target;
use crate::screens::select::{Confirm, Picked};
use crate::screens::{ScreenCommand, ScreenThreadIntent, agents, chat, select, status};
use crate::threads::{
    CdRefusal, Item, Note, NoteKey, RemoveNote, Stub, ThreadAsk, ThreadEv, ThreadModel, Toggle,
    UserMessage,
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

/// Reads the limits of `backend`'s account for `origin`; a refresh nobody
/// waits for folds into a read already in flight.
pub(super) fn refresh_rates(
    app: &mut CodexApp,
    backend: BackendId,
    origin: status::RateLimitOrigin,
    fx: &mut AppFx,
) {
    if origin.folds_into_a_read_in_flight()
        && app.outgoing.any(
            |ctx| matches!(ctx, AppCtx::RateLimits { backend: target, superseded: false, .. } if *target == backend),
        )
    {
        return;
    }
    fx.ask(
        app,
        AppCtx::RateLimits {
            backend,
            origin,
            superseded: false,
        },
        |_, ticket| Command::ReadRateLimits { ticket, backend },
    );
}

/// `/status` (oracle `SlashCommand::Status`): the card is an entry of the
/// transcript of the thread the command ran in, appended through that thread's
/// `apply` (ADR-003). A session that can read its account's limits asks for
/// them and the card says it is refreshing; the reply finishes the card of its
/// own request only (`ThreadEv::StatusRefreshed`). With no thread yet there is
/// no transcript to put it in.
fn status_command(app: &mut CodexApp, source: Option<ThreadId>, fx: &mut AppFx) {
    let Some(id) = source.or_else(|| app.session.active.clone()) else {
        return;
    };
    let Some(thread) = app.threads.get(&id).cloned() else {
        return;
    };
    let backend = app.screens.agents.owner_of(id.as_str());
    let prefetch =
        app.screens.chat.status.account_usage && app.screens.status.has_chatgpt_account();
    // every card has a number; a card that refreshes limits has it as the request's
    let number = Rc::make_mut(&mut app.screens).status.take_request();
    let request = prefetch.then_some(number);
    let mut card = status_card(app, &thread, backend, request.is_some());
    card.number = number;
    let item = Item::Status {
        card: Arc::new(card),
        request,
    };
    apply_thread(app, &id, ThreadEv::Appended { turn: None, item }, fx);
    if let Some(request) = request {
        refresh_rates(app, backend, status::RateLimitOrigin::Status(request), fx);
    }
}

/// The limits a card shows for `backend`: what the app has read of its
/// account, at the clock the shell last moved.
pub(super) fn card_limits(app: &CodexApp, backend: BackendId, refreshing: bool) -> status::Limits {
    status::Limits {
        snapshots: app
            .session
            .rate_snapshots
            .get(&backend)
            .map(|store| store.snapshots().cloned().collect())
            .unwrap_or_default(),
        now: app.now.secs(),
        utc_offset_secs: app.now.utc_offset_secs(),
        refreshing,
    }
}

/// The card of a `/status` that runs now, from the facts the app holds: the
/// loaded config, the account and catalog the host read, the thread's own facts
/// and the limits last read for its backend.
fn status_card(
    app: &CodexApp,
    thread: &ThreadModel,
    backend: BackendId,
    refreshing: bool,
) -> status::StatusCard {
    let facts = &app.screens.chat.status;
    let screen = &app.screens.status;
    let slots = thread.screen_slots();
    let (chat, own) = (&slots.chat, &slots.status);
    status::StatusCard::compose(status::Inputs {
        version: facts
            .version
            .as_deref()
            .unwrap_or(env!("CARGO_PKG_VERSION")),
        config: &facts.card,
        model: chat.model.as_deref().or(facts.model.as_deref()),
        effort: chat.effort.as_deref().or(facts.effort.as_deref()),
        provider: chat.provider.as_deref().or(facts.provider.as_deref()),
        requires_openai_auth: facts.account_usage,
        auto_review: facts.auto_review,
        home: facts.home_directory.as_deref(),
        catalog: &screen.catalog,
        account: screen.account.as_ref(),
        remote: screen.remote.as_ref(),
        thread: status::ThreadFacts {
            id: Some(thread.id().as_str()),
            name: chat.name.as_deref(),
            cwd: chat.cwd.as_deref(),
            forked_from: chat.forked_from.as_deref(),
            collaboration_mode: collaboration_mode_label(app, chat),
            agents: &own.agents,
            tokens: own.tokens,
            context: own.context,
        },
        limits: card_limits(app, backend, refreshing),
    })
}

/// `collaboration_mode_label`: the mode's name, when the modes are on and the mode is one the
/// terminal shows.
fn collaboration_mode_label(
    app: &CodexApp,
    chat: &crate::screens::chat::ThreadPart,
) -> Option<&'static str> {
    let mode = chat.mode();
    (app.screens.chat.flags.collaboration_modes_enabled && mode.is_tui_visible())
        .then(|| mode.display_name())
}

pub(super) fn session_intent(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    command: crate::commands::SlashCommand,
    intent: crate::commands::SessionIntent,
    fx: &mut AppFx,
) {
    use crate::commands::SessionIntent as S;
    if command != crate::commands::SlashCommand::Copy
        && let Some(thread) = &source
    {
        super::features::intent(
            app,
            crate::features::supersede_copy_status(thread.clone()),
            fx,
        );
    }
    match intent {
        // `/clear` starts the conversation where the session is; `/new` and `/fork` first ask
        // where it runs where a local repository offers a new checkout
        // (`show_session_checkout_picker`), and the pick comes back through `begin_conversation`.
        S::Clear { name } => begin_conversation(app, source, command, name, fx),
        S::New { name } | S::Fork { name } => {
            use crate::screens::select::{Intent, worktree::Conversation};
            let mode = if matches!(command, crate::commands::SlashCommand::Fork) {
                Conversation::Fork
            } else {
                Conversation::New
            };
            let open = Intent::OpenCheckout {
                mode,
                name,
                available: app.screens.chat.flags.worktrees_enabled,
                cwd: working_directory(app, source.as_ref()),
                thread: source,
            };
            super::select::intent(app, open, fx);
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
            let backend = app.screens.agents.owner_of(thread.as_str());
            fx.ask(
                app,
                AppCtx::NewConversation {
                    source: Some(thread),
                    backend,
                    name: None,
                    plan: None,
                    draft: Some(message),
                    spawned: None,
                },
                |app, ticket| super::start_thread(app, ticket, backend),
            );
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
                fx.host(crate::app::effects::Ask::open_url_or_report(
                    [DESKTOP_THREAD_URL, thread.as_str()].concat(),
                ));
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
            session_note(
                app,
                source,
                Note::from(NoteKey::Toggled(Toggle::RawOutput, enabled)),
                fx,
            );
        }
        S::Plan { prompt } => plan(app, source, command, prompt, fx),
        // A command a feature runs (`Feature::RUNS`): `/rename`, `/export <path>`, `/copy`,
        // `/goal`, `/model <name>`, `/mcp verbose`, `/recap`, ... The owner reads the arguments.
        S::Feature { args } => feature_command(app, source, command, args, fx),
        // `/permissions <profile> ..`: the profile is picked without a menu (`select_permission_profile`);
        // the select screen holds the arguments while the catalog is read.
        S::Permissions { args: Some(args) } => {
            use crate::screens::select::{Intent, permissions::Current};
            let current = Current::from_facts(&app.screens.chat.status);
            super::select::intent(app, Intent::PermissionsArgs { args, current }, fx);
        }
        // `/vim`: modal editing of the draft is UI state (the page toggles it and says so).
        S::Vim => {}
        // `/worktree`: the sheet of the worktree pickers (`show_managed_worktree_picker`), over
        // the thread's working directory. A local Git repository is what the host knows (the
        // flag that lists the command stands for it until the host says).
        S::Worktree => {
            let cwd = working_directory(app, source.as_ref());
            let available = app.screens.chat.flags.worktrees_enabled;
            let open = crate::screens::select::Intent::OpenWorktrees {
                available,
                cwd,
                thread: source,
            };
            super::select::intent(app, open, fx);
        }
        // `/review` with no text: the presets menu, the git lists asked in the thread's working
        // directory (`open_review_popup`).
        S::Review { args: None } => {
            let cwd = working_directory(app, source.as_ref());
            super::select::intent(app, crate::screens::select::Intent::OpenReview { cwd }, fx);
        }
        // `/permissions` with no profile: the menu for what the thread has now
        // (`open_permissions_popup`); the catalog is asked for when none is held.
        S::Permissions { args: None } => {
            use crate::screens::select::{Intent, permissions::Current};
            let current = Current::from_facts(&app.screens.chat.status);
            super::select::intent(app, Intent::OpenPermissions(current), fx);
        }
        // What another chunk or a later step builds: until then the answer that says so.
        S::VoiceControl(_) => session_note(
            app,
            source,
            Note::from(crate::commands::Notice::NotAvailable(command)),
            fx,
        ),
    }
}

/// A command a feature runs (`Feature::RUNS`), with the text after its name: the feature that owns
/// it reads the arguments; a command no feature owns says it is not available.
fn feature_command(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    command: crate::commands::SlashCommand,
    args: String,
    fx: &mut AppFx,
) {
    let slash = crate::features::Slash {
        command,
        args,
        source: source.clone(),
    };
    if !super::features::slash(app, slash, fx) {
        let note = Note::from(crate::commands::Notice::NotAvailable(command));
        session_note(app, source, note, fx);
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
    use super::BeforeThread;
    let Some(thread) = source else {
        // Before the first thread: the prompt waits for it, the switch is made then
        // (the oracle queues the line, `queue_user_message_with_options_and_source`).
        if let Some(prompt) = prompt {
            super::queue_before_thread(app, BeforeThread::Plan { command, prompt }, fx);
            return;
        }
        return session_note(
            app,
            None,
            Note::from(crate::commands::Notice::SessionStarting(command)),
            fx,
        );
    };
    plan_on(app, thread, command, prompt, fx)
}

/// What waited for the session goes out: `/plan <prompt>` on the thread that started.
pub(super) fn plan_after_start(
    app: &mut CodexApp,
    thread: &ThreadId,
    command: crate::commands::SlashCommand,
    prompt: String,
    fx: &mut AppFx,
) {
    plan_on(app, thread.clone(), command, Some(prompt), fx)
}

fn plan_on(
    app: &mut CodexApp,
    thread: ThreadId,
    command: crate::commands::SlashCommand,
    prompt: Option<String>,
    fx: &mut AppFx,
) {
    use crate::screens::select::collaboration::plan_mask;
    use crate::threads::ModeNote;
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

/// The new conversation of `/new`, `/clear` and `/fork`, in the working directory of the session:
/// a fork of `source` (`/fork`), or a fresh thread; the new thread takes the screen
/// (`AppCtx::NewConversation`) and is named once it has started.
pub(super) fn begin_conversation(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    command: crate::commands::SlashCommand,
    name: Option<String>,
    fx: &mut AppFx,
) {
    if app
        .outgoing
        .any(|ctx| matches!(ctx, AppCtx::NewConversation { .. }))
    {
        return;
    }
    let backend = source.as_ref().map_or(app.screens.agents.displayed, |id| {
        app.screens.agents.owner_of(id.as_str())
    });
    let fork = if matches!(command, crate::commands::SlashCommand::Fork) {
        let Some(thread) = &source else {
            return;
        };
        Some(ThreadAsk::Fork {
            thread: thread.to_string(),
        })
    } else {
        None
    };
    // `/new <name>`: the new thread is named once it has started (`name_started`).
    fx.ask(
        app,
        AppCtx::NewConversation {
            source,
            backend,
            name,
            plan: None,
            draft: None,
            spawned: None,
        },
        |app, ticket| match fork {
            Some(ask) => Command::Screen {
                ticket,
                thread: ThreadId::default(),
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
            },
            None => super::start_thread(app, ticket, backend),
        },
    );
}

/// `/new <name>`, `/fork <name>`: the thread is named once it has started, as `/rename <name>`
/// typed in it would (the command's owner does it).
pub(super) fn name_started(app: &mut CodexApp, thread: &ThreadId, name: String, fx: &mut AppFx) {
    let slash = crate::features::Slash {
        command: crate::commands::SlashCommand::Rename,
        args: name,
        source: Some(thread.clone()),
    };
    super::features::slash(app, slash, fx);
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
    let folder = cwd.clone().unwrap_or_default();
    let ask = if has_history {
        ThreadAsk::ForkAt {
            thread: thread.to_string(),
            cwd,
            temporary: None,
        }
    } else {
        ThreadAsk::Start { cwd }
    };
    // A folder whose trust is undecided is asked about first (`confirm_directory_trust`).
    super::consent::begin(
        app,
        super::consent::Action::Cd { source, ask },
        Target::Folder(Some(folder)),
        TrustCancel::CurrentTask,
        fx,
    );
}

/// The session goes on in the directory `/cd` chose: `ask` is the fork or the fresh thread, which
/// takes the screen as `/new` does.
pub(super) fn start_in(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    ask: ThreadAsk,
    fx: &mut AppFx,
) {
    let backend = source.as_ref().map_or_else(BackendId::default, |thread| {
        app.screens.agents.owner_of(thread.as_str())
    });
    fx.ask(
        app,
        AppCtx::NewConversation {
            source,
            backend,
            name: None,
            plan: None,
            draft: None,
            spawned: None,
        },
        |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        },
    );
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
        select::Intent::Confirm(confirm),
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
    fx.ask(app, AppCtx::Removal { thread, action }, |_, ticket| {
        Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(command),
        }
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
    if !matches!(answer, Answer::Thread(crate::threads::ThreadReply::Done)) {
        let note = Note::from(NoteKey::Remove(action, RemoveNote::Failed));
        session_note(app, Some(thread.clone()), note, fx);
        return false;
    }
    app.threads.forget(thread);
    true
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
            let backend = app.screens.agents.owner_of(thread.as_str());
            fx.ask(
                app,
                AppCtx::NewConversation {
                    source: Some(thread),
                    backend,
                    name: None,
                    plan: Some(markdown),
                    draft: None,
                    spawned: None,
                },
                |app, ticket| super::start_thread(app, ticket, backend),
            );
        }
    }
}
