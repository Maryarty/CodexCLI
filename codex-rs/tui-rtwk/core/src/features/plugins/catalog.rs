//! The plugin catalog `/plugins` browses: the marketplaces of a `plugin/list` answer, as data the
//! view turns into tabs and rows (the words are the view's). The derivations are the oracle's
//! (`chatwidget/plugin_catalog.rs`): the product of a marketplace, the sort (installed first,
//! then by name), the dedupe of a remote plugin and its local copy, the tab ids and the ones a
//! refresh finds again.

use std::collections::HashMap;
use std::fmt;

use codex_app_server_protocol as wire;

/// The ids of the fixed tabs (`ALL_PLUGINS_TAB_ID` and the others).
pub const ALL: &str = "all-plugins";
pub const INSTALLED: &str = "installed-plugins";
pub const CURATED: &str = "marketplace:openai-curated";
pub const ADD: &str = "add-marketplace";
const MARKET: &str = "marketplace:";
const LOADING: &str = "remote-loading:";
const EMPTY: &str = "remote-empty:";
const FAILED: &str = "remote-error:";
/// The marketplace of the user's own plugins, under the home directory.
const PERSONAL: &str = ".agents/plugins/marketplace.json";
/// Marketplaces the CLI never shows (`CLI_HIDDEN_PLUGIN_MARKETPLACES`).
const HIDDEN: [&str; 1] = ["openai-bundled"];

const GLOBAL: &str = "openai-curated-remote";
const WORKSPACE: &str = "workspace-directory";
const SHARED: &str = "workspace-shared-with-me";
const SHARED_PRIVATE: &str = "workspace-shared-with-me-private";
const SHARED_LINK: &str = "workspace-shared-with-me-unlisted";

fn curated_name(name: &str) -> bool {
    name == "openai-curated" || name == "openai-api-curated"
}

/// `PluginInstallPolicy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    NotAvailable,
    Available,
    InstalledByDefault,
}

/// `PluginAvailability`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    DisabledByAdmin,
}

/// `PluginAuthPolicy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    OnInstall,
    OnUse,
}

/// `PluginShareDiscoverability`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discoverability {
    Listed,
    Unlisted,
    Private,
}

/// Where a plugin comes from (`PluginSource`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Local,
    Git {
        url: String,
        reference: Option<String>,
    },
    Npm {
        package: String,
        version: Option<String>,
    },
    Remote,
}

/// A plugin's sharing (`PluginShareContext`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Share {
    pub remote_id: String,
    pub remote_version: Option<String>,
    pub discoverability: Option<Discoverability>,
    pub url: Option<String>,
    pub creator_name: Option<String>,
    pub creator_id: Option<String>,
    /// The names of the explicit principals, none when the list is not known.
    pub principals: Option<Vec<String>>,
}

/// What a marketplace is for the user (`MarketplaceProduct`): its tab label and its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    OpenAiCurated,
    Workspace,
    SharedWithMe,
    SharedWithMeLink,
    Local,
    Other,
}

impl Product {
    fn of(name: &str, path: Option<&str>, home: Option<&str>) -> Self {
        if let (Some(path), Some(home)) = (path, home)
            && path == [home.trim_end_matches('/'), "/", PERSONAL].concat()
        {
            return Self::Local;
        }
        Self::of_name(name)
    }

    pub fn of_name(name: &str) -> Self {
        if curated_name(name) || name == GLOBAL {
            return Self::OpenAiCurated;
        }
        match name {
            WORKSPACE => Self::Workspace,
            SHARED | SHARED_PRIVATE => Self::SharedWithMe,
            SHARED_LINK => Self::SharedWithMeLink,
            _ => Self::Other,
        }
    }

    /// The place of the product's tab among the marketplaces' (`tab_order`).
    fn order(self) -> u8 {
        match self {
            Self::Workspace => 0,
            Self::SharedWithMe => 1,
            Self::SharedWithMeLink => 2,
            Self::Local => 3,
            Self::OpenAiCurated | Self::Other => 4,
        }
    }

    fn by_openai(self) -> bool {
        self == Self::OpenAiCurated
    }
}

/// A plugin of a marketplace (`PluginSummary`, the part the lists and the detail read).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    /// The interface's display name, else the name.
    pub display_name: String,
    pub remote_id: Option<String>,
    pub installed: bool,
    pub enabled: bool,
    pub policy: Policy,
    pub availability: Availability,
    pub auth: Auth,
    pub source: Source,
    /// The short description, else the long one, trimmed.
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub keywords: Vec<String>,
    pub local_version: Option<String>,
    pub share: Option<Share>,
}

