//! The `[tui]` settings a screen test turns on after the screen is open (the host sends them
//! once at the start: `AppEv::Tui`).

use codex_tui_core::app::AppEv;
use codex_tui_core::app::facts::{Rendering, TuiSettings};

use super::Screen;

impl Screen {
    /// `tui.collapse_turn_details`: the tool calls of a finished turn fold into one row each
    /// run, and the messages before the answer behind the prompt.
    pub fn collapse_turn_details(&mut self) -> &mut Screen {
        self.h.inject([AppEv::Tui(TuiSettings {
            collapse_turn_details: true,
            ..TuiSettings::default()
        })]);
        self
    }

    /// `tui.rendering`: the Markdown renderers that are on (a table, list, mermaid or math
    /// renderer that is off keeps the source text).
    pub fn rendering(&mut self, rendering: Rendering) -> &mut Screen {
        self.h.inject([AppEv::Tui(TuiSettings {
            rendering,
            ..TuiSettings::default()
        })]);
        self
    }
}
