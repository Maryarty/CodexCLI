//! The row under a turn's last cell: how long it worked, when it ended and what the turn cost
//! (oracle `history_cell/separators.rs` `FinalMessageSeparator`, "Worked for 2m 5s • 2:32 PM").
//! Flutter: a `Padding` over a wrapping `Text`, the indent chosen from the measured width
//! (`LayoutBuilder` in the reader, the pane's measured width for terminal history). Nothing is
//! drawn for a turn with no duration, no time and no metrics.

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone};
use rtwk::layout::{BoxWidget, EdgeInsets, Padding};
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::cells::{ClockFormat, RuntimeMetrics, Totals, ViewerClock};
use codex_tui_core::threads::TurnTiming;

use super::strings::Strings;

/// The indent of every row (oracle: two spaces) while the width is more than two.
const INDENT: i32 = 2;
const INDENT_FROM: i32 = 3;

/// What the label reads besides the turn's own timing.
#[derive(Clone, Copy)]
pub struct Facts<'a> {
    /// The viewer's date when the host has provided a clock.
    pub today: Option<NaiveDate>,
    pub clock: ViewerClock,
    pub metrics: Option<&'a RuntimeMetrics>,
}

/// The separator row of a finished turn; nothing when there is nothing to say.
pub fn completion(
    timing: TurnTiming,
    facts: Facts<'_>,
    strings: &Strings,
    committed_width: Option<i32>,
) -> Option<AnyWidget> {
    let label = label(timing, facts, strings)?;
    let row = move |width| {
        let left = if width >= INDENT_FROM { INDENT } else { 0 };
        // The oracle's text is dim and default-coloured, its indent too: the box paints the DIM
        // under the indent blanks, which a Padding leaves as they are.
        let dim = Style::new().attrs(Attrs::DIM);
        Padding::new(
            EdgeInsets::only(0, 1, 0, 0),
            BoxWidget::new()
                .style(dim.clone())
                .padding(EdgeInsets::only(left, 0, 0, 0))
                .child(Text::new(label.clone()).style(dim)),
        )
        .into()
    };
    // The history sink asks for intrinsic height before the first layout. Give
    // it the native Padding and Text so it can measure both the spacer and the label.
    Some(match committed_width {
        Some(width) => row(width),
        None => LayoutBuilder::new(move |_, room| row(room.max_width)).into(),
    })
}

/// The text of the separator: the parts that are known, joined (a folded turn's disclosure
/// row reads it too, oracle `FinalMessageSeparator::display_label`).
pub fn label(timing: TurnTiming, facts: Facts<'_>, strings: &Strings) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(millis) = timing.duration_ms {
        let elapsed = elapsed(u64::try_from(millis / 1_000).unwrap_or(0), strings);
        parts.push(strings.label(strings.worked_for, &elapsed));
    }
    if let Some(at) = timing
        .completed_at
        .and_then(|at| civil(at, facts.clock.utc_offset))
    {
        parts.push(stamp(at, facts, strings));
    }
    if let Some(metrics) = facts
        .metrics
        .and_then(|metrics| metrics_label(metrics, strings))
    {
        parts.push(metrics);
    }
    (!parts.is_empty()).then(|| parts.join(strings.separator_join))
}

/// `1h 0m 5s`, `2m 5s`, `12s`, `<1s`: a duration the turn is known to have taken.
fn elapsed(seconds: u64, strings: &Strings) -> String {
    let (hours, minutes, rest) = (seconds / 3_600, seconds % 3_600 / 60, seconds % 60);
    let number = |value: u64| value.to_string();
    if hours > 0 {
        strings.fill(
            strings.elapsed_hours,
            &[&number(hours), &number(minutes), &number(rest)],
        )
    } else if minutes > 0 {
        strings.fill(strings.elapsed_minutes, &[&number(minutes), &number(rest)])
    } else if rest == 0 {
        strings.elapsed_under_a_second.to_owned()
    } else {
        strings.fill(strings.elapsed_seconds, &[&number(rest)])
    }
}

/// `epoch` seconds as the wall time of the viewer's zone.
fn civil(epoch: i64, utc_offset: Option<i32>) -> Option<NaiveDateTime> {
    match utc_offset {
        Some(seconds) => {
            let zone = FixedOffset::east_opt(seconds)?;
            Some(
                DateTime::from_timestamp(epoch, 0)?
                    .with_timezone(&zone)
                    .naive_local(),
            )
        }
        None => Some(Local.timestamp_opt(epoch, 0).single()?.naive_local()),
    }
}

/// The date is the part of the viewer's clock that can change an existing label.
pub fn viewer_date(now: i64, clock: ViewerClock) -> Option<NaiveDate> {
    (now > 0)
        .then(|| civil(now, clock.utc_offset))
        .flatten()
        .map(|now| now.date())
}

