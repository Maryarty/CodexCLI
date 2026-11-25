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
        r#"{"step":"entry","kind":"recap","summary":"The recap.","next":"Run it."}"#,
        r#"{"step":"entry","kind":"exec","command":"sleep 9","running":true,"end":false}"#,
    ]);
    let s = Screen::build(Owner::ChatLive)
        .size(60, 90)
        .script(text)
        .view_only()
        .open();
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
