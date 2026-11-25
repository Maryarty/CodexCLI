//! What a draft confirmed before the session started waits for (oracle `chatwidget/
//! startup_submission.rs`): the pure rules of the page's confirmation, and the model gate a
//! submit and the queue's drain both go through (`submit_user_message`,
//! `maybe_send_next_queued_input`). The draft itself is the page's (ADR-003): nothing reaches the
//! thread until the draft is submitted.

use crate::app::{Session, SessionSlice};
use crate::commands::parse_slash_name;

use super::{ThreadModel, is_start_turn};

/// The thread's model is known to be empty (`model().trim().is_empty()`): the server named none.
/// A thread that was never told its model (`None`) is not refused: the config's model stands in.
pub fn model_unavailable(model: Option<&str>) -> bool {
    model.is_some_and(|model| model.trim().is_empty())
}

/// Enter on the startup draft confirms `line` (the draft as it would be submitted) when it is
/// text for the model: a blank draft is nothing to confirm, and a slash command is run at once
/// (the commands that cannot run before the session say so).
pub fn confirms(line: &str) -> bool {
    !line.trim().is_empty()
        && !parse_slash_name(line).is_some_and(|(name, _, _)| !name.contains('/'))
}

/// The thread takes the confirmed draft now (the thread's half of
/// `restore_startup_input_when_ready`): its model is not known to be empty and no turn is
/// pending or running, no plan streams, nothing holds the queue.
pub fn thread_idle(thread: &ThreadModel) -> bool {
    !model_unavailable(thread.slots.chat.model.as_deref())
        && !thread.busy()
        && !thread.slots.chat.review_mode()
        && !(thread.slots.chat.plan.streaming() && thread.is_running())
        && !thread.outgoing.any(is_start_turn)
        && !thread.stream_lost
        && !thread.recovered
        && thread.pending_steers.is_empty()
        && thread.rejected_steers.is_empty()
        && !thread.safety.blocks_input()
}

/// The session is configured and its thread takes the confirmed draft.
pub fn confirm_ready(session: &SessionSlice, thread: &ThreadModel) -> bool {
    session.status == Session::Ready && thread_idle(thread)
}
