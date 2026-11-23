//! The `/model` family of the selection dialog: the catalog the pickers read,
//! the menus of the stack (data only, ADR-010: the words are the view's) and
//! the rules that turn a choice into the next menu or a final pick.
//! Oracle: `chatwidget/model_popups.rs`, `model_popup_state.rs`,
//! `session_model_selection.rs`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::RequestId;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;

pub type Effort = ReasoningEffort;

/// How many concurrent threads make an Ultra pick warn
/// (`ULTRA_REASONING_CONCURRENCY_WARNING_THRESHOLD`).
pub const ULTRA_WARNING_THREADS: usize = 8;

/// The id of the quick menu's "All models" row.
pub const ALL_MODELS: &str = "all-models";
/// The id of the reasoning menu's "More reasoning…" row.
pub const MORE_REASONING: &str = "more-reasoning";
/// The ids of the plan scope prompt's rows.
pub const PLAN_ONLY: &str = "plan-only";
pub const ALL_MODES: &str = "all-modes";

/// One effort a model supports (`ReasoningEffortPreset`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EffortOption {
    pub effort: Effort,
    #[serde(default)]
    pub description: String,
}

/// A service tier a model offers (`ModelServiceTier`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Tier {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// A model of the catalog (`ModelPreset`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Preset {
    /// The wire slug.
    pub model: String,
    /// The name the pickers show.
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "yes")]
    pub show_in_picker: bool,
    #[serde(default)]
    pub default_effort: Effort,
    #[serde(default)]
    pub efforts: Vec<EffortOption>,
    /// `model_specialty`: `cyber` for the models the full-access warning is
    /// stronger for.
    #[serde(default)]
    pub specialty: Option<String>,
    /// The service tiers the model offers (`service_tiers`).
    #[serde(default)]
    pub service_tiers: Vec<Tier>,
    /// The tier the model runs on when none is configured.
    #[serde(default)]
    pub default_service_tier: Option<String>,
}

fn yes() -> bool {
    true
}

impl Preset {
    /// `model/list` answers a hidden flag, the picker reads its inverse.
    pub fn from_wire(model: &codex_app_server_protocol::Model) -> Self {
        Preset {
            model: model.model.clone(),
            name: model.display_name.clone(),
            description: model.description.clone(),
            show_in_picker: !model.hidden,
            default_effort: model.default_reasoning_effort.clone(),
            efforts: model
                .supported_reasoning_efforts
                .iter()
                .map(|option| EffortOption {
                    effort: option.reasoning_effort.clone(),
                    description: option.description.clone(),
                })
                .collect(),
            specialty: model.model_specialty.clone(),
            service_tiers: model
                .service_tiers
                .iter()
                .map(|tier| Tier {
                    id: tier.id.clone(),
                    name: tier.name.clone(),
                    description: tier.description.clone(),
                })
                .collect(),
            default_service_tier: model.default_service_tier.clone(),
        }
    }

    /// `codex-auto-*`: the quick modes the first menu lists.
    pub fn is_auto(&self) -> bool {
        self.model.starts_with("codex-auto-")
    }

    fn auto_order(&self) -> usize {
        match self.model.as_str() {
            "codex-auto-fast" => 0,
            "codex-auto-balanced" => 1,
            "codex-auto-thorough" => 2,
            _ => 3,
        }
    }

    /// Max or Ultra is the default or one of the efforts.
    fn needs_advanced(&self) -> bool {
        is_advanced(&self.default_effort) || self.efforts.iter().any(|o| is_advanced(&o.effort))
    }

    /// The effort an all-models row applies without a second menu: the only
    /// one the model supports, unless it is advanced.
    fn direct_effort(&self) -> Option<Effort> {
        match self.efforts.as_slice() {
            [] => Some(self.default_effort.clone()),
            [option] => Some(option.effort.clone()),
            _ => None,
        }
        .filter(|effort| !is_advanced(effort))
    }

    /// The efforts of the standard reasoning menu and the advanced ones, in
    /// the order the model lists them (`open_reasoning_popup`).
    fn split_efforts(&self) -> (Vec<Effort>, Vec<Effort>) {
        let mut all: Vec<Effort> = self.efforts.iter().map(|o| o.effort.clone()).collect();
        if all.is_empty() {
            all.push(self.default_effort.clone());
        }
        all.into_iter().partition(|effort| !is_advanced(effort))
    }

