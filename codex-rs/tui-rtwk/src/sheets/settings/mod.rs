//! `settings`: the view half of S15 (settings dialogs): modal sheets over the
//! session page, each drawn without a frame at the bottom edge, as the
//! oracle draws its bottom-pane views (architecture §1, amendment
//! 2026-11-19b). `Screen` is the one widget the app opens; which dialog it
//! shows is what the model holds open (`core::screens::settings::Open`).
//! Their model part, events, intents and RPCs are core's
//! `codex_tui_core::screens::settings`.

use rtwk::overlay::{Dialog, DialogOptions, DialogResult, close_dialog, show_dialog};
use rtwk::provider::select;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::settings::{ID, NAME, Open};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;

pub mod features;
pub mod hooks;
pub mod items;
pub mod memories;
pub mod setup;
pub mod strings;
pub mod theme;

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
        (
            "theme_picker",
            "the syntax theme picker over six themes, the codex home under ~",
            include_str!("fixtures/theme_picker.jsonl"),
        ),
        (
            "skills_basic",
            "the skills toggle: two skills, the first on",
            include_str!("fixtures/skills_basic.jsonl"),
        ),
        (
            "skills_long_names",
            "the skills toggle with two long display names",
            include_str!("fixtures/skills_long_names.jsonl"),
        ),
        (
            "skills_empty",
            "the skills toggle with no skills",
            include_str!("fixtures/skills_empty.jsonl"),
        ),
        (
            "experimental_features_two",
            "the experimental features with two features, the first on",
            include_str!("fixtures/experimental_two.jsonl"),
        ),
        (
            "experimental_features_discovery",
            "the experimental features after the server's catalogue arrived: three on",
            include_str!("fixtures/experimental_discovery.jsonl"),
        ),
        (
            "experimental_features_save_failed",
            "the experimental features after a write that did not confirm",
            include_str!("fixtures/experimental_save_failed.jsonl"),
        ),
        (
            "experimental_features_empty",
            "the experimental features, the catalogue came empty",
            include_str!("fixtures/experimental_empty.jsonl"),
        ),
        (
            "experimental_features_unavailable",
            "the experimental features, the catalogue request failed",
            include_str!("fixtures/experimental_unavailable.jsonl"),
        ),
        (
            "experimental_features_changed_default",
            "the experimental features with one server feature",
            include_str!("fixtures/experimental_changed_default.jsonl"),
        ),
        (
            "memories_settings",
            "the memories dialog: use on, generate off",
            include_str!("fixtures/memories.jsonl"),
        ),
        (
            "memories_reset_confirmation",
            "the memories reset confirmation",
            include_str!("fixtures/memories_reset_confirmation.jsonl"),
        ),
        (
            "hooks_events",
            "the hooks browser: three hooks, the events page",
            include_str!("fixtures/hooks_events.jsonl"),
        ),
        (
            "hooks_events_review_column",
            "the hooks browser with one untrusted hook: the review column",
            include_str!("fixtures/hooks_events_review_column.jsonl"),
        ),
        (
            "hooks_events_issues",
            "the hooks browser with no hooks, a skipped matcher and a parse error",
            include_str!("fixtures/hooks_events_issues.jsonl"),
        ),
        (
            "hooks_status_messages",
            "the hooks browser: two status messages, one blank",
            include_str!("fixtures/hooks_status_messages.jsonl"),
        ),
        (
            "hooks_mcp_tool",
            "the hooks browser: one plugin hook with an MCP tool handler",
            include_str!("fixtures/hooks_mcp_tool.jsonl"),
        ),
        (
            "hooks_context_limit",
            "the hooks browser: one hook with an unlimited additional context",
            include_str!("fixtures/hooks_context_limit.jsonl"),
        ),
        (
            "hooks_untrusted_enabled",
            "the hooks browser: one untrusted, enabled, async hook",
            include_str!("fixtures/hooks_untrusted_enabled.jsonl"),
        ),
        (
            "hooks_trusted_and_review",
            "the hooks browser: a trusted hook and an untrusted one",
            include_str!("fixtures/hooks_trusted_and_review.jsonl"),
        ),
        (
            "hooks_two_managed",
            "the hooks browser: two system hooks of PreToolUse",
            include_str!("fixtures/hooks_two_managed.jsonl"),
        ),
        (
            "hooks_many",
            "the hooks browser: eleven hooks of PreToolUse, every second one named",
            include_str!("fixtures/hooks_many.jsonl"),
        ),
        (
            "hooks_long_command",
            "the hooks browser: one hook with a command of twenty words and a title",
            include_str!("fixtures/hooks_long_command.jsonl"),
        ),
        (
            "hooks_empty",
            "the hooks browser with no hooks at all",
            include_str!("fixtures/hooks_empty.jsonl"),
        ),
        (
            "hooks_review_needed",
            "the hooks browser: one untrusted, disabled hook",
            include_str!("fixtures/hooks_review_needed.jsonl"),
        ),
        (
            "hooks_managed_handler",
            "the hooks browser: one managed hook of PreToolUse",
            include_str!("fixtures/hooks_managed_handler.jsonl"),
        ),
        (
            "hooks_modified_handler",
            "the hooks browser: one modified, disabled hook",
            include_str!("fixtures/hooks_modified_handler.jsonl"),
        ),
        (
            "hooks_events_review_mixed",
            "the hooks browser: an untrusted, a modified and a trusted hook",
            include_str!("fixtures/hooks_events_review_mixed.jsonl"),
        ),
        (
            "hooks_wrapped_url",
            "the hooks browser: a command holding a URL longer than the details column",
            include_str!("fixtures/hooks_wrapped_url.jsonl"),
        ),
        (
            "theme_picker_all",
            "the syntax theme picker over the built-in themes and a custom one",
            include_str!("fixtures/theme_picker_all.jsonl"),
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
    Theme,
    Features,
    Memories,
    Hooks,
    Other,
}

/// The sheet of this slot: the dialog the model holds open. It closes the
/// slot when the model closes what it held (the sheets are replaced by this
/// widget's own build in that frame, so none of them sees it).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: whether a dialog was ever held open here.
pub struct ScreenState {
    had: bool,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState { had: false }
    }
}

impl_widget!(stateful eq Screen);

fn shown_of(cx: &mut Cx<'_, ScreenState>) -> Shown {
    select::<ScreenStates, Shown, _>(cx, |states| match &states.settings.open {
        None => Shown::Nothing,
        Some(Open::Setup(_)) => Shown::Setup,
        Some(Open::Theme(_)) => Shown::Theme,
        Some(Open::Experimental(_)) => Shown::Features,
        Some(Open::Memories(_)) => Shown::Memories,
        Some(Open::Hooks(_)) => Shown::Hooks,
        Some(_) => Shown::Other,
    })
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        match shown_of(cx) {
            Shown::Nothing if self.had => close_dialog::<_, ()>(cx, None),
            Shown::Nothing => {}
            _ => self.had = true,
        }
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        match shown_of(cx) {
            Shown::Setup => setup::SetupSheet.into(),
            Shown::Theme => theme::ThemeSheet.into(),
            Shown::Features => features::FeaturesSheet.into(),
            Shown::Memories => memories::MemoriesSheet.into(),
            Shown::Hooks => hooks::HooksSheet.into(),
            // No dialog seeded (the debug menu's row): the skeleton's
            // empty-slot line.
            Shown::Nothing | Shown::Other => {
                let shared = crate::l10n::strings::of(cx);
                Dialog::bare(Text::new(shared.empty_slot(NAME, ID).to_string())).into()
            }
        }
    }
}
