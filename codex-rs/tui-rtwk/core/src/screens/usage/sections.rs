//! Account-specific section order, request eligibility and independent report
//! ranges (oracle: `analytics/sections.rs`, `chrome.rs`,
//! `controls.rs` `control_available`). The oracle keeps these on
//! `AnalyticsView` next to its widget state; here they are pure functions of
//! the model, which holds only what chooses the requests.

use codex_protocol::account::PlanType;

use super::Model;
use super::data::GroupName;
use super::date::NaiveDate;
use super::models::{AccountAnalyticsReport, AccountKind};

/// Stable identities preserve selections while account layouts change their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Usage,
    Plugins,
    Credits,
    Chats,
    Activity,
    Skills,
    Plan,
    Summary,
}

impl Section {
    pub const COUNT: usize = 8;

    pub fn report(self) -> Option<AccountAnalyticsReport> {
        use AccountAnalyticsReport as Report;
        match self {
            Self::Usage => Some(Report::Usage),
            Self::Plugins => Some(Report::Plugins),
            Self::Credits => Some(Report::Credits),
            Self::Chats | Self::Plan | Self::Summary => None,
            Self::Activity => Some(Report::Messages),
            Self::Skills => Some(Report::Skills),
        }
    }
}

/// Usage, product activity and tools each share one range across their related sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeGroup {
    Usage,
    Activity,
    Tools,
}

/// The controls of the report row (oracle: `controls.rs` `Control`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Range,
    Group,
    Model,
    TaskMetric,
    Dashboard,
    PlanWindow,
    ZeroCreditGroups,
}

/// Match the existing CLI thread-usage capability without widening other
/// workspace reports (oracle: `models.rs`, kept there).
pub fn visible_for(
    kind: Option<AccountKind>,
    plan: Option<PlanType>,
    plan_enabled: bool,
) -> &'static [Section] {
    match kind {
        Some(AccountKind::Consumer) if plan_enabled => &[
            Section::Summary,
            Section::Usage,
            Section::Plan,
            Section::Activity,
            Section::Plugins,
            Section::Skills,
            Section::Chats,
        ],
        Some(AccountKind::Consumer) => &[
            Section::Summary,
            Section::Usage,
            Section::Activity,
            Section::Plugins,
            Section::Skills,
            Section::Chats,
        ],
        Some(AccountKind::Business | AccountKind::Enterprise)
            if super::models::thread_usage_supported(plan) =>
        {
            &[
                Section::Summary,
                Section::Credits,
                Section::Usage,
                Section::Plugins,
                Section::Skills,
                Section::Chats,
            ]
        }
        Some(AccountKind::Business | AccountKind::Enterprise) => &[
            Section::Summary,
            Section::Credits,
            Section::Usage,
            Section::Plugins,
            Section::Skills,
        ],
        Some(AccountKind::Unknown) => &[Section::Summary],
        None => &[],
    }
}

impl Model {
    pub fn account_kind(&self) -> Option<AccountKind> {
        self.account
            .ready()
            .copied()
            .map(|plan| AccountKind::from(Some(plan)))
    }

    pub fn business(&self) -> bool {
        matches!(
            self.account_kind(),
            Some(AccountKind::Business | AccountKind::Enterprise)
        )
    }

    pub fn visible_sections(&self) -> &'static [Section] {
        visible_for(
            self.account_kind(),
            self.account.ready().copied(),
            self.plan_enabled,
        )
    }

    pub fn range_group(&self, section: Section) -> RangeGroup {
        match section {
            Section::Usage if self.business() => RangeGroup::Activity,
            Section::Usage
            | Section::Credits
            | Section::Chats
            | Section::Plan
            | Section::Summary => RangeGroup::Usage,
            Section::Activity => RangeGroup::Activity,
            Section::Plugins | Section::Skills => RangeGroup::Tools,
        }
    }

    pub fn consumer_attribution(&self) -> bool {
        !self.business() && self.attributed_usage
    }

    /// The groupings the `g` control cycles through on the selected section.
    pub fn group_options(&self) -> Vec<usize> {
        if self.visible_sections().is_empty() {
            return Vec::new();
        }
        match self.section {
            Section::Summary => vec![0, 1, 2],
            Section::Plan => vec![1, 2, 0, 3],
            Section::Usage if self.business() => vec![6, 2],
            Section::Usage if self.consumer_attribution() => vec![1, 2, 0, 3],
            Section::Usage => vec![0, 2],
            Section::Credits => self.credit_groups.clone(),
            Section::Activity => vec![2, 0],
            Section::Plugins | Section::Skills | Section::Chats => Vec::new(),
        }
    }

    pub fn group_name(&self, section: Section, group: usize) -> GroupName {
        if section == Section::Summary {
            return match super::activity::VIEWS[group] {
                super::activity::TokenActivityView::Daily => GroupName::Daily,
                super::activity::TokenActivityView::Weekly => GroupName::Weekly,
                super::activity::TokenActivityView::Cumulative => GroupName::Cumulative,
            };
        }
        if group == 0
            && (section == Section::Credits
                || (section == Section::Usage && !self.consumer_attribution()))
        {
            GroupName::Product
        } else {
            super::data::GROUP_NAMES[group]
        }
    }

    /// Share action eligibility between painted controls, keyboard dispatch
    /// and help; `detail_open` is the Chats detail the widget holds.
    pub fn control_available(&self, control: Control, detail_open: bool) -> bool {
        if self.visible_sections().is_empty() {
            return false;
        }
        match control {
            Control::Range => self.section.report().is_some(),
            Control::Group => self.group_options().len() > 1,
            Control::Model => self.section == Section::Usage && self.business(),
            Control::TaskMetric => self.section == Section::Chats && !self.business(),
            Control::Dashboard => true,
            Control::PlanWindow => self.section == Section::Plan,
            Control::ZeroCreditGroups => {
                self.section == Section::Chats && self.business() && detail_open
            }
        }
    }

    pub fn range_of(&self, section: Section) -> usize {
        self.ranges[self.range_group(section) as usize]
    }

    /// 7 or 30 days.
    pub fn days_of(&self, section: Section) -> u32 {
        if self.range_of(section) == 0 { 7 } else { 30 }
    }

    pub fn section_date_range(&self, section: Section) -> std::ops::RangeInclusive<NaiveDate> {
        let end = self.end_date;
        let span = if self.range_of(section) == 0 { 6 } else { 29 };
        end.minus_days(span)..=end
    }
}