    fn effort_description(&self, effort: &Effort) -> String {
        self.efforts
            .iter()
            .find(|option| &option.effort == effort)
            .map(|option| option.description.clone())
            .unwrap_or_default()
    }
}

/// Max and Ultra cost more: they sit behind their own row.
pub fn is_advanced(effort: &Effort) -> bool {
    matches!(effort, Effort::Max | Effort::Ultra)
}

/// Whether the default provider is being written or was (oracle `ProviderWrite`,
/// `model_picker_unavailable`): the cached model list is then not trusted until the app
/// server and the TUI restart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// No provider write is known: the list is current.
    #[default]
    Settled,
    /// A write is in flight (`provider_write` is locked): try `/model` after it finishes.
    Saving,
    /// A write was committed, or its result is unknown: the list may be out of date.
    Changed,
}

/// How the `model/list` the pickers waited for was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The list came (it may be empty).
    Listed,
    /// The request failed.
    Failed,
}

/// Why a sheet of the family gave up (the sheet decides, because it alone knows whether the user
/// was still looking at it) and what the history says of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaveUp {
    /// `Loading models…` was on screen and the list could not be read.
    NotLoaded,
    /// A list with no choice was on screen (`reserve`: the Reserve-only one).
    Empty { reserve: bool },
}

/// The models the account can pick from, as the last `model/list` (or the
/// startup's) answered, and the request the next answer must belong to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    pub presets: Vec<Preset>,
    /// The provider write state; the pickers are blocked unless it is `Settled`.
    pub provider: Provider,
    /// The `model/list` the open picker waits for
    /// (`model_popup_request_id`): an answer of another request is dropped.
    pub pending: Option<u64>,
    /// The last answer of a request of the pickers, by the request it belongs to: a sheet that
    /// opened on `Loading models…` reads its own there (ADR-013: a fact under a request key).
    pub answered: Option<(u64, Answer)>,
    issued: u64,
}

impl Catalog {
    fn visible(&self) -> impl Iterator<Item = &Preset> {
        self.presets.iter().filter(|preset| preset.show_in_picker)
    }

    /// Why the pickers are blocked (`model_picker_unavailable`), if they are.
    pub fn unavailable(&self) -> Option<Provider> {
        (self.provider != Provider::Settled).then_some(self.provider)
    }

    /// A new request: older answers are no longer wanted.
    pub(super) fn request(&mut self) -> u64 {
        self.issued += 1;
        self.pending = Some(self.issued);
        self.issued
    }

    /// `MODEL_SPECIALTY_CYBER`: the catalog knows `model` as a cyber model.
    pub fn is_cyber(&self, model: &str) -> bool {
        self.find(model).is_some_and(|preset| {
            preset.specialty.as_deref()
                == Some(codex_protocol::openai_models::MODEL_SPECIALTY_CYBER)
        })
    }

    fn find(&self, model: &str) -> Option<&Preset> {
        self.presets.iter().find(|preset| preset.model == model)
    }
}

/// `model/list` with the hidden models left out by the picker itself.
pub fn list_request(request_id: RequestId) -> ClientRequest {
    ClientRequest::ModelList {
        request_id,
        params: ModelListParams { include_hidden: Some(true), ..Default::default() },
    }
}

/// The presets a `model/list` answer holds.
pub fn presets(result: serde_json::Value) -> Result<Vec<Preset>, String> {
    serde_json::from_value::<ModelListResponse>(result)
        .map(|response| response.data.iter().map(Preset::from_wire).collect())
        .map_err(|err| err.to_string())
}

/// Where the Plan reasoning default the scope prompt replaces comes from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanSource {
    /// The user's own override.
    Override(Effort),
    /// The built-in default (none: no reasoning).
    Default(Option<Effort>),
    Unknown,
}

/// The collaboration mode the thread is in when it is Plan
/// (`current_collaboration_mode` and `plan_mode_reasoning_effort`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PlanMode {
    pub model: String,
    pub effort: Option<Effort>,
    pub source: PlanSource,
}

