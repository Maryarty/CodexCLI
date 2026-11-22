//! The `/permissions` family of the selection dialog: the server's catalog of
//! permission profiles the picker reads, the menus of the stack (data only,
//! ADR-010: the words are the view's) and the rules that turn a choice into
//! the next menu or a final pick.
//! Oracle: `chatwidget/permission_popups.rs`, `permissions_menu.rs`,
//! `permission_discovery.rs`, `permission_shortcuts.rs`, and
//! `permission_discovery.rs` (the catalog and its availability rules).

use std::collections::BTreeMap;

use codex_app_server_protocol::ApprovalsReviewer;
use codex_app_server_protocol::AskForApproval;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigRequirements;
use codex_app_server_protocol::ConfigRequirementsReadResponse;
use codex_app_server_protocol::PermissionProfileListParams;
use codex_app_server_protocol::PermissionProfileListResponse;
use codex_app_server_protocol::PermissionProfileSummary;
use codex_app_server_protocol::RequestId;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use serde::Deserialize;

pub type Approval = AskForApproval;
pub type Reviewer = ApprovalsReviewer;

/// The id of the failure menu's `Retry` row.
pub const RETRY: &str = "retry";
/// What a row id of `Approve for me` adds to its profile's id.
const AUTO_REVIEW: &str = "+auto-review";
/// The ids of the full-access confirmation's rows.
pub const CONTINUE: &str = "continue";
pub const CANCEL: &str = "cancel";

/// The built-in approval presets (`builtin_approval_presets`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    ReadOnly,
    /// `Ask for approval` (and `Approve for me`, by its reviewer).
    Auto,
    FullAccess,
}

impl Preset {
    /// The built-in permission profile the preset selects.
    pub const fn profile(self) -> &'static str {
        match self {
            Preset::ReadOnly => BUILT_IN_PERMISSION_PROFILE_READ_ONLY,
            Preset::Auto => BUILT_IN_PERMISSION_PROFILE_WORKSPACE,
            Preset::FullAccess => BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS,
        }
    }

    /// The approval policy the preset applies.
    pub const fn approval(self) -> Approval {
        match self {
            Preset::ReadOnly | Preset::Auto => Approval::OnRequest,
            Preset::FullAccess => Approval::Never,
        }
    }

    /// The built-in preset that selects `profile`, if it is one.
    pub fn of_profile(profile: &str) -> Option<Preset> {
        [Preset::ReadOnly, Preset::Auto, Preset::FullAccess]
            .into_iter()
            .find(|preset| preset.profile() == profile)
    }
}

fn user() -> Reviewer {
    Reviewer::User
}

/// Why one approval policy cannot be set (`approval_policy.can_set(..)`): the
/// text is the config's error, shown as it is.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Limit {
    pub approval: Approval,
    pub reason: String,
}

/// What the config forbids of the legacy menu (`can_set` errors).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Limits {
    pub approvals: Vec<Limit>,
    /// `guardian_disabled_reason(false)`: the reviewer cannot be the user.
    pub reviewer_user: Option<String>,
    /// `guardian_disabled_reason(true)`: the reviewer cannot be auto-review.
    pub reviewer_auto: Option<String>,
}

/// What the thread had when the menu opened: the opener reads it from the
/// thread and the config and the menu keeps it (a snapshot that dies with the
/// menu, not a copy of the thread).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Current {
    pub approval: Option<Approval>,
    #[serde(default = "user")]
    pub reviewer: Reviewer,
    /// The built-in preset the approval policy and the permission profile
    /// both match (`preset_matches_current`).
    pub matches: Option<Preset>,
    /// The active permission profile's id.
    pub profile: Option<String>,
    /// The catalog is asked for (`thread_id.is_some()`, the explicit profile
    /// mode, a custom active profile): else the legacy preset list.
    pub discover: bool,
    /// `Feature::GuardianApproval`: the `Approve for me` row exists.
    pub guardian: bool,
    /// The requirements make auto-review mandatory for the model in use.
    pub auto_required: bool,
    /// The model in use is a cyber model (the full-access warning).
    pub cyber: bool,
    /// `auto_review_available(config)`: the cyber warning recommends it.
    pub auto_available: bool,
    pub limits: Limits,
}

