//! The Overview tab (oracle: `analytics/summary_panel.rs`, `activity_chart.rs`):
//! the profile block, the five figures, the token activity chart, the insights
//! beside the most used plugins and skills, the date the statistics are as of.
//! The block is at most 110 cells wide and centred; the data is core's
//! `Profile` and `Chart` (the chart's cells and levels are core's pure
//! functions, painted here).

use rtwk::layout::{
    Center, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::l10n::NumSyms;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextAlign, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::usage::activity::{CHART_LEFT_WIDTH, Caption, Cell, Chart, VIEWS};
use codex_tui_core::screens::usage::data::compact_amount;
use codex_tui_core::screens::usage::models::plan_label;
use codex_tui_core::screens::usage::profile::{
    InvocationKind, Profile, Stats, count_value, duration, percent,
};
use codex_tui_core::screens::usage::{Load, Model, Section, TokenActivityView};

use super::Look;
use super::strings::Strings;
use super::{gap, line, nothing};

/// The widest the block gets.
const BLOCK: i32 = 110;
/// From this inner width the insights and the most used sit side by side.
const TWO_COLUMNS: i32 = 76;
const TOP_ROWS: usize = 5;

/// The Overview's body for `width` cells of room.
pub fn body(model: &Model, strings: &Strings, num: &NumSyms, look: &Look, width: i32) -> AnyWidget {
    let profile = match &model.profile {
        Load::Ready(profile) => profile.clone(),
        other => {
            return other
                .message()
                .map_or_else(nothing, |message| line(message, Style::default()).into());
        }
    };
    let inner = width.min(BLOCK);
    let view = VIEWS[model.groups[Section::Summary as usize]];
    let (strings, num, look) = (*strings, *num, look.clone());
    let model = model.clone();
    LayoutBuilder::new(move |_, constraints| {
        let inner = constraints.max_width.min(inner).max(1);
        let block = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(identity(&model, &profile, &strings, &look))
            .child(figures(&profile.stats, inner, &strings, &num, &look))
            .child(line(strings.activity, look.bold.clone()))
            .child(line(strings.last_year, look.dim.clone()))
            .child(gap())
            .child(chart(&profile, &model, view, inner, &strings, &num, &look))
            .child(gap())
            .child(insights(&profile.stats, inner, &strings, &num, &look))
            .child(footer(&profile, &strings, &num, &look));
        Center::new(SizedBox::new(Some(inner), None, Some(block.into())))
            .height_factor(1)
            .into()
    })
    .into()
}

fn centred(shown: String, style: &Style) -> AnyWidget {
    line(shown, style.clone()).text_align(TextAlign::Center).into()
}

/// The name, the handle and the plan, centred, then a blank row.
fn identity(model: &Model, profile: &Profile, strings: &Strings, look: &Look) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(identity) = &profile.identity {
        let name = identity.display_name.as_deref().map(str::trim);
        if let Some(name) = name.filter(|name| !name.is_empty()) {
            column = column.child(centred(name.to_owned(), &look.bold));
        }
        let handle = identity.username.as_deref().map(str::trim);
        if let Some(handle) = handle.filter(|handle| !handle.is_empty()) {
            column = column.child(centred(
                format!("@{}", handle.trim_start_matches('@')),
                &look.dim,
            ));
        }
    }
    if let Some(plan) = model.account.ready() {
        column = column.child(centred(plan_label(*plan).to_owned(), &look.dim));
    }
    let _ = strings;
    column.child(gap()).into()
}

