//! Opening the dialogs: from the widget that asks (`open`), from the debug menu (`debug_entries`,
//! `open_empty`), and what a dialog asks of the app (`request`).

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::settings::{Edit, Intent};

use super::shown::Screen;
use super::which::Args;
use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::features::verb;

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
            "skills_menu",
            "the menu of /skills: list the skills, or enable and disable them",
            include_str!("fixtures/skills_menu.jsonl"),
        ),
        (
            "enable_subagents",
            "the prompt that offers to turn the subagents on",
            include_str!("fixtures/enable_subagents.jsonl"),
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
            "config_panel",
            "the configuration panel on the oracle's default settings",
            include_str!("fixtures/config_panel.jsonl"),
        ),
        (
            "keymap_picker",
            "the keymap editor over the built-in keymap",
            include_str!("fixtures/keymap_default.jsonl"),
        ),
        (
            "keymap_inspector",
            "the keypress inspector of the keymap editor",
            include_str!("fixtures/keymap_default.jsonl"),
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
            open: match name {
                // The dialog opens on the confirmation: the draft's start.
                "memories_reset_confirmation" => {
                    EntryOpen::Dialog(|seeded| debug_screen(seeded, true, false))
                }
                // The keymap editor opens on the inspector.
                "keymap_inspector" => EntryOpen::Dialog(|seeded| debug_screen(seeded, false, true)),
                _ => EntryOpen::Dialog(|seeded| debug_screen(seeded, false, false)),
            },
        })
        .collect()
}

/// The dialog a seeded entry shows: the kind the script offered (a script offers one).
fn debug_screen(seeded: &Seeded<'_>, confirm_reset: bool, inspect: bool) -> AnyWidget {
    let which = seeded
        .core
        .features()
        .settings
        .offers
        .keys()
        .next()
        .copied();
    Screen {
        which,
        confirm_reset,
        inspect,
        live: false,
        queued: false,
    }
    .into()
}

/// Opens the dialog of `args` from the widget that asks: the feature is told to build its
/// catalogue (`Intent::Open`, `Intent::Inspect`) and wishes for the sheet once it is there
/// (`Wish::Show`, shown by `FeatureView::shown`); a dialog with nothing to show is not opened.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
    if args.inspect {
        verb(cx, Intent::Inspect);
    } else {
        verb(cx, Intent::Open(args.which));
    }
}

/// Opens the sheet with no seeded model (debug menu): the empty slot.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Screen::default().into())
}

/// What a dialog asks of the app: a request the feature records (`Intent::Request`) and, when
/// it is a write of the config file, makes.
pub(super) fn request<S: 'static>(cx: &mut Cx<'_, S>, edit: Edit) {
    verb(cx, Intent::Request(edit));
}