impl Current {
    /// What the chat's status facts know of the thread: the approval policy,
    /// the reviewer and the built-in preset both match (the sandbox profile of
    /// the facts is the built-in one it names). The feature flag, the
    /// requirements' limits and the model's class are the config's, not the
    /// facts': they stay off until the opener has them.
    pub fn from_facts(facts: &crate::screens::chat::StatusFacts) -> Self {
        use crate::screens::status::Profile as Sandbox;
        let approval = match facts.approval_policy.as_deref() {
            Some("on-request") => Some(Approval::OnRequest),
            Some("never") => Some(Approval::Never),
            Some("untrusted") => Some(Approval::UnlessTrusted),
            _ => None,
        };
        let preset = match facts.profile {
            Sandbox::ReadOnly => Preset::ReadOnly,
            Sandbox::Workspace => Preset::Auto,
            Sandbox::Full => Preset::FullAccess,
        };
        let matches = (approval == Some(preset.approval())).then_some(preset);
        Current {
            approval,
            reviewer: if facts.auto_review { Reviewer::AutoReview } else { Reviewer::User },
            matches,
            profile: matches.map(|preset| preset.profile().to_owned()),
            discover: true,
            ..Current::default()
        }
    }
}

impl Default for Current {
    fn default() -> Self {
        Current {
            approval: None,
            reviewer: Reviewer::User,
            matches: None,
            profile: None,
            discover: false,
            guardian: false,
            auto_required: false,
            cyber: false,
            auto_available: false,
            limits: Limits::default(),
        }
    }
}

/// What the server's requirements allow (`ConfigRequirements`, the part the
/// menu reads).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Requirements {
    /// `allowed_permission_profiles`: the profiles that may be selected.
    pub profiles: Option<BTreeMap<String, bool>>,
    pub approvals: Option<Vec<Approval>>,
    pub reviewers: Option<Vec<Reviewer>>,
    /// `auto_review.required_on_models`.
    pub auto_required: Option<Vec<String>>,
}

impl Requirements {
    pub fn from_wire(wire: &ConfigRequirements) -> Self {
        Requirements {
            profiles: wire.allowed_permission_profiles.clone(),
            approvals: wire.allowed_approval_policies.clone(),
            reviewers: wire.allowed_approvals_reviewers.clone(),
            auto_required: wire
                .auto_review
                .as_ref()
                .and_then(|auto| auto.required_on_models.clone()),
        }
    }
}

/// One profile the server lists (`PermissionProfileSummary`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "yes")]
    pub allowed: bool,
}

fn yes() -> bool {
    true
}

impl Profile {
    fn from_wire(profile: PermissionProfileSummary) -> Profile {
        Profile { id: profile.id, description: profile.description, allowed: profile.allowed }
    }
}

/// The profiles of a `permissionProfile/list` page.
pub fn profiles_of(wire: Vec<PermissionProfileSummary>) -> Vec<Profile> {
    wire.into_iter().map(Profile::from_wire).collect()
}

/// Why a row cannot be chosen: the view gives it its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disabled {
    /// `Not available on this server`.
    NotOnServer,
    /// `Disabled by requirements`.
    Requirements,
    /// The config's own reason (the legacy menu), as it is.
    Reason(String),
    /// No reason given: a row that only waits (`Loading permission profiles…`).
    Idle,
}

/// The permission catalog of the connected server and the requirements over
/// it (`PermissionDiscovery`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Discovery {
    pub profiles: Vec<Profile>,
    pub requirements: Option<Requirements>,
}

