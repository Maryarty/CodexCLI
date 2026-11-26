//! `import` (S06, import from other agents): the core half of the screen's
//! directory. The flow of the oracle's `external_agent_config_migration`
//! (`flow.rs`: detect every source, pick one when several have items, review
//! the items, import) as a model changed only by `apply`: the phase says what
//! the screen is for, the requests the shell must make are derived from it
//! (`Model::request`), and an answer carries the request it answers so a stale
//! one is dropped. The view half is `src/pages/startup/import/`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ExternalAgentConfigImportCompletedNotification as Completed;
use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use serde::Deserialize;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::SeedError;
use crate::threads::ThreadFx;

pub mod detect;
pub mod item;

pub use detect::{Detected, Source};
pub use item::{Group, GroupKind, Kind as ItemKind, Marketplace, Places};

pub const NAME: &str = "import";
pub const ID: &str = "S06";
pub const KIND: Kind = Kind::Route;

/// Where the flow is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Nothing asked yet.
    #[default]
    Idle,
    /// The sources are being asked (`Request::Detect`).
    Detecting,
    /// Several sources have items: one is to be chosen.
    Source,
    /// The items of the source are reviewed.
    Review,
    /// The selection is being imported (`Request::Import`).
    Importing,
    /// The flow ended: see `Model::outcome`.
    Done,
}

/// How the flow ended (oracle: `ExternalAgentConfigMigrationFlowOutcome` and
/// the error the flow returns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The import started: what the notice for the history tells.
    Started(Started),
    NoItems,
    Cancelled,
    /// An error to tell the user.
    Failed(Failure),
}

/// What an import that started was asked to bring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Started {
    /// The items chosen.
    pub items: Vec<Sel>,
    /// How many items stay for another `/import`.
    pub remaining: usize,
}

/// Why the flow ended with an error (oracle: the errors of `flow.rs` and
/// `app_server_session.rs`); the view tells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// An import of an earlier `/import` still runs.
    InProgress,
    /// The source chosen is not among the detected ones.
    SourceGone,
    /// No source could be asked: what each said.
    Detect(Vec<(Source, String)>),
}

/// What the server's notice said an import finished with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    /// The results by item type, in the order of the notice.
    pub by_type: Vec<TypeResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeResult {
    pub kind: ItemKind,
    pub imported: usize,
    pub failed: usize,
}

impl Finished {
    pub fn imported(&self) -> usize {
        self.by_type.iter().map(|result| result.imported).sum()
    }

    pub fn failed(&self) -> usize {
        self.by_type.iter().map(|result| result.failed).sum()
    }
}

/// An item and whether it is to be imported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sel {
    pub item: Item,
    pub enabled: bool,
}

/// How a group of the summary is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    Some,
    All,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub phase: Phase,
    /// Identifies the flow opened by the Startup Page, across detection and import.
    generation: u64,
    /// The generated ask this phase is waiting on.
    request: Option<u64>,
    /// The directory of the project (`None`: the session's).
    cwd: Option<String>,
    pub places: Places,
    /// The sources with items.
    pub detected: Vec<Detected>,
    pub source: Option<Source>,
    pub items: Vec<Sel>,
    pub groups: Vec<Group>,
    /// Counts the reviews opened: a view's `Nav` is for the review it opened on.
    pub review_id: u64,
    /// What the app server said when the last import failed, told above the list.
    pub error: Option<String>,
    pub outcome: Option<Outcome>,
    /// An import started and has not completed yet.
    pub running: bool,
    /// The id the server gave the import that runs: only its completion is ours.
    pub import_id: Option<String>,
    /// What the last completed import finished with.
    pub finished: Option<Finished>,
}

/// What the shell is asked to do for the phase (`Model::request`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Ask every source (`externalAgentConfig/detect`, `includeHome`, the
    /// project directory `cwd`).
    Detect { request: u64, cwd: Option<String> },
    /// Import `items` of `source` (`externalAgentConfig/import`).
    Import {
        request: u64,
        source: &'static str,
        items: Vec<Item>,
    },
}

