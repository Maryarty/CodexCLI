//! The state of the plugins feature, by the kinds ADR-013 §1 names: the answers the server gave
//! are request records cached by key (`Remote<T>` per directory, per plugin), what the user
//! asked to change is an op under its `OpKey` (a second press while it runs collapses), the
//! lists a surface wants are loaded because it said so (`on_interest`), and the setup of an
//! installed plugin's apps is a wish for the surface. What is drawn (the tab, the cursor, the
//! page on top, the step of the setup) is not here: it is the sheet's.

use std::collections::{BTreeMap, BTreeSet};

use super::catalog::{
    self, Action, Catalog, Config, Detail, Kind, Part, RowKey, SectionError, Sections, Tab, Target,
    Toggle,
};
use super::rpcs::{AuthApp, Found, Want, Wanted, add_source};
use super::{Ctx, Intent, Say, SetupEnd, Wish};
use crate::app::effects::Ask;
use crate::features::{AppCx, Remote};
use crate::ids::Cwd;
use crate::rpc::RpcError;

/// A number that tells one ask (and the op it made) from the next: the request number of the
/// ask, so a late reply is told from a current one by it.
pub type Gen = u64;

/// A plugin as the list of a directory names it: the directory and the row of its catalog.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginKey {
    pub cwd: Cwd,
    pub row: RowKey,
}

/// What the user asked to change; one at a time per key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpKey {
    Install(PluginKey),
    Uninstall(PluginKey),
    Add { cwd: Cwd },
    Remove { cwd: Cwd, name: String },
    Upgrade { cwd: Cwd, name: Option<String> },
}

/// What came of an op, for the surface that waited for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Added {
        root: String,
        already: bool,
    },
    Removed,
    /// The marketplace's root, when exactly one was upgraded.
    Upgraded {
        root: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    /// Done, and what the list it changed holds is in (the list is read again behind an op that
    /// changes it, so a surface that waited for the op sees the new list).
    Done(Outcome),
    /// Why it failed, as the typed error (the words are the UI's).
    Failed(RpcError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    pub at: Gen,
    pub status: Status,
}

/// What the user can still do with the apps an installed plugin needs signed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setup {
    pub plugin: String,
    pub apps: Vec<AuthApp>,
}

/// A tab of the list that is a configured marketplace, for removing or upgrading it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Market {
    pub name: String,
    pub label: String,
    pub upgrade: bool,
}

/// The sections being loaded after the list (`fetch_additional_plugin_remote_sections`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Collect {
    waiting: BTreeSet<u64>,
    found: Vec<(Part, Vec<catalog::Marketplace>)>,
    errors: Vec<SectionError>,
}

/// The list of one directory.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Listing {
    request: u64,
    catalog: Remote<Catalog>,
    sections: Sections,
    collect: Option<Collect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Detailed {
    request: u64,
    remote: Remote<Detail>,
}

/// What an op waits for beyond the server's answer to it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Cue {
    List(Cwd),
    Read(PluginKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Follow {
    on: Cue,
    op: OpKey,
    at: Gen,
    outcome: Outcome,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    config: Config,
    issued: u64,
    lists: BTreeMap<Cwd, Listing>,
    details: BTreeMap<PluginKey, Detailed>,
    ops: BTreeMap<OpKey, Op>,
    follows: Vec<Follow>,
    setups: BTreeMap<PluginKey, Setup>,
    /// The switches the user flipped and the config has not taken yet.
    desired: BTreeMap<String, bool>,
    /// Writes in flight, each with the value asked for after it.
    writing: BTreeMap<String, Option<bool>>,
}

/// What the server said of a failed ask, for the classification of a section's error
/// (`SectionError::failed`): the words shown are the UI's.
fn said(error: &RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => {
            text.clone()
        }
        RpcError::Mismatch => String::new(),
        RpcError::Provider(error) => error.to_string(),
    }
}

// ---- what the surfaces read ---------------------------------------------------------------------

impl State {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// The list of the directory: none until someone asked.
    pub fn catalog(&self, cwd: &Cwd) -> Option<&Remote<Catalog>> {
        self.lists.get(cwd).map(|listing| &listing.catalog)
    }

