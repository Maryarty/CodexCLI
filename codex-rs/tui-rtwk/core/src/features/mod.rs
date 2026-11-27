//! Core feature slices (ADR-012). A feature is a flow that is not a screen: a slash command
//! that opens a menu or a prompt, asks the server, says what happened in the transcript
//! (`/goal`, `/export`, `/rename`, `/plugins`), or thread state that reacts to the turn
//! lifecycle (the queue). Its code lives in `core/src/features/<f>/` and its words and views in
//! `src/features/<f>/`; `build.rs` registers it from the directory (`feature_list!`) and generates the root
//! enums and their dispatch, as `screen_list!` does for the screens.
//!
//! A feature implements `Feature` for a marker type: its own state, per-thread part, intents,
//! RPCs, notes, wishes and interests are associated types, its steps are methods with default
//! no-ops. What the feature shows (a menu, a prompt, a sheet) is its UI half's
//! (`src/features/<f>/`, ADR-013): core says a domain fact (a wish, a note) and takes verbs. It reads the app through `AppView` / `ThreadView` and changes it only through the
//! outbox of `AppCx` and the effects of `ThreadCx`; it never names `CodexApp` or
//! `ThreadModel` (rule `features_hold_no_app`). The hub carries one generic variant per
//! component (`Intent::Feature`,
//! `NoteKey::Feature`, `ThreadEv::FeatureIntent`, `ThreadCtx::Feature`, `Command::Feature`)
//! and never a variant per feature (rule `hub_enums_only_shrink`).
//!
//! The order of the list (by name) is the order of every hook that runs over all features
//! (`on_resumed`, `on_turn_end`, `on_interrupted`, `on_settled`).
//!
//! A feature reaches the server through `wire` (an RPC) and the host through `AppCx::effect`
//! (an `effects::Ask`: write a file, suggest a title, copy; the host's list of capabilities,
//! never a feature's), whose answer comes back to `effect_done`.
//!
//! The recipe (files, the one list line, hooks, tests) is the end of ADR-012 in
//! `docs/rtwk-port/architecture.md`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;

use crate::app::Now;
use crate::app::effects::Done;
use crate::commands::{CommandRule, SlashCommand};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::{RpcError, ServerError};
use crate::screens::SeedError;
use crate::screens::status::RateRead;
use crate::threads::queue::ModalReceipt;
use crate::threads::{LiveThreadEvent, Tone, TurnEnd};

mod cx;
mod lifecycle;
mod remote;
mod surface;
mod verb;

pub use crate::threads::structured::{
    Kind as StructuredKind, StructuredTurn, Why as StructuredFailure,
};
pub use cx::{AppCx, AppFacts, AppView, ChatFacts, Out, ThreadCx, ThreadView};
pub use lifecycle::{Connection, Lifecycle, Refusal, ThreadSpawn};
pub use remote::Remote;
pub use settings::SettingId;
pub use surface::{InterestStep, Interests, Scope, SurfaceWish, Wishes};
pub use verb::{PermissionSetup, PublicVerb};

/// The type `$t` a slice wrote, or `$default` when it wrote none (`feature!`).
macro_rules! feature_or {
    ($default:ty;) => {
        $default
    };
    ($default:ty; $t:ty) => {
        $t
    };
}

/// `wire` and `decode` of a slice without RPCs: its `Command` is `Never<Self>` and there is nothing
/// to make a request of (`feature!`); a slice that has `type Command` writes both in its methods.
macro_rules! feature_no_rpcs {
    () => {
        fn wire(
            command: &$crate::features::Never<Self>,
            _thread: &$crate::ids::ThreadId,
            _id: codex_app_server_protocol::RequestId,
        ) -> codex_app_server_protocol::ClientRequest {
            command.absurd()
        }

        fn decode(
            command: &$crate::features::Never<Self>,
            _result: serde_json::Value,
        ) -> Result<$crate::features::Never<Self>, $crate::rpc::RpcError> {
            command.absurd()
        }
    };
    ($command:ty) => {};
}

/// `impl Feature for Marker`, with what a slice does not have left out: an associated type it
/// does not write is `Never<Self>` (`()` for `State` and `ThreadPart`), `wire` and `decode` of a
/// slice without `type Command` are `command.absurd()`, and every method it does not write is the
/// trait's default. A slice says what it has, in the order of the trait:
///
/// ```ignore
/// feature! {
///     impl Feature for Processes {
///         const NAME: &'static str = "processes";
///         const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Ps, Sheet));
///         type Target = Sheet;
///         type Intent = Intent;
///         type Note = Note;
///         {
///             fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) { .. }
///         }
///     }
/// }
/// ```
///
/// The constants are `NAME`, then `SEED`, `SEED_NOTE`, `OPENS`, `RUNS`, `COMMAND_RULES`,
/// `REFUSAL`, `SETTING`; the types are `State`,
/// `ThreadPart`, `Target`, `Intent`, `ThreadIntent`, `Command`, `Reply`, `Ctx`, `Note`, `Fact`, `Wish`,
/// `Interest`, `Event`; the methods are the items of the last braces. Each is written as in the
/// trait (rules scan these lines); a doc comment goes on a method, not on a constant or a type.
/// rustfmt does not format the inside of a macro call: keep the layout of the trait's methods.
macro_rules! feature {
    (
        impl Feature for $marker:ty {
            const NAME: &'static str = $name:expr;
            $(const SEED: $seed_ty:ty = $seed:expr;)?
            $(const SEED_NOTE: $seed_note_ty:ty = $seed_note:expr;)?
            $(const OPENS: $opens_ty:ty = $opens:expr;)?
            $(const RUNS: $runs_ty:ty = $runs:expr;)?
            $(const COMMAND_RULES: $rules_ty:ty = $rules:expr;)?
            $(const REFUSAL: $refusal_ty:ty = $refusal:expr;)?
            $(const SETTING: $setting_ty:ty = $setting:expr;)?
            $(type State = $state:ty;)?
            $(type ThreadPart = $part:ty;)?
            $(type Target = $target:ty;)?
            $(type Intent = $intent:ty;)?
            $(type ThreadIntent = $thread_intent:ty;)?
            $(type Command = $command:ty;)?
            $(type Reply = $reply:ty;)?
            $(type Ctx = $ctx:ty;)?
            $(type Note = $note:ty;)?
            $(type Fact = $fact:ty;)?
            $(type Wish = $wish:ty;)?
            $(type Interest = $interest:ty;)?
            $(type Event = $event:ty;)?
            $({ $($method:item)* })?
        }
    ) => {
        impl $crate::features::Feature for $marker {
            const NAME: &'static str = $name;
            $(const SEED: $seed_ty = $seed;)?
            $(const SEED_NOTE: $seed_note_ty = $seed_note;)?
            $(const OPENS: $opens_ty = $opens;)?
            $(const RUNS: $runs_ty = $runs;)?
            $(const COMMAND_RULES: $rules_ty = $rules;)?
            $(const REFUSAL: $refusal_ty = $refusal;)?
            $(const SETTING: $setting_ty = $setting;)?
            type State = feature_or!((); $($state)?);
            type ThreadPart = feature_or!((); $($part)?);
            type Target = feature_or!($crate::features::Never<Self>; $($target)?);
            type Intent = feature_or!($crate::features::Never<Self>; $($intent)?);
            type ThreadIntent = feature_or!($crate::features::Never<Self>; $($thread_intent)?);
            type Command = feature_or!($crate::features::Never<Self>; $($command)?);
            type Reply = feature_or!($crate::features::Never<Self>; $($reply)?);
            type Ctx = feature_or!($crate::features::Never<Self>; $($ctx)?);
            type Note = feature_or!($crate::features::Never<Self>; $($note)?);
            type Fact = feature_or!($crate::features::Never<Self>; $($fact)?);
            type Wish = feature_or!($crate::features::Never<Self>; $($wish)?);
            type Interest = feature_or!($crate::features::Never<Self>; $($interest)?);
            type Event = feature_or!($crate::features::Never<Self>; $($event)?);
            feature_no_rpcs!($($command)?);
            $($($method)*)?
        }
    };
}

