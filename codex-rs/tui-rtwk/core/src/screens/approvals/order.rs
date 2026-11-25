//! Which queued request is shown next (the oracle's view stack, `bottom_pane/mod.rs`
//! 2085-2130, and the queue each view keeps).
//!
//! The oracle opens one view per kind of request: a request whose kind is not that of the view on
//! top is a new view over it, one of the same kind joins the view's queue. The queue of
//! approvals is a `Vec` popped from the end (`approval_overlay.rs` `advance_queue`: the newest
//! comes next); forms and questions are first in, first out. The requests that are queued in
//! arrival order show that: the view on top is the last run of one kind.

use super::Prompt;

/// The kind of view a request opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Commands, file changes and permissions: one overlay.
    Approval,
    Elicitation,
    UserInput,
    UserVerification,
    /// An app link: a view of its own over the others, the newest on top.
    AppLink,
    /// A request with no prompt (the dismiss fallback).
    Other,
}

impl Family {
    pub fn of(prompt: Option<&Prompt>) -> Family {
        match prompt {
            Some(Prompt::Exec(_) | Prompt::Patch(_) | Prompt::Permissions(_)) => Family::Approval,
            Some(Prompt::Elicitation(_)) => Family::Elicitation,
            Some(Prompt::UserInput(_)) => Family::UserInput,
            Some(Prompt::UserVerification(_)) => Family::UserVerification,
            Some(Prompt::AppLink(_)) => Family::AppLink,
            None => Family::Other,
        }
    }
}

/// The request to show when none is open: `queue` is the requests in arrival order. The last
/// run of one kind is the view on top; approvals of that run come newest first, the others
/// oldest first.
pub fn to_show<K>(queue: &[(K, Family)]) -> Option<&K> {
    let (_, family) = queue.last()?;
    let start = queue
        .iter()
        .rposition(|(_, other)| other != family)
        .map_or(0, |before| before + 1);
    let run = &queue[start..];
    let (key, _) = match family {
        Family::Approval | Family::AppLink => run.last()?,
        _ => run.first()?,
    };
    Some(key)
}
