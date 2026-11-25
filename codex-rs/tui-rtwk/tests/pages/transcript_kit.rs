// test root: pages
//! The doors of `d-transcript-kit`: one test per door, each the smallest use a transcript feature
//! makes of it. The features that use them have their own tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use crate::kit::{Owner, Screen};

const TOP_BOTTOM: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_1.jsonl");

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn entries(lines: &[&str]) -> &'static str {
    let mut text = String::from(SESSION);
    for line in lines {
        text.push('\n');
        text.push_str(line);
    }
    Box::leak(text.into_boxed_str())
}

/// The oracle's select-mode tests drive the view at 32x6 with no bottom pane (`render` of
/// `select_mode_tests.rs`); the full chat leaves its list one row there.
#[test]
fn the_view_alone_gives_the_list_every_row_of_a_32x6_terminal() {
    let two_rows = entries(&[
        r#"{"step":"entry","kind":"agent","turn":"t","text":"top","end":false}"#,
        r#"{"step":"entry","kind":"agent","turn":"t","text":"bottom","start":false}"#,
    ]);
    let full = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(two_rows)
        .open();
    full.assert_absent("top");
    let view = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(two_rows)
        .view_only()
        .open();
    view.assert_row(1, "• top");
    view.assert_row(3, "• bottom");
    view.assert_contains("Worked for");
    // no composer, no footer: the transcript is the whole screen
    view.assert_absent("Ask Codex");
    view.assert_absent("? for shortcuts");
}

/// The oracle's `sm_entry_*` cells are one entry per turn; the port closes each turn with its
/// "Worked for" row, so two of them do not fit 6 rows: the newest is at the bottom, no bottom pane
/// under it, the older one is a scroll away.
#[test]
fn the_view_alone_scrolls_to_the_older_entry_of_the_oracles_cells() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(TOP_BOTTOM)
        .view_only()
        .open();
    s.assert_contains("• bottom");
    s.assert_absent("Ask Codex");
    s.wheel_up(0, 0, 10);
    s.assert_contains("• top");
}

/// The shapes the transcript features read: each is an `entry` step, drawn by the transcript sheet
/// as the chat's own cell (the tab and the styled spans are the entry features' to draw).
#[test]
fn every_entry_shape_has_a_script_step() {
    let text = entries(&[
        r#"{"step":"entry","kind":"user","text":"the prompt"}"#,
        r#"{"step":"entry","kind":"agent","text":"a\tb **bold**"}"#,
        r#"{"step":"entry","kind":"reasoning","text":"the reasoning"}"#,
        r#"{"step":"entry","kind":"plan","text":"the plan"}"#,
        r#"{"step":"entry","kind":"plan_update","steps":[["Write it","completed"],["Test it","inProgress"]]}"#,
        r#"{"step":"entry","kind":"exec","command":"printf visible","output":"visible\n","exit":0}"#,
        r#"{"step":"entry","kind":"exec","command":"sleep 9","running":true,"end":false}"#,
    ]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 90)
        .script(text)
        .view_only()
        .open();
    s.recap("The recap.", Some("Run it."));
    for shown in [
        "› the prompt",
        "• a b bold",
        "• the reasoning",
        "• Proposed Plan",
        "  the plan",
        "• Updated Plan",
        "  └ ✔ Write it",
        "    □ Test it",
        "$ printf visible",
        "↳ Recap: The recap.",
        "Next: Run it.",
        "$ sleep 9",
    ] {
        s.assert_contains(shown);
    }
}

/// The oracle's live cell: the answer streams (hidden from history until its turn ends), then
/// commits. The transcript shows it all along and the commit adds the turn's "Worked for" row.
#[test]
fn the_live_tail_grows_and_a_commit_ends_its_turn() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 8)
        .script(SESSION)
        .view_only()
        .open();
    s.live_tail("live needle");
    s.assert_contains("• live needle");
    s.assert_absent("Worked for");
    s.live_tail(" again needle");
    s.assert_contains("• live needle again needle");
    s.assert_absent("Worked for");
    s.commit_live_tail();
    s.assert_contains("• live needle again needle");
    s.assert_contains("Worked for");
}

/// The oracle sets `TranscriptHistoryState::Partial` by hand and joins a page by hand: here the
/// server's first page has a cursor behind it, the reader goes to the top, and the older page
/// the app asked for joins above, under a live answer that keeps streaming.
#[test]
fn an_older_page_joins_above_a_partial_history_and_the_live_tail() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history(3);
    s.assert_contains("row 02");
    s.live_tail("latest answer");
    s.assert_contains("• latest answer");
    s.join_older_page(&["older a", "older b"], false);
    assert_eq!(s.page_requests(), [None, Some("cursor-1".to_string())]);
    assert_eq!(s.pages_waiting(), 0);
    // the reader keeps the row it was on; the joined rows are above it
    s.assert_contains("row 00");
    s.assert_absent("older a");
    s.wheel_up(0, 0, 100);
    s.assert_contains("older a");
    s.live_tail(" and more");
    s.wheel_down(0, 0, 100);
    s.assert_contains("• latest answer and more");
}
