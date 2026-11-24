//! The facts the footer and the status line read besides the session's own
//! (oracle `chatwidget/status_surfaces.rs`, `branch_summary.rs`,
//! `workspace_messages.rs`, `bottom_pane` footer props). Each fact has one
//! writer:
//!
//! - per thread, in `ThreadPart`: the collaboration mode (`thread/settings/updated`) and whether
//!   the thread was resumed with history and awaits its token usage (`thread/started` with turns;
//!   the usage itself is `status::ThreadPart`'s, so "pending" is derived, not copied);
//! - app-wide, in `Facts`, by `Ev` only: the working directory the workspace is probed in, the
//!   clock, the git summary and the workspace headline (answers of the host's probes), the side
//!   conversation's label, the viewed agent's label and the agents-navigation flag.
//!
//! The side conversation's active flag is `CommandFlags::side_conversation_active` (its one
//! writer is `chat::Ev::Flags`); the goal is `ThreadModel::goal`; the model and tier names are
//! read from the select catalog and `chat::Model::service_tiers` by `model_display_name` and
//! `tier_display_name`.
//!
//! The probes are data: `Facts::probes` lists what the host is asked and has not yet answered,
//! each under a request number, so the shell runs one request once and an answer of an earlier
//! request (another cwd) is dropped. Nothing here reads a clock or a file.

use codex_app_server_protocol::GetWorkspaceMessagesResponse;
use codex_app_server_protocol::WorkspaceMessageType;
use codex_protocol::config_types::ModeKind;
use serde::Deserialize;

use super::{Model, StatusFacts, ThreadPart};
use crate::commands::ServiceTierCommand;
use crate::format::{format_goal_elapsed_seconds, format_tokens_compact};
use crate::screens::select::models::Catalog;
use crate::screens::ThreadSlots;
use crate::threads::{Goal, GoalStatus, ThreadModel, TurnState};

/// How long a workspace headline is kept before it is read again
/// (`WORKSPACE_HEADLINE_REFRESH_INTERVAL`), in seconds.
pub const HEADLINE_REFRESH_SECS: u64 = 5 * 60;

/// What the model column shows before any model is known (`DEFAULT_MODEL_DISPLAY_NAME`).
pub const LOADING_MODEL: &str = "loading";

/// An open pull request of the current branch (`StatusLinePullRequest`); the URL is kept so a
/// click opens the PR the label names.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
}

/// Committed additions and deletions between `HEAD` and the default branch
/// (`GitBranchDiffStats`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct BranchChanges {
    pub additions: u64,
    pub deletions: u64,
}

/// What the git probes found; either part may be missing when its probe failed
/// (`StatusLineGitSummary`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct GitSummary {
    pub pull_request: Option<PullRequest>,
    pub changes: Option<BranchChanges>,
}

impl GitSummary {
    /// The `pull-request-number` item's text (`PR #42`).
    pub fn pull_request_text(&self) -> Option<String> {
        self.pull_request.as_ref().map(|pr| format!("PR #{}", pr.number))
    }

    /// The `branch-changes` item's text (`+3 -1`, `No changes`).
    pub fn changes_text(&self) -> Option<String> {
        self.changes.map(|stats| {
            if stats.additions == 0 && stats.deletions == 0 {
                "No changes".to_string()
            } else {
                format!("+{} -{}", stats.additions, stats.deletions)
            }
        })
    }
}

/// What the headline probe answered (`WorkspaceHeadlineFetchResult`, and the error case).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlineAnswer {
    /// The first headline message, trimmed; none when there is none.
    Available(Option<String>),
    /// The backend has no workspace messages: do not ask again.
    FeatureDisabled,
    /// The read failed: the old headline stays, the next try is due after the interval.
    Failed,
}

impl HeadlineAnswer {
    /// `workspace_headline_from_response`.
    pub fn from_response(response: GetWorkspaceMessagesResponse) -> Self {
        if !response.feature_enabled {
            return Self::FeatureDisabled;
        }
        Self::Available(response.messages.into_iter().find_map(|message| {
            (message.message_type == WorkspaceMessageType::Headline)
                .then(|| message.message_body.trim().to_string())
                .filter(|headline| !headline.is_empty())
        }))
    }
}

