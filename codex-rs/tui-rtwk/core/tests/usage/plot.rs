//! Chart scales, signed bands, missing days, narrow viewports (oracle:
//! `analytics/plot_tests.rs`). The text rows of every chart are the oracle's
//! `.snap` rows; the palette sections of those snaps are the terminal's
//! colours, which the view owns (a need of the kit: a palette reader), so
//! the inks are asserted here by the marks themselves.

use codex_tui_core::screens::usage::date::NaiveDate;
use codex_tui_core::screens::usage::models::AccountAnalyticsGrouping as Grouping;
use codex_tui_core::screens::usage::models::AccountAnalyticsReport as Report;
use codex_tui_core::screens::usage::models::*;
use codex_tui_core::screens::usage::normalize::history;
use codex_tui_core::screens::usage::plot::{
    Ink, Plot, Slot, axis_max, categories, chart, parts, peak,
};
use pretty_assertions::assert_eq;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn label(date: NaiveDate, month: bool) -> String {
    let (_, m, d) = date.ymd();
    if month {
        format!("{} {d}", MONTHS[m as usize - 1])
    } else {
        d.to_string()
    }
}

fn value(key: &str, value: f64) -> AccountAnalyticsValue {
    AccountAnalyticsValue {
        key: key.into(),
        label: key.into(),
        value,
    }
}

