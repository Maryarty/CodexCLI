//! The Overview's token activity chart as data (oracle:
//! `analytics/activity_chart.rs`): the 52-week window, the levels per cell and
//! the month marks. The oracle builds ratatui `Line`s here; this returns the
//! cells, and the page paints them with the palette of the theme.

use std::collections::BTreeMap;

use super::date::NaiveDate;
use super::profile::Bucket;

pub const WEEK_COUNT: usize = 52;
pub const DAY_COUNT: usize = 7;
const CELL_COUNT: usize = WEEK_COUNT * DAY_COUNT;
/// The gutter left of the cells: weekday names or the bar axis.
pub const CHART_LEFT_WIDTH: usize = 4;

/// Selects the aggregation represented by the token activity chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenActivityView {
    Daily,
    Weekly,
    Cumulative,
}

pub const VIEWS: [TokenActivityView; 3] = [
    TokenActivityView::Daily,
    TokenActivityView::Weekly,
    TokenActivityView::Cumulative,
];

impl TokenActivityView {
    /// Parses the optional `/usage` argument into a supported chart view.
    /// An empty argument selects Daily when a mode is requested; `None` lets
    /// the slash-command dispatcher report unsupported arguments.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "day" | "daily" => Some(Self::Daily),
            "week" | "weekly" => Some(Self::Weekly),
            "cumulative" => Some(Self::Cumulative),
            _ => None,
        }
    }

    /// The canonical slash argument for this chart view.
    pub fn arg(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Cumulative => "cumulative",
        }
    }

    pub fn index(self) -> usize {
        VIEWS.iter().position(|view| *view == self).unwrap_or(0)
    }
}

/// One cell of the grid: nothing (a future day of the last week), or a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Square {
    Blank,
    Level(usize),
}

/// What the line under the grid says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Caption {
    /// The five-step legend of the daily view.
    Legend,
    /// `Each column = 1 week · tallest <peak>`: the lead and the scaled peak.
    Peak {
        lead: &'static str,
        peak: i64,
    },
    NoActivity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chart {
    pub view: TokenActivityView,
    /// No room for a single week: "Widen terminal to show activity graph".
    pub too_narrow: bool,
    /// The month row, one char per cell column (after the gutter).
    pub months: String,
    /// Seven rows of `shown` cells each, Sunday first.
    pub rows: Vec<Vec<Square>>,
    pub caption: Caption,
}

pub fn shown_columns(width: usize) -> usize {
    (width.saturating_sub(CHART_LEFT_WIDTH).saturating_add(1) / 2).min(WEEK_COUNT)
}

impl Chart {
    pub fn build(
        view: TokenActivityView,
        buckets: &[Bucket],
        today: NaiveDate,
        width: usize,
    ) -> Chart {
        let values = daily_values(buckets, today);
        let shown = shown_columns(width);
        if shown == 0 {
            return Chart {
                view,
                too_narrow: true,
                months: String::new(),
                rows: Vec::new(),
                caption: Caption::Legend,
            };
        }
        let levels = levels_for_view(&values, view);
        let first_column = WEEK_COUNT - shown;
        let rows = (0..DAY_COUNT)
            .map(|row| {
                (first_column..WEEK_COUNT)
                    .map(|column| {
                        let index = column * DAY_COUNT + row;
                        if view == TokenActivityView::Daily
                            && cell_date(today, index).is_some_and(|date| date > today)
                        {
                            Square::Blank
                        } else {
                            Square::Level(levels[index])
                        }
                    })
                    .collect()
            })
            .collect();
        let caption = match view {
            TokenActivityView::Daily => Caption::Legend,
            TokenActivityView::Weekly | TokenActivityView::Cumulative => {
                let weeks = weekly_totals(&values);
                let (lead, peak) = if view == TokenActivityView::Weekly {
                    (
                        "Each column = 1 week \u{b7} tallest ",
                        weeks.iter().copied().max().unwrap_or(0),
                    )
                } else {
                    (
                        "Running total \u{b7} top ",
                        weeks
                            .iter()
                            .fold(0_i64, |sum, value| sum.saturating_add(*value)),
                    )
                };
                if peak <= 0 {
                    Caption::NoActivity
                } else {
                    Caption::Peak { lead, peak }
                }
            }
        };
        Chart {
            view,
            too_narrow: false,
            months: month_labels(today, first_column, shown),
            rows,
            caption,
        }
    }
}