// The `pub mod` line of every slice and `feature_list!` are written by `build.rs` from the
// directories of `core/src/features/` (`test-mods/registry.rs`): a slice is a directory with a
// `mod.rs` and an `impl Feature for <Marker>`, and no line of this file names it. rustfmt does
// not follow the generated modules: `tools/rtwk-port/registry/fmt.sh` formats them.
include!(concat!(env!("OUT_DIR"), "/features.rs"));

/// The type of a component feature `F` does not use (no RPCs, no wishes...): nothing
/// can be made of it, so a match over it has no arms. It carries the feature, so two features
/// that both have none still have distinct types and the root enums' `From` impls do not
/// overlap: a feature says `type Wish = Never<Self>;`.
pub enum Never<F> {
    #[doc(hidden)]
    Void(
        std::convert::Infallible,
        std::marker::PhantomData<fn() -> F>,
    ),
}

/// What the server's refusal of a feature's RPC means to the feature (`Feature::REFUSAL`).
pub type RefusalFn<C, R> = fn(&C, &ServerError) -> Option<R>;

/// How a script's `fact` step reads one fact of a feature from its JSON (`Feature::SEED`).
pub type SeedFn<T> = fn(serde_json::Value) -> Result<T, SeedError>;

/// A slash command a feature runs (`Feature::RUNS`), as the composer's line made it: the command,
/// the text after its name (trimmed; empty for the bare command) and the thread it was typed in
/// (none before the session has started). The feature reads its own arguments from it, as
/// Flutter's `onGenerateRoute` reads `RouteSettings.arguments`: the hub does not parse them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slash {
    pub command: SlashCommand,
    pub args: String,
    pub source: Option<ThreadId>,
}

impl Slash {
    /// The arguments, none for the bare command.
    pub fn args(&self) -> Option<&str> {
        Some(self.args.as_str()).filter(|args| !args.is_empty())
    }

    /// The arguments by value, none for the bare command.
    pub fn into_args(self) -> Option<String> {
        Some(self.args).filter(|args| !args.is_empty())
    }
}

impl<F> Clone for Never<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> std::fmt::Debug for Never<F> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Never::Void(never, _) => match *never {},
        }
    }
}

impl<F> PartialEq for Never<F> {
    fn eq(&self, _: &Self) -> bool {
        match self {
            Never::Void(never, _) => match *never {},
        }
    }
}

impl<F> Eq for Never<F> {}

impl<F> Copy for Never<F> {}

impl<F> Never<F> {
    /// Anything, from a value that cannot exist: what a step a feature has no component for
    /// returns (`command.absurd()` in `wire` and `decode` of a feature without RPCs).
    pub fn absurd<T>(&self) -> T {
        match self {
            Never::Void(never, _) => match *never {},
        }
    }
}

