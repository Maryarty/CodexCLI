//! `approvals`: the view half of the slot of S13 (approvals and requests), the `mock-dialogs` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_rtwk_core::screens::approvals`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

mod dismiss;

use rtwk::overlay::{Dialog, DialogOptions, DialogResult, show_dialog};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::screens::approvals::{ID, NAME};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;

pub mod strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![DebugEntry {
        name: "exec_approval",
        description: "an exec approval request waiting for the user",
        script: include_str!("fixtures/exec_approval.jsonl"),
        open: EntryOpen::Dialog(|seeded: &Seeded| match seeded.core.requests.head() {
            Some(request) => request_dialog(request.key.clone()),
            None => AnyWidget::default(),
        }),
    }]
}

/// The dialog the request presenter shows for the request `key`.
pub fn request_dialog(key: codex_tui_rtwk_core::ids::RequestKey) -> AnyWidget {
    dismiss::DismissDialog { key }.into()
}

/// Opens the dialog from the widget that asks (`show_dialog`); the result
/// returns to the model through a handler, as an intent with its ticket.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    let title = strings::of(cx).title;
    show_dialog(
        cx,
        move |_| Dialog::new(Screen).title(title).into(),
        DialogOptions::default(),
    )
}

/// Opens the empty form of this slot (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|lang| Dialog::new(Screen).title(strings::title(lang)).into())
}

/// The empty screen widget of this slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids, drafts and controllers only (R14).
#[derive(Debug)]
pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let shared = crate::l10n::strings::of(cx);
        Text::new(shared.empty_slot(NAME, ID).to_string()).into()
    }
}
