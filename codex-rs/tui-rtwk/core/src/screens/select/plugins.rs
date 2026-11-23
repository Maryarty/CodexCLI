//! `/plugins` (oracle `chatwidget/plugins.rs`, `plugin_catalog.rs`, `app/background_requests.rs`):
//! the plugin browser of the select screen. The catalog, its tabs and rows, the detail of a plugin
//! and the flows around them (install and the apps it needs signed in, uninstall, the switch of a
//! plugin, a marketplace added, removed or upgraded) are data here (`catalog.rs` holds the
//! derivations); the words are the view's. The requests are the model's: it names each one
//! (`Plugins::waiting`) and the app sends it (`take_wanted`) and tells the answer back, so an
//! answer for another directory or for a request given up is dropped.

pub mod catalog;

use std::collections::{BTreeMap, BTreeSet};

use codex_app_server_protocol as wire;
use codex_app_server_protocol::{ClientRequest, RequestId};

pub use catalog::{
    Action, Catalog, Config, Configured, Detail, Kind, Location, Marketplace, Part, Placeholder,
    Plugin, Product, Remote, Row, Section, SectionError, Source, Status, Tab, Target, Toggle,
};
use catalog::{ADD, ALL};

/// The rows of the menus (the plugin rows are `catalog::row_id`).
pub const BACK: &str = "plugins.back";
pub const INSTALL: &str = "plugins.install";
pub const UNINSTALL: &str = "plugins.uninstall";
pub const REMOVE: &str = "plugins.remove";
pub const RETRY: &str = "plugins.retry";
/// The row of the Add tab that opens the prompt.
pub const ADD_ROW: &str = "plugins.add";
/// The rows of an app's setup: open its page, go on (the app is installed, or the user says so),
/// give up.
pub const OPEN: &str = "plugins.app.open";
pub const NEXT: &str = "plugins.app.next";
pub const SKIP: &str = "plugins.app.skip";

/// The plugin menu shown (the select screen's `Menu::Plugins` marker says it is on top).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// `Loading plugins...`.
    Loading,
    /// `Plugin marketplace unavailable`, and the error.
    Failed(String),
    /// The tabs of `Plugins::tabs`.
    List,
    /// `Add marketplace`: a text prompt; its answer is `Intent::Source`.
    AddPrompt,
    AddLoading,
    AddFailed,
    /// `Remove marketplace`, by name; `label` is what the tab says.
    RemoveConfirm { name: String, label: String },
    RemoveLoading { label: String },
    RemoveFailed { name: String, label: String },
    /// `Upgrading marketplace...`, the one named or all.
    UpgradeLoading { name: Option<String> },
    DetailLoading { name: String },
    Detail(Box<Detail>),
    /// The detail could not be read or the plugin not installed; `back` when the list can be
    /// shown again.
    DetailFailed { error: String, back: bool },
    InstallLoading { name: String },
    UninstallLoading { name: String },
    /// The setup of the apps a plugin needs (`Model::plugin_auth`).
    Auth,
}

/// An app a plugin needs signed in (`AppSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthApp {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

/// The step of the setup the user is at (`plugin_install_auth_popup_params`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthStep {
    pub plugin: String,
    pub app: AuthApp,
    /// This app's place, from 1, among `total`.
    pub current: usize,
    pub total: usize,
    pub installed: bool,
}

/// What the browser says in the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Say {
    /// The plugins feature is off.
    Disabled,
    /// `Installed {name} plugin.`; the apps still to sign in (none: nothing more is needed).
    Installed { name: String, apps: Vec<String> },
    Uninstalled { name: String },
    /// The setup of the apps was given up, or went through.
    Skipped { name: String },
    Completed { name: String },
    EnableFailed { id: String, error: String },
    MarketAdded { name: String, root: String, already: bool },
    MarketRemoved { label: String, name: String, root: Option<String> },
    /// No configured git marketplace to upgrade.
    NothingToUpgrade,
    UpToDate { selected: Vec<String> },
    Upgraded { roots: Vec<String> },
    UpgradeFailed(Vec<(String, String)>),
    /// The request itself failed.
    UpgradeError(String),
}

impl Say {
    pub fn error(&self) -> bool {
        matches!(
            self,
            Say::EnableFailed { .. } | Say::UpgradeFailed(_) | Say::UpgradeError(_)
        )
    }
}

