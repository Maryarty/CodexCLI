//! The activity shown alongside messages in live and restored turns.

use codex_app_server_protocol::{
    CommandExecutionStatus, DynamicToolCallOutputContentItem, DynamicToolCallStatus,
    McpToolCallStatus, ThreadItemPayload,
};

use crate::ids::ItemId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolActivity {
    pub kind: ToolKind,
    pub status: ToolStatus,
    pub output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Shell {
        command: String,
    },
    Call {
        namespace: Option<String>,
        name: String,
        arguments: serde_json::Value,
    },
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
            ..
        } => (
            id,
            ToolKind::Shell {
                command: command.clone(),
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
        } => (
            id,
            ToolKind::Call {
                namespace: Some(server.clone()),
                name: tool.clone(),
                arguments: arguments.clone(),
            },
            match status {
                McpToolCallStatus::InProgress => Running,
                McpToolCallStatus::Completed => Completed,
                McpToolCallStatus::Failed => Failed,
            },
            error
                .as_ref()
                .map(|error| error.message.clone())
                .or_else(|| {
                    result.as_ref().map(|result| {
                        let text = result
                            .content
                            .iter()
                            .filter_map(|block| {
                                (block.get("type")?.as_str()? == "text")
                                    .then(|| block.get("text")?.as_str())
                                    .flatten()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        if text.is_empty() {
                            result
                                .structured_content
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_default()
                        } else {
                            text
                        }
                    })
                }),
        ),
        ThreadItemPayload::DynamicToolCall {
            id,
            namespace,
            tool,
            status,
            arguments,
            content_items,
            ..
        } => (
            id,
            ToolKind::Call {
                namespace: namespace.clone(),
                name: tool.clone(),
                arguments: arguments.clone(),
            },
            match status {
                DynamicToolCallStatus::InProgress => Running,
                DynamicToolCallStatus::Completed => Completed,
                DynamicToolCallStatus::Failed => Failed,
            },
            content_items.as_ref().map(|items| {
                items
                    .iter()
                    .filter_map(|item| match item {
                        DynamicToolCallOutputContentItem::InputText { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
        ),
        _ => return None,
    };
    Some((
        ItemId::new(id),
        ToolActivity {
            kind,
            status,
            output,
        },
    ))
}
