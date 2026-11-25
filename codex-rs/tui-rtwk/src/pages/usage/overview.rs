//! The Overview tab (oracle: `analytics/summary_panel.rs`, `activity_chart.rs`):
//! the profile block, the five figures, the token activity chart, the insights
//! beside the most used plugins and skills, the date the statistics are as of.
//! The block is at most 110 cells wide and centred; the data is core's
//! `Profile` and `Chart` (the chart's cells and levels are core's pure
//! functions, painted here).

use rtwk::l10n::NumSyms;
use rtwk::layout::{
    Center, Column, CrossAxisAlignment, CustomPaint, EdgeInsets, Expanded, MainAxisSize, Padding,
    Row, SizedBox,
};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextAlign, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::usage::activity::{CHART_LEFT_WIDTH, Caption, Chart, Square, VIEWS};
use codex_tui_core::screens::usage::data::compact_amount;
use codex_tui_core::screens::usage::profile::{
    InvocationKind, Profile, Stats, count_value, duration, percent,
};
use codex_tui_core::screens::usage::{Load, Section, TokenActivityView, View};

use super::Look;
use super::cards::{Fit, Item};
use super::palette::{ActivityPainter, Glyphs, Seg, Source};
use super::strings::Strings;
use super::{gap, line, nothing};

/// The widest the block gets.
const BLOCK: i32 = 110;
/// From this inner width the insights and the most used sit side by side.
const TWO_COLUMNS: i32 = 76;
const TOP_ROWS: usize = 5;

/// The Overview's body for `width` cells of room.
pub fn body(
    model: &View<'_>,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    width: i32,
) -> AnyWidget {
    let profile = match &model.profile {
        Load::Ready(profile) => profile.clone(),
        other => {
            return other.note().map_or_else(nothing, |note| {
                line(strings.note(&note).to_owned(), Style::default()).into()
            });
        }
    };
    let inner = width.min(BLOCK);
    let (strings, num, look) = (*strings, *num, look.clone());
    let owned = model.chosen();
    LayoutBuilder::new(move |_, constraints| {
        let model = owned.view();
        let inner = constraints.max_width.min(inner).max(1);
        centred_block(&model, &profile, inner, &strings, &num, &look).0
    })
    .into()
}

/// The profile block `inner` cells wide, centred, and the rows it takes.
fn centred_block(
    model: &View<'_>,
    profile: &Profile,
    inner: i32,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
) -> (AnyWidget, usize) {
    let view = VIEWS[model.group_of(Section::Summary)];
    let items = [
        identity(model, profile, strings, look),
        figures(&profile.stats, inner, strings, num, look),
        Item::one(line(strings.activity, look.bold.clone())),
        Item::one(line(strings.last_year, look.dim.clone())),
        Item::one(gap()),
        chart(profile, model, view, inner, strings, num, look),
        Item::one(gap()),
        insights(&profile.stats, inner, strings, num, look),
        footer(profile, strings, num, look),
    ];
    let rows = items.iter().map(|item| item.rows).sum();
    let block = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .children(items.into_iter().map(|item| item.widget));
    let widget = Center::new(SizedBox::new(Some(inner), None, Some(block.into())))
        .height_factor(1)
        .into();
    (widget, rows)
}

/// What a card holds of the Overview: the block at most `BLOCK` cells wide
/// in the card's width, or why there is no profile (oracle: `summary_lines`).
pub fn card(
    model: &View<'_>,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    fit: Fit,
) -> Vec<Item> {
    match &model.profile {
        Load::Ready(profile) => {
            let inner = i32::try_from(fit.width).unwrap_or(i32::MAX).min(BLOCK);
            let (widget, rows) = centred_block(model, profile, inner, strings, num, look);
            vec![Item::new(widget, rows)]
        }
        other => other
            .note()
            .map(|note| {
                let message = strings.note(&note);
                Item::new(
                    Text::new(message.to_owned()).wrap(TextWrap::Word),
                    fit.rows(message),
                )
            })
            .into_iter()
            .collect(),
    }
}

fn centred(shown: String, style: &Style) -> AnyWidget {
    line(shown, style.clone())
        .text_align(TextAlign::Center)
        .into()
}

