//! What a feature is handed (ADR-012): the app and a thread as read-only views, and the
//! ways to change them that a feature has. The context is Flutter's `BuildContext` with an
//! outbox, or Elm's `Cmd`: the feature says what it wants (an ask, a note, a wish,
//! a change of a thread) and the app carries it out after the call, in the order it was said.
//! It never holds `&mut` of the app or of a thread's model.

use std::sync::OnceLock;

use super::lifecycle::{Lifecycle, ThreadSpawn};
use super::surface::{QueueModal, Scope};
use super::verb::PublicVerb;
use super::{
    FeatureCommand, FeatureCtx, FeatureNote, FeatureThreadIntent, FeatureThreadParts, FeatureWish,
};
use crate::app::effects::Ask;
use crate::app::facts::TuiSettings;
use crate::app::{Now, TranscriptText};
use crate::ids::{ThreadId, Ticket};
use crate::keymap::{ActionId, Keymap};
use crate::screens::ScreenStates;
use crate::screens::ThreadSlots;
use crate::screens::chat::BannerOccurrence;
use crate::screens::chat::facts::SideFooter;
use crate::screens::select::collaboration::Mask;
use crate::screens::status;
use crate::threads::daybreak::Access;
use crate::threads::queue::{ModalKind, QueueOperation};
use crate::threads::structured::StructuredTurn;
use crate::threads::{ThreadCore, ThreadFx, ThreadSet, UserMessage};

/// The facts of the app that both kinds of step read (ADR-012, "selectors"): the moment, the
/// `[tui]` settings, what the status screen holds of the account and the catalog, what the
/// host knows of the account's access programs and the key bindings. Typed, read-only handles into the app, `Copy`:
/// a feature's step is handed them whatever its scope (`AppView::facts`, `ThreadCx::facts`),
/// so a selector a slice writes over them (`fn x(facts: AppFacts<'_>) -> T`) works in both.
#[derive(Clone, Copy)]
pub struct AppFacts<'a> {
    now: Now,
    tui: &'a TuiSettings,
    status: &'a status::Model,
    access: &'a Access,
    keys: &'a Keymap,
    banner: Option<&'a BannerOccurrence>,
    rates: Option<&'a status::RateLimits>,
}

impl<'a> AppFacts<'a> {
    pub(crate) fn new(
        now: Now,
        tui: &'a TuiSettings,
        status: &'a status::Model,
        access: &'a Access,
        keys: &'a Keymap,
    ) -> Self {
        Self {
            now,
            tui,
            status,
            access,
            keys,
            banner: None,
            rates: None,
        }
    }

    /// The backend banner of the thread in use is read through these facts (an app step; a
    /// thread's step reads none).
    pub(crate) fn with_banner(mut self, banner: Option<&'a BannerOccurrence>) -> Self {
        self.banner = banner;
        self
    }

    /// The limits of the account of the thread in use, as the last read or rolling update left
    /// them (an app step; a thread's step reads none): what the names of the rate-limit items of
    /// the status line and the title are judged by (`/statusline monthly-limit`).
    pub(crate) fn with_rates(mut self, rates: Option<&'a status::RateLimits>) -> Self {
        self.rates = rates;
        self
    }

    /// The windows the account of the thread in use has now; none before the first read.
    pub fn rate_limits(&self) -> Option<&'a status::RateLimits> {
        self.rates
    }

    /// The backend banner occurrence the account of the thread in use has, shown or not: it
    /// shows while `BannerOccurrence::is_visible(model in use)` says so (oracle
    /// `has_applicable_backend_banner`, which the limit prompts wait for).
    pub fn backend_banner(&self) -> Option<&'a BannerOccurrence> {
        self.banner
    }

    /// The moment the shell last told the app (`AppEv::Clock`); the default one until it did.
    pub fn now(&self) -> Now {
        self.now
    }

    /// The `[tui]` settings.
    pub fn tui(&self) -> &'a TuiSettings {
        self.tui
    }

    /// What the status screen holds: the account the server is signed in to, the model catalog,
    /// the remote connection.
    pub fn status(&self) -> &'a status::Model {
        self.status
    }

    /// What the host knows of the account's access programs and the catalog's (`AppEv::Daybreak`).
    pub fn access(&self) -> &'a Access {
        self.access
    }

    /// The label of the action's first key binding (`⌃t`, `enter`): what a hint the feature's
    /// state decides to show names. None where the action is unbound.
    pub fn key_label(&self, action: ActionId) -> Option<String> {
        self.keys.label(action)
    }
}

