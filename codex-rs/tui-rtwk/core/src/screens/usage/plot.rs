//! Daily stacked bars with fixed calendar ticks, a selection cursor, and an
//! anchored value. Carried from the oracle's `analytics/plot.rs` and
//! `plot/{layout,painting,annotations}.rs`: the geometry as pure functions
//! of the days, the width and the height, answering a grid of marks (a
//! character and what it is drawn with) that the view paints with a
//! `CellPainter` (the oracle painted a ratatui `Buffer`; the colours and the
//! terminal's palette are the view's).
//! Relative usage retains the server's scale; signed credits keep both bands.

use std::collections::BTreeMap;

use super::data::{compact_amount, credit_amount};
use super::date::NaiveDate;
use super::models::{AccountAnalyticsDay, AccountAnalyticsHistory, AccountAnalyticsUnit};
use super::models::AccountAnalyticsValue;

/// One day of the range: its date and what the report holds for it.
pub type Slot<'a> = (NaiveDate, Option<&'a AccountAnalyticsDay>);

/// What a mark is drawn with; the view maps each to a style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ink {
    #[default]
    Plain,
    /// The series colour of a bar: 0 to 2, and 3 the terminal's own (the
    /// remainder).
    Series(usize),
    /// A refund's partial cell: the series colour on the reversed cell.
    Refund(usize),
    /// The rule, the missing-day markers and the other days' values.
    Dim,
    /// Axis numbers and calendar labels.
    Secondary,
    /// The selected day's value.
    Selected,
    /// The `▲` under the selected day.
    Accent,
    /// The selected day's calendar label.
    Today,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    pub ch: char,
    pub ink: Ink,
}

impl Default for Mark {
    fn default() -> Self {
        Self {
            ch: ' ',
            ink: Ink::Plain,
        }
    }
}

/// The chart: `height` rows of `width` marks, and how many value bands it
/// holds (two when a day is negative).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plot {
    pub width: usize,
    pub rows: Vec<Vec<Mark>>,
    pub bands: usize,
}

impl Plot {
    /// The rows as text, each cut after its last mark that is not a space.
    pub fn text(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|mark| mark.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    fn at(&mut self, x: usize, y: usize) -> &mut Mark {
        &mut self.rows[y][x]
    }

    fn put(&mut self, x: usize, y: usize, text: &str, ink: Ink) {
        for (offset, ch) in text.chars().enumerate() {
            if x + offset < self.width {
                *self.at(x + offset, y) = Mark { ch, ink };
            }
        }
    }
}

/// How a date is spelled on the axis: with the month (`Jan 15`) or without
/// (`15`); the view owns the month names.
pub type DateLabel<'a> = &'a dyn Fn(NaiveDate, bool) -> String;

/// The categories of a history, largest total first (oracle: `render.rs`).
pub fn categories(history: &AccountAnalyticsHistory) -> Vec<AccountAnalyticsValue> {
    let mut totals = BTreeMap::<String, AccountAnalyticsValue>::new();
    for value in history.data.iter().flat_map(|day| &day.values) {
        totals
            .entry(value.key.clone())
            .or_insert_with(|| AccountAnalyticsValue {
                value: 0.0,
                ..value.clone()
            })
            .value += value.value;
    }
    let mut values = totals.into_values().collect::<Vec<_>>();
    values.sort_by(|a, b| {
        b.value
            .abs()
            .total_cmp(&a.value.abs())
            .then_with(|| a.key.cmp(&b.key))
    });
    values
}

/// The last plotted category collects all remaining values without
/// renormalizing them (oracle: `render.rs` `parts`).
pub fn parts(values: &[AccountAnalyticsValue], categories: &[AccountAnalyticsValue]) -> [f64; 4] {
    let mut parts = [0.0; 4];
    for value in values {
        let index = categories
            .iter()
            .take(/*n*/ 3)
            .position(|category| category.key == value.key)
            .unwrap_or(/*default*/ 3);
        parts[index] += value.value;
    }
    parts
}

/// Share the chart's scale with the selected-day summary, including signed
/// values.
pub fn peak(days: &[Slot<'_>], categories: &[AccountAnalyticsValue]) -> f64 {
    days.iter()
        .filter_map(|(_, day)| *day)
        .map(|day| {
            parts(&day.values, categories)
                .iter()
                .map(|value| value.abs())
                .sum::<f64>()
                .max(day.total.abs())
        })
        .fold(/*init*/ 0.0, f64::max)
}

fn counts(unit: AccountAnalyticsUnit) -> bool {
    matches!(
        unit,
        AccountAnalyticsUnit::Count | AccountAnalyticsUnit::Tokens
    )
}

/// Two readable intervals cover the range; counts never use fractional ticks.
pub fn axis_max(peak: f64, unit: AccountAnalyticsUnit) -> f64 {
    if peak == 0.0 {
        return if counts(unit) { 2.0 } else { 1.0 };
    }
    let step = if counts(unit) {
        (peak / 2.0).max(/*other*/ 1.0)
    } else {
        peak / 2.0
    };
    let magnitude = 10.0_f64.powf(step.log10().floor());
    let normalized = step / magnitude;
    let multiple = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 7.5, 10.0]
        .into_iter()
        .find(|multiple| {
            *multiple >= normalized && (!counts(unit) || (multiple * magnitude).fract() == 0.0)
        })
        .unwrap_or(/*default*/ 10.0);
    let ceiling = multiple * magnitude * 2.0;
    if ceiling.is_finite() && ceiling >= peak {
        ceiling
    } else {
        peak
    }
}

/// A signed stack grows away from the baseline; amounts are nonnegative
/// within each band.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Band {
    Positive,
    Negative,
}

