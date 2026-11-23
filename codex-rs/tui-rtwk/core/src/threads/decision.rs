//! What answers and auto-reviews leave in a thread's history: the decision cells and the lines
//! `/auto-review` and the reviewer's warnings add (oracle `history_cell/approvals.rs`,
//! `chatwidget/approvals.rs`). The approvals slot (`screens::approvals::ThreadPart`) says what
//! should be appended (`History`, no prose); the thread appends it once, in order, after every
//! event it applied (ADR-003: one writer, the model's own `apply`). The words are the approvals
//! string table's.

use crate::screens::approvals::guardian::History;

use super::{Item, cells::Notice};

/// A line of the history `/auto-review` and the reviewer's paths leave besides a decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewLine {
    /// `/auto-review` with no denial kept (an info line with a dim detail).
    NoDenials,
    /// One retry of a denial was approved (an info line with a dim detail).
    RetryApproved,
    /// The denial picked is no longer kept (an error line).
    DenialGone,
    /// The server's file paths of a review could not be localized: the text of its error (an
    /// error line).
    Unlocalized(String),
}

/// The entry of the history that `history` stands for.
pub(super) fn item_of(history: History) -> Item {
    match history {
        History::Decided(decided) => Item::Decision(decided),
        // The oracle's overlay only submits answers, never an interruption: the flag stays false.
        History::Questioned(questions) => Item::Notice(Notice::Questions {
            questions,
            interrupted: false,
        }),
        // The reviewer's warning is the oracle's `on_warning`: a detail of the long transcript.
        History::Warning(message) => Item::Warning { message },
        History::Unlocalized(error) => Item::Review(ReviewLine::Unlocalized(error)),
        History::NoDenials => Item::Review(ReviewLine::NoDenials),
        History::RetryApproved => Item::Review(ReviewLine::RetryApproved),
        History::DenialGone => Item::Review(ReviewLine::DenialGone),
    }
}
