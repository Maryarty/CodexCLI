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

pub mod layout;
pub mod list;
pub mod models;
pub use list::List;
pub use models::{Catalog, Current, ModelMenu, Preset, Selection};
use models::{Effort, PlanSource, Step};

pub const NAME: &str = "select";
pub const ID: &str = "S14";
pub const KIND: Kind = Kind::Sheet;

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
    /// A menu of the `/model` family (`models.rs`): its rows come from the
    /// catalog and the thread's settings as they were when it opened.
    Model(Box<ModelMenu>),
    Permissions { current: Option<Access> },
    /// The generic form: every option of the oracle's `SelectionViewParams`
    /// (search, tabs, disabled rows, side preview, footers) as data. A named
    /// parameterisation is a variant above whose view turns it into a `List`.
    List(Box<List>),
}

/// What the user picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    /// A model and effort of the `/model` family, and where it applies.
    Selection(Selection),
    Access(Access),
    /// An item of a generic list, by id.
    Item(String),
    /// The secondary action of an item of a generic list, by id.
    ItemSecondary(String),
    /// A multi-select list (`List::multi`) was confirmed: the ids of the
    /// items whose toggle is on, in the order of the list.
    Items(Vec<String>),
}

/// What the picker says in the history (`add_info_message`/`add_error_message`):
/// data, the cell is the history's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `No additional models are available right now.`
    NoAdditionalModels,
    /// `Could not load models. Try /model again.`
    ModelsNotLoaded,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The menus open, the one on top is shown; a menu below stays until the
    /// ones above it are closed or one of them is picked from.
    pub stack: Vec<Menu>,
    /// The models the pickers list.
    pub catalog: Catalog,
    /// Picks made; the effect that stores them is the settings slice's.
    pub picked: Vec<Picked>,
    /// Notes for the history; whoever shows them takes them.
    pub notes: Vec<Note>,
}

impl Model {
    /// The menu shown now.
    pub fn menu(&self) -> Option<&Menu> {
        self.stack.last()
    }

    /// The rows of the menu shown now, if it is one of the `/model` family.
    pub fn page(&self) -> Option<models::Page> {
        match self.menu()? {
            Menu::Model(menu) => Some(menu.page(&self.catalog)),
            Menu::Permissions { .. } | Menu::List(_) => None,
        }
    }

    fn step(&mut self, step: Step) {
        match step {
            Step::None => {}
            Step::Push(menu) => self.stack.push(Menu::Model(Box::new(menu))),
            Step::Replace(menu) => {
                self.stack.pop();
                self.stack.push(Menu::Model(Box::new(menu)));
            }
            Step::Done(selection) => {
                self.stack.clear();
                self.picked.push(Picked::Selection(selection));
            }
            Step::Prompt(menu) => {
                self.stack.clear();
                self.stack.push(Menu::Model(Box::new(menu)));
            }
        }
    }

    /// `open_model_popup_with_presets`: the quick menu, or a note when the
    /// catalog has nothing to offer.
    fn open_models(&mut self, current: Current) {
        let menu = ModelMenu::Models(current);
        if menu.is_empty(&self.catalog) {
            self.notes.push(Note::NoAdditionalModels);
        } else {
            self.stack.push(Menu::Model(Box::new(menu)));
        }
    }