impl Band {
    fn amount(self, value: f64) -> f64 {
        match self {
            Self::Positive => value,
            Self::Negative => -value,
        }
        .max(/*other*/ 0.0)
    }

    fn row_y(self, baseline: usize, row: usize) -> usize {
        match self {
            Self::Positive => baseline - row - 1,
            Self::Negative => baseline + row + 1,
        }
    }

    fn partial_glyph(self, level: usize) -> char {
        // Reversing the complementary lower block paints a top-anchored partial refund.
        let index = match self {
            Self::Negative if level < 8 => 8 - level,
            Self::Positive | Self::Negative => level,
        };
        [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][index]
    }
}

/// Calendar viewport and shared coordinates for bars, axes, and annotations.
#[derive(Clone, Copy)]
struct Layout {
    width: usize,
    height: usize,
    axis_width: usize,
    plot_width: usize,
    count: usize,
    start: usize,
    baseline: usize,
    marker_row: usize,
}

impl Layout {
    fn new(
        day_count: usize,
        cursor: usize,
        width: usize,
        height: usize,
        label_width: usize,
        bands: usize,
    ) -> Self {
        // Hide the axis when a complete label would crowd out the bars.
        let axis_width = if width >= 30 && label_width <= width / 3 {
            label_width
        } else {
            0
        };
        let plot_width = width - axis_width;
        let count = day_count.min(plot_width);
        let start = cursor.saturating_sub(count - 1).min(day_count - count);
        let baseline = height + 1;
        let marker_row = baseline + if bands == 2 { height + 2 } else { 1 };
        Self {
            width,
            height,
            axis_width,
            plot_width,
            count,
            start,
            baseline,
            marker_row,
        }
    }

    fn center(self, index: usize) -> usize {
        self.axis_width
            + (index * self.plot_width / self.count + (index + 1) * self.plot_width / self.count
                - 1)
                / 2
    }
}

/// Shared inputs for painting and annotations; all phases use the same
/// calendar geometry.
struct Renderer<'a> {
    days: &'a [Slot<'a>],
    values: Vec<[f64; 4]>,
    cursor: usize,
    peak: f64,
    unit: AccountAnalyticsUnit,
    bands: &'static [Band],
    layout: Layout,
    date_label: DateLabel<'a>,
}

fn amount(value: f64, unit: AccountAnalyticsUnit) -> String {
    match unit {
        AccountAnalyticsUnit::RelativeUsage => super::data::amount(value),
        AccountAnalyticsUnit::Count | AccountAnalyticsUnit::Tokens => compact_amount(value.round()),
        AccountAnalyticsUnit::Credits => credit_amount(value),
    }
}

