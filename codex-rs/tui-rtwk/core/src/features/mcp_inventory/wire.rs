//! The RPC of `/mcp`: `mcpServerStatus/list`, one page at a time (oracle
//! `app/background_requests.rs` `fetch_all_mcp_server_statuses`), and the servers it answers
//! with as the plain data the cell is made of (`McpServerStatus` keeps the protocol's tool
//! schemas, which the cell never reads).

use codex_app_server_protocol::{
    ClientRequest, ListMcpServerStatusParams, ListMcpServerStatusResponse, McpAuthStatus,
    McpServerConnectionStatus, McpServerStatus, McpServerStatusDetail, RequestId,
};

use super::{Auth, Connection, Entry, Server};
use crate::ids::ThreadId;
use crate::rpc::RpcError;

/// Servers asked for per page (oracle: `limit: Some(100)`).
const PAGE: u32 = 100;

/// The one ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `mcpServerStatus/list`: the page after `cursor`, with or without the resources.
    List {
        /// The thread whose MCP connection is reused; none for the app's own.
        thread: Option<ThreadId>,
        full: bool,
        cursor: Option<String>,
    },
}

/// What a page answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Page {
        servers: Vec<Server>,
        next_cursor: Option<String>,
    },
}

pub(super) fn request(command: &Command, request_id: RequestId) -> ClientRequest {
    let Command::List {
        thread,
        full,
        cursor,
    } = command;
    ClientRequest::McpServerStatusList {
        request_id,
        params: ListMcpServerStatusParams {
            cursor: cursor.clone(),
            limit: Some(PAGE),
            detail: Some(if *full {
                McpServerStatusDetail::Full
            } else {
                McpServerStatusDetail::ToolsAndAuthOnly
            }),
            thread_id: thread.as_ref().map(ToString::to_string),
            server_name: None,
        },
    }
}

pub(super) fn reply(_command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let page = serde_json::from_value::<ListMcpServerStatusResponse>(result)
        .map_err(|error| RpcError::Malformed(error.to_string()))?;
    Ok(Reply::Page {
        servers: page.data.iter().map(server).collect(),
        next_cursor: page.next_cursor,
    })
}

/// One server, as the cell reads it: the connection the cell names (a server with no runtime
/// state that is not logged in needs authentication, oracle `new_mcp_tools_output_from_statuses`),
/// the tool names sorted, the resources by their title else their name.
pub fn server(status: &McpServerStatus) -> Server {
    let connection = match status.runtime_status {
        Some(McpServerConnectionStatus::Connected) => Connection::Connected,
        Some(McpServerConnectionStatus::Starting) => Connection::Starting,
        Some(McpServerConnectionStatus::AuthenticationRequired) => {
            Connection::AuthenticationRequired
        }
        Some(McpServerConnectionStatus::Failed) => Connection::Failed,
        Some(McpServerConnectionStatus::NotStarted) => Connection::NotStarted,
        Some(McpServerConnectionStatus::Disabled) => Connection::Disabled,
        Some(McpServerConnectionStatus::Cancelled) => Connection::Cancelled,
        None if status.auth_status == McpAuthStatus::NotLoggedIn => {
            Connection::AuthenticationRequired
        }
        None => Connection::Unknown,
    };
    let auth = match status.auth_status {
        McpAuthStatus::Unknown => Auth::Unknown,
        McpAuthStatus::Unsupported => Auth::Unsupported,
        McpAuthStatus::NotLoggedIn => Auth::NotLoggedIn,
        McpAuthStatus::BearerToken => Auth::BearerToken,
        McpAuthStatus::OAuth => Auth::OAuth,
    };
    let mut tools: Vec<String> = status.tools.keys().cloned().collect();
    tools.sort();
    Server {
        name: status.name.clone(),
        connection,
        auth,
        tools,
        resources: status
            .resources
            .iter()
            .map(|resource| Entry {
                label: resource
                    .title
                    .clone()
                    .unwrap_or_else(|| resource.name.clone()),
                uri: resource.uri.clone(),
            })
            .collect(),
        templates: status
            .resource_templates
            .iter()
            .map(|template| Entry {
                label: template
                    .title
                    .clone()
                    .unwrap_or_else(|| template.name.clone()),
                uri: template.uri_template.clone(),
            })
            .collect(),
    }
}
