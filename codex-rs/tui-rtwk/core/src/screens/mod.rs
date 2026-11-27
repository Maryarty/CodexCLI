//! The core half of the screen slots. One entry per screen directory in
//! `screen_list!`: `core_dirs!` (here) makes the module `screens/<dir>.rs`,
//! the model part, the root `ScreenEv`, `ScreenIntent` and the RPC enums with
//! their dispatch. The UI crate invokes the same `screen_list!` with its own
//! macro for the view half (`src/screens/<dir>/`), so the two lists cannot
//! differ. A slice edits only its own two directories.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::ThreadFx;

/// Screen intents that have an explicit thread target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenThreadIntent {
    Chat(chat::ThreadIntent),
    Backtrack(backtrack::ThreadIntent),
    Voice(voice::ThreadIntent),
}

pub(crate) fn apply_thread_intent(
    slots: &mut ThreadSlots,
    intent: ScreenThreadIntent,
    busy: bool,
    fx: &mut ThreadFx<'_>,
) {
    match intent {
        ScreenThreadIntent::Chat(intent) => {
            chat::apply_thread_intent(&mut slots.chat, intent, busy, fx)
        }
        ScreenThreadIntent::Backtrack(intent) => {
            backtrack::apply_thread_intent(&mut slots.backtrack, intent, fx)
        }
        ScreenThreadIntent::Voice(intent) => {
            voice::apply_intent_on_thread(&mut slots.voice, intent, fx)
        }
    }
}

/// Protocol projection for thread-scoped screen slots.
pub(crate) fn project_thread(
    notification: &codex_app_server_protocol::ServerNotification,
) -> Option<(ThreadId, ScreenThreadEv)> {
    chat::project(notification)
        .map(|(thread, ev)| (thread, ScreenThreadEv::Chat(ev)))
        .or_else(|| {
            status::project(notification).map(|(thread, ev)| (thread, ScreenThreadEv::Status(ev)))
        })
}

/// Where a slot shows up in the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The `Navigator`'s home.
    Home,
    /// A `PageRoute` pushed by the widget that opens it.
    Route,
    /// A modal sheet (`show_dialog` with the shared `sheet_options`) over the
    /// page that opens it.
    Sheet,
    /// A small centred `show_dialog` from the widget that asks.
    Dialog,
    /// A part of the chat screen's subtree.
    Part,
}

/// The screen map's facts about one directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirInfo {
    pub name: &'static str,
    pub id: &'static str,
    pub kind: Kind,
}

/// Why a script's `screen` step could not make an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedError {
    /// No screen directory of that name.
    UnknownDir,
    /// The directory has no seedable event (its `seed` is the slice's to
    /// write, in its own file).
    NoSeeds,
    /// The JSON is not one of the directory's seed events; serde's message.
    Bad(String),
}

/// The list of the screen directories, in screen-map order, handed to
/// `callback` as `dir => Variant;` entries. The UI crate calls it with its
/// own macro.
#[macro_export]
macro_rules! screen_list {
    ($callback:ident) => {
        $callback! {
        chat => Chat;
        cells => Cells;
        composer => Composer;
        popovers => Popovers;
        transcript => Transcript;
        select => Select;
        approvals => Approvals;
        backtrack => Backtrack;
        resume => Resume;
        status => Status;
        usage => Usage;
        agents => Agents;
        dialogs => Dialogs;
        pets => Pets;
        voice => Voice;
        onboarding => Onboarding;
        startup => Startup;
        import => Import;
        }
    };
}

/// A screen model kept in its own `Rc`: the app's `ScreenStates` is cloned
/// whenever any screen changes (the root provider holds the previous one), and
/// a model with a long history inside it would be copied each time. Cloning
/// this is a count; a write (`DerefMut`) copies the model only while another
/// holder has it, and comparing two takes the pointer first. A reader that
/// selects it is woken only when this model changed (VS1 review P1, P6).
#[derive(Debug, Default)]
pub struct Shared<T>(std::rc::Rc<T>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Shared<T> {
    /// Both are the same model, not equal copies.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> std::ops::Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> std::ops::DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        std::rc::Rc::make_mut(&mut self.0)
    }
}

impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other) || *self.0 == *other.0
    }
}

impl<T: Eq> Eq for Shared<T> {}

/// The type a directory's model has inside `ScreenStates`.
macro_rules! slot {
    (agents) => {
        Shared<agents::Model>
    };
    (resume) => {
        Shared<resume::Model>
    };
    ($dir:ident) => {
        $dir::Model
    };
}

