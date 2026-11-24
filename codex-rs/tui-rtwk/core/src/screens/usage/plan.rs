//! Historical allowance snapshots, separate from daily usage and current chat
//! allowances (oracle: `analytics/plan.rs`). Period ids are snapshot-scoped;
//! unknown accounting is never converted to zero. `Data` is the wire shape
//! the host answers with (the backend client's `PlanLimitHistory`); `Report`
//! is what the page shows, checked by `parse`.

use std::collections::HashSet;

use super::date::rfc3339_unix;

/// The history as the backend sent it.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Data {
    pub data_as_of: Option<String>,
    pub coverage_start: Option<String>,
    pub coverage_complete: bool,
    #[serde(default = "approximate_by_default")]
    pub approximate: bool,
    pub periods: Vec<PeriodData>,
}

// The wire's numbers are checked finite by `Report::parse` before anything
// shows them; the events only carry the history to it.
impl Eq for Data {}

fn approximate_by_default() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct PeriodData {
    pub id: String,
    pub window_minutes: u32,
    pub starts_at: String,
    pub ends_at: String,
    pub accounting_complete: bool,
    /// Hundredths of a percent of this period's historical allowance; null is
    /// unknown.
    pub used_basis_points: Option<f64>,
    pub breakdowns: Option<Vec<Breakdown>>,
}

/// What a period's usage is split by (an additive server dimension that is
/// not listed here is `Unknown` and never shown).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    ThreadSource,
    TurnTrigger,
    Model,
    Surface,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Breakdown {
    pub dimension: Dimension,
    pub rows: Vec<Share>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Share {
    pub key: String,
    pub basis_points: f64,
}

/// The window of the shorter table and the longer one, in minutes.
const FIVE_HOURS: u32 = 300;
const WEEK: u32 = 10_080;

#[derive(Clone, Debug, PartialEq)]
pub struct Period {
    pub id: String,
    /// Unix seconds, UTC.
    pub start: i64,
    pub end: i64,
    pub complete: bool,
    pub used: Option<f64>,
    pub breakdowns: Option<Vec<Breakdown>>,
}

/// What a plan history could not be turned into a report; none holds the
/// wire's own values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A snapshot, start or end time is not an RFC 3339 time.
    InvalidTimestamp,
    /// More periods than a report holds.
    TooManyPeriods,
    /// A period's window is neither five hours nor a week.
    UnsupportedWindow,
    /// A period that ends before it starts, repeats an id or holds a number
    /// that is not finite.
    InvalidPeriod,
}

/// The two tables' periods (5-hour, weekly), the newest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub as_of: i64,
    pub coverage_start: Option<i64>,
    pub coverage_complete: bool,
    pub approximate: bool,
    pub periods: [Vec<Period>; 2],
}

// Every number was checked finite by `parse`, so the equality is total.
impl Eq for Report {}

impl Report {
    /// `None` when the history has no snapshot time (nothing to show).
    pub fn parse(history: Data) -> Result<Option<Self>, Error> {
        let parse = |value: &str| {
            rfc3339_unix(value).ok_or(Error::InvalidTimestamp)
        };
        let Some(as_of) = history.data_as_of.as_deref().map(parse).transpose()? else {
            return Ok(None);
        };
        let mut periods = [Vec::new(), Vec::new()];
        let mut ids = HashSet::new();
        if history.periods.len() > 1_000 {
            return Err(Error::TooManyPeriods);
        }
        for period in history.periods {
            let window = match period.window_minutes {
                FIVE_HOURS => 0,
                WEEK => 1,
                _ => return Err(Error::UnsupportedWindow),
            };
            let start = parse(&period.starts_at)?;
            let end = parse(&period.ends_at)?;
            if start >= end
                || !ids.insert(period.id.clone())
                || period.used_basis_points.is_some_and(|amount| !amount.is_finite())
                || period
                    .breakdowns
                    .iter()
                    .flatten()
                    .flat_map(|group| &group.rows)
                    .any(|row| !row.basis_points.is_finite())
            {
                return Err(Error::InvalidPeriod);
            }
            periods[window].push(Period {
                id: period.id,
                start,
                end,
                complete: period.accounting_complete,
                used: period.used_basis_points,
                breakdowns: period.breakdowns,
            });
        }
        for window in &mut periods {
            window.sort_by_key(|period| std::cmp::Reverse(period.start));
        }
        Ok(Some(Self {
            as_of,
            coverage_start: history.coverage_start.as_deref().map(parse).transpose()?,
            coverage_complete: history.coverage_complete,
            approximate: history.approximate,
            periods,
        }))
    }

