//! Items copied verbatim from the oracle (`codex-rs/tui/src/app/`), each
//! under a marker `// oracle: <file> <kind> <name>`. `tests/carried.rs`
//! compares every marked item with the oracle's: an edit of the oracle that
//! changes one of them fails that test at the next repin. Visibility is
//! normalised by the test; nothing else may differ.

use std::collections::HashMap;

use codex_app_server_protocol::SessionSource;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadActiveFlag;
use codex_app_server_protocol::ThreadStatus;
use codex_protocol::protocol::SubAgentSource;

const PREVIEW_CHARS: usize = 512;

// oracle: agents_overview_view.rs enum AgentsOverviewGroup
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AgentsOverviewGroup {
    NeedsYou,
    Working,
    Ready,
    Finished,
}

impl AgentsOverviewGroup {
    // oracle: agents_overview_view.rs fn for_thread
    pub(super) fn for_thread(thread: &Thread) -> Self {
        match &thread.status {
            ThreadStatus::Active { active_flags }
                if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval)
                    || active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) =>
            {
                Self::NeedsYou
            }
            ThreadStatus::Active { .. } => Self::Working,
            ThreadStatus::Idle => Self::Ready,
            // A subagent failure ends its work without asking the user to act.
            ThreadStatus::SystemError if !is_subagent(thread) => Self::NeedsYou,
            ThreadStatus::SystemError | ThreadStatus::NotLoaded => Self::Finished,
        }
    }
}

// oracle: agents_overview_view.rs fn is_subagent
pub(super) fn is_subagent(thread: &Thread) -> bool {
    thread.parent_thread_id.is_some() || matches!(thread.source, SessionSource::SubAgent(_))
}

// oracle: agents_overview_view.rs fn display_title
pub(super) fn display_title(thread: &Thread) -> &str {
    let title = thread.name.as_deref().unwrap_or(&thread.preview);
    title.trim().lines().next().unwrap_or("Untitled task")
}

// oracle: agents_overview_grouping.rs fn model_name
pub(super) fn model_name(thread: &Thread) -> &str {
    thread
        .model
        .as_deref()
        .filter(|model| !model.is_empty())
        .unwrap_or("Unknown")
}

// oracle: agents_overview.rs fn agents_overview_group
pub(super) fn agents_overview_group(
    thread: &Thread,
    children: &HashMap<String, Vec<&Thread>>,
) -> AgentsOverviewGroup {
    children
        .get(&thread.id)
        .into_iter()
        .flatten()
        .fold(AgentsOverviewGroup::for_thread(thread), |group, child| {
            group.min(agents_overview_group(child, children))
        })
}

// oracle: agents_overview_discovery.rs fn matches_query
pub(super) fn matches_query(thread: &Thread, query: &str) -> bool {
    let name = thread.name.as_deref().unwrap_or_default();
    let cwd = thread.cwd.display();
    format!("{name} {} {cwd}", thread.preview)
        .to_lowercase()
        .contains(query)
}

// oracle: agents_overview_discovery.rs fn is_overview_thread
pub(super) fn is_overview_thread(thread: &Thread) -> bool {
    !thread.ephemeral
        && thread.parent_thread_id.is_none()
        && !matches!(
            thread.source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        )
}

// oracle: agents_overview_discovery.rs fn supports_shared_pinning
pub(super) fn supports_shared_pinning(source: &SessionSource) -> bool {
    matches!(
        source,
        SessionSource::Cli | SessionSource::VsCode | SessionSource::Exec | SessionSource::AppServer
    ) || matches!(source, SessionSource::Custom(source) if matches!(source.as_str(), "atlas" | "chatgpt"))
}

// oracle: agents_overview_details.rs fn preview_text
pub(super) fn preview_text(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .filter(|ch| !ch.is_control())
        .take(PREVIEW_CHARS)
        .collect()
}

// oracle: agents_overview_details.rs fn preview_markdown
pub fn preview_markdown(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .take(PREVIEW_CHARS)
        .collect()
}

// oracle: ../status/helpers.rs fn format_tokens_compact
pub fn format_tokens_compact(value: i64) -> String {
    let value = value.max(0);
    if value == 0 {
        return "0".to_string();
    }
    if value < 1_000 {
        return value.to_string();
    }

    let value_f64 = value as f64;
    // Promote values that would round to 1000 in the smaller unit.
    let (scaled, suffix) = if value >= 999_500_000_000 {
        (value_f64 / 1_000_000_000_000.0, "T")
    } else if value >= 999_500_000 {
        (value_f64 / 1_000_000_000.0, "B")
    } else if value >= 999_500 {
        (value_f64 / 1_000_000.0, "M")
    } else {
        (value_f64 / 1_000.0, "K")
    };

    let decimals = if scaled < 10.0 {
        2
    } else if scaled < 100.0 {
        1
    } else {
        0
    };

    let mut formatted = format!("{scaled:.decimals$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }

    format!("{formatted}{suffix}")
}
