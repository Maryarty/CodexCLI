//! `approvals`: the view half of the slot of S13 (approvals and requests), the `mock-dialogs` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::approvals`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

mod auto_review;
mod banner;
mod clip;
mod decision_cell;
mod dialog;
mod dismiss;
mod form;
mod guardian;
mod keys;
mod present;
mod tips;
mod user_input;

use rtwk::layout::SizedBox;
use rtwk::overlay::{Dialog, DialogResult, show_dialog};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::approvals::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;

pub mod strings;

pub use auto_review::{menu as auto_review_menu, open as open_auto_review};
pub use banner::PendingThreadApprovals;
pub use decision_cell::{DecisionCell, ReviewLineCell};
pub use guardian::{ReviewStatus, review_status, summary as review_summary};

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
            "exec_request",
            "make_exec_request: echo hi, a reason, Accept and Cancel",
            include_str!("fixtures/exec_request.jsonl"),
        ),
        (
            "exec_clipped",
            "a header cut to the pane: `[… N lines] ⌃a view all`",
            include_str!("fixtures/exec_clipped.jsonl"),
        ),
        (
            "exec_url_reason",
            "a reason that is a URL, wrapped",
            include_str!("fixtures/exec_url_reason.jsonl"),
        ),
        (
            "exec_cross_thread",
            "a request of another thread: the `o` hint",
            include_str!("fixtures/exec_cross_thread.jsonl"),
        ),
        (
            "exec_snippet",
            "header_includes_command_snippet",
            include_str!("fixtures/exec_snippet.jsonl"),
        ),
        (
            "exec_additional_permissions",
            "additional permissions: the rule line",
            include_str!("fixtures/exec_additional_permissions.jsonl"),
        ),
        (
            "exec_network",
            "a blocked network request: the host in the title",
            include_str!("fixtures/exec_network.jsonl"),
        ),
        (
            "exec_empty_command",
            "an exec approval whose command is empty",
            include_str!("fixtures/exec_empty_command.jsonl"),
        ),
        (
            "exec_zsh_git_add",
            "a long zsh command",
            include_str!("fixtures/exec_zsh_git_add.jsonl"),
        ),
        (
            "patch_no_changes",
            "no changes: destination unavailable",
            include_str!("fixtures/patch_no_changes.jsonl"),
        ),
        (
            "patch_thread_label",
            "another thread, with destinations",
            include_str!("fixtures/patch_thread_label.jsonl"),
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
            "mcp_tool_persist",
            "approval_form_tool_approval_with_persist_options",
            include_str!("fixtures/mcp_tool_persist.jsonl"),
        ),
        (
            "mcp_param_summary",
            "approval_form_tool_approval_with_param_summary",
            include_str!("fixtures/mcp_param_summary.jsonl"),
        ),
        (
            "mcp_message_only",
            "message_only_form_snapshot",
            include_str!("fixtures/mcp_message_only.jsonl"),
        ),
        (
            "mcp_message_only_persist",
            "message_only_form_with_persist_options_snapshot",
            include_str!("fixtures/mcp_message_only_persist.jsonl"),
        ),
        (
            "mcp_boolean_form",
            "boolean_form_snapshot",
            include_str!("fixtures/mcp_boolean_form.jsonl"),
        ),
        (
            "mcp_two_booleans",
            "horizontal_list_keys_move_between_select_fields",
            include_str!("fixtures/mcp_two_booleans.jsonl"),
        ),
        (
            "mcp_two_strings",
            "switching_fields_clears_length_validation_flash",
            include_str!("fixtures/mcp_two_strings.jsonl"),
        ),
        (
            "mcp_text_chords",
            "text_fields_inherit_composer_chord_context",
            include_str!("fixtures/mcp_text_chords.jsonl"),
        ),
        (
            "mcp_wrapped_url_form",
            "form_prompt_preserves_wrapped_url_destination",
            include_str!("fixtures/mcp_wrapped_url_form.jsonl"),
        ),
        (
            "mcp_select",
            "an enum, a oneOf, an optional text and a boolean field",
            include_str!("fixtures/mcp_select.jsonl"),
        ),
        (
            "user_input",
            "request_user_input_options: a question with options",
            include_str!("fixtures/user_input.jsonl"),
        ),
        (
            "user_input_options",
            "question_with_options, a tall frame",
            include_str!("fixtures/user_input_options.jsonl"),
        ),
        (
            "user_input_freeform",
            "question_without_options: the input takes the area",
            include_str!("fixtures/user_input_freeform.jsonl"),
        ),
        (
            "user_input_two_options",
            "two questions with options, the footer tips wrap",
            include_str!("fixtures/user_input_two_options.jsonl"),
        ),
        (
            "user_input_what_next",
            "five options in a short area: the footer names one",
            include_str!("fixtures/user_input_what_next.jsonl"),
        ),
        (
            "user_input_very_long_option",
            "an option whose text wraps",
            include_str!("fixtures/user_input_very_long_option.jsonl"),
        ),
        (
            "user_input_options_then_freeform",
            "an options question, then a freeform one",
            include_str!("fixtures/user_input_options_then_freeform.jsonl"),
        ),
        (
            "user_input_freeform_then_options",
            "a freeform question, then an options one",
            include_str!("fixtures/user_input_freeform_then_options.jsonl"),
        ),
        (
            "user_input_secret",
            "a freeform question with a secret answer: drawn as `*`",
            include_str!("fixtures/user_input_secret.jsonl"),
        ),
        (
            "user_input_url_question",
            "a freeform question whose text holds a URL",
            include_str!("fixtures/user_input_url_question.jsonl"),
        ),
        (
            "user_input_auto_resolution",
            "not blocking: a countdown, then an empty answer",
            include_str!("fixtures/user_input_auto_resolution.jsonl"),
        ),
        (
            "user_verification",
            "verify and approve, or cancel the request",
            include_str!("fixtures/user_verification.jsonl"),
        ),
        (
            "user_verification_waiting",
            "the verification runs: the proof is awaited",
            include_str!("fixtures/user_verification_waiting.jsonl"),
        ),
        (
            "user_verification_long_urls",
            "long URLs in the title and the description",
            include_str!("fixtures/user_verification_long_urls.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            // The oracle's form snapshots render the overlay into a buffer taller than its
            // desired height, and the form fills it; the debug menu is that frame.
            open: EntryOpen::Dialog(
                if name.starts_with("mcp_") || name.starts_with("user_input_") {
                    framed_dialog
                } else {
                    head_dialog
                },
            ),
        })
        .collect()
}

