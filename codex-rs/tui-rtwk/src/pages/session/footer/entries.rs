//! The debug-menu entries of the footer (`chat/footer_*`): one footer state
//! per oracle snapshot of `bottom_pane/footer.rs` `footer_snapshots`, drawn by
//! the real `Footer` widget over the model a fixture script makes (the oracle
//! builds `FooterProps` by hand; here a thread, a token count or a started
//! turn make the same state).

use crate::app::debug::{DebugEntry, EntryOpen};

use super::{Footer, Line};

const IDLE: &str = include_str!("fixtures/footer_idle.jsonl");
const RUNNING: &str = include_str!("fixtures/footer_running.jsonl");
const CONTEXT_72: &str = include_str!("fixtures/footer_context_72.jsonl");
const TOKENS_USED: &str = include_str!("fixtures/footer_tokens_used.jsonl");

/// The footer of a composer with `draft_empty`, drawn as `line`; the page's
/// agents hint is off as in the oracle's default key hints.
fn footer(line: Line, draft_empty: bool) -> Footer {
    Footer {
        line,
        agents: false,
        draft_empty,
        shell: false,
        vim: None,
        hint_override: None,
        flash: None,
        on_queue: None,
        on_warnings: None,
        on_close: None,
    }
}

/// The entries of this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    let mut entries = vec![
        DebugEntry {
            name: "footer_shortcuts_default",
            description: "the idle footer: ? for shortcuts and the context left",
            script: IDLE,
            open: EntryOpen::Route(|_| footer(Line::Passive, true).into()),
        },
        DebugEntry {
            name: "footer_shortcuts_context_running",
            description: "a running turn with 72% of the context left",
            script: CONTEXT_72,
            open: EntryOpen::Route(|_| footer(Line::Passive, true).into()),
        },
        DebugEntry {
            name: "footer_context_tokens_used",
            description: "tokens counted, the window unknown: 123K used",
            script: TOKENS_USED,
            open: EntryOpen::Route(|_| footer(Line::Passive, true).into()),
        },
        DebugEntry {
            name: "footer_composer_has_draft_queue_hint_enabled",
            description: "a running turn and a draft: tab to queue message",
            script: RUNNING,
            open: EntryOpen::Route(|_| footer(Line::Passive, false).into()),
        },
        DebugEntry {
            name: "footer_ctrl_c_quit_idle",
            description: "the quit reminder: ctrl c again to quit",
            script: IDLE,
            open: EntryOpen::Route(|_| footer(Line::Quit, true).into()),
        },
        DebugEntry {
            name: "footer_esc_hint_idle",
            description: "esc esc to edit previous message",
            script: IDLE,
            open: EntryOpen::Route(|_| footer(Line::EscIdle, true).into()),
        },
        DebugEntry {
            name: "footer_esc_hint_primed",
            description: "esc again to edit previous message",
            script: IDLE,
            open: EntryOpen::Route(|_| footer(Line::EscPrimed, true).into()),
        },
    ];
    entries.extend(super::effort::debug_entries());
    entries
}