fn month_labels(today: NaiveDate, first_column: usize, shown_columns: usize) -> String {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut cells = vec![' '; shown_columns * 2 - 1];
    let start = chart_start(today);
    let mut last_end = 0;
    for column in first_column..WEEK_COUNT {
        let date = start.plus_days((column * DAY_COUNT) as i64);
        if date.day() > 7 {
            continue;
        }
        let label = NAMES[date.month() as usize - 1];
        let offset = (column - first_column) * 2;
        if offset < last_end || offset + label.len() > cells.len() {
            continue;
        }
        for (index, ch) in label.chars().enumerate() {
            cells[offset + index] = ch;
        }
        last_end = offset + label.len() + 1;
    }
    cells.into_iter().collect()
}

/// Normalizes backend daily buckets into the fixed 52-week display window,
/// ordered by chart cell from the oldest Sunday. Invalid, out-of-window and
/// future dates are ignored; duplicates add up; negatives do not reduce.
pub fn daily_values(buckets: &[Bucket], today: NaiveDate) -> Vec<i64> {
    let start = chart_start(today);
    let end = start.plus_days(CELL_COUNT as i64);
    let mut by_date = BTreeMap::new();
    for bucket in buckets {
        let Some(date) = NaiveDate::parse_ymd(&bucket.start_date) else {
            continue;
        };
        if date < start || date >= end || date > today {
            continue;
        }
        let value: &mut i64 = by_date.entry(date).or_insert(0);
        *value = value.saturating_add(bucket.tokens.max(0));
    }
    (0..CELL_COUNT)
        .map(|offset| {
            by_date
                .get(&start.plus_days(offset as i64))
                .copied()
                .unwrap_or(0)
        })
        .collect()
}

pub fn levels_for_view(values: &[i64], view: TokenActivityView) -> Vec<usize> {
    match view {
        TokenActivityView::Daily => graded_levels(values),
        TokenActivityView::Weekly => bar_levels(&weekly_totals(values)),
        TokenActivityView::Cumulative => {
            let cumulative = weekly_totals(values)
                .into_iter()
                .scan(0, |sum, value| {
                    *sum = i64::saturating_add(*sum, value);
                    Some(*sum)
                })
                .collect::<Vec<_>>();
            bar_levels(&cumulative)
        }
    }
}

fn graded_levels(values: &[i64]) -> Vec<usize> {
    let max = values.iter().copied().max().unwrap_or(0);
    values
        .iter()
        .map(|value| match (*value, max) {
            (0, _) | (_, 0) => 0,
            (value, max) if i128::from(value) * 4 > i128::from(max) * 3 => 4,
            (value, max) if i128::from(value) * 2 > i128::from(max) => 3,
            (value, max) if i128::from(value) * 4 > i128::from(max) => 2,
            _ => 1,
        })
        .collect()
}

pub fn weekly_totals(values: &[i64]) -> Vec<i64> {
    values
        .chunks(DAY_COUNT)
        .map(|week| {
            week.iter()
                .fold(0_i64, |sum, value| sum.saturating_add(*value))
        })
        .collect()
}

fn bar_levels(totals: &[i64]) -> Vec<usize> {
    let max = totals.iter().copied().max().unwrap_or(0);
    totals
        .iter()
        .flat_map(|value| {
            let height = if *value <= 0 || max <= 0 {
                0
            } else {
                ((i128::from(*value) * DAY_COUNT as i128 + i128::from(max) - 1) / i128::from(max))
                    as usize
            };
            (0..DAY_COUNT).map(move |row| if DAY_COUNT - row <= height { 4 } else { 0 })
        })
        .collect()
}

fn chart_start(today: NaiveDate) -> NaiveDate {
    let week_start = today.minus_days(i64::from(today.weekday_from_sunday()));
    week_start.minus_days(((WEEK_COUNT - 1) * 7) as i64)
}

fn cell_date(today: NaiveDate, index: usize) -> Option<NaiveDate> {
    Some(chart_start(today).plus_days(index as i64))
}