/// `LUNA_RESERVE_MODEL` and `LUNA_MODEL` (`model_catalog.rs`): the temporary model that keeps
/// the thread going when ordinary usage ran out, and the normal model it stands for.
pub const RESERVE_MODEL: &str = "gpt-reserve";
const LUNA_MODEL: &str = "gpt-6-luna";

/// The thread runs on the Reserve and ordinary usage has not come back
/// (`restrict_model_picker_to_luna_reserve`): `/model` offers that one model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ReserveOnly {
    /// The normal model whose name and efforts the row borrows (the reserve quota's
    /// `normal_model_slug`); none reads the Luna model.
    #[serde(default)]
    pub normal_model: Option<String>,
}

/// What the thread had when the menu opened: the dispatcher reads it from
/// the thread's part and the menu keeps it (a snapshot that dies with the
/// menu, not a copy of the thread).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Current {
    pub model: String,
    /// The effective reasoning effort.
    #[serde(default)]
    pub effort: Option<Effort>,
    /// Set while collaboration modes are on and the mode is Plan.
    #[serde(default)]
    pub plan: Option<PlanMode>,
    /// `multi_agent_v2.max_concurrent_threads_per_session`.
    #[serde(default)]
    pub threads: Option<usize>,
    /// Set while the Reserve is the model and ordinary usage is not back.
    #[serde(default)]
    pub reserve: Option<ReserveOnly>,
}

impl Current {
    pub fn new(model: impl Into<String>) -> Self {
        Current { model: model.into(), effort: None, plan: None, threads: None, reserve: None }
    }

    /// What the chat's status facts know of the thread: its model, effective
    /// effort and the configured thread limit. The plan mode is not among
    /// them (the opener adds it when the thread model carries it).
    pub fn from_facts(facts: &crate::screens::chat::StatusFacts) -> Self {
        let effort = facts.effort.as_deref().and_then(|name| {
            serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
        });
        Current {
            effort,
            threads: facts.agent_threads,
            ..Current::new(facts.model.clone().unwrap_or_default())
        }
    }
}

/// What a final choice applies as (`AstraModelPickerAction` and the plan
/// scope prompt's two rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apply {
    /// The model and effort become the default.
    Selection,
    /// An Ultra effort (`ApplyAdvancedReasoning`).
    Advanced,
    /// The `s` key: this session only.
    Session,
    /// Plan scope: Plan mode only.
    PlanOnly,
    /// Plan scope: every mode.
    AllModes,
    /// The Reserve's effort (`UpdateLunaReserveReasoning`): the thread's effort for the task,
    /// no model and nothing saved.
    Reserve,
}

/// A model and effort the user settled on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub model: String,
    pub effort: Option<Effort>,
    pub apply: Apply,
    /// Ultra with this many concurrent threads configured: the history gets
    /// the concurrency warning (`ultra_reasoning_concurrency_warning`).
    pub warning: Option<usize>,
}

/// The menus (pages) of the `/model` family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelMenu {
    /// `Loading models…`: the catalog is empty until the first answer; the
    /// quick menu opens on it then.
    /// `request` is the `model/list` it waits for (`Catalog::answered` is the answer, by it).
    Loading { current: Current, request: u64 },
    /// The quick menu: the auto modes and "All models" (the whole list when
    /// the catalog has no auto mode).
    Models(Current),
    /// Every model that is not an auto mode.
    AllModels(Current),
    /// The standard efforts of one model; the preset is the one the menu
    /// opened on (a refresh of the catalog leaves it as it was).
    Reasoning { preset: Preset, current: Current },
    /// Max and Ultra of one model.
    Advanced { preset: Preset, current: Current },
    /// Where an effort change in Plan mode applies.
    PlanScope { model: String, effort: Option<Effort>, source: PlanSource, warning: Option<usize> },
}

/// What a row does when chosen, for the view to give it its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Model,
    AllModels,
    Effort {
        effort: Effort,
        /// The model's default effort: `(default)`.
        default: bool,
        /// A high effort on a model that burns Plus plan limits: the
        /// highlighted row says so.
        warns: bool,
    },
    /// `More reasoning…`, with the advanced efforts it leads to.
    More(Vec<Effort>),
    Scope(Apply),
}

