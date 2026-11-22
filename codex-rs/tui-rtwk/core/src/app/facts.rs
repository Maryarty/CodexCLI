//! The `[tui]` settings of the config that views read, kept by the app as one value (oracle
//! `local_settings.tui`): the host reads the config once and sends `AppEv::Tui`; a settings toggle
//! that succeeds sends the changed value again. Until the host speaks, the defaults of the
//! config hold.

/// The `[tui]` settings that change what a view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TuiSettings {
    /// `tui.collapse_turn_details`: the tool calls of a finished turn show as one summary line in
    /// the transcript view until the user opens them (oracle `transcript_view`
    /// `collapse_turn_details`).
    pub collapse_turn_details: bool,
}
