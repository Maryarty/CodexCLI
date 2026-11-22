//! The body of a report tab other than the Overview (oracle: `panels.rs`,
//! `tool_panel.rs`, `task_panel.rs`, `plan_panel.rs`, `chart.rs`): what its
//! load says while it is not ready, that the range holds no data, and the
//! chart reports (Usage, Credits, Messages, Plugins, Skills): the total, the
//! plot, the selected day and its breakdown. The words are `strings.rs`'s,
//! the decisions core's (`Model::focused`), the plot a `CustomPaint` over the
//! cell grid core answers. The lists of Chats and Plan are built in a later step.

use rtwk::l10n::NumSyms;
use rtwk::layout::{
    CellPainter, Column, CrossAxisAlignment, CustomPaint, Divider, Expanded, MainAxisSize, Row,
    SizedBox,
};
use rtwk::style::{Attrs, Color, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_core::canvas::Canvas;

use codex_tui_core::screens::usage::plot::{Ink, Plot};
use codex_tui_core::screens::usage::report::{
    Body, Head, Label, Left, Message, Note, Report, Right, Row as Line, Summary, View,
};
use codex_tui_core::screens::usage::{Load, Model, Section};

use super::Look;
use super::strings::Strings;
use super::{gap, line, nothing};

/// What the widget holds of a report: the selected day, the open detail and
/// whether the report is the focused one (R14: indexes and flags only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    pub cursor: usize,
    pub detail: Option<usize>,
    pub zoomed: bool,
}

/// The gap between the plot and the table beside it (oracle: `join_columns`).
const BESIDE: i32 = 4;

/// Sections drawn as a daily chart.
pub fn is_chart(section: Section) -> bool {
    matches!(
        section,
        Section::Usage | Section::Credits | Section::Activity | Section::Plugins | Section::Skills
    )
}

/// What the load of `section` says while it is not ready, or that its range
/// holds no data; nothing for a ready report.
pub fn message(model: &Model, section: Section, strings: &Strings) -> AnyWidget {
    let load = &model.histories[section as usize];
    match load {
        Load::Ready(history) if history.data.is_empty() => {
            line(strings.empty_range, Style::default()).into()
        }
        Load::Ready(_) => nothing(),
        other => other
            .message()
            .map_or_else(nothing, |message| line(message, Style::default()).into()),
    }
}

/// The report of `section` in the room the body gets.
pub fn body(
    model: &Model,
    section: Section,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    shown: Shown,
) -> AnyWidget {
    if !is_chart(section) || model.histories[section as usize].ready().is_none() {
        return message(model, section, strings);
    }
    let (model, strings, num, look) = (model.clone(), *strings, *num, look.clone());
    LayoutBuilder::new(move |_, constraints| {
        let view = View {
            cursor: shown.cursor,
            detail: shown.detail,
            zoomed: shown.zoomed,
            viewport: constraints.max_height.max(0) as usize,
        };
        let date_label = |date: codex_tui_core::screens::usage::NaiveDate, month: bool| {
            strings.date_short(date.ymd(), month, &num)
        };
        let width = constraints.max_width.max(0) as usize;
        match model.focused(section, view, width, &date_label) {
            Some(report) => report_view(&report, &strings, &num, &look),
            None => message(&model, section, &strings),
        }
    })
    .into()
}

/// The ink's style (oracle: `styles.rs`, `plot/painting.rs`): the series are
/// the terminal's cyan, magenta and green and its own colour for the rest.
fn style_of(ink: Ink, look: &Look) -> Style {
    let series = |index: usize| match index {
        0 => Style::new().fg(Color::CYAN),
        1 => Style::new().fg(Color::MAGENTA),
        2 => Style::new().fg(Color::GREEN),
        _ => Style::new(),
    };
    match ink {
        Ink::Plain => Style::new(),
        Ink::Series(index) => series(index),
        Ink::Refund(index) => series(index).attrs(Attrs::REVERSE),
        Ink::Dim => look.dim.clone(),
        Ink::Secondary => look.secondary.clone(),
        Ink::Selected => look.bold.clone(),
        Ink::Accent => look.accent.clone(),
        Ink::Today => look.accent.clone().attrs(Attrs::UNDERLINE),
    }
}

/// The plot's cells (a `CustomPaint`: no text widget draws a grid of cells
/// in several inks, G40's manner).
#[derive(Debug, PartialEq)]
struct PlotPainter {
    plot: Plot,
    look: Look,
}

impl CellPainter for PlotPainter {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        for (y, row) in self.plot.rows.iter().enumerate() {
            if y as i32 >= height {
                break;
            }
            let mut x = 0;
            while x < row.len() {
                let ink = row[x].ink;
                let end = x + row[x..].iter().take_while(|mark| mark.ink == ink).count();
                let run: String = row[x..end].iter().map(|mark| mark.ch).collect();
                if x as i32 >= width {
                    break;
                }
                if ink != Ink::Plain || !run.trim().is_empty() {
                    let pen = style_of(ink, &self.look).pen_in(canvas.styles_mut());
                    canvas.text(x as i32, y as i32, &run, pen);
                }
                x = end;
            }
        }
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>() != Some(self)
    }
}

fn plot_view(plot: &Plot, look: &Look) -> AnyWidget {
    CustomPaint::new()
        .painter(PlotPainter {
            plot: plot.clone(),
            look: look.clone(),
        })
        .size(plot.width as i32, plot.rows.len() as i32)
        .into()
}

