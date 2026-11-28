//! The debug-menu entries of the footer (`chat/footer_*`): one footer state
//! per oracle snapshot of `bottom_pane/footer.rs` `footer_snapshots`, drawn by
//! the real `Footer` widget over the model a fixture script makes (the oracle
//! builds `FooterProps` by hand; here a thread, a token count or a started
//! turn make the same state).

use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::ThreadModel;

use crate::app::debug::{DebugEntry, EntryOpen};
use crate::sheets::dialogs::{Panel, shortcut_panel_fixed};

use super::{Footer, Line};

const IDLE: &str = include_str!("fixtures/footer_idle.jsonl");
const RUNNING: &str = include_str!("fixtures/footer_running.jsonl");
const CONTEXT_72: &str = include_str!("fixtures/footer_context_72.jsonl");
const TOKENS_USED: &str = include_str!("fixtures/footer_tokens_used.jsonl");
const OVERLAY: &str = include_str!("fixtures/footer_overlay.jsonl");
const OVERLAY_MODES: &str = include_str!("fixtures/footer_overlay_modes.jsonl");
const OVERLAY_RUNNING: &str = include_str!("fixtures/footer_overlay_running.jsonl");

/// The shortcut overlay as the oracle's footer draws it for `ShortcutOverlay` (`lines`): the
/// panel alone, with the rows the model's flags decide; `esc_primed` is `esc_backtrack_hint`,
/// `enhanced` is the terminal telling Shift+Enter from Enter (`insert_newline` as `⇧enter`).
#[derive(Clone, PartialEq)]
struct Overlay {
    esc_primed: bool,
    enhanced: bool,
}

impl_widget!(stateless eq Overlay);

impl StatelessWidget for Overlay {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let running = read::<ThreadModel, _>(cx).is_running();
        let panel = Panel {
            running,
            esc_primed: self.esc_primed,
            enhanced: self.enhanced,
            ..Panel::default()
        }
        .with_flags(cx);
        shortcut_panel_fixed(cx, panel)
    }
}

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
        DebugEntry {
            name: "footer_shortcuts_shift_and_esc",
            description: "the shortcut overlay with the backtrack primed and Shift+Enter for the new line",
            script: OVERLAY,
            open: EntryOpen::Route(|_| Overlay { esc_primed: true, enhanced: true }.into()),
        },
        DebugEntry {
            name: "footer_shortcuts_collaboration_modes_enabled",
            description: "the shortcut overlay with the collaboration modes on (Change mode)",
            script: OVERLAY_MODES,
            open: EntryOpen::Route(|_| Overlay { esc_primed: false, enhanced: false }.into()),
        },
        DebugEntry {
            name: "footer_shortcuts_running",
            description: "the shortcut overlay while a turn runs (Queue message, Interrupt)",
            script: OVERLAY_RUNNING,
            open: EntryOpen::Route(|_| Overlay { esc_primed: false, enhanced: false }.into()),
        },
    ];
    entries.extend(super::effort::debug_entries());
    entries
}
