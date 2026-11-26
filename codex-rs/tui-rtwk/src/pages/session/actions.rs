//! What a submitted line does: slash commands, the warnings opener, typed failures.

use rtwk::layout::{EdgeInsets, Padding};
use rtwk::provider::read;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::commands::{
    DispatchCx, Notice, Open, Outcome, SessionIntent, SlashCommand, Target, dispatch_with_tiers,
    parent_owned_takes,
};
use codex_tui_core::ids::Cwd;
use codex_tui_core::screens::composer::{self, ShellLine};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, popovers};
use codex_tui_core::threads::{
    DueCommand, Failure, Note, NoteKey, Ran, ThreadModel, Toggle, UserMessage,
};

use crate::app::view::Hs;
use crate::features;
use crate::pages::session::composer::clear_sent;
use crate::pages::session::composer::queue::{self, Delivery};
use crate::pages::session::open::{note_opened, open};

/// The chat screen of one thread's controllers.
use super::session_page::*;

/// The words of a typed failure.
pub(super) fn failure_text(strings: &crate::l10n::strings::Shared, failure: &Failure) -> String {
    match failure {
        Failure::Message(message) => strings.error(message).to_string(),
        Failure::Lagged(skipped) => strings.lagged(*skipped).to_string(),
        Failure::Stopped => strings.chat.server_stopped.to_string(),
        Failure::ReplyMismatch => strings.chat.reply_mismatch.to_string(),
    }
}

/// The composer's prompt, as wide as its text: a trailing blank is a cell.
pub(super) fn prompt(text: &'static str, style: Style) -> AnyWidget {
    let label = text.trim_end_matches(' ');
    Padding::new(EdgeInsets::only(0, 0, 1, 0), Text::line(label).style(style)).into()
}

/// A line the user sent from the composer: a command opens its screen, asks
/// the session or says why not (the decision is core's `dispatch`); anything
/// else is a message for the model.
/// F2 and `/warnings`: the viewer over the retained warnings (with none it
/// shows its empty page, `No warnings`).
pub(super) fn open_warnings(cx: &mut Cx<'_, ChatState>) {
    super::open::open_warnings(cx);
}

/// A settings shortcut (`Shift+Tab`, `Alt+,`, `Alt+.`) on a thread its parent controls says so
/// and does nothing (`add_error_message(PARENT_OWNED_INPUT_MESSAGE)`); true when it refused.
pub(super) fn refuses_parent_owned(cx: &mut Cx<'_, ChatState>, hs: &Hs) -> bool {
    let thread = read::<ThreadModel, _>(cx);
    if !thread.screen_slots().chat.parent_owned {
        return false;
    }
    let intent = chat::Intent::Note {
        thread: read::<SessionSlice, _>(cx).active.clone(),
        note: Note::from(NoteKey::ParentOwned),
    };
    cx.emit(&hs.screen, ScreenIntent::from(intent));
    true
}

/// `handle_reasoning_shortcut`: one step of the model's reasoning effort, or the line that says
/// why there is none.
pub(super) fn reasoning_shortcut(
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
    direction: codex_tui_core::screens::select::reasoning::Direction,
) {
    use codex_tui_core::screens::select::{Intent, models::Current, models::Effort};
    let thread = read::<ThreadModel, _>(cx);
    if thread.read_only() || refuses_parent_owned(cx, hs) {
        return;
    }
    let states = read::<ScreenStates, _>(cx);
    let slots = thread.screen_slots();
    let model = slots
        .chat
        .model
        .clone()
        .or_else(|| states.chat.status.model.clone());
    let effort = slots
        .chat
        .effort
        .as_deref()
        .or(states.chat.status.effort.as_deref());
    let current = Current {
        effort: effort.and_then(|name| name.parse::<Effort>().ok()),
        ..Current::new(model.unwrap_or_default())
    };
    let plan = states.chat.flags.collaboration_modes_enabled
        && slots.chat.mode() == codex_protocol::config_types::ModeKind::Plan;
    let started = matches!(read::<SessionSlice, _>(cx).status, Session::Ready);
    cx.emit(
        &hs.screen,
        ScreenIntent::Select(Intent::Reasoning {
            direction,
            current,
            plan,
            started,
        }),
    );
}

pub(super) fn run_line(line: &str, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs, send: Delivery) {
    run(line, None, w, cx, hs, send, &mut Some(Ran::Submitted));
}

/// A supplied line can come from the queue or an immediately recalled preset.
#[derive(Clone, Copy)]
pub(super) enum DeferredLine<'a> {
    Due(&'a DueCommand),
    Preset(&'a UserMessage),
}

impl<'a> DeferredLine<'a> {
    fn message(self) -> &'a UserMessage {
        match self {
            Self::Due(due) => due.line(),
            Self::Preset(message) => message,
        }
    }
}