impl AppFacts<'static> {
    /// What a step that was given no facts reads: the clock not told yet, the defaults.
    pub(crate) fn unknown() -> AppFacts<'static> {
        static TUI: OnceLock<TuiSettings> = OnceLock::new();
        static STATUS: OnceLock<status::Model> = OnceLock::new();
        static ACCESS: OnceLock<Access> = OnceLock::new();
        static KEYS: OnceLock<Keymap> = OnceLock::new();
        AppFacts {
            now: Now::default(),
            tui: TUI.get_or_init(TuiSettings::default),
            status: STATUS.get_or_init(status::Model::default),
            access: ACCESS.get_or_init(Access::default),
            keys: KEYS.get_or_init(Keymap::defaults),
            banner: None,
            rates: None,
        }
    }
}

/// The app, read-only: the facts a feature decides by. It holds references to parts of the
/// app, so a feature's own state can be `&mut` at the same time. It offers the handles a
/// selector reads (`active`, `thread`, `facts`, `screens`...) and no fact that is one slice's
/// concern: a slice writes its own selectors over them (rule `view_methods_budget`).
#[derive(Clone, Copy)]
pub struct AppView<'a> {
    active: Option<&'a ThreadId>,
    threads: &'a ThreadSet,
    screens: &'a ScreenStates,
    tui: &'a TuiSettings,
    modes: &'a [Mask],
    transcript: Option<&'a TranscriptText>,
    access: Option<&'a Access>,
    keys: Option<&'a Keymap>,
    banner: Option<&'a BannerOccurrence>,
    rates: Option<&'a status::RateLimits>,
    navigation_pending: bool,
    now: Now,
}

impl<'a> AppView<'a> {
    pub(crate) fn new(
        active: Option<&'a ThreadId>,
        threads: &'a ThreadSet,
        screens: &'a ScreenStates,
        tui: &'a TuiSettings,
        modes: &'a [Mask],
        transcript: Option<&'a TranscriptText>,
    ) -> Self {
        Self {
            active,
            threads,
            screens,
            tui,
            modes,
            transcript,
            access: None,
            keys: None,
            banner: None,
            rates: None,
            navigation_pending: false,
            now: Now::default(),
        }
    }

    /// The host's account and catalog facts are read through this view.
    pub(crate) fn with_access(mut self, access: &'a Access) -> Self {
        self.access = Some(access);
        self
    }

    /// The key bindings are read through this view (`AppFacts::key_label`).
    pub(crate) fn with_keys(mut self, keys: &'a Keymap) -> Self {
        self.keys = Some(keys);
        self
    }

    /// The backend banner of the thread in use is read through this view (`AppFacts::backend_banner`).
    pub(crate) fn with_banner(mut self, banner: Option<&'a BannerOccurrence>) -> Self {
        self.banner = banner;
        self
    }

    /// The account's limits are read through this view (`AppFacts::rate_limits`).
    pub(crate) fn with_rates(mut self, rates: Option<&'a status::RateLimits>) -> Self {
        self.rates = rates;
        self
    }

    /// The owner is opening or making a conversation from the current one.
    pub(crate) fn with_navigation_pending(mut self, pending: bool) -> Self {
        self.navigation_pending = pending;
        self
    }

    pub(crate) fn navigation_pending(self) -> bool {
        self.navigation_pending
    }

    /// The moment the app was last told.
    pub(crate) fn at(mut self, now: Now) -> Self {
        self.now = now;
        self
    }

    /// The moment the shell last told the app (`AppEv::Clock`).
    pub fn now(&self) -> Now {
        self.now
    }

