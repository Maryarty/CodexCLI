//! `transcript`: the view half of the slot of S10 (transcript view), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::transcript`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::transcript::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;

pub mod copy;
mod copy_feedback;
pub mod follow;
pub mod history;
pub mod pager;
pub mod selection;
pub mod sheet;
pub mod strings;
pub mod turn_details;

use crate::app::debug::EntryOpen;

/// What the debug menu lists for this directory: the static pager over the
/// document a fixture script gives the model.
pub fn debug_entries() -> Vec<DebugEntry> {
    [
        (
            "pager_basic",
            "the static pager, three short lines",
            include_str!("fixtures/pager_basic.jsonl"),
        ),
        (
            "pager_wraps_long_lines",
            "the static pager, one long line wrapped",
            include_str!("fixtures/pager_wraps_long_lines.jsonl"),
        ),
        (
            "pager_long",
            "the static pager, forty numbered lines",
            include_str!("fixtures/pager_long.jsonl"),
        ),
        (
            "pager_colors",
            "the static pager, the colours of a diff and a blank line",
            include_str!("fixtures/pager_colors.jsonl"),
        ),
        (
            "pager_action",
            "the static pager with an action on Enter",
            include_str!("fixtures/pager_action.jsonl"),
        ),
    ]
    .into_iter()
    .map(|(name, description, script)| DebugEntry {
        name,
        description,
        script,
        open: EntryOpen::Route(|_| pager::StaticPager.into()),
    })
    .chain([
        DebugEntry {
            name: "sheet_basic",
            description: "the transcript sheet over a prompt, a reasoning summary and a long answer",
            script: include_str!("fixtures/sheet_basic.jsonl"),
            open: EntryOpen::Route(sheet::seeded),
        },
        DebugEntry {
            name: "sheet_owned",
            description: "the transcript sheet without pager chrome (the owned full-screen Ctrl+T)",
            script: include_str!("fixtures/sheet_basic.jsonl"),
            open: EntryOpen::Route(sheet::seeded_bare),
        },
        DebugEntry {
            name: "sheet_reveal",
            description: "the transcript sheet over a prompt and a four-line answer",
            script: include_str!("fixtures/sheet_reveal.jsonl"),
            open: EntryOpen::Route(sheet::seeded),
        },
        DebugEntry {
            name: "sheet_link",
            description: "the transcript sheet over an answer that is a web link",
            script: include_str!("fixtures/sheet_link.jsonl"),
            open: EntryOpen::Route(sheet::seeded),
        },
        DebugEntry {
            name: "sheet_prompts",
            description: "the transcript sheet over three prompts and their answers",
            script: include_str!("fixtures/sheet_prompts.jsonl"),
            open: EntryOpen::Route(sheet::seeded),
        },
    ])
    .collect()
}

/// The part as a widget, placed by the chat screen's subtree.
pub fn part() -> AnyWidget {
    Screen.into()
}

/// Opens the empty form of this slot (debug menu), as a page.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
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
