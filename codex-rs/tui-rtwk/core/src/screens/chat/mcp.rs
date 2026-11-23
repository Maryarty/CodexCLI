//! MCP startup of one thread (oracle `chatwidget/mcp_startup.rs`): the app
//! server reports each server's start and outcome; this keeps the round in
//! flight, the status header it makes and the warnings a failure or a settled
//! round adds to the history. Pure state: `threads::apply` appends the rows.

use std::collections::BTreeMap;

use codex_app_server_protocol::McpServerStartupState;
use codex_app_server_protocol::McpServerStatusUpdatedNotification;

/// The servers a starting header names before it ends in an ellipsis.
const HEADER_SERVERS: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpStatus {
    Starting,
    Ready,
    /// The server's own words, if it gave any.
    Failed { error: Option<String> },
    Cancelled,
}

/// A warning MCP startup adds to the history; the UI's string tables hold
/// the words (oracle `mcp_startup.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpWarning {
    /// One server failed to start.
    Failed {
        server: String,
        error: Option<String>,
    },
    /// The round settled with servers that never initialized (sorted).
    Interrupted { servers: Vec<String> },
    /// The round settled with servers that failed (sorted).
    Incomplete { servers: Vec<String> },
}

/// The status header while a server still starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpHeader {
    /// One server in the round.
    Booting { server: String },
    /// Several: how many answered, how many there are, the first still
    /// starting, and whether more follow.
    Starting {
        completed: usize,
        total: usize,
        servers: Vec<String>,
        more: bool,
    },
}

impl McpStatus {
    pub(crate) fn of(note: &McpServerStatusUpdatedNotification) -> McpStatus {
        match note.status {
            McpServerStartupState::Starting => McpStatus::Starting,
            McpServerStartupState::Ready => McpStatus::Ready,
            McpServerStartupState::Failed => McpStatus::Failed {
                error: note.error.clone(),
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
    pub fn update(&mut self, server: String, status: McpStatus) -> Vec<McpWarning> {
        let mut warnings = Vec::new();
        let report = if status.is_diagnostic() {
            self.last.insert(server.clone(), status.clone()).as_ref() != Some(&status)
        } else {
            self.last.remove(&server);
            false
        };
        if report && let McpStatus::Failed { error } = &status {
            warnings.push(McpWarning::Failed {
                server: server.clone(),
                error: error.clone(),
            });
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
    pub fn header(&self) -> Option<McpHeader> {
        let round = self.round.as_ref()?;
        let total = round.len();
        let starting: Vec<&String> = round
            .iter()
            .filter(|(_, status)| matches!(status, McpStatus::Starting))
            .map(|(name, _)| name)
            .collect();
        let first = starting.first()?;
        Some(if total > 1 {
            McpHeader::Starting {
                completed: total.saturating_sub(starting.len()),
                total,
                servers: starting
                    .iter()
                    .take(HEADER_SERVERS)
                    .map(ToString::to_string)
                    .collect(),
                more: starting.len() > HEADER_SERVERS,
            }
        } else {
            McpHeader::Booting {
                server: first.to_string(),
            }
        })
    }

    /// The round settled (every server answered): its summary warnings.
    pub fn finish(&mut self) -> Vec<McpWarning> {
        self.round
            .take()
            .map(|settled| summary(settled.iter()))
            .unwrap_or_default()
    }

    /// The app server's event stream lagged: what was missed cannot settle the
    /// round, so it settles now, and a missed start may repeat a failure.
    pub fn finish_after_lag(&mut self) -> Vec<McpWarning> {
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

fn summary<'a>(
    outcomes: impl IntoIterator<Item = (&'a String, &'a McpStatus)>,
) -> Vec<McpWarning> {
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
        warnings.push(McpWarning::Interrupted { servers: cancelled });
    }
    if !failed.is_empty() {
        failed.sort();
        warnings.push(McpWarning::Incomplete { servers: failed });
    }
    warnings
}