fn trimmed(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

impl Plugin {
    fn of(plugin: wire::PluginSummary) -> Self {
        let interface = plugin.interface.as_ref();
        let display_name = trimmed(interface.and_then(|i| i.display_name.as_deref()))
            .unwrap_or_else(|| plugin.name.clone());
        let description = trimmed(interface.and_then(|i| {
            i.short_description
                .as_deref()
                .or(i.long_description.as_deref())
        }));
        let long_description = trimmed(interface.and_then(|i| i.long_description.as_deref()));
        let source = match plugin.source {
            wire::PluginSource::Local { .. } => Source::Local,
            wire::PluginSource::Git { url, ref_name, .. } => Source::Git {
                url,
                reference: ref_name,
            },
            wire::PluginSource::Npm {
                package, version, ..
            } => Source::Npm { package, version },
            wire::PluginSource::Remote => Source::Remote,
        };
        Self {
            id: plugin.id,
            name: plugin.name,
            display_name,
            remote_id: plugin.remote_plugin_id,
            installed: plugin.installed,
            enabled: plugin.enabled,
            policy: match plugin.install_policy {
                wire::PluginInstallPolicy::NotAvailable => Policy::NotAvailable,
                wire::PluginInstallPolicy::Available => Policy::Available,
                wire::PluginInstallPolicy::InstalledByDefault => Policy::InstalledByDefault,
            },
            availability: match plugin.availability {
                wire::PluginAvailability::Available => Availability::Available,
                wire::PluginAvailability::DisabledByAdmin => Availability::DisabledByAdmin,
            },
            auth: match plugin.auth_policy {
                wire::PluginAuthPolicy::OnInstall => Auth::OnInstall,
                wire::PluginAuthPolicy::OnUse => Auth::OnUse,
            },
            source,
            description,
            long_description,
            keywords: plugin.keywords,
            local_version: plugin.local_version,
            share: plugin.share_context.map(|share| Share {
                remote_id: share.remote_plugin_id,
                remote_version: share.remote_version,
                discoverability: share.discoverability.map(|kind| match kind {
                    wire::PluginShareDiscoverability::Listed => Discoverability::Listed,
                    wire::PluginShareDiscoverability::Unlisted => Discoverability::Unlisted,
                    wire::PluginShareDiscoverability::Private => Discoverability::Private,
                }),
                url: share.share_url,
                creator_name: share.creator_name,
                creator_id: share.creator_account_user_id,
                principals: share
                    .share_principals
                    .map(|all| all.into_iter().map(|principal| principal.name).collect()),
            }),
        }
    }

    /// `plugin_remote_identity`: the sharing's remote id, else the plugin's own.
    pub fn identity(&self) -> Option<&str> {
        self.share
            .as_ref()
            .map(|share| share.remote_id.as_str())
            .or(self.remote_id.as_deref())
    }

    /// `plugin_request_name`: what `plugin/read` and `plugin/install` name it by.
    pub fn request_name(&self) -> String {
        match (&self.source, self.identity()) {
            (Source::Remote, Some(remote)) => remote.to_string(),
            _ => self.name.clone(),
        }
    }

    /// `plugin_uninstall_id`: what `plugin/uninstall` names it by; none for a remote plugin that
    /// gave no identity.
    pub fn uninstall_id(&self) -> Option<String> {
        match self.source {
            Source::Remote => self.identity().map(str::to_string),
            _ => Some(self.id.clone()),
        }
    }

    pub fn status(&self) -> Status {
        if self.availability == Availability::DisabledByAdmin {
            return Status::Disabled;
        }
        if !self.installed && self.policy == Policy::InstalledByDefault {
            return Status::AdminAssigned;
        }
        match (self.installed, self.enabled, self.policy) {
            (true, true, _) => Status::Installed,
            (true, false, _) => Status::Disabled,
            (false, _, Policy::NotAvailable) => Status::NotInstallable,
            (false, _, Policy::Available) => Status::Available,
            (false, _, Policy::InstalledByDefault) => Status::Installed,
        }
    }

    /// `plugin_detail_status_label`: the state the detail's header names.
    pub fn detail_status(&self) -> DetailStatus {
        if self.availability == Availability::DisabledByAdmin {
            return DetailStatus::DisabledByAdmin;
        }
        if self.policy == Policy::InstalledByDefault {
            return if self.installed {
                DetailStatus::InstalledByAdmin
            } else {
                DetailStatus::EnabledByAdmin
            };
        }
        match (self.installed, self.enabled, self.policy) {
            (true, true, _) => DetailStatus::Installed,
            (true, false, _) => DetailStatus::Disabled,
            (false, _, Policy::NotAvailable) => DetailStatus::NotInstallable,
            (false, _, _) => DetailStatus::CanBeInstalled,
        }
    }

    fn is_remote(&self) -> bool {
        self.source == Source::Remote
    }

    pub fn toggle(&self) -> Toggle {
        if self.availability == Availability::DisabledByAdmin {
            Toggle::Blocked
        } else if self.installed && self.policy != Policy::InstalledByDefault {
            Toggle::Switch(self.enabled)
        } else {
            Toggle::Unavailable
        }
    }
}

/// The state a list row names (`plugin_status_label`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Disabled,
    AdminAssigned,
    Installed,
    NotInstallable,
    Available,
}