    /// The tabs of the list of `cwd`, with the switches as the user left them; `fresh` is the tab
    /// of the marketplace just added.
    pub fn tabs(&self, cwd: &Cwd, fresh: Option<&str>) -> Vec<Tab> {
        let Some(listing) = self.lists.get(cwd) else {
            return Vec::new();
        };
        let Some(catalog) = listing.catalog.value() else {
            return Vec::new();
        };
        let mut tabs = catalog.tabs(&self.config, &listing.sections, fresh);
        for row in tabs.iter_mut().flat_map(|tab| tab.rows.iter_mut()) {
            if let (Toggle::Switch(on), Some(wanted)) =
                (&mut row.toggle, self.desired.get(&row.key.plugin))
            {
                *on = *wanted;
            }
        }
        tabs
    }

    /// The tab of this catalog a tab id saved before a refresh names now.
    pub fn tab_now(&self, cwd: &Cwd, saved: &str) -> Option<String> {
        self.lists.get(cwd)?.catalog.value()?.tab_for_saved(saved)
    }

    /// The marketplace of the tab `tab`, when it is a configured one that can be removed.
    pub fn market(&self, cwd: &Cwd, tab: &str, fresh: Option<&str>) -> Option<Market> {
        let catalog = self.lists.get(cwd)?.catalog.value()?;
        let tab = self
            .tabs(cwd, fresh)
            .into_iter()
            .find(|entry| entry.id == tab)?;
        let Kind::Marketplace {
            name,
            remove,
            upgrade,
            ..
        } = tab.kind
        else {
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
        Some(Market {
            name,
            label,
            upgrade,
        })
    }

    /// The name of a marketplace as its tab says it.
    pub fn market_label(&self, cwd: &Cwd, name: &str) -> Option<String> {
        let catalog = self.lists.get(cwd)?.catalog.value()?;
        let entry = catalog
            .marketplaces
            .iter()
            .find(|entry| entry.name == name)?;
        Some(entry.display().to_string())
    }

    /// Whether the row of the list can be opened: a plugin of the catalog.
    pub fn readable(&self, key: &PluginKey) -> bool {
        self.target(key).is_some()
    }

    /// The name a plugin's row says.
    pub fn plugin_name(&self, key: &PluginKey) -> Option<String> {
        let catalog = self.lists.get(&key.cwd)?.catalog.value()?;
        catalog
            .row(&key.row)
            .map(|(_, plugin)| plugin.display_name.clone())
    }

    pub fn detail(&self, key: &PluginKey) -> Option<&Remote<Detail>> {
        self.details.get(key).map(|entry| &entry.remote)
    }

    pub fn op(&self, key: &OpKey) -> Option<&Op> {
        self.ops.get(key)
    }

    /// The `at` of the op under `key`: what a surface remembers when it presses, to tell a later
    /// op from the one before.
    pub fn op_at(&self, key: &OpKey) -> Gen {
        self.ops.get(key).map_or(0, |op| op.at)
    }

    pub fn setup(&self, key: &PluginKey) -> Option<&Setup> {
        self.setups.get(key)
    }

    /// The plugins of `cwd` whose installed apps still need signing in: what a sheet open for the
    /// directory shows the setup page for (the wish to show it only opens a sheet that is not).
    pub fn setups_in<'a>(&'a self, cwd: &'a Cwd) -> impl Iterator<Item = &'a PluginKey> {
        self.setups.keys().filter(move |key| key.cwd == *cwd)
    }

    fn target(&self, key: &PluginKey) -> Option<Target> {
        let catalog = self.lists.get(&key.cwd)?.catalog.value()?;
        catalog.row(&key.row)?;
        catalog.detail_target(&key.row)
    }
}

// ---- what happens ------------------------------------------------------------------------------

impl State {
    pub(super) fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    fn next(&mut self) -> u64 {
        self.issued += 1;
        self.issued
    }

    fn ask(&self, cx: &mut AppCx<'_>, request: u64, want: Want, cwd: &Cwd, ctx: Ctx) {
        cx.ask(
            None,
            Wanted {
                request,
                want,
                cwd: cwd.to_string(),
                home: self.config.home.clone(),
            },
            ctx,
        );
    }

    /// A new op under `key`, unless one runs: the second press of a button collapses.
    fn start(&mut self, key: OpKey) -> Option<Gen> {
        if matches!(
            self.ops.get(&key),
            Some(Op {
                status: Status::Running,
                ..
            })
        ) {
            return None;
        }
        let at = self.next();
        self.ops.insert(
            key,
            Op {
                at,
                status: Status::Running,
            },
        );
        Some(at)
    }

