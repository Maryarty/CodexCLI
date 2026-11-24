//! The footer of Find (oracle `transcript_view/search_footer.rs`): the query
//! row (`Find: ` and the editor) and the status row under it, which takes the
//! first of its spellings that fits.
//!
//! The state the status is read from is the finder's (`Status`: where the
//! search stands, whether the reader is on a match and reading it, whether the
//! query was cut); this file only turns it into words. Flutter: a `Row` of a
//! `Text` and an `Expanded` field, and a `LayoutBuilder` + `segmenter_of` that
//! picks the spelling (as `follow.rs`).

use rtwk::layout::{Expanded, Row};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use super::strings::Strings;

/// Where the search stands (oracle `search.rs` `Progress`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Nothing asked since the query last changed, or the answer is shown.
    Idle,
    /// The query changed: the scan starts again.
    Restart,
    /// Entries are being read.
    Scanning,
    /// The match lies in history that has not been loaded.
    AwaitingHistory,
    /// A match is shown.
    Found,
    /// No entry is left to read in the direction.
    Exhausted,
}

/// What the status row is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub progress: Progress,
    /// The reader has accepted the match and reads it (the footer says `Find · …`).
    pub reading: bool,
    /// A match is the current one.
    pub has_match: bool,
    /// The query editor is empty.
    pub query_empty: bool,
    /// The query was cut to 4 KiB.
    pub truncated: bool,
    /// Loading older history failed.
    pub history_failed: bool,
}

impl Status {
    /// A finder that has not been asked anything.
    pub fn idle() -> Status {
        Status {
            progress: Progress::Idle,
            reading: false,
            has_match: false,
            query_empty: true,
            truncated: false,
            history_failed: false,
        }
    }
}

/// The spellings of the status row, the widest first (`status_line`).
pub fn candidates(table: &Strings, status: &Status) -> Vec<String> {
    let join = table.hint_joiner;
    let key_pair = |older: &str, newer: &str| {
        format!(
            "{older} {}{join}{newer} {}",
            table.find_older, table.find_newer
        )
    };
    let retry = format!("{} {}", table.find_key_older, table.find_retry);
    let unavailable = format!("{}{join}{retry}", table.find_history_unavailable);
    let next = format!(
        "{}{join}{}",
        table.find_accept,
        key_pair(table.find_key_older, table.find_key_newer)
    );
    let exhausted = format!("{}{join}{next}", table.find_no_more);
    let plain_older = format!("{} {}", table.find_plain_older, table.find_older);
    let plain = key_pair(table.find_plain_older, table.find_plain_newer);
    let reading_exhausted = format!("{}{join}{plain}", table.find_no_more);
    let (text, compact): (&str, &str) = match status.progress {
        Progress::Idle | Progress::Found if status.reading => (&plain, &plain_older),
        Progress::Idle if !status.query_empty => (&next, table.find_accept),
        Progress::Idle => (table.find_type, table.find_type),
        Progress::Restart | Progress::Scanning => (table.find_searching, table.find_searching),
        Progress::AwaitingHistory if status.history_failed => (&unavailable, &retry),
        Progress::AwaitingHistory => (table.find_searching_history, table.find_loading),
        Progress::Found => (&next, table.find_accept),
        Progress::Exhausted if status.reading && status.has_match => {
            (&reading_exhausted, &plain_older)
        }
        Progress::Exhausted if status.has_match => (&exhausted, table.find_accept),
        Progress::Exhausted => (table.find_no_matches, table.find_no_matches),
    };
    if status.reading {
        let find = table.find_label;
        let latest = table.find_latest;
        return vec![
            format!("{find}{join}{text}{join}{latest}"),
            format!("{find}{join}{compact}{join}{latest}"),
            latest.to_owned(),
        ];
    }
    let limit = if status.truncated {
        format!("{join}{}", table.find_limited)
    } else {
        String::new()
    };
    let cancel = table.find_cancel;
    vec![
        format!(
            "{text}{join}{}{join}{cancel}{limit}",
            table.find_full_transcript
        ),
        format!("{text}{join}{cancel}{limit}"),
        format!("{compact}{join}{cancel}"),
        format!("{compact}{join}{}", table.key_esc),
        cancel.to_owned(),
    ]
}

/// The status row: the first spelling that fits the width, clipped, empty when none does.
pub fn status_row(table: &'static Strings, status: Status, style: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let line = candidates(table, &status)
            .into_iter()
            .find(|line| {
                i32::try_from(segmenter.width(line)).unwrap_or(i32::MAX) <= constraints.max_width
            })
            .unwrap_or_default();
        Text::new(line)
            .style(style.clone())
            .overflow(TextOverflow::Clip)
            .into()
    })
    .into()
}

/// The query row: the dim label and the editor after it, which takes the rest.
pub fn query_row(table: &'static Strings, label: Style, editor: AnyWidget) -> AnyWidget {
    Row::new()
        .child(Text::new(table.find_prefix).style(label))
        .child(Expanded::new(1, editor))
        .into()
}
