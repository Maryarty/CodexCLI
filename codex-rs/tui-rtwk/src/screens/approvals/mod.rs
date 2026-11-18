//! `approvals`: the view half of the slot of S13 (approvals and requests), the `mock-dialogs` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_rtwk_core::screens::approvals`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

mod dialog;
mod dismiss;
mod present;

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
    let entries = [
        (
            "exec_approval",
            "a request with no presentation: the dismiss fallback",
            include_str!("fixtures/exec_approval.jsonl"),
        ),
        (
            "exec_command",
            "approval_modal_exec: a command with a reason",
            include_str!("fixtures/exec_command.jsonl"),
        ),
        (
            "patch_edits",
            "a file-change approval",
            include_str!("fixtures/patch_edits.jsonl"),
        ),
        (
            "permissions",
            "a permissions request",
            include_str!("fixtures/permissions.jsonl"),
        ),
        (
            "mcp_elicitation",
            "mcp_server_elicitation_approval_form_without_schema",
            include_str!("fixtures/mcp_elicitation.jsonl"),
        ),
        (
            "user_input",
            "request_user_input_options: a question with options",
            include_str!("fixtures/user_input.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(head_dialog),
        })
        .collect()
}

/// The dialog of the head of the seeded queue.
fn head_dialog(seeded: &Seeded) -> AnyWidget {
    match seeded.core.requests.head() {
        Some(request) => request_dialog(request.key.clone()),
        None => AnyWidget::default(),
    }
}

/// The dialog the request presenter shows for the request `key`: the prompt
/// the model holds for it, else the skeleton's dismiss dialog.
pub fn request_dialog(key: codex_tui_rtwk_core::ids::RequestKey) -> AnyWidget {
    dialog::RequestDialog { key }.into()
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