/// The state the detail's header names (`plugin_detail_status_label`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailStatus {
    DisabledByAdmin,
    InstalledByAdmin,
    EnabledByAdmin,
    Installed,
    Disabled,
    NotInstallable,
    CanBeInstalled,
}

/// The switch of a row (`SelectionToggle` and its placeholder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// Disabled by the admin: no switch, a blocked mark.
    Blocked,
    /// Not installed or installed by default: no switch.
    Unavailable,
    /// A switch, on or off.
    Switch(bool),
}

/// Where a plugin's marketplace is (`PluginLocation`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    Local { path: String },
    Remote { marketplace: String },
}

/// A request for a plugin's detail or install: where, and the name it is asked by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub location: Location,
    pub plugin: String,
}

/// A marketplace (`PluginMarketplaceEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marketplace {
    pub name: String,
    /// The file of a local marketplace; none for a remote catalog.
    pub path: Option<String>,
    /// The interface's display name, trimmed, when it has one.
    pub title: Option<String>,
    pub product: Product,
    pub plugins: Vec<Plugin>,
}

impl Marketplace {
    /// The name the tab and the rows say when the product has no label of its own
    /// (`marketplace_display_name` after the product's label).
    pub fn display(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    /// `marketplace_tab_id`.
    pub fn tab_id(&self) -> String {
        match &self.path {
            Some(path) => [MARKET, path].concat(),
            None => [MARKET, &self.name].concat(),
        }
    }

    fn of(entry: wire::PluginMarketplaceEntry, home: Option<&str>) -> Self {
        let path = entry.path.map(|path| path.to_string_lossy().into_owned());
        Self {
            product: Product::of(&entry.name, path.as_deref(), home),
            title: trimmed(
                entry
                    .interface
                    .as_ref()
                    .and_then(|interface| interface.display_name.as_deref()),
            ),
            name: entry.name,
            path,
            plugins: entry.plugins.into_iter().map(Plugin::of).collect(),
        }
    }
}

/// The marketplaces of a `plugin/list` answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    pub marketplaces: Vec<Marketplace>,
}

impl Catalog {
    /// The answer, without the marketplaces the CLI hides (`hide_cli_only_plugin_marketplaces`).
    pub fn of(response: wire::PluginListResponse, home: Option<&str>) -> Self {
        Self::of_marketplaces(response.marketplaces, home)
    }

    pub fn of_marketplaces(
        marketplaces: Vec<wire::PluginMarketplaceEntry>,
        home: Option<&str>,
    ) -> Self {
        Self {
            marketplaces: marketplaces
                .into_iter()
                .filter(|entry| !HIDDEN.contains(&entry.name.as_str()))
                .map(|entry| Marketplace::of(entry, home))
                .collect(),
        }
    }

    /// `merge_remote_marketplaces`: the remote sections replace the local copies of what they
    /// carry.
    pub fn merge(&mut self, remote: Vec<Marketplace>) {
        let names: Vec<&str> = remote.iter().map(|entry| entry.name.as_str()).collect();
        let curated_remote = names.contains(&GLOBAL);
        let sections: Vec<&str> = Section::ALL
            .iter()
            .flat_map(|section| section.marketplaces().iter().copied())
            .collect();
        self.marketplaces.retain(|entry| {
            if curated_remote && entry.path.is_some() && curated_name(&entry.name) {
                return false;
            }
            entry.path.is_some()
                || !sections.contains(&entry.name.as_str()) && !names.contains(&entry.name.as_str())
        });
        self.marketplaces.extend(remote);
    }

    /// `marketplace_tab_id_matching_saved_id`: the tab of this catalog a saved id names now.
    pub fn tab_for_saved(&self, saved: &str) -> Option<String> {
        if let Some(section) = Section::ALL
            .into_iter()
            .find(|section| section.fallback_id(saved))
            && let Some(found) = section.marketplaces().iter().find_map(|name| {
                self.marketplaces
                    .iter()
                    .find(|entry| entry.name == *name)
                    .map(Marketplace::tab_id)
            })
        {
            return Some(found);
        }
        if let Some(found) = self
            .marketplaces
            .iter()
            .map(Marketplace::tab_id)
            .find(|id| id == saved)
        {
            return Some(found);
        }
        let root = saved.strip_prefix(MARKET)?;
        if root.is_empty() {
            return None;
        }
        self.marketplaces
            .iter()
            .find(|entry| {
                entry
                    .path
                    .as_deref()
                    .is_some_and(|path| path == root || path.starts_with(&[root, "/"].concat()))
            })
            .map(Marketplace::tab_id)
    }