/// The calendar selection as marks; the cursor must index days and the
/// dimensions fit the terminal (oracle: `plot::chart`).
pub fn chart(
    days: &[Slot<'_>],
    categories: &[AccountAnalyticsValue],
    cursor: usize,
    width: usize,
    height: usize,
    unit: AccountAnalyticsUnit,
    date_label: DateLabel<'_>,
) -> Plot {
    if days.is_empty() || width == 0 {
        return Plot::default();
    }
    let values = days
        .iter()
        .map(|(_, day)| day.map_or(/*default*/ [0.0; 4], |day| parts(&day.values, categories)))
        .collect::<Vec<_>>();
    let bands: &'static [Band] = if values.iter().flatten().any(|value| *value < 0.0) {
        &[Band::Positive, Band::Negative]
    } else {
        &[Band::Positive]
    };
    let peak = axis_max(peak(days, categories), unit);
    let label_width = compact_amount(peak)
        .chars()
        .count()
        .max(compact_amount(peak / 2.0).chars().count())
        + bands.len()
        - 1
        + 2;
    let layout = Layout::new(days.len(), cursor, width, height, label_width, bands.len());
    let renderer = Renderer {
        days,
        values,
        cursor,
        peak,
        unit,
        bands,
        layout,
        date_label,
    };
    let mut plot = Plot {
        width,
        rows: vec![vec![Mark::default(); width]; layout.marker_row + 2],
        bands: bands.len(),
    };
    renderer.paint_bars(&mut plot);
    renderer.annotate_missing(&mut plot);
    renderer.paint_axis(&mut plot);
    renderer.annotate_selection(&mut plot);
    renderer.annotate_calendar(&mut plot);
    plot
}

