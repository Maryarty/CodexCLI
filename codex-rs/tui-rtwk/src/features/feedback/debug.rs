//! What the debug menu lists for the flow: each step on a state worth opening, made by a
//! fixture script (`fixtures/`).

use rtwk::AnyWidget;

use codex_tui_core::features::feedback::{Audience, Note};
use codex_tui_core::screens::dialogs::FeedbackCategory;

use super::cell;
use super::sheet::FeedbackSheet;
use super::steps::Step;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};

/// An entry that opens the sheet at `step`, the note starting with `text`.
macro_rules! entry {
    ($name:literal, $description:literal, $fixture:literal, $step:expr, $text:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $fixture)),
            open: EntryOpen::Pane(|_| {
                FeedbackSheet {
                    start: $step,
                    text: $text,
                    turn_id: None,
                }
                .into()
            }),
        }
    };
}

/// An entry that shows the success cell of a delivered report (a transcript row), over the menu.
macro_rules! success {
    ($name:literal, $category:ident, $logs:literal, $report:literal, $audience:ident) => {
        DebugEntry {
            name: $name,
            description: "the cell of a delivered report",
            script: include_str!("fixtures/categories.jsonl"),
            open: EntryOpen::Route(|_| {
                cell::of(&Note::Uploaded {
                    category: FeedbackCategory::$category,
                    include_logs: $logs,
                    report: $report.to_owned(),
                    audience: Audience::$audience,
                })
                .unwrap_or_default()
            }),
        }
    };
}

/// The note step of `category`, with the logs.
macro_rules! note {
    ($category:ident) => {
        Step::Note {
            category: FeedbackCategory::$category,
            include_logs: true,
        }
    };
    ($category:ident, no_logs) => {
        Step::Note {
            category: FeedbackCategory::$category,
            include_logs: false,
        }
    };
}

pub fn entries() -> Vec<DebugEntry> {
    vec![
        entry!(
            "feedback_categories",
            "how was this?",
            "categories.jsonl",
            Step::Category,
            ""
        ),
        entry!(
            "feedback_consent",
            "include logs and session history?",
            "consent.jsonl",
            Step::Consent(FeedbackCategory::Bug),
            ""
        ),
        entry!(
            "feedback_consent_windows",
            "include logs, with the Windows sandbox log",
            "consent_windows.jsonl",
            Step::Consent(FeedbackCategory::Bug),
            ""
        ),
        entry!(
            "feedback_note_bug",
            "the note of a bug, uploads on",
            "note.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_safety_check",
            "the note of a safety check",
            "note.jsonl",
            note!(SafetyCheck),
            ""
        ),
        entry!(
            "feedback_note_bug_no_logs",
            "the note of a bug, without the logs",
            "note.jsonl",
            note!(Bug, no_logs),
            ""
        ),
        entry!(
            "feedback_note_bug_external",
            "the note of a bug, an external user",
            "note.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_bug_employee",
            "the note of a bug, an employee",
            "note_employee.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_bug_remote",
            "the note of a bug over a remote workspace",
            "note_remote.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_employee_40x12",
            "the note of an employee, for a 40x12 screen",
            "note_employee.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_employee_80x8",
            "the note of an employee, for an 80x8 screen",
            "note_employee.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_employee_80x4",
            "the note of an employee, for an 80x4 screen",
            "note_employee.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_safety_check_employee_24x4",
            "the note of a safety check, an employee, for a 24x4 screen",
            "note_employee.jsonl",
            note!(SafetyCheck),
            ""
        ),
        entry!(
            "feedback_note_safety_check_external_24x4",
            "the note of a safety check, an external user, for a 24x4 screen",
            "note.jsonl",
            note!(SafetyCheck),
            ""
        ),
        entry!(
            "feedback_note_good_result",
            "the note of a good result, empty",
            "note.jsonl",
            note!(GoodResult),
            ""
        ),
        entry!(
            "feedback_note_unfinished",
            "the note of a bug with unfinished text",
            "note.jsonl",
            note!(Bug),
            "unfinished feedback"
        ),
        entry!(
            "feedback_note_remote_save",
            "the note over a remote workspace with a padded text",
            "note_remote.jsonl",
            note!(Bug),
            "  something broke  "
        ),
        entry!(
            "feedback_note_bug_turn_no_uploads",
            "the note with uploads disabled",
            "note_no_uploads.jsonl",
            note!(Bug),
            ""
        ),
        entry!(
            "feedback_note_bug_turn_external",
            "the note of a bug in a turn, an external user",
            "note.jsonl",
            note!(Bug),
            ""
        ),
        success!("feedback_success_external_bug", Bug, true, "thread-1", External),
        success!("feedback_success_employee_bug", Bug, true, "thread-2", OpenAiEmployee),
        success!("feedback_success_good_result", GoodResult, false, "thread-3", External),
        success!("feedback_success_bad_result", BadResult, true, "thread-4", External),
        success!("feedback_success_safety_check", SafetyCheck, true, "thread-4", External),
        success!("feedback_success_other", Other, true, "thread-4", External),
    ]
}

/// The flow with no seeded model (the directory's row): nothing to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
