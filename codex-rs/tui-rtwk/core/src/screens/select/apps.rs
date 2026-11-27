//! `/apps`: the directory of apps (connectors) the picker lists. The cache
//! (loading, ready, failed), the one request in flight and the refresh a notification or a second `/apps`
//! queued behind it are data here; the words are the view's (ADR-010).
//! Oracle: `chatwidget/connectors.rs` (`ConnectorsCacheState`,
//! `on_connectors_loaded`, `refresh_connector_directory_after_notification`,
//! `add_connectors_output`), `app/background_requests.rs`
//! (`fetch_connectors_list`).

use codex_app_server_protocol::AppInfo;
use codex_app_server_protocol::AppsListParams;
use codex_app_server_protocol::AppsListResponse;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

/// The id of the failure menu's `Retry` row.
pub const RETRY: &str = "retry";

/// What an app is for the user (`connector_status_label`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Installed,
    /// Installed and switched off.
    Disabled,
    /// `Can be installed`.
    Installable,
}

/// An app of the directory (the part of `AppInfo` the picker reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: String,
    /// `connector_display_label`: the app's name.
    pub name: String,
    pub description: Option<String>,
    pub install_url: Option<String>,
    /// Installed (`is_accessible`).
    pub installed: bool,
    /// Enabled in the config (`is_enabled`).
    pub enabled: bool,
}

/// The page of an app the link dialog shows (`AppLinkViewParams`): copied when the app is picked,
/// as the oracle's view keeps its own; the dialog is the sheet's (`sheets/select/app_link.rs`), so
/// the switch of the dialog changes the dialog's copy at once (`toggle_enabled`) and the directory
/// follows the saved write.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Link {
    pub id: String,
    /// The app's name (the dialog's title).
    pub name: String,
    pub description: Option<String>,
    /// The page the dialog opens in the browser.
    pub url: String,
    pub installed: bool,
    pub enabled: bool,
}

impl App {
    /// The link dialog of the app, when it has a page (`install_url`).
    pub fn link(&self) -> Option<Link> {
        Some(Link {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.about().map(str::to_owned),
            url: self.install_url.clone()?,
            installed: self.installed,
            enabled: self.enabled,
        })
    }

    pub fn of(info: AppInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            description: info.description,
            install_url: info.install_url,
            installed: info.is_accessible,
            enabled: info.is_enabled,
        }
    }

    pub fn status(&self) -> Status {
        match (self.installed, self.enabled) {
            (true, true) => Status::Installed,
            (true, false) => Status::Disabled,
            (false, _) => Status::Installable,
        }
    }

    /// `connector_description`: the description unless it is blank.
    pub fn about(&self) -> Option<&str> {
        self.description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }
}

/// The last action of an owned Apps dialog. A bare dismissal has no choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalChoice {
    Unlinked(String),
    RefreshAfterVisit,
}

/// An action taken while the directory or its link page remains open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalAction {
    Retry,
    Visit(String),
    Enable { id: String, enabled: bool },
}

/// The directory and thread whose config the switch update was sent to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalWriteOwner {
    pub thread: crate::ids::ThreadId,
    pub scope: u64,
}

/// What the directory holds (`ConnectorsCacheState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Cache {
    #[default]
    Uninitialized,
    Loading,
    Ready(Vec<App>),
    Failed(String),
}

/// The directory and its refreshes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Apps {
    pub cache: Cache,
    /// The `app/list` request out; its answer must carry its number.
    pub pending: Option<Request>,
    /// A forced refresh was asked while one ran: it follows.
    refetch_after: bool,
    /// A notification's directory that arrived while one ran: the final
    /// answer is compared with it.
    notified: Option<Vec<App>>,
    /// The notification's recovery already failed once.
    retried: bool,
    issued: u64,
    /// How many times the account or the scope changed: a sheet that shows the directory ends
    /// when it differs from the one it opened under.
    scope: u64,
}

impl Apps {
    /// `begin_connectors_refresh`: the request to send (`force`: bypass the
    /// server's caches), or none while one runs (a forced ask waits for it).
    pub fn refresh(&mut self, force: bool) -> Option<Request> {
        if self.pending.is_some() {
            if force {
                self.refetch_after = true;
            }
            return None;
        }
        self.issued += 1;
        let request = Request {
            request: self.issued,
            force,
        };
        self.pending = Some(request);
        if !matches!(self.cache, Cache::Ready(_)) {
            self.cache = Cache::Loading;
        }
        Some(request)
    }

