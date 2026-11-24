//! The words of `/feedback` (oracle `bottom_pane/feedback_view.rs` `feedback_success_cell`,
//! `issue_url_for_category`, `chatwidget.rs` `open_feedback`, `app/background_requests.rs`
//! `handle_feedback_thread_event`): its transcript notes and the copy of the success cell. The
//! wording is the oracle's English until the catalog has these entries.
//!
//! The success cell is several lines in the oracle (a bullet, the link, the ids), with the links
//! drawn as hyperlinks: the note's words are the lines joined by `\n` without the bullet or the
//! two-space hang, which the cell that draws them adds.

use codex_tui_core::features::feedback::{Audience, Note};

use crate::l10n::Lang;
use codex_tui_core::screens::dialogs::FeedbackCategory;

const BASE_CLI_BUG_ISSUE_URL: &str =
    "https://github.com/openai/codex/issues/new?template=3-cli.yml";
/// Internal routing link for employee feedback follow-ups: never shown to external users.
const CODEX_FEEDBACK_INTERNAL_URL: &str = "http://go/codex-feedback-internal";

/// The title of the flow in the debug menu.
pub fn title(_lang: Lang) -> &'static str {
    "Feedback"
}

/// `FEEDBACK_POLICY_URL`: how the data of a report is used (the link of `Learn more`).
pub const POLICY_URL: &str = "https://help.openai.com/en/articles/5722486-how-your-data-is-used-to-improve-model-performance";
/// `EMPLOYEE_FEEDBACK_FAQ_URL`: the link of `here` for an employee.
pub const EMPLOYEE_FAQ_URL: &str = "https://go/codex-employee-feedback-faq";

/// The title of the note step and its placeholder (`feedback_title_and_placeholder`).
pub fn title_and_placeholder(category: FeedbackCategory) -> (&'static str, &'static str) {
    const DESCRIBE: &str = "(optional) Write a short description to help us further";
    match category {
        FeedbackCategory::BadResult => ("Tell us more (bad result)", DESCRIBE),
        FeedbackCategory::GoodResult => ("Tell us more (good result)", DESCRIBE),
        FeedbackCategory::Bug => ("Tell us more (bug)", DESCRIBE),
        FeedbackCategory::SafetyCheck => (
            "Tell us more (safety check)",
            "(optional) Share what was refused and why it should have been allowed",
        ),
        FeedbackCategory::Other => ("Tell us more (other)", DESCRIBE),
    }
}

/// The disclosure of the note step, in paragraphs of pieces: a piece is its words and what it
/// is (`FeedbackNoteView::intro_lines`). Uploads disabled, `audience` is none.
pub fn disclosure(audience: Option<Audience>, can_save: bool) -> Vec<Vec<(&'static str, Piece)>> {
    let mut paragraphs = Vec::new();
    match audience {
        None => paragraphs.push(vec![("Feedback uploads are disabled.", Piece::Dim)]),
        Some(Audience::OpenAiEmployee) => {
            paragraphs.push(vec![(
                "Feedback sent to OpenAI may be used to improve our models and products",
                Piece::Bold,
            )]);
            paragraphs.push(vec![
                ("By submitting feedback, you agree that OpenAI can use your feedback for safety purposes and internal model training, as explained in more detail ", Piece::Dim),
                ("here", Piece::Link),
                (". Please avoid sharing tented information under NDA, or privileged, HR-related, or sensitive personal information.", Piece::Dim),
            ]);
        }
        Some(Audience::External) => paragraphs.push(vec![
            (
                "Feedback sent to OpenAI can be used to improve ChatGPT. ",
                Piece::Dim,
            ),
            ("Learn more", Piece::Link),
            (".", Piece::Dim),
        ]),
    }
    if can_save {
        paragraphs.push(vec![(
            if audience.is_some() {
                "You can also save feedback to a ZIP file, which is not sent to OpenAI."
            } else {
                "You can save feedback to a ZIP file, which is not sent to OpenAI."
            },
            Piece::Dim,
        )]);
    }
    paragraphs
}

/// What a piece of the disclosure is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Dim,
    Bold,
    /// The link of the audience: the accent, underlined.
    Link,
}

/// The words of the note step's keys, from the longest to the shortest spelling of the row: the
/// keys the view offers are `(key, words)` of each (`save`, `send`, `cancel`).
pub struct Hints {
    pub save: &'static str,
    pub send: &'static str,
    pub cancel: &'static str,
}

