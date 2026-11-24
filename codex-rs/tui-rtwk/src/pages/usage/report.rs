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
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_core::canvas::Canvas;

use codex_tui_core::screens::usage::plot::{Ink, Plot};
use codex_tui_core::screens::usage::report::{
    Body, Head, Label, Left, Message, Note, Report, ReportView, Right, Row as Line, Summary,
};
use codex_tui_core::screens::usage::{Load, Section, View};

use super::Look;
use super::cards::{Fit, Item};
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
pub fn message_text(model: &View<'_>, section: Section, strings: &Strings) -> Option<String> {
    let load = &model.history(section);
    match load {
        Load::Ready(history) if history.data.is_empty() => Some(strings.empty_range.to_owned()),
        Load::Ready(_) => None,
        other => other.note().map(|note| strings.note(&note).to_owned()),
    }
}

/// The words of [`message_text`], in the secondary style the oracle gives
/// a chart report's message (`chart.rs`) and none to the other tabs'.
pub fn message(model: &View<'_>, section: Section, strings: &Strings, look: &Look) -> AnyWidget {
    let style = if is_chart(section) {
        look.secondary.clone()
    } else {
        Style::default()
    };
    message_text(model, section, strings)
        .map_or_else(nothing, |message| line(message, style).into())
}

/// The report of `section` in the room the body gets.
pub fn body(
    model: &View<'_>,
    section: Section,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    shown: Shown,
    scroll: ScrollController,
) -> AnyWidget {
    if !is_chart(section) || model.history(section).ready().is_none() {
        return message(model, section, strings, look);
    }
    let (owned, strings, num, look) = (model.chosen(), *strings, *num, look.clone());
    LayoutBuilder::new(move |_, constraints| {
        let model = owned.view();
        let view = ReportView {
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
            Some(report) => SingleChildScrollView::new(report_view(&report, &strings, &num, &look))
                .controller(scroll.clone())
                .into(),
            None => message(&model, section, &strings, &look),
        }
    })
    .into()
}