    /// `set the enabled flag of every plugin with this id` (`on_plugin_enabled_set`).
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        for plugin in self
            .marketplaces
            .iter_mut()
            .flat_map(|entry| entry.plugins.iter_mut())
            .filter(|plugin| plugin.id == id)
        {
            plugin.enabled = enabled;
        }
    }

    /// The plugin with this id in the marketplace whose tab id is `tab`.
    fn find(&self, tab: &str, id: &str) -> Option<(&Marketplace, &Plugin)> {
        let market = self
            .marketplaces
            .iter()
            .find(|entry| entry.tab_id() == tab)?;
        market
            .plugins
            .iter()
            .find(|plugin| plugin.id == id)
            .map(|plugin| (market, plugin))
    }

    /// The enabled flag of the plugin with this id, as the list shows it.
    pub fn enabled(&self, id: &str) -> Option<bool> {
        self.marketplaces
            .iter()
            .flat_map(|entry| entry.plugins.iter())
            .find(|plugin| plugin.id == id)
            .map(|plugin| plugin.enabled)
    }
}

/// A marketplace the backend keeps apart from the local list: its tab is a placeholder until
/// its plugins are known (`REMOTE_MARKETPLACE_SECTIONS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Workspace,
    SharedWithMe,
}

impl Section {
    pub const ALL: [Section; 2] = [Section::Workspace, Section::SharedWithMe];

    pub fn id(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::SharedWithMe => "shared-with-me",
        }
    }

    /// The id the loading placeholder is named by.
    fn loading_id(self) -> &'static str {
        match self {
            Self::Workspace => "workspace-loading",
            Self::SharedWithMe => "shared-with-me-loading",
        }
    }

    pub fn marketplaces(self) -> &'static [&'static str] {
        match self {
            Self::Workspace => &[WORKSPACE],
            Self::SharedWithMe => &[SHARED, SHARED_PRIVATE, SHARED_LINK],
        }
    }

    /// Whether a section that loaded empty still has a tab.
    fn shows_empty(self) -> bool {
        self == Self::Workspace
    }

    fn order(self) -> u8 {
        match self {
            Self::Workspace => 0,
            Self::SharedWithMe => 1,
        }
    }

    fn holds(self, name: &str) -> bool {
        self.marketplaces().contains(&name)
    }

    /// A placeholder tab id of this section.
    fn fallback_id(self, tab: &str) -> bool {
        tab.strip_prefix(LOADING) == Some(self.loading_id())
            || tab.strip_prefix(EMPTY) == Some(self.id())
            || tab.strip_prefix(FAILED) == Some(self.id())
    }

    fn contains_tab(self, tab: &str) -> bool {
        self.fallback_id(tab)
            || tab
                .strip_prefix(MARKET)
                .is_some_and(|name| self.holds(name))
    }
}

/// What a remote section's request left as an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// `vertical`: the OpenAI Curated tab.
    Vertical,
    Workspace,
    SharedWithMe,
}

impl Part {
    pub fn id(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Workspace => "workspace",
            Self::SharedWithMe => "shared-with-me",
        }
    }
}

/// What to tell the user to do about a section's error (`plugin_remote_section_error_next_step`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    None,
    SignInWithChatGpt,
    SignIn,
    AskAdminToEnable,
    SwitchWorkspace,
    CheckWorkspace,
    Update,
    TryLater,
    AskAdminAccess,
    AskSharer,
}

impl Next {
    /// The error text, read for what it says (the oracle matches on the lower-cased message).
    pub fn of(part: Part, error: &str) -> Self {
        let err = error.to_ascii_lowercase();
        let has = |text: &str| err.contains(text);
        if has("api key auth is not supported") {
            Self::SignInWithChatGpt
        } else if has("authentication required") || has("not signed in") || has("not logged in") {
            Self::SignIn
        } else if has("codex plugins are disabled")
            || has("plugin sharing is disabled")
            || has("plugin sharing is not enabled")
            || has("feature disabled")
        {
            Self::AskAdminToEnable
        } else if has("workspace") && (has("access") || has("mismatch")) {
            Self::SwitchWorkspace
        } else if has("not found") || has("status 404") {
            Self::CheckWorkspace
        } else if has("old build") || has("update codex") || has("stale") {
            Self::Update
        } else if has("service unavailable")
            || has("temporarily unavailable")
            || has("status 503")
            || has("failed to send")
            || has("request")
            || has("status")
        {
            Self::TryLater
        } else if has("disabled by admin") || has("admin disabled") {
            Self::AskAdminAccess
        } else if part == Part::SharedWithMe && has("plugin") && has("disabled") {
            Self::AskSharer
        } else {
            Self::None
        }
    }
}

