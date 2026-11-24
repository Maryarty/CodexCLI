//! The plugin RPCs of the feature (oracle `app/background_requests.rs`): what is asked and what
//! comes back, as one `rpc_list!`. Data only; the state is `state.rs`.

use codex_app_server_protocol as wire;

use super::catalog::{Catalog, Detail, Location, Part, Target};

/// An app a plugin needs signed in (`AppSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthApp {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

/// A request to send: its number is what the answer is told back under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub request: u64,
    pub want: Want,
    /// The directory the request is for (`plugin/list` is asked in it).
    pub cwd: String,
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
    Added {
        name: String,
        root: String,
        already: bool,
    },
    Removed {
        name: String,
        root: Option<String>,
    },
    Upgraded {
        selected: Vec<String>,
        roots: Vec<String>,
        errors: Vec<(String, String)>,
    },
    Written,
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

/// A path the protocol types as an absolute one (`None` for one that is not).
fn absolute<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(path.to_string())).ok()
}

/// The marketplace a plugin is in: its file when local, its name when remote.
fn location<T: serde::de::DeserializeOwned>(target: &Target) -> (Option<T>, Option<String>) {
    match &target.location {
        Location::Local { path } => (absolute(path), None),
        Location::Remote { marketplace } => (None, Some(marketplace.clone())),
    }
}

/// `plugin/list` in the directory the request is for, of the given kinds (none: the local ones).
fn list(
    wanted: &Wanted,
    kinds: Option<Vec<wire::PluginListMarketplaceKind>>,
) -> wire::PluginListParams {
    wire::PluginListParams {
        cwds: Some(wanted.cwd.as_str())
            .filter(|cwd| !cwd.is_empty())
            .and_then(|cwd| serde_json::from_value(serde_json::json!([cwd])).ok()),
        marketplace_kinds: kinds,
        force_refetch: false,
    }
}

/// A path of an answer as the text the models keep.
fn root(root: impl AsRef<std::path::Path>) -> String {
    root.as_ref().to_string_lossy().into_owned()
}

crate::rpc_list! {
    /// What the app asks the backend for.
    pub enum Want => Found, request wanted: &Wanted, reply wanted: &Wanted;
    /// `plugin/list`: the local marketplaces.
    List, Self::List
        => PluginList(list(wanted, None))
        -> wire::PluginListResponse, |response| Found::Catalog(Catalog::of(response, wanted.home.as_deref()));
    /// `plugin/list` of the marketplaces of one remote section.
    Section(Part), Self::Section(part)
        => PluginList(list(wanted, Some(vec![match part {
            Part::Vertical => wire::PluginListMarketplaceKind::Vertical,
            Part::Workspace => wire::PluginListMarketplaceKind::WorkspaceDirectory,
            Part::SharedWithMe => wire::PluginListMarketplaceKind::SharedWithMe,
        }])))
        -> wire::PluginListResponse, |response| Found::Catalog(Catalog::of(response, wanted.home.as_deref()));
    /// `plugin/read`.
    Read(Target), Self::Read(plugin)
        => PluginRead({
            let (marketplace_path, remote_marketplace_name) = location(plugin);
            wire::PluginReadParams {
                marketplace_path,
                remote_marketplace_name,
                plugin_name: plugin.plugin.clone(),
            }
        })
        -> wire::PluginReadResponse, |response| Found::Detail(Box::new(Detail::of(response.plugin, wanted.home.as_deref())));
    /// `plugin/install`.
    Install(Target), Self::Install(plugin)
        => PluginInstall({
            let (marketplace_path, remote_marketplace_name) = location(plugin);
            wire::PluginInstallParams {
                marketplace_path,
                remote_marketplace_name,
                install_attempt_id: Some(["plugin-install-", &wanted.request.to_string()].concat()),
                plugin_name: plugin.plugin.clone(),
            }
        })
        -> wire::PluginInstallResponse, |response| Found::Installed(
            response
                .apps_needing_auth
                .into_iter()
                .map(|app| AuthApp {
                    id: app.id,
                    name: app.name,
                    url: app.install_url,
                })
                .collect(),
        );
    /// `plugin/uninstall`.
    Uninstall { id: String }, Self::Uninstall { id }
        => PluginUninstall(wire::PluginUninstallParams { plugin_id: id.clone() })
        -> wire::PluginUninstallResponse, |_| Found::Uninstalled;
    /// `marketplace/add`.
    Add { source: String }, Self::Add { source }
        => MarketplaceAdd(wire::MarketplaceAddParams {
            source: source.clone(),
            ref_name: None,
            sparse_paths: None,
        })
        -> wire::MarketplaceAddResponse, |added| Found::Added {
            name: added.marketplace_name,
            root: root(added.installed_root),
            already: added.already_added,
        };
    /// `marketplace/remove`.
    Remove { name: String }, Self::Remove { name }
        => MarketplaceRemove(wire::MarketplaceRemoveParams { marketplace_name: name.clone() })
        -> wire::MarketplaceRemoveResponse, |removed| Found::Removed {
            name: removed.marketplace_name,
            root: removed.installed_root.map(root),
        };
    /// `marketplace/upgrade`.
    Upgrade { name: Option<String> }, Self::Upgrade { name }
        => MarketplaceUpgrade(wire::MarketplaceUpgradeParams { marketplace_name: name.clone() })
        -> wire::MarketplaceUpgradeResponse, |upgraded| Found::Upgraded {
            selected: upgraded.selected_marketplaces,
            roots: upgraded.upgraded_roots.into_iter().map(root).collect(),
            errors: upgraded
                .errors
                .into_iter()
                .map(|error| (error.marketplace_name, error.message))
                .collect(),
        };
    /// `config/value/write` of `plugins.<id>`.
    Enable { id: String, enabled: bool }, Self::Enable { id, enabled }
        => ConfigValueWrite(wire::ConfigValueWriteParams {
            key_path: ["plugins.", id].concat(),
            value: serde_json::json!({ "enabled": enabled }),
            merge_strategy: wire::MergeStrategy::Upsert,
            file_path: None,
            expected_version: None,
        })
        -> serde_json::Value, |_| Found::Written;
}