/// The five figures, as many to a row as the room holds (oracle:
/// `summary_panel.rs`, 5 from 90 cells, 3 from 54, 2 from 36), each a bold
/// value over its dim label, a blank row after each pair.
fn figures(stats: &Stats, inner: i32, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let value = |amount: Option<String>| amount.unwrap_or_else(|| strings.dash.to_owned());
    let days = |days: Option<i64>| days.map(|days| strings.days(days.max(0) as u64, num));
    let figures = [
        (
            value(stats.lifetime_tokens.map(|v| compact_amount(v as f64))),
            strings.lifetime_tokens,
        ),
        (
            value(stats.peak_daily_tokens.map(|v| compact_amount(v as f64))),
            strings.peak_tokens,
        ),
        (
            value(stats.longest_running_turn_sec.map(duration)),
            strings.longest_chat,
        ),
        (value(days(stats.current_streak_days)), strings.current_streak),
        (value(days(stats.longest_streak_days)), strings.longest_streak),
    ];
    let count = match inner {
        90.. => 5,
        54.. => 3,
        36.. => 2,
        _ => 1,
    };
    let row = |chunk: &[(String, &'static str)], bold: bool| -> AnyWidget {
        Row::new()
            .children(chunk.iter().map(|(amount, label)| {
                let (shown, style) = if bold {
                    (amount.clone(), look.bold.clone())
                } else {
                    ((*label).to_owned(), look.dim.clone())
                };
                Expanded::new(1, line(shown, style).text_align(TextAlign::Center)).into()
            }))
            .into()
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for chunk in figures.chunks(count) {
        column = column.child(row(chunk, true)).child(row(chunk, false)).child(gap());
    }
    column.into()
}

/// The activity chart: the month marks over seven rows of cells, then the
/// legend or the caption of the bar views (oracle: `activity_chart.rs`).
fn chart(
    profile: &Profile,
    model: &Model,
    view: TokenActivityView,
    inner: i32,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
) -> AnyWidget {
    let Some(buckets) = &profile.stats.daily_usage_buckets else {
        return line(strings.history_unavailable, look.dim.clone()).into();
    };
    let chart = Chart::build(view, buckets, model.end_date, inner.max(0) as usize);
    let indent = |cells: i32, child: AnyWidget| -> AnyWidget {
        Padding::new(EdgeInsets::only(cells, 0, 0, 0), child).into()
    };
    if chart.too_narrow {
        return indent(3, line(strings.narrow_chart, look.dim.clone()).into());
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(indent(
            CHART_LEFT_WIDTH as i32,
            line(chart.months.clone(), look.dim.clone()).into(),
        ));
    for (row, cells) in chart.rows.iter().enumerate() {
        let mut spans = vec![Span::styled(gutter(view, row, strings), look.dim.clone())];
        for (at, cell) in cells.iter().enumerate() {
            if at > 0 {
                spans.push(Span::styled(strings.cell_gap, Style::default()));
            }
            spans.push(glyph(view, *cell, strings, look));
        }
        column = column.child(Text::rich(spans).wrap(TextWrap::None).max_lines(1));
    }
    column = column.child(gap());
    let caption = match chart.caption {
        Caption::Legend => {
            let mut spans = vec![
                Span::styled(strings.caption_indent, look.dim.clone()),
                Span::styled(strings.less, look.dim.clone()),
                Span::styled(strings.cell_gap, look.dim.clone()),
            ];
            for level in 0..=4 {
                if level > 0 {
                    spans.push(Span::styled(strings.cell_gap, Style::default()));
                }
                spans.push(glyph(TokenActivityView::Daily, Cell::Level(level), strings, look));
            }
            spans.push(Span::styled(strings.cell_gap, look.dim.clone()));
            spans.push(Span::styled(strings.more, look.dim.clone()));
            Text::rich(spans)
        }
        Caption::Peak { lead, peak } => Text::rich([
            Span::styled(strings.caption_indent, look.dim.clone()),
            Span::styled(lead, look.dim.clone()),
            Span::styled(compact_amount(peak as f64), look.accent.clone()),
        ]),
        Caption::NoActivity => Text::rich([
            Span::styled(strings.caption_indent, look.dim.clone()),
            Span::styled(strings.no_activity, look.dim.clone()),
        ]),
    };
    let _ = num;
    column.child(caption.wrap(TextWrap::None)).into()
}

/// The left gutter of a row: weekday names, or the axis of the bar views.
fn gutter(view: TokenActivityView, row: usize, strings: &Strings) -> String {
    if view == TokenActivityView::Daily {
        return format!(" {} ", strings.weekdays[row]);
    }
    match row {
        0 => strings.axis_max.to_owned(),
        6 => strings.axis_zero.to_owned(),
        _ => strings.axis_none.to_owned(),
    }
}

/// A cell: the hollow glyph for an empty day, the full one for the others;
/// bars are full blocks (oracle: `palette.rs`, the low-colour spelling).
fn glyph(view: TokenActivityView, cell: Cell, strings: &Strings, look: &Look) -> Span {
    let level = match cell {
        Cell::Blank => return Span::styled(strings.cell_gap, Style::default()),
        Cell::Level(level) => level,
    };
    let (shown, style) = match (view, level) {
        (TokenActivityView::Daily, 0) => (strings.cell_empty, look.dim.clone()),
        (TokenActivityView::Daily, _) => (strings.cell_active, look.accent.clone()),
        (_, 0) => (strings.cell_gap, look.dim.clone()),
        (_, _) => (strings.cell_bar, look.accent.clone()),
    };
    Span::styled(shown, style)
}

/// A label at the left and its value at the right of the room it gets.
fn entry(label: String, value: String, label_style: &Style, value_style: &Style) -> AnyWidget {
    Row::new()
        .child(Expanded::new(1, line(label, label_style.clone())))
        .child(line(value, value_style.clone()))
        .into()
}

/// The insights, and the most used plugins and skills beside them from
/// `TWO_COLUMNS` cells, below otherwise.
fn insights(stats: &Stats, inner: i32, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let plain = Style::default();
    let reasoning = match stats.most_used_reasoning_effort.as_deref() {
        Some(effort) => strings.reasoning_value(
            effort,
            &percent(stats.most_used_reasoning_effort_percentage),
            num,
        ),
        None => strings.dash.to_owned(),
    };
    let metrics = [
        (strings.fast_mode, percent(stats.fast_mode_usage_percentage)),
        (strings.reasoning, reasoning),
        (strings.skills_explored, count_value(stats.unique_skills_used)),
        (strings.skills_used, count_value(stats.total_skills_used)),
        (strings.total_chats, count_value(stats.total_threads)),
    ];
    let mut left = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(line(strings.insights, look.bold.clone()))
        .child(gap());
    for (label, value) in metrics {
        left = left.child(entry(label.to_owned(), value, &look.dim, &plain));
    }
    let mut right = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(line(strings.top_uses, look.bold.clone()))
        .child(gap());
    match &stats.top_invocations {
        Some(invocations) => {
            let rows = invocations
                .iter()
                .filter_map(|invocation| {
                    let (prefix, name) = match invocation.kind {
                        InvocationKind::Plugin => ('@', invocation.plugin_name.as_deref()),
                        InvocationKind::Skill => ('$', invocation.skill_name.as_deref()),
                        InvocationKind::Unknown => return None,
                    };
                    let name = name.map(str::trim).filter(|name| !name.is_empty())?;
                    let runs = invocation
                        .usage_count
                        .map_or_else(|| strings.dash.to_owned(), |count| strings.runs(count, num));
                    Some((format!("{prefix}{name}"), runs))
                })
                .take(TOP_ROWS)
                .collect::<Vec<_>>();
            if rows.is_empty() {
                right = right.child(line(strings.no_top, look.dim.clone()));
            }
            for (name, runs) in rows {
                right = right.child(entry(name, runs, &plain, &look.dim));
            }
        }
        None => right = right.child(line(strings.top_unavailable, look.dim.clone())),
    }
    if inner >= TWO_COLUMNS {
        let column = (inner - 4) / 2;
        SizedBox::new(
            Some(2 * column + 4),
            None,
            Some(
                Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(SizedBox::new(Some(column), None, Some(left.into())))
                    .child(SizedBox::fixed(4, 1))
                    .child(SizedBox::new(Some(column), None, Some(right.into())))
                    .into(),
            ),
        )
        .into()
    } else {
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(left)
            .child(gap())
            .child(right)
            .into()
    }
}

/// A blank row, then the date the statistics are as of and, when the backend
/// reported one, that some are unavailable.
fn footer(profile: &Profile, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let Some(metadata) = &profile.metadata else {
        return nothing();
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(gap());
    if let Some(date) = metadata.as_of() {
        column = column.child(line(strings.as_of(date.ymd(), num), look.dim.clone()));
    }
    if metadata
        .stats_error
        .as_deref()
        .is_some_and(|error| !error.trim().is_empty())
    {
        column = column.child(line(strings.stats_error, look.dim.clone()));
    }
    column.into()
}