/// A read the host is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// The pull request and the branch changes of the repository at `cwd`.
    GitSummary { request: u64, cwd: String },
    /// `account/workspaceMessages/read`.
    Headline { request: u64 },
}

impl Probe {
    pub fn request(&self) -> u64 {
        match self {
            Self::GitSummary { request, .. } | Self::Headline { request } => *request,
        }
    }
}

/// The events of the app-wide facts; `chat::Ev::Facts`. The seed form is the same JSON
/// (`{"facts":{"cwd":"/w"}}`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ev {
    /// The directory the workspace is probed in: the host's at open, the shown thread's after
    /// (`status_line_cwd`). A different one drops the git summary and reads it again.
    Cwd(String),
    /// The answer of `Probe::GitSummary { request, .. }`.
    GitSummary { request: u64, summary: GitSummary },
    /// The answer of `Probe::Headline { request }`.
    Headline { request: u64, answer: HeadlineAnswer },
    /// The side conversation's footer label, or none (`set_side_conversation_context_label`:
    /// `Side from main thread · …`, `ctrl-/ for side`).
    SideLabel(Option<String>),
    /// What the footer says of the side conversations, as data (the view words it): the app's
    /// fact (`app/side.rs`), where `SideLabel` is a made label (a seed).
    Side(Option<SideFooter>),
    /// The viewed agent's footer label, or none (`set_active_agent_label`).
    AgentLabel(Option<String>),
    /// The empty-prompt `←` opens the agents page (`enable_agents_navigation`).
    AgentsNavigation(bool),
    /// The app is closing (`ChatWidget::is_shutting_down`): the composer takes nothing more.
    ShuttingDown(bool),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Lookup {
    /// Not asked yet (or the cwd changed since).
    #[default]
    Idle,
    Pending { request: u64 },
    Done(GitSummary),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Headline {
    text: Option<String>,
    pending: Option<u64>,
    last_requested: Option<u64>,
    disabled: bool,
}

/// The app-wide facts (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub side_label: Option<String>,
    pub side: Option<SideFooter>,
    pub agent_label: Option<String>,
    pub agents_navigation: bool,
    pub shutting_down: bool,
    cwd: Option<String>,
    now: u64,
    issued: u64,
    git: Lookup,
    headline: Headline,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            side_label: None,
            side: None,
            agent_label: None,
            // the oracle's chat is built with agents navigation on
            agents_navigation: true,
            shutting_down: false,
            cwd: None,
            now: 0,
            issued: 0,
            git: Lookup::Idle,
            headline: Headline::default(),
        }
    }
}

/// `status_line_items_with_invalids().0.contains(..)`: the configured items (the default list
/// holds none of the git or headline items) name one of `ids`.
fn uses(status: &StatusFacts, ids: &[&str]) -> bool {
    status
        .items
        .as_ref()
        .is_some_and(|items| items.iter().any(|item| ids.contains(&item.as_str())))
}

impl Facts {
    /// The time now, in epoch seconds (`AppEv::Clock`, told through `ScreenStates::set_now`): the
    /// headline's due time is measured on it.
    pub(crate) fn set_now(&mut self, now: u64) {
        self.now = now;
    }

    /// The time the facts were last told, in epoch seconds.
    pub(crate) fn now(&self) -> u64 {
        self.now
    }

    pub(super) fn apply(&mut self, ev: Ev) {
        match ev {
            Ev::Cwd(cwd) => {
                // `sync_status_line_git_summary_state`: another directory, another summary
                if self.cwd.as_deref() != Some(cwd.as_str()) {
                    self.cwd = Some(cwd);
                    self.git = Lookup::Idle;
                }
            }
            Ev::GitSummary { request, summary } => {
                // an answer of an earlier request (an earlier cwd) is stale
                if self.git == (Lookup::Pending { request }) {
                    self.git = Lookup::Done(summary);
                }
            }
            Ev::Headline { request, answer } => {
                if self.headline.pending != Some(request) {
                    return;
                }
                self.headline.pending = None;
                match answer {
                    HeadlineAnswer::Available(text) => {
                        self.headline.disabled = false;
                        self.headline.text = text;
                    }
                    HeadlineAnswer::FeatureDisabled => {
                        self.headline.disabled = true;
                        self.headline.text = None;
                    }
                    HeadlineAnswer::Failed => {}
                }
            }
            Ev::SideLabel(label) => self.side_label = label,
            Ev::Side(side) => self.side = side,
            Ev::AgentLabel(label) => self.agent_label = label,
            Ev::AgentsNavigation(on) => self.agents_navigation = on,
            Ev::ShuttingDown(on) => self.shutting_down = on,
        }
    }