impl Discovery {
    /// The catalog of a `permissionProfile/list` answer and the requirements
    /// read beside it; a server that lists one id twice is refused.
    pub fn from_wire(
        profiles: Vec<PermissionProfileSummary>,
        requirements: Option<&ConfigRequirements>,
    ) -> Result<Discovery, Failure> {
        Discovery::new(
            profiles.into_iter().map(Profile::from_wire).collect(),
            requirements.map(Requirements::from_wire),
        )
    }

    /// The catalog of the pages read and the requirements read beside them.
    pub fn new(
        profiles: Vec<Profile>,
        requirements: Option<Requirements>,
    ) -> Result<Discovery, Failure> {
        let mut ids = std::collections::HashSet::new();
        if !profiles.iter().all(|profile| ids.insert(profile.id.clone())) {
            return Err(Failure::Duplicate);
        }
        Ok(Discovery { profiles, requirements })
    }

    /// `disabled_reason`: a profile the server does not list is not
    /// available; one the requirements do not allow (or with a policy or a
    /// reviewer they do not allow) is disabled by them.
    pub fn disabled(
        &self,
        id: &str,
        approval: Option<Approval>,
        reviewer: Option<Reviewer>,
    ) -> Option<Disabled> {
        let Some(profile) = self.profiles.iter().find(|profile| profile.id == id) else {
            return Some(Disabled::NotOnServer);
        };
        let requirements = self.requirements.as_ref();
        let barred = !profile.allowed
            || requirements
                .and_then(|r| r.profiles.as_ref())
                .is_some_and(|allowed| allowed.get(id) != Some(&true))
            || requirements
                .and_then(|r| r.approvals.as_ref())
                .is_some_and(|allowed| approval.is_some_and(|value| !allowed.contains(&value)))
            || requirements
                .and_then(|r| r.reviewers.as_ref())
                .is_some_and(|allowed| reviewer.is_some_and(|value| !allowed.contains(&value)));
        barred.then_some(Disabled::Requirements)
    }
}

/// Why the catalog could not be read: the text is the oracle's, the words
/// the view's (the server's own error stays as it came).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The server answered the same id twice.
    Duplicate,
    /// A cloud conversation uses the server's permissions.
    Cloud,
    /// An older server: no `permissionProfile/list`.
    Unsupported,
    /// The pages never ended.
    Pagination,
    /// No answer in time.
    Timeout,
    /// Anything else, with the error's text.
    Other(String),
}

/// The catalog the pickers read, as the last answer left it, and the request
/// the next answer must belong to (`permission_popup_request_id`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profiles {
    pub found: Option<Discovery>,
    /// The request the open menu waits for: an answer of another request is
    /// dropped.
    pub pending: Option<u64>,
    issued: u64,
}

impl Profiles {
    pub(super) fn request(&mut self) -> u64 {
        self.issued += 1;
        self.pending = Some(self.issued);
        self.issued
    }
}

/// `configRequirements/read`.
pub fn requirements_request(request_id: RequestId) -> ClientRequest {
    ClientRequest::ConfigRequirementsRead { request_id, params: None }
}

/// One page of `permissionProfile/list` (100 per page, at most 10 pages).
pub fn profiles_request(
    request_id: RequestId,
    cursor: Option<String>,
    cwd: Option<String>,
) -> ClientRequest {
    ClientRequest::PermissionProfileList {
        request_id,
        params: PermissionProfileListParams { cursor, limit: Some(100), cwd },
    }
}

/// The requirements a `configRequirements/read` answer holds.
pub fn requirements(result: serde_json::Value) -> Result<Option<ConfigRequirements>, String> {
    serde_json::from_value::<ConfigRequirementsReadResponse>(result)
        .map(|response| response.requirements)
        .map_err(|err| err.to_string())
}

/// A page of `permissionProfile/list`: its profiles and the next cursor.
pub fn profiles_page(
    result: serde_json::Value,
) -> Result<(Vec<PermissionProfileSummary>, Option<String>), String> {
    serde_json::from_value::<PermissionProfileListResponse>(result)
        .map(|response| (response.data, response.next_cursor))
        .map_err(|err| err.to_string())
}

