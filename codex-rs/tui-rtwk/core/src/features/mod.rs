//! Core feature slices (ADR-012). A feature is a flow that is not a screen: a slash command
//! that opens a menu or a prompt, asks the server, says what happened in the transcript
//! (`/goal`, `/export`, `/rename`, `/plugins`), or thread state that reacts to the turn
//! lifecycle (the queue). Its code lives in `core/src/features/<f>/` and its words and views in
//! `src/features/<f>/`; the list in `feature_list!` registers it once and generates the root
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
//! The order of the list is the order of every hook that runs over all features
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

use crate::app::effects::Done;
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::threads::{Tone, TurnEnd};

mod cx;
mod remote;
mod surface;

pub use cx::{AppCx, AppView, Out, ThreadCx, ThreadView};
pub use remote::Remote;
pub use surface::{InterestStep, Interests, Scope, SurfaceWish, Wishes};

// One line per feature of `feature_list!`, written out so that rustfmt (which follows `mod` lines,
// not macro expansions) formats the slices too. A feature of the list without its module does not
// compile: `feature_dirs!` names `$name::$variant`.
pub mod copy;
pub mod daemon;
pub mod diff;
pub mod export;
pub mod external_editor;
pub mod feedback;
pub mod goal;
pub mod mcp_inventory;
pub mod plugins;
pub mod processes;
pub mod recap;
pub mod rename;
pub mod security_setup;
pub mod settings;
pub mod side;
pub mod small_cmds;
pub mod windows_sandbox;

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

/// How a script's `fact` step reads one fact of a feature from its JSON (`Feature::SEED`).
pub type SeedFn<T> = fn(serde_json::Value) -> Result<T, SeedError>;

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

    /// The slash command that opens the feature's own sheet and what it names
    /// (`Target::Feature`; the generated `FeatureTarget` and `target_of`). None: no command opens
    /// a sheet of the feature (`/goal`, `/rename` run a verb), and `Target` is `Never<Self>`.
    const OPENS: Option<(SlashCommand, Self::Target)> = None;

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

    /// An intent of the feature.
    fn intent(state: &mut Self::State, intent: Self::Intent, cx: &mut AppCx<'_>) {
        let _ = (state, intent, cx);
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

    /// A turn of the thread ended (completed, failed or interrupted): what waited for it
    /// goes on.
    fn on_turn_end(part: &mut Self::ThreadPart, end: TurnEnd, cx: &mut ThreadCx<'_, '_>) {
        let _ = (part, end, cx);
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

    /// The wire request of one of the feature's RPCs.
    fn wire(command: &Self::Command, thread: &ThreadId, id: RequestId) -> ClientRequest;

    /// Decodes the JSON result of one of the feature's RPCs.
    fn decode(command: &Self::Command, result: serde_json::Value) -> Result<Self::Reply, RpcError>;
}

/// The list of the features, in hook order, handed to `callback` as `name => Type;` entries:
/// `name` is the module `core/src/features/<name>/`, `Type` the marker type in it that
/// implements `Feature`. The UI crate calls it with its own macro for the view half
/// (`src/features/<name>/`), so the two lists cannot differ. A slice is one line here.
#[macro_export]
macro_rules! feature_list {
    ($callback:ident) => {
        $callback! {
            goal => Goal;
            rename => Rename;
            export => Export;
            feedback => Feedback;
            plugins => Plugins;
            settings => Settings;
            processes => Processes;
            mcp_inventory => McpInventory;
            diff => Diff;
            daemon => Daemon;
            security_setup => SecuritySetup;
            windows_sandbox => WindowsSandbox;
            copy => Copier;
            small_cmds => SmallCmds;
            recap => Recap;
            external_editor => ExternalEditor;
            side => Side;
        }
    };
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

        /// The thread `thread` was resumed and is in use: every feature's hook, in list order.
        #[allow(unused_variables)]
        pub fn on_resumed(states: &mut FeatureStates, thread: &ThreadId, cx: &mut AppCx<'_>) {
            $(<$name::$variant as Feature>::on_resumed(&mut states.$name, thread, cx);)*
        }

        /// An intent for a thread's part of its feature.
        #[allow(unused_variables)]
        pub fn apply_thread_intent(
            parts: &mut FeatureThreadParts,
            intent: FeatureThreadIntent,
            cx: &mut ThreadCx<'_, '_>,
        ) {
            match intent {
                // an ask of one feature: only where this process writes the thread
                $(FeatureThreadIntent::$variant(intent) => {
                    if cx.view.has_writer() {
                        <$name::$variant as Feature>::thread_intent(&mut parts.$name, intent, cx)
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

#[cfg(test)]
mod framework;
