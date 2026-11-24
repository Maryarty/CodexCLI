//! The words of `/mcp` (oracle `history_cell/mcp.rs` `new_mcp_tools_output_from_statuses`,
//! `empty_mcp_output`, `mcp_auth_status_label`). The text of the cell as one note; its styled
//! layout (the magenta `/mcp`, the status colours, the dimmed counts) and the loading row's
//! spinner are the page's (UI LEFT).

use codex_tui_core::features::mcp_inventory::{Auth, Connection, Entry, Inventory, Note};

pub fn connection(connection: Connection) -> &'static str {
    match connection {
        Connection::Connected => "connected",
        Connection::Starting => "starting",
        Connection::AuthenticationRequired => "authentication required",
        Connection::Failed => "failed",
        Connection::NotStarted => "not started",
        Connection::Disabled => "disabled",
        Connection::Cancelled => "cancelled",
        Connection::Unknown => "unknown",
    }
}

pub fn auth(auth: Auth) -> &'static str {
    match auth {
        Auth::Unknown => "Unknown",
        Auth::Unsupported => "Unsupported",
        Auth::NotLoggedIn => "Not logged in",
        Auth::BearerToken => "Bearer token",
        Auth::OAuth => "OAuth",
    }
}

/// The loading row's text (oracle `McpInventoryLoadingCell`).
pub const LOADING: &str = "Loading MCP inventory…";

fn entries(label: &str, entries: &[Entry]) -> String {
    if entries.is_empty() {
        return format!("    • {label}: (none)");
    }
    let list: Vec<String> = entries
        .iter()
        .map(|entry| format!("{} ({})", entry.label, entry.uri))
        .collect();
    format!("    • {label}: {}", list.join(", "))
}

fn inventory(inventory: &Inventory) -> String {
    let mut lines = vec![
        "/mcp".to_owned(),
        String::new(),
        "🔌  MCP Tools".to_owned(),
        String::new(),
    ];
    if inventory.no_tools() {
        lines.push("  • No MCP tools available.".to_owned());
        lines.push(String::new());
    }
    for server in &inventory.servers {
        let count = server.tools.len();
        let unit = if count == 1 { "tool" } else { "tools" };
        lines.push(format!(
            "  • {}: {} ({count} {unit})",
            server.name,
            connection(server.connection)
        ));
        if !inventory.full {
            continue;
        }
        lines.push(format!("    • Auth: {}", auth(server.auth)));
        if server.tools.is_empty() {
            lines.push("    • Tools: (none)".to_owned());
        } else {
            lines.push(format!("    • Tools: {}", server.tools.join(", ")));
        }
        lines.push(entries("Resources", &server.resources));
        lines.push(entries("Resource templates", &server.templates));
        lines.push(String::new());
    }
    if !inventory.full {
        lines.push(String::new());
        lines.push("  Use /mcp verbose for tools and resources.".to_owned());
    }
    lines.join("\n")
}

pub fn note(note: &Note) -> String {
    match note {
        Note::Inventory(inv) => inventory(inv),
        Note::Empty => "/mcp\n\n🔌  MCP Tools\n\n  • No MCP servers configured.\n    See the MCP docs to configure them.".to_owned(),
        Note::Failed(error) => format!("Failed to load MCP inventory: {error}"),
    }
}