/// A feature of the core. Every associated type that a feature does not need is `Never<Self>`
/// (or `()` for the two states); every other one is the feature's own type, so the root enums'
/// `From` impls stay unique. Every method has a default that does nothing, so a feature
/// writes the steps it has. `wire` and `decode` have no default: a feature with RPCs must
/// say how they go out and come back, one with none matches on its `Never<Self>` command.
pub trait Feature: 'static {
    /// The directory and list name (`goal`).
    const NAME: &'static str;

    /// How a script's `fact` step makes one of the feature's facts (`seed_fact`): the feature's
    /// own reading of the JSON, so a slice seeds its facts without editing the shared loader.
    /// None: the feature has no seedable fact.
    const SEED: Option<SeedFn<Self::Fact>> = None;

    /// How a script's `note` step makes one of the feature's notes (`seed_note`): the line the
    /// feature says in a transcript (`AppCx::note`), read from the step's JSON by the feature
    /// itself, so a test sees how a note is drawn without running the flow that says it. None:
    /// the feature has no seedable note.
    const SEED_NOTE: Option<SeedFn<Self::Note>> = None;

    /// The slash command that opens the feature's own sheet and what it names
    /// (`Target::Feature`; the generated `FeatureTarget` and `target_of`). None: no command opens
    /// a sheet of the feature (`/goal`, `/rename` run a verb), and `Target` is `Never<Self>`.
    const OPENS: Option<(SlashCommand, Self::Target)> = None;

    /// The slash commands the feature runs, with their arguments (`slash`): the verbs that are no
    /// sheet. The one list line generates `runs_of` and `apply_slash`; `SessionIntent::Feature`
    /// carries the line's text to the owner, so the hub names no slice's command. A command has
    /// one owner (`one_owner_per_command`); one that opens a sheet bare and runs with arguments
    /// (`/export <path>`, `/mcp verbose`) is in `OPENS` and `RUNS` of the same feature.
    const RUNS: &'static [SlashCommand] = &[];

    /// Dispatch checks and routes of this feature beyond the usual sheet or verb.
    const COMMAND_RULES: &'static [CommandRule] = &[];

    /// What the server's refusal of one of the feature's RPCs means to the feature, when it is an
    /// answer and no failure (a list the server does not know, a write whose reply may have been
    /// lost after the server committed it): the reply the feature reads. None: every refusal is
    /// a failure. A constant, not a method: no feature but the one that asks needs it.
    const REFUSAL: Option<RefusalFn<Self::Command, Self::Reply>> = None;

    /// The row of the settings panel that opens the feature's own sheet (`open_setting`): the
    /// panel asks the list, so it names no slice. None: no row opens a sheet of the feature.
    const SETTING: Option<SettingId> = None;

    /// The feature's app-scoped state: a field of `CodexApp`, changed only by this trait's
    /// `intent`, `answered`, `effect_done`, `on_fact`, `on_interest` and `on_resumed`.
    type State: Clone + std::fmt::Debug + Default + PartialEq + Eq;
    /// The feature's per-thread state: a field of `ThreadModel`, changed by `thread_intent`
    /// only under the thread's writer, and by what happened to the thread (`thread_answered`,
    /// the hooks) always: a part that waits for a reply never stays stuck because the thread
    /// lost its writer. The effects of `ThreadCx` are the writer's (`ThreadCx::request`).
    type ThreadPart: Clone + std::fmt::Debug + Default + PartialEq + Eq;
    /// What a slash command that opens the feature's sheet names (`FeatureTarget`): a unit type
    /// of the feature's own (the UI half knows how to open it), or `Never<Self>`.
    type Target: Copy + Clone + std::fmt::Debug + PartialEq + Eq;
    /// What the user or a view asks of the feature (`Intent::Feature`).
    type Intent: Clone + std::fmt::Debug + PartialEq + Eq;
    /// What is asked of one thread's feature part (`ThreadEv::FeatureIntent`), applied by
    /// `threads::apply`.
    type ThreadIntent: Clone + std::fmt::Debug + PartialEq + Eq;
    /// The feature's RPCs: data; `wire` makes the request.
    type Command: Clone + std::fmt::Debug + PartialEq + Eq;
    /// The decoded answer of a command.
    type Reply: Clone + std::fmt::Debug + PartialEq + Eq;
    /// What an RPC was for, kept with its ticket and given back with the answer.
    type Ctx: Clone + std::fmt::Debug + PartialEq + Eq;
    /// A line the feature says in the transcript (`NoteKey::Feature`); its words are
    /// `src/features/<f>/strings.rs`.
    type Note: Clone + std::fmt::Debug + PartialEq + Eq;
    /// What the host tells the feature unasked (`AppEv::Feature`): a config feed, the presets
    /// the config holds. The host builds it with the feature's own type and `From`.
    type Fact: Clone + std::fmt::Debug + PartialEq + Eq;
    /// A one-shot the feature asks the surface to show (`AppCx::wish`): the setup page of an
    /// installed plugin, a question over a goal, the name prompt. The surface's presenter shows it once and says so (`WishShown`); the
    /// feature never knows how, or whether, it was drawn (ADR-013 Ж).
    type Wish: Clone + std::fmt::Debug + PartialEq + Eq;
    /// What a surface can be interested in (ADR-013 И): a counter per value, raised by a widget
    /// that shows the data and dropped when it goes. `on_interest` loads on the first, and a
    /// reconnect asks again for every interest still held.
    type Interest: Clone + std::fmt::Debug + PartialEq + Eq;
    /// What the feature makes of the server's stream (`project`, `project_request`), delivered to
    /// `on_event`: data of the feature's own, so the hub names no slice's case of a notification.
    type Event: Clone + std::fmt::Debug + PartialEq + Eq;

    /// An intent of the feature.
    fn intent(state: &mut Self::State, intent: Self::Intent, cx: &mut AppCx<'_>) {
        let _ = (state, intent, cx);
    }

    /// A slash command of `RUNS`, typed with `slash.args` after its name: the feature checks the
    /// arguments, says what is wrong with them as a note, and does what they ask through `cx`.
    fn slash(state: &mut Self::State, slash: Slash, cx: &mut AppCx<'_>) {
        let _ = (state, slash, cx);
    }

    /// The answer of an RPC the feature asked through `AppCx::ask`.
    fn answered(
        state: &mut Self::State,
        ctx: Self::Ctx,
        result: Result<Self::Reply, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let _ = (state, ctx, result, cx);
    }

    /// The host's answer to an ask the feature made through `AppCx::effect`, with the `ctx` it
    /// gave. A copy answers twice (`Done::Copy(Ok(CopyStatus::Pending(_)))`, then how it
    /// ended): `Done::is_final` says which one this is.
    fn effect_done(state: &mut Self::State, ctx: Self::Ctx, done: Done, cx: &mut AppCx<'_>) {
        let _ = (state, ctx, done, cx);
    }

    /// Whether the oracle shows the note as information, a success or an error.
    fn note_tone(note: &Self::Note) -> Tone {
        let _ = note;
        Tone::Info
    }

    /// A fact the host sent unasked (`AppEv::Feature`): the feature keeps it, and changes what
    /// its sheet shows by it (the sheet reads the state).
    fn on_fact(state: &mut Self::State, fact: Self::Fact, cx: &mut AppCx<'_>) {
        let _ = (state, fact, cx);
    }

    /// A resumed thread has been applied and is the one in use (the oracle's
    /// `maybe_prompt_resume_paused_goal_after_resume` is one).
    fn on_resumed(state: &mut Self::State, thread: &ThreadId, cx: &mut AppCx<'_>) {
        let _ = (state, thread, cx);
    }

    /// The user interrupted the running turn of `thread` (Esc, Ctrl+C: `Intent::Interrupt`), after
    /// the thread said the interrupt (the oracle's `pause_active_goal_for_interrupt` is one).
    fn on_interrupt(state: &mut Self::State, thread: &ThreadId, cx: &mut AppCx<'_>) {
        let _ = (state, thread, cx);
    }

    /// An intent for this thread's part: an ask, applied only under the thread's writer.
    fn thread_intent(
        part: &mut Self::ThreadPart,
        intent: Self::ThreadIntent,
        cx: &mut ThreadCx<'_, '_>,
    ) {
        let _ = (part, intent, cx);
    }

    /// The answer of an RPC the feature asked through `ThreadCx::request`: delivered whether or
    /// not this process still writes the thread (the effects are shut then, not the news).
    fn thread_answered(
        part: &mut Self::ThreadPart,
        ctx: Self::Ctx,
        result: Result<Self::Reply, RpcError>,
        cx: &mut ThreadCx<'_, '_>,
    ) {
        let _ = (part, ctx, result, cx);
    }

    /// Reconcile a thread-local fact with the feature part even without a writer. An intent
    /// returned here continues through the writer-gated, effectful `thread_intent`.
    fn thread_fact(
        part: &mut Self::ThreadPart,
        intent: Self::ThreadIntent,
    ) -> Option<Self::ThreadIntent> {
        let _ = part;
        Some(intent)
    }

    /// A turn of the thread ended (completed, failed or interrupted): what waited for it
    /// goes on.
    fn on_turn_end(part: &mut Self::ThreadPart, end: TurnEnd, cx: &mut ThreadCx<'_, '_>) {
        let _ = (part, end, cx);
    }

    /// The owner applied a live item or terminal event, including notifications held while
    /// attaching. A terminal event includes any input it returned to the composer.
    fn on_live_thread_event(part: &mut Self::ThreadPart, event: LiveThreadEvent<'_>) {
        let _ = (part, event);
    }

    /// What the feature makes of a server notification, if it is one it follows (ADR-012, "the
    /// stream"): a pure reading, with no state and no thread, delivered to `on_event` for every
    /// notification of the stream, whatever thread it names. The notification still goes to the
    /// threads as before; the event is the feature's own copy of the part it needs.
    fn project(notification: &ServerNotification) -> Option<Self::Event> {
        let _ = notification;
        None
    }

    /// The same for a server request: observe-only, the request is never answered by the
    /// feature (the app asks the user as before).
    fn project_request(request: &ServerRequest) -> Option<Self::Event> {
        let _ = request;
        None
    }

    /// An event of `project` or `project_request`.
    fn on_event(state: &mut Self::State, event: Self::Event, cx: &mut AppCx<'_>) {
        let _ = (state, event, cx);
    }

    /// A thread the feature asked for (`AppCx::spawn`, `AppCx::open_thread`) is open here, or
    /// could not be made: `Ok` carries its id, before it is shown.
    fn thread_opened(
        state: &mut Self::State,
        ctx: Self::Ctx,
        result: Result<ThreadId, Refusal>,
        cx: &mut AppCx<'_>,
    ) {
        let _ = (state, ctx, result, cx);
    }

    /// The answer of a structured turn the feature asked for (`AppCx::structured_turn`): the
    /// last message of the ephemeral thread's one turn, which follows the schema the feature
    /// gave (the feature parses it), or why there is none.
    fn structured_done(
        state: &mut Self::State,
        ctx: Self::Ctx,
        result: Result<String, StructuredFailure>,
        cx: &mut AppCx<'_>,
    ) {
        let _ = (state, ctx, result, cx);
    }

    /// Whether the structured turn the feature asked for is still wanted: asked once the
    /// ephemeral thread has started and before its turn does (the oracle's `request_is_fresh`
    /// of `handle_recap_started`). A feature that says no is told `StructuredFailure::Unwanted`
    /// through `structured_done`, the thread is let go and its turn never starts.
    fn structured_wanted(state: &Self::State, ctx: &Self::Ctx, view: &AppView<'_>) -> bool {
        let _ = (state, ctx, view);
        true
    }

    /// The server completed a user message of `thread` while the thread had no name (the
    /// oracle's `automatic_title_user_message`): asked after the thread took the message in,
    /// for every such message until a name is saved, so a feature that names the thread by its
    /// first message keeps its own record of what it asked. `text` is what the user asked: the
    /// text inputs joined by a space, each without the prompt-request context prefix; a message
    /// with no text is not told.
    fn on_user_message(state: &mut Self::State, thread: &ThreadId, text: &str, cx: &mut AppCx<'_>) {
        let _ = (state, thread, text, cx);
    }

    /// The account's limits of `backend` were read (`account/rateLimits/read`: the startup read, a
    /// refresh, a `/status` card's), after the app took the answer in. The rolling
    /// `account/rateLimits/updated` notifications are `project`'s; the answer of a read is on no
    /// stream, so this is how a feature learns what only a read tells. `read.codex` is the plan's
    /// own bucket.
    fn on_rate_limits_read(
        state: &mut Self::State,
        backend: BackendId,
        read: &RateRead,
        cx: &mut AppCx<'_>,
    ) {
        let _ = (state, backend, read, cx);
    }

    /// The words a feature says for the error of a failed turn of `thread` instead of the
    /// server's `message` (oracle `on_rate_limit_error`: a workspace owner is told what to do,
    /// not what the server said). Asked once per turn error that is the server's own words (not
    /// a policy refusal or an overload), after the features read the notification (`project`),
    /// in list order: the first that answers wins; none: the server's words stand.
    fn fault_words(
        state: &Self::State,
        thread: &ThreadId,
        message: &str,
        view: &AppView<'_>,
    ) -> Option<String> {
        let _ = (state, thread, message, view);
        None
    }

    /// The thread in use is now `thread` (the app showed it, whether it was held or just opened).
    fn on_shown(state: &mut Self::State, thread: &ThreadId, cx: &mut AppCx<'_>) {
        let _ = (state, thread, cx);
    }

    /// The connection to the server came up, was lost or is being closed by the host.
    fn on_connection(state: &mut Self::State, connection: Connection, cx: &mut AppCx<'_>) {
        let _ = (state, connection, cx);
    }

    /// The thread's running turn was interrupted (right after its `on_turn_end`).
    fn on_interrupted(part: &mut Self::ThreadPart, cx: &mut ThreadCx<'_, '_>) {
        let _ = (part, cx);
    }

    /// What held the thread's input back is over (a goal was set: `AppCx::settled`): what
    /// waits goes on. Runs over all features in list order, like the turn hooks, so a feature
    /// that waits for it does not name the one that said it.
    fn on_settled(part: &mut Self::ThreadPart, cx: &mut ThreadCx<'_, '_>) {
        let _ = (part, cx);
    }

    /// The first surface became interested in `interest` (`open`), or the last one lost interest
    /// (`!open`); also called with `open` for every interest still held after a reconnect.
    fn on_interest(
        state: &mut Self::State,
        interest: &Self::Interest,
        open: bool,
        cx: &mut AppCx<'_>,
    ) {
        let _ = (state, interest, open, cx);
    }

    /// When the feature wants to be woken, and with which of its own intents (ADR-012, "time as
    /// a declared wake"): the app reads this after every step, hands the nearest moment of all
    /// features to the shell (`AppFx::wake`), and when the clock (`AppEv::Clock`) has reached
    /// it delivers the intent through `intent`, as any other. The intent must change the state
    /// so that this answer is a later moment or `None`; a feature never reads a clock or starts a
    /// timer of its own. None: nothing to wait for.
    fn wake_at(state: &Self::State) -> Option<(Now, Self::Intent)> {
        let _ = state;
        None
    }

    /// The same for one thread's part: the intent is delivered to that thread under
    /// `thread_intent`, as any thread intent is: only where this process writes the thread (a
    /// thread it does not write asks for no moment).
    fn thread_wake_at(part: &Self::ThreadPart) -> Option<(Now, Self::ThreadIntent)> {
        let _ = part;
        None
    }

    /// The wire request of one of the feature's RPCs.
    fn wire(command: &Self::Command, thread: &ThreadId, id: RequestId) -> ClientRequest;

    /// Decodes the JSON result of one of the feature's RPCs.
    fn decode(command: &Self::Command, result: serde_json::Value) -> Result<Self::Reply, RpcError>;
}

