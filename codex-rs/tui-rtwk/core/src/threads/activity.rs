//! The activity shown alongside messages in live and restored turns.

use std::borrow::Cow;
use std::sync::Arc;

use codex_app_server_protocol::{
    CommandAction, CommandExecutionSource, CommandExecutionStatus,
    DynamicToolCallOutputContentItem, DynamicToolCallStatus, McpToolCallStatus, ThreadItemPayload,
};
use codex_protocol::mcp::is_node_repl_backed_server;
use serde_json::Value;

use super::live_output::{MAX_BYTES, OutputLines, RetainedOutput};
use crate::ids::ItemId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolActivity {
    pub kind: ToolKind,
    pub status: ToolStatus,
    pub output: Option<String>,
    retained_output: Option<Arc<RetainedOutput>>,
    output_lines: Option<OutputLines>,
    /// The app's clock (epoch milliseconds) when a shell command was seen to start: what an
    /// interrupted command's duration counts from (oracle `ExecCall::start_time`).
    pub started_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Shell {
        command: String,
        actions: Vec<ExplorationAction>,
        /// The command parsed to more than one action (oracle `parsed.len() > 1`): its one exit
        /// code is the command's, not an outcome of each action.
        compound: bool,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
        user_shell: bool,
        /// The server's process id of a unified-exec command (oracle
        /// `process_id`): the key of its background terminal.
        process_id: Option<String>,
        /// The command started a unified-exec session (oracle
        /// `ExecCommandSource::UnifiedExecStartup`): while it runs it is a
        /// background terminal.
        unified_exec: bool,
    },
    /// An `mcpToolCall` item (oracle `McpToolCallCell`).
    Mcp(McpCall),
    /// A `dynamicToolCall` item (oracle `DynamicToolCallCell`).
    Dynamic(DynamicCall),
}

impl ToolActivity {
    pub fn new(kind: ToolKind, status: ToolStatus, mut output: Option<String>) -> Self {
        let mut output_lines = None;
        let mut retained_output = None;
        if status == ToolStatus::Running
            && matches!(&kind, ToolKind::Shell { .. })
            && output.as_ref().is_some_and(|text| text.len() > MAX_BYTES)
            && let Some(text) = output.take()
        {
            output_lines = Some(OutputLines::of(&text));
            let mut retained = RetainedOutput::default();
            retained.append(&text);
            retained_output = Some(Arc::new(retained));
        }
        Self {
            kind,
            status,
            output,
            retained_output,
            output_lines,
            started_ms: None,
        }
    }

    /// Only streamed output can have a storage omission. A final item replaces it.
    pub fn retained_output(&self) -> Option<&RetainedOutput> {
        self.retained_output.as_deref()
    }

    pub fn streamed_output_line_counts(&self) -> Option<(usize, usize)> {
        let lines = self.output_lines.as_ref()?;
        Some((lines.initial, lines.visible))
    }

    pub fn streamed_output_tail(&self) -> Option<impl DoubleEndedIterator<Item = &str>> {
        Some(self.output_lines.as_ref()?.tail(self.output.as_deref()?))
    }

    pub(super) fn append_output(&mut self, delta: &str) {
        if matches!(self.kind, ToolKind::Shell { .. }) {
            let lines = self
                .output_lines
                .get_or_insert_with(|| OutputLines::of(self.output.as_deref().unwrap_or_default()));
            lines.append(delta);
        }
        if let Some(retained) = &mut self.retained_output {
            Arc::make_mut(retained).append(delta);
        } else if self
            .output
            .as_ref()
            .map_or(0, String::len)
            .saturating_add(delta.len())
            <= MAX_BYTES
        {
            self.output.get_or_insert_with(String::new).push_str(delta);
        } else {
            let mut retained = RetainedOutput::default();
            if let Some(full) = self.output.take() {
                retained.append(&full);
            }
            retained.append(delta);
            self.retained_output = Some(Arc::new(retained));
        }
    }

    /// Notification/snapshot with no output preserves what the command already reported.
    pub(super) fn keep_previous_output(&mut self, previous: &Self) {
        if self.output.is_none() && self.retained_output.is_none() {
            self.output.clone_from(&previous.output);
            self.retained_output.clone_from(&previous.retained_output);
            self.output_lines.clone_from(&previous.output_lines);
        }
    }

    /// The call joins a computer-activity group (oracle
    /// `McpInvocation::is_computer_activity`).
    pub fn is_computer_activity(&self) -> bool {
        matches!(&self.kind, ToolKind::Mcp(call) if call.is_computer_activity())
    }
}