    /// The facts both scopes read: the moment, the `[tui]` settings, the account, the catalog.
    pub fn facts(&self) -> AppFacts<'a> {
        let unknown = AppFacts::unknown();
        AppFacts::new(
            self.now,
            self.tui,
            &self.screens.status,
            self.access.unwrap_or(unknown.access),
            self.keys.unwrap_or(unknown.keys),
        )
        .with_banner(self.banner)
        .with_rates(self.rates)
    }

    /// The thread the chat shows (none before the session has started).
    pub fn active(&self) -> Option<&'a ThreadId> {
        self.active
    }

    /// The thread `id` as this process holds it.
    pub fn thread(&self, id: &ThreadId) -> Option<ThreadView<'a>> {
        self.threads.get(id).map(|model| model.view())
    }

    /// The thread the chat shows.
    pub fn active_thread(&self) -> Option<ThreadView<'a>> {
        self.active.and_then(|id| self.thread(id))
    }

    /// The screens' models (the select screen's catalog, the composer's...), as facts.
    pub fn screens(&self) -> &'a ScreenStates {
        self.screens
    }

    /// The server's collaboration-mode presets.
    pub fn modes(&self) -> &'a [Mask] {
        self.modes
    }

    /// The conversation of `thread` as the Markdown of an export, as the UI's renderer makes it
    /// (`CodexApp::with_transcript_text`); none for a thread this process does not hold, a
    /// conversation with nothing in it to export, or an app that was given no renderer.
    pub fn transcript_text(&self, thread: &ThreadId) -> Option<String> {
        let model = self.threads.get(thread)?;
        self.transcript?.render(model)
    }

    /// `config.cwd`: the directory of `thread`, else the one the host read for the status line
    /// (empty when neither is known).
    pub fn working_directory(&self, thread: Option<&ThreadId>) -> String {
        thread
            .and_then(|id| self.threads.get(id))
            .and_then(|model| model.screen_slots().chat.cwd.clone())
            .or_else(|| self.screens.chat.status.cwd.clone())
            .unwrap_or_default()
    }
}

/// One thread, read-only: its own state and its screens' parts; a feature's own part is
/// the `&mut` it is handed beside this.
///
/// What depends on the step (`busy`, `has_writer`) is a method here: `core.busy()` would not see a
/// turn start that was said in this step and is not in the model yet (`ThreadFx` hands its
/// tickets to the model after the whole `apply`), so the model itself is not a feature's to read
/// raw. Every other fact is a selector the slice writes over `core`, `slots` and `parts`.
#[derive(Clone, Copy)]
pub struct ThreadView<'a> {
    core: &'a ThreadCore,
    slots: &'a ThreadSlots,
    /// The features' parts of the thread, read-only: none inside a feature's own thread step
    /// (it holds its part `&mut` there).
    parts: Option<&'a FeatureThreadParts>,
    /// A turn start was said in this step and the model does not hold its ticket yet: the
    /// thread is busy for whoever reads after it (ADR-012: "a feature after it sees the
    /// thread already busy").
    start_said: bool,
}

impl<'a> ThreadView<'a> {
    pub(crate) fn new(core: &'a ThreadCore, slots: &'a ThreadSlots) -> Self {
        Self {
            core,
            slots,
            parts: None,
            start_said: false,
        }
    }

    /// Reads the features' parts of the thread too.
    pub(crate) fn with_parts(mut self, parts: &'a FeatureThreadParts) -> Self {
        self.parts = Some(parts);
        self
    }

    pub fn id(&self) -> &'a ThreadId {
        self.core.id()
    }

    /// A turn runs.
    pub fn is_running(&self) -> bool {
        self.core.is_running()
    }

    /// A turn runs or is about to (its start is in flight, or was said earlier in this step, or
    /// a compaction was asked for and its turn has not begun).
    pub fn busy(&self) -> bool {
        self.start_said || self.core.busy() || self.slots.chat.is_compacting()
    }

    /// This process writes the thread and the server has not refused it for its model
    /// provider.
    pub fn has_writer(&self) -> bool {
        self.core.attached() && self.slots.chat.provider_error.is_none()
    }

    /// The thread's event stream was lost and has not come back (`ThreadEv::StreamLost`): what
    /// the user sends waits in the queue (the oracle's `thread_stream_disconnected`).
    pub fn stream_lost(&self) -> bool {
        self.core.stream_lost()
    }

    /// The thread's own domain facts (its transcript, its terminals...), read-only: a slice's
    /// selector reads what it needs through this handle.
    pub fn core(&self) -> &'a ThreadCore {
        self.core
    }

    /// The thread's parts for the screens (the chat's name, directory, provider error...).
    pub fn slots(&self) -> &'a ThreadSlots {
        self.slots
    }

    /// The features' parts of the thread, read-only: none inside a feature's own thread step
    /// (it holds its part `&mut` there). A slice reads its own part by its field.
    pub fn parts(&self) -> Option<&'a FeatureThreadParts> {
        self.parts
    }
}