/// Everything the list generates: the states, the root enums with their `From` impls and the
/// dispatch. Invoked once for the real list (below) and by the framework's own test with a
/// list of probes, in a module of its own.
macro_rules! feature_dirs {
    ($($name:ident => $variant:ident;)*) => {
        /// The app-scoped state of every feature: part of `CodexApp`.
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct FeatureStates {
            $(pub $name: crate::screens::Shared<<$name::$variant as Feature>::State>,)*
        }

        /// The per-thread part of every feature: part of each `ThreadModel`.
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct FeatureThreadParts {
            $(pub $name: <$name::$variant as Feature>::ThreadPart,)*
        }

        /// The intents of every feature: `Intent::Feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureIntent {
            $($variant(<$name::$variant as Feature>::Intent),)*
        }

        /// The thread intents of every feature: `ThreadEv::FeatureIntent`, and the one that is
        /// no feature's: what held a thread's input back is over (`AppCx::settled`).
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureThreadIntent {
            $($variant(<$name::$variant as Feature>::ThreadIntent),)*
            /// Every feature's `on_settled`, in list order.
            Settled,
        }

        /// The RPCs of every feature: `Command::Feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureCommand {
            $($variant(<$name::$variant as Feature>::Command),)*
        }

        /// The replies of every feature's RPCs: `Reply::feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureReply {
            $($variant(<$name::$variant as Feature>::Reply),)*
        }

        /// What every feature's RPCs are for: `ThreadCtx::Feature`, `AppCtx::Feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureCtx {
            $($variant(<$name::$variant as Feature>::Ctx),)*
        }

        /// The notes of every feature: `NoteKey::Feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureNote {
            $($variant(<$name::$variant as Feature>::Note),)*
        }

        /// The facts the host sends unasked to every feature: `AppEv::Feature`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureFact {
            $($variant(<$name::$variant as Feature>::Fact),)*
        }

        /// The wishes of every feature: `SurfaceWish` (`AppCx::wish`).
        #[derive(Clone, Debug, PartialEq, Eq)]
        #[allow(dead_code)]
        pub enum FeatureWish {
            $($variant(<$name::$variant as Feature>::Wish),)*
        }

        /// What a slash command that opens a feature's own sheet names (`Target::Feature`): one
        /// variant per feature, uninhabited for a feature without a sheet of its own.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum FeatureTarget {
            $($variant(<$name::$variant as Feature>::Target),)*
        }

        /// The interests of every feature: `Intent::Interest`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureInterest {
            $($variant(<$name::$variant as Feature>::Interest),)*
        }

        /// What every feature made of the server's stream: handed to `on_event`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum FeatureEvent {
            $($variant(<$name::$variant as Feature>::Event),)*
        }

        $(
        impl From<<$name::$variant as Feature>::Wish> for FeatureWish {
            fn from(wish: <$name::$variant as Feature>::Wish) -> Self { Self::$variant(wish) }
        }
        impl From<<$name::$variant as Feature>::Interest> for FeatureInterest {
            fn from(interest: <$name::$variant as Feature>::Interest) -> Self { Self::$variant(interest) }
        }
        impl From<<$name::$variant as Feature>::Intent> for FeatureIntent {
            fn from(intent: <$name::$variant as Feature>::Intent) -> Self { Self::$variant(intent) }
        }
        impl From<<$name::$variant as Feature>::ThreadIntent> for FeatureThreadIntent {
            fn from(intent: <$name::$variant as Feature>::ThreadIntent) -> Self { Self::$variant(intent) }
        }
        impl From<<$name::$variant as Feature>::Command> for FeatureCommand {
            fn from(command: <$name::$variant as Feature>::Command) -> Self { Self::$variant(command) }
        }
        impl From<<$name::$variant as Feature>::Reply> for FeatureReply {
            fn from(reply: <$name::$variant as Feature>::Reply) -> Self { Self::$variant(reply) }
        }
        impl From<<$name::$variant as Feature>::Ctx> for FeatureCtx {
            fn from(ctx: <$name::$variant as Feature>::Ctx) -> Self { Self::$variant(ctx) }
        }
        impl From<<$name::$variant as Feature>::Note> for FeatureNote {
            fn from(note: <$name::$variant as Feature>::Note) -> Self { Self::$variant(note) }
        }
        impl From<<$name::$variant as Feature>::Target> for FeatureTarget {
            fn from(target: <$name::$variant as Feature>::Target) -> Self { Self::$variant(target) }
        }
        impl From<<$name::$variant as Feature>::Fact> for FeatureFact {
            fn from(fact: <$name::$variant as Feature>::Fact) -> Self { Self::$variant(fact) }
        }
        impl From<<$name::$variant as Feature>::Event> for FeatureEvent {
            fn from(event: <$name::$variant as Feature>::Event) -> Self { Self::$variant(event) }
        }
        )*

        /// The fact a script's `fact` step makes: `value` is read by the feature's own `SEED`.
        #[allow(dead_code)]
        pub fn seed_fact(
            feature: &str,
            value: serde_json::Value,
        ) -> Result<FeatureFact, $crate::screens::SeedError> {
            $(if feature == <$name::$variant as Feature>::NAME {
                return match <$name::$variant as Feature>::SEED {
                    Some(seed) => seed(value).map(FeatureFact::$variant),
                    None => Err($crate::screens::SeedError::NoSeeds),
                };
            })*
            Err($crate::screens::SeedError::UnknownDir)
        }

        /// The note a script's `note` step makes: `value` is read by the feature's own `SEED_NOTE`.
        #[allow(dead_code)]
        pub fn seed_note(
            feature: &str,
            value: serde_json::Value,
        ) -> Result<FeatureNote, $crate::screens::SeedError> {
            $(if feature == <$name::$variant as Feature>::NAME {
                return match <$name::$variant as Feature>::SEED_NOTE {
                    Some(seed) => seed(value).map(FeatureNote::$variant),
                    None => Err($crate::screens::SeedError::NoSeeds),
                };
            })*
            Err($crate::screens::SeedError::UnknownDir)
        }

        #[allow(dead_code)]
        impl FeatureTarget {
            /// The directory of the feature whose sheet is named (`Target::dir`).
            pub fn dir(self) -> &'static str {
                match self {
                    $(Self::$variant(_) => <$name::$variant as Feature>::NAME,)*
                }
            }
        }

        /// The sheet a slash command opens, if some feature's is the one.
        #[allow(unused_variables, dead_code)]
        pub fn target_of(command: $crate::commands::SlashCommand) -> Option<FeatureTarget> {
            $(if let Some((opens, target)) = <$name::$variant as Feature>::OPENS {
                if opens == command {
                    return Some(FeatureTarget::$variant(target));
                }
            })*
            None
        }

        /// Whether some feature runs the slash command (`Feature::RUNS`).
        #[allow(unused_variables, dead_code)]
        pub fn runs_of(command: $crate::commands::SlashCommand) -> bool {
            false $(|| <$name::$variant as Feature>::RUNS.contains(&command))*
        }

        /// Additional command policy declared by the owning feature.
        #[allow(unused_variables, dead_code)]
        pub(crate) fn command_rule(
            command: $crate::commands::SlashCommand,
        ) -> Option<&'static $crate::commands::CommandRule> {
            $(if let Some(rule) = <$name::$variant as Feature>::COMMAND_RULES
                .iter().find(|rule| rule.command == command)
            {
                return Some(rule);
            })*
            None
        }

        /// Hands a slash command to the feature that runs it; false when none does (the app says
        /// the command is not available).
        #[allow(unused_variables, unreachable_code)]
        pub fn apply_slash(states: &mut FeatureStates, slash: Slash, cx: &mut AppCx<'_>) -> bool {
            $(if <$name::$variant as Feature>::RUNS.contains(&slash.command) {
                <$name::$variant as Feature>::slash(&mut states.$name, slash, cx);
                return true;
            })*
            false
        }

        /// The names of the features, in list (hook) order.
        #[allow(unused_variables)]
        pub fn names() -> Vec<&'static str> {
            vec![$(<$name::$variant as Feature>::NAME,)*]
        }

        /// Applies an intent to its feature.
        #[allow(unused_variables)]
        pub fn apply_intent(states: &mut FeatureStates, intent: FeatureIntent, cx: &mut AppCx<'_>) {
            match intent {
                $(FeatureIntent::$variant(intent) => {
                    <$name::$variant as Feature>::intent(&mut states.$name, intent, cx)
                })*
            }
        }

        /// The answer of a feature's app-scoped RPC.
        #[allow(unused_variables)]
        pub fn answered(
            states: &mut FeatureStates,
            ctx: FeatureCtx,
            result: Result<FeatureReply, RpcError>,
            cx: &mut AppCx<'_>,
        ) {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    let result = result.and_then(|reply| match reply {
                        FeatureReply::$variant(reply) => Ok(reply),
                        #[allow(unreachable_patterns)]
                        _ => Err(RpcError::Mismatch),
                    });
                    <$name::$variant as Feature>::answered(&mut states.$name, ctx, result, cx)
                })*
            }
        }

        /// The host's answer to an ask a feature made (`AppCx::effect`).
        #[allow(unused_variables)]
        pub fn effect_done(
            states: &mut FeatureStates,
            ctx: FeatureCtx,
            done: Done,
            cx: &mut AppCx<'_>,
        ) {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    <$name::$variant as Feature>::effect_done(&mut states.$name, ctx, done, cx)
                })*
            }
        }

        /// How the oracle shows a feature's note.
        #[allow(unused_variables)]
        pub fn note_tone(note: &FeatureNote) -> Tone {
            match *note {
                $(FeatureNote::$variant(ref note) => <$name::$variant as Feature>::note_tone(note),)*
            }
        }

        /// A fact the host sent unasked, to the feature it is for.
        #[allow(unused_variables)]
        pub fn on_fact(states: &mut FeatureStates, fact: FeatureFact, cx: &mut AppCx<'_>) {
            match fact {
                $(FeatureFact::$variant(fact) => {
                    <$name::$variant as Feature>::on_fact(&mut states.$name, fact, cx)
                })*
            }
        }

        /// A surface became interested in (`open`) or lost interest in (`!open`) what a feature
        /// shows.
        #[allow(unused_variables)]
        pub fn on_interest(
            states: &mut FeatureStates,
            interest: &FeatureInterest,
            open: bool,
            cx: &mut AppCx<'_>,
        ) {
            match *interest {
                $(FeatureInterest::$variant(ref interest) => {
                    <$name::$variant as Feature>::on_interest(&mut states.$name, interest, open, cx)
                })*
            }
        }

        /// What the features make of one notification of the server's stream, in list order.
        #[allow(unused_variables, unused_mut)]
        pub fn project(notification: &codex_app_server_protocol::ServerNotification) -> Vec<FeatureEvent> {
            let mut events = Vec::new();
            $(if let Some(event) = <$name::$variant as Feature>::project(notification) {
                events.push(FeatureEvent::$variant(event));
            })*
            events
        }

        /// What the features make of one server request, observe-only, in list order.
        #[allow(unused_variables, unused_mut)]
        pub fn project_request(request: &codex_app_server_protocol::ServerRequest) -> Vec<FeatureEvent> {
            let mut events = Vec::new();
            $(if let Some(event) = <$name::$variant as Feature>::project_request(request) {
                events.push(FeatureEvent::$variant(event));
            })*
            events
        }

        /// An event of the stream, to the feature that made it.
        #[allow(unused_variables)]
        pub fn on_event(states: &mut FeatureStates, event: FeatureEvent, cx: &mut AppCx<'_>) {
            match event {
                $(FeatureEvent::$variant(event) => {
                    <$name::$variant as Feature>::on_event(&mut states.$name, event, cx)
                })*
            }
        }

        /// A thread a feature asked for is open (or could not be made), to the feature that asked.
        #[allow(unused_variables)]
        pub fn thread_opened(
            states: &mut FeatureStates,
            ctx: FeatureCtx,
            result: Result<ThreadId, $crate::features::Refusal>,
            cx: &mut AppCx<'_>,
        ) {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    <$name::$variant as Feature>::thread_opened(&mut states.$name, ctx, result, cx)
                })*
            }
        }

        /// The answer of a structured turn, to the feature that asked.
        #[allow(unused_variables)]
        pub fn structured_done(
            states: &mut FeatureStates,
            ctx: FeatureCtx,
            result: Result<String, $crate::features::StructuredFailure>,
            cx: &mut AppCx<'_>,
        ) {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    <$name::$variant as Feature>::structured_done(&mut states.$name, ctx, result, cx)
                })*
            }
        }

        /// Whether the feature that asked for a structured turn still wants it (before its turn
        /// starts), read from the app as a view.
        #[allow(unused_variables)]
        pub fn structured_wanted(
            states: &FeatureStates,
            ctx: &FeatureCtx,
            view: &$crate::features::AppView<'_>,
        ) -> bool {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    <$name::$variant as Feature>::structured_wanted(&states.$name, ctx, view)
                })*
            }
        }

        /// A user message of the unnamed thread `thread` was completed: every feature's hook, in
        /// list order.
        #[allow(unused_variables)]
        pub fn on_user_message(
            states: &mut FeatureStates,
            thread: &ThreadId,
            text: &str,
            cx: &mut AppCx<'_>,
        ) {
            $(<$name::$variant as Feature>::on_user_message(&mut states.$name, thread, text, cx);)*
        }

        /// The words a feature says for a failed turn's error instead of the server's: the first
        /// in list order that has any.
        #[allow(unused_variables)]
        pub fn fault_words(
            states: &FeatureStates,
            thread: &ThreadId,
            message: &str,
            view: &$crate::features::AppView<'_>,
        ) -> Option<String> {
            $(if let Some(words) = <$name::$variant as Feature>::fault_words(&states.$name, thread, message, view) {
                return Some(words);
            })*
            None
        }

        /// The account's limits were read: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_rate_limits_read(
            states: &mut FeatureStates,
            backend: $crate::ids::BackendId,
            read: &$crate::screens::status::RateRead,
            cx: &mut AppCx<'_>,
        ) {
            $(<$name::$variant as Feature>::on_rate_limits_read(&mut states.$name, backend, read, cx);)*
        }

        /// The thread in use is `thread`: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_shown(states: &mut FeatureStates, thread: &ThreadId, cx: &mut AppCx<'_>) {
            $(<$name::$variant as Feature>::on_shown(&mut states.$name, thread, cx);)*
        }

        /// The connection changed: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_connection(states: &mut FeatureStates, connection: $crate::features::Connection, cx: &mut AppCx<'_>) {
            $(<$name::$variant as Feature>::on_connection(&mut states.$name, connection, cx);)*
        }

        /// The thread `thread` was resumed and is in use: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_resumed(states: &mut FeatureStates, thread: &ThreadId, cx: &mut AppCx<'_>) {
            $(<$name::$variant as Feature>::on_resumed(&mut states.$name, thread, cx);)*
        }

        /// The user interrupted the running turn of `thread`: every feature's hook, in list order.
        #[allow(unused_variables, dead_code)]
        pub fn on_interrupt(states: &mut FeatureStates, thread: &ThreadId, cx: &mut AppCx<'_>) {
            $(<$name::$variant as Feature>::on_interrupt(&mut states.$name, thread, cx);)*
        }

        /// The nearest moment any feature's `wake_at` asks for.
        pub fn next_wake(states: &FeatureStates) -> Option<$crate::app::Now> {
            let mut next: Option<$crate::app::Now> = None;
            $(
                if let Some((at, _)) = <$name::$variant as Feature>::wake_at(&states.$name) {
                    next = Some(next.map_or(at, |nearest| nearest.min(at)));
                }
            )*
            next
        }

        /// The intents the features asked to be woken with, whose moment `now` has reached, in
        /// list order.
        pub fn wakes_due(states: &FeatureStates, now: $crate::app::Now) -> Vec<FeatureIntent> {
            let mut due = Vec::new();
            $(
                if let Some((at, intent)) = <$name::$variant as Feature>::wake_at(&states.$name)
                    && now.reached(at)
                {
                    due.push(FeatureIntent::$variant(intent));
                }
            )*
            due
        }

        /// The nearest moment any feature's `thread_wake_at` asks for, in one thread's parts.
        pub fn next_thread_wake(parts: &FeatureThreadParts) -> Option<$crate::app::Now> {
            let mut next: Option<$crate::app::Now> = None;
            $(
                if let Some((at, _)) = <$name::$variant as Feature>::thread_wake_at(&parts.$name) {
                    next = Some(next.map_or(at, |nearest| nearest.min(at)));
                }
            )*
            next
        }

        /// The thread intents the features of one thread asked to be woken with, reached by
        /// `now`, in list order.
        pub fn thread_wakes_due(parts: &FeatureThreadParts, now: $crate::app::Now) -> Vec<FeatureThreadIntent> {
            let mut due = Vec::new();
            $(
                if let Some((at, intent)) = <$name::$variant as Feature>::thread_wake_at(&parts.$name)
                    && now.reached(at)
                {
                    due.push(FeatureThreadIntent::$variant(intent));
                }
            )*
            due
        }

        /// An intent for a thread's part of its feature.
        #[allow(unused_variables)]
        pub fn apply_thread_intent(
            parts: &mut FeatureThreadParts,
            intent: FeatureThreadIntent,
            cx: &mut ThreadCx<'_, '_>,
        ) {
            match intent {
                $(FeatureThreadIntent::$variant(intent) => {
                    let part = &mut parts.$name;
                    if let Some(intent) = <$name::$variant as Feature>::thread_fact(part, intent)
                        && cx.view.has_writer()
                    {
                        <$name::$variant as Feature>::thread_intent(part, intent, cx)
                    }
                })*
                // a fact: what waited for it goes on, as far as its effects are open
                FeatureThreadIntent::Settled => {
                    $(<$name::$variant as Feature>::on_settled(&mut parts.$name, cx);)*
                }
            }
        }

        /// The answer of a feature's RPC made on a thread (`ThreadCx::request`).
        #[allow(unused_variables)]
        pub fn apply_thread_reply(
            parts: &mut FeatureThreadParts,
            ctx: FeatureCtx,
            result: Result<FeatureReply, RpcError>,
            cx: &mut ThreadCx<'_, '_>,
        ) {
            match ctx {
                $(FeatureCtx::$variant(ctx) => {
                    let result = result.and_then(|reply| match reply {
                        FeatureReply::$variant(reply) => Ok(reply),
                        #[allow(unreachable_patterns)]
                        _ => Err(RpcError::Mismatch),
                    });
                    <$name::$variant as Feature>::thread_answered(&mut parts.$name, ctx, result, cx)
                })*
            }
        }

        /// A turn of the thread ended: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_turn_end(parts: &mut FeatureThreadParts, end: TurnEnd, cx: &mut ThreadCx<'_, '_>) {
            $(<$name::$variant as Feature>::on_turn_end(&mut parts.$name, end, cx);)*
        }

        /// Deliver a live fact to thread features, with no effect door.
        #[allow(unused_variables, dead_code)]
        pub fn on_live_thread_event(
            parts: &mut FeatureThreadParts,
            event: $crate::threads::LiveThreadEvent<'_>,
        ) {
            $(<$name::$variant as Feature>::on_live_thread_event(&mut parts.$name, event);)*
        }

        /// The thread's turn was interrupted: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_interrupted(parts: &mut FeatureThreadParts, cx: &mut ThreadCx<'_, '_>) {
            $(<$name::$variant as Feature>::on_interrupted(&mut parts.$name, cx);)*
        }

        /// The wire request of a feature's RPC.
        #[allow(unused_variables)]
        pub fn wire(command: &FeatureCommand, thread: &ThreadId, id: RequestId) -> ClientRequest {
            match *command {
                $(FeatureCommand::$variant(ref command) => {
                    <$name::$variant as Feature>::wire(command, thread, id)
                })*
            }
        }

        /// What the server's refusal of a feature's RPC means to the feature, when it is an answer.
        #[allow(unused_variables, dead_code)]
        pub fn refusal(command: &FeatureCommand, error: &$crate::rpc::ServerError) -> Option<FeatureReply> {
            match *command {
                $(FeatureCommand::$variant(ref command) => {
                    <$name::$variant as Feature>::REFUSAL
                        .and_then(|refusal| refusal(command, error))
                        .map(FeatureReply::$variant)
                })*
            }
        }

        /// Decodes the JSON result of a feature's RPC.
        #[allow(unused_variables)]
        pub fn decode(
            command: &FeatureCommand,
            result: serde_json::Value,
        ) -> Result<FeatureReply, RpcError> {
            match *command {
                $(FeatureCommand::$variant(ref command) => {
                    <$name::$variant as Feature>::decode(command, result).map(FeatureReply::$variant)
                })*
            }
        }
    };
}