/// The name, the handle and the plan, centred, then a blank row.
fn identity(model: &View<'_>, profile: &Profile, strings: &Strings, look: &Look) -> Item {
    let mut rows = 1;
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(identity) = &profile.identity {
        let name = identity.display_name.as_deref().map(str::trim);
        if let Some(name) = name.filter(|name| !name.is_empty()) {
            rows += 1;
            column = column.child(centred(name.to_owned(), &look.bold));
        }
        let handle = identity.username.as_deref().map(str::trim);
        if let Some(handle) = handle.filter(|handle| !handle.is_empty()) {
            rows += 1;
            column = column.child(centred(
                format!("@{}", handle.trim_start_matches('@')),
                &look.dim,
            ));
        }
    }
    if let Some(plan) = model.account.ready() {
        rows += 1;
        column = column.child(centred(strings.plan_name(*plan).to_owned(), &look.dim));
    }
    Item::new(column.child(gap()), rows)
}

/// The five figures, as many to a row as the room holds (oracle:
/// `summary_panel.rs`, 5 from 90 cells, 3 from 54, 2 from 36), each a bold
/// value over its dim label, a blank row after each pair.
fn figures(stats: &Stats, inner: i32, strings: &Strings, num: &NumSyms, look: &Look) -> Item {
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
        (
            value(days(stats.current_streak_days)),
            strings.current_streak,
        ),
        (
            value(days(stats.longest_streak_days)),
            strings.longest_streak,
        ),
    ];
    let count = match inner {
        90.. => 5,
        54.. => 3,
        36.. => 2,
        _ => 1,
    };
    let row = |chunk: &[(String, &'static str)], bold: bool| -> AnyWidget {
        // Oracle: the room divides evenly among the row's figures; the
        // remainder stays unused at the right.
        let cell = inner / i32::try_from(chunk.len()).unwrap_or(1);
        Row::new()
            .children(chunk.iter().map(|(amount, label)| {
                let (shown, style) = if bold {
                    (amount.clone(), look.bold.clone())
                } else {
                    ((*label).to_owned(), look.dim.clone())
                };
                SizedBox::new(
                    Some(cell),
                    Some(1),
                    Some(line(shown, style).text_align(TextAlign::Center).into()),
                )
                .into()
            }))
            .into()
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut rows = 0;
    for chunk in figures.chunks(count) {
        rows += 3;
        column = column
            .child(row(chunk, true))
            .child(row(chunk, false))
            .child(gap());
    }
    Item::new(column, rows)
}

/// The activity chart: the month marks over seven rows of cells, then the
/// legend or the caption of the bar views (oracle: `activity_chart.rs`).
fn chart(
    profile: &Profile,
    model: &View<'_>,
    view: TokenActivityView,
    inner: i32,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
) -> Item {
    let Some(buckets) = &profile.stats.daily_usage_buckets else {
        return Item::one(line(strings.history_unavailable, look.dim.clone()));
    };
    let chart = Chart::build(view, buckets, model.end_date, inner.max(0) as usize);
    let indent = |cells: i32, child: AnyWidget| -> AnyWidget {
        Padding::new(EdgeInsets::only(cells, 0, 0, 0), child).into()
    };
    if chart.too_narrow {
        return Item::one(indent(
            3,
            line(strings.narrow_chart, look.dim.clone()).into(),
        ));
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(indent(
            CHART_LEFT_WIDTH as i32,
            line(chart.months.clone(), look.dim.clone()).into(),
        ));
    let glyphs = Glyphs {
        empty: strings.cell_empty,
        active: strings.cell_active,
        bar: strings.cell_bar,
        gap: strings.cell_gap,
    };
    let source = Source::of(look, glyphs, view);
    let rows: Vec<Vec<Seg>> = chart
        .rows
        .iter()
        .enumerate()
        .map(|(row, cells)| {
            let mut segs = vec![Seg::Text(gutter(view, row, strings), look.dim.clone())];
            for (at, cell) in cells.iter().enumerate() {
                if at > 0 {
                    segs.push(Seg::Text(strings.cell_gap.to_owned(), Style::default()));
                }
                segs.push(Seg::Cell(*cell));
            }
            segs
        })
        .collect();
    column = column.child(SizedBox::new(
        None,
        Some(chart.rows.len() as i32),
        Some(
            CustomPaint::new()
                .painter(ActivityPainter {
                    source: source.clone(),
                    rows,
                })
                .into(),
        ),
    ));
    column = column.child(gap());
    let caption: AnyWidget = match chart.caption {
        Caption::Legend => {
            let mut segs = vec![
                Seg::Text(strings.caption_indent.to_owned(), look.dim.clone()),
                Seg::Text(strings.less.to_owned(), look.dim.clone()),
                Seg::Text(strings.cell_gap.to_owned(), look.dim.clone()),
            ];
            for level in 0..=4 {
                if level > 0 {
                    segs.push(Seg::Text(strings.cell_gap.to_owned(), Style::default()));
                }
                segs.push(Seg::Cell(Square::Level(level)));
            }
            segs.push(Seg::Text(strings.cell_gap.to_owned(), look.dim.clone()));
            segs.push(Seg::Text(strings.more.to_owned(), look.dim.clone()));
            SizedBox::new(
                None,
                Some(1),
                Some(
                    CustomPaint::new()
                        .painter(ActivityPainter {
                            source: Source {
                                view: TokenActivityView::Daily,
                                ..source
                            },
                            rows: vec![segs],
                        })
                        .into(),
                ),
            )
            .into()
        }
        Caption::Peak { lead, peak } => Text::rich([
            Span::styled(strings.caption_indent, look.dim.clone()),
            Span::styled(lead, look.dim.clone()),
            Span::styled(compact_amount(peak as f64), look.accent.clone()),
        ])
        .wrap(TextWrap::None)
        .into(),
        Caption::NoActivity => Text::rich([
            Span::styled(strings.caption_indent, look.dim.clone()),
            Span::styled(strings.no_activity, look.dim.clone()),
        ])
        .wrap(TextWrap::None)
        .into(),
    };
    let _ = num;
    // The months, the rows, a blank row and the caption.
    let rows = chart.rows.len() + 3;
    Item::new(column.child(caption), rows)
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

/// A label at the left and its value at the right of the room it gets.
fn entry(label: String, value: String, label_style: &Style, value_style: &Style) -> AnyWidget {
    Row::new()
        .child(Expanded::new(1, line(label, label_style.clone())))
        .child(line(value, value_style.clone()))
        .into()
}

/// The insights, and the most used plugins and skills beside them from
/// `TWO_COLUMNS` cells, below otherwise.
fn insights(stats: &Stats, inner: i32, strings: &Strings, num: &NumSyms, look: &Look) -> Item {
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
        (
            strings.skills_explored,
            count_value(stats.unique_skills_used),
        ),
        (strings.skills_used, count_value(stats.total_skills_used)),
        (strings.total_chats, count_value(stats.total_threads)),
    ];
    let metrics_count = metrics.len();
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
    // The heading, a blank row and the rows below.
    let mut right_rows = 2;
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
                right_rows += 1;
                right = right.child(line(strings.no_top, look.dim.clone()));
            }
            for (name, runs) in rows {
                right_rows += 1;
                right = right.child(entry(name, runs, &plain, &look.dim));
            }
        }
        None => {
            right_rows += 1;
            right = right.child(line(strings.top_unavailable, look.dim.clone()));
        }
    }
    // The heading, a blank row and the metrics.
    let left_rows = 2 + metrics_count;
    if inner >= TWO_COLUMNS {
        let column = (inner - 4) / 2;
        Item::new(
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
            ),
            left_rows.max(right_rows),
        )
    } else {
        Item::new(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(left)
                .child(gap())
                .child(right),
            left_rows + 1 + right_rows,
        )
    }
}

/// A blank row, then the date the statistics are as of and, when the backend
/// reported one, that some are unavailable.
fn footer(profile: &Profile, strings: &Strings, num: &NumSyms, look: &Look) -> Item {
    let Some(metadata) = &profile.metadata else {
        return Item::new(nothing(), 0);
    };
    let mut rows = 1;
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(gap());
    if let Some(date) = metadata.as_of() {
        rows += 1;
        column = column.child(line(strings.as_of(date.ymd(), num), look.dim.clone()));
    }
    if metadata
        .stats_error
        .as_deref()
        .is_some_and(|error| !error.trim().is_empty())
    {
        rows += 1;
        column = column.child(line(strings.stats_error, look.dim.clone()));
    }
    Item::new(column, rows)
}
