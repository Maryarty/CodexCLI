//! The RPC of `/mcp`: `mcpServerStatus/list`, one page at a time (oracle
//! `app/background_requests.rs` `fetch_all_mcp_server_statuses`), and the servers it answers
//! with as the plain data the cell is made of (`McpServerStatus` keeps the protocol's tool
//! schemas, which the cell never reads).

use codex_app_server_protocol::{
    ListMcpServerStatusParams, ListMcpServerStatusResponse, McpServerConnectionStatus,
    McpServerStatus, McpServerStatusDetail,
};

use super::{Connection, Entry, Server};
use crate::data::wire::McpAuthStatus;
use crate::ids::ThreadId;

/// Servers asked for per page (oracle: `limit: Some(100)`).
const PAGE: u32 = 100;

/// What a page answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Page {
        servers: Vec<Server>,
        next_cursor: Option<String>,
    },
}

crate::rpc_list! {
    /// The one ask.
    pub enum Command => Reply;
    /// `mcpServerStatus/list`: the page after `cursor`, with or without the resources.
    List {
        /// The thread whose MCP connection is reused; none for the app's own.
        thread: Option<ThreadId>,
        full: bool,
        cursor: Option<String>,
    }, Self::List { thread, full, cursor }
        => McpServerStatusList(ListMcpServerStatusParams {
            cursor: cursor.clone(),
            limit: Some(PAGE),
            detail: Some(if *full {
                McpServerStatusDetail::Full
            } else {
                McpServerStatusDetail::ToolsAndAuthOnly
            }),
            thread_id: thread.as_ref().map(ToString::to_string),
            server_name: None,
        })
        -> ListMcpServerStatusResponse, |page| Reply::Page {
            servers: page.data.iter().map(server).collect(),
            next_cursor: page.next_cursor,
        };
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
    let mut tools: Vec<String> = status.tools.keys().cloned().collect();
    tools.sort();
    Server {
        name: status.name.clone(),
        connection,
        auth: status.auth_status,
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