/// What the app asks the backend for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Want {
    /// `plugin/list`: the local marketplaces.
    List,
    /// `plugin/list` of the marketplaces of one remote section.
    Section(Part),
    Read(Target),
    Install(Target),
    Uninstall { id: String },
    Add { source: String },
    Remove { name: String },
    Upgrade { name: Option<String> },
    /// `config/value/write` of `plugins.<id>`.
    Enable { id: String, enabled: bool },
}

/// A request to send: its number is what the answer is told back under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub request: u64,
    pub want: Want,
    pub cwd: Option<String>,
    /// The home directory: the personal marketplace is under it.
    pub home: Option<String>,
}

/// The decoded answer of a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Catalog(Catalog),
    Detail(Box<Detail>),
    Installed(Vec<AuthApp>),
    Uninstalled,
    Added { name: String, root: String, already: bool },
    Removed { name: String, root: Option<String> },
    Upgraded { selected: Vec<String>, roots: Vec<String>, errors: Vec<(String, String)> },
    Written,
}

/// What is waited for, with what its answer needs.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Pending {
    List,
    Section(Part),
    Read,
    Install { name: String, target: Target },
    Uninstall { name: String },
    Add,
    Remove { name: String, label: String },
    Upgrade,
    Enable { id: String, enabled: bool },
}

/// The list's cache for the directory (`PluginsCacheState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Cache {
    #[default]
    Uninitialized,
    Loading,
    Ready(Catalog),
    Failed,
}

/// The apps a plugin needs, and the one the user is at (`PluginInstallAuthFlowState`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Flow {
    plugin: String,
    apps: Vec<AuthApp>,
    next: usize,
}

/// The sections being loaded after the list (`fetch_additional_plugin_remote_sections`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Collect {
    waiting: BTreeSet<u64>,
    found: Vec<(Part, Vec<Marketplace>)>,
    errors: Vec<SectionError>,
}

/// The state of the browser that outlives a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugins {
    config: Config,
    cwd: Option<String>,
    cache: Cache,
    cached_for: Option<String>,
    in_flight: bool,
    remote: Remote,
    menu: Menu,
    /// The tab the user is at (kept across a refresh).
    tab: String,
    /// The tab of the marketplace just added.
    fresh: Option<String>,
    flow: Option<Flow>,
    issued: u64,
    waiting: BTreeMap<u64, Pending>,
    unsent: Vec<Wanted>,
    collect: Option<Collect>,
    /// The switches the user flipped and the config has not taken yet.
    desired: BTreeMap<String, bool>,
    /// Writes in flight, each with the value asked for after it.
    writing: BTreeMap<String, Option<bool>>,
    said: Vec<Say>,
    /// A plugin was installed, uninstalled, or its marketplaces changed.
    written: bool,
    /// The setup of apps wants to be shown whether or not a menu is.
    shown: bool,
}

impl Default for Plugins {
    fn default() -> Self {
        Self {
            config: Config::default(),
            cwd: None,
            cache: Cache::Uninitialized,
            cached_for: None,
            in_flight: false,
            remote: Remote::default(),
            menu: Menu::Loading,
            tab: ALL.to_string(),
            fresh: None,
            flow: None,
            issued: 0,
            waiting: BTreeMap::new(),
            unsent: Vec::new(),
            collect: None,
            desired: BTreeMap::new(),
            writing: BTreeMap::new(),
            said: Vec::new(),
            written: false,
            shown: false,
        }
    }
}

/// The intents of the browser that are not a pick of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The user moved to the tab with this id.
    Tab(String),
    /// The text of the add prompt.
    Source(String),
    /// Ctrl-R on a marketplace tab: remove it.
    Remove,
    /// Ctrl-U on a marketplace tab: upgrade it.
    Upgrade,
}

/// The events of the browser: the config the host read, and the answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Config(Config),
    Answer {
        request: u64,
        result: Result<Found, String>,
    },
}

/// What `/plugins` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// The browser is up (loading, or showing the cache).
    Shown,
    /// The feature is off: the history says so.
    Disabled,
}

/// What a pick of a row asks of the screen beyond the browser itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    Nothing,
    /// Open this page in the browser.
    Open(String),
    /// "I've installed it": the app directory is refreshed.
    RefreshApps,
}