    /// The op `key` of the ask `at` ended: a reply for an op that was replaced is dropped.
    fn settle(&mut self, key: &OpKey, at: Gen, status: Status) {
        if let Some(op) = self.ops.get_mut(key)
            && op.at == at
        {
            op.status = status;
        }
    }

    /// `op` is done when `cue` comes in; at once when nothing is on its way.
    fn track(&mut self, cue: Cue, op: OpKey, at: Gen, outcome: Outcome) {
        let coming = match &cue {
            Cue::List(cwd) => self
                .lists
                .get(cwd)
                .is_some_and(|listing| listing.catalog.is_loading()),
            Cue::Read(key) => self
                .details
                .get(key)
                .is_some_and(|entry| entry.remote.is_loading()),
        };
        if coming {
            self.follows.push(Follow {
                on: cue,
                op,
                at,
                outcome,
            });
        } else {
            self.settle(&op, at, Status::Done(outcome));
        }
    }

    /// `cue` came in: what waited for it is done.
    fn cued(&mut self, cue: &Cue) {
        let (due, rest): (Vec<Follow>, Vec<Follow>) = std::mem::take(&mut self.follows)
            .into_iter()
            .partition(|follow| follow.on == *cue);
        self.follows = rest;
        for follow in due {
            self.settle(&follow.op, follow.at, Status::Done(follow.outcome));
        }
    }

    pub(super) fn intent(&mut self, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Run { cwd: _ } => {
                if !self.config.enabled {
                    cx.note(None, Say::Disabled);
                }
            }
            Intent::Read { key } => self.read(key, cx),
            Intent::Install { key } => self.install(key, cx),
            Intent::Uninstall { key } => self.uninstall(key, cx),
            Intent::SetEnabled { key, enabled } => self.set_enabled(&key, enabled, cx),
            Intent::AddMarketplace { cwd, source } => self.add(cwd, source.trim(), cx),
            Intent::RemoveMarketplace { cwd, name } => self.remove(cwd, name, cx),
            Intent::UpgradeMarketplace { cwd, name } => self.upgrade(cwd, name, cx),
            Intent::OpenSetupPage { key, app } => {
                let url = self
                    .setups
                    .get(&key)
                    .and_then(|setup| setup.apps.iter().find(|entry| entry.id == app))
                    .and_then(|entry| entry.url.clone());
                if let Some(url) = url {
                    cx.effect(Ask::open_url(url.clone()), Ctx::Page { url });
                }
            }
            Intent::EndSetup { key, how } => {
                let Some(setup) = self.setups.remove(&key) else {
                    return;
                };
                let name = setup.plugin;
                match how {
                    SetupEnd::Completed => cx.note(None, Say::Completed { name }),
                    SetupEnd::Skipped => cx.note(None, Say::Skipped { name }),
                    SetupEnd::Dismissed => {}
                }
            }
        }
    }

    fn read(&mut self, key: PluginKey, cx: &mut AppCx<'_>) {
        let Some(target) = self.target(&key) else {
            return;
        };
        let request = self.next();
        let cwd = key.cwd.clone();
        self.details.insert(
            key.clone(),
            Detailed {
                request,
                remote: Remote::loading(),
            },
        );
        self.ask(
            cx,
            request,
            Want::Read(target),
            &cwd,
            Ctx::Read { key, request },
        );
    }

    fn actionable(&self, key: &PluginKey) -> Option<(Action, String)> {
        let Some(Remote::Ready(detail)) = self.detail(key) else {
            return None;
        };
        Some((detail.action(), detail.plugin.display_name.clone()))
    }

    fn install(&mut self, key: PluginKey, cx: &mut AppCx<'_>) {
        let Some((Action::Install(target), name)) = self.actionable(&key) else {
            return;
        };
        let Some(at) = self.start(OpKey::Install(key.clone())) else {
            return;
        };
        let cwd = key.cwd.clone();
        let ctx = Ctx::Install {
            key,
            at,
            name,
            target: target.clone(),
        };
        self.ask(cx, at, Want::Install(target), &cwd, ctx);
    }

    fn uninstall(&mut self, key: PluginKey, cx: &mut AppCx<'_>) {
        let Some((Action::Uninstall { id }, name)) = self.actionable(&key) else {
            return;
        };
        let Some(at) = self.start(OpKey::Uninstall(key.clone())) else {
            return;
        };
        let cwd = key.cwd.clone();
        self.ask(
            cx,
            at,
            Want::Uninstall { id },
            &cwd,
            Ctx::Uninstall { key, at, name },
        );
    }

