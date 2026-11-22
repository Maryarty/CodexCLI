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

pub mod apps;
pub mod layout;
pub mod list;
pub mod models;
pub mod permissions;
pub mod persist;
pub mod reasoning;
pub use list::List;
pub use models::{Catalog, Current, ModelMenu, Preset, Selection};
use models::{Effort, PlanSource, Step};
pub use permissions::{PermissionsMenu, Profiles};

pub const NAME: &str = "select";
pub const ID: &str = "S14";
pub const KIND: Kind = Kind::Sheet;

/// A question with two answers, No first: the oracle's confirmations of
/// `/archive` and `/delete` (`SelectionViewParams::confirmation`). The words
/// are the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// Archive the session; `running` says a turn is in progress, which the
    /// archive stops.
    Archive { running: bool },
    /// Delete the session for good.
    Delete,
}

impl Confirm {
    /// The ids of its two rows (`Intent::Choose`); No is the first row.
    pub const NO: &'static str = "no";
    pub const YES: &'static str = "yes";
}

/// A menu the generic selection dialog shows; one variant per
/// parameterisation, its rows data (the words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// A menu of the `/model` family (`models.rs`): its rows come from the
    /// catalog and the thread's settings as they were when it opened.
    Model(Box<ModelMenu>),
    /// A menu of the `/permissions` family (`permissions.rs`): the presets,
    /// the server's profiles, the full-access confirmation.
    Permissions(Box<PermissionsMenu>),
    /// `/apps`: the directory of apps, its rows `Model::apps` (loading, the
    /// failure with its Retry row, or the list).
    Apps,
    /// A confirmation: No (row 0), then Yes (row 1).
    Confirm(Confirm),
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
    /// A permission mode or profile of the `/permissions` family, and how it
    /// applies.
    Permissions(permissions::Selection),
    /// The service tier the model's tier command switched to (`default` is the one that
    /// leaves the tier).
    Tier(String),
    /// A step of a reasoning shortcut: the thread's effort for its next turns, nothing saved;
    /// in Plan mode (`plan`) the Plan override only.
    Effort { model: String, effort: Effort, plan: bool },
    /// The answer of a confirmation was Yes (No picks nothing).
    Confirmed(Confirm),
    /// An item of a generic list, by id.
    Item(String),
    /// The secondary action of an item of a generic list, by id.
    ItemSecondary(String),
    /// A multi-select list (`List::multi`) was confirmed: the ids of the
    /// items whose toggle is on, in the order of the list.
    Items(Vec<String>),
    /// An app of the directory (`Model::apps`), by id: its page opens in the
    /// browser (the app link dialog) or says it has no link.
    App(String),
}

/// A typed menu as rows: what the view turns into its list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Model(models::Page),
    Permissions(permissions::Page),
    Apps(apps::Page),
}

/// What the picker says in the history (`add_info_message`/`add_error_message`):
/// data, the cell is the history's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `No additional models are available right now.`
    NoAdditionalModels,
    /// `Could not load models. Try /model again.`
    ModelsNotLoaded,
    /// `Model changed to {model} {effort}` (`for this session only` when the
    /// pick was the `s` key), after the pick was applied.
    ModelChanged {
        model: String,
        effort: Option<Effort>,
        session: bool,
    },
    /// The Ultra concurrency warning of a pick
    /// (`ultra_reasoning_concurrency_warning`): the threads configured.
    UltraConcurrency(usize),
    /// `Permissions updated to {label}`.
    PermissionsUpdated(permissions::Label),
    /// `Service tier set to {tier}`, after the default was saved.
    ServiceTierSet(String),
    /// `Reasoning shortcuts are disabled until startup completes.`
    ReasoningDisabled,
    /// `Reasoning shortcuts are unavailable for {model}.`
    ReasoningUnavailable(String),
    /// `Reasoning is already at the lowest level ({effort}).` / `...highest...`
    ReasoningBound { direction: reasoning::Direction, effort: Effort },
    /// `Ultra is available under /model -> ... -> More reasoning...`: a raise stops before Ultra.
    UltraAvailable(String),
    /// `Permission selection requested: {label}`: the server took a profile
    /// pick of a thread.
    PermissionSelectionRequested(permissions::Label),
    /// `Failed to save default model: {error}`, or `Failed to set permission
    /// profile`: what was being stored and the server's text.
    SaveFailed { setting: Setting, error: String },
    /// `Saved {setting}, but a higher-priority configuration layer overrides
    /// the saved value.`
    Overridden(Setting),
    /// `Apps are disabled.` and the hint `Enable the apps feature to use $ or
    /// /apps.`
    AppsDisabled,
    /// `No apps available.`
    NoApps,
    /// The directory failed before: the error as an error cell.
    AppsFailed(String),
    /// `{status} · App link unavailable`: an app without an install URL was
    /// picked.
    AppLinkUnavailable(apps::Status),
}

