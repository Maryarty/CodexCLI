//! `select` (S14, generic selection dialog): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/select/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "select";
pub const ID: &str = "S14";
pub const KIND: Kind = Kind::Sheet;

/// A model of the picker: its name, description and whether it is the one in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRow {
    pub name: String,
    pub description: String,
    pub current: bool,
}

/// The permission presets of `/permissions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Read and edit workspace files, approval for the rest.
    Ask,
    /// No approvals, no sandbox.
    Full,
}

/// A menu the generic selection dialog shows; one variant per
/// parameterisation, its rows data (the words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    Models(Vec<ModelRow>),
    Permissions { current: Option<Access> },
}

impl Menu {
    /// How many rows the menu has.
    pub fn len(&self) -> usize {
        match self {
            Menu::Models(rows) => rows.len(),
            Menu::Permissions { .. } => 2,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// What the user picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    Model(String),
    Access(Access),
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The menu open now.
    pub menu: Option<Menu>,
    /// Picks made; the effect that stores them is the settings slice's.
    pub picked: Vec<Picked>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A menu was opened.
    Opened(Menu),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The row at this index was picked: the menu closes.
    Pick(usize),
    /// The menu was closed without a pick.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(menu) => model.menu = Some(menu),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Pick(index) => {
            let picked = match &model.menu {
                Some(Menu::Models(rows)) => rows.get(index).map(|row| Picked::Model(row.name.clone())),
                Some(Menu::Permissions { .. }) => match index {
                    0 => Some(Picked::Access(Access::Ask)),
                    1 => Some(Picked::Access(Access::Full)),
                    _ => None,
                },
                None => None,
            };
            if let Some(picked) = picked {
                model.picked.push(picked);
                model.menu = None;
            }
        }
        Intent::Close => model.menu = None,
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// The seed form of this screen's events:
/// `{"kind":"models","rows":[{"name":..,"description":..,"current":true}]}` and
/// `{"kind":"permissions","current":"ask"}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Models { rows: Vec<SeedRow> },
    Permissions {
        #[serde(default)]
        current: Option<SeedAccess>,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedRow {
    name: String,
    description: String,
    #[serde(default)]
    current: bool,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedAccess {
    Ask,
    Full,
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(Ev::Opened(match seed {
        SeedEv::Models { rows } => Menu::Models(
            rows.into_iter()
                .map(|row| ModelRow {
                    name: row.name,
                    description: row.description,
                    current: row.current,
                })
                .collect(),
        ),
        SeedEv::Permissions { current } => Menu::Permissions {
            current: current.map(|access| match access {
                SeedAccess::Ask => Access::Ask,
                SeedAccess::Full => Access::Full,
            }),
        },
    }))
}