    fn add(&mut self, cwd: Cwd, source: &str, cx: &mut AppCx<'_>) {
        if source.is_empty() {
            return;
        }
        let Some(at) = self.start(OpKey::Add { cwd: cwd.clone() }) else {
            return;
        };
        let source = add_source(cwd.as_str(), source);
        self.ask(
            cx,
            at,
            Want::Add { source },
            &cwd.clone(),
            Ctx::Add { cwd, at },
        );
    }

    fn remove(&mut self, cwd: Cwd, name: String, cx: &mut AppCx<'_>) {
        let label = self
            .market_label(&cwd, &name)
            .unwrap_or_else(|| name.clone());
        let key = OpKey::Remove {
            cwd: cwd.clone(),
            name: name.clone(),
        };
        let Some(at) = self.start(key) else {
            return;
        };
        let ctx = Ctx::Remove {
            cwd: cwd.clone(),
            at,
            name: name.clone(),
            label,
        };
        self.ask(cx, at, Want::Remove { name }, &cwd, ctx);
    }

    fn upgrade(&mut self, cwd: Cwd, name: Option<String>, cx: &mut AppCx<'_>) {
        let key = OpKey::Upgrade {
            cwd: cwd.clone(),
            name: name.clone(),
        };
        let Some(at) = self.start(key) else {
            return;
        };
        let ctx = Ctx::Upgrade {
            cwd: cwd.clone(),
            at,
            name: name.clone(),
        };
        self.ask(cx, at, Want::Upgrade { name }, &cwd, ctx);
    }

    /// The switch of the plugin row `key` set to `enabled` (`SetPluginEnabled`, coalesced: while a
    /// write is in flight the last value asked for is written after it).
    fn set_enabled(&mut self, key: &PluginKey, enabled: bool, cx: &mut AppCx<'_>) {
        let Some(catalog) = self.lists.get(&key.cwd).and_then(|l| l.catalog.value()) else {
            return;
        };
        let Some((_, plugin)) = catalog.row(&key.row) else {
            return;
        };
        if !matches!(plugin.toggle(), Toggle::Switch(_)) {
            return;
        }
        let id = plugin.id.clone();
        self.desired.insert(id.clone(), enabled);
        match self.writing.get_mut(&id) {
            Some(queued) => *queued = Some(enabled),
            None => {
                self.writing.insert(id.clone(), None);
                let request = self.next();
                let cwd = key.cwd.clone();
                let ctx = Ctx::Enable {
                    cwd: cwd.clone(),
                    id: id.clone(),
                    enabled,
                };
                self.ask(cx, request, Want::Enable { id, enabled }, &cwd, ctx);
            }
        }
    }

    /// A surface wants the list of `cwd` (`prefetch_plugins`): asked again, unless it is on its
    /// way; what was known stays on show meanwhile.
    pub(super) fn prefetch(&mut self, cwd: &Cwd, cx: &mut AppCx<'_>) {
        if !self.config.enabled
            || self
                .lists
                .get(cwd)
                .is_some_and(|listing| listing.catalog.is_loading())
        {
            return;
        }
        let request = self.next();
        let vertical = !self.config.remote;
        let before = self.lists.remove(cwd);
        let listing = match before {
            Some(mut listing) => {
                listing.request = request;
                listing.catalog = listing.catalog.reloading();
                listing.sections.vertical = vertical;
                listing
            }
            None => Listing {
                request,
                catalog: Remote::loading(),
                sections: Sections {
                    vertical,
                    ..Sections::default()
                },
                collect: None,
            },
        };
        self.lists.insert(cwd.clone(), listing);
        self.ask(
            cx,
            request,
            Want::List,
            cwd,
            Ctx::List {
                cwd: cwd.clone(),
                request,
            },
        );
    }