    /// `sync_status_surface_shared_state`: what the configured items need decides what is kept
    /// and what is asked. An item that left the line drops its fact.
    pub(super) fn sync(&mut self, status: &StatusFacts, signed_in: bool) {
        if uses(status, &["pull-request-number", "branch-changes"]) {
            if self.git == Lookup::Idle && self.cwd.is_some() {
                self.issued += 1;
                self.git = Lookup::Pending { request: self.issued };
            }
        } else {
            self.git = Lookup::Idle;
        }

        if uses(status, &["workspace-headline"]) {
            if self.headline_due(signed_in) {
                self.issued += 1;
                self.headline.pending = Some(self.issued);
                self.headline.last_requested = Some(self.now);
            }
        } else {
            self.headline = Headline::default();
        }
    }

    /// `status_line_workspace_headline_should_fetch`: no read in flight, the backend has the
    /// feature, a ChatGPT sign-in, and the last read is older than the interval.
    fn headline_due(&self, signed_in: bool) -> bool {
        self.headline.pending.is_none()
            && !self.headline.disabled
            && signed_in
            && self.headline.last_requested.is_none_or(|last| {
                self.now.saturating_sub(last) >= HEADLINE_REFRESH_SECS
            })
    }

    /// The reads asked and not yet answered, oldest first.
    pub fn probes(&self) -> Vec<Probe> {
        let mut probes = Vec::new();
        if let (Lookup::Pending { request }, Some(cwd)) = (&self.git, &self.cwd) {
            probes.push(Probe::GitSummary { request: *request, cwd: cwd.clone() });
        }
        if let Some(request) = self.headline.pending {
            probes.push(Probe::Headline { request });
        }
        probes.sort_by_key(Probe::request);
        probes
    }

    /// The directory the workspace is probed in.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// The summary of the current cwd, once its probe answered.
    pub fn git_summary(&self) -> Option<&GitSummary> {
        match &self.git {
            Lookup::Done(summary) => Some(summary),
            _ => None,
        }
    }

    /// The `workspace-headline` item's text.
    pub fn headline(&self) -> Option<&str> {
        self.headline.text.as_deref()
    }

    /// When the headline is due to be read again (epoch seconds), for the view to schedule a
    /// frame at; none while it is not read at all (item unused, feature off, read in flight).
    pub fn headline_due_at(&self) -> Option<u64> {
        if self.headline.disabled || self.headline.pending.is_some() {
            return None;
        }
        self.headline.last_requested.map(|last| last + HEADLINE_REFRESH_SECS)
    }
}

/// `format_agent_picker_item_name` and `AgentNavigationState::active_agent_label`: the label of
/// the viewed agent, hidden until more than one thread is known. `threads` are the known
/// threads; `primary` the session's first. Feeds `Ev::AgentLabel`.
pub fn agent_label(
    threads: &[codex_app_server_protocol::Thread],
    displayed: Option<&str>,
    primary: Option<&str>,
) -> Option<String> {
    use codex_app_server_protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    if threads.len() <= 1 {
        return None;
    }
    let displayed = displayed?;
    let is_primary = primary == Some(displayed);
    let known = threads.iter().find(|thread| thread.id == displayed);
    let Some(thread) = known else {
        return Some(picker_name(None, None, is_primary));
    };
    if !is_primary
        && let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_path: Some(path), .. }) =
            &thread.source
        && !path.to_string().trim().is_empty()
    {
        return Some(format!("`{path}`"));
    }
    Some(picker_name(thread.agent_nickname.as_deref(), thread.agent_role.as_deref(), is_primary))
}

fn picker_name(nickname: Option<&str>, role: Option<&str>, is_primary: bool) -> String {
    if is_primary {
        return "Main [default]".to_string();
    }
    fn clean(text: Option<&str>) -> Option<&str> {
        text.map(str::trim).filter(|text| !text.is_empty())
    }
    match (clean(nickname), clean(role)) {
        (Some(nickname), Some(role)) => format!("{nickname} [{role}]"),
        (Some(nickname), None) => nickname.to_string(),
        (None, Some(role)) => format!("[{role}]"),
        (None, None) => "Agent".to_string(),
    }
}

