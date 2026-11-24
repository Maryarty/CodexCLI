//! What a chart report (Usage, Credits, Messages, Plugins, Skills) shows for
//! its history: the total, the stacked plot, the selected day's summary and
//! its breakdown rows. Carried from the oracle's `analytics/chart.rs`
//! (`history_lines`) and the height rule of `render.rs`: the oracle built
//! styled lines; this builds the same decisions as values, the words being
//! the view's (`Unit`, `Note`, `Message` name the text, they do not hold it).
//! The cursor, the open detail, the zoom and the viewport are the widget's
//! (R14) and come in as `ReportView`.

use super::View;
use super::data::{amount, compact_amount, credit_amount};
use super::date::NaiveDate;
use super::models::{AccountAnalyticsDay, AccountAnalyticsUnit, AccountAnalyticsValue};
use super::plot::{self, DateLabel, Plot, Slot};
use super::sections::Section;

/// What the widget holds that decides the report's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportView {
    /// The selected day, an index into the range.
    pub cursor: usize,
    /// The open detail (Enter): the whole numbers, no remainder row.
    pub detail: Option<usize>,
    /// The focused report (the default); false is a card of the dashboard.
    pub zoomed: bool,
    /// The body's rows.
    pub viewport: usize,
}

/// The word of the report's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Tokens,
    Messages,
    Calls,
    Uses,
    Credits,
}

/// The first line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// `Total usage · relative units`.
    Relative,
    /// The total (`1.5K tokens`, `12 messages reported`).
    Total {
        amount: String,
        unit: Unit,
        reported: bool,
    },
}

/// A line of explanation under the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Approximate · may be delayed up to 6 hours`.
    Approximate,
    /// `All usage by how each turn started`.
    TurnStart,
}

/// The plot, or why there is none.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Plot(Plot),
    /// No activity in the range: `No <unit> reported in this range.` for the
    /// counted reports, `No activity reported in this range.` for the others.
    Empty(Option<Unit>),
    /// A body too short for a plot.
    Hidden,
}

/// What the selected day says after its date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Summary {
    NotReported,
    NoActivity,
    /// `<amount> usage units`.
    Relative(String),
    Amount { amount: String, unit: Unit },
}

/// The first column's header of the breakdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Left {
    Plugin,
    Skill,
    Chat,
    /// The grouping's name, `Model`, `Surface`, ...
    Group(super::data::GroupName),
}

/// The second column's header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Right {
    Tokens,
    Share,
    Messages,
    Calls,
    Uses,
    Credits,
}

/// A breakdown row's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    Name(String),
    /// `… N more`: the rows folded into one.
    More(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The series colour (0 to 3), none for the folded row.
    pub colour: Option<usize>,
    pub label: Label,
    pub amount: String,
    /// The amount is faint: no data for the day, or a zero.
    pub dim: bool,
}

/// What replaces the breakdown when the selected day has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// `Data not reported for <date>.`
    NotReported,
    /// `No <unit> reported for <date>.`
    NoUnit(Unit),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub date: NaiveDate,
    pub summary: Summary,
    /// The column headers, in the focused report only.
    pub header: Option<(Left, Right)>,
    pub rows: Vec<Row>,
    pub message: Option<Message>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub head: Head,
    pub notes: Vec<Note>,
    pub body: Body,
    pub table: Table,
    /// `Updated <time> UTC`, in a card (Unix seconds).
    pub updated: Option<i64>,
    pub zoomed: bool,
    /// The plot's width, and the table beside it from 140 cells.
    pub plot_width: usize,
    pub side_by_side: bool,
    pub table_width: usize,
    pub bands: usize,
}

impl Report {
    /// The rows left of the table (the head, the notes, the body).
    fn lead_rows(&self) -> usize {
        1 + self.notes.len()
            + match &self.body {
                Body::Plot(plot) => plot.rows.len(),
                Body::Empty(_) => 1,
                Body::Hidden => 0,
            }
    }

    /// The rows of the table: the rule, the summary, the headers, the rows.
    pub fn table_rows(&self) -> usize {
        usize::from(self.zoomed)
            + 1
            + usize::from(self.table.header.is_some())
            + self.table.rows.len()
    }

    /// The rows the report takes (oracle: the panel's `lines.len()`).
    pub fn line_count(&self) -> usize {
        if self.side_by_side {
            self.lead_rows().max(self.table_rows())
        } else {
            self.lead_rows() + 1 + self.table_rows() + usize::from(self.updated.is_some())
        }
    }
}