/// One row of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// What the intent for this row carries: the model slug, the effort's
    /// wire name or one of the constants above.
    pub id: String,
    /// The model's name (empty for rows whose words are the view's).
    pub name: String,
    /// The model's description or an effort's; the `All models` row carries
    /// the display name of the model in use.
    pub description: String,
    pub kind: Kind,
    pub current: bool,
    /// The effort the row applies (the default one of a model row), if it
    /// applies one without another menu.
    pub effort: Option<Effort>,
    /// The `s` key picks it for this session.
    pub session: bool,
}

/// What the view needs to say above the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heading {
    Loading,
    Quick,
    All,
    Reasoning { model: String },
    Advanced,
    PlanScope { effort: Option<Effort>, source: PlanSource },
    /// The Reserve-only picker (`Other models return when ordinary usage is available again.`).
    Reserve,
}

/// A menu as rows: what `ModelMenu` and the catalog make of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub heading: Heading,
    pub rows: Vec<Choice>,
    /// The row the cursor starts on (the reasoning menu's current effort).
    pub initial: Option<usize>,
}

/// What a page does on its own when the facts of the catalog say so (the sheet asks after every
/// change of them): the page is the sheet's, the decision is the domain's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// The default provider was written (`dismiss_stale_model_picker`): the sheet is stale.
    Close,
    /// The answer of the request the loading page waits for came: this page takes its place.
    Replace(ModelMenu),
    /// The page cannot go on and leaves, saying why (`Intent::GiveUp`).
    GiveUp(GaveUp),
}

/// What a choice does to the pages of the sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Onward {
    /// Nothing: the id is not a row, or the row is not for this key.
    None,
    /// A menu on top of this one; this one stays below (Esc comes back).
    Push(ModelMenu),
    /// This menu is dismissed and the next one takes its place.
    Replace(ModelMenu),
    /// The pick is final: every menu closes.
    Done(Selection),
    /// Every menu closes and this one (the plan scope prompt) opens.
    Prompt(ModelMenu),
}

impl ModelMenu {
    /// The menu as rows.
    pub fn page(&self, catalog: &Catalog) -> Page {
        match self {
            ModelMenu::Loading { .. } => Page { heading: Heading::Loading, rows: Vec::new(), initial: None },
            ModelMenu::Models(current) => quick(catalog, current),
            ModelMenu::AllModels(current) => Page {
                heading: Heading::All,
                rows: all(catalog, current),
                initial: None,
            },
            ModelMenu::Reasoning { preset, current } => reasoning(preset, current),
            ModelMenu::Advanced { preset, current } => advanced(preset, current),
            ModelMenu::PlanScope { effort, source, .. } => Page {
                heading: Heading::PlanScope { effort: effort.clone(), source: source.clone() },
                rows: [(PLAN_ONLY, Apply::PlanOnly), (ALL_MODES, Apply::AllModes)]
                    .into_iter()
                    .map(|(id, apply)| Choice {
                        id: id.into(),
                        name: String::new(),
                        description: String::new(),
                        kind: Kind::Scope(apply),
                        current: false,
                        effort: None,
                        session: false,
                    })
                    .collect(),
                initial: None,
            },
        }
    }

    /// What this page does with the catalog as it stands: the provider write closes every page;
    /// the loading page goes on with the answer of its own request (never one of another); a
    /// list a refresh left with no choice gives itself up (`refresh_open_model_picker`), and the
    /// pages above it stay (they hold the model they were opened on).
    pub fn next(&self, catalog: &Catalog) -> Option<Next> {
        if catalog.provider != Provider::Settled {
            return Some(Next::Close);
        }
        let gave_up = |menu: &ModelMenu| Next::GiveUp(GaveUp::Empty { reserve: menu.is_reserve() });
        match self {
            ModelMenu::Loading { current, request } => match catalog.answered {
                Some((answered, Answer::Failed)) if answered == *request => {
                    Some(Next::GiveUp(GaveUp::NotLoaded))
                }
                Some((answered, Answer::Listed)) if answered == *request => {
                    let menu = ModelMenu::Models(current.clone());
                    Some(if menu.is_empty(catalog) { gave_up(&menu) } else { Next::Replace(menu) })
                }
                _ => None,
            },
            ModelMenu::Models(_) | ModelMenu::AllModels(_) if self.is_empty(catalog) => Some(gave_up(self)),
            _ => None,
        }
    }