/// `model_display_name`: the catalog's name of the model, else the slug (the reserve model has
/// its own name); `loading` while no model is known.
pub fn model_display_name(catalog: &Catalog, model: Option<&str>) -> String {
    let Some(model) = model.filter(|model| !model.is_empty()) else {
        return LOADING_MODEL.to_string();
    };
    match catalog.presets.iter().find(|preset| preset.model == model) {
        Some(preset) => preset.name.clone(),
        None => StatusFacts { model: Some(model.to_string()), ..StatusFacts::default() }
            .model_name()
            .unwrap_or(model)
            .to_string(),
    }
}

/// `processing_tier_display_name`: the name the model's offered tiers give the current tier, or
/// `Flex` for the flex tier on a provider other than Bedrock.
pub fn tier_display_name(
    tiers: &[ServiceTierCommand],
    tier: Option<&str>,
    provider: Option<&str>,
) -> Option<String> {
    let id = tier?;
    tiers
        .iter()
        .find(|offered| offered.id == id)
        .map(|offered| offered.name.clone())
        .or_else(|| {
            (id == "flex" && provider != Some("amazon-bedrock")).then(|| "Flex".to_string())
        })
}

/// `current_model_service_tier_commands`: the tiers the current model offers in the catalog,
/// each as a command named in lower case; a tier is offered when its feature is on (flex always;
/// `ultrafast` needs `UltrafastMode`, any other `FastMode`). A model the catalog does not know
/// offers the tiers the session was seeded with, filtered the same way.
pub fn tier_commands(
    status: &StatusFacts,
    thread_model: Option<&str>,
    catalog: &Catalog,
    seeded: &[ServiceTierCommand],
) -> Vec<ServiceTierCommand> {
    let enabled = |id: &str| match id {
        "flex" => true,
        "ultrafast" => status.ultrafast_mode,
        _ => status.fast_mode,
    };
    let command = |id: &str, name: &str, description: &str| ServiceTierCommand {
        id: id.to_string(),
        name: name.to_lowercase(),
        description: description.to_string(),
    };
    let known = thread_model
        .or(status.model.as_deref())
        .and_then(|model| catalog.presets.iter().find(|preset| preset.model == model));
    match known {
        Some(preset) => preset
            .service_tiers
            .iter()
            .filter(|tier| enabled(&tier.id))
            .map(|tier| command(&tier.id, &tier.name, &tier.description))
            .collect(),
        None => seeded
            .iter()
            .filter(|tier| enabled(&tier.id))
            .map(|tier| command(&tier.id, &tier.name, &tier.description))
            .collect(),
    }
}

/// `effective_service_tier`: the tier the thread runs on. `configured` is the thread's own tier,
/// else the config's. The flex tier stands; a model the catalog does not know keeps a configured
/// tier its feature allows; a known model keeps `default` and the tiers it offers, and runs on its
/// own default tier when none is configured. Every tier needs its feature.
pub fn effective_service_tier(
    status: &StatusFacts,
    configured: Option<&str>,
    model: Option<&str>,
    catalog: &Catalog,
) -> Option<String> {
    let enabled = |id: &str| match id {
        "flex" => true,
        "ultrafast" => status.ultrafast_mode,
        _ => status.fast_mode,
    };
    if configured == Some("flex") {
        return configured.map(str::to_owned);
    }
    let preset = model.and_then(|model| catalog.presets.iter().find(|preset| preset.model == model));
    let Some(preset) = preset else {
        return configured.filter(|tier| enabled(tier)).map(str::to_owned);
    };
    let offers = |id: &str| preset.service_tiers.iter().any(|tier| tier.id == id);
    match configured {
        Some("default") => configured.map(str::to_owned),
        Some(tier) if offers(tier) => Some(tier.to_owned()),
        Some(_) => None,
        None => preset.default_service_tier.clone().filter(|tier| offers(tier)),
    }
    .filter(|tier| enabled(tier))
}

