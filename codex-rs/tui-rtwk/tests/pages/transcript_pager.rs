// test root: pages
//! Tests of the static pager (S16; oracle `pager_overlay.rs` `PagerView` and
//! `StaticOverlay`, `keymap.rs` pager defaults): the text scrolled by the
//! pager's keys and the wheel, the percent in the rule, the ANSI colours of a
//! program's text, the action's key, closing.
//!
//! The pager is opened through the debug entries of the transcript directory
//! (`pager_long`: forty numbered lines; 40x10 gives the text five rows: the
//! header, the rule and the three hint rows take the others).

use crate::kit::{Owner, Screen};

fn pager(entry: &'static str) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(40, 10)
        .entry("transcript", entry)
        .open()
}

/// The first row of the text (under the header).
fn top(s: &Screen) -> String {
    s.rows()[1].trim().to_string()
}

/// The percent in the rule (`─── 14% ─`).
fn percent(s: &Screen) -> String {
    let rule = s.rows()[6].clone();
    rule.trim_matches(|c: char| c == '─' || c == ' ')
        .to_string()
}

#[test]
fn the_text_starts_at_the_first_column_over_the_filler() {
    let s = pager("pager_basic");
    s.assert_row(1, "one");
    s.assert_row(2, "two");
    s.assert_row(3, "three");
    s.assert_row(4, "~");
    s.assert_row(5, "~");
}

#[test]
fn down_and_up_scroll_one_row() {
    let mut s = pager("pager_long");
    assert_eq!(top(&s), "line 00");
    s.keys("down");
    assert_eq!(top(&s), "line 01");
    s.keys("j j");
    assert_eq!(top(&s), "line 03");
    s.keys("up");
    assert_eq!(top(&s), "line 02");
    s.keys("k k k");
    assert_eq!(top(&s), "line 00", "the top stops the scroll");
}

#[test]
fn page_keys_scroll_a_page() {
    let mut s = pager("pager_long");
    s.keys("pagedown");
    assert_eq!(top(&s), "line 05");
    s.keys("space");
    assert_eq!(top(&s), "line 10");
    s.keys("ctrl+f");
    assert_eq!(top(&s), "line 15");
    s.keys("pageup");
    assert_eq!(top(&s), "line 10");
    s.keys("ctrl+b");
    assert_eq!(top(&s), "line 05");
}

#[test]
fn half_page_keys_scroll_half_a_page_rounded_up() {
    let mut s = pager("pager_long");
    s.keys("ctrl+d");
    // (5 + 1) / 2
    assert_eq!(top(&s), "line 03");
    s.keys("ctrl+d ctrl+u");
    assert_eq!(top(&s), "line 03");
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let mut s = pager("pager_long");
    s.keys("end");
    assert_eq!(top(&s), "line 35", "the last page ends at the last line");
    s.assert_row(5, "line 39");
    s.keys("down");
    assert_eq!(top(&s), "line 35", "the end stops the scroll");
    s.keys("home");
    assert_eq!(top(&s), "line 00");
}

#[test]
fn the_rule_names_the_percent_scrolled() {
    let mut s = pager("pager_long");
    assert_eq!(percent(&s), "0%");
    s.keys("pagedown");
    // 5 of the 35 rows that can scroll
    assert_eq!(percent(&s), "14%");
    s.keys("end");
    assert_eq!(percent(&s), "100%");
    // a text that fits is read whole
    assert_eq!(percent(&pager("pager_basic")), "100%");
}

#[test]
fn the_wheel_scrolls_the_text() {
    let mut s = pager("pager_long");
    s.wheel_down(5, 3, 1);
    assert_ne!(top(&s), "line 00", "{}", s.text());
    s.wheel_up(5, 3, 5);
    assert_eq!(top(&s), "line 00");
}

#[test]
fn q_and_ctrl_c_close_the_pager() {
    for key in ["q", "ctrl+c"] {
        let mut s = pager("pager_basic");
        s.keys(key);
        s.assert_absent("S T A T I C");
        s.assert_contains("debug screens");
    }
}

#[test]
fn the_action_key_is_listed_and_bound_only_for_a_pager_with_an_action() {
    let mut plain = pager("pager_basic");
    plain.assert_absent("Edit configuration");
    plain.keys("enter");
    plain.assert_contains("S T A T I C");

    let mut s = pager("pager_action");
    s.assert_contains("q close · enter Edit configuration");
    s.keys("enter");
    s.assert_absent("P R O V I D E R");
}

#[test]
fn ansi_colours_of_a_program_paint_the_text() {
    let s = pager("pager_colors");
    s.assert_row(1, "-removed");
    s.assert_row(2, "+added");
    s.assert_row(3, "");
    s.assert_row(4, "plain");
    s.assert_text_style("-removed", "fg=Red");
    s.assert_text_style("+added", "fg=Green");
    s.assert_text_style("plain", "plain");
}

#[test]
fn the_hints_clip_where_the_row_ends() {
    let s = pager("pager_basic");
    s.assert_row(7, " ↑/↓ to scroll · pgup/pgdn to page · hom");
    s.assert_row(8, " q close");
}