impl Note {
    /// The oracle says each as information or as an error.
    pub fn tone(&self) -> crate::threads::Tone {
        use crate::threads::Tone;
        match self {
            Note::ModelsNotLoaded | Note::SaveFailed { .. } | Note::AppsFailed(_) => Tone::Error,
            _ => Tone::Info,
        }
    }

    /// Whether the main transcript shows the line: the two warnings
    /// (`add_warning_message`: `visible_in_transcript: false`) are for the
    /// detailed transcript only.
    pub fn in_transcript(&self) -> bool {
        !matches!(self, Note::UltraConcurrency(_) | Note::Overridden(_))
    }
}

/// What a failed or overridden write was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// The default model and reasoning effort.
    Model,
    /// The Plan mode's reasoning effort.
    PlanEffort,
    /// The thread's permissions.
    Permissions,
    /// The default service tier.
    ServiceTier,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The menus open, the one on top is shown; a menu below stays until the
    /// ones above it are closed or one of them is picked from.
    pub stack: Vec<Menu>,
    /// The models the pickers list.
    pub catalog: Catalog,
    /// The permission profiles the pickers list.
    pub profiles: Profiles,
    /// The apps `/apps` lists.
    pub apps: apps::Apps,
    /// Picks made; the effect that stores them is the settings slice's.
    pub picked: Vec<Picked>,
    /// Notes for the history; whoever shows them takes them.
    pub notes: Vec<Note>,
    /// The id of the row that opened the menu above the one at this depth of
    /// the stack: Esc shows that menu again with the cursor on it (the
    /// oracle's parent view keeps its highlight).
    kept: std::collections::BTreeMap<usize, String>,
}

impl Model {
    /// The menu shown now.
    pub fn menu(&self) -> Option<&Menu> {
        self.stack.last()
    }

    /// The rows of the menu shown now, if it is one of the typed families.
    pub fn page(&self) -> Option<Page> {
        match self.menu()? {
            Menu::Model(menu) => {
                let mut page = menu.page(&self.catalog);
                let kept = self.kept.get(&(self.stack.len() - 1));
                if let Some(at) = kept.and_then(|id| page.rows.iter().position(|row| &row.id == id))
                {
                    page.initial = Some(at);
                }
                Some(Page::Model(page))
            }
            Menu::Permissions(menu) => Some(Page::Permissions(menu.page(&self.profiles))),
            Menu::Apps => Some(Page::Apps(self.apps.page())),
            Menu::Confirm(_) | Menu::List(_) => None,
        }
    }

    /// `/apps` (`add_connectors_output`): the directory is asked for again,
    /// and what it held before decides what the user sees.
    fn open_apps(&mut self, enabled: bool) {
        if !enabled {
            self.notes.push(Note::AppsDisabled);
            return;
        }
        let held = self.apps.cache.clone();
        self.apps.open();
        match held {
            apps::Cache::Ready(apps) if apps.is_empty() => self.notes.push(Note::NoApps),
            apps::Cache::Failed(error) => self.notes.push(Note::AppsFailed(error)),
            _ => self.stack = vec![Menu::Apps],
        }
    }

