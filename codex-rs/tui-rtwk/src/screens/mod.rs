//! The view half of the screen slots. The list of directories is core's
//! `screen_list!` (the same one that makes the model half), so a directory is
//! registered once; `view_dirs!` makes the modules `screens/<dir>/` and the
//! registry the debug menu reads. A directory holds view files only: widgets,
//! `strings.rs`, debug entries. Its model, events, intents and RPCs are
//! `codex_tui_core::screens::<dir>`. A slice edits only its own two
//! directories.

use codex_tui_core::screens::Kind;
use rtwk::AnyWidget;

use crate::l10n::Lang;
use crate::ui::seeded::Seeded;

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
    /// The JSONL event script (core's `fixtures::script`) that makes the
    /// model state the entry shows; `include_str!` of a file under the
    /// directory's `fixtures/`.
    pub script: &'static str,
    pub open: EntryOpen,
}

/// How an entry shows its seeded model: the builder makes the screen, the
/// menu provides the model above it (`Seeded::scope`).
#[derive(Clone, Copy)]
pub enum EntryOpen {
    /// Pushed as a route over the menu.
    Route(fn(&Seeded<'_>) -> AnyWidget),
    /// Shown with `show_dialog` over the menu; the builder returns the whole
    /// dialog.
    Dialog(fn(&Seeded<'_>) -> AnyWidget),
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

macro_rules! view_dirs {
    ($($dir:ident => $variant:ident;)*) => {
        $(pub mod $dir;)*

        /// Every directory, in screen-map order.
        pub fn dirs() -> Vec<Dir> {
            vec![$(Dir {
                name: codex_tui_core::screens::$dir::NAME,
                id: codex_tui_core::screens::$dir::ID,
                kind: codex_tui_core::screens::$dir::KIND,
                title: $dir::strings::title,
                debug_entries: $dir::debug_entries,
                open_empty: $dir::open_empty,
            },)*]
        }
    };
}

codex_tui_core::screen_list!(view_dirs);
