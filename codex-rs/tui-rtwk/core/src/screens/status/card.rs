//! `StatusCard`: what `/status` shows, as the raw facts of the oracle's
//! `StatusHistoryCell::new` (`status/card.rs`): the session's configuration,
//! the account, the token counters and the rate-limit snapshots. Every shown
//! value is derived from these by the methods here and the view's string
//! table; nothing is a stored copy of a derived value. A card is a value: a
//! refresh makes a new one, it is never changed in place.

use codex_protocol::account::PlanType;

use super::helpers::{self, AgentsSource, PlanLabel};
use super::limits::{self, Limits};
use super::{Approval, Context, Tokens};

/// The connection to a server other than the in-process one (oracle
/// `RemoteConnectionStatus`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Remote {
    pub address: String,
    pub version: String,
    /// The local background daemon: the row says so instead of the address.
    #[serde(default)]
    pub local_daemon: bool,
}

/// The reasoning settings the model row details (oracle `config_entries`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Reasoning {
    pub effort: String,
    /// `None` when the connected server owns the setting (remote sessions).
    #[serde(default)]
    pub summary: Option<String>,
}

/// One part of the model row's parenthesis (oracle `compose_model_display`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelDetail {
    Reasoning(String),
    SummariesOff,
    Summaries(String),
}

// oracle: ../status/helpers.rs fn compose_model_display (adapted: typed, no prose)
pub fn model_details(reasoning: &Reasoning) -> Vec<ModelDetail> {
    let mut details = vec![ModelDetail::Reasoning(reasoning.effort.to_ascii_lowercase())];
    if let Some(summary) = &reasoning.summary {
        let summary = summary.trim();
        if summary.eq_ignore_ascii_case("none") || summary.eq_ignore_ascii_case("off") {
            details.push(ModelDetail::SummariesOff);
        } else if !summary.is_empty() {
            details.push(ModelDetail::Summaries(summary.to_ascii_lowercase()));
        }
    }
    details
}

/// What the sandbox of the session is, in the words of the oracle's
/// `summarize_permission_profile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxKind {
    ReadOnly,
    Workspace,
    Custom,
    DangerFullAccess,
    ExternalSandbox,
}

/// The permission facts of the session (oracle `status_permissions_label`'s
/// inputs).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Permissions {
    /// The id of the active permission profile (`:workspace`, a user's own).
    #[serde(default)]
    pub profile: Option<String>,
    pub sandbox: SandboxKind,
    /// The sandbox lets the network through.
    #[serde(default)]
    pub network: bool,
    /// The profile is the disabled one (no sandbox at all).
    #[serde(default)]
    pub disabled: bool,
    pub approval: Approval,
    /// The auto-review reviewer answers approvals.
    #[serde(default)]
    pub auto_review: bool,
    /// Workspace roots other than the directory.
    #[serde(default)]
    pub extra_roots: Vec<String>,
}

/// How approvals read (oracle `status_approval_label`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalLabel {
    ApproveForMe,
    Ask,
    Policy(Approval),
}

/// A sandbox as the custom and profile labels spell it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxText {
    pub sandbox: SandboxKind,
    pub network: bool,
    /// The extra roots, spelled after a workspace sandbox only.
    pub roots: Vec<String>,
}

/// The permissions row (oracle `status_permissions_label`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionsLabel {
    ReadOnly {
        network: bool,
        approval: ApprovalLabel,
    },
    Workspace {
        network: bool,
        roots: Vec<String>,
        approval: ApprovalLabel,
    },
    FullAccess,
    NoSandbox {
        approval: ApprovalLabel,
    },
    Profile {
        id: String,
        sandbox: SandboxText,
        approval: ApprovalLabel,
    },
    Custom {
        sandbox: SandboxText,
        approval: ApprovalLabel,
    },
}