    /// A row of the apps menu: `Retry` reloads, an app opens its page (or
    /// says it has no link).
    fn choose_app(&mut self, id: &str) {
        if id == apps::RETRY {
            if matches!(self.apps.cache, apps::Cache::Failed(_)) {
                self.apps.refresh(true);
            }
            return;
        }
        let Some(app) = self
            .apps
            .ready()
            .and_then(|apps| apps.iter().find(|app| app.id == id))
        else {
            return;
        };
        let note = app
            .install_url
            .is_none()
            .then(|| Note::AppLinkUnavailable(app.status()));
        self.notes.extend(note);
        self.picked.push(Picked::App(id.to_string()));
        self.stack.pop();
    }

    /// What a pick of the row `id` did to the stack; a menu pushed over this
    /// one remembers the row that opened it.
    fn step_from(&mut self, id: String, step: Step) {
        if matches!(step, Step::Push(_)) {
            self.kept.insert(self.stack.len() - 1, id);
        }
        self.step(step);
    }

    /// Forgets the rows kept for menus that are gone.
    fn tidy(&mut self) {
        let depth = self.stack.len();
        self.kept.retain(|at, _| *at < depth);
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

    fn permissions_step(&mut self, step: permissions::Step) {
        use permissions::Step;
        match step {
            Step::None => {}
            Step::Replace(mut menu) => {
                // The catalog is the app's cache of `model/list`
                // (`model_catalog.try_list_models()`): unknown stays plain.
                if let permissions::PermissionsMenu::FullAccess { current, .. } = &mut menu {
                    let cyber = current
                        .model
                        .as_deref()
                        .is_some_and(|model| self.catalog.is_cyber(model));
                    current.cyber |= cyber;
                }
                self.stack.pop();
                self.stack.push(Menu::Permissions(Box::new(menu)));
            }
            Step::Done(selection) => {
                self.stack.clear();
                self.picked.push(Picked::Permissions(selection));
            }
            Step::Reopen(current) | Step::Retry(current) => self.open_permissions(current),
        }
    }

    /// `open_permissions_popup`: the legacy list of presets, or (with a
    /// thread, the explicit profile mode or a custom active profile) the
    /// server's catalog: the menu on the one held, or `Loading…` and a
    /// request while there is none (`request_permission_profiles`).
    fn open_permissions(&mut self, current: permissions::Current) {
        use permissions::PermissionsMenu as Perms;
        self.stack.clear();
        // The cyber warning of full access reads the model catalog: have it
        // before the confirmation (a request in flight serves as well).
        if self.catalog.presets.is_empty() && self.catalog.pending.is_none() {
            self.catalog.request();
        }
        let menu = if !current.discover {
            Perms::Presets(current)
        } else if self.profiles.found.is_some() {
            Perms::Profiles(current)
        } else {
            // A request already out serves this menu too.
            if self.profiles.pending.is_none() {
                self.profiles.request();
            }
            Perms::Loading(current)
        };
        self.stack.push(Menu::Permissions(Box::new(menu)));
    }

    /// The answer of the `permissionProfile/list` request: the menu waiting on
    /// it (the one on top) shows the catalog, or why there is none
    /// (`on_permission_profiles_loaded`).
    fn discovered(&mut self, result: Result<permissions::Discovery, permissions::Failure>) {
        use permissions::PermissionsMenu as Perms;
        if let Ok(discovery) = &result {
            self.profiles.found = Some(discovery.clone());
        }
        let Some(Menu::Permissions(menu)) = self.stack.last() else {
            return;
        };
        let Perms::Loading(current) = &**menu else {
            return;
        };
        let current = current.clone();
        let next = match result {
            Ok(_) => Perms::Profiles(current),
            Err(failure) => Perms::Failed { current, failure },
        };
        self.stack.pop();
        self.stack.push(Menu::Permissions(Box::new(next)));
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
        self.notes
            .extend(std::iter::repeat_n(Note::NoAdditionalModels, dismissed));
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
    /// The permission catalog the app holds; menus open on it show it in
    /// place.
    Listed(permissions::Discovery),
    /// The answer of the `permissionProfile/list` request `request`
    /// (`Profiles::pending`); the profiles of every page and the
    /// requirements, or why they could not be read.
    Discovered {
        request: u64,
        result: Result<permissions::Discovery, permissions::Failure>,
    },
    /// The account or the scope changed (`invalidate_permission_discovery`):
    /// the answer in flight is no longer wanted and the menus that waited on
    /// it or showed it are dismissed.
    ScopeChanged,
    /// The answer of the `model/list` request `request` (`Catalog::pending`).
    Fetched {
        request: u64,
        result: Result<Vec<Preset>, String>,
    },
    /// The answer of the `app/list` request `request` (`Apps::pending`).
    AppsLoaded {
        request: u64,
        result: Result<Vec<apps::App>, String>,
    },
    /// The app server says the directory changed
    /// (`refresh_connector_directory_after_notification`).
    AppsNotified(Vec<apps::App>),
    /// An app was switched on or off (`update_connector_enabled`).
    AppSwitched { id: String, enabled: bool },
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
    /// The catalog is asked for, with no menu: the app does it when it connects.
    LoadModels,
    /// `/permissions`: the menu for what the thread has now; the catalog is
    /// asked for when it has none (`Profiles::pending` is the request).
    OpenPermissions(permissions::Current),
    /// `/apps` (`enabled`: the apps feature is on): the directory, asked for
    /// again (`Apps::pending` is the request).
    OpenApps { enabled: bool },
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
    /// `/fast`, `/ultrafast`...: the tier `id` is switched on, or off when it is `current`
    /// (`toggle_service_tier_from_ui`).
    ServiceTier { id: String, current: Option<String> },
    /// A reasoning shortcut on the thread `current` describes: `plan` is the thread in Plan
    /// mode (the step is its override only), `started` the session being configured.
    Reasoning { direction: reasoning::Direction, current: Current, plan: bool, started: bool },
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    apply_ev(model, ev, fx);
    model.tidy();
}

fn apply_ev(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(menu) => model.stack = vec![menu],
        Ev::Loaded(presets) => model.accept(presets),
        Ev::Listed(discovery) => model.profiles.found = Some(discovery),
        Ev::Discovered { request, result } => {
            if model.profiles.pending != Some(request) {
                return;
            }
            model.profiles.pending = None;
            model.discovered(result);
        }
        Ev::ScopeChanged => {
            use permissions::PermissionsMenu as Perms;
            model.profiles.pending = None;
            model.apps.invalidate();
            model.stack.retain(|menu| match menu {
                Menu::Apps => false,
                Menu::Permissions(menu) => !matches!(
                    **menu,
                    Perms::Loading(_) | Perms::Profiles(_) | Perms::Failed { .. }
                ),
                _ => true,
            });
        }
        Ev::AppsLoaded { request, result } => {
            model.apps.loaded(request, result);
        }
        Ev::AppsNotified(apps) => {
            model.apps.notified(apps);
        }
        Ev::AppSwitched { id, enabled } => model.apps.switched(&id, enabled),
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
    apply_choice(model, intent, fx);
    model.tidy();
}

fn apply_choice(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open(menu) => model.stack = vec![menu],
        Intent::OpenModels(current) => {
            model.stack.clear();
            model.catalog.request();
            if model.catalog.presets.is_empty() {
                model
                    .stack
                    .push(Menu::Model(Box::new(ModelMenu::Loading(current))));
            } else {
                model.open_models(current);
            }
        }
        Intent::LoadModels => {
            model.catalog.request();
        }
        Intent::OpenPermissions(current) => model.open_permissions(current),
        Intent::ServiceTier { id, current } => {
            let next = if current.as_deref() == Some(id.as_str()) {
                codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()
            } else {
                id
            };
            model.picked.push(Picked::Tier(next));
        }
        Intent::Reasoning { direction, current, plan, started } => {
            match reasoning::step(&model.catalog, &current, direction, started) {
                reasoning::Outcome::Set(effort) => {
                    model.picked.push(Picked::Effort { model: current.model, effort, plan });
                }
                reasoning::Outcome::Say(note) => model.notes.push(note),
            }
        }
        Intent::OpenApps { enabled } => model.open_apps(enabled),
        Intent::Choose(id) => match model.stack.last_mut() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, false);
                model.step_from(id, step);
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
            Some(Menu::Permissions(menu)) => {
                let step = menu.choose(&model.profiles, &id);
                model.permissions_step(step);
            }
            Some(Menu::Apps) => model.choose_app(&id),
            Some(Menu::Confirm(confirm)) => {
                let confirm = *confirm;
                if id == Confirm::YES {
                    model.picked.push(Picked::Confirmed(confirm));
                }
                if id == Confirm::YES || id == Confirm::NO {
                    model.stack.pop();
                }
            }
            None => {}
        },
        Intent::Secondary(id) => match model.stack.last() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, true);
                model.step_from(id, step);
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

