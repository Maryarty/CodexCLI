//! The plugin RPCs of the feature (oracle `app/background_requests.rs`): what is asked, how it
//! goes out (`wire_request`) and what comes back (`decode`). Data only; the state is
//! `state.rs`.

use codex_app_server_protocol as wire;
use codex_app_server_protocol::{ClientRequest, RequestId};

use super::catalog::{Catalog, Detail, Location, Part, Target};

/// An app a plugin needs signed in (`AppSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthApp {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
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
    Uninstall {
        id: String,
    },
    Add {
        source: String,
    },
    Remove {
        name: String,
    },
    Upgrade {
        name: Option<String>,
    },
    /// `config/value/write` of `plugins.<id>`.
    Enable {
        id: String,
        enabled: bool,
    },
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

/// The wire request of `wanted`.
pub fn wire_request(wanted: &Wanted, id: RequestId) -> ClientRequest {
    let path =
        |path: &str| serde_json::from_value(serde_json::Value::String(path.to_string())).ok();
    let list = |kinds: Option<Vec<wire::PluginListMarketplaceKind>>| wire::PluginListParams {
        cwds: Some(wanted.cwd.as_str())
            .filter(|cwd| !cwd.is_empty())
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
                    install_attempt_id: Some(
                        ["plugin-install-", &wanted.request.to_string()].concat(),
                    ),
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
        Want::Enable {
            id: plugin,
            enabled,
        } => ClientRequest::ConfigValueWrite {
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
        Want::List | Want::Section(_) => Found::Catalog(Catalog::of(
            parse::<wire::PluginListResponse>(result)?,
            home,
        )),
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