impl Plugins {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn menu(&self) -> &Menu {
        &self.menu
    }

    pub fn tab(&self) -> &str {
        &self.tab
    }

    /// The catalog, once the list for this directory is in.
    pub fn catalog(&self) -> Option<&Catalog> {
        match &self.cache {
            Cache::Ready(catalog) if self.cached_for == self.cwd => Some(catalog),
            _ => None,
        }
    }

    /// The tabs of the list, with the switches as the user left them.
    pub fn tabs(&self) -> Vec<Tab> {
        let Some(catalog) = self.catalog() else {
            return Vec::new();
        };
        let mut tabs = catalog.tabs(&self.config, &self.remote, self.fresh.as_deref());
        for row in tabs.iter_mut().flat_map(|tab| tab.rows.iter_mut()) {
            if let (Toggle::Switch(on), Some(wanted)) =
                (&mut row.toggle, self.desired.get(&row.plugin))
            {
                *on = *wanted;
            }
        }
        tabs
    }

    /// The step of the setup of apps, if one is on.
    pub fn auth(&self, installed: impl Fn(&str) -> bool) -> Option<AuthStep> {
        let flow = self.flow.as_ref()?;
        let app = flow.apps.get(flow.next)?;
        Some(AuthStep {
            plugin: flow.plugin.clone(),
            installed: installed(&app.id),
            app: app.clone(),
            current: flow.next + 1,
            total: flow.apps.len(),
        })
    }

    /// The lines for the history that came up since last asked.
    pub(super) fn take_said(&mut self) -> Vec<Say> {
        std::mem::take(&mut self.said)
    }

    /// Whether a plugin was written since last asked (the mentions are read again).
    pub fn take_written(&mut self) -> bool {
        std::mem::take(&mut self.written)
    }

    /// Whether the setup of apps asks the screen to show the browser.
    pub(super) fn take_shown(&mut self) -> bool {
        std::mem::take(&mut self.shown)
    }

    /// The requests the app has still to send, once each.
    pub fn take_wanted(&mut self) -> Vec<Wanted> {
        std::mem::take(&mut self.unsent)
    }

    pub(super) fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// The scope changed: nothing in flight is wanted any more.
    pub(super) fn forget(&mut self) {
        self.waiting.clear();
        self.unsent.clear();
        self.collect = None;
        self.in_flight = false;
        self.remote.loading = false;
        self.remote.vertical = false;
        self.flow = None;
        self.cache = Cache::Uninitialized;
        self.cached_for = None;
    }

    /// The menus are gone: the browser's own pages start again at the list next time.
    pub(super) fn dismissed(&mut self) {
        self.flow = None;
        if !matches!(self.menu, Menu::List) {
            self.menu = Menu::List;
        }
    }

    fn ask(&mut self, want: Want, pending: Pending) {
        self.issued += 1;
        let request = self.issued;
        self.waiting.insert(request, pending);
        self.unsent.push(Wanted {
            request,
            want,
            cwd: self.cwd.clone(),
            home: self.config.home.clone(),
        });
    }

    /// `/plugins` in `cwd` (`add_plugins_output`).
    pub(super) fn open(&mut self, cwd: String) -> Opened {
        if !self.config.enabled {
            self.said.push(Say::Disabled);
            return Opened::Disabled;
        }
        if self.cwd.as_deref() != Some(cwd.as_str()) {
            self.forget();
        }
        self.cwd = Some(cwd);
        self.tab = ALL.to_string();
        self.fresh = None;
        self.prefetch();
        self.menu = if self.catalog().is_some() {
            Menu::List
        } else {
            Menu::Loading
        };
        Opened::Shown
    }

    /// `prefetch_plugins`: the list asked for again, unless it is on its way.
    fn prefetch(&mut self) {
        if self.in_flight {
            return;
        }
        self.in_flight = true;
        self.remote.vertical = !self.config.remote;
        if self.cached_for != self.cwd {
            self.cache = Cache::Loading;
        }
        self.ask(Want::List, Pending::List);
    }

    /// The list again, for the browser as it is (a write changed what it holds).
    fn refresh(&mut self) {
        self.prefetch();
    }

    /// The list, or the loading page while there is none.
    fn list_menu(&self) -> Menu {
        if self.catalog().is_some() {
            Menu::List
        } else {
            Menu::Loading
        }
    }