/// The RPCs of this screen. They are about the app (the catalog of models,
/// the server's permission profiles), not about one thread: the app sends
/// them (`app/select.rs`) as a screen command with no thread, under a ticket
/// of its own, and the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `model/list` for the request `Catalog::pending` names.
    Models { request: u64 },
    /// `configRequirements/read`, the first half of a permission read.
    Requirements { request: u64 },
    /// One page of `permissionProfile/list` (100 per page, at most 10).
    Profiles {
        request: u64,
        cursor: Option<String>,
        cwd: Option<String>,
    },
    /// `app/list` for the request `Apps::pending` names (`force`: past the
    /// server's caches), for the thread displayed.
    Apps {
        request: u64,
        force: bool,
        thread: Option<String>,
    },
    /// `thread/settings/update` of `thread`: what a pick changes for its
    /// next turns.
    Settings {
        thread: String,
        patch: crate::threads::SettingsPatch,
    },
    /// `config/batchWrite`: the defaults a pick saves.
    Write { edits: Vec<persist::Edit> },
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Models(Vec<Preset>),
    Apps(Vec<apps::App>),
    Requirements(Option<permissions::Requirements>),
    Profiles {
        profiles: Vec<permissions::Profile>,
        next: Option<String>,
    },
    /// The thread's settings were updated.
    Updated,
    /// The config was written; whether a layer above overrides the value.
    Written {
        overridden: bool,
    },
}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Models {
        request: u64,
    },
    Apps {
        request: u64,
    },
    Requirements {
        request: u64,
    },
    /// The pages of the catalog read so far, and the requirements read before
    /// them.
    Profiles {
        request: u64,
        requirements: Option<permissions::Requirements>,
        found: Vec<permissions::Profile>,
        pages: u8,
    },
    /// A pick's settings update: what to tell if it fails, and the line that
    /// follows once the server took it.
    Settings {
        setting: Setting,
        then: Option<Note>,
    },
    /// A pick's write: the pick whose line follows a saved write.
    Write {
        setting: Setting,
        then: Option<Note>,
    },
}