pub const HINTS_LONG: Hints = Hints {
    save: "save ZIP",
    send: "send to OpenAI",
    cancel: "cancel",
};
pub const HINTS_SHORT: Hints = Hints {
    save: "save",
    send: "send",
    cancel: "cancel",
};
/// The line over the hints when the disclosure scrolls.
pub const SCROLL_HINT: (&str, &str) = ("pgup/pgdn", " scroll disclosure");

/// The label of the Sentry link line of the employee success cell.
pub const SENTRY_LABEL: &str = "Sentry URL: ";

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Unavailable => {
            "Feedback uploads are disabled, and saving a ZIP requires a local app server."
                .to_owned()
        }
        Note::ZipNeedsLocalServer => {
            "Saving a feedback ZIP requires a local app server.".to_owned()
        }
        Note::Saved(path) => format!("Feedback ZIP saved to {path}"),
        Note::Failed(error) => error.clone(),
        Note::Uploaded {
            category,
            include_logs,
            report,
            audience,
        } => success(*category, *include_logs, report, *audience).join("\n"),
    }
}

/// The follow-up link of a category (`issue_url_for_category`): a good result has none.
pub fn issue_url(
    category: FeedbackCategory,
    thread_id: &str,
    audience: Audience,
) -> Option<String> {
    match category {
        FeedbackCategory::GoodResult => None,
        FeedbackCategory::Bug
        | FeedbackCategory::BadResult
        | FeedbackCategory::SafetyCheck
        | FeedbackCategory::Other => Some(match audience {
            Audience::OpenAiEmployee => CODEX_FEEDBACK_INTERNAL_URL.to_owned(),
            Audience::External => {
                format!("{BASE_CLI_BUG_ISSUE_URL}&steps=Uploaded%20thread:%20{thread_id}")
            }
        }),
    }
}

/// The lines of the success cell (`feedback_success_cell`), after the bullet.
pub fn success(
    category: FeedbackCategory,
    include_logs: bool,
    thread_id: &str,
    audience: Audience,
) -> Vec<String> {
    let prefix = if include_logs {
        "Feedback uploaded."
    } else {
        "Feedback recorded (no logs)."
    };
    let url = issue_url(category, thread_id, audience);
    let head = match (&url, audience) {
        (Some(_), Audience::OpenAiEmployee) => {
            format!("{prefix} You can share this in #codex-feedback:")
        }
        (Some(_), Audience::External) => {
            format!("{prefix} Please open an issue using the following URL:")
        }
        (None, _) => format!("{prefix} Thanks for the feedback!"),
    };
    let mut lines = vec![head, String::new()];
    match (url, audience) {
        (Some(url), Audience::OpenAiEmployee) => lines.extend([
            url,
            String::new(),
            format!("Sentry Feedback ID: {thread_id}"),
            format!("{SENTRY_LABEL}https://go/codex-feedback/{thread_id}"),
        ]),
        (Some(url), Audience::External) => lines.extend([
            url,
            String::new(),
            format!("Or mention your thread ID {thread_id} in an existing issue."),
        ]),
        (None, _) => lines.push(format!("Thread ID: {thread_id}")),
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_external_bug_copy_is_the_oracles() {
        let lines = success(FeedbackCategory::Bug, true, "thread-1", Audience::External);
        assert_eq!(
            lines.join("\n"),
            "Feedback uploaded. Please open an issue using the following URL:\n\nhttps://github.com/openai/codex/issues/new?template=3-cli.yml&steps=Uploaded%20thread:%20thread-1\n\nOr mention your thread ID thread-1 in an existing issue."
        );
    }

    #[test]
    fn a_good_result_has_no_link() {
        let lines = success(
            FeedbackCategory::GoodResult,
            false,
            "thread-3",
            Audience::External,
        );
        assert_eq!(
            lines.join("\n"),
            "Feedback recorded (no logs). Thanks for the feedback!\n\nThread ID: thread-3"
        );
    }

    #[test]
    fn an_employee_is_routed_to_the_internal_link() {
        assert_eq!(
            issue_url(FeedbackCategory::Other, "t", Audience::OpenAiEmployee).as_deref(),
            Some("http://go/codex-feedback-internal")
        );
    }
}