/// What a profile or built-in mode is called in the history
/// (`display_label`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    /// `Ask for approval`, `Approve for me`, `Full Access`, `Read Only`.
    Mode { preset: Preset, reviewer: Reviewer },
    /// A profile of the server, by its id.
    Profile(String),
}

/// What a final pick applies as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// The legacy list (no thread yet): the approval policy, reviewer and
    /// sandbox of the preset become the turn context
    /// (`approval_preset_actions`).
    Preset { preset: Preset, reviewer: Reviewer },
    /// A profile of the catalog (`SelectPermissionProfile`); a built-in mode
    /// carries its policy and reviewer, a profile of the server's own none.
    Profile {
        id: String,
        approval: Option<Approval>,
        reviewer: Option<Reviewer>,
        label: Label,
    },
}

impl Selection {
    fn full_access(&self) -> bool {
        match self {
            Selection::Preset { preset, .. } => *preset == Preset::FullAccess,
            Selection::Profile { id, .. } => id == BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS,
        }
    }
}

/// The menus of the `/permissions` family on the stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionsMenu {
    /// The legacy list of presets (`open_legacy_permissions_popup`).
    Presets(Current),
    /// `Loading permission profiles…`: the catalog is asked for.
    Loading(Current),
    /// The built-in modes and the server's profiles, read from the catalog
    /// the model holds.
    Profiles(Current),
    /// The catalog could not be read: the message and a `Retry` row.
    Failed { current: Current, failure: Failure },
    /// `Enable full access?` before the pick is applied.
    FullAccess { current: Current, pick: Selection },
}

/// What a row does when chosen, for the view to give it its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A built-in mode (label and description by preset and reviewer).
    Mode { preset: Preset, reviewer: Reviewer },
    /// A profile the server lists; its description is the server's.
    Profile { description: Option<String> },
    /// The active profile the server does not list (`Current permission
    /// profile`).
    Unlisted,
    /// `Loading permission profiles…`.
    Loading,
    Retry,
    /// `Yes, continue anyway`.
    Continue,
    /// `Cancel`.
    Cancel,
}

/// One row of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// What the intent for this row carries: `:profile` for a mode (with
    /// `+auto-review` for `Approve for me`), the profile's id, or one of the
    /// constants above.
    pub id: String,
    /// The profile's id (the row's name) when the row is one of the server's.
    pub name: String,
    pub kind: Kind,
    pub current: bool,
    pub disabled: Option<Disabled>,
}

/// What the view says above the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heading {
    /// `Update Model Permissions`, with a line under it when the catalog
    /// came back empty or could not be read.
    Permissions { notice: Option<Notice> },
    /// `Enable full access?`, with the warning; a cyber model's says more.
    FullAccess { cyber: Option<Recommend> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// `No permission profiles returned by the server.`
    NoProfiles,
    Failed(Failure),
}

/// What the cyber model's warning recommends instead of full access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recommend {
    /// `Approve for me`, when auto-review is available.
    AutoReview,
    /// `Ask for approval`.
    Ask,
}

/// A menu as rows: what `PermissionsMenu` and the catalog make of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub heading: Heading,
    pub rows: Vec<Row>,
}

/// What a choice does to the stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// Nothing: the id is not a row, or the row is disabled.
    None,
    /// This menu is dismissed and the next one takes its place.
    Replace(PermissionsMenu),
    /// The pick is final: every menu closes.
    Done(Selection),
    /// The confirmation was declined: the permissions menu opens again from
    /// the start (`OpenPermissionsPopup`).
    Reopen(Current),
    /// `Retry`: the catalog is asked for again.
    Retry(Current),
}

fn mode_id(preset: Preset, reviewer: Reviewer) -> String {
    match reviewer {
        Reviewer::User => preset.profile().to_string(),
        Reviewer::AutoReview => [preset.profile(), AUTO_REVIEW].concat(),
    }
}