/// The word of a section's unit.
fn unit(section: Section, history_unit: AccountAnalyticsUnit) -> Unit {
    match section {
        Section::Usage if history_unit == AccountAnalyticsUnit::Tokens => Unit::Tokens,
        Section::Summary => Unit::Tokens,
        Section::Activity => Unit::Messages,
        Section::Plugins => Unit::Calls,
        Section::Skills => Unit::Uses,
        Section::Usage | Section::Credits | Section::Chats | Section::Plan => Unit::Credits,
    }
}

/// Human-readable plugin and skill labels (oracle: `tool_panel.rs`).
pub fn tool_name(label: &str) -> String {
    let label = match label.split_once(':') {
        Some((namespace, name)) if namespace.trim().eq_ignore_ascii_case(name.trim()) => {
            name.trim()
        }
        _ => label,
    };
    label
        .split_whitespace()
        .map(|word| match word.to_ascii_lowercase().as_str() {
            "pr" => "PR",
            "cli" => "CLI",
            "api" => "API",
            "ide" => "IDE",
            "mcp" => "MCP",
            "sdk" => "SDK",
            "ui" => "UI",
            "tui" => "TUI",
            "openai" => "OpenAI",
            _ => word,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl View<'_> {
    /// The focused report of `section` for a body of `width` cells and
    /// `view.viewport` rows: the plot takes a third of the rows (2 to 10),
    /// none under 15 rows or 50 cells, then what the rest of the report
    /// leaves (oracle: `render.rs`).
    pub fn focused(
        &self,
        section: Section,
        view: ReportView,
        width: usize,
        date_label: DateLabel<'_>,
    ) -> Option<Report> {
        let mut height = (view.viewport / 3).clamp(/*min*/ 2, /*max*/ 10);
        if view.viewport < 15 || width < 50 {
            height = 0;
        }
        let mut report = self.report(section, view, width, height, date_label)?;
        if report.bands > 0 {
            let overhead = report.line_count().saturating_sub(height * report.bands);
            let available = view.viewport.saturating_sub(overhead) / report.bands;
            height = available.max(/*other*/ 2);
            report = self.report(section, view, width, height, date_label)?;
        }
        Some(report)
    }

    /// The selected day of `section` has something to show beside its total
    /// (oracle: `day_has_details`).
    pub fn day_has_details(&self, section: Section, day_index: usize) -> bool {
        let history = self.history(section);
        let Some(history) = history.ready() else {
            return false;
        };
        let date = self.section_date_range(section).start().iter_days().nth(day_index);
        history.data.iter().any(|day| {
            date.is_some_and(|date| day.date == date) && day.values.iter().any(|value| value.value != 0.0)
        })
    }

    /// The days of the section's range: 7 or 30.
    pub fn day_count(&self, section: Section) -> usize {
        self.days_of(section) as usize
    }

    /// The report of `section` for `width` cells and a plot `height` rows
    /// tall, or none while its history is not ready or empty (oracle:
    /// `history_lines`).
    pub fn report(
        &self,
        section: Section,
        view: ReportView,
        width: usize,
        height: usize,
        date_label: DateLabel<'_>,
    ) -> Option<Report> {
        let history = self.history(section);
        let history = history.ready()?;
        if history.data.is_empty() {
            return None;
        }
        let range = self.section_date_range(section);
        // At most 30 daily slots; index by date if report ranges grow.
        let days: Vec<Slot<'_>> = range
            .start()
            .iter_days()
            .take_while(|date| date <= range.end())
            .map(|date| (date, history.data.iter().find(|day| day.date == date)))
            .collect();
        let cursor = view.cursor.min(days.len() - 1);
        let day: Option<&AccountAnalyticsDay> = days[cursor].1;
        let group = self.group_of(section);
        let expanded = view.zoomed && view.detail.is_some();
        let mut categories = plot::categories(history);
        if matches!(section, Section::Plugins | Section::Skills) {
            for category in &mut categories {
                category.label = tool_name(&category.label);
            }
        }
        if group == 2 {
            for category in &mut categories {
                category.label = self.model_name(&category.label).to_string();
            }
        }
        let unit = unit(section, history.unit);
        let counted = matches!(
            section,
            Section::Plugins | Section::Activity | Section::Skills
        );
        let empty_day = day.is_none_or(|day| {
            day.total == 0.0 && day.values.iter().all(|value| value.value == 0.0)
        });
        let relative = history.unit == AccountAnalyticsUnit::RelativeUsage;
        let format_amount: fn(f64) -> String = if history.unit == AccountAnalyticsUnit::Credits {
            credit_amount
        } else {
            amount
        };
        let total: f64 = history.data.iter().map(|day| day.total).sum();
        let formatted_total = if total.abs() >= 1_000.0 {
            compact_amount(total)
        } else {
            format_amount(total)
        };
        let head = if relative {
            Head::Relative
        } else {
            Head::Total {
                amount: formatted_total,
                unit,
                reported: section == Section::Activity,
            }
        };
        let mut notes = Vec::new();
        if section == Section::Usage && !self.business() && view.zoomed {
            notes.push(Note::Approximate);
        }
        if group == 3 {
            notes.push(Note::TurnStart);
        }
        let has_activity = history
            .data
            .iter()
            .any(|day| day.total != 0.0 || day.values.iter().any(|value| value.value != 0.0));
        let side_by_side = view.zoomed && width >= 140 && height > 0;
        let plot_width = if side_by_side { width - 48 } else { width };
        let (body, bands) = if has_activity && height > 0 {
            let plot = plot::chart(
                &days,
                &categories,
                cursor,
                plot_width,
                height,
                history.unit,
                date_label,
            );
            let bands = plot.bands;
            (Body::Plot(plot), bands)
        } else if !has_activity {
            (Body::Empty(counted.then_some(unit)), 0)
        } else {
            (Body::Hidden, 0)
        };
        let table_width = if side_by_side { 44 } else { width.min(56) };
        let summary = match day {
            None => Summary::NotReported,
            Some(_) if empty_day => Summary::NoActivity,
            Some(day) if relative => Summary::Relative(amount(day.total)),
            Some(day) => Summary::Amount {
                amount: if expanded {
                    day.total.to_string()
                } else {
                    format_amount(day.total)
                },
                unit,
            },
        };
        let header = view.zoomed.then(|| {
            (
                match section {
                    Section::Plugins => Left::Plugin,
                    Section::Skills => Left::Skill,
                    Section::Chats => Left::Chat,
                    _ => Left::Group(self.group_name(section, group)),
                },
                if history.unit == AccountAnalyticsUnit::Tokens {
                    Right::Tokens
                } else if section == Section::Usage {
                    Right::Share
                } else if section == Section::Activity {
                    Right::Messages
                } else if section == Section::Plugins {
                    Right::Calls
                } else if section == Section::Skills {
                    Right::Uses
                } else {
                    Right::Credits
                },
            )
        });
        let mut values = categories
            .iter()
            .enumerate()
            .map(|(index, category)| {
                (
                    index,
                    AccountAnalyticsValue {
                        value: day
                            .and_then(|day| day.values.iter().find(|value| value.key == category.key))
                            .map_or(/*default*/ 0.0, |value| value.value),
                        ..category.clone()
                    },
                )
            })
            .collect::<Vec<_>>();
        values.sort_by(|(_, a), (_, b)| b.value.abs().total_cmp(&a.value.abs()));
        let row_budget = if view.zoomed {
            (view.viewport / 4).max(/*other*/ 4)
        } else {
            4
        };
        let remainder = if !expanded && values.len() > row_budget {
            let remaining = values.split_off(row_budget - 1);
            Some((
                remaining.len(),
                remaining.iter().map(|(_, value)| value.value).sum::<f64>(),
            ))
        } else {
            None
        };
        let money = |value: f64| match day {
            None => "—".to_owned(),
            Some(day) if section == Section::Usage && history.unit != AccountAnalyticsUnit::Tokens => {
                format!(
                    "{:.1}%",
                    if day.total == 0.0 {
                        0.0
                    } else {
                        value / day.total * 100.0
                    }
                )
            }
            Some(_) if expanded => value.to_string(),
            Some(_) => format_amount(value),
        };
        let rows = values
            .iter()
            .map(|(index, value)| (Some(*index), Label::Name(value.label.clone()), value.value))
            .chain(remainder.map(|(count, value)| (None, Label::More(count), value)))
            .take(if view.zoomed { usize::MAX } else { 2 })
            .map(|(colour, label, value)| Row {
                colour: colour.map(|index| index.min(/*other*/ 3)),
                label,
                amount: money(value),
                dim: day.is_none() || value == 0.0,
            })
            .collect();
        let message = (counted && empty_day).then(|| {
            if day.is_none() {
                Message::NotReported
            } else {
                Message::NoUnit(unit)
            }
        });
        Some(Report {
            head,
            notes,
            body,
            table: Table {
                date: days[cursor].0,
                summary,
                header,
                rows,
                message,
            },
            updated: if view.zoomed { None } else { history.updated_at },
            zoomed: view.zoomed,
            plot_width,
            side_by_side,
            table_width,
            bands,
        })
    }
}
