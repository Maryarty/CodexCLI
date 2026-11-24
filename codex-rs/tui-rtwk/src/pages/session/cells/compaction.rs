//! The line a finished context compaction leaves (oracle `chatwidget/compaction.rs`
//! `on_context_compaction_completed`): `Context compacted`, and `· 1m 03s` after it when the
//! compaction ran live here (`Notice::ContextCompacted::took`); one that came with history says
//! no time. The status row's header and timer are `activity.rs`'s.

use super::strings::Strings;

/// The words after the bullet.
pub fn line(took: Option<u64>, strings: &Strings) -> String {
    match took {
        Some(seconds) => strings.label(
            strings.context_compacted_in,
            &super::super::activity::compact_elapsed(seconds),
        ),
        None => strings.context_compacted.to_owned(),
    }
}
