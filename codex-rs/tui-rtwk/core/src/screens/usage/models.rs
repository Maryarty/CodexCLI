//! Private analytics display types, independent of the app-server wire protocol.

use codex_protocol::account::PlanType;
use codex_protocol::auth::KnownPlan;


/// Billing families match the App's consumer, business, and workspace scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Consumer,
    Business,
    Enterprise,
    Unknown,
}

impl From<Option<PlanType>> for AccountKind {
    fn from(plan: Option<PlanType>) -> Self {
        match plan {
            None | Some(PlanType::Unknown) => Self::Unknown,
            Some(PlanType::Sci | PlanType::SciPlus) => Self::Business,
            Some(plan) if plan.is_team_like() => Self::Business,
            Some(plan) if plan.is_workspace_account() => Self::Enterprise,
            Some(_) => Self::Consumer,
        }
    }
}

/// Match the existing CLI thread-usage capability without widening other workspace reports.
pub fn thread_usage_supported(plan: Option<PlanType>) -> bool {
    matches!(
        plan,
        Some(
            PlanType::Business
                | PlanType::EnterpriseCbpUsageBased
                | PlanType::EnterpriseCbpAutomation
        )
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAnalyticsReport {
    Usage,
    Credits,
    Messages,
    Plugins,
    Skills,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAnalyticsGrouping {
    Feature,
    Model,
    Surface,
    TaskStart,
    Speed,
    Reasoning,
    TokenType,
}

impl AccountAnalyticsGrouping {
    /// Credit breakdowns supported by the account's billing report.
    pub fn credit_groupings(plan: Option<PlanType>) -> &'static [Self] {
        match AccountKind::from(plan) {
            AccountKind::Business => &[Self::Surface, Self::Model, Self::Speed],
            AccountKind::Enterprise => &[Self::Surface, Self::Model, Self::Speed, Self::Reasoning],
            AccountKind::Consumer => &[Self::Surface],
            AccountKind::Unknown => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountAnalyticsUnit {
    RelativeUsage,
    Credits,
    Count,
    Tokens,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AccountAnalyticsHistory {
    pub unit: AccountAnalyticsUnit,
    /// Source freshness in Unix seconds, when provided. Not a completeness watermark.
    pub updated_at: Option<i64>,
    pub data: Vec<AccountAnalyticsDay>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AccountAnalyticsDay {
    /// Source reporting date; display formatting belongs to the UI.
    pub date: super::date::NaiveDate,
    /// Full daily denominator, including non-task usage when grouped by task start.
    pub total: f64,
    pub values: Vec<AccountAnalyticsValue>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AccountAnalyticsValue {
    pub key: String,
    pub label: String,
    pub value: f64,
}

// Every number in a history was checked finite by `normalize`/`tokens` (the
// seed form is held to it by the fixtures), so the equality is total.
impl Eq for AccountAnalyticsHistory {}

/// The subscription's name on the dashboard heading and the profile block
/// (oracle: `subscription.rs`, `SubscriptionDisplay::Analytics`).
pub fn plan_label(plan: PlanType) -> &'static str {
    match plan {
        PlanType::Free => "Free",
        PlanType::Go => "Go",
        PlanType::Plus => "Plus",
        PlanType::Pro => "Pro 200",
        PlanType::ProLite => "Pro 100",
        PlanType::ProMax => "Pro 500",
        PlanType::Team
        | PlanType::SelfServeBusinessUsageBased
        | PlanType::Business
        | PlanType::SelfServeBusinessProLite => "Business",
        PlanType::EnterpriseCbpAutomation
        | PlanType::Enterprise
        | PlanType::Ent26
        | PlanType::EnterpriseCbpUsageBased => "Enterprise",
        PlanType::Edu | PlanType::EduPlus | PlanType::EduPro => "Education",
        PlanType::Sci => KnownPlan::Sci.display_name(),
        PlanType::SciPlus => KnownPlan::SciPlus.display_name(),
        PlanType::Unknown => "Account",
    }
}