    /// A catalog arrived (the startup one, or the answer of a request): a
    /// catalog that differs and is not empty replaces the held one, and the
    /// pickers open on it show it without being reopened; one left with no
    /// choice is dismissed (`refresh_open_model_picker`).
    fn accept(&mut self, presets: Vec<Preset>) {
        if presets.is_empty() || self.catalog.presets == presets {
            return;
        }
        self.catalog.presets = presets;
        let catalog = &self.catalog;
        let mut dismissed = 0;
        self.stack.retain(|menu| match menu {
            Menu::Model(menu) if menu.is_empty(catalog) => {
                dismissed += 1;
                false
            }
            _ => true,
        });
        self.notes.extend(std::iter::repeat_n(Note::NoAdditionalModels, dismissed));
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A menu was opened: the only one on the stack.
    Opened(Menu),
    /// The catalog the app started with.
    Loaded(Vec<Preset>),
    /// The answer of the `model/list` request `request` (`Catalog::pending`).
    Fetched { request: u64, result: Result<Vec<Preset>, String> },
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A screen asks for this menu (a generic list: its picks come back as
    /// `Picked::Item(id)` in `Model::picked`).
    Open(Menu),
    /// `/model`: the quick menu on the catalog held (`Loading models…` while
    /// it is empty), and the catalog is asked for again: the request is
    /// `Catalog::pending`.
    OpenModels(Current),
    /// The row at this index was picked: the menu closes.
    Pick(usize),
    /// The item with this id (`List::id_of`, or a row of `Page`) was
    /// accepted.
    Choose(String),
    /// The secondary action of the item with this id (`s for secondary`).
    Secondary(String),
    /// The toggle of the item with this id was flipped.
    Toggle(String),
    /// The orderable item with this id moves one place (`true`: down).
    Reorder(String, bool),
    /// A multi-select list (`List::multi`) was confirmed (Enter): the ids of
    /// the toggled-on items are picked together and the menu closes.
    Confirm,
    /// The menu shown was closed without a pick; the one below shows again.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(menu) => model.stack = vec![menu],
        Ev::Loaded(presets) => model.accept(presets),
        Ev::Fetched { request, result } => {
            // Another request, or none: the answer is not for the pickers.
            if model.catalog.pending != Some(request) {
                return;
            }
            model.catalog.pending = None;
            let loading = match model.stack.last() {
                Some(Menu::Model(menu)) => match &**menu {
                    ModelMenu::Loading(current) => Some(current.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(current) = loading {
                model.stack.pop();
                match result {
                    Ok(presets) => {
                        if !presets.is_empty() {
                            model.catalog.presets = presets;
                        }
                        model.open_models(current);
                    }
                    Err(_) => model.notes.push(Note::ModelsNotLoaded),
                }
            } else if let Ok(presets) = result {
                model.accept(presets);
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open(menu) => model.stack = vec![menu],
        Intent::OpenModels(current) => {
            model.stack.clear();
            model.catalog.request();
            if model.catalog.presets.is_empty() {
                model.stack.push(Menu::Model(Box::new(ModelMenu::Loading(current))));
            } else {
                model.open_models(current);
            }
        }
        Intent::Pick(index) => {
            let picked = match model.menu() {
                Some(Menu::Permissions { .. }) => match index {
                    0 => Some(Picked::Access(Access::Ask)),
                    1 => Some(Picked::Access(Access::Full)),
                    _ => None,
                },
                Some(Menu::Model(_) | Menu::List(_)) | None => None,
            };
            if let Some(picked) = picked {
                model.picked.push(picked);
                model.stack.clear();
            }
        }
        Intent::Choose(id) => match model.stack.last_mut() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, false);
                model.step(step);
            }
            Some(Menu::List(list)) => {
                let Some(item) = list.find(&id) else { return };
                if item.is_disabled() {
                    return;
                }
                let keep = item.keep_open;
                if item.checkbox && item.toggle.is_some() {
                    list.flip(&id);
                    return;
                }
                model.picked.push(Picked::Item(id));
                if !keep {
                    model.stack.pop();
                }
            }
            Some(Menu::Permissions { .. }) | None => {}
        },
        Intent::Secondary(id) => match model.stack.last() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, true);
                model.step(step);
            }
            Some(Menu::List(list)) if list.find(&id).is_some_and(|item| !item.is_disabled()) => {
                model.picked.push(Picked::ItemSecondary(id));
                model.stack.pop();
            }
            _ => {}
        },
        Intent::Toggle(id) => {
            if let Some(Menu::List(list)) = model.stack.last_mut() {
                list.flip(&id);
            }
        }
        Intent::Reorder(id, down) => {
            if let Some(Menu::List(list)) = model.stack.last_mut() {
                list.reorder(&id, down);
            }
        }
        Intent::Confirm => {
            if let Some(Menu::List(list)) = model.stack.last()
                && list.multi
            {
                model.picked.push(Picked::Items(list.checked()));
                model.stack.pop();
            }
        }
        Intent::Close => {
            model.stack.pop();
        }
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

/// The seed form of this screen's events: `{"kind":"catalog","presets":[..]}`
/// (the catalog the app started with), the menus of the `/model` family
/// (`{"kind":"models","current":{"model":..}}`, `all_models`, `loading`,
/// `reasoning` and `advanced` with a `preset`), `{"kind":"permissions",
/// "current":"ask"}` and `{"kind":"list", ..}` (the fields of `List`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Catalog { presets: Vec<Preset> },
    Loading { current: Current },
    Models { current: Current },
    AllModels { current: Current },
    Reasoning { preset: Preset, current: Current },
    Advanced { preset: Preset, current: Current },
    PlanScope {
        model: String,
        effort: Option<Effort>,
        source: PlanSource,
        #[serde(default)]
        warning: Option<usize>,
    },
    List(Box<List>),
    Permissions {
        #[serde(default)]
        current: Option<SeedAccess>,
    },
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
    let model = |menu| Ev::Opened(Menu::Model(Box::new(menu)));
    Ok(match seed {
        SeedEv::Catalog { presets } => Ev::Loaded(presets),
        SeedEv::Loading { current } => model(ModelMenu::Loading(current)),
        SeedEv::Models { current } => model(ModelMenu::Models(current)),
        SeedEv::AllModels { current } => model(ModelMenu::AllModels(current)),
        SeedEv::Reasoning { preset, current } => model(ModelMenu::Reasoning { preset, current }),
        SeedEv::Advanced { preset, current } => model(ModelMenu::Advanced { preset, current }),
        SeedEv::PlanScope { model: name, effort, source, warning } => {
            model(ModelMenu::PlanScope { model: name, effort, source, warning })
        }
        SeedEv::List(list) => Ev::Opened(Menu::List(list)),
        SeedEv::Permissions { current } => Ev::Opened(Menu::Permissions {
            current: current.map(|access| match access {
                SeedAccess::Ask => Access::Ask,
                SeedAccess::Full => Access::Full,
            }),
        }),
    })
}