/// One MCP call with its result projected once: the wire result can hold
/// multi-megabyte bodies, only what a row shows is kept (oracle
/// `history_cell/mcp_result.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpCall {
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    /// `None` while the call runs; an error is the wire error's message.
    pub result: Option<Result<McpResult, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpResult {
    pub content: Vec<McpBlock>,
    pub is_error: bool,
    /// A valid image block came back (oracle: one that decodes; core has no
    /// decoder, a block with its `data` and `mimeType` counts).
    pub has_image: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpBlock {
    display: BlockDisplay,
    pub is_image: bool,
    /// Code mode reads a top-level `text` even from a malformed or non-text block.
    original_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BlockDisplay {
    Text(String),
    Image,
    Audio,
    Resource(String),
    Link(String),
    Json(String),
}

/// What a block says, for the view to word (oracle `render_full`: the text,
/// a marker for media and for an embedded resource).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockBody<'a> {
    Text(Cow<'a, str>),
    Image,
    Audio,
    Resource(&'a str),
    Link(&'a str),
}

/// What a result's text means (oracle `McpResultKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpResultKind {
    Standard,
    NodeRepl,
}

impl McpCall {
    pub fn is_computer_activity(&self) -> bool {
        self.server == "cua_repl"
    }

    pub fn result_kind(&self) -> McpResultKind {
        if self.is_computer_activity()
            || (is_node_repl_backed_server(&self.server) && self.tool == "js")
        {
            McpResultKind::NodeRepl
        } else {
            McpResultKind::Standard
        }
    }

    /// `Some(true)` called, `Some(false)` failed, `None` still running.
    pub fn success(&self) -> Option<bool> {
        match self.result.as_ref()? {
            Ok(result) => Some(!result.is_error),
            Err(_) => Some(false),
        }
    }

    pub fn has_image(&self) -> bool {
        matches!(&self.result, Some(Ok(result)) if result.has_image)
    }

    /// The `title` argument of a code-mode call, blanks collapsed (oracle
    /// `compact_tool_name`); `server.tool` otherwise.
    pub fn title(&self) -> String {
        self.arguments
            .get("title")
            .filter(|_| self.result_kind() == McpResultKind::NodeRepl)
            .and_then(Value::as_str)
            .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| format!("{}.{}", self.server, self.tool))
    }

    /// The `title` argument as the call wrote it, `None` when blank.
    pub fn action_title(&self) -> Option<&str> {
        self.arguments
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.trim().is_empty())
    }

    /// The first line of the failure a computer-activity row shows (oracle
    /// `computer_activity.rs` `error_preview`): the error, a `Script error:`
    /// block, else the first text block with words. CUA errors can append
    /// whole API manuals; the later lines stay in the expanded form.
    pub fn error_preview(&self) -> Option<&str> {
        let text = match self.result.as_ref()? {
            Err(error) => error.as_str(),
            Ok(result) => result
                .content
                .iter()
                .filter_map(McpBlock::text)
                .find(|text| text.starts_with("Script error:"))
                .or_else(|| {
                    result
                        .content
                        .iter()
                        .find_map(|block| block.text().filter(|text| !text.trim().is_empty()))
                })?,
        };
        text.strip_prefix("Script error:")
            .unwrap_or(text)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
    }

    /// The row source of one block in the collapsed preview (oracle
    /// `compact_mcp_lines`): a successful code-mode result shows its output.
    pub fn preview_text<'a>(&self, block: &'a McpBlock) -> BlockBody<'a> {
        if self.result_kind() == McpResultKind::NodeRepl
            && self.success() == Some(true)
            && let Some(text) = block.text()
        {
            if text.starts_with("Script completed\n")
                && let Some((_, output)) = text.split_once("\nOutput:\n")
            {
                return BlockBody::Text(Cow::Borrowed(output));
            }
            if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(text)
                && fields.get("exit_code").and_then(Value::as_i64) == Some(0)
                && let Some(output) = fields.get("output").and_then(Value::as_str)
            {
                return BlockBody::Text(Cow::Owned(output.to_owned()));
            }
        }
        block.render_full()
    }
}

impl McpResult {
    fn project(content: &[Value], is_error: bool, kind: McpResultKind) -> Self {
        let mut has_image = false;
        let content = content
            .iter()
            .map(|block| {
                let block = McpBlock::project(block, kind);
                has_image |= block.is_image;
                block
            })
            .collect();
        Self {
            content,
            is_error,
            has_image,
        }
    }
}

