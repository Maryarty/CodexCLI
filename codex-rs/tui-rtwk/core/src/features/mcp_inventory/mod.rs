//! `/mcp` as a feature slice (ADR-012; oracle `history_cell/mcp.rs`
//! `new_mcp_tools_output_from_statuses`, `new_mcp_inventory_loading`, `app/background_requests.rs`
//! `fetch_mcp_inventory`, `handle_mcp_inventory_result`): the inventory of the MCP servers the
//! thread in use runs, asked of the server a page at a time, and the cell that says it.
//!
//! `/mcp` asks for the connection summary (`ToolsAndAuthOnly`), `/mcp verbose` for the tools and
//! resources too (`Full`); `/mcp login <server>` is not this feature's. While the ask is out the
//! feature keeps a [`Loading`] row by thread (`State::loading`: the row the transcript shows
//! until the answer, never a transcript entry, so nothing has to be taken back out of it). The
//! answer is one note of the thread, [`Note::Inventory`] (the servers sorted by name, with the
//! detail asked for) or [`Note::Empty`] for no servers, or `Failed to load MCP inventory: ...`;
//! the words and the cell's layout are the UI's (`src/features/mcp_inventory/`). An answer for a
//! thread that is no longer the one in use is dropped (oracle: `current_displayed_thread_id`),
//! and so is one of a request that a later `/mcp` replaced.

mod wire;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use wire::{Command, Reply, server};

/// The marker type of the feature.
pub struct McpInventory;

/// What the user asks of `/mcp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/mcp` (`full: false`) and `/mcp verbose`, typed in the thread `source` (none before the
    /// session has started: the app's own servers).
    Show {
        source: Option<ThreadId>,
        full: bool,
    },
}

/// Which request an answer belongs to: the thread it was asked for and its generation (a later
/// `/mcp` of the same thread replaces the earlier one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    pub thread: Option<ThreadId>,
    pub gen_: Gen,
}

/// Which request for an inventory an answer belongs to.
pub type Gen = u64;

/// How a server is connected (oracle `McpServerConnectionStatus`, with the unknown state of a
/// server that said none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Connected,
    Starting,
    AuthenticationRequired,
    Failed,
    NotStarted,
    Disabled,
    Cancelled,
    Unknown,
}

/// How a server authenticates (oracle `McpAuthStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    Unknown,
    Unsupported,
    NotLoggedIn,
    BearerToken,
    OAuth,
}

/// A resource or a resource template: its title, else its name, and its uri (template).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub uri: String,
}

/// One server of the inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    pub name: String,
    pub connection: Connection,
    pub auth: Auth,
    /// The tool names, sorted. Their count is the summary's `(N tools)`.
    pub tools: Vec<String>,
    /// Empty unless the full detail was asked for.
    pub resources: Vec<Entry>,
    pub templates: Vec<Entry>,
}

/// The inventory the cell lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    /// Sorted by name.
    pub servers: Vec<Server>,
    /// `/mcp verbose`: the tools, resources and templates are listed; else the summary says
    /// `Use /mcp verbose for tools and resources.`
    pub full: bool,
}

impl Inventory {
    /// The `No MCP tools available.` line leads a full inventory in which no server has a tool.
    pub fn no_tools(&self) -> bool {
        self.full && self.servers.iter().all(|server| server.tools.is_empty())
    }
}

/// The loading row: an ask for the inventory is out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loading {
    pub full: bool,
}

/// The asks out, by thread (the app's own under the default id), with the pages gathered so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    asks: BTreeMap<ThreadId, Ask>,
    asked: Gen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ask {
    gen_: Gen,
    full: bool,
    servers: Vec<Server>,
}

impl State {
    /// The loading row of `thread` (none: the app's own), while its ask is out.
    pub fn loading(&self, thread: Option<&ThreadId>) -> Option<Loading> {
        self.asks
            .get(&thread.cloned().unwrap_or_default())
            .map(|ask| Loading { full: ask.full })
    }
}

/// What `/mcp` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The inventory (`/mcp`, `/mcp verbose`).
    Inventory(Inventory),
    /// The server runs no MCP server at all (oracle `empty_mcp_output`).
    Empty,
    /// `Failed to load MCP inventory: {error}`
    Failed(String),
}

/// The key of the asks: a request about the app has the default thread id.
fn key(thread: &Option<ThreadId>) -> ThreadId {
    thread.clone().unwrap_or_default()
}

fn ask(
    cx: &mut AppCx<'_>,
    thread: &Option<ThreadId>,
    gen_: Gen,
    full: bool,
    cursor: Option<String>,
) {
    cx.ask(
        thread.as_ref(),
        Command::List {
            thread: thread.clone(),
            full,
            cursor,
        },
        Ctx {
            thread: thread.clone(),
            gen_,
        },
    );
}

impl Feature for McpInventory {
    const NAME: &'static str = "mcp_inventory";

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    /// The thread is asked about only when this process knows it (oracle
    /// `mcp_inventory_request_thread_id`: the thread in use that is not closed).
    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Show { source, full } = intent;
        let thread = source.filter(|id| cx.view().thread(id).is_some());
        state.asked += 1;
        let gen_ = state.asked;
        state.asks.insert(
            key(&thread),
            Ask {
                gen_,
                full,
                servers: Vec::new(),
            },
        );
        ask(cx, &thread, gen_, full, None);
    }

    fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let slot = key(&ctx.thread);
        let Some(current) = state.asks.get_mut(&slot).filter(|a| a.gen_ == ctx.gen_) else {
            return;
        };
        match result {
            Ok(Reply::Page {
                servers,
                next_cursor,
            }) => {
                current.servers.extend(servers);
                if next_cursor.is_some() {
                    let full = current.full;
                    return ask(cx, &ctx.thread, ctx.gen_, full, next_cursor);
                }
            }
            Err(error) => {
                state.asks.remove(&slot);
                return say(cx, &ctx.thread, Note::Failed(error_text(&error)));
            }
        }
        let Some(done) = state.asks.remove(&slot) else {
            return;
        };
        let note = if done.servers.is_empty() {
            Note::Empty
        } else {
            let mut servers = done.servers;
            servers.sort_by(|a, b| a.name.cmp(&b.name));
            Note::Inventory(Inventory {
                servers,
                full: done.full,
            })
        };
        say(cx, &ctx.thread, note);
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::Failed(_) => Tone::Error,
            Note::Inventory(_) | Note::Empty => Tone::Info,
        }
    }

    fn wire(command: &Command, _thread: &ThreadId, id: RequestId) -> ClientRequest {
        wire::request(command, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        wire::reply(command, result)
    }
}

/// The note, unless the thread it is for is no longer the one in use (oracle
/// `handle_mcp_inventory_result`).
fn say(cx: &mut AppCx<'_>, thread: &Option<ThreadId>, note: Note) {
    if thread.is_some() && thread.as_ref() != cx.view().active() {
        return;
    }
    cx.note(thread.clone(), note);
}

fn error_text(error: &RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) => text.clone(),
        RpcError::Mismatch => "unexpected reply".to_owned(),
        RpcError::Prepare(text) => text.clone(),
        RpcError::Provider(_) => "model provider refused the request".to_owned(),
    }
}
