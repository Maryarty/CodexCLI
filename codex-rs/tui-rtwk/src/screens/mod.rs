//! The screen slots. One entry per directory below, in `screen_dirs!`: the
//! macro makes the module, the model part, the root `ScreenEv` and
//! `ScreenIntent` variants, their dispatch and the registry the debug menu
//! reads. A slice edits only its own directory.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use rtwk::AnyWidget;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::l10n::Lang;
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

/// How the debug menu opens an entry.
#[derive(Clone, Copy)]
pub enum DebugOpen {
    /// Pushed as a route over the menu.
    Route(fn(Lang) -> AnyWidget),
    /// Shown with `show_dialog` over the menu.
    Dialog(fn(Lang) -> AnyWidget),
}

/// One state of one screen on made-up data, listed by the debug menu.
#[derive(Clone, Copy)]
pub struct DebugEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub open: DebugOpen,
}

/// A registered directory.
#[derive(Clone, Copy)]
pub struct Dir {
    pub name: &'static str,
    pub id: &'static str,
    pub kind: Kind,
    /// The title in a language (from the directory's string table).
    pub title: fn(Lang) -> &'static str,
    pub debug_entries: fn() -> Vec<DebugEntry>,
    pub open_empty: fn() -> DebugOpen,
}

macro_rules! screen_dirs {
    ($($dir:ident => $variant:ident, $id:literal, $kind:ident;)*) => {
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
            result: Result<ScreenReply, String>,
            fx: &mut ThreadFx<'_>,
        ) {
            match ctx {
                $(ScreenCtx::$variant(ctx) => {
                    let result = result.and_then(|reply| match reply {
                        ScreenReply::$variant(reply) => Ok(reply),
                        _ => Err("a reply of another screen".to_string()),
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
        pub fn decode(command: &ScreenCommand, result: serde_json::Value) -> Result<ScreenReply, String> {
            match command {
                $(ScreenCommand::$variant(command) => $dir::decode(command, result).map(ScreenReply::$variant),)*
            }
        }

        /// The events of every screen, one variant per directory.
        #[derive(Clone, Debug)]
        pub enum ScreenEv {
            $($variant($dir::Ev),)*
        }

        /// The intents of every screen, one variant per directory.
        #[derive(Clone, Debug)]
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

        /// Every directory, in screen-map order.
        pub fn dirs() -> Vec<Dir> {
            vec![$(Dir {
                name: $dir::NAME,
                id: $dir::ID,
                kind: $dir::KIND,
                title: $dir::strings::title,
                debug_entries: $dir::debug_entries,
                open_empty: $dir::open_empty,
            },)*]
        }
    };
}

screen_dirs! {
    chat => Chat, "S10", Home;
    cells => Cells, "S11", Part;
    composer => Composer, "S10", Part;
    popovers => Popovers, "S12", Part;
    transcript => Transcript, "S10", Part;
    select => Select, "S14", Dialog;
    approvals => Approvals, "S13", Dialog;
    settings => Settings, "S15", Dialog;
    pager => Pager, "S16", Route;
    backtrack => Backtrack, "S17", Part;
    resume => Resume, "S18", Route;
    status => Status, "S19", Dialog;
    usage => Usage, "S20", Route;
    agents => Agents, "S21", Route;
    sidebar => Sidebar, "S22", Part;
    dialogs => Dialogs, "S23", Dialog;
    pets => Pets, "S24", Part;
    voice => Voice, "S25", Part;
    ui_tree => UiTree, "S26", Part;
    onboarding => Onboarding, "S02-S04", Route;
    startup => Startup, "S05", Dialog;
    import => Import, "S06", Route;
}