fn mode_row(
    current: &Current,
    preset: Preset,
    reviewer: Reviewer,
    is_current: bool,
    disabled: Option<Disabled>,
) -> Row {
    let _ = current;
    Row {
        id: mode_id(preset, reviewer),
        name: String::new(),
        kind: Kind::Mode { preset, reviewer },
        current: is_current,
        disabled,
    }
}

impl PermissionsMenu {
    /// The thread's snapshot the menu opened with.
    pub fn current(&self) -> &Current {
        match self {
            PermissionsMenu::Presets(current)
            | PermissionsMenu::Loading(current)
            | PermissionsMenu::Profiles(current)
            | PermissionsMenu::Failed { current, .. }
            | PermissionsMenu::FullAccess { current, .. } => current,
        }
    }

    /// The menu as rows.
    pub fn page(&self, profiles: &Profiles) -> Page {
        let permissions = |notice| Heading::Permissions { notice };
        match self {
            PermissionsMenu::Presets(current) => {
                Page { heading: permissions(None), rows: presets(current) }
            }
            PermissionsMenu::Loading(_) => Page {
                heading: permissions(None),
                rows: vec![Row {
                    id: String::new(),
                    name: String::new(),
                    kind: Kind::Loading,
                    current: false,
                    disabled: Some(Disabled::Idle),
                }],
            },
            PermissionsMenu::Profiles(current) => {
                let empty = Discovery::default();
                let discovery = profiles.found.as_ref().unwrap_or(&empty);
                let rows = profile_rows(current, discovery);
                let notice = discovery.profiles.is_empty().then_some(Notice::NoProfiles);
                Page { heading: permissions(notice), rows }
            }
            PermissionsMenu::Failed { failure, .. } => Page {
                heading: permissions(Some(Notice::Failed(failure.clone()))),
                rows: vec![Row {
                    id: RETRY.to_string(),
                    name: String::new(),
                    kind: Kind::Retry,
                    current: false,
                    disabled: None,
                }],
            },
            PermissionsMenu::FullAccess { current, .. } => {
                let cyber = current.cyber.then_some(if current.auto_available {
                    Recommend::AutoReview
                } else {
                    Recommend::Ask
                });
                let row = |id: &str, kind| Row {
                    id: id.to_string(),
                    name: String::new(),
                    kind,
                    current: false,
                    disabled: None,
                };
                Page {
                    heading: Heading::FullAccess { cyber },
                    rows: vec![row(CONTINUE, Kind::Continue), row(CANCEL, Kind::Cancel)],
                }
            }
        }
    }

    /// A row was chosen.
    pub(super) fn choose(&self, profiles: &Profiles, id: &str) -> Step {
        let page = self.page(profiles);
        let Some(row) = page.rows.iter().find(|row| row.id == id) else {
            return Step::None;
        };
        if row.disabled.is_some() {
            return Step::None;
        }
        match (self, &row.kind) {
            (PermissionsMenu::Presets(current), Kind::Mode { preset, reviewer }) => {
                let pick = Selection::Preset { preset: *preset, reviewer: *reviewer };
                settle(current, pick)
            }
            (PermissionsMenu::Profiles(current), Kind::Mode { preset, reviewer }) => {
                let pick = Selection::Profile {
                    id: preset.profile().to_string(),
                    approval: Some(preset.approval()),
                    reviewer: Some(*reviewer),
                    label: Label::Mode { preset: *preset, reviewer: *reviewer },
                };
                settle(current, pick)
            }
            (PermissionsMenu::Profiles(current), Kind::Profile { .. }) => {
                let pick = Selection::Profile {
                    id: row.id.clone(),
                    approval: None,
                    reviewer: None,
                    label: Label::Profile(row.id.clone()),
                };
                settle(current, pick)
            }
            (PermissionsMenu::Failed { current, .. }, Kind::Retry) => Step::Retry(current.clone()),
            (PermissionsMenu::FullAccess { pick, .. }, Kind::Continue) => Step::Done(pick.clone()),
            (PermissionsMenu::FullAccess { current, .. }, Kind::Cancel) => {
                Step::Reopen(current.clone())
            }
            _ => Step::None,
        }
    }
}