    /// The apps the picker shows now: the cache once it is ready.
    pub fn ready(&self) -> Option<&[App]> {
        match &self.cache {
            Cache::Ready(apps) => Some(apps),
            _ => None,
        }
    }

    /// The app `id` of the directory held, once it is ready.
    pub fn app(&self, id: &str) -> Option<&App> {
        self.ready()?.iter().find(|app| app.id == id)
    }

    /// `/apps` was asked for: the refresh it starts (forced unless one is
    /// running and the cache is not ready; `add_connectors_output`).
    pub fn open(&mut self) -> Option<Request> {
        let force = self.pending.is_none() || matches!(self.cache, Cache::Ready(_));
        self.refresh(force)
    }

    /// `refresh_connector_directory_after_notification`: the app server says
    /// the directory changed. Its contents are never taken as they are; a
    /// directory that differs from the held one is read again.
    pub fn notified(&mut self, apps: Vec<App>) -> Option<Request> {
        if self.ready() == Some(apps.as_slice()) {
            return None;
        }
        if self.pending.is_some() {
            self.notified = Some(apps);
            return None;
        }
        self.refresh(false)
    }

    /// `on_connectors_loaded` for the final answer of request `request`: the
    /// cache the picker shows, and the refresh that follows, if any.
    pub fn loaded(&mut self, request: u64, result: Result<Vec<App>, String>) -> Option<Request> {
        if self.pending.map(|pending| pending.request) != Some(request) {
            return None;
        }
        self.pending = None;
        let was_retry = std::mem::take(&mut self.retried);
        let revalidate = self.notified.take().is_some_and(|notified| match &result {
            Ok(apps) => *apps != notified,
            Err(_) if !was_retry => {
                self.retried = true;
                true
            }
            Err(_) => false,
        });
        let forced = std::mem::take(&mut self.refetch_after);
        match result {
            Ok(mut apps) => {
                // A kept switch wins over the server's: the user flipped it.
                if let Cache::Ready(held) = &self.cache {
                    for app in &mut apps {
                        if let Some(was) = held.iter().find(|held| held.id == app.id) {
                            app.enabled = was.enabled;
                        }
                    }
                }
                self.cache = Cache::Ready(apps);
            }
            // A failed refresh keeps the snapshot held.
            Err(error) => {
                if !matches!(self.cache, Cache::Ready(_)) {
                    self.cache = Cache::Failed(error);
                }
            }
        }
        if forced || revalidate {
            self.refresh(forced)
        } else {
            None
        }
    }

    /// The account or the scope changed: nothing held is trusted
    /// (`invalidate_connector_scope`). The numbers stay, so an answer of
    /// before is dropped.
    pub fn invalidate(&mut self) {
        *self = Self {
            issued: self.issued,
            scope: self.scope + 1,
            ..Self::default()
        };
    }

    /// How many times the account or the scope changed (`invalidate`).
    pub fn scope(&self) -> u64 {
        self.scope
    }

    /// `update_connector_enabled`: the app with `id` was switched.
    pub fn switched(&mut self, id: &str, enabled: bool) {
        if let Cache::Ready(apps) = &mut self.cache
            && let Some(app) = apps.iter_mut().find(|app| app.id == id)
        {
            app.enabled = enabled;
        }
    }
}

/// An `app/list` to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub request: u64,
    pub force: bool,
}

/// `app/list` for the displayed thread.
pub fn list_request(id: RequestId, force: bool, thread: Option<String>) -> ClientRequest {
    ClientRequest::AppsList {
        request_id: id,
        params: AppsListParams {
            cursor: None,
            limit: None,
            thread_id: thread,
            force_refetch: force,
        },
    }
}

/// The apps of an `app/list` answer.
pub fn apps(result: serde_json::Value) -> Result<Vec<App>, String> {
    serde_json::from_value::<AppsListResponse>(result)
        .map(|response| response.data.into_iter().map(App::of).collect())
        .map_err(|error| error.to_string())
}

/// What the menu shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    /// `Loading installed and available apps...`.
    Loading,
    /// `Failed to load apps.` and a Retry row.
    Failed,
    /// The directory: counts and rows.
    Apps {
        installed: usize,
        total: usize,
        rows: Vec<App>,
    },
}

impl Apps {
    pub fn page(&self) -> Page {
        match &self.cache {
            Cache::Ready(apps) => Page::Apps {
                installed: apps.iter().filter(|app| app.installed).count(),
                total: apps.len(),
                rows: apps.clone(),
            },
            Cache::Failed(_) => Page::Failed,
            Cache::Uninitialized | Cache::Loading => Page::Loading,
        }
    }
}