/// The head's dialog in a frame as tall as the screen (the oracle snapshot tests' buffer): the
/// form's footer stays at the foot and its input takes the rows between.
fn framed_dialog(seeded: &Seeded) -> AnyWidget {
    let dialog = head_dialog(seeded);
    LayoutBuilder::new(move |_, constraints| {
        SizedBox::new(None, Some(constraints.max_height), Some(dialog.clone())).into()
    })
    .into()
}

/// The dialog of the head of the seeded queue.
fn head_dialog(seeded: &Seeded) -> AnyWidget {
    match seeded.core.requests().head() {
        Some(request) => request_dialog(request.key.clone()),
        None => AnyWidget::default(),
    }
}

/// The dialog the request presenter shows for the request `key`: the prompt
/// the model holds for it, else the skeleton's dismiss dialog.
pub fn request_dialog(key: codex_tui_core::ids::RequestKey) -> AnyWidget {
    dialog::RequestDialog { key }.into()
}

/// Opens the dialog from the widget that asks (`show_dialog`); the result
/// returns to the model through a handler, as an intent with its ticket.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    let title = strings::of(cx).title;
    show_dialog(
        cx,
        move |_| Dialog::new(Screen).title(title).into(),
        // The empty slot has no keys of its own: the barrier closes it.
        rtwk::overlay::DialogOptions {
            barrier_dismissible: true,
            ..crate::sheets::sheet_options()
        },
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