/// What an `AppCx` call asked for, in order; the app carries it out after the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    /// An RPC about `thread` (empty: about the app); its answer comes back to
    /// `Feature::answered` with `ctx`.
    Ask {
        thread: ThreadId,
        command: FeatureCommand,
        ctx: FeatureCtx,
    },
    /// An ask of the host (write a file, suggest a title, copy): its answer comes back to
    /// `Feature::effect_done` with `ctx`.
    Effect { ask: Ask, ctx: FeatureCtx },
    /// A line in the transcript of `thread` (none: the thread in use, or the chat before
    /// any thread).
    Note {
        thread: Option<ThreadId>,
        note: FeatureNote,
    },
    /// A change of one thread's feature part: a queued `ThreadEv::FeatureIntent`.
    Thread {
        thread: ThreadId,
        intent: FeatureThreadIntent,
    },
    /// What held `thread`'s input back is over: every feature's `on_settled`.
    Settled { thread: ThreadId },
    /// A one-shot for the surface (ADR-013 Ж): it is queued under `scope` (the directory the
    /// feature's data is for) and shown once by the surface's presenter.
    Wish {
        scope: Scope,
        wish: FeatureWish,
        queue_modal: Option<QueueModal>,
    },
    /// What a feature asked of a thread's life (open, spawn, discard, submit, give back, a
    /// structured question): the app's own steps carry it out (`lifecycle.rs`).
    Lifecycle(Lifecycle),
    /// What the chat reads of the thread in use that a feature knows.
    Chat(ChatFacts),
    /// A verb of the whitelist for the one that owns it (`PublicVerb`): the app routes it.
    Verb(PublicVerb),
    /// The app ends now (`ExitMode::Immediate`): what the host kept for after the exit runs.
    End,
}

/// What the chat's footer and its command gate read of the thread in use, said by the feature that
/// knows it (a side conversation: whether the thread is one, and the footer's label). Each step
/// says all of it: the app writes both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatFacts {
    /// The thread in use is a side conversation: the commands it lacks are gated by it.
    pub side_conversation: bool,
    /// The footer's side label (`Ctrl+/ for side`, the parent's name and status).
    pub side: Option<SideFooter>,
}

/// The context of an app-scoped step of a feature: the app as a view, and the outbox.
pub struct AppCx<'a> {
    view: AppView<'a>,
    out: Vec<Out>,
}

impl<'a> AppCx<'a> {
    pub(crate) fn new(view: AppView<'a>) -> Self {
        Self {
            view,
            out: Vec::new(),
        }
    }

