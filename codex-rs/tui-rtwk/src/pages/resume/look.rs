//! What the picker draws with: the styles of its rows and chrome, from the
//! theme below it (`Theme::of`) and the terminal's own background, as a
//! Flutter widget reads `Theme.of(context)`. Oracle: `resume_picker.rs`
//! (`secondary_text_style`, `selected_session_style`, `dense_zebra_style`,
//! `toolbar_value`) and `style.rs` (`selection_style`).

use rtwk::Cx;
use rtwk::style::{Attrs, Style, Theme};

use crate::app::theme::{ThreadPalette, Tokens};

/// The styles of one frame of the picker.
#[derive(Clone, Debug)]
pub struct Look {
    pub dim: Style,
    pub italic: Style,
    pub bold: Style,
    pub accent: Style,
    pub error: Style,
    /// The unread dot.
    pub attention: Style,
    /// The selected row's fill, and the focused toolbar value's.
    pub selected: Style,
    pub tab: Style,
    /// The toolbar value that is on while the control has no focus.
    pub current: Style,
    /// Every other row's faint stripe.
    pub zebra: Style,
    /// The box over the list while a transcript loads.
    pub overlay: Style,
    threads: ThreadPalette,
}

impl Look {
    pub fn of<S>(cx: &mut Cx<'_, S>) -> Look {
        let tokens = Tokens::of(cx);
        let data = Theme::of(cx);
        // A tint of the terminal's own background, as the oracle blends one
        // (`dense_row_background_style`, `transcript_loading_overlay_style`).
        let zebra = tokens.tint(cx, 0.055, 0.04);
        let overlay = tokens.tint(cx, 0.14, 0.08);
        Look {
            dim: tokens.dim(),
            italic: tokens.dim().attrs(Attrs::ITALIC),
            bold: tokens.bold(),
            accent: tokens.accent(),
            error: tokens.error(),
            attention: tokens.attention(),
            selected: data.selection.clone(),
            tab: data.selection.clone().attrs(Attrs::BOLD),
            current: tokens.prompt_surface(cx, false).attrs(Attrs::BOLD),
            zebra,
            overlay,
            threads: ThreadPalette::of(&data),
        }
    }

    /// `base` in the colour of the thread `id`.
    pub fn thread(&self, base: &Style, id: &str) -> Style {
        match self.threads.color(id) {
            Some(color) => base.clone().fg(color),
            None => base.clone(),
        }
    }
}
