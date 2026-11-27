//! The transcript entries beside messages and tool calls: reasoning, patches,
//! plans, web activity, viewed images and session notices. Each is plain data
//! projected once from a protocol item here (R4); views get these values and
//! never the protocol. Oracle: `codex-rs/tui/src/history_cell/` (messages.rs
//! reasoning, patches.rs, plans.rs, search.rs, notices.rs).

use codex_app_server_protocol::{
    HookOutputEntryKind, HookRunStatus, HookRunSummary, PatchApplyStatus, PatchChangeKind,
    ThreadItemPayload, TurnPlanStepStatus, TurnPlanUpdatedNotification,
};

use super::ToolStatus;
use crate::ids::ItemId;

/// A set of file changes the agent applied (oracle `PatchHistoryCell`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchActivity {
    pub changes: Vec<PatchChange>,
    pub status: ToolStatus,
    pub source: PatchSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchSource {
    Live,
    History,
}

impl PatchActivity {
    /// A stored failure records the outcome without reporting the attempted edits as applied.
    pub fn recorded_failure(&self) -> bool {
        self.source == PatchSource::History && self.status == ToolStatus::Failed
    }
}

/// One file of a patch: its path, what happened to it and its diff (the
/// file's content for an added or deleted file, a unified diff otherwise).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchChange {
    pub path: String,
    pub kind: PatchKind,
    pub diff: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchKind {
    Add,
    Delete,
    Update { move_path: Option<String> },
}

impl PatchChange {
    /// Lines added and removed (oracle `diff_render::line_counts`).
    pub fn counts(&self) -> (usize, usize) {
        match self.kind {
            PatchKind::Add => (self.diff.lines().count(), 0),
            PatchKind::Delete => (0, self.diff.lines().count()),
            PatchKind::Update { .. } => {
                // oracle diff_render.rs calculate_add_remove_from_diff
                let mut added = 0;
                let mut removed = 0;
                for line in self.diff.lines() {
                    if line.starts_with("+++") || line.starts_with("---") {
                        continue;
                    }
                    if line.starts_with('+') {
                        added += 1;
                    } else if line.starts_with('-') {
                        removed += 1;
                    }
                }
                (added, removed)
            }
        }
    }
}

/// A web action of the model (oracle `WebSearchCell`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebActivity {
    pub completed: bool,
    pub action: WebAction,
    /// What the action was about: the query, the page or `'pattern' in url`;
    /// empty when the server sent nothing.
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebAction {
    /// A search; `known` is false for an action of no known kind or none
    /// sent yet (the oracle then reads `Browsing the web` while it runs).
    Search {
        known: bool,
    },
    OpenPage,
    FindInPage {
        pattern: bool,
    },
}

/// The plan the model reports with `update_plan` (oracle `PlanUpdateCell`):
/// the note and the steps, one entry of the transcript per notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanUpdate {
    pub explanation: Option<String>,
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanStep {
    pub text: String,
    pub status: StepStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
}

impl PlanUpdate {
    pub(super) fn from_notification(note: &TurnPlanUpdatedNotification) -> Self {
        Self {
            explanation: note.explanation.clone(),
            steps: note
                .plan
                .iter()
                .map(|step| PlanStep {
                    text: step.step.clone(),
                    status: match step.status {
                        TurnPlanStepStatus::Pending => StepStatus::Pending,
                        TurnPlanStepStatus::InProgress => StepStatus::InProgress,
                        TurnPlanStepStatus::Completed => StepStatus::Completed,
                    },
                })
                .collect(),
        }
    }

    /// The note, trimmed; a blank one is none.
    pub fn note(&self) -> Option<&str> {
        self.explanation
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
    }

    /// The steps done.
    pub fn completed(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == StepStatus::Completed)
            .count()
    }
}