    /// Whether the picker has nothing to offer from `catalog`: it is then
    /// dismissed with a note (`open_all_models_popup_with_view_id`).
    pub fn is_empty(&self, catalog: &Catalog) -> bool {
        match self {
            ModelMenu::Models(_) | ModelMenu::AllModels(_) => self.page(catalog).rows.is_empty(),
            _ => false,
        }
    }

    /// The Reserve-only list.
    pub fn is_reserve(&self) -> bool {
        matches!(self, ModelMenu::Models(current) if current.reserve.is_some())
    }

    /// A row was chosen with the primary key, or with `s` (`session`).
    pub fn choose(&self, catalog: &Catalog, id: &str, session: bool) -> Onward {
        let page = self.page(catalog);
        let Some(row) = page.rows.iter().find(|row| row.id == id) else {
            return Onward::None;
        };
        if session {
            return match (&row.session, row_model(self, row)) {
                (true, Some(model)) => Onward::Done(Selection {
                    apply: if model == RESERVE_MODEL { Apply::Reserve } else { Apply::Session },
                    model,
                    effort: row.effort.clone(),
                    warning: warning(self.current(), row.effort.as_ref()),
                }),
                _ => Onward::None,
            };
        }
        // The quick menu of a catalog without an auto mode is the list of all
        // models: its rows act as that list's.
        let listing = matches!(self, ModelMenu::AllModels(_))
            || (matches!(self, ModelMenu::Models(_)) && page.heading == Heading::All);
        // The Reserve-only list: its one row opens the reasoning of the normal model it borrows,
        // under the Reserve's slug.
        if let (ModelMenu::Models(current), Kind::Model) = (self, &row.kind)
            && current.reserve.is_some()
        {
            let Some(preset) = reserve_preset(catalog, current) else { return Onward::None };
            let mut preset = preset.clone();
            preset.model = RESERVE_MODEL.to_string();
            return reasoning_of(&preset, current);
        }
        match (self, &row.kind) {
            (ModelMenu::Models(current) | ModelMenu::AllModels(current), Kind::Model) if listing => {
                let Some(preset) = catalog.find(&row.id) else { return Onward::None };
                reasoning_of(preset, current)
            }
            (ModelMenu::Models(current), Kind::AllModels) => Onward::Replace(ModelMenu::AllModels(current.clone())),
            (ModelMenu::Models(current), Kind::Model) => {
                let Some(preset) = catalog.find(&row.id) else { return Onward::None };
                if preset.needs_advanced() {
                    return Onward::Push(ModelMenu::Reasoning {
                        preset: preset.clone(),
                        current: current.clone(),
                    });
                }
                settle(current, &preset.model, Some(preset.default_effort.clone()))
            }
            (ModelMenu::Reasoning { preset, current }, Kind::Effort { effort, .. }) => {
                settle(current, &preset.model, Some(effort.clone()))
            }
            (ModelMenu::Reasoning { preset, current }, Kind::More(_)) => Onward::Push(ModelMenu::Advanced {
                preset: preset.clone(),
                current: current.clone(),
            }),
            (ModelMenu::Advanced { preset, current }, Kind::Effort { effort, .. }) => {
                settle(current, &preset.model, Some(effort.clone()))
            }
            (ModelMenu::PlanScope { model, effort, warning, .. }, Kind::Scope(apply)) => {
                Onward::Done(Selection {
                    model: model.clone(),
                    effort: effort.clone(),
                    apply: *apply,
                    warning: *warning,
                })
            }
            _ => Onward::None,
        }
    }

    fn current(&self) -> Option<&Current> {
        match self {
            ModelMenu::Models(current)
            | ModelMenu::AllModels(current)
            | ModelMenu::Reasoning { current, .. }
            | ModelMenu::Advanced { current, .. } => Some(current),
            ModelMenu::Loading { .. } | ModelMenu::PlanScope { .. } => None,
        }
    }
}