/// `line` as the composer sends it. A deferred line supplies its own images and mentions.
/// The result tells how to continue if this dispatch came from the queue.
pub(super) fn run(
    line: &str,
    deferred: Option<DeferredLine<'_>>,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
    send: Delivery,
    ran: &mut Option<Ran>,
) {
    let drained = send == Delivery::Drain;
    // An empty line is nothing to run: the composer was already taken by the
    // pick or the submit that came with the same Enter. Remote images alone
    // are a message (`user_message.remote_image_urls.is_empty()`).
    if !drained && line.trim().is_empty() && w.ui.draft.remote_count() == 0 {
        return;
    }
    // the composer is emptied for a line it sent, not for one the queue ran
    let clear_sent = |ui: &_| {
        if !drained {
            clear_sent(ui);
        }
    };
    let states = read::<ScreenStates, _>(cx);
    let thread = read::<ThreadModel, _>(cx);
    let session = read::<SessionSlice, _>(cx);
    // the commands the current model's service tiers add (`/fast`)
    let thread_model = thread.screen_slots().chat.model.clone();
    let tiers = codex_tui_core::screens::chat::facts::tier_commands(
        &states.chat.status,
        thread_model.as_deref(),
        &states.select.catalog,
        &states.chat.service_tiers,
    );
    let mut flags = states.chat.flags;
    flags.service_tier_commands_enabled = !tiers.is_empty();
    let facts = DispatchCx {
        flags,
        task_running: thread.busy(), // a turn running or a start in flight
        side_conversation: states.chat.flags.side_conversation_active,
        review_mode: thread.screen_slots().chat.review_mode(),
        queued_follow_up: !thread.queue().is_empty() || !thread.rejected_steers().is_empty(),
        thread_started: matches!(session.status, Session::Ready),
        has_backend_auth: states.chat.signed_in,
    };
    let screen = |cx: &mut Cx<'_, ChatState>, intent: chat::Intent| {
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    // What the line answered with is a note of the thread it was sent in.
    let say = |cx: &mut Cx<'_, ChatState>, note: Note| {
        let intent = chat::Intent::Note {
            thread: session.active.clone(),
            note,
        };
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let record = |cx: &mut Cx<'_, ChatState>| {
        if !drained {
            let intent = popovers::Intent::Record(line.to_string());
            cx.emit(&hs.screen, ScreenIntent::from(intent));
        }
    };
    // A thread its parent controls takes the commands that do not reach the
    // agent and nothing else; the composer keeps the line
    // (`InputResult::ParentOwnedInputBlocked`, `handle_parent_owned_submission`).
    if thread.screen_slots().chat.parent_owned && !parent_owned_takes(line, flags) {
        say(cx, Note::from(NoteKey::ParentOwned));
        return;
    }
    // What the model takes whole: nothing over the backend's limit (Enter
    // refuses it and the draft stays, `prepare_submission_text`).
    if let Some(provided) = composer::too_large(line) {
        say(cx, Note::from(Notice::InputTooLarge { provided }));
        return;
    }
    // (Tab behind a running turn: a command or a shell line is parsed when its turn comes)
    if send.queues(line, thread.busy(), w.ui.draft.shell()) {
        queue::keep(line, w, cx, hs, thread.id(), &states);
        return;
    }
    // A draft in shell mode runs as the user's own shell command (`!cmd`,
    // `submit_shell_command`), through the thread, a turn or none running;
    // before the first thread the app keeps it with the messages.
    if let Some(shell) = composer::shell_line(line, !drained && w.ui.draft.shell()) {
        // a thread that exists but is not taking input (no connection) keeps
        // the draft; with no thread yet the app queues it
        if !facts.thread_started && session.active.is_some() {
            return;
        }
        match shell {
            ShellLine::Help => {
                clear_sent(&w.ui);
                say(cx, Note::from(Notice::ShellHelp));
            }
            ShellLine::Run(command) => {
                record(cx);
                clear_sent(&w.ui);
                let intent = chat::Intent::Shell {
                    thread: session.active.clone(),
                    command,
                };
                screen(cx, intent);
            }
        }
        return;
    }
    let outcome = dispatch_with_tiers(line, &facts, &tiers);
    match outcome {
        Outcome::NotACommand => {
            // The decision is `apply`'s (R10): it starts a turn or queues
            // the message; the composer clears when it has.
            *ran = Some(Ran::Submitted);
            if let Some(deferred) = deferred {
                cx.emit(&hs.submit, deferred.message().clone());
            } else if !line.trim().is_empty() || w.ui.draft.remote_count() > 0 {
                record(cx);
                let message = queue::message(line, w, cx, &states);
                // (the queue key and the drain left before dispatch)
                queue::send(cx, hs, message, session.active.clone());
            }
        }
        // A feature's own sheet (`/plugins`, a bare `/export`): its UI half opens it with the
        // typed arguments (`features::OpenArgs`), the feature is told what it needs to be.
        Outcome::Open(Open {
            target: Target::Feature(feature),
            command,
            ..
        }) => {
            *ran = Some(queue::opened_by(command));
            record(cx);
            clear_sent(&w.ui);
            let args = features::OpenArgs::of(
                feature,
                Cwd::new(review_cwd(&states, &thread)),
                session.active.clone(),
            );
            features::open(cx, args);
            note_opened(Target::Feature(feature));
        }
        Outcome::Open(opened) => {
            *ran = Some(queue::opened_by(opened.command));
            record(cx);
            clear_sent(&w.ui);
            if opened.target == Target::Usage {
                let intent =
                    ScreenIntent::Usage(codex_tui_core::screens::usage::Intent::RefreshRateLimits);
                cx.emit(&hs.screen, intent);
            }
            // The settings dialog is opened by the kind the command names (`open_settings`):
            // the feature builds its catalogue from its facts.
            if opened.target == Target::Settings {
                features::open_settings(cx, opened.command, &opened.args);
            }
            // The confirmation of `/archive` and `/delete` is the model's
            // question (is a turn running?), then the select sheet shows it.
            if let Some(intent) = removal_intent(opened.command) {
                screen(
                    cx,
                    chat::Intent::Session {
                        thread: session.active.clone(),
                        command: opened.command,
                        intent,
                    },
                );
                // The select sheet shows the question once the model has asked for it (the presenter).
                note_opened(Target::Select);
                return;
            }
            // `/auto-review` lists the thread's denials in a sheet of its own, or says there is
            // none (`open_auto_review_denials_popup`).
            if opened.command == SlashCommand::AutoReview {
                let denials = &thread.screen_slots().approvals.recent_denials;
                if denials.is_empty() {
                    let what = codex_tui_core::app::AutoReview::NoDenials;
                    let thread = thread.id().clone();
                    cx.emit(
                        &hs.intent,
                        codex_tui_core::app::Intent::AutoReview { thread, what },
                    );
                } else {
                    drop(crate::sheets::approvals::open_auto_review(
                        cx,
                        thread.id().clone(),
                        denials,
                    ));
                }
                note_opened(Target::Select);
                return;
            }
            // `/model`, `/permissions` and `/apps` fill the selection dialog
            // from the thread's facts, and the app asks the server for the rest.
            let mut shown_by_presenter = false;
            if let Some(intent) = select_intent(
                opened.command,
                &states.chat.status,
                states.chat.flags.connectors_enabled,
                reserve_only(&session, &states, &thread),
                review_cwd(&states, &thread),
            ) {
                // `/review`, `/apps`, `/permissions` and `/model` are sheets of their own: the model
                // asks for them and the presenter shows them (a `/model` the provider blocks shows
                // nothing: a note).
                shown_by_presenter = matches!(
                    intent,
                    codex_tui_core::screens::select::Intent::OpenReview { .. }
                        | codex_tui_core::screens::select::Intent::OpenApps { .. }
                        | codex_tui_core::screens::select::Intent::OpenPermissions(_)
                        | codex_tui_core::screens::select::Intent::OpenModels(_)
                );
                let intent = match (deferred, intent) {
                    (
                        Some(DeferredLine::Due(due)),
                        codex_tui_core::screens::select::Intent::OpenModels(current),
                    ) => {
                        // The select reducer acknowledges this exact request before it opens.
                        // The usual dispatch result would release it ahead of that decision.
                        *ran = None;
                        codex_tui_core::screens::select::Intent::OpenQueuedModels(Box::new(
                            codex_tui_core::screens::select::QueuedModels {
                                operation: due.operation().clone(),
                                current,
                            },
                        ))
                    }
                    (_, intent) => intent,
                };
                cx.emit(&hs.screen, ScreenIntent::Select(intent));
            }
            if opened.command == SlashCommand::Warnings {
                open_warnings(cx);
            } else if opened.command == SlashCommand::Presets {
                // the menu opens where presets can be used (`open_quick_dial_menu`): the page says
                // so after this line is run
                cx.local_sender()(ChatMsg::Presets(crate::sheets::select::hud::Step::Open));
            } else if shown_by_presenter {
                note_opened(opened.target);
            } else {
                open(cx, opened.target);
            }
        }
        // `/vim`: modal editing is the draft's own state, toggled here and said
        // (`toggle_vim_mode_and_notify`); the composer clears with the line.
        Outcome::Session {
            intent: SessionIntent::Vim,
            ..
        } => {
            record(cx);
            clear_sent(&w.ui);
            let on = !w.ui.draft.vim_enabled();
            w.ui.draft.set_vim_enabled(on);
            say(cx, Note::from(NoteKey::Toggled(Toggle::Vim, on)));
        }
        Outcome::Session { command, intent } => {
            *ran = Some(queue::session_ran(command));
            record(cx);
            clear_sent(&w.ui);
            screen(
                cx,
                chat::Intent::Session {
                    thread: session.active.clone(),
                    command,
                    intent,
                },
            );
        }
        Outcome::Tier(tier) => {
            record(cx);
            clear_sent(&w.ui);
            // the tier the thread runs on now, else the config's
            let current = match &thread.screen_slots().chat.service_tier {
                Some(tier) => tier.clone(),
                None => states.chat.status.service_tier.clone(),
            };
            let intent = codex_tui_core::screens::select::Intent::ServiceTier {
                id: tier.id,
                current,
            };
            cx.emit(&hs.screen, ScreenIntent::Select(intent));
        }
        Outcome::Insert(text) => {
            if !drained {
                w.ui.composer.reset(text);
            }
        }
        Outcome::Quit => cx.emit(&hs.quit, ()),
        // What the composer does with the line is the oracle's: it keeps an
        // unknown command and one refused while a task runs, recalls the rest.
        Outcome::Notice(notice) => {
            if notice.is_recalled() {
                record(cx);
            }
            match (notice.keeps_draft(), notice.is_recalled()) {
                (false, _) => clear_sent(&w.ui),
                (true, true) => w.ui.draft.recorded(&w.ui.composer),
                (true, false) => {}
            }
            say(cx, Note::from(notice));
        }
    }
}