/// The ink's style (oracle: `styles.rs`, `plot/painting.rs`): the series are
/// the terminal's cyan, magenta and green and its own colour for the rest.
fn style_of(ink: Ink, look: &Look) -> Style {
    let series = |index: usize| look.series[index.min(3)].clone();
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

/// A line of words: one row the room's end cuts (a report), or wrapped by
/// word in a card, where `fit` counts the rows it takes (oracle: the card's
/// `word_wrap_lines`).
pub(super) fn words(shown: String, style: Style, fit: Option<Fit>) -> Item {
    match fit {
        None => Item::one(line(shown, style)),
        Some(fit) => {
            let rows = fit.rows(&shown);
            Item::new(Text::new(shown).style(style).wrap(TextWrap::Word), rows)
        }
    }
}

/// Spans in one row cut with an ellipsis (a report), or wrapped by word in
/// a card.
fn spans_row(spans: Vec<Span>, fit: Option<Fit>) -> Item {
    match fit {
        None => Item::one(text(spans)),
        Some(fit) => {
            let shown: String = spans.iter().map(|span| span.text.as_str()).collect();
            Item::new(Text::rich(spans).wrap(TextWrap::Word), fit.rows(&shown))
        }
    }
}

/// The words of a report's first line and their style.
fn head(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> (String, Style) {
    match &report.head {
        Head::Relative => (strings.relative_head.to_owned(), look.secondary.clone()),
        Head::Total {
            amount,
            unit,
            reported,
        } => (
            strings.total_line(amount, *unit, *reported, num),
            look.accent.clone(),
        ),
    }
}

fn note(note: Note, strings: &Strings) -> &'static str {
    match note {
        Note::Approximate => strings.approximate,
        Note::TurnStart => strings.turn_start,
    }
}

/// The head, the notes and the plot (or why there is none).
fn lead(
    report: &Report,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    fit: Option<Fit>,
) -> Vec<Item> {
    let (shown, style) = head(report, strings, num, look);
    let mut items = vec![words(shown, style, fit)];
    for entry in &report.notes {
        items.push(words(
            note(*entry, strings).to_owned(),
            look.secondary.clone(),
            fit,
        ));
    }
    match &report.body {
        Body::Plot(plot) => items.push(Item::new(plot_view(plot, look), plot.rows.len())),
        Body::Empty(unit) => items.push(words(
            unit.map_or_else(
                || strings.no_activity_range.to_owned(),
                |unit| strings.no_unit_range(unit, num),
            ),
            look.secondary.clone(),
            fit,
        )),
        Body::Hidden => {}
    }
    items
}

/// The items one under the other.
fn stacked(items: Vec<Item>, cross: CrossAxisAlignment) -> AnyWidget {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(cross)
        .children(items.into_iter().map(|item| item.widget))
        .into()
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
        Left::Group(name) => strings.group_name(name),
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
fn table(
    report: &Report,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    fit: Option<Fit>,
) -> Vec<Item> {
    let table = &report.table;
    let date = strings.date_short(table.date.ymd(), true, num);
    let mut items = Vec::new();
    if report.zoomed {
        items.push(Item::one(
            Divider::new()
                .kind(rtwk::layout::BorderType::Single)
                .style(look.dim.clone())
                .merge(false),
        ));
    }
    if let Some(message) = table.message {
        // The message keeps the rows the table measured (oracle: the table
        // allocation is preserved).
        let shown = match message {
            Message::NotReported => strings.day_not_reported(&date, num),
            Message::NoUnit(unit) => strings.day_no_unit(unit, &date, num),
        };
        let rows = report.table_rows() - usize::from(report.zoomed);
        items.push(Item::new(
            SizedBox::new(
                None,
                Some(rows as i32),
                Some(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(text(vec![Span::styled(shown, look.secondary.clone())]))
                        .into(),
                ),
            ),
            rows,
        ));
        return items;
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
    items.push(spans_row(
        vec![Span::styled(date, look.bold.clone()), summary],
        fit,
    ));
    if let Some((left, right)) = table.header {
        items.push(Item::one(header(left, right, strings, look)));
    }
    for row in &table.rows {
        items.push(Item::one(entry(row, strings, num, look)));
    }
    items
}

/// The report: the table beside the plot from 140 cells, below it otherwise.
fn report_view(report: &Report, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let lead = stacked(
        lead(report, strings, num, look, None),
        CrossAxisAlignment::Start,
    );
    let table = stacked(
        table(report, strings, num, look, None),
        CrossAxisAlignment::Stretch,
    );
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

/// What a card holds of the report of `section`, the plot `fit.plot_rows` tall
/// (oracle: `dashboard_lines`, `history_lines`): the report as in the
/// focused view, the table of the two first rows, the time it was updated
/// last, each with the rows it takes. A report that is not ready says why.
pub fn card(
    model: &View<'_>,
    section: Section,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    shown: Shown,
    fit: Fit,
) -> Vec<Item> {
    let report = is_chart(section)
        .then(|| {
            let view = ReportView {
                cursor: shown.cursor,
                detail: shown.detail,
                zoomed: false,
                viewport: fit.plot_rows,
            };
            let date_label = |date: codex_tui_core::screens::usage::NaiveDate, month: bool| {
                strings.date_short(date.ymd(), month, num)
            };
            model.report(section, view, fit.width, fit.plot_rows, &date_label)
        })
        .flatten();
    let Some(report) = report else {
        return message_text(model, section, strings)
            .map(|message| words(message, look.secondary.clone(), Some(fit)))
            .into_iter()
            .collect();
    };
    let mut items = lead(&report, strings, num, look, Some(fit));
    items.push(Item::new(gap(), 1));
    let table = table(&report, strings, num, look, Some(fit));
    let rows = table.iter().map(|item| item.rows).sum();
    items.push(Item::new(
        SizedBox::new(
            Some(report.table_width as i32),
            None,
            Some(stacked(table, CrossAxisAlignment::Stretch)),
        ),
        rows,
    ));
    if let Some(updated) = report.updated {
        items.push(words(
            strings.updated_card(updated, model.hour24, num),
            look.secondary.clone(),
            Some(fit),
        ));
    }
    items
}