    pub(super) fn intent(&mut self, intent: Intent) {
        match intent {
            Intent::Tab(id) => {
                if self.tab != id {
                    self.fresh = None;
                }
                self.tab = id;
            }
            Intent::Source(text) => self.add(text.trim()),
            Intent::Remove => self.confirm_remove(),
            Intent::Upgrade => self.upgrade(),
        }
    }

    /// The marketplace of the tab the user is at, when it is a configured one.
    fn marketplace(&self) -> Option<(String, String, bool)> {
        if !matches!(self.menu, Menu::List) {
            return None;
        }
        let catalog = self.catalog()?;
        let tab = self.tabs().into_iter().find(|tab| tab.id == self.tab)?;
        let Kind::Marketplace { name, remove, upgrade, .. } = tab.kind else {
            return None;
        };
        if !remove {
            return None;
        }
        let label = catalog
            .marketplaces
            .iter()
            .find(|entry| entry.name == name)?
            .display()
            .to_string();
        Some((name, label, upgrade))
    }

    fn confirm_remove(&mut self) {
        if let Some((name, label, _)) = self.marketplace() {
            self.menu = Menu::RemoveConfirm { name, label };
        }
    }

    fn upgrade(&mut self) {
        if let Some((name, _, true)) = self.marketplace() {
            self.menu = Menu::UpgradeLoading {
                name: Some(name.clone()),
            };
            self.ask(Want::Upgrade { name: Some(name) }, Pending::Upgrade);
        }
    }

    fn add(&mut self, source: &str) {
        if source.is_empty() || !matches!(self.menu, Menu::AddPrompt) {
            return;
        }
        self.menu = Menu::AddLoading;
        let source = add_source(self.cwd.as_deref().unwrap_or_default(), source);
        self.ask(Want::Add { source }, Pending::Add);
    }

    /// A pick of the row `id` of the menu shown.
    pub(super) fn choose(&mut self, id: &str) -> Chosen {
        match self.menu.clone() {
            Menu::List => return self.choose_in_list(id),
            Menu::Detail(detail) => match id {
                BACK => self.menu = self.list_menu(),
                INSTALL => {
                    if let Action::Install(target) = detail.action() {
                        let name = detail.plugin.display_name.clone();
                        self.menu = Menu::InstallLoading { name: name.clone() };
                        self.ask(
                            Want::Install(target.clone()),
                            Pending::Install { name, target },
                        );
                    }
                }
                UNINSTALL => {
                    if let Action::Uninstall { id } = detail.action() {
                        let name = detail.plugin.display_name.clone();
                        self.menu = Menu::UninstallLoading { name: name.clone() };
                        self.ask(Want::Uninstall { id }, Pending::Uninstall { name });
                    }
                }
                _ => {}
            },
            Menu::DetailFailed { .. } if id == BACK => self.menu = self.list_menu(),
            Menu::AddFailed => match id {
                RETRY => self.menu = Menu::AddPrompt,
                BACK => self.menu = self.list_menu(),
                _ => {}
            },
            Menu::RemoveConfirm { name, label } => match id {
                REMOVE => {
                    self.menu = Menu::RemoveLoading {
                        label: label.clone(),
                    };
                    self.ask(Want::Remove { name: name.clone() }, Pending::Remove { name, label });
                }
                BACK => self.menu = self.list_menu(),
                _ => {}
            },
            Menu::RemoveFailed { name, label } => match id {
                RETRY => self.menu = Menu::RemoveConfirm { name, label },
                BACK => self.menu = self.list_menu(),
                _ => {}
            },
            Menu::Auth => return self.choose_auth(id),
            _ => {}
        }
        Chosen::Nothing
    }

    fn choose_in_list(&mut self, id: &str) -> Chosen {
        if id == ADD_ROW {
            self.tab = ADD.to_string();
            self.menu = Menu::AddPrompt;
            return Chosen::Nothing;
        }
        let Some(catalog) = self.catalog() else {
            return Chosen::Nothing;
        };
        let (Some(target), Some((_, plugin))) = (catalog.detail_target(id), catalog.row(id)) else {
            return Chosen::Nothing;
        };
        let name = plugin.display_name.clone();
        self.menu = Menu::DetailLoading { name };
        self.ask(Want::Read(target), Pending::Read);
        Chosen::Nothing
    }