feature_list!(feature_dirs);

/// Whether a registered feature claims a sheet, verb or dispatch policy for the command.
pub fn owns_command(command: SlashCommand) -> bool {
    runs_of(command) || target_of(command).is_some() || command_rule(command).is_some()
}

/// The intent a permissions pick waits for before it is applied, when a feature needs a setup first
/// (the Windows sandbox of an Auto pick: its enable prompt, which lets the pick go when the setup
/// is done); none: the pick is applied at once. The app asks through the list, like every
/// selector (ADR-012, "selectors").
pub(crate) fn setup_before_pick(
    states: &FeatureStates,
    selection: &crate::screens::select::permissions::Selection,
) -> Option<FeatureIntent> {
    states
        .windows_sandbox
        .pick_needs_setup(selection)
        .then(|| FeatureIntent::from(windows_sandbox::Intent::Prompt(selection.clone())))
}

pub(crate) fn setup_before_owned_pick(
    states: &FeatureStates,
    token: &crate::threads::queue::ModalToken<crate::screens::select::permissions::Selection>,
    selection: &crate::screens::select::permissions::Selection,
) -> Option<FeatureIntent> {
    states.windows_sandbox.pick_needs_setup(selection).then(|| {
        windows_sandbox::Intent::PromptOwned(windows_sandbox::OwnedPrompt(token.clone())).into()
    })
}

