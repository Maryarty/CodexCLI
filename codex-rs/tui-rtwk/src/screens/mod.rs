//! The screen slots. One entry per directory below, in `screen_dirs!`: the
//! macro makes the module, the model part, the root `ScreenEv` and
//! `ScreenIntent` variants, their dispatch and the registry the debug menu
//! reads. A slice edits only its own directory.

use rtwk::AnyWidget;

use crate::app::AppFx;

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
    Route(fn() -> AnyWidget),
    /// Shown with `show_dialog` over the menu.
    Dialog(fn() -> AnyWidget),
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
    pub title: &'static str,
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
                title: $dir::TITLE,
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