    fn choose_auth(&mut self, id: &str) -> Chosen {
        let Some(flow) = &self.flow else {
            return Chosen::Nothing;
        };
        match id {
            OPEN => flow
                .apps
                .get(flow.next)
                .and_then(|app| app.url.clone())
                .map_or(Chosen::Nothing, Chosen::Open),
            NEXT => {
                self.advance();
                Chosen::RefreshApps
            }
            SKIP => {
                self.finish_flow(true);
                Chosen::Nothing
            }
            _ => Chosen::Nothing,
        }
    }

    /// `advance_plugin_install_auth_flow`.
    fn advance(&mut self) {
        let Some(flow) = self.flow.as_mut() else { return };
        flow.next += 1;
        if flow.next >= flow.apps.len() {
            self.finish_flow(false);
        }
    }

    /// `finish_plugin_install_auth_flow`.
    fn finish_flow(&mut self, abandoned: bool) {
        let Some(flow) = self.flow.take() else { return };
        let name = flow.plugin;
        self.said.push(if abandoned {
            Say::Skipped { name }
        } else {
            Say::Completed { name }
        });
        self.menu = self.list_menu();
    }

    /// The switch of the plugin row `id` was flipped (`SetPluginEnabled`, coalesced: while a write
    /// is in flight the last value asked for is written after it).
    pub(super) fn toggle(&mut self, id: &str) {
        if !matches!(self.menu, Menu::List) {
            return;
        }
        let Some(catalog) = self.catalog() else { return };
        let Some((_, plugin)) = catalog.row(id) else { return };
        if !matches!(plugin.toggle(), Toggle::Switch(_)) {
            return;
        }
        let key = plugin.id.clone();
        let enabled = !self.desired.get(&key).copied().unwrap_or(plugin.enabled);
        self.desired.insert(key.clone(), enabled);
        match self.writing.get_mut(&key) {
            Some(queued) => *queued = Some(enabled),
            None => {
                self.writing.insert(key.clone(), None);
                self.ask(
                    Want::Enable { id: key.clone(), enabled },
                    Pending::Enable { id: key, enabled },
                );
            }
        }
    }

    /// Esc: back to the list from a page over it; false when the browser closes.
    pub(super) fn back(&mut self) -> bool {
        match self.menu {
            Menu::Detail(_)
            | Menu::AddPrompt
            | Menu::AddFailed
            | Menu::RemoveConfirm { .. }
            | Menu::RemoveFailed { .. }
            | Menu::DetailFailed { back: true, .. } => {
                self.menu = self.list_menu();
                true
            }
            _ => false,
        }
    }

