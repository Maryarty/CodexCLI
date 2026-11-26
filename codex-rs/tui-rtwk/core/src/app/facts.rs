//! The `[tui]` settings of the config that views read, kept by the app as one value (oracle
//! `local_settings.tui`): the host reads the config once and sends `AppEv::Tui`; a settings toggle
//! that succeeds sends the changed value again. Until the host speaks, the defaults of the
//! config hold.

/// `tui.rendering`: which rich renderers the Markdown views use (oracle `TuiRendering`); a
/// disabled one keeps the source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rendering {
    pub mermaid: bool,
    pub math: bool,
    pub tables: bool,
    pub lists: bool,
}

impl Default for Rendering {
    fn default() -> Self {
        Self {
            mermaid: true,
            math: true,
            tables: true,
            lists: true,
        }
    }
}

/// The `[tui]` settings that change what a view shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuiSettings {
    /// `tui.collapse_turn_details`: the tool calls of a finished turn show as one summary line in
    /// the transcript view until the user opens them (oracle `transcript_view`
    /// `collapse_turn_details`).
    pub collapse_turn_details: bool,
    /// `tui.mouse_scroll_speed`: rows per wheel event, including fractional rows.
    /// Without an explicit speed the reader uses rtwk's native scroll policy.
    pub wheel_rows_per_event: Option<f64>,
    /// `tui.show_tooltips`: the startup tooltip under the welcome (default on).
    pub show_tooltips: bool,
    /// `tui.rendering`.
    pub rendering: Rendering,
    /// `tui.copy_on_select` resolved for this terminal (oracle `local_settings.copy_on_select`):
    /// a selection is copied when the mouse is released (default on; `auto` turns it off only
    /// for a direct terminal known to forward its own copy shortcut).
    pub copy_on_select: bool,
    /// The host can publish local X11 PRIMARY independently of the clipboard.
    pub primary_selection: bool,
    /// `tui.copy_on_select_clear_selection`: a selection is let go once its copy is confirmed
    /// (default off).
    pub deselect_after_copy: bool,
    /// `tui.vim_mode_default`: a new draft starts in Vim Insert mode (oracle `constructor`
    /// `enable_vim_in_insert_mode`).
    pub vim_mode_default: bool,
    /// `notice.hide_rate_limit_model_nudge`: the rate limit prompt that offers a cheaper model
    /// is not shown (oracle `local_settings.notices`).
    pub hide_rate_limit_model_nudge: bool,
    /// The chat owns its transcript (oracle `local_settings.transcript_mode.is_owned()`:
    /// `tui.fullscreen_transcript` on an alternate screen): `/copy` begins copy mode over it
    /// instead of opening the picker. Off is the oracle's Terminal mode, where the history lives
    /// in the terminal's scrollback and the picker is the way.
    pub transcript_owned: bool,
}

impl Default for TuiSettings {
    fn default() -> Self {
        Self {
            collapse_turn_details: false,
            wheel_rows_per_event: None,
            show_tooltips: true,
            rendering: Rendering::default(),
            copy_on_select: true,
            primary_selection: false,
            deselect_after_copy: false,
            vim_mode_default: false,
            hide_rate_limit_model_nudge: false,
            transcript_owned: false,
        }
    }
}
