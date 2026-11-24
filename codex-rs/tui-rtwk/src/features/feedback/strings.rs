//! The words of `/feedback` (oracle `bottom_pane/feedback_view.rs` `feedback_success_cell`,
//! `issue_url_for_category`, `chatwidget.rs` `open_feedback`, `app/background_requests.rs`
//! `handle_feedback_thread_event`): its transcript notes and the copy of the success cell. The
//! wording is the oracle's English until the catalog has these entries.
//!
//! The success cell is several lines in the oracle (a bullet, the link, the ids), with the links
//! drawn as hyperlinks: the note's words are the lines joined by `\n` without the bullet or the
//! two-space hang, which the cell that draws them adds.

use codex_tui_core::features::feedback::{Audience, Note};
use codex_tui_core::screens::dialogs::FeedbackCategory;

const BASE_CLI_BUG_ISSUE_URL: &str =
    "https://github.com/openai/codex/issues/new?template=3-cli.yml";
/// Internal routing link for employee feedback follow-ups: never shown to external users.
const CODEX_FEEDBACK_INTERNAL_URL: &str = "http://go/codex-feedback-internal";

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Unavailable => {
            "Feedback uploads are disabled, and saving a ZIP requires a local app server."
                .to_owned()
        }
        Note::ZipNeedsLocalServer => "Saving a feedback ZIP requires a local app server.".to_owned(),
        Note::Saved(path) => format!("Feedback ZIP saved to {path}"),
        Note::Failed(error) => error.clone(),
        Note::Uploaded {
            category,
            include_logs,
            thread_id,
            audience,
        } => success(*category, *include_logs, thread_id, *audience).join("\n"),
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
            format!("Sentry URL: https://go/codex-feedback/{thread_id}"),
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
