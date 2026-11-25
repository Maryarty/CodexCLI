//! The words of `/plugins` (oracle `chatwidget/plugin_catalog.rs`, `chatwidget/plugins.rs`,
//! `app/background_requests.rs`): a table of `&'static str` fields per locale, read by
//! reference through `of(cx)`. `EN` is the oracle's text byte for byte. A language is one more
//! `const` here, registered with `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::features::plugins::catalog::{Cause, DetailStatus, Discoverability};
use codex_tui_core::features::plugins::catalog::{Next, Part, Section};
use codex_tui_core::features::plugins::{PluginStatus as Status, Product, Say};
use codex_tui_core::rpc::RpcError;

use crate::l10n::Tables;
use crate::l10n::lang;

/// What stands between a note's message and its hint.
const INLINE_GAP: &str = " ";

/// The words of the plugin browser, its pages and its prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The title every page has.
    pub title: &'static str,
    // `plugins_popup_params`: the tabs and what each says under the title.
    pub tab_all: &'static str,
    pub tab_installed: Msg<1>,
    pub tab_add: &'static str,
    pub tab_twin: Msg<3>,
    pub product_curated: &'static str,
    pub product_workspace: &'static str,
    pub product_shared: &'static str,
    pub product_shared_link: &'static str,
    pub product_local: &'static str,
    pub all_title: &'static str,
    pub all_count: Msg<2>,
    pub installed_title: &'static str,
    pub installed_count: Msg<1>,
    pub curated_title: &'static str,
    pub curated_count: Msg<2>,
    pub market_title: Msg<1>,
    pub market_count: Msg<3>,
    pub market_fresh_title: Msg<1>,
    pub market_fresh_count: &'static str,
    pub section_loading_title: Msg<1>,
    pub section_loading_count: &'static str,
    pub section_empty_count: &'static str,
    pub section_failed_title: Msg<1>,
    pub section_failed_count: &'static str,
    pub add_title: &'static str,
    pub add_count: &'static str,
    pub search: &'static str,
    // The rows of an empty tab and the placeholders of the remote sections.
    pub none_all: &'static str,
    pub none_all_description: &'static str,
    pub none_installed: &'static str,
    pub none_curated: &'static str,
    pub none_market: &'static str,
    pub none_workspace: &'static str,
    pub none_workspace_description: &'static str,
    pub none_shared: &'static str,
    pub none_shared_description: &'static str,
    pub row_loading: Msg<1>,
    pub row_unavailable: Msg<1>,
    pub loading_curated: &'static str,
    pub loading_workspace: &'static str,
    pub loading_shared: &'static str,
    pub add_row: &'static str,
    pub add_row_description: &'static str,
    pub add_row_selected: &'static str,
    // `plugin_status_label` and the description of a row.
    pub status_disabled: &'static str,
    pub status_admin: &'static str,
    pub status_installed: &'static str,
    pub status_not_installable: &'static str,
    pub status_available: &'static str,
    pub dot: &'static str,
    pub selected: Msg<2>,
    pub selected_toggle: Msg<1>,
    pub selected_toggle_only: Msg<1>,
    pub word_enable: &'static str,
    pub word_disable: &'static str,
    pub selected_view: &'static str,
    pub selected_unavailable: &'static str,
    pub selected_install: &'static str,
    pub selected_remote: &'static str,
    pub unavailable_reason: &'static str,
    /// The mark of a row that cannot be switched, and one the admin blocked.
    pub mark_unavailable: &'static str,
    pub mark_blocked: &'static str,
    // `plugins_popup_hint_line`.
    pub hint_upgrade: Msg<1>,
    pub hint_remove: Msg<1>,
    pub hint_toggle: &'static str,
    pub hint_tabs: &'static str,
    pub hint_details: &'static str,
    pub hint_close: &'static str,
    pub hint_select_close: &'static str,
    pub hint_join: &'static str,
    // The errors of the remote sections (`plugin_remote_section_error_next_step`).
    pub error_next: Msg<2>,
    pub sharing_disabled: &'static str,
    pub next_chatgpt_auth: &'static str,
    pub next_sign_in: &'static str,
    pub next_enable: &'static str,
    pub next_switch: &'static str,
    pub next_check: &'static str,
    pub next_update: &'static str,
    pub next_later: &'static str,
    pub next_admin: &'static str,
    pub next_sharer: &'static str,
    // The loading pages and the failure pages.
    pub loading_head: &'static str,
    pub loading_note: &'static str,
    pub loading_row: &'static str,
    pub loading_row_description: &'static str,
    pub failed_subtitle: &'static str,
    pub failed_row: &'static str,
    pub add_loading: &'static str,
    pub add_loading_description: &'static str,
    pub add_failed_subtitle: &'static str,
    pub add_failed_row: &'static str,
    pub add_failed_description: &'static str,
    pub retry: &'static str,
    pub retry_add_description: &'static str,
    pub retry_remove_description: &'static str,
    pub remove_confirm_head: Msg<1>,
    pub remove_confirm_note: &'static str,
    pub remove_row: &'static str,
    pub remove_row_description: &'static str,
    pub remove_keep_description: &'static str,
    pub remove_loading_subtitle: Msg<1>,
    pub remove_loading_row: &'static str,
    pub remove_loading_description: &'static str,
    pub remove_failed_subtitle: &'static str,
    pub remove_failed_row: &'static str,
    pub remove_failed_description: &'static str,
    pub upgrade_loading_one: Msg<1>,
    pub upgrade_loading_all: &'static str,
    pub upgrade_loading_description: &'static str,
    pub detail_loading: Msg<1>,
    pub detail_loading_row: &'static str,
    pub detail_loading_description: &'static str,
    pub detail_failed_subtitle: &'static str,
    pub detail_failed_row: &'static str,
    pub install_loading_subtitle: Msg<1>,
    pub install_loading_row: &'static str,
    pub install_loading_description: &'static str,
    pub uninstall_loading_subtitle: Msg<1>,
    pub uninstall_loading_row: &'static str,
    pub uninstall_loading_description: &'static str,
    // `plugin_detail_popup_params`.
    pub detail_head: Msg<3>,
    pub detail_disabled_by_admin: &'static str,
    pub detail_installed_by_admin: &'static str,
    pub detail_enabled_by_admin: &'static str,
    pub detail_installed: &'static str,
    pub detail_disabled: &'static str,
    pub detail_not_installable: &'static str,
    pub detail_can_install: &'static str,
    pub disclosure_start: &'static str,
    pub disclosure_terms: &'static str,
    pub disclosure_and: &'static str,
    pub disclosure_privacy: &'static str,
    pub disclosure_stop: &'static str,
    pub disclosure_link: &'static str,
    pub disclosure_end: &'static str,
    pub back: &'static str,
    pub back_description: &'static str,
    pub admin_installed: &'static str,
    pub admin_installed_description: &'static str,
    pub uninstall: &'static str,
    pub uninstall_description: &'static str,
    pub uninstall_missing: &'static str,
    pub install: &'static str,
    pub install_description: &'static str,
    pub install_disabled: &'static str,
    pub install_not_available: &'static str,
    pub install_no_location: &'static str,
    pub meta_source: &'static str,
    pub meta_auth: &'static str,
    pub meta_version: &'static str,
    pub meta_sharing: &'static str,
    pub meta_skills: &'static str,
    pub meta_hooks: &'static str,
    pub meta_apps: &'static str,
    pub meta_mcp: &'static str,
    pub source_local: &'static str,
    pub source_git: Msg<1>,
    pub source_git_ref: Msg<2>,
    pub source_npm: Msg<1>,
    pub source_npm_version: Msg<2>,
    pub source_remote: Msg<1>,
    pub auth_on_install: &'static str,
    pub auth_on_use: &'static str,
    pub version_local: Msg<1>,
    pub version_remote: Msg<1>,
    pub share_listed: &'static str,
    pub share_unlisted: &'static str,
    pub share_private: &'static str,
    pub share_creator_account: Msg<2>,
    pub share_creator: Msg<1>,
    pub share_account: Msg<1>,
    pub share_no_principals: &'static str,
    pub share_one_principal: Msg<1>,
    pub share_principals: Msg<1>,
    pub share_remote_id: Msg<1>,
    pub none_skills: &'static str,
    pub none_hooks: &'static str,
    pub none_apps: &'static str,
    pub none_mcp: &'static str,
    pub list_join: &'static str,
    pub hook: Msg<2>,
    // `plugin_install_auth_popup_params`.
    pub auth_installed: Msg<1>,
    pub auth_step: Msg<3>,
    pub auth_already: &'static str,
    pub auth_needed: &'static str,
    pub auth_install: &'static str,
    pub auth_manage: &'static str,
    pub auth_open_description: &'static str,
    pub auth_open_selected: &'static str,
    pub auth_no_link: &'static str,
    pub auth_no_link_description: &'static str,
    pub auth_continue: &'static str,
    pub auth_continue_description: &'static str,
    pub auth_continue_selected: &'static str,
    pub auth_done: &'static str,
    pub auth_done_description: &'static str,
    pub auth_done_selected: &'static str,
    pub auth_skip: &'static str,
    pub auth_skip_description: &'static str,
    pub auth_skip_selected: &'static str,
    // `marketplace_add_prompt` (chatwidget/plugins.rs).
    pub prompt_title: &'static str,
    pub prompt_context: &'static str,
    pub prompt_placeholder: &'static str,
    pub num: NumSyms,
}