/// A section that could not be loaded (`PluginRemoteSectionError`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionError {
    pub part: Part,
    pub cause: Cause,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The session did not turn plugin sharing on.
    SharingDisabled,
    /// The backend's message, and what to do about it.
    Failed { error: String, next: Next },
}

impl SectionError {
    pub fn failed(part: Part, error: String) -> Self {
        let next = Next::of(part, &error);
        Self {
            part,
            cause: Cause::Failed { error, next },
        }
    }
}

/// How the remote sections stand (`plugin_remote_sections_loading`, `..._loaded`,
/// `plugin_remote_section_errors`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sections {
    pub loading: bool,
    pub loaded: bool,
    pub errors: Vec<SectionError>,
    /// The curated section is a request of its own (the remote plugin feature is off).
    pub vertical: bool,
}

impl Sections {
    fn error(&self, part: Part) -> Option<&SectionError> {
        self.errors.iter().find(|error| error.part == part)
    }
}

/// What the user's config says of the marketplaces and the features the list depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub enabled: bool,
    /// `Feature::RemotePlugin`.
    pub remote: bool,
    /// `Feature::PluginSharing`.
    pub sharing: bool,
    /// The home directory (the personal marketplace is under it).
    pub home: Option<String>,
    /// The marketplaces of the user's config.
    pub configured: Vec<Configured>,
}

/// A marketplace of the user's config (`marketplace_is_user_configured`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configured {
    pub name: String,
    /// Its `source_type` is `git`: it can be upgraded.
    pub git: bool,
}

impl Config {
    fn configured(&self, name: &str) -> Option<&Configured> {
        self.configured.iter().find(|entry| entry.name == name)
    }
}

/// A row of a tab (`plugin_selection_items`): what the view needs to say it and to act on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Unique among the rows of the catalog: the marketplace tab the row is listed under and
    /// the plugin (what a verb about the row names).
    pub key: RowKey,
    pub name: String,
    pub plugin_name: String,
    /// The marketplace's name as the row says it: the product's label, else this.
    pub marketplace: String,
    pub product: Product,
    pub status: Status,
    pub toggle: Toggle,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    /// What `Enter` opens; none when the plugin has no detail to read.
    pub detail: Option<Target>,
}

impl Row {
    pub fn can_view(&self) -> bool {
        self.detail.is_some()
    }
}

/// A trailing or replacing note of a tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    None,
    /// The section is still loading.
    Loading,
    /// The section failed.
    Failed(SectionError),
}

/// What a tab is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    All,
    Installed,
    Curated,
    /// A marketplace; the same label for two of them is told apart (`index` of `of`).
    Marketplace {
        name: String,
        product: Product,
        /// The user's config holds it: it can be removed.
        remove: bool,
        /// It is a configured git marketplace with a local file: it can be upgraded.
        upgrade: bool,
        twin: Option<(usize, usize)>,
    },
    /// A placeholder of a remote section.
    Section {
        section: Section,
        state: Placeholder,
    },
    Add,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placeholder {
    Loading,
    Empty,
    Failed(SectionError),
}

/// A tab of the browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: String,
    pub kind: Kind,
    pub installed: usize,
    pub total: usize,
    pub rows: Vec<Row>,
    pub note: Note,
    /// The marketplace was just added: its header says so.
    pub fresh: bool,
}

type Entry<'a> = (&'a Marketplace, &'a Plugin);

fn identity_of(entry: &Entry) -> Option<String> {
    entry.1.identity().map(str::to_string)
}

/// `plugin_entry_preferred`: whether `candidate` replaces `existing` (the same remote plugin).
fn preferred(candidate: &Entry, existing: &Entry) -> bool {
    let (c, e) = (candidate.1, existing.1);
    if c.installed != e.installed {
        return c.installed;
    }
    let admin = |plugin: &Plugin| plugin.policy == Policy::InstalledByDefault;
    if admin(c) != admin(e) {
        return admin(c);
    }
    let local_share = |plugin: &Plugin| plugin.share.is_some() && !plugin.is_remote();
    if local_share(c) != local_share(e) {
        return local_share(c);
    }
    !c.is_remote() && e.is_remote()
}

/// `dedupe_plugin_entries`: a remote plugin and its local copy are one row.
fn dedupe(entries: Vec<Entry<'_>>) -> Vec<Entry<'_>> {
    let mut kept: Vec<Entry> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for entry in entries {
        let Some(identity) = identity_of(&entry) else {
            kept.push(entry);
            continue;
        };
        match at.get(&identity).copied() {
            Some(index) => {
                if preferred(&entry, &kept[index]) {
                    kept[index] = entry;
                }
            }
            None => {
                at.insert(identity, kept.len());
                kept.push(entry);
            }
        }
    }
    kept
}

fn entries<'a>(markets: impl IntoIterator<Item = &'a Marketplace>) -> Vec<Entry<'a>> {
    dedupe(
        markets
            .into_iter()
            .flat_map(|market| market.plugins.iter().map(move |plugin| (market, plugin)))
            .collect(),
    )
}