/// A completed hook run worth a row (oracle `HookCell`): how it ended and
/// the output it has for the user. The model-facing context is dropped here,
/// as the oracle never draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookRun {
    pub status: HookStatus,
    pub entries: Vec<HookEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookStatus {
    Completed,
    Failed,
    Blocked,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookEntry {
    pub kind: HookEntryKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookEntryKind {
    Warning,
    Stop,
    Feedback,
    Error,
}

impl HookRun {
    /// The row of a finished run; a run still running and a quiet success
    /// (completed with nothing but context) leave none (oracle
    /// `hook_run_is_quiet_success`, `add_completed_run`).
    pub(super) fn from_summary(run: &HookRunSummary) -> Option<Self> {
        let status = match run.status {
            HookRunStatus::Running => return None,
            HookRunStatus::Completed => HookStatus::Completed,
            HookRunStatus::Failed => HookStatus::Failed,
            HookRunStatus::Blocked => HookStatus::Blocked,
            HookRunStatus::Stopped => HookStatus::Stopped,
        };
        let entries: Vec<HookEntry> = run
            .entries
            .iter()
            .filter_map(|entry| {
                Some(HookEntry {
                    kind: match entry.kind {
                        HookOutputEntryKind::Warning => HookEntryKind::Warning,
                        HookOutputEntryKind::Stop => HookEntryKind::Stop,
                        HookOutputEntryKind::Feedback => HookEntryKind::Feedback,
                        HookOutputEntryKind::Error => HookEntryKind::Error,
                        HookOutputEntryKind::Context => return None,
                    },
                    text: entry.text.clone(),
                })
            })
            .collect();
        (status != HookStatus::Completed || !entries.is_empty()).then_some(Self { status, entries })
    }
}

/// A notice the session leaves in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// A context compaction finished. `took` is the seconds a compaction that ran live here took
    /// (`Compaction::took`); one that came with history has none (oracle
    /// `on_context_compaction_completed`).
    ContextCompacted {
        took: Option<u64>,
    },
    /// The thread is a fork (oracle `emit_forked_thread_event`, `ForkedThreadHistoryCell`):
    /// the thread it was forked from, and that thread's name when it has one.
    ForkedThread {
        from: String,
        title: Option<String>,
    },
    EnteredReview(String),
    ExitedReview(String),
    /// Esc or Ctrl+C ended the turn (`new_interruption_notice`).
    Interrupted,
    /// The turn was interrupted to send the steers that waited
    /// (`Model interrupted to submit steer instructions.`).
    InterruptedForSteer,
    /// The first submission still awaiting an exact receipt after reconnecting.
    SubmissionUnconfirmed(String),
    /// The goal's token budget stopped the turn (`Goal budget reached`).
    GoalBudgetReached,
    /// Esc Esc with no prompt to edit (`NO_PREVIOUS_MESSAGE_TO_EDIT`).
    NoPreviousMessage,
    /// The edit of a past prompt reverted the thread, and the history it kept is shown
    /// (oracle `emit_prompt_edit_thread_event`).
    PromptReverted,
    /// The e-mail to the owner of the workspace that a banner action asked for ended
    /// (`finish_add_credits_nudge_email_request`).
    AddCreditsNudge {
        nudge: crate::screens::chat::Nudge,
        outcome: crate::screens::chat::NudgeOutcome,
    },
    /// The startup check found usage-limit resets to use (oracle
    /// `set_rate_limit_reset_available_hint`): how many.
    ResetAvailable {
        count: i64,
    },
    /// A limit of the account is close (oracle `new_visible_warning_event` of
    /// `RateLimitWarningState`): the share that is left and the window's name.
    RateLimitWarning {
        remaining_percent: i32,
        window: crate::screens::status::WindowName,
    },
    /// A request refused for biology safety (oracle `new_safety_access_block_event`).
    SafetyAccessBlock,
    /// A refused cybersecurity request, worded by the Daybreak setting at the
    /// moment it was refused (oracle `new_cyber_policy_error_event`).
    CyberPolicy(crate::screens::cells::Daybreak),
    /// A newer release exists (oracle `UpdateAvailableHistoryCell`): the
    /// running version, the latest, and the command that updates this
    /// install (`None` when the install is not one the CLI knows).
    UpdateAvailable {
        current: String,
        latest: String,
        command: Option<String>,
    },
    /// The server runs another version than the client (oracle
    /// `new_server_version_warning`): its message, and whether the local
    /// background server can be updated (two more lines).
    ServerVersion {
        message: String,
        offer_update: bool,
    },
    /// A write to (or a wait on) a background terminal (oracle
    /// `UnifiedExecInteractionCell`): the command it runs and what was sent
    /// to its stdin (empty: it only waited). Shown in the detailed transcript
    /// only ([`Notice::detailed_only`]).
    TerminalInteraction {
        command: Option<String>,
        stdin: String,
    },
    /// `/recap` is generating (oracle `ThreadRecapLoadingCell`).
    RecapLoading,
    /// A `request_user_input` exchange ended (oracle
    /// `RequestUserInputResultCell`).
    Questions {
        questions: Vec<crate::screens::cells::AnsweredQuestion>,
        interrupted: bool,
    },
    /// The startup card under the header (oracle `new_session_info`).
    SessionInfo(crate::screens::cells::SessionInfo),
}

impl Notice {
    /// Entries only the detailed transcript (Ctrl+T) shows; the app's
    /// `Item::is_detailed_only` asks.
    pub fn detailed_only(&self) -> bool {
        matches!(self, Self::TerminalInteraction { .. })
    }
}

/// A message another task delegated to this thread (oracle
/// `DelegatedMessageCell`): the task it came from and its Markdown body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegatedMessage {
    pub source: String,
    pub markdown: String,
}