fn text(spans: Vec<Span>) -> Text {
    Text::rich(spans)
        .wrap(TextWrap::None)
        .overflow(TextOverflow::Ellipsis)
        .max_lines(1)
}

/// The words of a report's first line.
fn head(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> Text {
    match &report.head {
        Head::Relative => line(strings.relative_head, look.secondary.clone()),
        Head::Total {
            amount,
            unit,
            reported,
        } => line(
            strings.total_line(amount, *unit, *reported, num),
            look.accent.clone(),
        ),
    }
}

fn note(note: Note, strings: &Strings, look: &Look) -> Text {
    line(
        match note {
            Note::Approximate => strings.approximate,
            Note::TurnStart => strings.turn_start,
        },
        look.secondary.clone(),
    )
}

/// The head, the notes and the plot (or why there is none).
fn lead(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(head(report, strings, num, look));
    for entry in &report.notes {
        column = column.child(note(*entry, strings, look));
    }
    match &report.body {
        Body::Plot(plot) => column = column.child(plot_view(plot, look)),
        Body::Empty(unit) => {
            column = column.child(line(
                unit.map_or_else(
                    || strings.no_activity_range.to_owned(),
                    |unit| strings.no_unit_range(unit, num),
                ),
                look.secondary.clone(),
            ));
        }
        Body::Hidden => {}
    }
    column.into()
}

/// A label at the left and its value at the right (oracle: `columns`): the
/// label is cut with an ellipsis before it meets the value's column.
fn columns(left: Text, right: Text) -> AnyWidget {
    Row::new()
        .child(Expanded::new(1, left))
        .child(SizedBox::new(Some(2), None, None))
        .child(right)
        .into()
}

fn header(left: Left, right: Right, strings: &Strings, look: &Look) -> AnyWidget {
    let left = match left {
        Left::Plugin => strings.head_plugin,
        Left::Skill => strings.head_skill,
        Left::Chat => strings.head_chat,
        Left::Group(label) => label,
    };
    let right = match right {
        Right::Tokens => strings.head_tokens,
        Right::Share => strings.head_share,
        Right::Messages => strings.head_messages,
        Right::Calls => strings.head_calls,
        Right::Uses => strings.head_uses,
        Right::Credits => strings.head_credits,
    };
    let bold = look.bold.clone();
    columns(line(left, bold.clone()), line(right, bold))
}

fn entry(row: &Line, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let label = match &row.label {
        Label::Name(name) => text(vec![
            Span::styled(
                strings.bullet,
                row.colour
                    .map_or_else(Style::new, |index| style_of(Ink::Series(index), look)),
            ),
            Span::styled(name.clone(), Style::new()),
        ]),
        Label::More(count) => text(vec![
            Span::styled(strings.fold_mark, look.dim.clone()),
            Span::styled(strings.more_rows(*count as u64, num), look.dim.clone()),
        ]),
    };
    let amount = line(
        row.amount.clone(),
        if row.dim {
            look.dim.clone()
        } else {
            Style::new()
        },
    );
    columns(label, amount)
}

/// The selected day: its date and summary, then its breakdown (or the
/// message that replaces both when the day has none).
fn table(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let table = &report.table;
    let date = strings.date_short(table.date.ymd(), true, num);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if report.zoomed {
        column = column.child(
            Divider::new()
                .kind(rtwk::layout::BorderType::Single)
                .style(look.dim.clone())
                .merge(false),
        );
    }
    if let Some(message) = table.message {
        // The message keeps the rows the table measured (oracle: the table
        // allocation is preserved).
        let shown = match message {
            Message::NotReported => strings.day_not_reported(&date, num),
            Message::NoUnit(unit) => strings.day_no_unit(unit, &date, num),
        };
        let rows = report.table_rows() - usize::from(report.zoomed);
        return column
            .child(SizedBox::new(
                None,
                Some(rows as i32),
                Some(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(text(vec![Span::styled(shown, look.secondary.clone())]))
                        .into(),
                ),
            ))
            .into();
    }
    let summary = match &table.summary {
        Summary::NotReported => Span::styled(strings.summary_not_reported, look.secondary.clone()),
        Summary::NoActivity => Span::styled(strings.summary_no_activity, look.secondary.clone()),
        Summary::Relative(amount) => {
            Span::styled(strings.summary_relative(amount, num), look.accent.clone())
        }
        Summary::Amount { amount, unit } => Span::styled(
            strings.summary_amount(amount, *unit, num),
            look.accent.clone(),
        ),
    };
    column = column.child(text(vec![Span::styled(date, look.bold.clone()), summary]));
    if let Some((left, right)) = table.header {
        column = column.child(header(left, right, strings, look));
    }
    for row in &table.rows {
        column = column.child(entry(row, strings, num, look));
    }
    column.into()
}

/// The report: the table beside the plot from 140 cells, below it otherwise.
fn report_view(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let lead = lead(report, strings, num, look);
    let table = table(report, strings, num, look);
    if report.side_by_side {
        return Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(
                Some(report.plot_width as i32),
                None,
                Some(lead),
            ))
            .child(SizedBox::new(Some(BESIDE), None, None))
            .child(SizedBox::new(
                Some(report.table_width as i32),
                None,
                Some(table),
            ))
            .into();
    }
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(lead)
        .child(gap())
        .child(SizedBox::new(
            Some(report.table_width as i32),
            None,
            Some(table),
        ))
        .into()
}