/// `sort_plugin_entries`: installed first, then by display name (case-insensitive first).
fn sort(list: &mut [Entry<'_>]) {
    list.sort_by(|left, right| {
        right
            .1
            .installed
            .cmp(&left.1.installed)
            .then_with(|| {
                left.1
                    .display_name
                    .to_ascii_lowercase()
                    .cmp(&right.1.display_name.to_ascii_lowercase())
            })
            .then_with(|| left.1.display_name.cmp(&right.1.display_name))
            .then_with(|| left.1.name.cmp(&right.1.name))
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
}

/// The local copy a remote plugin's detail is read from (`preferred_local_plugin_sources`).
struct LocalCopy<'a> {
    path: &'a str,
    name: &'a str,
    installed: bool,
    policy: Policy,
}

fn local_copies(markets: &[Marketplace]) -> HashMap<&str, LocalCopy<'_>> {
    let mut found = HashMap::new();
    for market in markets {
        let Some(path) = market.path.as_deref() else {
            continue;
        };
        for plugin in &market.plugins {
            if plugin.is_remote() {
                continue;
            }
            let Some(share) = &plugin.share else { continue };
            found
                .entry(share.remote_id.as_str())
                .or_insert_with(|| LocalCopy {
                    path,
                    name: &plugin.name,
                    installed: plugin.installed,
                    policy: plugin.policy,
                });
        }
    }
    found
}

/// `plugin_location_for_marketplace`.
fn location_of(market: &Marketplace, plugin: &Plugin) -> Option<Location> {
    if let Some(path) = &market.path {
        return Some(Location::Local { path: path.clone() });
    }
    plugin.identity().map(|_| Location::Remote {
        marketplace: market.name.clone(),
    })
}

/// `plugin_detail_request_for_entry`.
fn detail_of(
    market: &Marketplace,
    plugin: &Plugin,
    copies: &HashMap<&str, LocalCopy<'_>>,
) -> Option<Target> {
    if plugin.is_remote()
        && let Some(identity) = plugin.identity()
        && let Some(copy) = copies.get(identity)
        && copy.installed == plugin.installed
        && copy.policy == plugin.policy
    {
        return Some(Target {
            location: Location::Local {
                path: copy.path.to_string(),
            },
            plugin: copy.name.to_string(),
        });
    }
    location_of(market, plugin).map(|location| Target {
        location,
        plugin: plugin.request_name(),
    })
}

fn rows(list: Vec<Entry<'_>>, copies: &HashMap<&str, LocalCopy<'_>>) -> Vec<Row> {
    let mut list = list;
    sort(&mut list);
    list.into_iter()
        .map(|(market, plugin)| Row {
            key: RowKey::new(market.tab_id(), plugin.id.clone()),
            name: plugin.display_name.clone(),
            plugin_name: plugin.name.clone(),
            marketplace: market.display().to_string(),
            product: market.product,
            status: plugin.status(),
            toggle: plugin.toggle(),
            description: plugin.description.clone(),
            keywords: plugin.keywords.clone(),
            detail: detail_of(market, plugin, copies),
        })
        .collect()
}

/// The tab id of the marketplace installed under `root` (`marketplace_tab_id_from_path`).
pub fn tab_id_of_root(root: &str) -> String {
    [MARKET, root].concat()
}

/// A plugin's row in the list of a directory: the tab of the marketplace it is listed under and
/// the plugin's id there. Typed, so a verb names a row and never a string that was parsed; the
/// text form (`Display`, `parse`) is what the picker's rows carry as their id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowKey {
    /// The tab id of the marketplace (`Marketplace::tab_id`).
    pub market: String,
    pub plugin: String,
}

impl RowKey {
    pub fn new(market: impl Into<String>, plugin: impl Into<String>) -> Self {
        Self {
            market: market.into(),
            plugin: plugin.into(),
        }
    }

    /// The row a picker item id names (`Display` is its inverse); none for an id of another kind
    /// of item (`Add marketplace`).
    /// The market's byte length leads (`plugin:<len>|<market><plugin>`), so a `|` in either name
    /// cannot move the split.
    pub fn parse(id: &str) -> Option<Self> {
        let (len, rest) = id.strip_prefix("plugin:")?.split_once('|')?;
        let (market, plugin) = rest.split_at_checked(len.parse().ok()?)?;
        Some(Self::new(market, plugin))
    }
}