    /// The period is the one in progress at the snapshot (`*` in the table).
    pub fn is_current(&self, period: &Period) -> bool {
        period.start <= self.as_of && period.end > self.as_of
    }
}

/// From this width of the body the two tables stand side by side, each
/// `table_width` wide with `TABLE_GAP` cells between (oracle: `plan_lines`).
pub const SIDE_BY_SIDE: usize = 100;
pub const TABLE_GAP: usize = 4;

pub fn side_by_side(width: usize) -> bool {
    width >= SIDE_BY_SIDE
}

/// The width a table is laid out in.
pub fn table_width(width: usize) -> usize {
    if side_by_side(width) {
        (width - TABLE_GAP) / 2
    } else {
        width
    }
}

impl Dimension {
    /// What the breakdown of a period is split by for the group the Plan
    /// report shows (1 thread source, 2 model, 3 turn start, else surface).
    pub fn of_group(group: usize) -> Self {
        match group {
            1 => Self::ThreadSource,
            2 => Self::Model,
            3 => Self::TurnTrigger,
            _ => Self::Surface,
        }
    }
}

/// Where the keys are in the two tables: the one in front, a row in each
/// and whether its details are open. A new snapshot starts the rows over
/// (oracle: `State::poll`); ids are snapshot-scoped, and moving off a row
/// closes it, so an open row is always the one under the cursor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pos {
    pub window: usize,
    pub cursor: [usize; 2],
    pub expanded: [bool; 2],
    snapshot: Option<i64>,
}

/// What a key does to the tables (oracle: `plan_action`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStep {
    /// Left or right: the other table.
    Switch,
    Up,
    Down,
    Top,
    Bottom,
    /// Open or close the details of the row.
    Toggle,
    /// Close the details; with none open, the page.
    Back,
}

impl Pos {
    /// A snapshot other than the one the rows were chosen in starts them over.
    pub fn sync(&mut self, report: Option<&Report>) {
        let snapshot = report.map(|report| report.as_of);
        if snapshot != self.snapshot {
            *self = Self {
                window: self.window,
                snapshot,
                ..Self::default()
            };
        }
    }

    /// Does `step`; `true` when it closes the page.
    pub fn step(&mut self, step: PlanStep, report: Option<&Report>) -> bool {
        self.sync(report);
        let window = self.window;
        let count = report.map_or(0, |report| report.periods[window].len());
        let last = count.saturating_sub(1);
        let before = self.cursor[window];
        let mut close = false;
        match step {
            PlanStep::Switch => self.window ^= 1,
            PlanStep::Up => self.cursor[window] = before.saturating_sub(1),
            PlanStep::Down => self.cursor[window] = (before + 1).min(last),
            PlanStep::Top => self.cursor[window] = 0,
            PlanStep::Bottom => self.cursor[window] = last,
            PlanStep::Toggle => {
                if before < count {
                    self.expanded[window] = !self.expanded[window];
                }
            }
            PlanStep::Back => close = !std::mem::take(&mut self.expanded[window]),
        }
        if self.cursor[window] != before {
            self.expanded[window] = false;
        }
        close
    }

    /// A click on the `index`-th period of table `window`: the row under the
    /// cursor of the table in front opens or closes; any other becomes it.
    pub fn pick(&mut self, window: usize, index: usize, report: Option<&Report>) {
        self.sync(report);
        if window == self.window && self.cursor[window] == index {
            self.step(PlanStep::Toggle, report);
            return;
        }
        self.window = window;
        if self.cursor[window] != index {
            self.cursor[window] = index;
            self.expanded[window] = false;
        }
    }