/// The pages a catalog may take (`permission_discovery.rs`).
pub const MAX_PAGES: u8 = 10;

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
    // The replies are the app's (`app/select.rs`): no thread asks for them.
    let _ = (part, ctx, result, fx);
}

/// The wire request of `command`; the thread is none.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    match command {
        Command::Settings { thread, patch } => crate::threads::ThreadAsk::SettingsUpdate {
            thread: thread.clone(),
            patch: patch.clone(),
        }
        .wire(id),
        Command::Write { edits } => persist::write_request(edits, id),
        Command::Models { .. } => models::list_request(id),
        Command::Apps { force, thread, .. } => apps::list_request(id, *force, thread.clone()),
        Command::Requirements { .. } => permissions::requirements_request(id),
        Command::Profiles { cursor, cwd, .. } => {
            permissions::profiles_request(id, cursor.clone(), cwd.clone())
        }
    }
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::Settings { .. } => Ok(Reply::Updated),
        Command::Write { .. } => {
            persist::written(result).map(|overridden| Reply::Written { overridden })
        }
        Command::Models { .. } => models::presets(result).map(Reply::Models),
        Command::Apps { .. } => apps::apps(result).map(Reply::Apps),
        Command::Requirements { .. } => permissions::requirements(result).map(|wire| {
            Reply::Requirements(wire.as_ref().map(permissions::Requirements::from_wire))
        }),
        Command::Profiles { .. } => {
            permissions::profiles_page(result).map(|(wire, next)| Reply::Profiles {
                profiles: permissions::profiles_of(wire),
                next,
            })
        }
    }
    .map_err(RpcError::Malformed)
}

