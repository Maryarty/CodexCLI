//! Tests of the transcript sheet (S16; oracle `pager_overlay/transcript.rs`
//! `TranscriptOverlay::render`, `render_hints`): the title row, the whole
//! transcript with its reasoning, the status row, the two hint rows, the
//! pager's keys over the thread's follow controller, the wheel, closing.
//!
//! The sheet is opened through the transcript directory's `sheet_basic`
//! entry: a prompt, a reasoning summary and an answer of forty rows. At
//! 60x12 the transcript has eight rows: the title row, the status row and the
//! two hint rows take the others.

use crate::kit::{Owner, Screen};

fn sheet() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(60, 12)
        .entry("transcript", "sheet_basic")
        .open()
}

#[test]
fn the_title_row_is_slashes_with_the_title_over_its_start() {
    let s = sheet();
    s.assert_row(
        0,
        "/ T R A N S C R I P T / / / / / / / / / / / / / / / / / / /",
    );
}

#[test]
fn the_sheet_shows_the_reasoning_the_chat_hides() {
    let mut s = sheet();
    s.keys("home");
    s.assert_contains("hello");
    s.assert_contains("Detailed reasoning goes here.");
}

#[test]
fn the_status_row_names_the_select_key_and_the_hints_follow() {
    let s = sheet();
    let rows = s.rows();
    assert_eq!(
        rows[10].trim(),
        "↑/↓ to scroll · pgup/pgdn to page · home/end to jump"
    );
    assert_eq!(rows[11].trim(), "q close · f3 find · esc browse prompts");
    assert!(rows[9].starts_with("⌃space select"), "{}", s.text());
}

#[test]
fn up_down_and_the_page_keys_scroll_the_transcript() {
    let mut s = sheet();
    // the chat's list starts at its last row: the sheet shares its reader
    s.assert_contains("row 39");
    s.keys("home");
    s.assert_contains("hello");
    s.assert_absent("row 39");
    s.keys("down down down down");
    s.assert_absent("hello");
    s.keys("end");
    s.assert_contains("row 39");
    s.keys("pageup");
    s.assert_absent("row 39");
    s.keys("pagedown");
    s.assert_contains("row 39");
}

#[test]
fn the_wheel_scrolls_the_transcript() {
    let mut s = sheet();
    s.wheel_up(5, 3, 3);
    s.assert_absent("row 39");
    s.wheel_down(5, 3, 30);
    s.assert_contains("row 39");
}

#[test]
fn q_and_ctrl_c_close_the_sheet() {
    for key in ["q", "ctrl+c"] {
        let mut s = sheet();
        s.keys(key);
        s.assert_absent("T R A N S C R I P T");
        s.assert_contains("debug screens");
    }
}