fn count(n: usize) -> Arg<'static> {
    Arg::Int(u64::try_from(n).unwrap_or(u64::MAX))
}

impl Strings {
    /// A message with its arguments, numbers in this locale's digits.
    pub fn fmt<const N: usize>(&self, msg: Msg<N>, args: [Arg<'_>; N]) -> String {
        Fmt::new(msg, args, &self.num).to_string()
    }

    /// A message of one text.
    pub fn with(&self, msg: Msg<1>, text: &str) -> String {
        self.fmt(msg, [Arg::Text(text)])
    }

    /// A message of one number.
    pub fn with_count(&self, msg: Msg<1>, n: usize) -> String {
        self.fmt(msg, [count(n)])
    }

    /// A message of two numbers.
    pub fn with_counts(&self, msg: Msg<2>, first: usize, second: usize) -> String {
        self.fmt(msg, [count(first), count(second)])
    }

    /// `Installed {installed} of {total} {label} plugins.`
    pub fn market_counts(&self, installed: usize, total: usize, label: &str) -> String {
        self.fmt(
            self.market_count,
            [count(installed), count(total), Arg::Text(label)],
        )
    }

    /// `{label} ({index}/{total})`: a tab label told apart from its twins.
    pub fn twin(&self, label: &str, index: usize, total: usize) -> String {
        self.fmt(
            self.tab_twin,
            [Arg::Text(label), count(index), count(total)],
        )
    }

    /// The label of a marketplace by its product; none for one named by its own words.
    pub fn product(&self, product: Product) -> Option<&'static str> {
        match product {
            Product::OpenAiCurated => Some(self.product_curated),
            Product::Workspace => Some(self.product_workspace),
            Product::SharedWithMe => Some(self.product_shared),
            Product::SharedWithMeLink => Some(self.product_shared_link),
            Product::Local => Some(self.product_local),
            Product::Other => None,
        }
    }