/// The footer's facts of the shown thread, one projection of the parts that hold them
/// (oracle `collaboration_mode_label`, `collaboration_mode_indicator`, `FooterProps`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footer {
    /// `Default` / `Plan`: the label of the mode, when collaboration modes are on.
    pub mode_label: Option<&'static str>,
    /// The `Plan mode` indicator (the goal's indicator takes its place when this is off).
    pub plan_indicator: bool,
    pub side_active: bool,
    pub side_label: Option<String>,
    pub side: Option<SideFooter>,
    pub agent_label: Option<String>,
    pub agents_navigation: bool,
    /// The resumed thread has no token usage yet: the footer hides `Context 100% left` and the
    /// token items stay out of the status line.
    pub usage_pending: bool,
}

impl ThreadPart {
    /// The thread's collaboration mode (`Default` until the server says another).
    pub fn mode(&self) -> ModeKind {
        self.mode
    }

    /// The thread was resumed with history and no token usage has come since
    /// (`token_usage_pending`).
    pub fn awaits_usage(&self) -> bool {
        self.awaiting_usage
    }
}

/// The token usage of the thread is still pending: resumed with history, nothing counted yet.
pub fn usage_pending(slots: &ThreadSlots) -> bool {
    slots.chat.awaiting_usage && !slots.status.received
}

/// The footer's facts for the thread whose parts are `slots`.
pub fn footer(chat: &Model, slots: &ThreadSlots) -> Footer {
    let modes = chat.flags.collaboration_modes_enabled;
    let mode = slots.chat.mode;
    Footer {
        mode_label: (modes && mode.is_tui_visible()).then(|| mode.display_name()),
        plan_indicator: modes && mode == ModeKind::Plan,
        side_active: chat.flags.side_conversation_active,
        side_label: chat.facts.side_label.clone(),
        side: chat.facts.side.clone(),
        agent_label: chat.facts.agent_label.clone(),
        agents_navigation: chat.facts.agents_navigation,
        usage_pending: usage_pending(slots),
    }
}

/// A parent thread's condition that changed while its side conversation is shown
/// (`SideParentStatus`); the words are the footer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideStatus {
    NeedsInput,
    NeedsApproval,
    Failed,
    Interrupted,
    Closed,
    Finished,
}

impl SideStatus {
    /// A status the user can act on (an approval or a question waits): it clears when the request
    /// is resolved or an item starts.
    pub fn is_actionable(self) -> bool {
        matches!(self, SideStatus::NeedsInput | SideStatus::NeedsApproval)
    }
}

/// What the footer shows of side conversations: data; its words are the footer's strings.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideFooter {
    /// The shown thread has a side conversation: `ctrl + / for side`.
    Parent { switch: String },
    /// The shown thread is a side conversation.
    Side {
        parent_is_main: bool,
        /// The parent's name when it is not the main thread.
        parent_label: String,
        status: Option<SideStatus>,
        /// The toggle key's label.
        switch: Option<String>,
    },
}

/// What the footer says of the thread's goal (`GoalStatusIndicator`); the words are the footer's,
/// the usage is the compact figure the oracle builds (`40K / 50K`, `3m`, `40K / 50K tokens`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoalIndicator {
    Active { usage: Option<GoalUsage> },
    Paused,
    Blocked,
    UsageLimited,
    BudgetLimited { usage: Option<GoalUsage> },
    Complete { usage: Option<GoalUsage> },
}

/// The figure in a goal line's parentheses: `40K / 50K`, `40K` or `3m`; `tokens` says the footer
/// names the unit after it (`Goal unmet (40K / 50K tokens)`, `Goal achieved (40K tokens)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalUsage {
    pub figure: String,
    pub tokens: bool,
}

impl GoalUsage {
    fn plain(figure: String) -> Self {
        Self { figure, tokens: false }
    }

    fn of_tokens(figure: String) -> Self {
        Self { figure, tokens: true }
    }
}