/// A finished image generation call (oracle `ImageGenerationHistoryCell`):
/// the revised prompt (else the call id) and where the image was saved, as
/// the row shows it (a `file://` URL when the path is absolute).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedImage {
    pub failed: bool,
    pub detail: String,
    pub saved_to: Option<String>,
    /// The host lookup uses the original path, independent of its display spelling.
    pub saved_path: Option<String>,
}

/// The namespaces and tools whose output carries a delegated message
/// (oracle `dynamic_tools::parse_delegated_tool_output`).
const DELEGATING_NAMESPACES: [&str; 2] = ["codex_tui", "codex_app"];
const DELEGATING_TOOLS: [&str; 2] = ["create_thread", "send_message_to_thread"];

/// Oracle `parse_delegated_prompt`: the XML-like wrapper the tool wrote.
fn delegated(output: &str) -> Option<DelegatedMessage> {
    let delegation = output.strip_prefix("<codex_delegation>\n  <source_thread_id>")?;
    let (source, body) = delegation.split_once("</source_thread_id>\n  <input>")?;
    let body = body.strip_suffix("</input>\n</codex_delegation>")?;
    let unescape = |value: &str| {
        value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
    };
    Some(DelegatedMessage {
        source: unescape(source),
        markdown: unescape(body),
    })
}

