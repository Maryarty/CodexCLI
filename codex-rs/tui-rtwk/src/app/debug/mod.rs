//! Debug-only screens: the `--debug-screens` menu and the seeded fixture screen.

pub mod menu;
pub mod seeded;

use codex_tui_core::screens::Kind;
use rtwk::AnyWidget;

use crate::app::debug::seeded::Seeded;
use crate::l10n::Lang;

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

/// A registered slot.
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

/// Makes the registry entry of core's slot `$core` from the view module
/// `$view` (its `strings::title`, `debug_entries`, `open_empty`).
macro_rules! entry {
    ($core:ident, $($view:tt)+) => {
        Dir {
            name: codex_tui_core::screens::$core::NAME,
            id: codex_tui_core::screens::$core::ID,
            kind: codex_tui_core::screens::$core::KIND,
            title: $($view)+::strings::title,
            debug_entries: $($view)+::debug_entries,
            open_empty: $($view)+::open_empty,
        }
    };
}

/// Every slot the debug menu lists, in screen-map order: a hand-written list
/// (a feature may feed a page, a sheet or a cell; core's `screen_list!` is the
/// model side only).
pub fn dirs() -> Vec<Dir> {
    use crate::pages::session;
    use crate::pages::startup;
    use crate::{pages, sheets};
    let mut dirs = vec![
        entry!(chat, session),
        entry!(cells, session::cells),
        entry!(composer, session::composer),
        entry!(popovers, session::popovers),
        entry!(transcript, session::transcript),
        entry!(select, sheets::select),
        entry!(approvals, sheets::approvals),
        entry!(backtrack, session::backtrack),
        entry!(resume, pages::resume),
        entry!(status, session::cells::status),
        entry!(usage, pages::usage),
        entry!(agents, pages::agents),
        entry!(dialogs, sheets::dialogs),
        entry!(pets, sheets::pets),
        entry!(voice, sheets::voice),
        entry!(onboarding, startup::onboarding),
        entry!(startup, startup::prompts),
        entry!(import, startup::import),
    ];
    // The features' slots (`FeatureView::debug_dir`), after the screens'.
    dirs.extend(crate::features::debug_dirs());
    dirs
}