    /// The label of a remote section (its tab and its error).
    pub fn section(&self, section: Section) -> &'static str {
        match section {
            Section::Workspace => self.product_workspace,
            Section::SharedWithMe => self.product_shared,
        }
    }

    /// The label of the part of the catalog a request left an error for.
    pub fn part(&self, part: Part) -> &'static str {
        match part {
            Part::Vertical => self.product_curated,
            Part::Workspace => self.product_workspace,
            Part::SharedWithMe => self.product_shared,
        }
    }

    pub fn status(&self, status: Status) -> &'static str {
        match status {
            Status::Disabled => self.status_disabled,
            Status::AdminAssigned => self.status_admin,
            Status::Installed => self.status_installed,
            Status::NotInstallable => self.status_not_installable,
            Status::Available => self.status_available,
        }
    }

    pub fn detail_status(&self, status: DetailStatus) -> &'static str {
        match status {
            DetailStatus::DisabledByAdmin => self.detail_disabled_by_admin,
            DetailStatus::InstalledByAdmin => self.detail_installed_by_admin,
            DetailStatus::EnabledByAdmin => self.detail_enabled_by_admin,
            DetailStatus::Installed => self.detail_installed,
            DetailStatus::Disabled => self.detail_disabled,
            DetailStatus::NotInstallable => self.detail_not_installable,
            DetailStatus::CanBeInstalled => self.detail_can_install,
        }
    }

    pub fn discoverability(&self, kind: Discoverability) -> &'static str {
        match kind {
            Discoverability::Listed => self.share_listed,
            Discoverability::Unlisted => self.share_unlisted,
            Discoverability::Private => self.share_private,
        }
    }

    /// The message of a section that could not be loaded: the backend's own words, then what to
    /// do about it (`plugin_remote_section_error_message`).
    pub fn section_error(&self, cause: &Cause) -> String {
        match cause {
            Cause::SharingDisabled => self.sharing_disabled.to_string(),
            Cause::Failed { error, next } => match self.next(*next) {
                Some(step) => self.fmt(self.error_next, [Arg::Text(error), Arg::Text(step)]),
                None => error.clone(),
            },
        }
    }

    fn next(&self, next: Next) -> Option<&'static str> {
        Some(match next {
            Next::None => return None,
            Next::SignInWithChatGpt => self.next_chatgpt_auth,
            Next::SignIn => self.next_sign_in,
            Next::AskAdminToEnable => self.next_enable,
            Next::SwitchWorkspace => self.next_switch,
            Next::CheckWorkspace => self.next_check,
            Next::Update => self.next_update,
            Next::TryLater => self.next_later,
            Next::AskAdminAccess => self.next_admin,
            Next::AskSharer => self.next_sharer,
        })
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "Plugins",
    tab_all: "All Plugins",
    tab_installed: msg!("Installed ({0})"),
    tab_add: "Add Marketplace",
    tab_twin: msg!("{0} ({1}/{2})"),
    product_curated: "OpenAI Curated",
    product_workspace: "Workspace",
    product_shared: "Shared with me",
    product_shared_link: "Shared with me (link)",
    product_local: "Local",
    all_title: "Browse plugins from available marketplaces.",
    all_count: msg!("Installed {0} of {1} available plugins."),
    installed_title: "Installed plugins.",
    installed_count: msg!("Showing {0} installed plugins."),
    curated_title: "OpenAI Curated marketplace.",
    curated_count: msg!("Installed {0} of {1} OpenAI Curated plugins."),
    market_title: msg!("{0}."),
    market_count: msg!("Installed {0} of {1} {2} plugins."),
    market_fresh_title: msg!("{0} installed successfully."),
    market_fresh_count: "Select the plugins you want to use and press Enter to install or view \
                         details.",
    section_loading_title: msg!("Loading {0} plugins."),
    section_loading_count: "Local plugin functionality is already available.",
    section_empty_count: "This section loaded successfully.",
    section_failed_title: msg!("{0} unavailable."),
    section_failed_count: "Local plugin functionality is still available.",
    add_title: "Add a marketplace from a Git repo or local root.",
    add_count: "Enter a source to make its plugins available in this menu.",
    search: "Type to search plugins",
    none_all: "No marketplace plugins available",
    none_all_description: "No plugins are available in the discovered marketplaces",
    none_installed: "No installed plugins",
    none_curated: "No OpenAI Curated plugins available",
    none_market: "No plugins available in this marketplace",
    none_workspace: "No workspace plugins available",
    none_workspace_description: "No workspace directory plugins are available",
    none_shared: "No shared plugins available",
    none_shared_description: "No plugins have been shared with you",
    row_loading: msg!("Loading {0} plugins..."),
    row_unavailable: msg!("{0} unavailable"),
    loading_curated: "This updates when OpenAI Curated plugins finish loading",
    loading_workspace: "This updates when workspace plugins finish loading",
    loading_shared: "This updates when shared plugins finish loading",
    add_row: "Add marketplace",
    add_row_description: "Enter owner/repo, a Git URL, or a local marketplace path",
    add_row_selected: "Press Enter to enter a marketplace source",
    status_disabled: "Disabled",
    status_admin: "Admin assigned",
    status_installed: "Installed",
    status_not_installable: "Not installable",
    status_available: "Available",
    dot: " \u{b7} ",
    selected: msg!("{0}   {1}"),
    selected_toggle: msg!("Tab to {0}; Enter view details"),
    selected_toggle_only: msg!("Tab to {0}"),
    word_enable: "enable",
    word_disable: "disable",
    selected_view: "Press Enter to view plugin details",
    selected_unavailable: "Plugin details are unavailable",
    selected_install: "Press Enter to install or view plugin details",
    selected_remote: "Remote plugin details are not available yet",
    unavailable_reason: "plugin details are unavailable",
    mark_unavailable: "[-] ",
    mark_blocked: "[!] ",
    hint_upgrade: msg!("{0} upgrade"),
    hint_remove: msg!("{0} remove"),
    hint_toggle: "tab toggle",
    hint_tabs: "\u{2190}/\u{2192} tabs",
    hint_details: "enter details",
    hint_close: "esc close",
    hint_select_close: "enter select \u{b7} esc close",
    hint_join: " \u{b7} ",
    error_next: msg!("{0} {1}"),
    sharing_disabled: "Enable plugin sharing for this Codex session to load shared plugins",
    next_chatgpt_auth: "Sign in with ChatGPT auth; API key auth cannot load remote plugin \
                        catalogs",
    next_sign_in: "Sign in to ChatGPT, then try loading this section again",
    next_enable: "Ask a workspace admin to enable Codex plugins or plugin sharing",
    next_switch: "Switch to the matching workspace or ask the sharer for access",
    next_check: "Check that you are signed in to the correct workspace and still have access",
    next_update: "Update Codex, then try opening the shared plugin again",
    next_later: "Try again later; local plugin functionality is still available",
    next_admin: "Ask a workspace admin to confirm plugin access",
    next_sharer: "Ask the sharer or a workspace admin to confirm plugin access",
    loading_head: "Loading available plugins...",
    loading_note: "This updates when the marketplace list is ready.",
    loading_row: "Loading plugins...",
    loading_row_description: "This updates when the marketplace list is ready",
    failed_subtitle: "Failed to load plugins.",
    failed_row: "Plugin marketplace unavailable",
    add_loading: "Adding marketplace...",
    add_loading_description: "This updates when marketplace installation completes",
    add_failed_subtitle: "Failed to add marketplace.",
    add_failed_row: "Marketplace add failed",
    add_failed_description: "Failed to add marketplace from the provided source",
    retry: "Try again",
    retry_add_description: "Enter a marketplace source",
    retry_remove_description: "Review the confirmation prompt again",
    remove_confirm_head: msg!("Remove {0} marketplace?"),
    remove_confirm_note: "This removes the configured marketplace from Codex.",
    remove_row: "Remove marketplace",
    remove_row_description: "Remove this marketplace from the available plugin list",
    remove_keep_description: "Keep this marketplace installed",
    remove_loading_subtitle: msg!("Removing {0}..."),
    remove_loading_row: "Removing marketplace...",
    remove_loading_description: "This updates when marketplace removal completes",
    remove_failed_subtitle: "Failed to remove marketplace.",
    remove_failed_row: "Marketplace removal failed",
    remove_failed_description: "Failed to remove the selected marketplace",
    upgrade_loading_one: msg!("Upgrading {0} marketplace..."),
    upgrade_loading_all: "Upgrading marketplaces...",
    upgrade_loading_description: "This updates when marketplace upgrade completes",
    detail_loading: msg!("Loading details for {0}..."),
    detail_loading_row: "Loading plugin details...",
    detail_loading_description: "This updates when plugin details load",
    detail_failed_subtitle: "Failed to load plugin details.",
    detail_failed_row: "Plugin detail unavailable",
    install_loading_subtitle: msg!("Installing {0}..."),
    install_loading_row: "Installing plugin...",
    install_loading_description: "This updates when plugin installation completes",
    uninstall_loading_subtitle: msg!("Uninstalling {0}..."),
    uninstall_loading_row: "Uninstalling plugin...",
    uninstall_loading_description: "This updates when the plugin removal completes",
    detail_head: msg!("{0} \u{b7} {1} \u{b7} {2}"),
    detail_disabled_by_admin: "Disabled by admin",
    detail_installed_by_admin: "Installed by admin",
    detail_enabled_by_admin: "Enabled by Admin",
    detail_installed: "Installed",
    detail_disabled: "Disabled",
    detail_not_installable: "Not installable",
    detail_can_install: "Can be installed",
    disclosure_start: "Data shared with this app is subject to the app's ",
    disclosure_terms: "terms of service",
    disclosure_and: " and ",
    disclosure_privacy: "privacy policy",
    disclosure_stop: ". ",
    disclosure_link: "Learn more",
    disclosure_end: ".",
    back: "Back to plugins",
    back_description: "Return to the plugin list",
    admin_installed: "Installed by admin",
    admin_installed_description: "This plugin is installed by your workspace admin",
    uninstall: "Uninstall plugin",
    uninstall_description: "Remove this plugin now",
    uninstall_missing: "This remote plugin did not provide an uninstall identity",
    install: "Install plugin",
    install_description: "Install this plugin now",
    install_disabled: "This plugin is disabled by your workspace admin",
    install_not_available: "This plugin is not installable from this marketplace",
    install_no_location: "This plugin did not provide an install location",
    meta_source: "Source",
    meta_auth: "Auth",
    meta_version: "Version",
    meta_sharing: "Sharing",
    meta_skills: "Skills",
    meta_hooks: "Hooks",
    meta_apps: "Apps",
    meta_mcp: "MCP Servers",
    source_local: "Local",
    source_git: msg!("Git \u{b7} {0}"),
    source_git_ref: msg!("Git \u{b7} {0}@{1}"),
    source_npm: msg!("npm \u{b7} {0}"),
    source_npm_version: msg!("npm \u{b7} {0}@{1}"),
    source_remote: msg!("Remote \u{b7} {0}"),
    auth_on_install: "Auth on install",
    auth_on_use: "Auth on use",
    version_local: msg!("local {0}"),
    version_remote: msg!("remote {0}"),
    share_listed: "Listed",
    share_unlisted: "Workspace link",
    share_private: "Private",
    share_creator_account: msg!("creator {0} ({1})"),
    share_creator: msg!("creator {0}"),
    share_account: msg!("creator account {0}"),
    share_no_principals: "No explicit principals",
    share_one_principal: msg!("1 principal: {0}"),
    share_principals: msg!("{0} principals"),
    share_remote_id: msg!("Remote ID {0}"),
    none_skills: "No plugin skills",
    none_hooks: "No plugin hooks",
    none_apps: "No plugin apps",
    none_mcp: "No plugin MCP servers",
    list_join: ", ",
    hook: msg!("{0} ({1})"),
    auth_installed: msg!("{0} plugin installed."),
    auth_step: msg!("App setup {0}/{1}: {2}"),
    auth_already: "Already installed in this session.",
    auth_needed: "Install the required Apps in ChatGPT to continue:",
    auth_install: "Install on ChatGPT",
    auth_manage: "Manage on ChatGPT",
    auth_open_description: "Open the ChatGPT app management page",
    auth_open_selected: "Open the app page in your browser",
    auth_no_link: "ChatGPT apps link unavailable",
    auth_no_link_description: "This app did not provide an install/manage URL",
    auth_continue: "Continue",
    auth_continue_description: "This app is already installed",
    auth_continue_selected: "Advance to the next app",
    auth_done: "I've installed it",
    auth_done_description: "Trust your confirmation and continue to the next app",
    auth_done_selected: "Continue without waiting for refresh to complete",
    auth_skip: "Skip remaining app setup",
    auth_skip_description: "Stop this follow-up flow for this plugin",
    auth_skip_selected: "Abandon remaining required app setup",
    prompt_title: "Add marketplace",
    prompt_context: "Examples: owner/repo, git URL, ./marketplace",
    prompt_placeholder: "owner/repo, git URL, or local marketplace path",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The words of a failed ask: what the server or the decoder said.