fn date(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

fn plot(
    days: &[Slot<'_>],
    categories: &[AccountAnalyticsValue],
    cursor: usize,
    width: usize,
    height: usize,
    unit: AccountAnalyticsUnit,
) -> Plot {
    chart(days, categories, cursor, width, height, unit, &label)
}

/// The text of each case of an oracle `.snap` (the rows above its
/// `Palette:` section), the cases separated as the oracle's test joined them.
fn cases(name: &str) -> Vec<Vec<String>> {
    let path = format!(
        "{}/../../tui/src/analytics/snapshots/codex_tui__analytics__plot__tests__{name}.snap",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"));
    let mut lines = text.lines().skip(1);
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
    }
    enum Mode {
        Text,
        Palette,
        Rows,
    }
    let (mut cases, mut current, mut mode) = (Vec::new(), Vec::new(), Mode::Text);
    for line in lines {
        match mode {
            Mode::Text if line == "Palette:" => {
                current.pop();
                cases.push(std::mem::take(&mut current));
                mode = Mode::Palette;
            }
            Mode::Text => current.push(line.to_owned()),
            Mode::Palette if line.starts_with("Rows (column:palette)") => mode = Mode::Rows,
            Mode::Palette => {}
            Mode::Rows if line.is_empty() => mode = Mode::Text,
            Mode::Rows => {}
        }
    }
    cases
}

/// A snapshot of one case: the leading blank row insta drops is dropped.
fn same(plot: &Plot, expected: &[String]) {
    let mut expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    let mut shown: Vec<String> = plot.text();
    while expected.first() == Some(&"") {
        expected.remove(0);
        if shown.first().is_some_and(String::is_empty) {
            shown.remove(0);
        }
    }
    while shown.last().is_some_and(String::is_empty) {
        shown.pop();
    }
    while expected.last() == Some(&"") {
        expected.pop();
    }
    assert_eq!(shown.join("\n"), expected.join("\n"));
}

fn inks(plot: &Plot, ch: char) -> Vec<Ink> {
    plot.rows
        .iter()
        .flatten()
        .filter(|mark| mark.ch == ch)
        .map(|mark| mark.ink)
        .collect()
}

#[test]
fn numeric_axes_use_readable_intervals() {
    use AccountAnalyticsUnit::{Count, Credits, RelativeUsage, Tokens};
    let cases = [
        (82.0, Count, 100.0),
        (4_400.0, Credits, 5_000.0),
        (4_999.0, Credits, 5_000.0),
        (5_001.0, Credits, 6_000.0),
        (49.0, Count, 50.0),
        (1.0, Count, 2.0),
        (3.0, Count, 4.0),
        (1_337.0, Credits, 1_500.0),
        (0.07, Credits, 0.08),
        (0.000004, Credits, 0.000004),
        (82.0, RelativeUsage, 100.0),
        (1.0, Tokens, 2.0),
    ];
    assert_eq!(
        cases.map(|(peak, unit, _)| axis_max(peak, unit)),
        cases.map(|(_, _, expected)| expected)
    );
}

#[test]
fn stacked_parts_keep_the_remainder_and_full_denominator() {
    let values = vec![
        value("a", 4.0),
        value("b", 2.0),
        value("c", 1.0),
        value("d", 6.0),
        value("e", -1.0),
    ];
    let categories = vec![values[1].clone(), values[0].clone(), values[2].clone()];
    assert_eq!(parts(&values, &categories), [2.0, 4.0, 1.0, 5.0]);
    let day = AccountAnalyticsDay {
        date: date("2026-01-15"),
        total: 20.0,
        values,
    };
    let at = day.date;
    assert_eq!(peak(&[(at, Some(&day))], &categories), 20.0);
    let refund = AccountAnalyticsDay {
        total: -1.0,
        values: vec![value("a", 4.0), value("b", -5.0)],
        ..day
    };
    assert_eq!(peak(&[(at, Some(&refund))], &categories), 9.0);
}

fn signed_history() -> AccountAnalyticsHistory {
    let day = |date: &str, total, a, b| AccountAnalyticsDay {
        date: self::date(date),
        total,
        values: vec![value("a", a), value("b", b)],
    };
    AccountAnalyticsHistory {
        unit: AccountAnalyticsUnit::Credits,
        updated_at: None,
        data: vec![
            day("2026-01-13", 8.0, 10.0, -2.0),
            day("2026-01-14", -3.0, 1.0, -4.0),
            day("2026-01-15", 0.0, 3.0, -3.0),
        ],
    }
}

/// `signed_chart_light` and `signed_chart_dark`: the same rows; only the
/// terminal's colours differ, and they are the view's.
#[test]
fn signed_chart_keeps_both_bands() {
    let history = signed_history();
    let days: Vec<Slot> = history
        .data
        .iter()
        .map(|day| (day.date, Some(day)))
        .collect();
    let categories = categories(&history);
    let rendered = plot(&days, &categories, 1, 44, 4, history.unit);
    assert_eq!(rendered.bands, 2);
    for name in ["signed_chart_light", "signed_chart_dark"] {
        same(&rendered, &cases(name)[0]);
    }
    // The refund's partial cells are the reversed lower blocks of series b.
    assert!(inks(&rendered, '▄').contains(&Ink::Refund(1)));
    assert_eq!(inks(&rendered, '▲'), [Ink::Accent]);
}

#[test]
fn missing_zero_and_narrow_charts_keep_the_selection_visible() {
    let zero = AccountAnalyticsDay {
        date: date("2026-01-14"),
        total: 0.0,
        values: vec![value("a", 0.0)],
    };
    let used = AccountAnalyticsDay {
        date: date("2026-01-15"),
        total: 5.0,
        values: vec![value("a", 5.0)],
    };
    let days = [
        (date("2026-01-13"), None),
        (zero.date, Some(&zero)),
        (used.date, Some(&used)),
    ];
    let expected = cases("missing_zero_and_narrow_charts_keep_the_selection_visible");
    for (case, (width, cursor)) in [(36, 0), (36, 1), (8, 2), (1, 2)].into_iter().enumerate() {
        let rendered = plot(
            &days,
            &used.values,
            cursor,
            width,
            3,
            AccountAnalyticsUnit::Count,
        );
        assert_eq!(rendered.bands, 1);
        assert!(rendered.rows.iter().all(|row| row.len() <= width));
        assert_eq!(inks(&rendered, '▲').len(), 1);
        if cursor == 0 {
            let at = |ch| {
                rendered
                    .text()
                    .iter()
                    .find_map(|row| row.chars().position(|c| c == ch))
            };
            assert_eq!(at('—'), at('▲'));
        }
        // The first row of each oracle case is `width=.., cursor=..`.
        same(&rendered, &expected[case][1..]);
    }
}

#[test]
fn zero_credit_history_uses_a_readable_scale() {
    let history = history(
        serde_json::from_str(r#"{"data":[]}"#).unwrap(),
        Report::Credits,
        Grouping::Surface,
        date("2026-01-09"),
        date("2026-01-15"),
    )
    .unwrap()
    .unwrap();
    let days: Vec<Slot> = history
        .data
        .iter()
        .map(|day| (day.date, Some(day)))
        .collect();
    let rendered = plot(&days, &[], 6, 44, 4, history.unit);
    assert_eq!(axis_max(peak(&days, &[]), history.unit), 1.0);
    same(
        &rendered,
        &cases("zero_credit_history_uses_a_readable_scale")[0],
    );
}

#[test]
fn partial_cells_use_the_midpoint_and_descend_from_the_baseline() {
    let categories = vec![value("a", 0.8), value("b", 0.95)];
    let expected = cases("partial_cells_use_the_midpoint_and_descend_from_the_baseline");
    for (case, sign) in [1.0, -1.0].into_iter().enumerate() {
        let day = AccountAnalyticsDay {
            date: date("2026-01-14"),
            total: 1.75 * sign,
            values: categories
                .iter()
                .map(|value| AccountAnalyticsValue {
                    value: value.value * sign,
                    ..value.clone()
                })
                .collect(),
        };
        let days = [(day.date, Some(&day)), (date("2026-01-15"), None)];
        let rendered = plot(&days, &categories, 1, 12, 1, AccountAnalyticsUnit::Credits);
        let mark = rendered.rows[if sign > 0.0 { 1 } else { 3 }][1];
        assert_eq!(mark.ch, if sign > 0.0 { '▇' } else { '▁' });
        assert_eq!(
            mark.ink,
            if sign > 0.0 {
                Ink::Series(1)
            } else {
                Ink::Refund(1)
            }
        );
        same(&rendered, &expected[case][1..]);
    }
}

#[test]
fn oversized_axes_are_hidden_instead_of_truncated() {
    let expected = cases("oversized_axes_are_hidden_instead_of_truncated");
    let runs = [
        (8, 100_000_000.0),
        (30, 100_000_000.0),
        (35, 100_000_000.0),
        (44, 100_000_000.0),
        (30, 1e30),
    ];
    for (case, (width, total)) in runs.into_iter().enumerate() {
        let day = AccountAnalyticsDay {
            date: date("2026-01-15"),
            total,
            values: vec![value("a", total)],
        };
        let days = [(day.date, Some(&day))];
        let rendered = plot(
            &days,
            &day.values,
            0,
            width,
            4,
            AccountAnalyticsUnit::Tokens,
        );
        let baseline = &rendered.text()[5];
        assert_eq!(baseline.starts_with('─'), width < 30 || total == 1e30);
        if width == 8 {
            assert!(rendered.text()[0].contains("100M"));
        }
        same(&rendered, &expected[case][1..]);
    }
}

#[test]
fn selected_callout_keeps_the_authoritative_daily_total() {
    let day = AccountAnalyticsDay {
        date: date("2026-01-15"),
        total: 20.0,
        values: vec![value("a", 8.0), value("b", 4.0)],
    };
    let days = [(day.date, Some(&day))];
    let rendered = plot(&days, &day.values, 0, 44, 4, AccountAnalyticsUnit::Credits);
    let text = rendered.text().join("\n");
    assert!(text.contains("20.00"));
    assert!(!text.contains("12.00"));
    same(
        &rendered,
        &cases("selected_callout_keeps_the_authoritative_daily_total")[0],
    );
}

#[test]
fn narrow_long_range_charts_never_truncate_dates() {
    let start = date("2026-01-01");
    let rows: Vec<AccountAnalyticsDay> = start
        .iter_days()
        .take(30)
        .map(|date| AccountAnalyticsDay {
            date,
            total: 1.0,
            values: vec![value("a", 1.0)],
        })
        .collect();
    let days: Vec<Slot> = rows.iter().map(|day| (day.date, Some(day))).collect();
    let expected = cases("narrow_long_range_charts_never_truncate_dates");
    for (case, (width, last)) in [(1, ""), (2, "15"), (5, "15"), (6, "Jan 15")]
        .into_iter()
        .enumerate()
    {
        let rendered = plot(
            &days,
            &rows[0].values,
            14,
            width,
            3,
            AccountAnalyticsUnit::Count,
        );
        assert_eq!(rendered.text().last().unwrap().trim(), last);
        same(&rendered, &expected[case][1..]);
    }
}

#[test]
fn tiny_segments_keep_their_series_color_beside_a_large_day() {
    let large = AccountAnalyticsDay {
        date: date("2026-01-14"),
        total: 100.0,
        values: vec![value("a", 100.0)],
    };
    let expected = cases("tiny_segments_keep_their_series_color_beside_a_large_day");
    for (case, sign) in [1.0, -1.0].into_iter().enumerate() {
        let tiny = AccountAnalyticsDay {
            date: date("2026-01-15"),
            total: 0.01 * sign,
            values: vec![value("a", 0.01 * sign)],
        };
        let days = [(large.date, Some(&large)), (tiny.date, Some(&tiny))];
        let rendered = plot(
            &days,
            &large.values,
            0,
            44,
            4,
            AccountAnalyticsUnit::Credits,
        );
        let glyph = if sign > 0.0 { '▁' } else { '▇' };
        let tiny_cells = inks(&rendered, glyph);
        assert!(!tiny_cells.is_empty());
        // The tiny day keeps series a's colour (the refund on the reversed cell).
        let own = if sign > 0.0 {
            Ink::Series(0)
        } else {
            Ink::Refund(0)
        };
        assert!(tiny_cells.contains(&own));
        same(&rendered, &expected[case][1..]);
    }
}

#[test]
fn duplicate_message_dates_render_the_full_other_remainder() {
    let at = date("2026-01-15");
    let response = serde_json::from_value(serde_json::json!({"data": [
        {"date": "2026-01-15", "totals": {"turns": 10}, "clients": [{"client_id": "CODEX_CLI", "turns": 8}]},
        {"date": "2026-01-15", "totals": {"turns": 10}, "clients": [{"client_id": "CODEX_CLI", "turns": 8}]}
    ]}))
    .unwrap();
    let history = history(response, Report::Messages, Grouping::Surface, at, at)
        .unwrap()
        .unwrap();
    let day = &history.data[0];
    let rendered = plot(
        &[(at, Some(day))],
        &day.values,
        0,
        44,
        10,
        AccountAnalyticsUnit::Count,
    );
    same(
        &rendered,
        &cases("duplicate_message_dates_render_the_full_other_remainder")[0],
    );
}

#[test]
fn seven_day_totals_render_without_selecting_each_bar() {
    let start = date("2026-01-01");
    let rows: Vec<AccountAnalyticsDay> = (0..7)
        .map(|index| AccountAnalyticsDay {
            date: start.plus_days(index),
            total: 111.0 + index as f64 * 100.0,
            values: vec![value("a", 111.0 + index as f64 * 100.0)],
        })
        .collect();
    let days: Vec<Slot> = rows.iter().map(|day| (day.date, Some(day))).collect();
    let rendered = plot(
        &days,
        &rows[0].values,
        3,
        100,
        8,
        AccountAnalyticsUnit::Count,
    );
    let text = rendered.text().join("\n");
    for day in &rows {
        assert!(text.contains(&day.total.to_string()));
    }
    same(&rendered, &cases("seven_day_totals")[0]);
}

#[test]
fn narrow_equal_height_totals_remain_separate_numbers() {
    let start = date("2026-01-01");
    let rows: Vec<AccountAnalyticsDay> = (0..7)
        .map(|index| AccountAnalyticsDay {
            date: start.plus_days(index),
            total: 111.0,
            values: vec![value("a", 111.0)],
        })
        .collect();
    let days: Vec<Slot> = rows.iter().map(|day| (day.date, Some(day))).collect();
    // The oracle joins three widths with a blank row and no `Palette:`
    // section: one case, three charts.
    let mut shown = Vec::new();
    for width in [21, 28, 35] {
        let rendered = plot(
            &days,
            &rows[0].values,
            3,
            width,
            4,
            AccountAnalyticsUnit::Count,
        );
        let text = rendered.text().join("\n");
        assert!(text.contains("111"));
        assert!(!text.contains("111111"));
        shown.push(text);
    }
    let path = format!(
        "{}/../../tui/src/analytics/snapshots/codex_tui__analytics__plot__tests__narrow_equal_height_totals_remain_separate_numbers.snap",
        env!("CARGO_MANIFEST_DIR")
    );
    let snap = std::fs::read_to_string(path).unwrap();
    let body = snap.splitn(3, "---\n").nth(2).unwrap().trim_end();
    assert_eq!(
        shown.join("\n\n").trim_matches('\n'),
        body.trim_matches('\n')
    );
}

/// The text rows of an oracle `analytics_tests` snapshot (below its `---`), trailing blanks cut.
fn panel_rows(name: &str) -> Vec<String> {
    let path = format!(
        "{}/../../tui/src/snapshots/codex_tui__analytics__tests__{name}.snap",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"));
    text.lines()
        .skip(1)
        .skip_while(|line| *line != "---")
        .skip(1)
        .map(|line| line.trim_end().to_owned())
        .collect()
}

/// The oracle's `fixture::history(0, 1, 2)` day: 40 units a day (73 on the last week's days),
/// split by model as 60/25/10 and the rest.
fn model_day(date: &str, total: u32) -> AccountAnalyticsDay {
    let mut parts = [60, 25, 10].map(|weight| total * weight / 100).to_vec();
    parts.push(total - parts.iter().sum::<u32>());
    AccountAnalyticsDay {
        date: self::date(date),
        total: f64::from(total),
        values: ["GPT-5.6-Sol", "GPT-5.6-Terra", "GPT-5.5", "Other"]
            .into_iter()
            .zip(parts)
            .map(|(key, part)| value(key, f64::from(part)))
            .collect(),
    }
}

/// The oracle's `analytics_preserves_full_denominators_and_signed_small_credits` Usage panel is
/// `panel(Usage, 54, 2)`: a plot of two rows, which a screen of the page never draws (under 15
/// body rows the plot is hidden); its chart rows are the oracle's snapshot.
#[test]
fn a_day_of_one_hundred_units_keeps_its_denominator_in_a_two_row_chart() {
    let day = AccountAnalyticsDay {
        date: date("2026-09-02"),
        total: 100.0,
        values: vec![value("goal", 30.0)],
    };
    let history = AccountAnalyticsHistory {
        unit: AccountAnalyticsUnit::RelativeUsage,
        updated_at: None,
        data: vec![day.clone()],
    };
    let days: Vec<Slot> = (27..=31)
        .map(|d| (date(&format!("2026-08-{d}")), None))
        .chain([
            (date("2026-09-01"), None),
            (day.date, Some(&day)),
        ])
        .collect();
    let rendered = plot(&days, &categories(&history), 6, 54, 2, history.unit);
    let want = panel_rows("analytics_preserves_full_denominators_and_signed_small_credits");
    // the oracle's rows of the panel between its lead lines and the rule: the chart
    same(&rendered, &want[3..9]);
}

/// `analytics_series_rank_by_range_and_keep_calendar_ticks_stable`: the series rank by the sum
/// of their range (`Other`, `GPT-5.6-Sol`, `GPT-5.6-Terra`), the tick row is the same wherever
/// the cursor stands and one `▲` marks it; the crowded chart (54 cells, 10 rows, cursor on the
/// day of 0.4) is the oracle's snapshot. The screen of the page holds the legend and the
/// ticks (tests/oracle/usage.rs); no screen is 54 cells with a 10-row plot.
#[test]
fn the_crowded_month_chart_keeps_its_ranked_series_and_stable_ticks() {
    let dates: Vec<String> = (0..30)
        .map(|offset| {
            let start = date("2026-08-04");
            start.iter_days().nth(offset).unwrap().to_string()
        })
        .collect();
    let mut data: Vec<AccountAnalyticsDay> = dates
        .iter()
        .enumerate()
        .map(|(index, date)| {
            let total = if index < 23 {
                40
            } else {
                [20, 73, 64, 91, 81, 100, 86][index - 23]
            };
            let mut day = model_day(date, total);
            day.values[3].value += 500.0;
            day.total += 500.0;
            day
        })
        .collect();
    let history = AccountAnalyticsHistory {
        unit: AccountAnalyticsUnit::RelativeUsage,
        updated_at: None,
        data: data.clone(),
    };
    let series = categories(&history);
    assert_eq!(
        series
            .iter()
            .take(3)
            .map(|series| series.label.as_str())
            .collect::<Vec<_>>(),
        ["Other", "GPT-5.6-Sol", "GPT-5.6-Terra"]
    );
    data[8].total = 0.4;
    for value in &mut data[8].values {
        value.value = 0.1;
    }
    let days: Vec<Slot> = data.iter().map(|day| (day.date, Some(day))).collect();
    let mut ticks = None;
    for cursor in [0, 7, 8, 25, 29] {
        let rendered = plot(&days, &series, cursor, 96, 10, history.unit);
        let text = rendered.text();
        let labels = text.last().cloned().unwrap();
        if let Some(ticks) = &ticks {
            assert_eq!(&labels, ticks, "cursor {cursor}");
        }
        ticks = Some(labels);
        assert_eq!(text.iter().filter(|row| row.contains('▲')).count(), 1);
    }
    let crowded = plot(&days, &series, 8, 54, 10, history.unit);
    let text = crowded.text();
    assert!(text.iter().any(|row| row.contains('┊')));
    assert!(text.iter().any(|row| row.contains("0.4")));
    same(
        &crowded,
        &panel_rows("analytics_series_rank_by_range_and_keep_calendar_ticks_stable"),
    );
}
