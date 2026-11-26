//! The details of a task: what needs the user, the latest activity, the last
//! message, tokens. Typed lines: the words are the UI's string table's.
//! Oracle: `agents_overview_details.rs` (`agents_overview_details`).

use std::collections::HashMap;

use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadActiveFlag;
use codex_app_server_protocol::ThreadStatus;

use super::CenterThreads;
use super::Model;
use super::carried::AgentsOverviewGroup;
use super::carried::preview_text;
use crate::threads::ThreadPreview;

/// One line of the activity block, in the oracle's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailLine {
    Blank,
    /// `Agent: <name>` for a child that needs the user more than the root.
    Agent(String),
    NeedsAttention,
    /// What a pending request asks (preview text).
    Request(String),
    OpenToReview,
    WaitingApproval,
    WaitingResponse,
    TaskError,
    /// The `Latest activity` label and its header.
    LatestActivity(String),
}

/// Oracle: `AgentsOverviewDetails`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Details {
    pub latest_prompt: Option<String>,
    pub lines: Vec<DetailLine>,
    /// Tokens in and out of the root task, when known.
    pub tokens: Option<(i64, i64)>,
    /// Tokens and the cached estimate of the root task; the UI turns them
    /// into the `Tokens:` and `Est. usage:` lines (`Usage::token_counts`).
    pub usage: super::Usage,
    /// The source's last message (Markdown) and its directory for local links.
    pub last_message: Option<(String, String)>,
}

impl Model {
    pub(super) fn details(
        &self,
        root: &Thread,
        children: &HashMap<String, Vec<&Thread>>,
        state: &CenterThreads,
    ) -> Details {
        let mut lines = Vec::new();
        // Urgency comes first; on ties, prefer available current detail over history.
        let priority = |thread: &Thread| {
            let group = AgentsOverviewGroup::for_thread(thread);
            let has_detail = match group {
                AgentsOverviewGroup::NeedsYou => state.request(&thread.id).is_some(),
                AgentsOverviewGroup::Working => state
                    .preview(&thread.id)
                    .is_some_and(|activity| activity.reasoning_header().is_some()),
                AgentsOverviewGroup::Ready | AgentsOverviewGroup::Finished => false,
            };
            (group, !has_detail)
        };
        let mut source = root;
        let mut pending = vec![root];
        while let Some(thread) = pending.pop() {
            if priority(thread) < priority(source) {
                source = thread;
            }
            pending.extend(children.get(&thread.id).into_iter().flatten().copied());
            pending.sort_by(|left, right| right.id.cmp(&left.id));
        }
        let is_child = source.id != root.id;
        if is_child {
            let name = source
                .agent_nickname
                .as_deref()
                .or(source.name.as_deref())
                .unwrap_or(&source.id);
            lines.push(DetailLine::Blank);
            lines.push(DetailLine::Agent(preview_text(name)));
        }
        if AgentsOverviewGroup::for_thread(source) == AgentsOverviewGroup::NeedsYou {
            if !is_child {
                lines.extend([DetailLine::Blank, DetailLine::NeedsAttention]);
            }
            if let Some(request) = state.request(&source.id) {
                lines.push(DetailLine::Request(request.to_owned()));
            }
            lines.push(DetailLine::OpenToReview);
            match &source.status {
                ThreadStatus::Active { active_flags } => {
                    if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval) {
                        lines.push(DetailLine::WaitingApproval);
                    }
                    if active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) {
                        lines.push(DetailLine::WaitingResponse);
                    }
                }
                ThreadStatus::SystemError => lines.push(DetailLine::TaskError),
                ThreadStatus::NotLoaded | ThreadStatus::Idle => {}
            }
        }
        let activity = state.preview(&source.id);
        if matches!(source.status, ThreadStatus::Active { .. })
            && let Some(header) = activity.and_then(ThreadPreview::reasoning_header)
        {
            if !is_child {
                lines.push(DetailLine::Blank);
            }
            lines.push(DetailLine::LatestActivity(header.clone()));
        }
        let last_message = activity
            .and_then(|activity| activity.last_message.as_ref())
            .or_else(|| activity.and_then(|activity| activity.history_message.as_ref()))
            .filter(|message| !message.trim().is_empty())
            .map(|message| (message.clone(), source.cwd.display().to_string()));
        Details {
            latest_prompt: state
                .preview(&root.id)
                .and_then(|activity| activity.latest_prompt.clone()),
            lines,
            tokens: self.usage.get(&root.id).and_then(|usage| usage.tokens),
            usage: self.usage.get(&root.id).cloned().unwrap_or_default(),
            last_message,
        }
    }
}