impl GoalIndicator {
    /// `goal_status_indicator_from_app_goal`; `running_secs` is the time an active goal has run
    /// since it was last seen (`GoalStatusState::indicator`).
    pub fn of(goal: &Goal, running_secs: i64) -> Self {
        let elapsed = goal.time_used_seconds.saturating_add(running_secs.max(0));
        match goal.status {
            GoalStatus::Active => Self::Active {
                usage: Some(GoalUsage::plain(match goal.token_budget {
                    Some(budget) => format!(
                        "{} / {}",
                        format_tokens_compact(goal.tokens_used),
                        format_tokens_compact(budget)
                    ),
                    None => format_goal_elapsed_seconds(elapsed),
                })),
            },
            GoalStatus::Paused => Self::Paused,
            GoalStatus::Blocked => Self::Blocked,
            GoalStatus::UsageLimited => Self::UsageLimited,
            GoalStatus::BudgetLimited => Self::BudgetLimited {
                usage: goal.token_budget.map(|budget| {
                    GoalUsage::of_tokens(format!(
                        "{} / {}",
                        format_tokens_compact(goal.tokens_used),
                        format_tokens_compact(budget)
                    ))
                }),
            },
            GoalStatus::Complete => Self::Complete {
                usage: Some(if goal.token_budget.is_some() {
                    GoalUsage::of_tokens(format_tokens_compact(goal.tokens_used))
                } else {
                    GoalUsage::plain(format_goal_elapsed_seconds(elapsed))
                }),
            },
        }
    }
}

/// The goal indicator of the thread at `now` (epoch seconds, 0 while the clock is unknown): an
/// active goal counts the seconds of the running turn since the goal was last updated or the turn
/// began, whichever is later (`active_turn_started_at`).
pub fn goal_indicator(thread: &ThreadModel, now: i64) -> Option<GoalIndicator> {
    let goal = thread.goal()?;
    let running = match thread.turn() {
        TurnState::Running(turn) if goal.status == GoalStatus::Active && now > 0 => {
            let since = goal.updated_at.max(turn.started_at.unwrap_or(0));
            now.saturating_sub(since)
        }
        _ => 0,
    };
    Some(GoalIndicator::of(goal, running))
}

#[cfg(test)]
mod goal_tests {
    use super::*;

    fn goal(status: GoalStatus, budget: Option<i64>, used: i64, secs: i64) -> Goal {
        Goal {
            objective: "ship it".into(),
            status,
            token_budget: budget,
            tokens_used: used,
            time_used_seconds: secs,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// `goal_status_indicator_line_formats_goal_text`: the usage of each status.
    #[test]
    fn the_usage_of_each_status_is_the_oracles() {
        let usage = |goal: Goal| GoalIndicator::of(&goal, 0);
        assert_eq!(
            usage(goal(GoalStatus::Active, Some(50_000), 40_000, 0)),
            GoalIndicator::Active { usage: Some(GoalUsage::plain("40K / 50K".into())) }
        );
        assert_eq!(
            usage(goal(GoalStatus::Active, None, 0, 125)),
            GoalIndicator::Active { usage: Some(GoalUsage::plain("2m".into())) }
        );
        assert_eq!(
            usage(goal(GoalStatus::BudgetLimited, Some(50_000), 50_000, 0)),
            GoalIndicator::BudgetLimited { usage: Some(GoalUsage::of_tokens("50K / 50K".into())) }
        );
        assert_eq!(
            usage(goal(GoalStatus::BudgetLimited, None, 5, 0)),
            GoalIndicator::BudgetLimited { usage: None }
        );
        assert_eq!(
            usage(goal(GoalStatus::Complete, None, 0, 3_700)),
            GoalIndicator::Complete { usage: Some(GoalUsage::plain("1h 1m".into())) }
        );
        assert_eq!(
            usage(goal(GoalStatus::Complete, Some(100), 70, 9)),
            GoalIndicator::Complete { usage: Some(GoalUsage::of_tokens("70".into())) }
        );
        assert_eq!(usage(goal(GoalStatus::Paused, None, 0, 0)), GoalIndicator::Paused);
    }

    /// The seconds a running turn adds count only for an active goal.
    #[test]
    fn a_running_turn_adds_its_seconds_to_an_active_goal() {
        let active = goal(GoalStatus::Active, None, 0, 10);
        assert_eq!(
            GoalIndicator::of(&active, 5),
            GoalIndicator::Active { usage: Some(GoalUsage::plain("15s".into())) }
        );
        let paused = goal(GoalStatus::Paused, None, 0, 10);
        assert_eq!(GoalIndicator::of(&paused, 5), GoalIndicator::Paused);
    }
}