/// What one source's detection answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub source: Source,
    pub result: Result<Vec<Item>, String>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// Every source was asked. `selected`: the items to start checked (the
    /// oracle's `selected_items`; `None`: all).
    Detected {
        request: u64,
        places: Places,
        found: Vec<Found>,
        selected: Option<Vec<usize>>,
    },
    /// The import request was answered.
    Imported {
        request: u64,
        result: Result<String, String>,
    },
    /// The server's notice that an import finished.
    Completed(Box<Completed>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/import`: ask the sources (`cwd`: the project directory).
    Open { cwd: Option<String> },
    /// The source with this id (`Source::id`) was chosen.
    Source(String),
    /// The entries checked (indexes of `items`, as the page's checklist holds them) are to be
    /// imported: the checks are the page's draft until this verb, the request records them.
    Proceed(Vec<usize>),
    /// Cancel the flow (the source prompt left, `ctrl+c`, `ctrl+d`, skip, escape).
    Cancel,
}

impl Model {
    pub(crate) fn waits_for(&self, request: u64) -> bool {
        self.request == Some(request)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The request the shell has to make for this phase.
    pub fn request(&self) -> Option<Request> {
        let request = self.request?;
        match self.phase {
            Phase::Detecting => Some(Request::Detect {
                request,
                cwd: self.cwd.clone(),
            }),
            Phase::Importing => Some(Request::Import {
                request,
                source: self.source?.id(),
                items: self.selected_items(),
            }),
            _ => None,
        }
    }

    pub fn selected_items(&self) -> Vec<Item> {
        self.items
            .iter()
            .filter(|sel| sel.enabled)
            .map(|sel| sel.item.clone())
            .collect()
    }

    /// The items checked at the start of a review: what the detection preselected, or what
    /// the last import asked for. The page's checklist starts from these.
    pub fn preselected(&self) -> Vec<bool> {
        self.items.iter().map(|sel| sel.enabled).collect()
    }

    /// The mark of a group of the summary over the page's `checked` items.
    pub fn mark(group: &Group, checked: &[bool]) -> Mark {
        let on = group
            .items
            .iter()
            .filter(|at| checked.get(**at).copied().unwrap_or(false))
            .count();
        match on {
            0 => Mark::None,
            n if n == group.items.len() => Mark::All,
            _ => Mark::Some,
        }
    }

    fn start(&mut self, cwd: Option<String>) {
        let (generation, running, finished) =
            (self.generation + 1, self.running, self.finished.take());
        let import_id = self.import_id.take();
        *self = Model {
            generation,
            running,
            finished,
            import_id,
            cwd,
            ..Model::default()
        };
        if self.running {
            self.end(Outcome::Failed(Failure::InProgress));
        } else {
            self.phase = Phase::Detecting;
        }
    }

    fn end(&mut self, outcome: Outcome) {
        self.request = None;
        self.phase = Phase::Done;
        self.outcome = Some(outcome);
    }

    /// The review of `items` of `source`; `selected`: which are checked.
    fn review(
        &mut self,
        source: Source,
        items: Vec<Item>,
        selected: &[Item],
        error: Option<String>,
    ) {
        self.request = None;
        self.groups = item::groups(&items);
        self.items = items
            .into_iter()
            .map(|item| Sel {
                enabled: selected.contains(&item),
                item,
            })
            .collect();
        self.source = Some(source);
        self.phase = Phase::Review;
        self.review_id += 1;
        self.error = error;
    }

    fn proceed(&mut self, checked: &[usize]) {
        for (at, sel) in self.items.iter_mut().enumerate() {
            sel.enabled = checked.contains(&at);
        }
        self.error = None;
        self.phase = Phase::Importing;
    }

    fn chosen(&mut self, source: Source) {
        let Some(found) = self.detected.iter().find(|found| found.source == source) else {
            return self.end(Outcome::Failed(Failure::SourceGone));
        };
        let items = found.items.clone();
        let selected = items.clone();
        self.review(source, items, &selected, None);
    }
}

/// The results of the server's notice, by item type.
fn finished(done: &Completed) -> Finished {
    Finished {
        by_type: done
            .item_type_results
            .iter()
            .map(|result| TypeResult {
                kind: ItemKind::of(result.item_type),
                imported: result.successes.len(),
                failed: result.failures.len(),
            })
            .collect(),
    }
}

/// A server notification the flow reads: the completion of the import it
/// started.
/// Only the one of the import this client started counts (a shared daemon
/// tells every client of every import: oracle
/// `consume_external_agent_config_import_completion`).
pub fn observe(model: &mut Model, notification: &ServerNotification, fx: &mut AppFx) {
    if let ServerNotification::ExternalAgentConfigImportCompleted(done) = notification
        && model.import_id.as_deref() == Some(done.import_id.as_str())
    {
        apply(model, Ev::Completed(Box::new(done.clone())), fx);
    }
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Detected {
            request,
            places,
            found,
            selected,
        } => {
            // A seeded event has no request to answer: only the idle model takes it.
            let answers = match model.phase {
                Phase::Detecting => model.request == Some(request),
                Phase::Idle => request == 0,
                _ => false,
            };
            if !answers {
                return;
            }
            model.request = None;
            let mut detection = detect::Detection::default();
            for Found { source, result } in found {
                match result {
                    Ok(items) => detection.record_items(source, items),
                    Err(error) => detection.record_error(source, &error),
                }
            }
            model.places = places;
            match detection.finish() {
                detect::Outcome::NoItems => model.end(Outcome::NoItems),
                detect::Outcome::Failed(errors) => {
                    model.end(Outcome::Failed(Failure::Detect(errors)))
                }
                detect::Outcome::Sources(sources) => {
                    model.detected = sources;
                    if let [only] = model.detected.as_slice() {
                        let (source, items) = (only.source, only.items.clone());
                        let checked: Vec<Item> = match &selected {
                            None => items.clone(),
                            Some(at) => {
                                at.iter().filter_map(|at| items.get(*at).cloned()).collect()
                            }
                        };
                        model.review(source, items, &checked, None);
                    } else {
                        model.phase = Phase::Source;
                    }
                }
            }
        }
        Ev::Imported { request, result } => {
            if model.phase != Phase::Importing || model.request != Some(request) {
                return;
            }
            match result {
                Ok(import_id) => {
                    model.import_id = Some(import_id);
                    let remaining = model
                        .items
                        .len()
                        .saturating_sub(model.selected_items().len());
                    model.running = true;
                    model.end(Outcome::Started(Started {
                        items: model
                            .items
                            .iter()
                            .filter(|sel| sel.enabled)
                            .cloned()
                            .collect(),
                        remaining,
                    }));
                }
                Err(error) => {
                    let (source, items) = (model.source, model.selected_items());
                    let all: Vec<Item> = model.items.iter().map(|sel| sel.item.clone()).collect();
                    if let Some(source) = source {
                        model.review(source, all, &items, Some(error));
                    }
                }
            }
        }
        Ev::Completed(done) => {
            model.finished = Some(finished(&done));
            model.running = false;
            model.import_id = None;
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let reviewing = model.phase == Phase::Review;
    match intent {
        Intent::Open { cwd } => {
            model.start(cwd);
            ask(model, fx);
        }
        Intent::Source(id) => {
            if model.phase == Phase::Source {
                match Source::of_id(&id) {
                    Some(source) => model.chosen(source),
                    None => model.end(Outcome::Failed(Failure::SourceGone)),
                }
            }
        }
        Intent::Cancel => {
            if matches!(
                model.phase,
                Phase::Detecting | Phase::Source | Phase::Review
            ) {
                // a detection still on its way answers a generation that is gone: dropped
                model.end(Outcome::Cancelled);
            }
        }
        _ if !reviewing => {}
        Intent::Proceed(checked) => {
            if checked.iter().any(|at| *at < model.items.len()) {
                model.proceed(&checked);
                ask(model, fx);
            }
        }
    }
}

pub(crate) fn connection_lost(model: &mut Model, replaced: Option<&mut AppFx>) {
    match model.phase {
        Phase::Detecting => {
            if let Some(fx) = replaced {
                ask(model, fx);
            } else {
                model.end(Outcome::Cancelled);
            }
        }
        Phase::Importing => {
            let items = model.selected_items();
            let all = model.items.iter().map(|sel| sel.item.clone()).collect();
            if let Some(source) = model.source {
                // The old write may have reached the server. It is only sent again by the user.
                model.review(
                    source,
                    all,
                    &items,
                    Some(crate::app::CONNECTION_LOST.to_owned()),
                );
            }
        }
        _ => {}
    }
}

/// Asks the shell for what the phase waits on (`Model::request`).
fn ask(model: &mut Model, fx: &mut AppFx) {
    if !matches!(model.phase, Phase::Detecting | Phase::Importing) {
        return;
    }
    model.request = Some(fx.host_request_number());
    if let Some(ask) = model.request() {
        fx.host_command(crate::app::Command::Import { ask });
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
/// `{"detected":{"places":{..},"found":[{"source":"claude-code","items":[..]},
/// {"source":"cursor","error":".."}],"selected":[0,1]}}`, the items as the app
/// server sends them (`ExternalAgentConfigMigrationItem`), and
/// `{"completed":{..}}` (`ExternalAgentConfigImportCompletedNotification`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Detected {
        #[serde(default)]
        places: Places,
        found: Vec<SeedFound>,
        #[serde(default)]
        selected: Option<Vec<usize>>,
    },
    Completed(Box<Completed>),
}

#[derive(Clone, Debug, Deserialize)]
struct SeedFound {
    source: Source,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    error: Option<String>,
}

/// A script's `screen` step for this directory (`fixtures::script`).
pub fn seed(value: serde_json::Value) -> Result<Ev, SeedError> {
    let seed: SeedEv =
        serde_json::from_value(value).map_err(|err| SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Detected {
            places,
            found,
            selected,
        } => Ev::Detected {
            request: 0,
            places,
            found: found
                .into_iter()
                .map(|one| Found {
                    source: one.source,
                    result: match one.error {
                        Some(error) => Err(error),
                        None => Ok(one.items),
                    },
                })
                .collect(),
            selected,
        },
        SeedEv::Completed(done) => Ev::Completed(done),
    })
}