    /// The answer of the request `request`; `shown` says the browser is on top.
    pub(super) fn answered(&mut self, request: u64, result: Result<Found, String>, shown: bool) {
        let Some(pending) = self.waiting.remove(&request) else {
            return;
        };
        match pending {
            Pending::List => self.listed(result, shown),
            Pending::Section(part) => self.section(request, part, result, shown),
            Pending::Read => match result {
                Ok(Found::Detail(detail)) => {
                    if shown && self.catalog().is_some() {
                        self.menu = Menu::Detail(detail);
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    if shown {
                        let back = self.catalog().is_some();
                        self.menu = Menu::DetailFailed { error, back };
                    }
                }
            },
            Pending::Install { name, target } => self.installed(name, target, result, shown),
            Pending::Uninstall { name } => match result {
                Ok(_) => {
                    self.flow = None;
                    self.said.push(Say::Uninstalled { name });
                    self.written = true;
                    self.refresh();
                }
                Err(error) => {
                    if shown {
                        let back = self.catalog().is_some();
                        self.menu = Menu::DetailFailed { error, back };
                    }
                }
            },
            Pending::Add => match result {
                Ok(Found::Added { name, root, already }) => {
                    let tab = catalog::tab_id_of_root(&root);
                    self.tab = tab.clone();
                    self.fresh = (!already).then_some(tab);
                    self.said.push(Say::MarketAdded { name, root, already });
                    self.written = true;
                    self.refresh();
                }
                Ok(_) => {}
                Err(_) => {
                    self.tab = ADD.to_string();
                    self.menu = Menu::AddFailed;
                }
            },
            Pending::Remove { name, label } => match result {
                Ok(Found::Removed { name: removed, root }) => {
                    self.tab = ALL.to_string();
                    self.said.push(Say::MarketRemoved {
                        label,
                        name: removed,
                        root,
                    });
                    self.written = true;
                    self.refresh();
                }
                Ok(_) => {}
                Err(_) => self.menu = Menu::RemoveFailed { name, label },
            },
            Pending::Upgrade => {
                match result {
                    Ok(Found::Upgraded { selected, roots, errors }) => {
                        self.upgraded(selected, roots, errors);
                    }
                    Ok(_) => {}
                    Err(error) => self.said.push(Say::UpgradeError(error)),
                }
                self.refresh();
            }
            Pending::Enable { id, enabled } => self.enabled(id, enabled, result),
        }
    }

    /// `on_plugins_loaded`.
    fn listed(&mut self, result: Result<Found, String>, shown: bool) {
        self.in_flight = false;
        let quiet = self.flow.is_some();
        match result {
            Ok(Found::Catalog(catalog)) => {
                self.cached_for = self.cwd.clone();
                self.remote.loading = true;
                self.remote.loaded = false;
                self.remote.errors.clear();
                if let Some(found) = catalog.tab_for_saved(&self.tab) {
                    self.tab = found;
                }
                self.fresh = self
                    .fresh
                    .take()
                    .and_then(|tab| catalog.tab_for_saved(&tab));
                self.cache = Cache::Ready(catalog);
                if shown && !quiet {
                    self.menu = Menu::List;
                }
                self.fetch_sections();
            }
            Ok(_) => {}
            Err(error) => {
                self.remote.loading = false;
                self.remote.loaded = false;
                self.remote.vertical = false;
                if !quiet && (shown || self.catalog().is_none()) {
                    self.cached_for = None;
                    self.cache = Cache::Failed;
                    if shown {
                        self.menu = Menu::Failed(error);
                    }
                }
            }
        }
    }

    /// `fetch_additional_plugin_remote_sections`: the marketplaces of the remote sections.
    fn fetch_sections(&mut self) {
        let mut collect = Collect::default();
        let mut parts = Vec::new();
        if !self.config.remote {
            parts.push(Part::Vertical);
        }
        parts.push(Part::Workspace);
        if self.config.sharing {
            parts.push(Part::SharedWithMe);
        } else {
            collect.errors.push(SectionError {
                part: Part::SharedWithMe,
                cause: catalog::Cause::SharingDisabled,
            });
        }
        let before = self.unsent.len();
        for part in parts {
            self.ask(Want::Section(part), Pending::Section(part));
        }
        collect.waiting = self.unsent[before..].iter().map(|want| want.request).collect();
        self.collect = Some(collect);
    }

    fn section(&mut self, request: u64, part: Part, result: Result<Found, String>, shown: bool) {
        let Some(collect) = self.collect.as_mut() else { return };
        if !collect.waiting.remove(&request) {
            return;
        }
        match result {
            Ok(Found::Catalog(found)) => collect.found.push((part, found.marketplaces)),
            Ok(_) => {}
            Err(error) => collect.errors.push(SectionError::failed(part, error)),
        }
        if collect.waiting.is_empty() {
            self.sections_loaded(shown);
        }
    }

    /// `on_plugin_remote_sections_loaded`.
    fn sections_loaded(&mut self, _shown: bool) {
        let Some(mut collect) = self.collect.take() else { return };
        self.remote.loading = false;
        self.remote.loaded = true;
        self.remote.vertical = false;
        collect.found.sort_by_key(|(part, _)| *part);
        let ready = self.catalog().is_some();
        if let (true, Cache::Ready(catalog)) = (ready, &mut self.cache) {
            catalog.merge(collect.found.into_iter().flat_map(|(_, found)| found).collect());
        }
        self.remote.errors = collect.errors;
    }

    /// `on_plugin_install_loaded`: the apps still to sign in start the setup; else the detail is
    /// read again, and the list in both cases.
    fn installed(&mut self, name: String, target: Target, result: Result<Found, String>, shown: bool) {
        match result {
            Ok(Found::Installed(apps)) => {
                self.written = true;
                self.flow = None;
                let names = apps.iter().map(|app| app.name.clone()).collect();
                self.said.push(Say::Installed {
                    name: name.clone(),
                    apps: names,
                });
                self.refresh();
                if apps.is_empty() {
                    self.ask(Want::Read(target), Pending::Read);
                } else {
                    self.flow = Some(Flow {
                        plugin: name,
                        apps,
                        next: 0,
                    });
                    self.menu = Menu::Auth;
                    self.shown = true;
                }
            }
            Ok(_) => {}
            Err(error) => {
                self.flow = None;
                if shown {
                    let back = self.catalog().is_some();
                    self.menu = Menu::DetailFailed { error, back };
                }
            }
        }
    }

    /// `on_marketplace_upgrade_loaded`.
    fn upgraded(&mut self, selected: Vec<String>, roots: Vec<String>, errors: Vec<(String, String)>) {
        if let [root] = roots.as_slice() {
            self.tab = catalog::tab_id_of_root(root);
        }
        if selected.is_empty() {
            self.said.push(Say::NothingToUpgrade);
            return;
        }
        if roots.is_empty() && errors.is_empty() {
            self.said.push(Say::UpToDate { selected });
            return;
        }
        if !roots.is_empty() {
            self.written = true;
            self.said.push(Say::Upgraded { roots });
        }
        if !errors.is_empty() {
            self.said.push(Say::UpgradeFailed(errors));
        }
    }

    /// `on_plugin_enabled_set`, and the write queued behind it.
    fn enabled(&mut self, id: String, enabled: bool, result: Result<Found, String>) {
        let queued = self.writing.get_mut(&id).and_then(Option::take);
        if let Some(next) = queued
            && (result.is_err() || next != enabled)
        {
            self.ask(
                Want::Enable { id: id.clone(), enabled: next },
                Pending::Enable { id, enabled: next },
            );
            return;
        }
        self.writing.remove(&id);
        self.desired.remove(&id);
        match result {
            Ok(_) => {
                self.written = true;
                if let Cache::Ready(catalog) = &mut self.cache {
                    catalog.set_enabled(&id, enabled);
                }
            }
            Err(error) => self.said.push(Say::EnableFailed { id, error }),
        }
    }
}

/// `marketplace_add_source_for_request`: a relative local path is made absolute against `cwd`,
/// its `#ref` or `@ref` kept.
pub fn add_source(cwd: &str, source: &str) -> String {
    let (base, suffix) = if let Some((base, reference)) = source.rsplit_once('#') {
        (base, Some(["#", reference].concat()))
    } else if let Some((base, reference)) = source.rsplit_once('@') {
        (base, Some(["@", reference].concat()))
    } else {
        (source, None)
    };
    let relative = matches!(base, "." | "..")
        || ["./", "../", ".\\", "..\\"]
            .iter()
            .any(|prefix| base.starts_with(prefix));
    if !relative {
        return source.to_string();
    }
    let mut parts: Vec<&str> = cwd.split('/').filter(|part| !part.is_empty()).collect();
    for part in base.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    let mut resolved = ["/", &parts.join("/")].concat();
    resolved.extend(suffix);
    resolved
}

/// The wire request of `wanted`.
pub fn wire_request(wanted: &Wanted, id: RequestId) -> ClientRequest {
    let path = |path: &str| serde_json::from_value(serde_json::Value::String(path.to_string())).ok();
    let list = |kinds: Option<Vec<wire::PluginListMarketplaceKind>>| wire::PluginListParams {
        cwds: wanted
            .cwd
            .as_deref()
            .and_then(|cwd| serde_json::from_value(serde_json::json!([cwd])).ok()),
        marketplace_kinds: kinds,
        force_refetch: false,
    };
    let target = |target: &Target| match &target.location {
        Location::Local { path: file } => (path(file), None),
        Location::Remote { marketplace } => (None, Some(marketplace.clone())),
    };
    match &wanted.want {
        Want::List => ClientRequest::PluginList {
            request_id: id,
            params: list(None),
        },
        Want::Section(part) => ClientRequest::PluginList {
            request_id: id,
            params: list(Some(vec![match part {
                Part::Vertical => wire::PluginListMarketplaceKind::Vertical,
                Part::Workspace => wire::PluginListMarketplaceKind::WorkspaceDirectory,
                Part::SharedWithMe => wire::PluginListMarketplaceKind::SharedWithMe,
            }])),
        },
        Want::Read(plugin) => {
            let (marketplace_path, remote_marketplace_name) = target(plugin);
            ClientRequest::PluginRead {
                request_id: id,
                params: wire::PluginReadParams {
                    marketplace_path,
                    remote_marketplace_name,
                    plugin_name: plugin.plugin.clone(),
                },
            }
        }
        Want::Install(plugin) => {
            let (marketplace_path, remote_marketplace_name) = target(plugin);
            ClientRequest::PluginInstall {
                request_id: id,
                params: wire::PluginInstallParams {
                    marketplace_path,
                    remote_marketplace_name,
                    install_attempt_id: Some(["plugin-install-", &wanted.request.to_string()].concat()),
                    plugin_name: plugin.plugin.clone(),
                },
            }
        }
        Want::Uninstall { id: plugin } => ClientRequest::PluginUninstall {
            request_id: id,
            params: wire::PluginUninstallParams {
                plugin_id: plugin.clone(),
            },
        },
        Want::Add { source } => ClientRequest::MarketplaceAdd {
            request_id: id,
            params: wire::MarketplaceAddParams {
                source: source.clone(),
                ref_name: None,
                sparse_paths: None,
            },
        },
        Want::Remove { name } => ClientRequest::MarketplaceRemove {
            request_id: id,
            params: wire::MarketplaceRemoveParams {
                marketplace_name: name.clone(),
            },
        },
        Want::Upgrade { name } => ClientRequest::MarketplaceUpgrade {
            request_id: id,
            params: wire::MarketplaceUpgradeParams {
                marketplace_name: name.clone(),
            },
        },
        Want::Enable { id: plugin, enabled } => ClientRequest::ConfigValueWrite {
            request_id: id,
            params: wire::ConfigValueWriteParams {
                key_path: ["plugins.", plugin].concat(),
                value: serde_json::json!({ "enabled": enabled }),
                merge_strategy: wire::MergeStrategy::Upsert,
                file_path: None,
                expected_version: None,
            },
        },
    }
}

/// The answer `result` (the JSON result of the request) of `wanted`.
pub fn decode(wanted: &Wanted, result: serde_json::Value) -> Result<Found, String> {
    fn parse<T: serde::de::DeserializeOwned>(result: serde_json::Value) -> Result<T, String> {
        serde_json::from_value(result).map_err(|error| error.to_string())
    }
    let home = wanted.home.as_deref();
    fn root(root: impl AsRef<std::path::Path>) -> String {
        root.as_ref().to_string_lossy().into_owned()
    }
    Ok(match &wanted.want {
        Want::List | Want::Section(_) => {
            Found::Catalog(Catalog::of(parse::<wire::PluginListResponse>(result)?, home))
        }
        Want::Read(_) => Found::Detail(Box::new(Detail::of(
            parse::<wire::PluginReadResponse>(result)?.plugin,
            home,
        ))),
        Want::Install(_) => Found::Installed(
            parse::<wire::PluginInstallResponse>(result)?
                .apps_needing_auth
                .into_iter()
                .map(|app| AuthApp {
                    id: app.id,
                    name: app.name,
                    url: app.install_url,
                })
                .collect(),
        ),
        Want::Uninstall { .. } => {
            parse::<wire::PluginUninstallResponse>(result)?;
            Found::Uninstalled
        }
        Want::Add { .. } => {
            let added = parse::<wire::MarketplaceAddResponse>(result)?;
            Found::Added {
                name: added.marketplace_name,
                root: root(added.installed_root),
                already: added.already_added,
            }
        }
        Want::Remove { .. } => {
            let removed = parse::<wire::MarketplaceRemoveResponse>(result)?;
            Found::Removed {
                name: removed.marketplace_name,
                root: removed.installed_root.map(root),
            }
        }
        Want::Upgrade { .. } => {
            let upgraded = parse::<wire::MarketplaceUpgradeResponse>(result)?;
            Found::Upgraded {
                selected: upgraded.selected_marketplaces,
                roots: upgraded.upgraded_roots.into_iter().map(root).collect(),
                errors: upgraded
                    .errors
                    .into_iter()
                    .map(|error| (error.marketplace_name, error.message))
                    .collect(),
            }
        }
        Want::Enable { .. } => Found::Written,
    })
}