impl fmt::Display for RowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plugin:{}|{}{}",
            self.market.len(),
            self.market,
            self.plugin
        )
    }
}

impl Catalog {
    /// The plugin a row names: its marketplace, and the plugin.
    pub fn row(&self, row: &RowKey) -> Option<(&Marketplace, &Plugin)> {
        self.find(&row.market, &row.plugin)
    }

    /// The detail request of a row.
    pub fn detail_target(&self, row: &RowKey) -> Option<Target> {
        let (market, plugin) = self.row(row)?;
        detail_of(market, plugin, &local_copies(&self.marketplaces))
    }

    /// `plugins_popup_params`: the tabs, in the oracle's order. `fresh` is the tab of a
    /// marketplace just added.
    pub fn tabs(&self, config: &Config, remote: &Sections, fresh: Option<&str>) -> Vec<Tab> {
        let copies = local_copies(&self.marketplaces);
        let all = entries(&self.marketplaces);
        let counts = |list: &[Entry]| {
            (
                list.iter().filter(|entry| entry.1.installed).count(),
                list.len(),
            )
        };
        let (installed, total) = counts(&all);
        let installed_list: Vec<Entry> = all.iter().filter(|e| e.1.installed).copied().collect();
        let mut tabs = vec![
            Tab {
                id: ALL.to_string(),
                kind: Kind::All,
                installed,
                total,
                rows: rows(all, &copies),
                note: Note::None,
                fresh: false,
            },
            Tab {
                id: INSTALLED.to_string(),
                kind: Kind::Installed,
                installed,
                total: installed,
                rows: rows(installed_list, &copies),
                note: Note::None,
                fresh: false,
            },
        ];

        let curated = entries(self.marketplaces.iter().filter(|m| m.product.by_openai()));
        let (curated_installed, curated_total) = counts(&curated);
        let has_curated = !curated.is_empty();
        let loading = remote.loading && remote.vertical;
        let failed = remote.error(Part::Vertical);
        let note = if loading {
            Note::Loading
        } else if let Some(error) = failed {
            Note::Failed(error.clone())
        } else {
            Note::None
        };
        tabs.push(Tab {
            id: CURATED.to_string(),
            kind: Kind::Curated,
            installed: curated_installed,
            total: curated_total,
            rows: rows(curated, &copies),
            note: if has_curated || !matches!(note, Note::None) {
                note
            } else {
                Note::None
            },
            fresh: false,
        });

        let mut others: Vec<&Marketplace> = self
            .marketplaces
            .iter()
            .filter(|m| !m.product.by_openai())
            .collect();
        others.sort_by_cached_key(|m| {
            let label = Self::label_key(m);
            (
                m.product.order(),
                label.to_ascii_lowercase(),
                label,
                m.name.clone(),
            )
        });

        let mut extra: Vec<(u8, Tab)> = Vec::new();
        for section in Section::ALL {
            if let Some(found) = self.placeholder(section, remote) {
                extra.push(found);
            }
        }
        let keys: Vec<String> = others.iter().map(|m| Self::label_key(m)).collect();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (market, key) in others.iter().zip(&keys) {
            let list = entries([*market]);
            let (installed, total) = counts(&list);
            let tab = market.tab_id();
            let same = keys.iter().filter(|other| *other == key).count();
            let twin = (same > 1).then(|| {
                let index = seen.entry(key.as_str()).or_insert(0);
                *index += 1;
                (*index, same)
            });
            let configured = config.configured(&market.name);
            extra.push((
                market.product.order(),
                Tab {
                    fresh: fresh == Some(tab.as_str()),
                    id: tab,
                    kind: Kind::Marketplace {
                        name: market.name.clone(),
                        product: market.product,
                        remove: configured.is_some(),
                        upgrade: market.path.is_some() && configured.is_some_and(|c| c.git),
                        twin,
                    },
                    installed,
                    total,
                    rows: rows(list, &copies),
                    note: Note::None,
                },
            ));
        }
        extra.sort_by_key(|(order, _)| *order);
        tabs.extend(extra.into_iter().map(|(_, tab)| tab));
        tabs.push(Tab {
            id: ADD.to_string(),
            kind: Kind::Add,
            installed: 0,
            total: 0,
            rows: Vec::new(),
            note: Note::None,
            fresh: false,
        });
        tabs
    }

    /// What two marketplaces are told apart by: their label (the product's, else their name).
    fn label_key(market: &Marketplace) -> String {
        match market.product {
            Product::Other => market.display().to_string(),
            product => [PRODUCT, &(product as u8).to_string()].concat(),
        }
    }

