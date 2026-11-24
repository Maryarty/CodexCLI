//! What the person chose on the `/usage` page: the tab, the range per range group, the group per
//! section, the token report's model filter and the chats' sort (ADR-013: drawn state, held by the
//! page's widget state, never by `Model`). The choice says what to load as `ReportKey`s the page
//! sends in `Intent::Load`; the widget decides "next", the model only answers. `View` reads a
//! `Model` through a `UsageChoice` for the pure derivations of both (the options a control cycles,
//! the dates of a range, the report a tab shows).

use std::ops::{Deref, RangeInclusive};
use std::rc::Rc;

use crate::screens::usage::data::GroupName;
use crate::screens::usage::date::NaiveDate;
use crate::screens::usage::models::{AccountAnalyticsHistory as History, AccountKind};
use crate::screens::usage::sections::{Control, Section};
use crate::screens::usage::{Load, Model, ReportKey, Seed, TokenActivityView};

/// The choice of one open page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageChoice {
    pub section: Section,
    /// The range per `RangeGroup`: 0 is 7 days, 1 is 30.
    pub ranges: [usize; 3],
    /// The group (an index of `data::GROUPINGS`) per section.
    pub groups: [usize; Section::COUNT],
    pub token_model: Option<String>,
    /// The chats' sort, an index of `chats::Metric::ALL`.
    pub chat_metric: usize,
}

impl Default for UsageChoice {
    fn default() -> Self {
        let mut groups = [0; Section::COUNT];
        groups[Section::Usage as usize] = 1;
        groups[Section::Plan as usize] = 1;
        groups[Section::Activity as usize] = 2;
        Self {
            section: Section::Summary,
            ranges: [0; 3],
            groups,
            token_model: None,
            chat_metric: 0,
        }
    }
}

impl UsageChoice {
    /// The choice of a page that opens a session, on the Overview's `view` when
    /// `/usage daily|weekly|cumulative` asked for one.
    pub fn opening(view: Option<TokenActivityView>) -> Self {
        let mut choice = Self::default();
        if let Some(view) = view {
            choice.section = Section::Summary;
            choice.groups[Section::Summary as usize] = view.index();
        }
        choice
    }

    /// What a fixture script chose, over the account it seeded (`model` already settled).
    pub fn seeded(seed: &Seed, model: &Model) -> Self {
        let mut choice = Self::default();
        if let Some(ranges) = seed.ranges {
            choice.ranges = ranges;
        }
        for crate::screens::usage::GroupSeed { section, group } in &seed.groups {
            choice.groups[*section as usize] = *group;
        }
        choice.token_model.clone_from(&seed.token_model);
        let visible = model.visible_sections();
        choice.section = seed
            .section
            .filter(|section| visible.contains(section))
            .or_else(|| visible.first().copied())
            .unwrap_or(Section::Summary);
        choice
    }

    /// The account of a session is known: the choice is made to fit it (oracle: `start_reports`),
    /// and the model filter of the session before is dropped.
    pub fn settled(&mut self, model: &Model) {
        self.token_model = None;
        let visible = model.visible_sections();
        let Some(first) = visible.first() else {
            return;
        };
        if !visible.contains(&self.section) {
            self.section = *first;
        }
        let usage_groups: &[usize] = if model.business() {
            &[6, 2]
        } else {
            &[1, 2, 0, 3]
        };
        if !usage_groups.contains(&self.groups[Section::Usage as usize]) {
            self.groups[Section::Usage as usize] =
                if matches!(model.account_kind(), Some(AccountKind::Business)) {
                    2
                } else {
                    usage_groups[0]
                };
        }
        if visible.contains(&Section::Credits)
            && !model
                .credit_groups
                .contains(&self.groups[Section::Credits as usize])
        {
            self.groups[Section::Credits as usize] = model.credit_groups[0];
        }
    }