    /// The answer of an ask of the feature.
    pub(super) fn answered(
        &mut self,
        ctx: Ctx,
        result: Result<Found, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        match ctx {
            Ctx::List { cwd, request } => self.listed(&cwd, request, result, cx),
            Ctx::Section { cwd, request, part } => self.section(&cwd, request, part, result),
            Ctx::Read { key, request } => self.read_answered(key, request, result),
            Ctx::Install {
                key,
                at,
                name,
                target,
            } => self.installed(key, at, name, target, result, cx),
            Ctx::Uninstall { key, at, name } => match result {
                Ok(_) => {
                    self.setups.remove(&key);
                    cx.note(None, Say::Uninstalled { name });
                    let cwd = key.cwd.clone();
                    self.prefetch(&cwd, cx);
                    self.track(Cue::List(cwd), OpKey::Uninstall(key), at, Outcome::Nothing);
                }
                Err(error) => self.settle(&OpKey::Uninstall(key), at, Status::Failed(error)),
            },
            Ctx::Add { cwd, at } => match result {
                Ok(Found::Added {
                    name,
                    root,
                    already,
                }) => {
                    cx.note(
                        None,
                        Say::MarketAdded {
                            name,
                            root: root.clone(),
                            already,
                        },
                    );
                    self.prefetch(&cwd, cx);
                    let outcome = Outcome::Added { root, already };
                    self.track(Cue::List(cwd.clone()), OpKey::Add { cwd }, at, outcome);
                }
                Ok(_) => {}
                Err(error) => self.settle(&OpKey::Add { cwd }, at, Status::Failed(error)),
            },
            Ctx::Remove {
                cwd,
                at,
                name,
                label,
            } => {
                let key = OpKey::Remove {
                    cwd: cwd.clone(),
                    name,
                };
                match result {
                    Ok(Found::Removed {
                        name: removed,
                        root,
                    }) => {
                        cx.note(
                            None,
                            Say::MarketRemoved {
                                label,
                                name: removed,
                                root,
                            },
                        );
                        self.prefetch(&cwd, cx);
                        self.track(Cue::List(cwd), key, at, Outcome::Removed);
                    }
                    Ok(_) => {}
                    Err(error) => self.settle(&key, at, Status::Failed(error)),
                }
            }
            Ctx::Upgrade { cwd, at, name } => {
                let key = OpKey::Upgrade {
                    cwd: cwd.clone(),
                    name,
                };
                let outcome = match result {
                    Ok(Found::Upgraded {
                        selected,
                        roots,
                        errors,
                    }) => self.upgraded(selected, roots, errors, cx),
                    Ok(_) => Outcome::Nothing,
                    Err(error) => {
                        cx.note(None, Say::UpgradeError(error));
                        Outcome::Nothing
                    }
                };
                self.prefetch(&cwd, cx);
                self.track(Cue::List(cwd), key, at, outcome);
            }
            Ctx::Enable { cwd, id, enabled } => self.enable_done(&cwd, id, enabled, result, cx),
            // an ask of the host, never an RPC: `Feature::effect_done` answers it
            Ctx::Page { .. } => {}
        }
    }

    /// `on_plugins_loaded`.
    fn listed(
        &mut self,
        cwd: &Cwd,
        request: u64,
        result: Result<Found, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let Some(listing) = self.lists.get_mut(cwd) else {
            return;
        };
        if listing.request != request {
            return;
        }
        match result {
            Ok(Found::Catalog(catalog)) => {
                listing.catalog = Remote::Ready(catalog);
                listing.sections.loading = true;
                listing.sections.loaded = false;
                listing.sections.errors.clear();
                self.fetch_sections(cwd, cx);
            }
            Ok(_) => {}
            Err(error) => {
                listing.sections.loading = false;
                listing.sections.loaded = false;
                listing.sections.vertical = false;
                listing.catalog = Remote::Failed(error);
            }
        }
        self.cued(&Cue::List(cwd.clone()));
    }

    /// `fetch_additional_plugin_remote_sections`: the marketplaces of the remote sections.
    fn fetch_sections(&mut self, cwd: &Cwd, cx: &mut AppCx<'_>) {
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
        for part in parts {
            let request = self.next();
            collect.waiting.insert(request);
            let ctx = Ctx::Section {
                cwd: cwd.clone(),
                request,
                part,
            };
            self.ask(cx, request, Want::Section(part), cwd, ctx);
        }
        if let Some(listing) = self.lists.get_mut(cwd) {
            listing.collect = Some(collect);
        }
    }

    fn section(&mut self, cwd: &Cwd, request: u64, part: Part, result: Result<Found, RpcError>) {
        let Some(listing) = self.lists.get_mut(cwd) else {
            return;
        };
        let Some(collect) = listing.collect.as_mut() else {
            return;
        };
        if !collect.waiting.remove(&request) {
            return;
        }
        match result {
            Ok(Found::Catalog(found)) => collect.found.push((part, found.marketplaces)),
            Ok(_) => {}
            Err(error) => collect
                .errors
                .push(SectionError::failed(part, said(&error))),
        }
        if !collect.waiting.is_empty() {
            return;
        }
        // `on_plugin_remote_sections_loaded`
        let Some(mut collect) = listing.collect.take() else {
            return;
        };
        listing.sections.loading = false;
        listing.sections.loaded = true;
        listing.sections.vertical = false;
        collect.found.sort_by_key(|(part, _)| *part);
        if let Some(catalog) = listing.catalog.value_mut() {
            catalog.merge(
                collect
                    .found
                    .into_iter()
                    .flat_map(|(_, found)| found)
                    .collect(),
            );
        }
        listing.sections.errors = collect.errors;
    }

