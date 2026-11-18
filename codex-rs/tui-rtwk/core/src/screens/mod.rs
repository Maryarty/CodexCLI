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

/// Where a slot shows up in the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The `Navigator`'s home.
    Home,
    /// A `PageRoute` pushed by the widget that opens it.
    Route,
    /// A `show_dialog` from the widget that asks.
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
        settings => Settings;
        pager => Pager;
        backtrack => Backtrack;
        resume => Resume;
        status => Status;
        usage => Usage;
        agents => Agents;
        sidebar => Sidebar;
        dialogs => Dialogs;
        pets => Pets;
        voice => Voice;
        ui_tree => UiTree;
        onboarding => Onboarding;
        startup => Startup;
        import => Import;
        }
    };
}

macro_rules! core_dirs {
    ($($dir:ident => $variant:ident;)*) => {
        $(pub mod $dir;)*

        /// The app-scoped model parts of every screen.
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct ScreenStates {
            $(pub $dir: $dir::Model,)*
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

screen_list!(core_dirs);
