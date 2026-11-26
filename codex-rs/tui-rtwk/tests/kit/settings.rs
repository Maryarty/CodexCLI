//! The `[tui]` settings a screen test turns on after the screen is open (the host sends them
//! once at the start: `AppEv::Tui`).

use codex_tui_core::app::AppEv;
use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_rtwk::app::UiApp;

use super::Screen;

/// The `tui.rendering` switches a test names: each Markdown renderer on (the default) or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Renderers {
    pub mermaid: bool,
    pub math: bool,
    pub tables: bool,
    pub lists: bool,
}

impl Default for Renderers {
    fn default() -> Self {
        Self {
            mermaid: true,
            math: true,
            tables: true,
            lists: true,
        }
    }
}

impl Screen {
    /// The host's explicit rows per native wheel event.
    pub fn mouse_scroll_speed(&mut self, speed: Option<f64>) -> &mut Screen {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            wheel_rows_per_event: speed,
            ..settings
        })]);
        self
    }

    /// The clipboard policy for mouse selections.
    pub fn selection_copy_policy(&mut self, copy_on_select: bool, deselect_after_copy: bool) {
        self.h.inject([AppEv::Tui(TuiSettings {
            copy_on_select,
            deselect_after_copy,
            ..TuiSettings::default()
        })]);
    }

    /// Seed the host's local X11 clipboard capability.
    pub fn primary_selection(&mut self) {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            primary_selection: true,
            ..settings
        })]);
    }

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
    pub fn rendering(&mut self, rendering: Renderers) -> &mut Screen {
        self.h.inject([AppEv::Tui(TuiSettings {
            rendering: Rendering {
                mermaid: rendering.mermaid,
                math: rendering.math,
                tables: rendering.tables,
                lists: rendering.lists,
            },
            ..TuiSettings::default()
        })]);
        self
    }
}