/// A pick of the full-access preset asks first (`requires_confirmation`);
/// the rest apply.
fn settle(current: &Current, pick: Selection) -> Step {
    if pick.full_access() {
        Step::Replace(PermissionsMenu::FullAccess { current: current.clone(), pick })
    } else {
        Step::Done(pick)
    }
}

/// `open_legacy_permissions_popup`: Ask for approval, Approve for me (when the
/// feature is on), Full Access; Read Only is listed on Windows only.
fn presets(current: &Current) -> Vec<Row> {
    let blocked = |preset: Preset| {
        current
            .limits
            .approvals
            .iter()
            .find(|limit| limit.approval == preset.approval())
            .map(|limit| Disabled::Reason(limit.reason.clone()))
    };
    let reason = |text: &Option<String>| text.clone().map(Disabled::Reason);
    let mut rows = Vec::new();
    let windows = cfg!(target_os = "windows");
    for preset in [Preset::ReadOnly, Preset::Auto, Preset::FullAccess] {
        if preset == Preset::ReadOnly && !windows {
            continue;
        }
        match preset {
            Preset::Auto => {
                rows.push(mode_row(
                    current,
                    preset,
                    Reviewer::User,
                    current.reviewer == Reviewer::User && current.matches == Some(preset),
                    blocked(preset).or_else(|| reason(&current.limits.reviewer_user)),
                ));
                if current.guardian {
                    rows.push(mode_row(
                        current,
                        preset,
                        Reviewer::AutoReview,
                        current.reviewer == Reviewer::AutoReview
                            && (current.matches == Some(preset)
                                || (current.approval == Some(Approval::OnRequest)
                                    && current.auto_required)),
                        blocked(preset).or_else(|| reason(&current.limits.reviewer_auto)),
                    ));
                }
            }
            _ => rows.push(mode_row(
                current,
                preset,
                Reviewer::User,
                current.matches == Some(preset),
                blocked(preset).or_else(|| reason(&current.limits.reviewer_user)),
            )),
        }
    }
    rows
}

/// `open_permission_profiles_popup`: the built-in modes in the oracle's
/// order, then the server's own profiles; an empty catalog leaves only the
/// active profile.
fn profile_rows(current: &Current, discovery: &Discovery) -> Vec<Row> {
    let active = current.profile.as_deref();
    let mut rows = Vec::new();
    if !discovery.profiles.is_empty() {
        let mut modes = vec![(Preset::Auto, Reviewer::User)];
        if current.guardian {
            modes.push((Preset::Auto, Reviewer::AutoReview));
        }
        modes.push((Preset::FullAccess, Reviewer::User));
        modes.push((Preset::ReadOnly, Reviewer::User));
        for (preset, reviewer) in modes {
            let approval = preset.approval();
            let is_current = active.map_or(current.matches == Some(preset), |active| {
                active == preset.profile()
            }) && current.approval == Some(approval)
                && current.reviewer == reviewer;
            rows.push(mode_row(
                current,
                preset,
                reviewer,
                is_current,
                discovery.disabled(preset.profile(), Some(approval), Some(reviewer)),
            ));
        }
        rows.extend(
            discovery
                .profiles
                .iter()
                .filter(|profile| !profile.id.starts_with(':'))
                .map(|profile| Row {
                    id: profile.id.clone(),
                    name: profile.id.clone(),
                    kind: Kind::Profile { description: profile.description.clone() },
                    current: active == Some(profile.id.as_str()),
                    disabled: discovery.disabled(&profile.id, None, None),
                }),
        );
    }
    if let Some(id) = active
        && !discovery.profiles.iter().any(|profile| profile.id == id)
        && !rows.iter().any(|row| row.current)
    {
        rows.push(Row {
            id: id.to_string(),
            name: id.to_string(),
            kind: Kind::Unlisted,
            current: true,
            disabled: Some(Disabled::NotOnServer),
        });
    }
    rows
}
