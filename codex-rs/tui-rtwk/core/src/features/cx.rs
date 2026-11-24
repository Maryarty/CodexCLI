//! What a feature is handed (ADR-012): the app and a thread as read-only views, and the
//! ways to change them that a feature has. The context is Flutter's `BuildContext` with an
//! outbox, or Elm's `Cmd`: the feature says what it wants (an ask, a note, a wish,
//! a change of a thread) and the app carries it out after the call, in the order it was said.
//! It never holds `&mut` of the app or of a thread's model.

use super::surface::Scope;
use super::{
    FeatureCommand, FeatureCtx, FeatureNote, FeatureThreadIntent, FeatureThreadParts, FeatureWish,
};
use crate::app::TranscriptText;
use crate::app::effects::Ask;
use crate::app::facts::TuiSettings;
use crate::ids::{ThreadId, Ticket};
use crate::screens::ScreenStates;
use crate::screens::ThreadSlots;
use crate::screens::select::collaboration::Mask;
use crate::threads::daybreak::Access;
use crate::threads::{Item, ThreadCore, ThreadFx, ThreadSet};

/// The app, read-only: the facts a feature decides by. It holds references to parts of the
/// app, so a feature's own state can be `&mut` at the same time.
#[derive(Clone, Copy)]
pub struct AppView<'a> {
    active: Option<&'a ThreadId>,
    threads: &'a ThreadSet,
    screens: &'a ScreenStates,
    tui: &'a TuiSettings,
    modes: &'a [Mask],
    transcript: Option<&'a TranscriptText>,
    daybreak: Option<&'a Access>,
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
            daybreak: None,
        }
    }

    /// The host's Daybreak facts (the account, the model catalog) are read through this view.
    pub(crate) fn with_daybreak(mut self, daybreak: &'a Access) -> Self {
        self.daybreak = Some(daybreak);
        self
    }

    /// What the host knows about Daybreak (`AppEv::Daybreak`): nothing until it said.
    pub fn daybreak(&self) -> Access {
        self.daybreak.cloned().unwrap_or_default()
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

    /// The `[tui]` settings.
    pub fn tui(&self) -> &'a TuiSettings {
        self.tui
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
/// The thread is read through the methods below only (a feature adds the one it needs, each a
/// decision): `core.busy()` would not see a turn start that was said in this step and is not in
/// the model yet (`ThreadFx` hands its tickets to the model after the whole `apply`), so the
/// model itself is not a feature's to read raw.
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

    /// A turn runs or is about to (its start is in flight, or was said earlier in this step).
    pub fn busy(&self) -> bool {
        self.start_said || self.core.busy()
    }

    /// This process writes the thread and the server has not refused it for its model
    /// provider.
    pub fn has_writer(&self) -> bool {
        self.core.attached() && self.slots.chat.provider_error.is_none()
    }

    /// The messages a copy can take, oldest first: an assistant message or a plan, as the server
    /// sent it (oracle `record_agent_markdown`'s callers).
    pub fn response_sources(&self) -> std::vec::IntoIter<&'a str> {
        let sources: Vec<&'a str> = self
            .core
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Agent { text, .. } | Item::Plan { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        sources.into_iter()
    }

    /// The messages of the conversation a reader sees, oldest first: what the user typed and
    /// what the assistant said (no tools, reasoning or plans), as history draws their text.
    pub fn conversation(&self) -> Vec<super::recap::Message> {
        use super::recap::Message;
        self.core
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::User(prompt) => Some(Message::User(prompt.shown().text.into_owned())),
                Item::Agent { text, .. } => Some(Message::Assistant(text.clone())),
                _ => None,
            })
            .collect()
    }

    /// The Daybreak choice of the thread: what `/daybreak` saved for it, else what its metadata
    /// holds (`Metadata::daybreak`).
    pub fn daybreak(&self) -> Option<bool> {
        self.parts
            .and_then(|parts| parts.small_cmds.saved())
            .or(self.slots.chat.daybreak)
    }

    /// The thread is not kept (`config.ephemeral`): it has no metadata to save a choice in.
    pub fn ephemeral(&self) -> bool {
        self.slots.chat.ephemeral
    }

    /// The thread's name, when it has one (`/rename`).
    pub fn name(&self) -> Option<&'a str> {
        self.slots.chat.name.as_deref()
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
    Wish { scope: Scope, wish: FeatureWish },
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
        });
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
        Self { view, fx }
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
}
