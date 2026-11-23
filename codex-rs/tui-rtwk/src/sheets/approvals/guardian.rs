//! The auto-reviewer's words (the oracle's `auto_review_denials::action_summary`, the guardian
//! arms of `chatwidget/tool_requests.rs` and `PendingGuardianReviewStatus`): what a review is
//! about, in the one line the `/auto-review` rows and the status line show, and the status the
//! reviews under way give the Working row.
//!
//! The model (core's `approvals::guardian`) says WHAT is reviewed; this file spells it with the
//! locale's table. Nothing here lays out or cuts a row: the widgets that show these strings wrap
//! and ellipsize them.

use codex_tui_core::screens::approvals::decided::Reviewed;
use codex_tui_core::screens::approvals::guardian::{Action, Reviewing};

use super::decision_cell::snippet;
use super::strings::Strings;

/// The most reviews the status line lists before it says how many more.
const LISTED_REVIEWS: usize = 3;
/// The detail lines of the status line: one review, or the list.
const ONE_REVIEW_LINES: usize = 1;
const MANY_REVIEWS_LINES: usize = 4;

/// What the status line says while reviews run: the header, the detail under it and how many
/// lines of detail the Working row keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewStatus {
    pub header: String,
    pub details: String,
    pub max_lines: usize,
}

/// One review as the `/auto-review` rows say it (`action_summary`).
pub fn summary(s: &Strings, action: &Action) -> String {
    match action {
        Action::Command { line } => line.clone(),
        Action::WriteStdin { process, input } => send_input(s, process, input),
        Action::ApplyPatch { files } => match files.as_slice() {
            [file] => s.sum_patch_one(file).to_string(),
            _ => s.sum_patch_many(files.len()).to_string(),
        },
        Action::NetworkAccess { target } => s.sum_network(target).to_string(),
        Action::McpToolCall {
            server,
            tool,
            connector,
        } => s
            .sum_mcp(tool, connector.as_deref().unwrap_or(server))
            .to_string(),
        Action::RequestPermissions { reason } => match reason {
            Some(reason) => s.with_reason(s.sum_permission, reason).to_string(),
            None => s.sum_permission.to_string(),
        },
    }
}

/// One review as the status line says it (`guardian_action_summary`): the same, but a permission
/// request names its reason only when it has one besides blanks.
fn detail(s: &Strings, action: &Action) -> String {
    match action {
        Action::RequestPermissions { reason } => {
            permission_request(s, s.sum_permission, reason.as_deref())
        }
        _ => summary(s, action),
    }
}

/// `permission_request_summary`: the subject, and the reason after it when it is not blank.
fn permission_request(s: &Strings, subject: &str, reason: Option<&str>) -> String {
    match reason.map(str::trim).filter(|reason| !reason.is_empty()) {
        Some(reason) => s.with_reason(subject, reason).to_string(),
        None => subject.to_string(),
    }
}

/// `send input to terminal 42: "confirm\n"`: the input as Rust quotes it, at most eighty graphemes.
fn send_input(s: &Strings, process: &str, input: &str) -> String {
    s.act_send_input(process, &snippet(s, &format!("{input:?}")))
        .to_string()
}

/// What the history cell of a verdict says was judged, after `codex to ` (denied) or
/// `codex could ` (timed out): the actions that are not a command or a patch. None for a patch,
/// which has its own sentence.
pub(super) fn judged(s: &Strings, about: &Reviewed) -> Option<String> {
    Some(match about {
        Reviewed::Patch(_) => return None,
        Reviewed::Stdin { process, input } => send_input(s, process, input),
        Reviewed::Mcp { server, tool } => s.act_call_mcp(server, tool).to_string(),
        Reviewed::Network(target) => s.act_access(target).to_string(),
        Reviewed::Permissions(reason) => {
            permission_request(s, s.act_request_permissions, reason.as_deref())
        }
    })
}

/// The status the reviews under way give (`status_indicator_state`): none when none runs.
pub fn review_status(s: &Strings, reviews: &[Reviewing]) -> Option<ReviewStatus> {
    match reviews {
        [] => None,
        [one] => Some(ReviewStatus {
            header: s.review_one.to_string(),
            details: detail(s, &one.action),
            max_lines: ONE_REVIEW_LINES,
        }),
        many => {
            let mut lines: Vec<String> = many
                .iter()
                .take(LISTED_REVIEWS)
                .map(|review| s.review_bullet(&detail(s, &review.action)).to_string())
                .collect();
            if many.len() > LISTED_REVIEWS {
                lines.push(s.review_more(many.len() - LISTED_REVIEWS).to_string());
            }
            Some(ReviewStatus {
                header: s.review_many(many.len()).to_string(),
                details: lines.join("\n"),
                max_lines: MANY_REVIEWS_LINES,
            })
        }
    }
}
