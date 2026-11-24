// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The status row of Find (oracle `transcript_view/search_footer.rs`
//! `Search::status_line`): the first spelling that fits the width, for the
//! states and widths the oracle's tests (`search_tests.rs`) pin.

use rtwk::style::Style;
use rtwk_test::{Options, Tester};

use codex_tui_rtwk::pages::session::transcript::find_footer::{Progress, Status, status_row};
use codex_tui_rtwk::pages::session::transcript::strings::EN;

fn row(width: u16, status: Status) -> String {
    let mut t = Tester::with(Options::new().size(width, 1));
    t.pump_widget(status_row(&EN, status, Style::new()));
    t.screen().trim_end().to_owned()
}

fn at(progress: Progress) -> Status {
    Status {
        progress,
        ..Status::idle()
    }
}

#[test]
fn an_empty_finder_asks_for_a_query() {
    assert_eq!(
        row(80, Status::idle()),
        "Type to find · full transcript · esc cancel"
    );
}

#[test]
fn a_scan_says_so_and_the_history_wait_names_the_history() {
    assert_eq!(
        row(80, at(Progress::Scanning)),
        "Searching… · full transcript · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::Restart)),
        "Searching… · full transcript · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::AwaitingHistory)),
        "Searching earlier history… · full transcript · esc cancel"
    );
}

#[test]
fn a_match_offers_accept_and_the_two_walking_keys() {
    let found = Status {
        has_match: true,
        query_empty: false,
        ..at(Progress::Found)
    };
    assert_eq!(
        row(80, found),
        "enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel"
    );
    // too narrow for the whole: the compact spellings, then the bare key
    assert_eq!(row(40, found), "enter accept · esc cancel");
    assert_eq!(row(3, found), "");
}

#[test]
fn the_end_of_the_matches_drops_the_full_transcript_hint_when_it_does_not_fit() {
    let exhausted = Status {
        has_match: true,
        query_empty: false,
        ..at(Progress::Exhausted)
    };
    assert_eq!(
        row(80, exhausted),
        "No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::Exhausted)),
        "No matches · full transcript · esc cancel"
    );
}

#[test]
fn a_failed_history_offers_a_retry() {
    let failed = Status {
        history_failed: true,
        ..at(Progress::AwaitingHistory)
    };
    assert_eq!(
        row(80, failed),
        "History unavailable · ⌃p retry · full transcript · esc cancel"
    );
    assert_eq!(row(30, failed), "⌃p retry · esc cancel");
}

#[test]
fn a_cut_query_says_so_while_it_fits() {
    let cut = Status {
        truncated: true,
        ..at(Progress::Scanning)
    };
    assert_eq!(
        row(100, cut),
        "Searching… · full transcript · esc cancel · query limited to 4 KiB"
    );
}

#[test]
fn reading_a_match_is_find_dot_the_walking_keys_and_the_way_back() {
    let reading = Status {
        reading: true,
        has_match: true,
        ..at(Progress::Found)
    };
    assert_eq!(
        row(80, reading),
        "Find · ctrl+p older · ctrl+n newer · esc latest"
    );
    assert_eq!(row(34, reading), "Find · ctrl+p older · esc latest");
    assert_eq!(row(30, reading), "esc latest");
    assert_eq!(row(14, reading), "esc latest");
}
