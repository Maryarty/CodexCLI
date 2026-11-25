//! The debug-menu entries of the status area (`chat/status_*`, `chat/render_*`):
//! the Working row, the pending preview and the background-terminals line, each
//! oracle snapshot on the model a fixture script makes. The stack entries run
//! the real chat page (the status, the previews and the composer are one
//! column there); the preview and terminals entries draw their widget alone, as
//! the oracle's unit snapshots do.

use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::keymap::actions::CHAT_INTERRUPT_TURN;
use codex_tui_core::screens::chat::{StatusDetails, StatusFacts};

use crate::app::debug::{DebugEntry, EntryOpen};

use super::super::activity::{Activity, effects};
use super::{PendingInput, TerminalsLine};

/// The Working row alone, as the oracle's `StatusIndicatorWidget` unit
/// snapshots draw it: the switches of the animations, the interrupt hint, the
/// details and the summary after the label are the case's.
#[derive(Clone, Debug, PartialEq)]
struct WorkingRow {
    animations: bool,
    shimmer: bool,
    progress: bool,
    /// The interrupt key after the elapsed label; none while the turn cannot be interrupted.
    hint: bool,
    /// The details under the header; the flag keeps their case.
    details: Option<(&'static str, bool)>,
    /// The background-terminal summary after the label.
    terminal: bool,
    hook: Option<&'static str>,
}

impl WorkingRow {
    const DEFAULT: WorkingRow = WorkingRow {
        animations: true,
        shimmer: true,
        progress: true,
        hint: true,
        details: None,
        terminal: false,
        hook: None,
    };
}

impl_widget!(stateless eq WorkingRow);

impl StatelessWidget for WorkingRow {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let facts = StatusFacts {
            animations: self.animations,
            shimmer_effect: self.shimmer,
            progress_effect: self.progress,
            ..StatusFacts::default()
        };
        let (spinner, shimmer) = effects(&facts);
        let keymap = crate::keymap::keymap(cx);
        let strings = super::strings::of(cx);
        Activity {
            interrupt: self
                .hint
                .then(|| crate::keymap::label(&keymap, CHAT_INTERRUPT_TURN))
                .flatten(),
            spinner,
            shimmer,
            header: crate::l10n::strings::of(cx).chat.working.to_string(),
            details: self.details.map(|(text, preserve)| {
                if preserve {
                    StatusDetails::preserved(
                        text,
                        codex_tui_core::screens::chat::STATUS_DETAILS_MAX_LINES,
                    )
                } else {
                    StatusDetails::new(
                        text.to_string(),
                        codex_tui_core::screens::chat::STATUS_DETAILS_MAX_LINES,
                    )
                }
            }),
            inline: self.terminal.then(|| strings.terminals(1)),
            hook: self.hook.map(str::to_string),
            offset_seconds: 0,
        }
        .into()
    }
}

macro_rules! row {
    ($name:literal, $description:literal, $case:expr) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!("fixtures/working_row.jsonl"),
            open: EntryOpen::Route(|_| -> AnyWidget { $case.into() }),
        }
    };
}

macro_rules! preview {
    ($name:literal, $file:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $file, ".jsonl")),
            open: EntryOpen::Route(|_| -> AnyWidget { PendingInput { questions: false }.into() }),
        }
    };
}

macro_rules! stack {
    ($name:literal, $file:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $file, ".jsonl")),
            open: EntryOpen::Route(|seeded| super::super::seeded(seeded)),
        }
    };
}

macro_rules! terminals {
    ($name:literal, $file:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $file, ".jsonl")),
            open: EntryOpen::Route(|_| -> AnyWidget { TerminalsLine.into() }),
        }
    };
}

