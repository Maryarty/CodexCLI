//! `dialogs`: the view half of S23 (the small dialogs): the shortcut overlay,
//! and the warnings viewer. They share one host widget that
//! reads which dialog the model holds (`core::screens::dialogs::Open`) through
//! a provider projection and returns what the user does as intents.
//! Oracle: `bottom_pane/shortcut_overlay.rs`, `warnings_view.rs`,
//! (`/feedback` is `features/feedback`.)

mod dialog;
mod prompt_entries;
mod shortcuts;
mod warnings;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogResult, show_dialog};

pub use dialog::Page;

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;

pub mod strings;

/// The shortcut overlay's body for a bottom pane, and its close hint (the
/// footer's line while it is open). Oracle: `shortcut_overlay.rs`.
///
/// `running` is the turn: `Queue message` and `Interrupt` for `Send message`
/// and `Quit`. The panel is a `Flexible` that outweighs the reader's gap
/// beside it (flex 1, `transcript::follow::gap`): it takes the rows it needs
/// before the gap's one.
pub fn shortcut_panel<S: 'static>(cx: &mut Cx<'_, S>, running: bool) -> (AnyWidget, &'static str) {
    let tokens = crate::app::theme::Tokens::of(cx);
    let s = strings::of(cx);
    let keymap = crate::keymap::keymap(cx);
    let sections = shortcuts::resolve(s, &keymap, running);
    (shortcuts::panel(s, &tokens, sections), s.shortcuts_close)
}

/// What the debug menu lists for this directory: one entry per dialog worth
/// opening, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries = [
        (
            "shortcut_overlay",
            "the shortcut overlay, list page",
            include_str!("fixtures/shortcut_overlay.jsonl"),
        ),
        (
            "warnings",
            "the warnings viewer, two warnings",
            include_str!("fixtures/warnings.jsonl"),
        ),
        (
            "warnings_long",
            "the warnings viewer, a long text",
            include_str!("fixtures/warnings_long.jsonl"),
        ),
    ];
    let mut all: Vec<DebugEntry> = entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(match name {
                "shortcut_overlay" => |_| dialog::SmallDialog::of(Page::Shortcuts).into(),
                "warnings" | "warnings_long" => |_| dialog::SmallDialog::of(Page::Warnings).into(),
                _ => |_| dialog::SmallDialog::of(Page::Shortcuts).into(),
            }),
        })
        .collect();
    all.extend(prompt_entries::entries());
    all
}

/// Opens the dialog of `page` from the widget that asks (`show_dialog`); what the user decides
/// returns as intents.
pub fn open_page<S>(cx: &mut Cx<'_, S>, page: Page) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| dialog::SmallDialog::of(page).into(),
        crate::sheets::sheet_options(),
    )
}

/// F2 and `/warnings`: the viewer over the warnings the session retains (with none it shows
/// its empty page, `No warnings`).
pub fn open_warnings<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    open_page(cx, Page::Warnings)
}

/// Opens the dialogs with no seeded model (debug menu): nothing to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
