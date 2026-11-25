//! Positive eligibility for the fresh-conversation decoration (oracle
//! `empty_state_animation/policy.rs` and `app/empty_state_policy.rs`): startup metadata is the
//! whole allowlist; every other entry of the transcript is activity, even one that draws nothing,
//! and ends the welcome for the thread.

use codex_tui_core::threads::{Item, Notice, Transcript};

/// Whether `item` is startup metadata (oracle `is_startup_cell`: the session card, the startup
/// warnings, a deprecation notice, an available update, the server version warning and the
/// session notice of a usage-limit reset). The session header is not an entry: the transcript
/// draws it from the thread's facts.
pub fn is_startup(item: &Item) -> bool {
    matches!(
        item,
        Item::StartupWarning(_)
            | Item::Deprecation { .. }
            | Item::Notice(
                Notice::SessionInfo(_)
                    | Notice::UpdateAvailable { .. }
                    | Notice::ServerVersion { .. }
                    | Notice::ResetAvailable { .. }
            )
    )
}

/// The transcript holds startup metadata and nothing else.
pub fn startup_only(transcript: &Transcript) -> bool {
    transcript.iter().all(|(_, item)| is_startup(item))
}