    /// `RemoteMarketplaceSection::fallback_tab`.
    fn placeholder(&self, section: Section, remote: &Sections) -> Option<(u8, Tab)> {
        if self.marketplaces.iter().any(|m| section.holds(&m.name)) {
            return None;
        }
        let part = match section {
            Section::Workspace => Part::Workspace,
            Section::SharedWithMe => Part::SharedWithMe,
        };
        let (id, state) = if remote.loading {
            (
                [LOADING, section.loading_id()].concat(),
                Placeholder::Loading,
            )
        } else if remote.loaded {
            if let Some(error) = remote.error(part) {
                (
                    [FAILED, section.id()].concat(),
                    Placeholder::Failed(error.clone()),
                )
            } else if !section.shows_empty() {
                return None;
            } else {
                ([EMPTY, section.id()].concat(), Placeholder::Empty)
            }
        } else {
            return None;
        };
        Some((
            section.order(),
            Tab {
                id,
                kind: Kind::Section { section, state },
                installed: 0,
                total: 0,
                rows: Vec::new(),
                note: Note::None,
                fresh: false,
            },
        ))
    }
}

const PRODUCT: &str = "product:";

/// `plugin_tab_id_matching_saved_id`: the tab of `tabs` a saved id names now.
pub fn tab_matching(saved: &str, tabs: &[Tab]) -> Option<String> {
    if let Some(found) = tabs.iter().find(|tab| tab.id == saved) {
        return Some(found.id.clone());
    }
    let section = Section::ALL
        .into_iter()
        .find(|section| section.contains_tab(saved))?;
    tabs.iter()
        .find(|tab| section.contains_tab(&tab.id))
        .map(|tab| tab.id.clone())
}

/// A plugin's detail (`PluginDetail`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detail {
    pub marketplace: String,
    pub path: Option<String>,
    pub product: Product,
    pub plugin: Plugin,
    /// The description the header says: the detail's own, else the long one, else the short one.
    pub description: Option<String>,
    pub skills: Vec<String>,
    /// The hook events and how many handlers each has, in first-seen order.
    pub hooks: Vec<(String, usize)>,
    pub apps: Vec<String>,
    pub mcp_servers: Vec<String>,
}

/// What the detail offers beside going back (`plugin_detail_popup_params`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Installed by the admin: nothing to do.
    InstalledByAdmin,
    Uninstall {
        id: String,
    },
    /// A remote plugin gave no identity to uninstall it by.
    UninstallUnavailable,
    DisabledByAdmin,
    NotInstallable,
    Install(Target),
    /// The plugin gave no location to install it from.
    InstallUnavailable,
}

impl Detail {
    pub fn of(detail: wire::PluginDetail, home: Option<&str>) -> Self {
        let path = detail
            .marketplace_path
            .map(|path| path.to_string_lossy().into_owned());
        let plugin = Plugin::of(detail.summary);
        let description = trimmed(
            detail
                .description
                .as_deref()
                .or(plugin.long_description.as_deref())
                .or(plugin.description.as_deref()),
        );
        let mut hooks: Vec<(String, usize)> = Vec::new();
        for hook in &detail.hooks {
            let event = format!("{:?}", hook.event_name);
            match hooks.iter_mut().find(|(seen, _)| *seen == event) {
                Some((_, count)) => *count += 1,
                None => hooks.push((event, 1)),
            }
        }
        Self {
            product: Product::of(&detail.marketplace_name, path.as_deref(), home),
            marketplace: detail.marketplace_name,
            path,
            plugin,
            description,
            skills: detail.skills.into_iter().map(|skill| skill.name).collect(),
            hooks,
            apps: detail.apps.into_iter().map(|app| app.name).collect(),
            mcp_servers: detail.mcp_servers,
        }
    }

    /// `plugin_detail_location`.
    pub fn location(&self) -> Option<Location> {
        if let Some(path) = &self.path {
            return Some(Location::Local { path: path.clone() });
        }
        self.plugin.identity().map(|_| Location::Remote {
            marketplace: self.marketplace.clone(),
        })
    }

    /// The disclosure of an app's terms is shown until the plugin is installed.
    pub fn disclosure(&self) -> bool {
        !self.plugin.installed
    }

    pub fn action(&self) -> Action {
        let plugin = &self.plugin;
        if plugin.installed {
            if plugin.policy == Policy::InstalledByDefault {
                Action::InstalledByAdmin
            } else if let Some(id) = plugin.uninstall_id() {
                Action::Uninstall { id }
            } else {
                Action::UninstallUnavailable
            }
        } else if plugin.availability == Availability::DisabledByAdmin {
            Action::DisabledByAdmin
        } else if plugin.policy == Policy::NotAvailable {
            Action::NotInstallable
        } else if let Some(location) = self.location() {
            Action::Install(Target {
                location,
                plugin: plugin.request_name(),
            })
        } else {
            Action::InstallUnavailable
        }
    }
}