pub(crate) fn start_owned_permission_setup(
    states: &mut FeatureStates,
    token: &crate::threads::queue::ModalToken<crate::screens::select::permissions::Selection>,
    mode: codex_app_server_protocol::WindowsSandboxSetupMode,
    cx: &mut AppCx<'_>,
) {
    states.windows_sandbox.start_owned(token, mode, cx);
}

pub(crate) fn clear_owned_permission_setup(
    states: &mut FeatureStates,
    token: &crate::threads::queue::ModalToken<crate::screens::select::permissions::Selection>,
) {
    states.windows_sandbox.clear_owned(token);
}

/// The Daybreak choice of a thread (`/daybreak`'s saved choice, else its metadata's): the one
/// selector of a slice the app reads (it words a cyber refusal, `app/notices.rs`), through the
/// list like every feature name (ADR-012, "selectors").
pub fn daybreak_of(thread: ThreadView<'_>) -> Option<bool> {
    small_cmds::daybreak_of(thread)
}

// The app owns turn completion and account boundaries. The slice owns the offer and its
// eligibility; expose its operations through the feature list.
pub(crate) use rate_limit_prompt::{
    Failure as RateFailure, Remedy as RateRemedy, banner_shows as rate_banner_shows,
    change_remedy as change_rate_remedy, dismiss_nudge as dismiss_rate_nudge,
    failed as rate_failed, failure_of as rate_failure_of, forget_remedy as forget_rate_remedy,
    nudge_origin as rate_nudge_origin, origin_available as rate_offer_origin_available,
    reserve as reserve_completion_prompt, retire as retire_rate_offer,
    revalidate as revalidate_rate_offer, rolling as rolling_rate_limits,
};

