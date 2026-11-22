//! MCP startup of one thread (oracle `chatwidget/mcp_startup.rs`): the app
//! server reports each server's start and outcome; this keeps the round in
//! flight, the status header it makes and the warnings a failure or a settled
//! round adds to the history. Pure state: `threads::apply` appends the rows.

use std::collections::BTreeMap;

use codex_app_server_protocol::McpServerStartupState;
use codex_app_server_protocol::McpServerStatusUpdatedNotification;

const SINGLE_HEADER: &str = "Booting MCP server:";
const MULTI_HEADER: &str = "Starting MCP servers";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpStatus {
    Starting,
    Ready,
    Failed { error: String },
    Cancelled,
}

impl McpStatus {
    pub(crate) fn of(note: &McpServerStatusUpdatedNotification) -> McpStatus {
        match note.status {
            McpServerStartupState::Starting => McpStatus::Starting,
            McpServerStartupState::Ready => McpStatus::Ready,
            McpServerStartupState::Failed => McpStatus::Failed {
                error: note
                    .error
                    .clone()
                    .unwrap_or_else(|| format!("MCP client for `{}` failed to start", note.name)),
            },
            McpServerStartupState::Cancelled => McpStatus::Cancelled,
        }
    }

    fn is_diagnostic(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McpStartup {
    /// The servers of the round in flight, by name; `None` once it settled.
    round: Option<BTreeMap<String, McpStatus>>,
    /// The last failure or cancellation per server: the same one is not
    /// reported twice until something else happened to the server.
    last: BTreeMap<String, McpStatus>,
}

impl McpStartup {
    /// A server's start or outcome; the warnings to add to the history.
    pub fn update(&mut self, server: String, status: McpStatus) -> Vec<String> {
        let mut warnings = Vec::new();
        let report = if status.is_diagnostic() {
            self.last.insert(server.clone(), status.clone()).as_ref() != Some(&status)
        } else {
            self.last.remove(&server);
            false
        };
        if report && let McpStatus::Failed { error } = &status {
            warnings.push(error.clone());
        }
        // An outcome with no start (after an attach or a lag) is reported at once.
        if self.round.is_none() && !matches!(status, McpStatus::Starting) {
            if report {
                warnings.extend(summary([(&server, &status)]));
            }
            return warnings;
        }
        self.round
            .get_or_insert_with(BTreeMap::new)
            .insert(server, status);
        if self.header().is_none() {
            warnings.extend(self.finish());
        }
        warnings
    }

    /// The status header the round makes while a server still starts.
    pub fn header(&self) -> Option<String> {
        let round = self.round.as_ref()?;
        let total = round.len();
        let starting: Vec<&String> = round
            .iter()
            .filter(|(_, status)| matches!(status, McpStatus::Starting))
            .map(|(name, _)| name)
            .collect();
        let first = starting.first()?;
        let completed = total.saturating_sub(starting.len());
        let mut shown: Vec<String> = starting.iter().take(3).map(|name| name.to_string()).collect();
        if starting.len() > 3 {
            shown.push("…".to_string());
        }
        Some(if total > 1 {
            format!("{MULTI_HEADER} ({completed}/{total}): {}", shown.join(", "))
        } else {
            format!("{SINGLE_HEADER} {first}")
        })
    }

    /// The round settled (every server answered): its summary warnings.
    pub fn finish(&mut self) -> Vec<String> {
        self.round
            .take()
            .map(|settled| summary(settled.iter()))
            .unwrap_or_default()
    }

    /// The app server's event stream lagged: what was missed cannot settle the
    /// round, so it settles now, and a missed start may repeat a failure.
    pub fn finish_after_lag(&mut self) -> Vec<String> {
        let warnings = if self.round.is_some() {
            self.finish()
        } else {
            Vec::new()
        };
        self.last.clear();
        warnings
    }

    /// A server still starts.
    pub fn is_starting(&self) -> bool {
        self.round.is_some()
    }
}

fn summary<'a>(outcomes: impl IntoIterator<Item = (&'a String, &'a McpStatus)>) -> Vec<String> {
    let mut failed = Vec::new();
    let mut cancelled = Vec::new();
    for (server, status) in outcomes {
        match status {
            McpStatus::Failed { .. } => failed.push(server.clone()),
            McpStatus::Cancelled => cancelled.push(server.clone()),
            McpStatus::Starting | McpStatus::Ready => {}
        }
    }
    let mut warnings = Vec::new();
    if !cancelled.is_empty() {
        cancelled.sort();
        warnings.push(format!(
            "MCP startup interrupted. The following servers were not initialized: {}",
            cancelled.join(", ")
        ));
    }
    if !failed.is_empty() {
        failed.sort();
        warnings.push(format!(
            "MCP startup incomplete (failed: {})",
            failed.join(", ")
        ));
    }
    warnings
}