/// `Url::from_file_path` for an absolute Unix path: each segment percent
/// encoded as a URL path segment; any other path is shown as it is.
fn file_url(path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_owned();
    }
    let mut url = String::from("file://");
    for byte in path.bytes() {
        let keep = byte == b'/'
            || (byte.is_ascii() && !byte.is_ascii_control() && !b" \"#%<>?`{}".contains(&byte));
        if keep {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

/// The entries of this module for one protocol item; `completed` tells an
/// `item/completed` (or a stored item) from an `item/started`.
pub(super) fn entry(payload: &ThreadItemPayload, completed: bool) -> Option<(ItemId, super::Item)> {
    use super::Item;
    Some(match payload {
        ThreadItemPayload::Reasoning {
            id,
            summary,
            content,
        } => (
            ItemId::new(id),
            Item::Reasoning {
                summary: summary_text(summary),
                with_raw: (!content.is_empty())
                    .then(|| summary_text(&[summary.as_slice(), content.as_slice()].concat())),
            },
        ),
        ThreadItemPayload::Plan { id, text } => {
            (ItemId::new(id), Item::Plan { text: text.clone() })
        }
        ThreadItemPayload::FileChange {
            id,
            changes,
            status,
        } => (
            ItemId::new(id),
            Item::Patch(std::sync::Arc::new(PatchActivity {
                changes: {
                    let mut changes: Vec<_> = changes
                        .iter()
                        .map(|change| PatchChange {
                            path: change.path.clone(),
                            kind: match &change.kind {
                                PatchChangeKind::Add => PatchKind::Add,
                                PatchChangeKind::Delete => PatchKind::Delete,
                                PatchChangeKind::Update { move_path } => PatchKind::Update {
                                    move_path: move_path
                                        .as_ref()
                                        .map(|path| path.to_string_lossy().into_owned()),
                                },
                            },
                            diff: change.diff.clone(),
                        })
                        .collect();
                    // oracle diff_render.rs collect_rows: by path
                    changes.sort_by(|left, right| left.path.cmp(&right.path));
                    changes
                },
                status: match status {
                    PatchApplyStatus::InProgress => ToolStatus::Running,
                    PatchApplyStatus::Completed => ToolStatus::Completed,
                    PatchApplyStatus::Failed => ToolStatus::Failed,
                    PatchApplyStatus::Declined => ToolStatus::Declined,
                },
                source: PatchSource::Live,
            })),
        ),
        ThreadItemPayload::WebSearch(search) => (
            ItemId::new(&search.id),
            Item::Web(web_activity(search, completed)),
        ),
        ThreadItemPayload::ImageView { id, path } => {
            let path = path.to_string();
            (
                ItemId::new(id),
                Item::ImageView {
                    name: file_name(&path).to_owned(),
                    path,
                },
            )
        }
        ThreadItemPayload::ContextCompaction { id } => (
            ItemId::new(id),
            Item::Notice(Notice::ContextCompacted { took: None }),
        ),
        ThreadItemPayload::EnteredReviewMode { id, review } => (
            ItemId::new(id),
            Item::Notice(Notice::EnteredReview(review.clone())),
        ),
        ThreadItemPayload::ExitedReviewMode { id, review } => (
            ItemId::new(id),
            Item::Notice(Notice::ExitedReview(review.clone())),
        ),
        ThreadItemPayload::FunctionCallOutput {
            id,
            name,
            namespace,
            output,
        } if namespace
            .as_deref()
            .is_some_and(|namespace| DELEGATING_NAMESPACES.contains(&namespace))
            && DELEGATING_TOOLS.contains(&name.as_str()) =>
        {
            let message = delegated(&output.to_text()?)?;
            (
                ItemId::new(id),
                Item::Delegated(std::sync::Arc::new(message)),
            )
        }
        ThreadItemPayload::ImageGeneration(image) if completed => (
            ItemId::new(&image.id),
            Item::Generated(std::sync::Arc::new(GeneratedImage {
                failed: image.status == "failed",
                detail: image
                    .revised_prompt
                    .clone()
                    .unwrap_or_else(|| image.id.clone()),
                saved_to: image
                    .saved_path
                    .as_ref()
                    .map(|path| file_url(&path.as_path().to_string_lossy())),
                saved_path: image
                    .saved_path
                    .as_ref()
                    .map(|path| path.as_path().to_string_lossy().into_owned()),
            })),
        ),
        _ => return None,
    })
}

/// The last part of a path written with either separator (oracle
/// `ViewImageHistoryCell::filename`).
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
}

/// Items projected from a persisted turn rather than a live notification
/// (oracle `thread_items_to_transcript_cells`). In reasoning, raw content follows the split
/// summary body unchanged, so a heading of its own stays. For patches, keep the outcome and
/// attempted changes under the original item ID, together with where the record came from.
pub(super) fn history(
    payload: &ThreadItemPayload,
    entry: (ItemId, super::Item),
) -> (ItemId, super::Item) {
    match (payload, entry) {
        (
            ThreadItemPayload::Reasoning { content, .. },
            (id, super::Item::Reasoning { summary, .. }),
        ) => {
            let with_raw = (!content.is_empty()).then(|| {
                let raw = content.join("\n\n");
                if summary.is_empty() {
                    raw
                } else {
                    format!("{summary}\n\n{raw}")
                }
            });
            (id, super::Item::Reasoning { summary, with_raw })
        }
        (_, (id, super::Item::Patch(mut patch))) => {
            std::sync::Arc::make_mut(&mut patch).source = PatchSource::History;
            (id, super::Item::Patch(patch))
        }
        (_, entry) => entry,
    }
}

/// The summary parts as the Markdown body the transcript shows (oracle
/// messages.rs `split_reasoning_summary_parts`): blank parts and parts whose
/// body is an empty HTML comment are left out, the rest joined by blank lines,
/// and a `**Header**` that opens the result and ends its line is dropped (the
/// oracle keeps it as the status header, which the transcript never shows).
fn summary_text(parts: &[String]) -> String {
    let content = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| {
            if part.is_empty() {
                return false;
            }
            let header_end = part.strip_prefix("**").and_then(|after| {
                after
                    .find("**")
                    .and_then(|close| (close > 0).then_some(close + 4))
            });
            let body = header_end.map_or(*part, |end| &part[end..]);
            body.trim() != "<!-- -->"
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(after_open) = content.strip_prefix("**")
        && let Some(close) = after_open.find("**")
    {
        let after_close = &content[2 + close + 2..];
        if after_close.starts_with(['\n', '\r']) {
            return after_close.to_owned();
        }
    }
    content
}

fn web_activity(search: &codex_app_server_protocol::WebSearchItem, completed: bool) -> WebActivity {
    use codex_app_server_protocol::WebSearchAction as Wire;
    // oracle search.rs web_search_action_detail / web_search_detail
    let (action, detail) = match &search.action {
        Some(Wire::Search { query, queries }) => (
            WebAction::Search { known: true },
            query
                .clone()
                .filter(|query| !query.is_empty())
                .unwrap_or_else(|| queries.as_deref().unwrap_or_default().join(", ")),
        ),
        Some(Wire::OpenPage { url }) => (WebAction::OpenPage, url.clone().unwrap_or_default()),
        Some(Wire::FindInPage { url, pattern }) => {
            let pattern = pattern.as_deref().filter(|pattern| !pattern.is_empty());
            let url = url.as_deref().filter(|url| !url.is_empty());
            (
                WebAction::FindInPage {
                    pattern: pattern.is_some(),
                },
                match (pattern, url) {
                    (Some(pattern), Some(url)) => format!("'{pattern}' in {url}"),
                    (Some(pattern), None) => format!("'{pattern}'"),
                    (None, Some(url)) => url.to_owned(),
                    (None, None) => String::new(),
                },
            )
        }
        Some(Wire::Other) => (WebAction::Search { known: false }, String::new()),
        None => (WebAction::Search { known: false }, String::new()),
    };
    let detail = if detail.is_empty() {
        search.query.clone()
    } else {
        detail
    };
    WebActivity {
        completed,
        action,
        detail,
    }
}