    /// There is a row under the cursor to open (the `details` hint).
    pub fn has_row(&self, report: Option<&Report>) -> bool {
        let mut pos = self.clone();
        pos.sync(report);
        report.is_some_and(|report| pos.cursor[pos.window] < report.periods[pos.window].len())
    }
}

/// A row of a table (words are the page's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// The table's name; `front` marks the table the keys move in.
    Heading { window: usize, front: bool },
    Blank,
    /// The table holds no period: the range has none (`complete`), or the
    /// history does not reach it yet.
    Empty { complete: bool },
    /// The history is not shown: it has not been asked for.
    Unavailable,
    /// The history is not shown: what its load says (loading, an error).
    Notice(super::Note),
    /// The `index`-th period of table `window`: its span, whether it is the
    /// one in progress, the used share
    /// as the number shown (none: not available), the row under the cursor
    /// (`selected`) and whether that cursor is the one in front (`lit`).
    Period {
        window: usize,
        index: usize,
        start: i64,
        end: i64,
        current: bool,
        used: Option<String>,
        selected: bool,
        lit: bool,
    },
    Incomplete,
    Share { label: String, percent: String },
    NoBreakdown,
}

/// What stands under the tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Turn,
    Gaps,
    AsOf(i64),
    Since(i64),
    Approximate,
}

/// The two tables and the notes under them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanView {
    pub tables: [Vec<Line>; 2],
    pub notes: Vec<Note>,
}

impl super::View<'_> {
    /// The Plan report for the cursor `pos`: the first period of each table
    /// on the card, every one and the open details when `zoomed`.
    pub fn plan_view(&self, pos: &Pos, zoomed: bool) -> PlanView {
        let report = self.plan.ready().map(|report| &**report);
        let mut pos = pos.clone();
        pos.sync(report);
        let group = self.group_of(super::Section::Plan);
        let dimension = Dimension::of_group(group);
        let tables = [0, 1].map(|window| {
            let mut lines = vec![
                Line::Heading {
                    window,
                    front: pos.window == window,
                },
                Line::Blank,
            ];
            let Some(report) = report else {
                lines.push(match &self.plan {
                    super::Load::Unavailable => Line::Unavailable,
                    load => load.note().map_or(Line::Unavailable, Line::Notice),
                });
                return lines;
            };
            if report.periods[window].is_empty() {
                lines.push(Line::Empty {
                    complete: report.coverage_complete,
                });
            }
            let shown = if zoomed { usize::MAX } else { 1 };
            for (index, period) in report.periods[window].iter().take(shown).enumerate() {
                let selected = pos.cursor[window] == index;
                lines.push(Line::Period {
                    window,
                    index,
                    start: period.start,
                    end: period.end,
                    current: report.is_current(period),
                    used: period.used.map(|used| super::data::amount(used / 100.0)),
                    selected,
                    lit: selected && pos.window == window,
                });
                if !(zoomed && selected && pos.expanded[window]) {
                    continue;
                }
                if !period.complete {
                    lines.push(Line::Incomplete);
                }
                let group = period
                    .breakdowns
                    .as_ref()
                    .and_then(|groups| groups.iter().find(|group| group.dimension == dimension));
                let Some(group) = group else {
                    lines.push(Line::NoBreakdown);
                    continue;
                };
                for share in &group.rows {
                    let key = if dimension == Dimension::TurnTrigger {
                        ["start-", share.key.as_str()].concat()
                    } else {
                        share.key.clone()
                    };
                    let label = if dimension == Dimension::Model {
                        self.model_name(&key)
                    } else {
                        super::normalize::label(&key)
                    };
                    lines.push(Line::Share {
                        label: label.to_owned(),
                        percent: super::data::amount(share.basis_points / 100.0),
                    });
                }
            }
            lines
        });
        let mut notes = Vec::new();
        if group == 3 {
            notes.push(Note::Turn);
        }
        if let Some(report) = report {
            if !report.coverage_complete {
                notes.push(Note::Gaps);
            }
            notes.push(Note::AsOf(report.as_of));
            if let Some(start) = report.coverage_start {
                notes.push(Note::Since(start));
            }
            if report.approximate {
                notes.push(Note::Approximate);
            }
        }
        PlanView { tables, notes }
    }
}