    fn read_answered(&mut self, key: PluginKey, request: u64, result: Result<Found, RpcError>) {
        let Some(entry) = self.details.get_mut(&key) else {
            return;
        };
        if entry.request != request {
            return;
        }
        match result {
            Ok(Found::Detail(detail)) => entry.remote = Remote::Ready(*detail),
            Ok(_) => return,
            Err(error) => entry.remote = Remote::Failed(error),
        }
        self.cued(&Cue::Read(key));
    }

    /// `on_plugin_install_loaded`: the apps still to sign in are a wish for the surface; else the
    /// detail is read again, and the list in both cases.
    fn installed(
        &mut self,
        key: PluginKey,
        at: Gen,
        name: String,
        target: Target,
        result: Result<Found, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let op = OpKey::Install(key.clone());
        match result {
            Ok(Found::Installed(apps)) => {
                let names = apps.iter().map(|app| app.name.clone()).collect();
                cx.note(
                    None,
                    Say::Installed {
                        name: name.clone(),
                        apps: names,
                    },
                );
                let cwd = key.cwd.clone();
                self.prefetch(&cwd, cx);
                if apps.is_empty() {
                    self.setups.remove(&key);
                    let request = self.next();
                    self.details.insert(
                        key.clone(),
                        Detailed {
                            request,
                            remote: Remote::loading(),
                        },
                    );
                    let ctx = Ctx::Read {
                        key: key.clone(),
                        request,
                    };
                    self.ask(cx, request, Want::Read(target), &cwd, ctx);
                    self.track(Cue::Read(key), op, at, Outcome::Nothing);
                } else {
                    self.setups
                        .insert(key.clone(), Setup { plugin: name, apps });
                    cx.wish(cwd, Wish::Auth(key));
                    self.settle(&op, at, Status::Done(Outcome::Nothing));
                }
            }
            Ok(_) => self.settle(&op, at, Status::Failed(RpcError::Mismatch)),
            Err(error) => {
                self.setups.remove(&key);
                self.settle(&op, at, Status::Failed(error));
            }
        }
    }

    /// `on_marketplace_upgrade_loaded`.
    fn upgraded(
        &mut self,
        selected: Vec<String>,
        roots: Vec<String>,
        errors: Vec<(String, String)>,
        cx: &mut AppCx<'_>,
    ) -> Outcome {
        let root = match roots.as_slice() {
            [root] => Some(root.clone()),
            _ => None,
        };
        let outcome = Outcome::Upgraded { root };
        if selected.is_empty() {
            cx.note(None, Say::NothingToUpgrade);
            return outcome;
        }
        if roots.is_empty() && errors.is_empty() {
            cx.note(None, Say::UpToDate { selected });
            return outcome;
        }
        if !roots.is_empty() {
            cx.note(None, Say::Upgraded { roots });
        }
        if !errors.is_empty() {
            cx.note(None, Say::UpgradeFailed(errors));
        }
        outcome
    }

    /// `on_plugin_enabled_set`, and the write queued behind it.
    fn enable_done(
        &mut self,
        cwd: &Cwd,
        id: String,
        enabled: bool,
        result: Result<Found, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let queued = self.writing.get_mut(&id).and_then(Option::take);
        if let Some(next) = queued
            && (result.is_err() || next != enabled)
        {
            let request = self.next();
            let ctx = Ctx::Enable {
                cwd: cwd.clone(),
                id: id.clone(),
                enabled: next,
            };
            self.ask(cx, request, Want::Enable { id, enabled: next }, cwd, ctx);
            return;
        }
        self.writing.remove(&id);
        self.desired.remove(&id);
        match result {
            Ok(_) => {
                for listing in self.lists.values_mut() {
                    if let Some(catalog) = listing.catalog.value_mut() {
                        catalog.set_enabled(&id, enabled);
                    }
                }
            }
            Err(error) => cx.note(None, Say::EnableFailed { id, error }),
        }
    }
}
