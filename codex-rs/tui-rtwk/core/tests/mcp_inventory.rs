#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/mcp` (oracle `history_cell/mcp.rs`, `app/background_requests.rs`): `/mcp` asks
//! `mcpServerStatus/list` for the connection summary, `/mcp verbose` for the full detail; the
//! loading row is data while the ask is out; pages are gathered; the answer is one note of the
//! thread with the servers sorted by name, or the empty note, or the failure. The words and the
//! cell are the view's (`src/features/mcp_inventory/`).

use codex_app_server_protocol::RequestId;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::mcp_inventory::{Auth, Connection, Note as McpNote};
use codex_tui_core::features::{self, FeatureCommand, FeatureNote};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{Item, Note, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn mcp(app: &mut CodexApp, args: Option<&str>) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Mcp,
            intent: SessionIntent::Mcp {
                args: args.map(str::to_owned),
            },
        }))),
    )
}

/// The feature RPCs a step said: their tickets, commands and `mcpServerStatus/list` parameters.
fn lists(fx: &AppFx) -> Vec<(Ticket, FeatureCommand, Value)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Feature {
                ticket,
                command: command @ FeatureCommand::McpInventory(_),
                ..
            } => {
                let request = features::wire(command, &ThreadId::new(T), RequestId::Integer(1));
                let value = serde_json::to_value(request).expect("serializes");
                Some((*ticket, command.clone(), value["params"].clone()))
            }
            _ => None,
        })
        .collect()
}

fn page(servers: Value, next: Option<&str>) -> Value {
    json!({"data": servers, "nextCursor": next})
}

fn server(name: &str, runtime: Value, auth: &str, tools: &[&str]) -> Value {
    let tools: serde_json::Map<String, Value> = tools
        .iter()
        .map(|tool| {
            (
                (*tool).to_owned(),
                json!({"name": tool, "inputSchema": {"type": "object"}}),
            )
        })
        .collect();
    json!({"name": name, "runtimeStatus": runtime, "pluginId": null, "httpOrigin": null,
        "serverInfo": null, "serverCapabilities": null, "toolListMeta": null, "tools": tools,
        "toolsError": null, "resources": [], "resourceTemplates": [], "authStatus": auth})
}

fn answer(app: &mut CodexApp, asked: &(Ticket, FeatureCommand, Value), result: Value) -> AppFx {
    let reply = features::decode(&asked.1, result).expect("decodes");
    run(app, AppEv::Reply(asked.0, Reply::feature(reply)))
}

fn loading(app: &CodexApp) -> bool {
    app.features()
        .mcp_inventory
        .loading(Some(&ThreadId::new(T)))
        .is_some()
}

fn inventories(app: &CodexApp) -> Vec<McpNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(Note {
                key: NoteKey::Feature(FeatureNote::McpInventory(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn mcp_asks_for_the_summary_and_verbose_for_the_full_detail_with_the_loading_row_between() {
    let mut app = ready();
    let fx = mcp(&mut app, None);
    let asked = lists(&fx);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].2["detail"], "toolsAndAuthOnly");
    assert_eq!(asked[0].2["threadId"], T);
    assert_eq!(asked[0].2["limit"], 100);
    assert!(loading(&app), "the loading row");

    let fx = mcp(&mut app, Some("verbose"));
    assert_eq!(lists(&fx)[0].2["detail"], "full");
}

#[test]
fn pages_are_gathered_and_the_servers_come_out_sorted_with_their_connection() {
    let mut app = ready();
    let first = lists(&mcp(&mut app, None))[0].clone();
    let fx = answer(
        &mut app,
        &first,
        page(
            json!([server("zed", json!("connected"), "oAuth", &["b", "a"])]),
            Some("c2"),
        ),
    );
    let next = lists(&fx);
    assert_eq!(next.len(), 1, "the next page is asked for");
    assert_eq!(next[0].2["cursor"], "c2");
    assert!(
        inventories(&app).is_empty(),
        "nothing said before the last page"
    );

    answer(
        &mut app,
        &next[0],
        page(
            json!([server("alpha", json!(null), "notLoggedIn", &[])]),
            None,
        ),
    );
    let notes = inventories(&app);
    assert_eq!(notes.len(), 1);
    let McpNote::Inventory(inventory) = &notes[0] else {
        panic!("{notes:?}");
    };
    assert!(!inventory.full);
    assert_eq!(
        inventory
            .servers
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zed"]
    );
    assert_eq!(
        inventory.servers[0].connection,
        Connection::AuthenticationRequired,
        "no runtime state and not logged in needs authentication"
    );
    assert_eq!(inventory.servers[0].auth, Auth::NotLoggedIn);
    assert_eq!(inventory.servers[1].tools, ["a", "b"]);
    assert!(!loading(&app), "the row is gone");
}

#[test]
fn no_servers_is_the_empty_note_and_a_failure_is_its_own() {
    let mut app = ready();
    let asked = lists(&mcp(&mut app, Some("verbose")))[0].clone();
    answer(&mut app, &asked, page(json!([]), None));
    assert_eq!(inventories(&app), vec![McpNote::Empty]);

    let asked = lists(&mcp(&mut app, None))[0].clone();
    run(&mut app, AppEv::Reply(asked.0, Reply::failed("boom")));
    assert_eq!(
        inventories(&app).last(),
        Some(&McpNote::Failed("boom".to_owned()))
    );
}