/// The model a row of `menu` picks.
fn row_model(menu: &ModelMenu, row: &Choice) -> Option<String> {
    match menu {
        ModelMenu::Models(_) | ModelMenu::AllModels(_) => Some(row.id.clone()),
        ModelMenu::Reasoning { preset, .. } | ModelMenu::Advanced { preset, .. } => Some(preset.model.clone()),
        ModelMenu::Loading { .. } | ModelMenu::PlanScope { .. } => None,
    }
}

fn warning(current: Option<&Current>, effort: Option<&Effort>) -> Option<usize> {
    let threads = current?.threads?;
    (effort == Some(&Effort::Ultra) && threads >= ULTRA_WARNING_THREADS).then_some(threads)
}

/// `should_prompt_plan_mode_reasoning_scope`: in Plan mode, a pick of the
/// model in use that is not a no-op for both the effective effort and the
/// stored Plan defaults asks where it applies.
fn asks_scope(current: &Current, model: &str, effort: &Option<Effort>) -> bool {
    let Some(plan) = &current.plan else { return false };
    model == current.model
        && (effort != &current.effort || model != plan.model || effort != &plan.effort)
}

/// A final pick of `model` with `effort`: the plan scope prompt first when
/// Plan mode asks for it.
fn settle(current: &Current, model: &str, effort: Option<Effort>) -> Onward {
    let warning = warning(Some(current), effort.as_ref());
    // The Reserve is temporary: its effort is the task's, never a default, and never asks
    // where it applies in Plan mode.
    if model == RESERVE_MODEL {
        return Onward::Done(Selection { model: model.into(), effort, apply: Apply::Reserve, warning });
    }
    if asks_scope(current, model, &effort) {
        let source = current.plan.as_ref().map_or(PlanSource::Unknown, |plan| plan.source.clone());
        return Onward::Prompt(ModelMenu::PlanScope { model: model.into(), effort, source, warning });
    }
    let apply = if effort == Some(Effort::Ultra) { Apply::Advanced } else { Apply::Selection };
    Onward::Done(Selection { model: model.into(), effort, apply, warning })
}

/// The reasoning step of a model chosen in the all-models list: a model with
/// one standard effort is picked at once; the others open the menu above the
/// list (`open_reasoning_popup`).
fn reasoning_of(preset: &Preset, current: &Current) -> Onward {
    let (standard, advanced) = preset.split_efforts();
    if standard.len() == 1 && advanced.is_empty() {
        return settle(current, &preset.model, standard.first().cloned());
    }
    Onward::Push(ModelMenu::Reasoning { preset: preset.clone(), current: current.clone() })
}

fn model_row(preset: &Preset, current: &Current, effort: Option<Effort>, session: bool) -> Choice {
    Choice {
        id: preset.model.clone(),
        name: preset.name.clone(),
        description: preset.description.clone(),
        kind: Kind::Model,
        current: preset.model == current.model,
        effort,
        session,
    }
}

/// The preset the Reserve-only row borrows: the normal model of the reserve quota, else Luna
/// (`open_luna_reserve_model_popup`).
fn reserve_preset<'a>(catalog: &'a Catalog, current: &Current) -> Option<&'a Preset> {
    let slug = current.reserve.as_ref().and_then(|reserve| reserve.normal_model.as_deref());
    slug.and_then(|slug| catalog.find(slug)).or_else(|| catalog.find(LUNA_MODEL))
}

/// The Reserve-only list: one row, the borrowed model's name and words, marked current.
fn reserve(catalog: &Catalog, current: &Current) -> Page {
    let rows = reserve_preset(catalog, current)
        .map(|preset| Choice {
            id: RESERVE_MODEL.into(),
            name: preset.name.clone(),
            description: preset.description.clone(),
            kind: Kind::Model,
            current: true,
            effort: None,
            session: false,
        })
        .into_iter()
        .collect();
    Page { heading: Heading::Reserve, rows, initial: None }
}