impl Renderer<'_> {
    fn paint_bars(&self, plot: &mut Plot) {
        let Layout {
            axis_width,
            baseline,
            count,
            height,
            plot_width,
            start,
            ..
        } = self.layout;
        for (index, parts) in self.values[start..start + count].iter().enumerate() {
            let left = axis_width + index * plot_width / count;
            let step = (index + 1) * plot_width / count - index * plot_width / count;
            let bar_width = (step * 2 / 3).clamp(/*min*/ 1, /*max*/ 12);
            let x = left + (step - bar_width) / 2;
            for band in self.bands {
                let plotted = parts.iter().map(|value| band.amount(*value)).sum::<f64>();
                let scaled = ((plotted * (height * 8) as f64 / self.peak) as usize)
                    .max(usize::from(plotted > 0.0));
                for row in 0..height {
                    let level = scaled.saturating_sub(row * 8).min(/*other*/ 8);
                    if level == 0 {
                        continue;
                    }
                    let reverse = *band == Band::Negative && level < 8;
                    let glyph = band.partial_glyph(level);
                    let bottom = (row * 8) as f64 * self.peak / (height * 8) as f64;
                    let top =
                        ((row * 8 + level) as f64 * self.peak / (height * 8) as f64).min(plotted);
                    let sample = bottom + (top - bottom) / 2.0;
                    let mut cumulative = 0.0;
                    let color = parts
                        .iter()
                        .position(|value| {
                            cumulative += band.amount(*value);
                            sample < cumulative
                        })
                        .unwrap_or(/*default*/ 3);
                    let y = band.row_y(baseline, row);
                    let ink = if reverse {
                        Ink::Refund(color)
                    } else {
                        Ink::Series(color)
                    };
                    for col in x..x + bar_width {
                        *plot.at(col, y) = Mark { ch: glyph, ink };
                    }
                }
            }
        }
    }

    fn paint_axis(&self, plot: &mut Plot) {
        let Layout {
            axis_width,
            baseline,
            height,
            width,
            ..
        } = self.layout;
        for x in axis_width..width {
            *plot.at(x, baseline) = Mark {
                ch: '─',
                ink: Ink::Dim,
            };
        }
        if axis_width > 0 {
            let mut ticks = vec![(1, self.peak), (baseline, 0.0)];
            if height >= 4 {
                ticks.push((1 + height / 2, self.peak / 2.0));
            }
            if self.bands.len() == 2 {
                ticks.push((baseline + height, -self.peak));
            }
            for (y, value) in ticks {
                let label = compact_amount(value);
                let x = axis_width.saturating_sub(label.len() + 1);
                plot.put(x, y, &label, Ink::Secondary);
            }
        }
    }

    fn annotate_missing(&self, plot: &mut Plot) {
        let Layout {
            count,
            height,
            start,
            ..
        } = self.layout;
        for (index, parts) in self.values[start..start + count].iter().enumerate() {
            // The selected zero/missing value gets its own callout below; avoid repeating it.
            if start + index != self.cursor {
                let marker = match self.days[start + index].1 {
                    None => Some('—'),
                    Some(day) if day.total == 0.0 && parts.iter().all(|value| *value == 0.0) => {
                        Some('0')
                    }
                    Some(_) => None,
                };
                if let Some(marker) = marker {
                    *plot.at(self.layout.center(index), height) = Mark {
                        ch: marker,
                        ink: Ink::Dim,
                    };
                }
            }
        }
    }

    fn annotate_selection(&self, plot: &mut Plot) {
        let Layout {
            axis_width,
            baseline,
            height,
            marker_row,
            plot_width,
            start,
            width,
            ..
        } = self.layout;
        // Give the selected value first choice of space; the other labels never overwrite it.
        for index in
            std::iter::once(self.cursor).chain((start..start + self.layout.count).filter(|index| {
                self.days.len() <= 7
                    && *index != self.cursor
                    && self.days[*index].1.is_some_and(|day| {
                        day.total != 0.0 || self.values[*index].iter().any(|value| *value != 0.0)
                    })
            }))
        {
            let total = self.days[index].1.map_or(/*default*/ 0.0, |day| day.total);
            let label = self.days[index]
                .1
                .map_or_else(|| "—".into(), |_| amount(total, self.unit));
            let label_width = label.chars().count();
            // A partial number can mean a different amount; leave the cursor when the value cannot fit.
            if label_width > plot_width {
                continue;
            }
            let x = self.layout.center(index - start);
            let label_x = x
                .saturating_sub(label_width / 2)
                .clamp(axis_width, width - label_width);
            let occupied = label_x.saturating_sub(/*rhs*/ 1).max(axis_width)
                ..(label_x + label_width + 1).min(width);
            let below = total < 0.0;
            let band = if below {
                Band::Negative
            } else {
                Band::Positive
            };
            let extent = self.values[index]
                .iter()
                .map(|value| band.amount(*value))
                .sum::<f64>();
            let rows = ((extent * height as f64 / self.peak).ceil() as usize).min(height);
            let cap = if below {
                baseline + rows
            } else {
                baseline - rows
            };
            let mut label_y = if below {
                cap + 1
            } else {
                cap.saturating_sub(/*rhs*/ 1)
            };
            // Keep the callout close without painting over a neighboring bar. A guide connects any gap.
            while occupied.clone().any(|col| plot.at(col, label_y).ch != ' ') {
                if below && label_y + 1 < marker_row {
                    label_y += 1;
                } else if !below && label_y > 0 {
                    label_y -= 1;
                } else {
                    break;
                }
            }
            if occupied.clone().any(|col| plot.at(col, label_y).ch != ' ') {
                continue;
            }
            let ink = if index == self.cursor {
                Ink::Selected
            } else {
                Ink::Dim
            };
            let guide = if below {
                cap + 1..label_y
            } else {
                label_y + 1..cap
            };
            for y in guide {
                let cell = plot.at(x, y);
                if cell.ch == ' ' {
                    *cell = Mark { ch: '┊', ink };
                }
            }
            plot.put(label_x, label_y, &label, ink);
        }
        let x = self.layout.center(self.cursor - start);
        *plot.at(x, marker_row) = Mark {
            ch: '▲',
            ink: Ink::Accent,
        };
    }

    fn annotate_calendar(&self, plot: &mut Plot) {
        let Layout {
            axis_width,
            count,
            marker_row,
            plot_width,
            start,
            width,
            ..
        } = self.layout;
        // Calendar ticks depend only on the range/width, never on the current selection.
        let mut occupied = vec![false; width];
        // Give the two range endpoints priority over intermediate weekly ticks.
        for index in std::iter::once(/*value*/ 0).chain((1..count).rev()) {
            if self.days.len() > 7
                && !(start + index).is_multiple_of(/*rhs*/ 7)
                && index + 1 != count
            {
                continue;
            }
            let date = self.days[start + index].0;
            let month = !(self.days.len() <= 7 && plot_width / count < 7);
            let mut label = (self.date_label)(date, month);
            if label.chars().count() > plot_width {
                label = (self.date_label)(date, false);
            }
            let len = label.chars().count();
            if len > plot_width {
                continue;
            }
            let offset = self
                .layout
                .center(index)
                .saturating_sub(len / 2)
                .clamp(axis_width, width - len);
            if occupied[offset.saturating_sub(/*rhs*/ 1)..(offset + len + 1).min(width)]
                .iter()
                .any(|used| *used)
            {
                continue;
            }
            occupied[offset..offset + len].fill(/*value*/ true);
            let ink = if start + index == self.cursor {
                Ink::Today
            } else {
                Ink::Secondary
            };
            plot.put(offset, marker_row + 1, &label, ink);
        }
    }
}