/// The entries of this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        row!("working_row", "the Working row alone", WorkingRow::DEFAULT),
        row!(
            "working_row_wrapped_details",
            "details that wrap, no interrupt hint, no effects",
            WorkingRow {
                animations: false,
                hint: false,
                details: Some(("A man a plan a canal panama", false)),
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_no_animations",
            "the animations switched off: no bullet",
            WorkingRow {
                animations: false,
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_remapped_interrupt",
            "the interrupt key remapped (run with a keymap)",
            WorkingRow {
                animations: false,
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_hook",
            "a hook message and details, no background terminal",
            WorkingRow {
                animations: false,
                details: Some(("existing details", true)),
                hook: Some("checking 日本語 ｶﾞﾊﾟ policy"),
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_hook_terminal",
            "a hook message, details and a background terminal",
            WorkingRow {
                animations: false,
                details: Some(("existing details", true)),
                terminal: true,
                hook: Some("checking 日本語 ｶﾞﾊﾟ policy"),
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_effects_plain",
            "animations on, shimmer and progress off",
            WorkingRow {
                shimmer: false,
                progress: false,
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_effects_progress",
            "animations on, progress only",
            WorkingRow {
                shimmer: false,
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_effects_shimmer",
            "animations on, shimmer only",
            WorkingRow {
                progress: false,
                ..WorkingRow::DEFAULT
            }
        ),
        row!(
            "working_row_effects_master_off",
            "the master switch off, shimmer and progress on",
            WorkingRow {
                animations: false,
                ..WorkingRow::DEFAULT
            }
        ),
        preview!(
            "render_one_message",
            "preview_render_one_message",
            "one queued message"
        ),
        preview!(
            "render_two_messages",
            "preview_render_two_messages",
            "two queued messages"
        ),
        preview!(
            "render_more_than_three_messages",
            "preview_render_more_than_three_messages",
            "four queued messages"
        ),
        preview!(
            "render_wrapped_message",
            "preview_render_wrapped_message",
            "a queued message that wraps"
        ),
        preview!(
            "render_many_line_message",
            "preview_render_many_line_message",
            "a queued message of five lines: three rows, then an ellipsis"
        ),
        preview!(
            "render_one_pending_steer",
            "preview_render_one_pending_steer",
            "one steer waiting for the next tool call"
        ),
        preview!(
            "render_one_pending_steer_with_remapped_interrupt_binding",
            "preview_render_one_pending_steer",
            "one steer, the interrupt key remapped (run with a keymap)"
        ),
        preview!(
            "render_pending_steers_above_queued_messages",
            "preview_render_pending_steers_above_queued_messages",
            "two steers, a refused steer and a queued follow-up"
        ),
        preview!(
            "render_multiline_pending_steer_uses_single_prefix_and_truncates",
            "preview_render_multiline_pending_steer",
            "a multi-line steer under one prefix, clipped at three rows"
        ),
        preview!(
            "render_oversized_preview_url",
            "preview_render_oversized_preview_url",
            "a queued URL wider than the pane"
        ),
        preview!(
            "render_narrow_preview",
            "preview_render_narrow_preview",
            "a queued message at four columns"
        ),
        preview!(
            "long_url_like_message",
            "preview_long_url_like_message",
            "a URL-like token takes one row, not wrapped ellipsis rows"
        ),
        terminals!(
            "render_more_sessions",
            "render_more_sessions",
            "one background terminal"
        ),
        terminals!(
            "render_many_sessions",
            "render_many_sessions",
            "123 background terminals"
        ),
        stack!(
            "status_timer_hidden_row",
            "status_timer_hidden_row",
            "Working at 42 s over the composer"
        ),
        stack!(
            "status_and_composer",
            "status_running",
            "Working over the composer"
        ),
        stack!(
            "status_only",
            "status_running",
            "Working, nothing queued, the composer"
        ),
        stack!(
            "status_unified_exec",
            "status_unified_exec",
            "Working with one background terminal after the elapsed label"
        ),
        stack!(
            "status_with_details_queued",
            "status_with_details_queued",
            "Working with two detail lines and a queued follow-up"
        ),
        stack!(
            "queued_status_hidden",
            "queued_status_hidden",
            "a queued follow-up with no turn running"
        ),
        stack!(
            "status_and_queued",
            "status_and_queued",
            "Working and a queued follow-up"
        ),
    ]
}