impl McpBlock {
    fn project(block: &Value, kind: McpResultKind) -> Self {
        let field = |name: &str| block.get(name).and_then(Value::as_str);
        let valid_text =
            block.get("type").and_then(Value::as_str) == Some("text") && field("text").is_some();
        let (display, is_image) = match block.get("type").and_then(Value::as_str) {
            Some("text") if let Some(text) = field("text") => {
                (BlockDisplay::Text(text.to_owned()), false)
            }
            Some("image") if field("data").is_some() && field("mimeType").is_some() => {
                (BlockDisplay::Image, true)
            }
            Some("audio") if field("data").is_some() && field("mimeType").is_some() => {
                (BlockDisplay::Audio, false)
            }
            Some("resource")
                if let Some(resource) = block.get("resource")
                    && let Some(uri) = resource.get("uri").and_then(Value::as_str)
                    && (resource.get("text").is_some_and(Value::is_string)
                        || resource.get("blob").is_some_and(Value::is_string)) =>
            {
                (BlockDisplay::Resource(uri.to_owned()), false)
            }
            Some("resource_link") if field("uri").is_some() && field("name").is_some() => {
                let uri = field("uri").unwrap_or_default();
                (BlockDisplay::Link(uri.to_owned()), false)
            }
            _ => (BlockDisplay::Json(block.to_string()), false),
        };
        let original_text = match kind {
            McpResultKind::NodeRepl if !valid_text => field("text").map(str::to_owned),
            _ => None,
        };
        Self {
            display,
            is_image,
            original_text,
        }
    }

    /// All retained text, with a marker for media and embedded resources
    /// (oracle `render_full`).
    pub fn render_full(&self) -> BlockBody<'_> {
        if let Some(text) = &self.original_text {
            return BlockBody::Text(Cow::Borrowed(text.trim_end_matches('\n')));
        }
        match &self.display {
            BlockDisplay::Text(text) | BlockDisplay::Json(text) => {
                BlockBody::Text(Cow::Borrowed(text.trim_end_matches('\n')))
            }
            BlockDisplay::Image => BlockBody::Image,
            BlockDisplay::Audio => BlockBody::Audio,
            BlockDisplay::Resource(uri) => BlockBody::Resource(uri),
            BlockDisplay::Link(uri) => BlockBody::Link(uri),
        }
    }

    /// The top-level text code mode reads (oracle `text`).
    pub fn text(&self) -> Option<&str> {
        match &self.display {
            BlockDisplay::Text(text) => Some(text),
            _ => self.original_text.as_deref(),
        }
    }
}

/// One content item of a dynamic call's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DynamicOutput {
    Text(String),
    Image,
    Audio,
}

/// One dynamic tool call (oracle `DynamicToolCallData`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicCall {
    /// `namespace.tool`, or the tool alone.
    pub name: String,
    pub arguments: Value,
    pub status: ToolStatus,
    /// One entry per content item.
    pub output: Option<Vec<DynamicOutput>>,
    pub duration_ms: Option<i64>,
    /// The turn ended before the call returned (oracle `mark_interrupted`): the header says
    /// `Interrupted`, the output says why.
    pub interrupted: bool,
}

/// What a dynamic call that never returned says (oracle `DynamicToolCallData::mark_interrupted`).
pub const DYNAMIC_INTERRUPTED: &str = "Interrupted before this tool returned a result.";