/// The seed form of this screen's events: `{"kind":"catalog","presets":[..]}`
/// (the catalog the app started with), the menus of the `/model` family
/// (`{"kind":"models","current":{"model":..}}`, `all_models`, `loading`,
/// `reasoning` and `advanced` with a `preset`), `{"kind":"profiles",
/// "discovery":{..}}` (the permission catalog), `{"kind":"permissions",
/// "menu":"profiles","current":{..}}` and `{"kind":"list", ..}` (the fields
/// of `List`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Catalog {
        presets: Vec<Preset>,
    },
    Loading {
        current: Current,
    },
    Models {
        current: Current,
    },
    AllModels {
        current: Current,
    },
    Reasoning {
        preset: Preset,
        current: Current,
    },
    Advanced {
        preset: Preset,
        current: Current,
    },
    PlanScope {
        model: String,
        effort: Option<Effort>,
        source: PlanSource,
        #[serde(default)]
        warning: Option<usize>,
    },
    List(Box<List>),
    /// The profiles the app holds (a startup read of the catalog).
    Profiles {
        discovery: permissions::Discovery,
    },
    /// A menu of the `/permissions` family: the legacy presets unless `menu`
    /// says otherwise; `failure` is the text of a read that failed.
    Permissions {
        #[serde(default)]
        menu: SeedPermissions,
        #[serde(default)]
        current: permissions::Current,
        #[serde(default)]
        failure: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedPermissions {
    #[default]
    Presets,
    Loading,
    Profiles,
    Failed,
    /// The confirmation `Enable full access?` (the pick is Full Access).
    FullAccess,
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
        SeedEv::PlanScope {
            model: name,
            effort,
            source,
            warning,
        } => model(ModelMenu::PlanScope {
            model: name,
            effort,
            source,
            warning,
        }),
        SeedEv::List(list) => Ev::Opened(Menu::List(list)),
        SeedEv::Profiles { discovery } => Ev::Listed(discovery),
        SeedEv::Permissions {
            menu,
            current,
            failure,
        } => {
            use permissions::PermissionsMenu as Perms;
            let menu = match menu {
                SeedPermissions::Presets => Perms::Presets(current),
                SeedPermissions::Loading => Perms::Loading(current),
                SeedPermissions::Profiles => Perms::Profiles(current),
                SeedPermissions::Failed => Perms::Failed {
                    current,
                    failure: permissions::Failure::Other(failure.unwrap_or_default()),
                },
                SeedPermissions::FullAccess => {
                    let pick = permissions::Selection::Preset {
                        preset: permissions::Preset::FullAccess,
                        reviewer: permissions::Reviewer::User,
                    };
                    Perms::FullAccess { current, pick }
                }
            };
            Ev::Opened(Menu::Permissions(Box::new(menu)))
        }
    })
}
