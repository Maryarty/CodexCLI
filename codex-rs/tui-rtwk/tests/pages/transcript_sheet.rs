// test root: pages
//! Tests of the transcript sheet (S16; oracle `pager_overlay/transcript.rs`
//! `TranscriptOverlay::render`, `render_hints`): the title row, the whole
//! transcript with its reasoning, the status row, the two hint rows, the
//! pager's keys over the thread's follow controller, the wheel, closing.
//!
//! The sheet is opened through the transcript directory's `sheet_basic`
//! entry: a prompt, a reasoning summary and an answer of forty rows. At
//! 60x12 the transcript has eight rows: the title row, the status row and the
//! two hint rows take the others. `sheet_owned` is the same transcript
//! without chrome (the person's decision: the owned full-screen Ctrl+T and
//! backtrack draw none).

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
fn away_from_the_last_row_the_status_row_names_the_way_back_and_the_hints_give_way() {
    let mut s = sheet();
    s.keys("home");
    let rows = s.rows();
    assert_eq!(rows[9].trim(), "end latest", "{}", s.text());
    assert_eq!(rows[10].trim(), "", "{}", s.text());
    assert_eq!(rows[11].trim(), "", "{}", s.text());
    // the key is bold, what it does is dim (`navigation_line`)
    s.assert_text_style("end", "bold");
    // back at the last row the select key and the hints return
    s.keys("end");
    let rows = s.rows();
    assert!(rows[9].starts_with("⌃space select"), "{}", s.text());
    assert_eq!(
        rows[10].trim(),
        "↑/↓ to scroll · pgup/pgdn to page · home/end to jump"
    );
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

// ---- browsing prompts (oracle `app_backtrack.rs`, `prompt_navigation.rs`)

fn prompts(width: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, 12)
        .entry("transcript", "sheet_prompts")
        .open()
}

#[test]
fn esc_starts_browsing_and_names_the_keys_in_the_status_row() {
    let mut s = prompts(100);
    s.assert_row(9, "⌃space select");
    s.keys("esc");
    s.assert_row(
        9,
        "Browsing transcript · ↑↓/jk scroll · ←→/hl prompts · ⌃t details · ↵ rewind · esc back",
    );
    // a prompt is marked: the hints add the keys that edit it
    s.assert_row(
        11,
        " q close · f3 find · esc browse prompts · → to edit next · enter to edit message",
    );
}

#[test]
fn a_second_esc_ends_browsing() {
    let mut s = prompts(100);
    s.keys("esc esc");
    s.assert_row(9, "⌃space select");
    s.assert_row(11, " q close · f3 find · esc browse prompts");
}

#[test]
fn the_browsing_status_row_takes_the_first_spelling_that_fits() {
    let spellings = [
        (
            100,
            "Browsing transcript · ↑↓/jk scroll · ←→/hl prompts · ⌃t details · ↵ rewind · esc back",
        ),
        (
            80,
            "Browsing · ↑↓/jk scroll · ←→/hl prompts · ⌃t details · ↵ rewind · esc back",
        ),
        (
            60,
            "Browsing · ↑↓/jk ←→/hl · ⌃t details · ↵ rewind · esc back",
        ),
        (45, "Browsing · ↑↓/jk ←→/hl · ↵ rewind · esc"),
        (30, "Browsing · ↵ rewind · esc back"),
        (20, "Browsing · esc back"),
        (14, "Browsing · esc"),
        (13, "Browsing"),
        (8, "Browsing"),
    ];
    for (width, row) in spellings {
        let mut s = prompts(width);
        s.keys("esc");
        s.assert_row(9, row);
    }
}

#[test]
fn enter_chooses_the_prompt_and_closes_the_sheet() {
    let mut s = prompts(100);
    // no prompt is chosen outside browsing: Enter does nothing
    s.keys("enter");
    s.assert_contains("T R A N S C R I P T");
    s.keys("esc left left left enter");
    s.assert_absent("T R A N S C R I P T");
    s.assert_contains("debug screens");
}

#[test]
fn q_closes_the_sheet_while_browsing_and_forgets_the_mode() {
    let mut s = prompts(100);
    s.keys("esc q");
    s.assert_absent("T R A N S C R I P T");
}

// ---- the marked prompt (oracle `transcript_view.rs` `set_highlight`, `transcript_tests.rs`
// `highlighting_reveals_prompt_body_when_only_padding_is_visible`)

/// All three prompts in view at once.
fn tall() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(100, 30)
        .entry("transcript", "sheet_prompts")
        .open()
}

#[test]
fn browsing_reverses_the_body_of_the_marked_prompt_only() {
    let mut s = tall();
    s.assert_text_style("third prompt", "!reversed");
    s.keys("esc");
    // the newest prompt is marked first
    s.assert_text_style("third prompt", "reversed");
    s.assert_text_style("second prompt", "!reversed");
    s.assert_text_style("first prompt", "!reversed");
    s.keys("left");
    s.assert_text_style("second prompt", "reversed");
    s.assert_text_style("third prompt", "!reversed");
}

#[test]
fn the_mark_goes_when_browsing_ends() {
    let mut s = tall();
    s.keys("esc");
    s.assert_text_style("third prompt", "reversed");
    s.keys("esc");
    s.assert_text_style("third prompt", "!reversed");
}

#[test]
fn a_marked_prompt_out_of_view_is_scrolled_into_view() {
    let mut s = sheet();
    // the sheet starts at the end of a long answer: the prompt is far above
    s.assert_absent("hello");
    s.keys("esc");
    s.assert_contains("hello");
    s.assert_text_style("hello", "reversed");
    // the least that shows it: the reader does not jump to the top of the list
    s.keys("esc");
    s.keys("end");
    s.assert_absent("hello");
    s.keys("esc");
    s.assert_contains("hello");
}

// ---- the chrome (the person's decision on the pager chrome, as the oracle has it:
// the overlay has title, status row and hints but no percent rule; the owned
// full-screen Ctrl+T draws none)

fn owned() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(60, 12)
        .entry("transcript", "sheet_owned")
        .open()
}

#[test]
fn the_overlay_draws_no_percent_rule() {
    let s = sheet();
    s.assert_absent("%");
    s.assert_absent("───");
}

#[test]
fn the_owned_sheet_draws_no_title_status_or_hint_rows() {
    let s = owned();
    s.assert_absent("T R A N S C R I P T");
    s.assert_absent("select");
    s.assert_absent("to scroll");
    s.assert_absent("q close");
    // the transcript has every row: the last one is the answer's
    let rows = s.rows();
    assert!(!rows[0].starts_with('/'), "{}", s.text());
    assert!(!rows[11].trim().is_empty(), "{}", s.text());
}

#[test]
fn the_owned_sheet_reads_and_browses_with_the_same_keys() {
    let mut s = owned();
    s.keys("home");
    s.assert_contains("hello");
    s.keys("esc");
    s.assert_text_style("hello", "reversed");
    // the browsing keys are the chat footer's to name, not the sheet's
    s.assert_absent("Browsing");
    s.keys("esc");
    s.assert_text_style("hello", "!reversed");
    s.keys("q");
    s.assert_absent("Detailed reasoning goes here.");
    s.assert_contains("debug screens");
}
