//! The row under a turn's last cell: how long it worked and when it ended
//! (oracle `FinalMessageSeparator`, the "Worked for" line).

use chrono::{Datelike, Local, TimeZone};
use rtwk::AnyWidget;
use rtwk::layout::{EdgeInsets, Padding};
use rtwk::style::Attrs;
use rtwk::widgets::Text;

use codex_tui_core::threads::TurnTiming;

use super::strings::Strings;
use crate::app::theme::Tokens;

pub fn completion(timing: TurnTiming, now: i64, strings: &Strings, tokens: &Tokens) -> AnyWidget {
    let mut parts = Vec::new();
    if let Some(millis) = timing.duration_ms {
        let elapsed = if millis < 1000 {
            "<1s".to_string()
        } else {
            crate::pages::session::activity::compact_elapsed((millis / 1000) as u64)
        };
        parts.push(strings.label(strings.worked_for, &elapsed));
    }
    if let Some(at) = timing
        .completed_at
        .and_then(|at| Local.timestamp_opt(at, 0).single())
    {
        let today = Local.timestamp_opt(now, 0).single();
        let format = match today {
            Some(today) if today.date_naive() == at.date_naive() => "%H:%M",
            Some(today) if today.year() == at.year() => "%b %-d %H:%M",
            _ => "%Y-%m-%d %H:%M",
        };
        parts.push(at.format(format).to_string());
    }
    Padding::new(
        EdgeInsets::only(2, 1, 1, 0),
        Text::new(parts.join(" • ")).style(tokens.dim().attrs(Attrs::DIM)),
    )
    .into()
}