    /// The app, read-only.
    pub fn view(&self) -> AppView<'a> {
        self.view
    }

    /// Asks the server about `thread` (none: about the app, not a thread); the answer comes
    /// back to the feature's `answered` with `ctx`.
    pub fn ask(
        &mut self,
        thread: Option<&ThreadId>,
        command: impl Into<FeatureCommand>,
        ctx: impl Into<FeatureCtx>,
    ) {
        self.out.push(Out::Ask {
            thread: thread.cloned().unwrap_or_default(),
            command: command.into(),
            ctx: ctx.into(),
        });
    }

    /// Asks the host for `ask`, one of its capabilities (`effects::Ask`: write a file, suggest a
    /// title, copy); the answer comes back to the feature's `effect_done` with `ctx`.
    pub fn effect(&mut self, ask: Ask, ctx: impl Into<FeatureCtx>) {
        self.out.push(Out::Effect {
            ask,
            ctx: ctx.into(),
        });
    }

    /// Says `note` in the transcript of `thread` (none: the thread in use).
    pub fn note(&mut self, thread: Option<ThreadId>, note: impl Into<FeatureNote>) {
        self.out.push(Out::Note {
            thread,
            note: note.into(),
        });
    }

    /// Changes `thread`'s part of a feature: the change is a thread event, applied by
    /// `threads::apply` under the thread's writer.
    pub fn thread(&mut self, thread: &ThreadId, intent: impl Into<FeatureThreadIntent>) {
        self.out.push(Out::Thread {
            thread: thread.clone(),
            intent: intent.into(),
        });
    }

    /// Says that what held `thread`'s input back is over (a goal was set): what waits for it
    /// (the queue) goes on, in the features' own words, `Feature::on_settled`.
    pub fn settled(&mut self, thread: &ThreadId) {
        self.out.push(Out::Settled {
            thread: thread.clone(),
        });
    }

    /// Asks the surface to show `wish` once, for `scope`: the presenter of the surface opens
    /// the page or routes to it where the feature is already shown, and says it did
    /// (`Intent::WishShown`); nothing in the feature learns how.
    pub fn wish(&mut self, scope: impl Into<Scope>, wish: impl Into<FeatureWish>) {
        self.out.push(Out::Wish {
            scope: scope.into(),
            wish: wish.into(),
            queue_modal: None,
        });
    }

    /// Shows a modal over `thread`: its result settles the hold on queued input.
    pub(super) fn wish_for_queue(
        &mut self,
        thread: ThreadId,
        kind: ModalKind,
        wish: impl Into<FeatureWish>,
    ) {
        self.out.push(Out::Wish {
            scope: Scope::Thread(thread),
            wish: wish.into(),
            queue_modal: Some(QueueModal {
                operation: None,
                kind,
            }),
        });
    }

    /// Carries the operation already issued for the queued command that opened the modal.
    pub(super) fn wish_for_queued_operation(
        &mut self,
        operation: QueueOperation,
        kind: ModalKind,
        wish: impl Into<FeatureWish>,
    ) -> bool {
        let thread = operation.thread();
        if !self
            .view
            .threads
            .get(thread)
            .and_then(|model| model.due_command())
            .is_some_and(|due| due.operation() == &operation)
        {
            return false;
        }
        let visible = self.view.active() == Some(thread);
        self.out.push(Out::Wish {
            scope: Scope::Thread(thread.clone()),
            wish: wish.into(),
            queue_modal: Some(QueueModal {
                operation: Some(operation),
                kind,
            }),
        });
        visible
    }

    /// Asks the app for one of its public verbs (`PublicVerb`): the app carries it out in the
    /// owner's own step, nothing goes through the UI.
    pub fn verb(&mut self, verb: PublicVerb) {
        self.out.push(Out::Verb(verb));
    }

    /// Ends the app (`ExitMode::Immediate`): the view quits, the host runs what it kept for after
    /// the exit (a daemon update).
    pub fn end_app(&mut self) {
        self.out.push(Out::End);
    }

    /// Shows `thread`, resuming it when this process does not hold it (`Feature::thread_opened`
    /// answers with `ctx`: `Err` when the server has no such thread).
    pub fn open_thread(&mut self, thread: &ThreadId, ctx: impl Into<FeatureCtx>) {
        self.out.push(Out::Lifecycle(Lifecycle::Open {
            thread: thread.clone(),
            ctx: ctx.into(),
        }));
    }

    /// Makes a thread (a fork, a fresh start); `Feature::thread_opened` is told with `ctx` once it
    /// is open here, before it is shown. Refused (an `Err`) while another conversation is being
    /// made.
    pub fn spawn_thread(&mut self, spawn: ThreadSpawn, ctx: impl Into<FeatureCtx>) {
        self.out.push(Out::Lifecycle(Lifecycle::Spawn {
            spawn,
            ctx: ctx.into(),
        }));
    }

    /// An isolated request to the model (oracle `temporary_structured_request`): an ephemeral
    /// read-only thread with no tools starts in the directory of `turn.source`, runs one turn
    /// of `turn.prompt` that follows `turn.schema`, and is let go. `Feature::structured_done`
    /// is told with `ctx` the last message, or why there is none (a server refusal, a turn that
    /// did not complete, 30 seconds of silence). Nothing of the thread reaches the screen.
    pub fn structured_turn(&mut self, turn: StructuredTurn, ctx: impl Into<FeatureCtx>) {
        self.out.push(Out::Lifecycle(Lifecycle::Structured {
            turn: Box::new(turn),
            ctx: ctx.into(),
        }));
    }

    /// Drops `thread`: its turn is interrupted, the server stops holding it and the app forgets
    /// it. The thread in use is never dropped.
    pub fn discard(&mut self, thread: &ThreadId) {
        self.out.push(Out::Lifecycle(Lifecycle::Discard {
            thread: thread.clone(),
        }));
    }

    /// The user's message in `thread`, through the thread's own input pipeline (a running turn
    /// is steered, a thread that waits queues it).
    pub fn submit(&mut self, thread: &ThreadId, message: UserMessage) {
        self.out.push(Out::Lifecycle(Lifecycle::Submit {
            thread: thread.clone(),
            message,
        }));
    }

    /// Text that could not go is put back in `thread`'s composer.
    pub fn give_back(&mut self, thread: &ThreadId, text: String) {
        self.out.push(Out::Lifecycle(Lifecycle::GiveBack {
            thread: thread.clone(),
            text,
        }));
    }

    /// Whether `thread` keeps the notice of an interrupted turn out of its transcript: a side
    /// conversation says it of its own thread (`set_interrupted_turn_notice_mode`). The
    /// thread's own, until it is told again; the default is to show it.
    pub fn quiet_interrupts(&mut self, thread: &ThreadId, quiet: bool) {
        self.out.push(Out::Lifecycle(Lifecycle::QuietInterrupts {
            thread: thread.clone(),
            quiet,
        }));
    }

    /// Says what the chat reads of the thread in use (a side conversation's flag and footer).
    pub fn chat_facts(&mut self, facts: ChatFacts) {
        self.out.push(Out::Chat(facts));
    }

    /// What was asked, in order.
    pub(crate) fn into_out(self) -> Vec<Out> {
        self.out
    }
}

