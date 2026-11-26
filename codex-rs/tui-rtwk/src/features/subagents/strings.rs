//! The transcript words of the Subagents status summary.

use codex_tui_core::features::subagents::{Activity, AgentAction, Entry};
use codex_tui_core::threads::SubAgentKind;

pub(super) const COMMAND: &str = "/subagents";
pub(super) const TITLE: &str = "Sub-agents running";
pub(super) const NONE_RUNNING: &str = "  • No sub-agents running.";
pub(super) const NO_ACTIVITY: &str = "No recent activity yet.";
pub(super) const NO_AGENTS: &str = "No agents available yet.";

pub(super) fn agent_title(entry: &Entry) -> String {
    format!("  • `{}`", entry.path)
}

pub(super) fn activity(activity: &Activity) -> String {
    match activity {
        Activity::Text(text) => text.clone(),
        Activity::Command(command) => format!("$ {command}"),
        Activity::Updated(files) => format!("Updated {files} file(s)"),
        Activity::Mcp(server, tool) => format!("MCP {server}/{tool}"),
        Activity::Tool(tool) => format!("Tool {tool}"),
        Activity::Agent(action) => match action {
            AgentAction::Spawned => "Spawned an agent",
            AgentAction::SentInput => "Sent input to an agent",
            AgentAction::Resumed => "Resumed an agent",
            AgentAction::Waited => "Waited for an agent",
            AgentAction::Closed => "Closed an agent",
        }
        .to_owned(),
        Activity::Subagent(kind, path) => {
            let action = match kind {
                SubAgentKind::Started => "Started",
                SubAgentKind::Interacted => "Contacted",
                SubAgentKind::Interrupted => "Interrupted",
                SubAgentKind::Completed => "Completed",
            };
            format!("{action} {path}")
        }
        Activity::Web(query) => format!("Web search: {query}"),
        Activity::Viewed(path) => format!("Viewed {path}"),
        Activity::Generated => "Generated an image".to_owned(),
        Activity::EnteredReview => "Entered review mode".to_owned(),
        Activity::ExitedReview => "Exited review mode".to_owned(),
        Activity::Compacted => "Compacted context".to_owned(),
    }
}
