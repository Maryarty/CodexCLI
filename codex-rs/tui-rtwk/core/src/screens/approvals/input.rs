//! The answers of a `request_user_input` request (`item/tool/requestUserInput`): what a draft
//! of one question holds, when a question counts as answered and what the whole answer sends.
//! Pure data and logic, no rtwk, no words of its own. Carried from the oracle's
//! `bottom_pane/request_user_input/mod.rs` (`is_question_answered`, `unanswered_count`,
//! `submit_answers`).

use std::collections::BTreeMap;

use super::{Question, RequestAnswer};

/// The answer a choice of the "other" row sends (the oracle's `OTHER_OPTION_LABEL`: the model
/// reads this text on the wire, the sheet shows its own words on the row).
pub const OTHER_ANSWER: &str = "None of the above";

/// How a note follows the chosen option in the answer list (`user_note: ...`).
const NOTE_PREFIX: &str = "user_note: ";

/// What the user has done on one question: the option the selection is on, whether that
/// answer was committed (Enter, Space, a digit), and the notes (or the freeform answer) typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    pub selected: Option<usize>,
    pub committed: bool,
    pub notes: String,
}

/// Whether the question offers options (a question without options is freeform).
pub fn has_options(question: &Question) -> bool {
    !question.options.is_empty()
}

/// Whether the extra "other" row follows the options.
pub fn other_enabled(question: &Question) -> bool {
    question.is_other && has_options(question)
}

/// How many rows the options list has: the options and the other row.
pub fn choices(question: &Question) -> usize {
    question.options.len() + usize::from(other_enabled(question))
}

/// The text the answer list carries for the option at `index`.
pub fn label(question: &Question, index: usize) -> Option<String> {
    if let Some(option) = question.options.get(index) {
        return Some(option.label.clone());
    }
    (index == question.options.len() && other_enabled(question)).then(|| OTHER_ANSWER.to_string())
}

/// An option question is answered by a committed selection; a freeform question by a committed
/// non-empty text.
pub fn answered(question: &Question, draft: &Draft) -> bool {
    if has_options(question) {
        draft.selected.is_some() && draft.committed
    } else {
        draft.committed
    }
}

/// How many questions would send an empty answer list.
pub fn unanswered(questions: &[Question], drafts: &[Draft]) -> usize {
    questions
        .iter()
        .zip(drafts)
        .filter(|(question, draft)| !answered(question, draft))
        .count()
}

/// The first question that is not answered.
pub fn first_unanswered(questions: &[Question], drafts: &[Draft]) -> Option<usize> {
    questions
        .iter()
        .zip(drafts)
        .position(|(question, draft)| !answered(question, draft))
}

/// The answer of a submit: per question the committed option's text, then the committed notes
/// as `user_note: ...`; a question not answered sends an empty list.
pub fn answer(questions: &[Question], drafts: &[Draft]) -> RequestAnswer {
    let answers: BTreeMap<String, Vec<String>> = questions
        .iter()
        .zip(drafts)
        .map(|(question, draft)| {
            let selected = if has_options(question) && draft.committed {
                draft.selected.and_then(|index| label(question, index))
            } else {
                None
            };
            let notes = if draft.committed {
                draft.notes.trim()
            } else {
                ""
            };
            let mut list: Vec<String> = selected.into_iter().collect();
            if !notes.is_empty() {
                list.push(format!("{NOTE_PREFIX}{notes}"));
            }
            (question.id.clone(), list)
        })
        .collect();
    RequestAnswer::UserInput(answers)
}