pub fn failure(error: &RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => {
            text.clone()
        }
        RpcError::Mismatch => String::new(),
        RpcError::Provider(error) => error.to_string(),
    }
}

/// The words of what the plugin browser said in the history (`Say`); the oracle's English stands
/// until the catalog has them.
pub fn note(say: &Say) -> String {
    let noun = |count: usize| {
        if count == 1 {
            "marketplace"
        } else {
            "marketplaces"
        }
    };
    let both = |message: String, hint: String| format!("{message}{INLINE_GAP}{hint}");
    match say {
        Say::Disabled => both(
            "Plugins are disabled.".to_string(),
            "Enable the plugins feature to use /plugins.".to_string(),
        ),
        Say::Installed { name, apps } if apps.is_empty() => both(
            format!("Installed {name} plugin."),
            "No additional app authentication is required.".to_string(),
        ),
        Say::Installed { name, apps } => both(
            format!("Installed {name} plugin."),
            format!(
                "{} app(s) still need authentication: {}",
                apps.len(),
                apps.join(", ")
            ),
        ),
        Say::Uninstalled { name } => both(
            format!("Uninstalled {name} plugin."),
            "Bundled apps remain installed.".to_string(),
        ),
        Say::Skipped { name } => both(
            format!("Skipped remaining app setup for {name} plugin."),
            "The plugin may not be usable until required apps are installed.".to_string(),
        ),
        Say::Completed { name } => both(
            format!("Completed app setup flow for {name} plugin."),
            "You can now continue managing plugins from /plugins.".to_string(),
        ),
        Say::EnableFailed { id, error } => {
            format!(
                "Failed to update plugin config for {id}: {}",
                failure(error)
            )
        }
        Say::PageFailed { url, error } => format!("Failed to open browser for {url}: {error}"),
        Say::MarketAdded {
            name,
            root,
            already,
        } => both(
            if *already {
                format!("Marketplace {name} is already added.")
            } else {
                format!("Added marketplace {name}.")
            },
            format!("Marketplace root: {root}"),
        ),
        Say::MarketRemoved { label, name, root } => both(
            format!("Removed marketplace {label}."),
            match root {
                Some(root) => format!("Marketplace root: {root}"),
                None => format!("Removed marketplace config for {name}."),
            },
        ),
        Say::NothingToUpgrade => both(
            "No configured Git marketplaces to upgrade.".to_string(),
            "Only configured Git marketplaces can be upgraded.".to_string(),
        ),
        Say::UpToDate { selected } => both(
            match selected.as_slice() {
                [one] => format!("Marketplace {one} is already up to date."),
                many => format!(
                    "Checked {} marketplaces; all are already up to date.",
                    many.len()
                ),
            },
            format!("Checked: {}", selected.join(", ")),
        ),
        Say::Upgraded { roots } => both(
            format!("Upgraded {} {}.", roots.len(), noun(roots.len())),
            format!("Updated roots: {}", roots.join(", ")),
        ),
        Say::UpgradeFailed(errors) => format!(
            "Failed to upgrade {} {}: {}",
            errors.len(),
            noun(errors.len()),
            errors
                .iter()
                .map(|(name, message)| format!("{name}: {message}"))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Say::UpgradeError(error) => failure(error),
    }
}