impl ToolActivity {
    /// The turn ended while the call still ran (oracle `finalize_turn`: `mark_failed` of a computer
    /// or MCP call, `mark_interrupted` of a dynamic one). A shell command is the transcript's own.
    pub(super) fn interrupt(&mut self) -> bool {
        if self.status != ToolStatus::Running {
            return false;
        }
        match &mut self.kind {
            ToolKind::Mcp(call) if call.result.is_none() => {
                call.result = Some(Err("interrupted".to_owned()));
            }
            ToolKind::Dynamic(call) => {
                call.interrupted = true;
                call.output
                    .get_or_insert_with(Vec::new)
                    .push(DynamicOutput::Text(DYNAMIC_INTERRUPTED.to_owned()));
            }
            _ => return false,
        }
        self.status = ToolStatus::Failed;
        if let ToolKind::Dynamic(call) = &mut self.kind {
            call.status = ToolStatus::Failed;
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorationAction {
    Read(Vec<String>),
    List(String),
    Search { query: String, path: Option<String> },
}

fn exploration(actions: &[CommandAction]) -> Vec<ExplorationAction> {
    let reads_only = actions
        .iter()
        .all(|action| matches!(action, CommandAction::Read { .. }));
    let mut result = Vec::new();
    for action in actions {
        match action {
            CommandAction::Read { name, .. } => {
                if reads_only && let Some(ExplorationAction::Read(names)) = result.last_mut() {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                } else {
                    result.push(ExplorationAction::Read(vec![name.clone()]));
                }
            }
            CommandAction::ListFiles { command, path } => result.push(ExplorationAction::List(
                path.as_ref().unwrap_or(command).clone(),
            )),
            CommandAction::Search {
                command,
                query,
                path,
            } => result.push(ExplorationAction::Search {
                query: query.as_ref().unwrap_or(command).clone(),
                path: path.clone().filter(|_| query.is_some()),
            }),
            CommandAction::Unknown { .. } => return Vec::new(),
        }
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Completed,
    Failed,
    Declined,
}

pub(super) fn tool(payload: &ThreadItemPayload) -> Option<(ItemId, ToolActivity)> {
    use ToolStatus::{Completed, Declined, Failed, Running};
    let (id, kind, status, output) = match payload {
        ThreadItemPayload::CommandExecution {
            id,
            command,
            status,
            aggregated_output,
            command_actions,
            exit_code,
            duration_ms,
            source,
            process_id,
            ..
        } => (
            id,
            ToolKind::Shell {
                command: command.clone(),
                actions: if *source == CommandExecutionSource::UserShell {
                    Vec::new()
                } else {
                    exploration(command_actions)
                },
                compound: command_actions.len() > 1,
                exit_code: *exit_code,
                duration_ms: duration_ms.and_then(|millis| u64::try_from(millis).ok()),
                user_shell: *source == CommandExecutionSource::UserShell,
                process_id: process_id.clone(),
                unified_exec: *source == CommandExecutionSource::UnifiedExecStartup,
            },
            match status {
                CommandExecutionStatus::InProgress => Running,
                CommandExecutionStatus::Completed => Completed,
                CommandExecutionStatus::Failed => Failed,
                CommandExecutionStatus::Declined => Declined,
            },
            aggregated_output.clone(),
        ),
        ThreadItemPayload::McpToolCall {
            id,
            server,
            tool,
            status,
            arguments,
            result,
            error,
            ..
        } => {
            let mut call = McpCall {
                server: server.clone(),
                tool: tool.clone(),
                arguments: arguments.clone(),
                result: None,
            };
            // Oracle `McpHistory::from_item`: an error wins, a result takes
            // `is_error` from the status, neither is an error of its own.
            if *status != McpToolCallStatus::InProgress {
                let kind = call.result_kind();
                call.result = Some(match (result, error) {
                    (_, Some(error)) => Err(error.message.clone()),
                    (Some(result), None) => Ok(McpResult::project(
                        &result.content,
                        *status == McpToolCallStatus::Failed,
                        kind,
                    )),
                    (None, None) => Err("MCP tool call completed without a result".to_owned()),
                });
            }
            (
                id,
                ToolKind::Mcp(call),
                match status {
                    McpToolCallStatus::InProgress => Running,
                    McpToolCallStatus::Completed => Completed,
                    McpToolCallStatus::Failed => Failed,
                },
                None,
            )
        }
        ThreadItemPayload::DynamicToolCall {
            id,
            namespace,
            tool,
            status,
            arguments,
            content_items,
            success,
            duration_ms,
        } => {
            let status = match (success, status) {
                (Some(false), _) | (_, DynamicToolCallStatus::Failed) => Failed,
                (_, DynamicToolCallStatus::InProgress) => Running,
                (_, DynamicToolCallStatus::Completed) => Completed,
            };
            (
                id,
                ToolKind::Dynamic(DynamicCall {
                    name: namespace
                        .as_ref()
                        .map_or_else(|| tool.clone(), |namespace| format!("{namespace}.{tool}")),
                    arguments: arguments.clone(),
                    status,
                    output: content_items.as_ref().map(|items| {
                        items
                            .iter()
                            .map(|item| match item {
                                DynamicToolCallOutputContentItem::InputText { text } => {
                                    DynamicOutput::Text(text.clone())
                                }
                                DynamicToolCallOutputContentItem::InputImage { .. } => {
                                    DynamicOutput::Image
                                }
                                DynamicToolCallOutputContentItem::InputAudio { .. } => {
                                    DynamicOutput::Audio
                                }
                            })
                            .collect()
                    }),
                    duration_ms: *duration_ms,
                    interrupted: false,
                }),
                status,
                None,
            )
        }
        _ => return None,
    };
    Some((ItemId::new(id), ToolActivity::new(kind, status, output)))
}
