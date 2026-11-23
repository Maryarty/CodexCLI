//! `settings`: the view half of S15 (settings dialogs): modal sheets over the
//! session page, each drawn without a frame at the bottom edge, as the
//! oracle draws its bottom-pane views (architecture §1, amendment
//! 2026-11-19b). `Screen` is the one widget the app opens; which dialog it
//! shows is the one a slash command (or a seed) asked for
//! (`core::screens::settings::Model::requested`), taken once. What each dialog
//! is before it is saved (its checks, its order, its page) is its sheet's
//! draft (`draft.rs`); the model keeps what it is offered and what it asked
//! of the app (`Edit`, ADR-013). Their model part, events, intents and RPCs
//! are core's `codex_tui_core::screens::settings`.

use rtwk::overlay::{Dialog, DialogOptions, DialogResult, show_dialog};
use rtwk::provider::{read, select};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::settings::{Edit, ID, Intent, NAME, Which};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::view::Hs;

pub mod draft;
pub mod features;
pub mod hooks;
pub mod items;
mod keys;
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
            open: if name == "memories_reset_confirmation" {
                // The dialog opens on the confirmation: the draft's start.
                EntryOpen::Dialog(|_| {
                    Screen {
                        confirm_reset: true,
                    }
                    .into()
                })
            } else {
                EntryOpen::Dialog(|_| Screen::default().into())
            },
        })
        .collect()
}

/// Opens the dialog from the widget that asks (`show_dialog`); the result
/// returns to the model through a handler, as an intent with its ticket.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| Screen::default().into(),
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
    DebugOpen::Dialog(|_| Screen::default().into())
}

/// What a dialog asks of the app: a request the model records (`Intent::Request`).
fn request<S: State + 'static>(cx: &mut Cx<'_, S>, edit: Edit) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Request(edit)));
}

/// Which sheet the dialog asked for is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    Setup(Which),
    Theme,
    Features,
    Memories,
    Hooks,
    Other,
}

impl Shown {
    fn of(which: Which) -> Self {
        match which {
            Which::StatusLine | Which::Title | Which::Skills => Shown::Setup(which),
            Which::Theme => Shown::Theme,
            Which::Experimental => Shown::Features,
            Which::Memories => Shown::Memories,
            Which::Hooks => Shown::Hooks,
            Which::Config | Which::Keymap => Shown::Other,
        }
    }
}

/// The sheet of this slot: the dialog that was asked for, taken from the model when it is
/// first there (the sheet is opened before or after the slash command's intent is applied).
/// The sheets close themselves when they are done.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    /// The memories dialog opens on its reset confirmation (a debug entry's start).
    pub confirm_reset: bool,
}

/// Widget state: the sheet taken, if one was asked for.
pub struct ScreenState {
    shown: Option<Shown>,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState { shown: None }
    }
}

impl_widget!(stateful eq Screen);

fn requested(cx: &mut Cx<'_, ScreenState>) -> Option<Which> {
    select::<ScreenStates, Option<Which>, _>(cx, |states| states.settings.requested)
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        if self.shown.is_some() {
            return;
        }
        if let Some(which) = requested(cx) {
            self.shown = Some(Shown::of(which));
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Taken));
        }
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        match self.shown {
            Some(Shown::Setup(which)) => setup::SetupSheet { which }.into(),
            Some(Shown::Theme) => theme::ThemeSheet.into(),
            Some(Shown::Features) => features::FeaturesSheet.into(),
            Some(Shown::Memories) => memories::MemoriesSheet {
                confirming: w.confirm_reset,
            }
            .into(),
            Some(Shown::Hooks) => hooks::HooksSheet.into(),
            // No dialog seeded (the debug menu's row): the skeleton's
            // empty-slot line.
            None | Some(Shown::Other) => {
                let shared = crate::l10n::strings::of(cx);
                Dialog::bare(Text::new(shared.empty_slot(NAME, ID).to_string())).into()
            }
        }
    }
}
