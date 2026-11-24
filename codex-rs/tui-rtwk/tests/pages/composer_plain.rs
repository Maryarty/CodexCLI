// test root: pages
//! The plain-text composer (`Composer::plain`, the answers inside question
//! sheets, debug entry `composer/plain`): what it does not build (popups,
//! shell mode, the shortcut overlay), the large paste and the over-long
//! message's flash, and the mouse on its text (ADR-011: it takes the mouse
//! as the chat composer's does; a text area has no wheel of its own in rtwk, and the
//! plain composer has no popup rows to move it over). Oracle: `ChatComposerConfig::plain_text`,
//! `chat_composer/embedded_input_tests.rs`.

use crate::kit::{Owner, Screen};

/// The plain-text composer with its footer, `w` x `h` cells.
fn plain(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .entry("composer", "plain")
        .open()
}

/// The prompt's row (`›`), the last one.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›') || r.starts_with("! "))
        .unwrap_or(usize::MAX)
}

fn cell(row: usize) -> u16 {
    u16::try_from(row).unwrap_or(u16::MAX)
}

#[test]
fn a_slash_opens_no_command_popup() {
    let mut s = plain(80, 24);
    s.type_text("/di");
    s.assert_row(prompt(&s), "› /di");
    s.assert_absent("/diff");
    // the chat's composer lists the commands for the same text
    let mut chat = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    chat.wait_for("› Ask Codex to do anything");
    chat.type_text("/di");
    chat.assert_contains("/diff");
}

#[test]
fn a_leading_bang_is_text_and_takes_no_shell_mode() {
    let mut s = plain(80, 24);
    s.type_text("!ls -la");
    s.assert_row(prompt(&s), "› !ls -la");
    s.assert_absent("Shell mode");
    // Delete at the start takes the `!`, a character, not a mode
    s.keys("home");
    s.keys("delete");
    s.assert_row(prompt(&s), "› ls -la");
}

#[test]
fn a_question_mark_is_text() {
    let mut s = plain(80, 24);
    s.type_text("?");
    s.assert_row(prompt(&s), "› ?");
    s.assert_absent("for commands");
}

#[test]
fn an_at_and_a_dollar_are_text_and_ask_for_no_file_search() {
    let mut s = plain(80, 24);
    s.type_text("@src/ma $sk");
    s.assert_row(prompt(&s), "› @src/ma $sk");
    // nothing but the prompt and the footer: no mention popup rows
    let rows = s.rows();
    let below = rows.len() - prompt(&s) - 1;
    assert!(
        rows[prompt(&s) + 1..]
            .iter()
            .take(below.saturating_sub(1))
            .all(|r| r.trim().is_empty()),
        "{rows:?}"
    );
}

#[test]
fn a_paste_that_starts_with_a_bang_is_text() {
    let mut s = plain(80, 24);
    s.paste("!echo hi");
    s.assert_row(prompt(&s), "› !echo hi");
}

#[test]
fn a_large_paste_is_its_placeholder() {
    let mut s = plain(80, 24);
    s.paste(&"x".repeat(1_001));
    s.assert_row(prompt(&s), "› [Pasted Content 1001 chars]");
}

#[test]
fn the_over_long_flash_stays_five_seconds_and_the_draft_with_it() {
    let mut s = plain(80, 24);
    s.paste(&"x".repeat(1_048_577));
    s.keys("enter");
    s.assert_contains("Message too long; limit 1048576 characters");
    s.wait(4_000);
    s.assert_contains("Message too long; limit 1048576 characters");
    s.wait(2_000);
    s.assert_absent("Message too long");
    s.assert_row(prompt(&s), "› [Pasted Content 1048577 chars]");
}

#[test]
fn a_click_puts_the_caret_in_the_draft() {
    let mut s = plain(80, 24);
    s.type_text("hello world");
    let row = cell(prompt(&s));
    // the prompt takes two cells: x 7 is between `hello` and the space
    s.click(7, row);
    s.type_text("X");
    s.assert_row(prompt(&s), "› helloX world");
}

#[test]
fn a_drag_selects_and_typing_replaces_the_selection() {
    let mut s = plain(80, 24);
    s.type_text("hello world");
    let row = cell(prompt(&s));
    s.drag(8, row, 13, row);
    s.type_text("X");
    s.assert_row(prompt(&s), "› hello X");
}

#[test]
fn a_double_click_selects_the_word() {
    let mut s = plain(80, 24);
    s.type_text("hello world");
    let row = cell(prompt(&s));
    s.double_click(10, row);
    s.assert_style(8, usize::from(row), "reversed");
    s.assert_style(7, usize::from(row), "!reversed");
    s.type_text("X");
    s.assert_row(prompt(&s), "› hello X");
}
