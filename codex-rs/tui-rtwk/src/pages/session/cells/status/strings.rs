//! The strings of `status` (S19): a table of `&'static str` fields and typed
//! messages per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte (`status/card.rs`, `status/helpers.rs`,
//! `status/rate_limits.rs`). A language is one more `const` here, registered
//! with `with_ru(&RU)` on `TABLES`; a screen never changes for it. Nothing
//! else in this directory holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::screens::status::{
    Account, Approval, ApprovalLabel, CreditsText, ModelDetail, PermissionsLabel, PlanLabel,
    ResetTime, RowLabel, SandboxKind, SandboxText, WindowName,
};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    pub num: NumSyms,
    /// The command the card answers, the dialog's title.
    pub command: &'static str,
    pub model: &'static str,
    pub directory: &'static str,
    pub permissions: &'static str,
    pub agents: &'static str,
    pub token_usage: &'static str,
    pub context_window: &'static str,
    pub agents_none: &'static str,
    pub reasoning: Msg<1>,
    pub summaries: Msg<1>,
    pub details_separator: &'static str,
    pub approval_ask: &'static str,
    pub approval_auto: &'static str,
    pub am: &'static str,
    pub pm: &'static str,
    // ---- the cell in the transcript (`cell.rs`)
    pub title_mark: &'static str,
    pub title_name: &'static str,
    pub title_version: Msg<1>,
    pub usage_visit: &'static str,
    pub usage_url: &'static str,
    pub usage_for: &'static str,
    pub usage_info: &'static str,
    pub server: &'static str,
    pub server_local: &'static str,
    /// A dim detail after a value: ` (v0.133.0)`.
    pub parenthesised: Msg<1>,
    pub model_provider: &'static str,
    pub account: &'static str,
    pub workspace: &'static str,
    pub thread_name: &'static str,
    pub collaboration_mode: &'static str,
    pub session: &'static str,
    pub forked_from: &'static str,
    pub limits: &'static str,
    pub warning: &'static str,
    pub credits: &'static str,
    pub monthly_credit_limit: &'static str,
    pub summaries_off: &'static str,
    pub tokens_head: Msg<1>,
    pub tokens_detail: Msg<2>,
    pub context_head: Msg<1>,
    pub context_detail: Msg<2>,
    pub limits_unavailable: &'static str,
    pub limits_missing: &'static str,
    pub limits_missing_refreshing: &'static str,
    pub limits_stale: &'static str,
    pub limits_stale_refreshing: &'static str,
    pub percent_left: Msg<1>,
    pub limit_label: Msg<1>,
    pub bucket_limit_label: Msg<2>,
    pub window_5h: &'static str,
    pub window_daily: &'static str,
    pub window_weekly: &'static str,
    pub window_monthly: &'static str,
    pub window_annual: &'static str,
    pub window_usage: &'static str,
    pub window_secondary_usage: &'static str,
    pub reset_at: Msg<1>,
    pub time_12: Msg<3>,
    pub time_24: Msg<2>,
    pub on_date: Msg<3>,
    pub months: [&'static str; 12],
    pub credits_unlimited: &'static str,
    pub credits_balance: Msg<1>,
    pub credits_available: &'static str,
    pub spend_used: Msg<2>,
    pub account_chatgpt: &'static str,
    pub account_with_plan: Msg<2>,
    pub account_api_key: &'static str,
    /// The plan names, in the order of `PlanLabel`.
    pub plans: [&'static str; 16],
    pub permissions_read_only: Msg<1>,
    pub permissions_read_only_network: Msg<1>,
    pub permissions_workspace: Msg<2>,
    pub permissions_workspace_network: Msg<2>,
    pub permissions_full_access: &'static str,
    pub permissions_no_sandbox: Msg<1>,
    pub permissions_profile: Msg<3>,
    pub permissions_custom: Msg<2>,
    pub roots: Msg<1>,
    pub roots_separator: &'static str,
    pub sandbox_read_only: &'static str,
    pub sandbox_read_only_network: &'static str,
    pub sandbox_workspace: &'static str,
    pub sandbox_workspace_network: &'static str,
    pub sandbox_custom: &'static str,
    pub sandbox_custom_network: &'static str,
    pub sandbox_danger: &'static str,
    pub sandbox_external: &'static str,
    pub policy_never: &'static str,
    pub policy_on_failure: &'static str,
    pub policy_untrusted: &'static str,
    pub policy_granular: &'static str,
    pub policy_on_request: &'static str,
    pub space: &'static str,
    /// Between a label and its value when a row flows as a paragraph.
    pub gap: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "/status card",
    num: NumSyms::ENGLISH,
    command: "/status",
    model: "Model:",
    directory: "Directory:",
    permissions: "Permissions:",
    agents: "Agents.md:",
    token_usage: "Token usage:",
    context_window: "Context window:",
    agents_none: "<none>",
    reasoning: msg!("reasoning {0}"),
    summaries: msg!("summaries {0}"),
    details_separator: ", ",
    approval_ask: "Ask for approval",
    approval_auto: "Approve for me",
    am: "AM",
    pm: "PM",
    title_mark: ">_ ",
    title_name: "OpenAI Codex",
    title_version: msg!(" (v{0})"),
    usage_visit: "Visit ",
    usage_url: "https://chatgpt.com/settings/usage",
    usage_for: " for up-to-date",
    usage_info: "information on rate limits and credits",
    server: "Server:",
    server_local: "Local background server",
    parenthesised: msg!(" ({0})"),
    model_provider: "Model provider:",
    account: "Account:",
    workspace: "ChatGPT workspace:",
    thread_name: "Thread name:",
    collaboration_mode: "Collaboration mode:",
    session: "Session:",
    forked_from: "Forked from:",
    limits: "Limits:",
    warning: "Warning:",
    credits: "Credits:",
    monthly_credit_limit: "Monthly credit limit:",
    summaries_off: "summaries off",
    tokens_head: msg!("{0} total "),
    tokens_detail: msg!(" ({0} input + {1} output)"),
    context_head: msg!("{0}% left"),
    context_detail: msg!(" ({0} used / {1})"),
    limits_unavailable: "not available for this account",
    limits_missing: "data not available yet",
    limits_missing_refreshing: "refresh requested; run /status again shortly.",
    limits_stale: "limits may be stale - start new turn to refresh.",
    limits_stale_refreshing: "limits may be stale - run /status again shortly.",
    percent_left: msg!("{0}% left"),
    limit_label: msg!("{0} limit:"),
    bucket_limit_label: msg!("{0} {1} limit:"),
    window_5h: "5h",
    window_daily: "Daily",
    window_weekly: "Weekly",
    window_monthly: "Monthly",
    window_annual: "Annual",
    window_usage: "Usage",
    window_secondary_usage: "Secondary usage",
    reset_at: msg!("(resets {0})"),
    time_12: msg!("{0}:{1} {2}"),
    time_24: msg!("{0}:{1}"),
    on_date: msg!("{0} on {1} {2}"),
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    credits_unlimited: "Unlimited",
    credits_balance: msg!("{0} credits"),
    credits_available: "Available",
    spend_used: msg!("{0} of {1} credits used"),
    account_chatgpt: "ChatGPT",
    account_with_plan: msg!("{0} ({1})"),
    account_api_key: "API key configured (run codex login to use ChatGPT)",
    plans: [
        "Free",
        "Go",
        "Plus",
        "Pro 200",
        "Pro 100",
        "Pro 500",
        "Business",
        "Business Premium",
        "Enterprise",
        "Enterprise (Automation)",
        "Edu",
        "Edu Plus",
        "Edu Pro",
        "Academic Researchers",
        "Academic Researchers Plus",
        "Unknown",
    ],
    permissions_read_only: msg!("Read Only ({0})"),
    permissions_read_only_network: msg!("Read Only with network access ({0})"),
    permissions_workspace: msg!("Workspace{0} ({1})"),
    permissions_workspace_network: msg!("Workspace with network access{0} ({1})"),
    permissions_full_access: "Full Access",
    permissions_no_sandbox: msg!("No Sandbox ({0})"),
    permissions_profile: msg!("Profile {0} ({1}, {2})"),
    permissions_custom: msg!("Custom ({0}, {1})"),
    roots: msg!(" [{0}]"),
    roots_separator: ", ",
    sandbox_read_only: "read-only",
    sandbox_read_only_network: "read-only with network access",
    sandbox_workspace: "workspace",
    sandbox_workspace_network: "workspace with network access",
    sandbox_custom: "custom permissions",
    sandbox_custom_network: "custom permissions with network access",
    sandbox_danger: "danger-full-access",
    sandbox_external: "external-sandbox",
    policy_never: "never",
    policy_on_failure: "on-failure",
    policy_untrusted: "untrusted",
    policy_granular: "granular",
    policy_on_request: "on-request",
    space: " ",
    gap: "  ",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}

impl Strings {
    pub fn reasoning<'a>(&'a self, effort: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.reasoning, [Arg::Text(effort)], &self.num)
    }

    pub fn summaries<'a>(&'a self, summary: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.summaries, [Arg::Text(summary)], &self.num)
    }

    // ---- the cell's derived texts: each is a message of the table filled with
    // the card's facts.

    pub fn title_version<'a>(&'a self, version: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.title_version, [Arg::Text(version)], &self.num)
    }

    pub fn parenthesised<'a>(&'a self, text: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.parenthesised, [Arg::Text(text)], &self.num)
    }

    pub fn tokens_head<'a>(&'a self, total: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.tokens_head, [Arg::Text(total)], &self.num)
    }

    pub fn tokens_detail<'a>(&'a self, input: &'a str, output: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.tokens_detail,
            [Arg::Text(input), Arg::Text(output)],
            &self.num,
        )
    }

    pub fn context_head(&self, percent: i64) -> Fmt<'_, 1> {
        Fmt::new(
            self.context_head,
            [Arg::Int(u64::try_from(percent).unwrap_or(0))],
            &self.num,
        )
    }

    pub fn context_detail<'a>(&'a self, used: &'a str, window: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.context_detail,
            [Arg::Text(used), Arg::Text(window)],
            &self.num,
        )
    }

    pub fn percent_left(&self, percent: i32) -> Fmt<'_, 1> {
        Fmt::new(
            self.percent_left,
            [Arg::Int(u64::try_from(percent).unwrap_or(0))],
            &self.num,
        )
    }

    pub fn window_name(&self, name: WindowName) -> &'static str {
        match name {
            WindowName::FiveHours => self.window_5h,
            WindowName::Daily => self.window_daily,
            WindowName::Weekly => self.window_weekly,
            WindowName::Monthly => self.window_monthly,
            WindowName::Annual => self.window_annual,
            WindowName::Generic { secondary: false } => self.window_usage,
            WindowName::Generic { secondary: true } => self.window_secondary_usage,
        }
    }

    /// The label of a limit row, its colon included.
    pub fn row_label(&self, label: &RowLabel) -> String {
        let one = |msg: Msg<1>, text: &str| Fmt::new(msg, [Arg::Text(text)], &self.num).to_string();
        match label {
            RowLabel::Bucket(bucket) => one(self.limit_label, bucket),
            RowLabel::Window { bucket: None, name } => {
                one(self.limit_label, self.window_name(*name))
            }
            RowLabel::Window {
                bucket: Some(bucket),
                name,
            } => Fmt::new(
                self.bucket_limit_label,
                [Arg::Text(bucket), Arg::Text(self.window_name(*name))],
                &self.num,
            )
            .to_string(),
            RowLabel::Credits => self.credits.to_string(),
            RowLabel::MonthlyCreditLimit => self.monthly_credit_limit.to_string(),
        }
    }

    pub fn credits_text(&self, text: &CreditsText) -> String {
        match text {
            CreditsText::Unlimited => self.credits_unlimited.to_string(),
            CreditsText::Balance(balance) => {
                Fmt::new(self.credits_balance, [Arg::Text(balance)], &self.num).to_string()
            }
            CreditsText::Available => self.credits_available.to_string(),
        }
    }

    pub fn spend_used(&self, used: &str, limit: &str) -> String {
        Fmt::new(
            self.spend_used,
            [Arg::Text(used), Arg::Text(limit)],
            &self.num,
        )
        .to_string()
    }

    /// `(resets 3:14 AM)`: the time in the user's clock, with its day when it
    /// is not the day the limits were read.
    pub fn reset(&self, at: &ResetTime, hour24: bool) -> String {
        let minute = format!("{:02}", at.minute);
        let time = if hour24 {
            let hour = format!("{:02}", at.hour);
            Fmt::new(
                self.time_24,
                [Arg::Text(&hour), Arg::Text(&minute)],
                &self.num,
            )
            .to_string()
        } else {
            let hour = match at.hour % 12 {
                0 => 12,
                hour => hour,
            };
            let meridiem = if at.hour >= 12 { self.pm } else { self.am };
            Fmt::new(
                self.time_12,
                [
                    Arg::Int(u64::from(hour)),
                    Arg::Text(&minute),
                    Arg::Text(meridiem),
                ],
                &self.num,
            )
            .to_string()
        };
        let when = match at.date {
            Some(date) => {
                let month = self
                    .months
                    .get(usize::from(date.month).saturating_sub(1))
                    .copied()
                    .unwrap_or_default();
                Fmt::new(
                    self.on_date,
                    [
                        Arg::Text(&time),
                        Arg::Int(u64::from(date.day)),
                        Arg::Text(month),
                    ],
                    &self.num,
                )
                .to_string()
            }
            None => time,
        };
        Fmt::new(self.reset_at, [Arg::Text(&when)], &self.num).to_string()
    }

    pub fn plan(&self, plan: PlanLabel) -> &'static str {
        self.plans[plan as usize]
    }

    /// The account row's value.
    pub fn account_text(&self, account: &Account) -> String {
        let plan = account.plan_label().map(|plan| self.plan(plan));
        match account {
            Account::ChatGpt { email, .. } => match (email, plan) {
                (Some(email), Some(plan)) => Fmt::new(
                    self.account_with_plan,
                    [Arg::Text(email), Arg::Text(plan)],
                    &self.num,
                )
                .to_string(),
                (Some(email), None) => email.clone(),
                (None, Some(plan)) => plan.to_string(),
                (None, None) => self.account_chatgpt.to_string(),
            },
            Account::ApiKey => self.account_api_key.to_string(),
        }
    }

    pub fn model_detail(&self, detail: &ModelDetail) -> String {
        match detail {
            ModelDetail::Reasoning(effort) => self.reasoning(effort).to_string(),
            ModelDetail::SummariesOff => self.summaries_off.to_string(),
            ModelDetail::Summaries(summary) => self.summaries(summary).to_string(),
        }
    }

    fn approval(&self, approval: ApprovalLabel) -> &'static str {
        match approval {
            ApprovalLabel::ApproveForMe => self.approval_auto,
            ApprovalLabel::Ask => self.approval_ask,
            ApprovalLabel::Policy(Approval::Never) => self.policy_never,
            ApprovalLabel::Policy(Approval::OnFailure) => self.policy_on_failure,
            ApprovalLabel::Policy(Approval::UnlessTrusted) => self.policy_untrusted,
            ApprovalLabel::Policy(Approval::Granular) => self.policy_granular,
            ApprovalLabel::Policy(Approval::OnRequest) => self.policy_on_request,
        }
    }

    fn roots_suffix(&self, roots: &[String]) -> String {
        if roots.is_empty() {
            return String::new();
        }
        let joined = roots.join(self.roots_separator);
        Fmt::new(self.roots, [Arg::Text(&joined)], &self.num).to_string()
    }

    fn sandbox_text(&self, sandbox: &SandboxText) -> String {
        let name = match (sandbox.sandbox, sandbox.network) {
            (SandboxKind::ReadOnly, false) => self.sandbox_read_only,
            (SandboxKind::ReadOnly, true) => self.sandbox_read_only_network,
            (SandboxKind::Workspace, false) => self.sandbox_workspace,
            (SandboxKind::Workspace, true) => self.sandbox_workspace_network,
            (SandboxKind::Custom, false) => self.sandbox_custom,
            (SandboxKind::Custom, true) => self.sandbox_custom_network,
            (SandboxKind::DangerFullAccess, _) => self.sandbox_danger,
            (SandboxKind::ExternalSandbox, _) => self.sandbox_external,
        };
        [name, &self.roots_suffix(&sandbox.roots)].concat()
    }

    /// The permissions row's value.
    pub fn permissions_text(&self, label: &PermissionsLabel) -> String {
        let one = |msg: Msg<1>, a: &str| Fmt::new(msg, [Arg::Text(a)], &self.num).to_string();
        let two = |msg: Msg<2>, a: &str, b: &str| {
            Fmt::new(msg, [Arg::Text(a), Arg::Text(b)], &self.num).to_string()
        };
        match label {
            PermissionsLabel::ReadOnly { network, approval } => {
                let msg = if *network {
                    self.permissions_read_only_network
                } else {
                    self.permissions_read_only
                };
                one(msg, self.approval(*approval))
            }
            PermissionsLabel::Workspace {
                network,
                roots,
                approval,
            } => {
                let msg = if *network {
                    self.permissions_workspace_network
                } else {
                    self.permissions_workspace
                };
                two(msg, &self.roots_suffix(roots), self.approval(*approval))
            }
            PermissionsLabel::FullAccess => self.permissions_full_access.to_string(),
            PermissionsLabel::NoSandbox { approval } => {
                one(self.permissions_no_sandbox, self.approval(*approval))
            }
            PermissionsLabel::Profile {
                id,
                sandbox,
                approval,
            } => Fmt::new(
                self.permissions_profile,
                [
                    Arg::Text(id),
                    Arg::Text(&self.sandbox_text(sandbox)),
                    Arg::Text(self.approval(*approval)),
                ],
                &self.num,
            )
            .to_string(),
            PermissionsLabel::Custom { sandbox, approval } => two(
                self.permissions_custom,
                &self.sandbox_text(sandbox),
                self.approval(*approval),
            ),
        }
    }
}
