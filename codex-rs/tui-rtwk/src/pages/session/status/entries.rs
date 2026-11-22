//! The debug-menu entries of the status area (`chat/status_*`, `chat/render_*`):
//! the Working row, the pending preview and the background-terminals line, each
//! oracle snapshot on the model a fixture script makes. The stack entries run
//! the real chat page (the status, the previews and the composer are one
//! column there); the preview and terminals entries draw their widget alone, as
//! the oracle's unit snapshots do.

use rtwk::AnyWidget;

use crate::app::debug::{DebugEntry, EntryOpen};

use super::{PendingInput, TerminalsLine};

macro_rules! preview {
    ($name:literal, $file:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $file, ".jsonl")),
            open: EntryOpen::Route(|_| -> AnyWidget { PendingInput.into() }),
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
        preview!("render_one_message", "preview_render_one_message", "one queued message"),
        preview!("render_two_messages", "preview_render_two_messages", "two queued messages"),
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
        stack!("status_and_composer", "status_running", "Working over the composer"),
        stack!("status_only", "status_running", "Working, nothing queued, the composer"),
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