macro_rules! core_dirs {
    ($($dir:ident => $variant:ident;)*) => {
        $(pub mod $dir;)*

        /// The app-scoped model parts of every screen.
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct ScreenStates {
            $(pub $dir: slot!($dir),)*
        }

        /// The per-thread parts of every screen: part of each `ThreadModel`.
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct ThreadSlots {
            $(pub $dir: $dir::ThreadPart,)*
        }

        /// The thread events of every screen: `ThreadEv::Screen`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenThreadEv {
            $($variant($dir::ThreadEv),)*
        }

        /// The RPCs of every screen: `Command::Screen`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenCommand {
            $($variant($dir::Command),)*
        }

        /// The replies of every screen's RPCs: `Reply::screen`.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenReply {
            $($variant($dir::Reply),)*
        }

        /// What every screen's outgoing RPCs are for.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenCtx {
            $($variant($dir::Ctx),)*
        }

        $(
        impl From<$dir::ThreadEv> for ScreenThreadEv {
            fn from(ev: $dir::ThreadEv) -> Self { Self::$variant(ev) }
        }
        impl From<$dir::Command> for ScreenCommand {
            fn from(command: $dir::Command) -> Self { Self::$variant(command) }
        }
        impl From<$dir::Reply> for ScreenReply {
            fn from(reply: $dir::Reply) -> Self { Self::$variant(reply) }
        }
        impl From<$dir::Ctx> for ScreenCtx {
            fn from(ctx: $dir::Ctx) -> Self { Self::$variant(ctx) }
        }
        impl From<$dir::Ev> for ScreenEv {
            fn from(ev: $dir::Ev) -> Self { Self::$variant(ev) }
        }
        impl From<$dir::Intent> for ScreenIntent {
            fn from(intent: $dir::Intent) -> Self { Self::$variant(intent) }
        }
        )*

        /// Applies a thread event to its screen's part of the thread.
        pub fn apply_thread_ev(slots: &mut ThreadSlots, ev: ScreenThreadEv, fx: &mut ThreadFx<'_>) {
            match ev {
                $(ScreenThreadEv::$variant(ev) => $dir::apply_thread(&mut slots.$dir, ev, fx),)*
            }
        }

        /// Applies the reply of a screen's RPC to its part of the thread.
        pub fn apply_reply(
            slots: &mut ThreadSlots,
            ctx: ScreenCtx,
            result: Result<ScreenReply, RpcError>,
            fx: &mut ThreadFx<'_>,
        ) {
            match ctx {
                $(ScreenCtx::$variant(ctx) => {
                    let result = result.and_then(|reply| match reply {
                        ScreenReply::$variant(reply) => Ok(reply),
                        _ => Err(RpcError::Mismatch),
                    });
                    $dir::apply_reply(&mut slots.$dir, ctx, result, fx)
                })*
            }
        }

        /// The wire request of a screen's RPC.
        pub fn wire(command: &ScreenCommand, thread: &ThreadId, id: RequestId) -> ClientRequest {
            match command {
                $(ScreenCommand::$variant(command) => $dir::wire(command, thread, id),)*
            }
        }

        /// Decodes the JSON result of a screen's RPC.
        pub fn decode(command: &ScreenCommand, result: serde_json::Value) -> Result<ScreenReply, RpcError> {
            match command {
                $(ScreenCommand::$variant(command) => $dir::decode(command, result).map(ScreenReply::$variant),)*
            }
        }

        /// The events of every screen, one variant per directory.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenEv {
            $($variant($dir::Ev),)*
        }

        /// The intents of every screen, one variant per directory.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum ScreenIntent {
            $($variant($dir::Intent),)*
        }

        /// Applies a screen event to its model part.
        pub fn apply_ev(states: &mut ScreenStates, ev: ScreenEv, fx: &mut AppFx) {
            match ev {
                $(ScreenEv::$variant(ev) => $dir::apply(&mut states.$dir, ev, fx),)*
            }
        }

        /// Applies a screen intent to its model part.
        pub fn apply_intent(states: &mut ScreenStates, intent: ScreenIntent, fx: &mut AppFx) {
            match intent {
                $(ScreenIntent::$variant(intent) => $dir::apply_intent(&mut states.$dir, intent, fx),)*
            }
        }

        /// The event a script's `screen` step makes: `value` is read by the
        /// directory's own `seed`, so a slice seeds its events without editing
        /// the shared loader.
        pub fn seed_ev(dir: &str, value: serde_json::Value) -> Result<ScreenEv, SeedError> {
            match dir {
                $($dir::NAME => $dir::seed(value).map(ScreenEv::$variant),)*
                _ => Err(SeedError::UnknownDir),
            }
        }

        /// Every directory, in screen-map order.
        pub fn dirs() -> Vec<DirInfo> {
            vec![$(DirInfo {
                name: $dir::NAME,
                id: $dir::ID,
                kind: $dir::KIND,
            },)*]
        }
    };
}

pub mod tips;

screen_list!(core_dirs);

impl ScreenStates {
    /// The clock the app was told (`AppEv::Clock`), for the screens whose views word a time (the
    /// command center's and the resume picker's ages, a `/status` card's limits, the headline's
    /// refresh): none of them has an event for it, the app is the one place the time is kept
    /// (`CodexApp::now`). A model is written only when its reading changes, so a screen that
    /// is shared by a view is not copied for a clock that did not move it.
    pub(crate) fn set_now(&mut self, now: crate::app::Now) {
        let secs = now.secs();
        if self.agents.now != secs {
            self.agents.now = secs;
        }
        if self.resume.clock != secs {
            self.resume.clock = secs;
        }
        if self.status.now != secs || self.status.utc_offset_secs != now.utc_offset_secs() {
            self.status.now = secs;
            self.status.utc_offset_secs = now.utc_offset_secs();
        }
        let secs = u64::try_from(secs).unwrap_or(0);
        if self.chat.facts.now() != secs {
            chat::set_now(&mut self.chat, secs);
        }
    }
}