/// Applies the value returned by a dialog whose hold the app has verified.
pub(crate) fn apply_modal_choice(
    states: &mut FeatureStates,
    thread: &ThreadId,
    receipt: &mut ModalReceipt,
    cx: &mut AppCx<'_>,
) {
    match receipt {
        ModalReceipt::Copy(_, choice) => {
            if let Some(choice) = choice.take() {
                copy::choose(thread.clone(), choice, cx);
            }
        }
        ModalReceipt::StatusLine(_, choice) => {
            if let Some(choice) = choice.take() {
                states.settings.save_status_line(choice, cx);
            }
        }
        ModalReceipt::Permissions(..) | ModalReceipt::Apps(..) | ModalReceipt::Preset(..) => {}
    }
}

pub(crate) fn remember_copy_status(parts: &mut FeatureThreadParts, key: crate::threads::ItemKey) {
    parts.copy.remember_status(key);
}

pub(crate) fn clear_copy_status(parts: &mut FeatureThreadParts) {
    parts.copy.clear_status();
}

pub(crate) use images::retire as retire_image_snapshots;
pub(crate) use small_cmds::{ide_opted_in, ide_permit, settle_ide_submit};

pub(crate) fn has_copy_status(parts: &FeatureThreadParts) -> bool {
    parts.copy.has_status()
}

pub(crate) fn has_visible_copy_response(text: &str, cwd: &str) -> bool {
    copy::copy_markdown(text, cwd).is_some()
}

/// A later command in this thread takes precedence over its previous Status card.
pub fn supersede_copy_status(thread: crate::ids::ThreadId) -> FeatureIntent {
    copy::Intent::StatusSuperseded { thread }.into()
}

#[cfg(test)]
mod framework;