fn quick(catalog: &Catalog, current: &Current) -> Page {
    if current.reserve.is_some() {
        return reserve(catalog, current);
    }
    let (mut auto, other): (Vec<&Preset>, Vec<&Preset>) =
        catalog.visible().partition(|preset| preset.is_auto());
    if auto.is_empty() {
        return Page { heading: Heading::All, rows: all(catalog, current), initial: None };
    }
    auto.sort_by_key(|preset| preset.auto_order());
    let mut rows: Vec<Choice> = auto
        .into_iter()
        .map(|preset| {
            let advanced = preset.needs_advanced();
            let effort = (!advanced).then(|| preset.default_effort.clone());
            model_row(preset, current, effort, !advanced)
        })
        .collect();
    if !other.is_empty() {
        let none_current = !rows.iter().any(|row| row.current);
        // The row says which model is in use: its display name when the
        // catalog has it, else the slug.
        let label = catalog.visible().find(|preset| preset.model == current.model).map_or_else(
            || current.model.clone(),
            |preset| preset.name.clone(),
        );
        rows.push(Choice {
            id: ALL_MODELS.into(),
            name: String::new(),
            description: label,
            kind: Kind::AllModels,
            current: none_current,
            effort: None,
            session: false,
        });
    }
    Page { heading: Heading::Quick, rows, initial: None }
}

fn all(catalog: &Catalog, current: &Current) -> Vec<Choice> {
    catalog
        .visible()
        .filter(|preset| !preset.is_auto())
        .map(|preset| {
            let direct = preset.direct_effort();
            model_row(preset, current, direct.clone(), direct.is_some())
        })
        .collect()
}

fn reasoning(preset: &Preset, current: &Current) -> Page {
    let (standard, advanced) = preset.split_efforts();
    let default_choice = standard.contains(&preset.default_effort).then(|| preset.default_effort.clone());
    let in_current_model = current.model == preset.model;
    let highlight = if in_current_model {
        match &current.plan {
            Some(PlanMode { source: PlanSource::Override(effort), .. }) => Some(effort.clone()),
            _ => current.effort.clone(),
        }
    } else {
        default_choice.clone().or_else(|| standard.first().cloned())
    };
    let selection = highlight.clone().or_else(|| default_choice.clone());
    let initial = standard.iter().position(|effort| Some(effort) == selection.as_ref());
    let warn_effort = [Effort::XHigh, Effort::High]
        .into_iter()
        .find(|warn| preset.efforts.iter().any(|option| &option.effort == warn));
    let warn_model = preset.model.starts_with("gpt-5.1-codex") || preset.model.starts_with("gpt-5.2");
    let mut rows: Vec<Choice> = standard
        .iter()
        .map(|effort| Choice {
            id: effort.as_str().into(),
            name: String::new(),
            description: preset.effort_description(effort),
            kind: Kind::Effort {
                effort: effort.clone(),
                default: Some(effort) == default_choice.as_ref(),
                warns: warn_model && Some(effort) == warn_effort.as_ref(),
            },
            current: in_current_model && Some(effort) == highlight.as_ref(),
            effort: Some(effort.clone()),
            session: true,
        })
        .collect();
    if !advanced.is_empty() {
        rows.push(Choice {
            id: MORE_REASONING.into(),
            name: String::new(),
            description: String::new(),
            current: in_current_model && highlight.as_ref().is_some_and(is_advanced),
            effort: None,
            session: false,
            kind: Kind::More(advanced),
        });
    }
    Page { heading: Heading::Reasoning { model: preset.name.clone() }, rows, initial }
}

fn advanced(preset: &Preset, current: &Current) -> Page {
    let mut efforts: Vec<Effort> = preset
        .efforts
        .iter()
        .map(|option| option.effort.clone())
        .filter(is_advanced)
        .collect();
    if efforts.is_empty() && is_advanced(&preset.default_effort) {
        efforts.push(preset.default_effort.clone());
    }
    efforts.sort_by_key(|effort| matches!(effort, Effort::Ultra));
    let highlight = (current.model == preset.model).then(|| current.effort.clone()).flatten();
    Page {
        heading: Heading::Advanced,
        rows: efforts
            .into_iter()
            .map(|effort| Choice {
                id: effort.as_str().into(),
                name: String::new(),
                description: String::new(),
                current: Some(&effort) == highlight.as_ref(),
                kind: Kind::Effort { effort: effort.clone(), default: false, warns: false },
                effort: Some(effort),
                session: true,
            })
            .collect(),
        initial: None,
    }
}