impl Permissions {
    // oracle: status/card.rs fn status_permissions_label (adapted: typed, no prose)
    pub fn label(&self) -> PermissionsLabel {
        let approval = match (self.approval, self.auto_review) {
            (Approval::OnRequest, true) => ApprovalLabel::ApproveForMe,
            (Approval::OnRequest, false) => ApprovalLabel::Ask,
            (policy, _) => ApprovalLabel::Policy(policy),
        };
        let workspace = self.sandbox == SandboxKind::Workspace;
        let plain = |sandbox| !self.network && self.sandbox == sandbox;
        let sandbox = SandboxText {
            sandbox: self.sandbox,
            network: self.network,
            roots: if workspace {
                self.extra_roots.clone()
            } else {
                Vec::new()
            },
        };
        match self.profile.as_deref() {
            Some(":read-only") => {
                return PermissionsLabel::ReadOnly {
                    network: self.sandbox == SandboxKind::ReadOnly && self.network,
                    approval,
                };
            }
            Some(":workspace") if workspace => {
                return PermissionsLabel::Workspace {
                    network: self.network,
                    roots: self.extra_roots.clone(),
                    approval,
                };
            }
            Some(":workspace") => {}
            Some(":danger-full-access") if self.disabled => {
                return if self.approval == Approval::Never {
                    PermissionsLabel::FullAccess
                } else {
                    PermissionsLabel::NoSandbox { approval }
                };
            }
            Some(id) => {
                return PermissionsLabel::Profile {
                    id: id.to_string(),
                    sandbox,
                    approval,
                };
            }
            None => {}
        }
        if plain(SandboxKind::ReadOnly) {
            return PermissionsLabel::ReadOnly {
                network: false,
                approval,
            };
        }
        if self.approval == Approval::OnRequest && plain(SandboxKind::Workspace) {
            return PermissionsLabel::Workspace {
                network: false,
                roots: self.extra_roots.clone(),
                approval,
            };
        }
        if self.approval == Approval::Never && self.disabled {
            return PermissionsLabel::FullAccess;
        }
        PermissionsLabel::Custom { sandbox, approval }
    }
}

/// Who the card says is signed in (oracle `StatusAccountDisplay`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Account {
    /// The workspace is already dropped by the host when the model provider
    /// does not use the first-party auth path.
    ChatGpt {
        #[serde(default)]
        email: Option<String>,
        #[serde(default)]
        plan: Option<PlanType>,
        #[serde(default)]
        workspace: Option<String>,
    },
    ApiKey,
}

impl Account {
    /// The plan's label, for a ChatGPT account that has a plan.
    pub fn plan_label(&self) -> Option<PlanLabel> {
        match self {
            Account::ChatGpt {
                plan: Some(plan), ..
            } => Some(helpers::plan_label(*plan)),
            _ => None,
        }
    }
}

/// What `/status` shows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct StatusCard {
    pub version: String,
    #[serde(default)]
    pub remote: Option<Remote>,
    pub model: String,
    /// The model's slug, as the thread runs it: what the copy menu copies.
    #[serde(default)]
    pub model_id: Option<String>,
    /// `None` when the model's wire protocol has no reasoning settings.
    #[serde(default)]
    pub reasoning: Option<Reasoning>,
    /// The model provider's id, when the session names one.
    #[serde(default)]
    pub provider: Option<String>,
    pub directory: String,
    #[serde(default)]
    pub home: Option<String>,
    pub permissions: Permissions,
    #[serde(default)]
    pub agents: Vec<AgentsSource>,
    #[serde(default)]
    pub account: Option<Account>,
    #[serde(default)]
    pub thread_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub forked_from: Option<String>,
    #[serde(default)]
    pub collaboration_mode: Option<String>,
    /// The provider needs OpenAI auth: the card links the usage page.
    #[serde(default)]
    pub usage_link: bool,
    pub tokens: Tokens,
    #[serde(default)]
    pub context: Option<Context>,
    #[serde(default)]
    pub limits: Limits,
    /// The 24-hour clock is the user's preference (reset times).
    #[serde(default)]
    pub hour24: bool,
    /// The number of the `/status` that made the card (the request number of its refresh, when
    /// it has one): what the card's thread usage is kept under (`UsageInterest::Card`).
    #[serde(default)]
    pub number: u64,
}

impl StatusCard {
    /// The directory as shown: home collapsed to `~`.
    pub fn directory_text(&self) -> String {
        helpers::format_directory_display(&self.directory, self.home.as_deref())
    }

    /// The loaded instruction files as shown; none when empty.
    pub fn agents_summary(&self) -> Vec<String> {
        helpers::compose_agents_summary(&self.directory, self.home.as_deref(), &self.agents)
    }

    /// The limit section's rows, or the notice that stands for them.
    pub fn limit_data(&self) -> limits::Data {
        self.limits.data()
    }

    /// The model row's details, none when the wire has no reasoning settings.
    pub fn model_details(&self) -> Vec<ModelDetail> {
        self.reasoning.as_ref().map(model_details).unwrap_or_default()
    }

    /// The token counters row is hidden for ChatGPT subscribers.
    pub fn shows_token_usage(&self) -> bool {
        !matches!(self.account, Some(Account::ChatGpt { .. }))
    }

    /// The `Forked from` row needs the session's own id to stand beside.
    pub fn forked_from(&self) -> Option<&str> {
        self.session_id.as_ref().and(self.forked_from.as_deref())
    }

    /// The thread name, unless it is empty.
    pub fn thread_name(&self) -> Option<&str> {
        self.thread_name.as_deref().filter(|name| !name.is_empty())
    }
}