/// The time the turn ended, with its date when the viewer's today is another day (and its year
/// when that is another year). The viewer's today is unknown until the host's first clock: then
/// the turn ended today.
fn stamp(at: NaiveDateTime, facts: Facts<'_>, strings: &Strings) -> String {
    let today = facts.today.unwrap_or_else(|| at.date());
    let date = if at.date() == today {
        ""
    } else if at.year() == today.year() {
        strings.stamp_other_day
    } else {
        strings.stamp_other_year
    };
    let time = match facts.clock.format {
        ClockFormat::TwelveHour => strings.time_twelve_hour,
        ClockFormat::TwentyFourHour => strings.time_twenty_four_hour,
    };
    format!("{}{}", at.format(date), at.format(time))
}

/// `Local tools: 3 calls (2.5s) • Inference: ...`: the metrics that are not zero.
pub(super) fn metrics_label(metrics: &RuntimeMetrics, strings: &Strings) -> Option<String> {
    let mut parts = Vec::new();
    let count = |totals: Totals| totals.count.to_string();
    let plural = |count: u64, words: (&'static str, &'static str)| {
        if count == 1 { words.0 } else { words.1 }
    };
    let (tools, api) = (metrics.tool_calls, metrics.api_calls);
    if tools.count > 0 {
        parts.push(strings.fill(
            strings.metric_local_tools,
            &[
                &count(tools),
                plural(tools.count, strings.metric_call),
                &duration(tools.duration_ms, strings),
            ],
        ));
    }
    if api.count > 0 {
        parts.push(strings.fill(
            strings.metric_inference,
            &[
                &count(api),
                plural(api.count, strings.metric_call),
                &duration(api.duration_ms, strings),
            ],
        ));
    }
    if metrics.websocket_calls.count > 0 {
        parts.push(strings.fill(
            strings.metric_websocket_sent,
            &[
                &count(metrics.websocket_calls),
                &duration(metrics.websocket_calls.duration_ms, strings),
            ],
        ));
    }
    let streams = metrics.streaming_events;
    if streams.count > 0 {
        parts.push(strings.fill(
            strings.metric_streams,
            &[
                plural(streams.count, strings.metric_stream),
                &count(streams),
                plural(streams.count, strings.metric_event),
                &duration(streams.duration_ms, strings),
            ],
        ));
    }
    if metrics.websocket_events.count > 0 {
        parts.push(strings.fill(
            strings.metric_events_received,
            &[
                &count(metrics.websocket_events),
                &duration(metrics.websocket_events.duration_ms, strings),
            ],
        ));
    }
    let timed = |template: &str, millis: u64| {
        (millis > 0).then(|| strings.fill(template, &[&duration(millis, strings)]))
    };
    parts.extend(timed(
        strings.metric_overhead,
        metrics.responses_api_overhead_ms,
    ));
    parts.extend(timed(
        strings.metric_inference_time,
        metrics.responses_api_inference_time_ms,
    ));
    let first_token = engine_times(
        strings,
        metrics.responses_api_engine_iapi_ttft_ms > 0,
        duration(metrics.responses_api_engine_iapi_ttft_ms, strings),
        metrics.responses_api_engine_service_ttft_ms > 0,
        duration(metrics.responses_api_engine_service_ttft_ms, strings),
    );
    parts.extend(first_token.map(|times| strings.fill(strings.metric_first_token, &[&times])));
    let (iapi, service) = (
        metrics.responses_api_engine_iapi_tbt_ms,
        metrics.responses_api_engine_service_tbt_ms,
    );
    let between_tokens = engine_times(
        strings,
        iapi > 0.0,
        duration(iapi.round() as u64, strings),
        service > 0.0,
        duration(service.round() as u64, strings),
    );
    parts
        .extend(between_tokens.map(|times| strings.fill(strings.metric_between_tokens, &[&times])));
    (!parts.is_empty()).then(|| parts.join(strings.separator_join))
}

/// `410ms (iapi) 460ms (service)`: the engine's figures that are there.
fn engine_times(
    strings: &Strings,
    has_iapi: bool,
    iapi: String,
    has_service: bool,
    service: String,
) -> Option<String> {
    let mut times = Vec::new();
    if has_iapi {
        times.push(strings.fill(strings.metric_iapi, &[&iapi]));
    }
    if has_service {
        times.push(strings.fill(strings.metric_service, &[&service]));
    }
    (!times.is_empty()).then(|| times.join(" "))
}

/// `1.2s` from a second on, `700ms` below it.
fn duration(millis: u64, strings: &Strings) -> String {
    if millis >= 1_000 {
        strings.fill(
            strings.metric_seconds,
            &[&format!("{:.1}", millis as f64 / 1_000.0)],
        )
    } else {
        strings.fill(strings.metric_milliseconds, &[&millis.to_string()])
    }
}
