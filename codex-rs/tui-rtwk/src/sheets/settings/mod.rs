//! `settings`: the view half of S15 (settings dialogs): modal sheets over the
//! session page, each drawn without a frame at the bottom edge, as the
//! oracle draws its bottom-pane views (architecture §1, amendment
//! 2026-11-19b). `Screen` is the one widget the app opens; which dialog it
//! shows is what the model holds open (`core::screens::settings::Open`).
//! Their model part, events, intents and RPCs are core's
//! `codex_tui_core::screens::settings`.

use rtwk::overlay::{Dialog, DialogOptions, DialogResult, show_dialog};
use rtwk::provider::select;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::settings::{ID, NAME, Open};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;

pub mod setup;
pub mod strings;

/// What the debug menu lists for this directory: each dialog on a state
/// worth opening, made by a fixture script.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries = [
        (
            "status_line_preview",
            "the status line setup with three items and runtime values",
            include_str!("fixtures/status_line.jsonl"),
        ),
        (
            "status_line_thread_usage",
            "the status line setup with the thread credits and cost",
            include_str!("fixtures/status_line_thread_usage.jsonl"),
        ),
        (
            "status_line_hostname",
            "the status line setup with the hostname",
            include_str!("fixtures/status_line_hostname.jsonl"),
        ),
        (
            "terminal_title",
            "the terminal title setup",
            include_str!("fixtures/title.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(|_| Screen.into()),
        })
        .collect()
}

/// Opens the dialog from the widget that asks (`show_dialog`); the result
/// returns to the model through a handler, as an intent with its ticket.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| Screen.into(),
        // Esc is the dialog's own key when one is up; the barrier closes
        // the unseeded slot.
        DialogOptions {
            barrier_dismissible: true,
            ..crate::sheets::sheet_options()
        },
    )
}

/// Opens the sheet with no seeded model (debug menu): the empty slot.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Screen.into())
}

/// Which dialog the model holds open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    Nothing,
    Setup,
    Other,
}

/// The sheet of this slot: the dialog the model holds open.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

impl_widget!(stateless eq Screen);

impl StatelessWidget for Screen {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let shown = select::<ScreenStates, Shown, _>(cx, |states| match &states.settings.open {
            None => Shown::Nothing,
            Some(Open::Setup(_)) => Shown::Setup,
            Some(_) => Shown::Other,
        });
        match shown {
            Shown::Setup => setup::SetupSheet.into(),
            // No dialog seeded (the debug menu's row): the skeleton's
            // empty-slot line.
            Shown::Nothing | Shown::Other => {
                let shared = crate::l10n::strings::of(cx);
                Dialog::bare(Text::new(shared.empty_slot(NAME, ID).to_string())).into()
            }
        }
    }
}
