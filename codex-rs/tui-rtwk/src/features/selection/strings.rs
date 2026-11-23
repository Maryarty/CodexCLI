//! The words of the selection's copy feedback (oracle `composer_gap.rs` `CopyFeedback::line`):
//! the composer gap shows the longest of three spellings that fits (the oracle's English
//! until the catalog has the entries; a typed const table, ADR-010).

use codex_tui_core::app::effects::CopyStatus;

/// The three spellings of how the copy of `characters` characters went, longest first.
pub fn feedback(result: Result<CopyStatus, ()>, characters: usize) -> [String; 3] {
    match result {
        Ok(CopyStatus::Confirmed) => [
            format!("Copied {characters} chars to host clipboard"),
            format!("Copied {characters} chars"),
            "Copied".into(),
        ],
        Ok(CopyStatus::Pending(_)) => [
            format!("Copying {characters} chars…"),
            "Copying…".into(),
            "Copying…".into(),
        ],
        Ok(CopyStatus::Busy) => [
            "Copy already in progress".into(),
            "Copy in progress".into(),
            "Copy busy".into(),
        ],
        Ok(CopyStatus::Unconfirmed) => [
            "Copy sent to terminal · paste to verify".into(),
            "Copy sent · verify paste".into(),
            "Copy unconfirmed".into(),
        ],
        Err(()) => [
            "Copy failed · try again".into(),
            "Copy failed".into(),
            "Copy failed".into(),
        ],
    }
}