/// What the model is asked before the select sheet shows the confirmation of
/// `/archive` or `/delete`.
fn removal_intent(command: SlashCommand) -> Option<SessionIntent> {
    match command {
        SlashCommand::Archive => Some(SessionIntent::Archive),
        SlashCommand::Delete => Some(SessionIntent::Delete),
        _ => None,
    }
}

/// The directory `/review` lists branches and commits of (`self.config.cwd`): the thread's own,
/// else the one the workspace is probed in; none known, the process's.
fn review_cwd(states: &ScreenStates, thread: &ThreadModel) -> String {
    thread
        .screen_slots()
        .chat
        .cwd
        .clone()
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| states.chat.facts.cwd().map(str::to_owned))
        .unwrap_or_else(|| ".".to_owned())
}

/// `restrict_model_picker_to_luna_reserve`: the thread runs on the Reserve and the last account
/// read did not say ordinary usage is back.
pub(super) fn reserve_only(
    session: &SessionSlice,
    states: &ScreenStates,
    thread: &ThreadModel,
) -> Option<codex_tui_core::screens::select::models::ReserveOnly> {
    use codex_tui_core::screens::select::models::{RESERVE_MODEL, ReserveOnly};
    let slots = thread.screen_slots();
    let model = slots
        .chat
        .model
        .as_deref()
        .or(states.chat.status.model.as_deref())?;
    if model != RESERVE_MODEL {
        return None;
    }
    let backend = states.agents.owner_of(thread.id().as_str());
    let facts = session.reserve.get(&backend);
    if facts.is_some_and(|facts| facts.recovered) {
        return None;
    }
    Some(ReserveOnly {
        normal_model: facts.and_then(|facts| facts.normal_model.clone()),
    })
}

/// The menu a slash command opens in the selection dialog, built from the
/// thread's facts and whether the apps feature is on (the others open their slot empty until they land).
fn select_intent(
    command: SlashCommand,
    facts: &chat::StatusFacts,
    apps: bool,
    reserve: Option<codex_tui_core::screens::select::models::ReserveOnly>,
    cwd: String,
) -> Option<codex_tui_core::screens::select::Intent> {
    use codex_tui_core::screens::select::{Intent, models, permissions};
    match command {
        SlashCommand::Model => Some(Intent::OpenModels(models::Current {
            reserve,
            ..models::Current::from_facts(facts)
        })),
        SlashCommand::Permissions => Some(Intent::OpenPermissions(
            permissions::Current::from_facts(facts),
        )),
        SlashCommand::Apps => Some(Intent::OpenApps { enabled: apps }),
        SlashCommand::Review => Some(Intent::OpenReview { cwd }),
        _ => None,
    }
}