    /// What every settled load checks (oracle: `poll_reports`): a consumer account whose usage is
    /// not attributed has no per-surface groups.
    pub fn after_settle(&mut self, model: &Model) {
        if !model.business()
            && !model.consumer_attribution()
            && ![0, 2].contains(&self.groups[Section::Usage as usize])
        {
            self.groups[Section::Usage as usize] = 0;
        }
    }

    pub fn range_of(&self, model: &Model, section: Section) -> usize {
        self.ranges[model.range_group(section) as usize]
    }

    /// 7 or 30.
    pub fn days_of(&self, model: &Model, section: Section) -> u32 {
        if self.range_of(model, section) == 0 {
            7
        } else {
            30
        }
    }

    /// The report this choice asks of `section`, none for a section without a report.
    pub fn key(&self, model: &Model, section: Section) -> Option<ReportKey> {
        section.report()?;
        Some(ReportKey {
            section,
            days: self.days_of(model, section),
            group: self.groups[section as usize],
            model: (section == Section::Usage)
                .then(|| self.token_model.clone())
                .flatten(),
        })
    }

    /// The reports of every visible tab, in tab order: what a session starts with.
    pub fn keys(&self, model: &Model) -> Vec<ReportKey> {
        model
            .visible_sections()
            .iter()
            .filter_map(|section| self.key(model, *section))
            .collect()
    }

    /// A tab (digits, Tab, a click): only a visible one is picked.
    pub fn pick(&mut self, model: &Model, section: Section) {
        if model.visible_sections().contains(&section) {
            self.section = section;
        }
    }

    /// `r`: 7 or 30 days for the tab's range group; the reports to load again.
    pub fn next_range(&mut self, model: &Model) -> Vec<ReportKey> {
        if !self.control_available(model, Control::Range, false) {
            return Vec::new();
        }
        let group = model.range_group(self.section);
        self.ranges[group as usize] ^= 1;
        model
            .visible_sections()
            .iter()
            .filter(|section| model.range_group(**section) == group)
            .filter_map(|section| self.key(model, *section))
            .collect()
    }

    /// `g`: the next grouping of the tab; the reports to load again.
    pub fn next_group(&mut self, model: &Model) -> Vec<ReportKey> {
        if !self.control_available(model, Control::Group, false) {
            return Vec::new();
        }
        let options = self.group_options(model);
        let current = self.groups[self.section as usize];
        let next = options
            .iter()
            .position(|group| *group == current)
            .map_or(0, |index| (index + 1) % options.len());
        self.groups[self.section as usize] = options[next];
        self.key(model, self.section).into_iter().collect()
    }

    /// `m`: the next model of the token report; the reports to load again.
    pub fn next_model(&mut self, model: &Model) -> Vec<ReportKey> {
        if !self.control_available(model, Control::Model, false) {
            return Vec::new();
        }
        let models = &model.token_models;
        self.token_model = match self
            .token_model
            .as_ref()
            .and_then(|current| models.iter().position(|candidate| candidate == current))
        {
            Some(index) => models.get(index + 1).cloned(),
            None => models.first().cloned(),
        };
        self.key(model, Section::Usage).into_iter().collect()
    }

    /// `s`: the next sort of the chats (nothing is loaded again).
    pub fn next_metric(&mut self, model: &Model) {
        if self.control_available(model, Control::TaskMetric, false) {
            let metrics = model.task_metrics();
            let at = metrics
                .iter()
                .position(|metric| *metric == self.task_metric(model))
                .unwrap_or_default();
            self.chat_metric = metrics[(at + 1) % metrics.len()].index();
        }
    }

    pub fn task_metric(&self, model: &Model) -> crate::screens::usage::chats::Metric {
        let metrics = model.task_metrics();
        crate::screens::usage::chats::Metric::ALL
            .get(self.chat_metric)
            .copied()
            .filter(|metric| metrics.contains(metric))
            .unwrap_or(metrics[0])
    }