/// The context of a thread-scoped step of a feature: the thread as a view, and the thread's
/// effects (its RPCs, its tickets).
///
/// There is one for every step, and the effects are the writer's (ADR-003): a thread this
/// process does not write (detached, another process writes it, or the server refused it for
/// its provider) still tells its features what happened (a reply, a turn that ended), so a
/// part that waits for it (a spinner, a pending flag) does not stay stuck, but the effect door
/// is shut there: `request` says `None`. A feature that must not even change its part there
/// asks `view.has_writer()`.
pub struct ThreadCx<'a, 'b> {
    pub view: ThreadView<'a>,
    /// The app's facts at this step (the moment, the account, the `[tui]` settings): read-only
    /// handles, the same a step of the app's scope reads through `AppView::facts`.
    pub facts: AppFacts<'a>,
    /// Private: a feature has the effects this type names (`request`, and what a feature
    /// that needs more adds here), never the thread's lifecycle RPCs. An effect that starts a
    /// turn sets `view.start_said` (the next feature in the list sees the thread busy).
    fx: &'a mut ThreadFx<'b>,
}

impl<'a, 'b> ThreadCx<'a, 'b> {
    /// The context of a step over `view`. A turn start the thread said before this step (its
    /// own `send_next`) already counts: the features' hooks run after the thread's own work.
    pub(crate) fn new(mut view: ThreadView<'a>, fx: &'a mut ThreadFx<'b>) -> Self {
        view.start_said = fx.start_said();
        let facts = fx.facts();
        Self { view, facts, fx }
    }

    /// An RPC of the feature on this thread: `ctx` comes back with the answer to the
    /// feature's `thread_answered`. None where this process does not write the thread: nothing
    /// was asked, no answer will come.
    pub fn request(
        &mut self,
        ctx: impl Into<FeatureCtx>,
        command: impl Into<FeatureCommand>,
    ) -> Option<Ticket> {
        if !self.view.has_writer() {
            return None;
        }
        Some(self.fx.request_feature(ctx, command))
    }

    /// Publish the current thread's acknowledged name to its chat surfaces.
    pub(crate) fn publish_name(&mut self, name: String) {
        if self.view.has_writer() && self.view.slots().chat.name.as_ref() != Some(&name) {
            self.fx.publish_name(name);
        }
    }

    /// Starts a turn of this thread with `message` (Retry, Rewrite): the thread must be idle,
    /// which the feature asks (`view.busy()`); the message comes back when a start was already
    /// said in this step or this process does not write the thread. The next feature in the
    /// list sees the thread busy.
    pub fn start_turn(&mut self, message: UserMessage) -> Result<Ticket, UserMessage> {
        if !self.view.has_writer() {
            return Err(message);
        }
        let ticket = self.fx.start_turn(message)?;
        self.view.start_said = true;
        Ok(ticket)
    }

    /// Interrupts the thread's running turn (none: nothing runs, or this process does not write
    /// the thread). True when an interrupt was said.
    pub fn interrupt(&mut self) -> bool {
        if !self.view.has_writer() {
            return false;
        }
        let Some(turn) = self.view.core().running_turn().cloned() else {
            return false;
        };
        self.fx.interrupt_turn(turn);
        true
    }
}