    /// The groupings the `g` control cycles through on the selected section.
    pub fn group_options(&self, model: &Model) -> Vec<usize> {
        if model.visible_sections().is_empty() {
            return Vec::new();
        }
        match self.section {
            Section::Summary => vec![0, 1, 2],
            Section::Plan => vec![1, 2, 0, 3],
            Section::Usage if model.business() => vec![6, 2],
            Section::Usage if model.consumer_attribution() => vec![1, 2, 0, 3],
            Section::Usage => vec![0, 2],
            Section::Credits => model.credit_groups.clone(),
            Section::Activity => vec![2, 0],
            Section::Plugins | Section::Skills | Section::Chats => Vec::new(),
        }
    }

    /// Share action eligibility between painted controls, keyboard dispatch and help;
    /// `detail_open` is the Chats detail the widget holds.
    pub fn control_available(&self, model: &Model, control: Control, detail_open: bool) -> bool {
        if model.visible_sections().is_empty() {
            return false;
        }
        match control {
            Control::Range => self.section.report().is_some(),
            Control::Group => self.group_options(model).len() > 1,
            Control::Model => self.section == Section::Usage && model.business(),
            Control::TaskMetric => self.section == Section::Chats && !model.business(),
            Control::Dashboard => true,
            Control::PlanWindow => self.section == Section::Plan,
            Control::ZeroCreditGroups => {
                self.section == Section::Chats && model.business() && detail_open
            }
        }
    }

    pub fn section_date_range(&self, model: &Model, section: Section) -> RangeInclusive<NaiveDate> {
        let end = model.end_date;
        let span = if self.range_of(model, section) == 0 {
            6
        } else {
            29
        };
        end.minus_days(span)..=end
    }
}

/// A model and the choice made on it, owned: what a widget closure keeps to draw from later.
#[derive(Clone, Debug, PartialEq)]
pub struct Chosen {
    pub model: Model,
    pub sel: UsageChoice,
}

impl Chosen {
    pub fn view(&self) -> View<'_> {
        View::new(&self.model, &self.sel)
    }
}

/// A model read through the choice made on it: what the page draws from. It derefs to the
/// `Model`, so the facts read as before; what depends on the choice is here.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    pub model: &'a Model,
    pub sel: &'a UsageChoice,
}

impl Deref for View<'_> {
    type Target = Model;

    fn deref(&self) -> &Model {
        self.model
    }
}

impl<'a> View<'a> {
    pub fn new(model: &'a Model, sel: &'a UsageChoice) -> Self {
        Self { model, sel }
    }

    /// An owned copy, for a closure that draws later.
    pub fn chosen(&self) -> Chosen {
        Chosen {
            model: self.model.clone(),
            sel: self.sel.clone(),
        }
    }

    pub fn section(&self) -> Section {
        self.sel.section
    }

    /// The load of `section`'s report as the choice asks it: unavailable while it was never asked.
    pub fn history(&self, section: Section) -> Load<Rc<History>> {
        self.sel
            .key(self.model, section)
            .and_then(|key| self.model.histories.get(&key).cloned())
            .unwrap_or_default()
    }

    pub fn group_options(&self) -> Vec<usize> {
        self.sel.group_options(self.model)
    }

    pub fn group_of(&self, section: Section) -> usize {
        self.sel.groups[section as usize]
    }

    pub fn control_available(&self, control: Control, detail_open: bool) -> bool {
        self.sel.control_available(self.model, control, detail_open)
    }

    pub fn range_of(&self, section: Section) -> usize {
        self.sel.range_of(self.model, section)
    }

    /// 7 or 30 days.
    pub fn days_of(&self, section: Section) -> u32 {
        self.sel.days_of(self.model, section)
    }

    pub fn section_date_range(&self, section: Section) -> RangeInclusive<NaiveDate> {
        self.sel.section_date_range(self.model, section)
    }

    pub fn task_metric(&self) -> crate::screens::usage::chats::Metric {
        self.sel.task_metric(self.model)
    }

    pub fn group_name(&self, section: Section, group: usize) -> GroupName {
        self.model.group_name(section, group)
    }
}
