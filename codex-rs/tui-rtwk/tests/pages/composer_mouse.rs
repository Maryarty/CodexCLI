// test root: pages
//! Mouse tests of the composer (ADR-011): a click puts the caret where it
//! lands in the draft, a drag selects and the next typed text replaces the
//! selection, a click on a popup row picks it. The oracle's composer has no
//! mouse; these hold the port's ADR-011 contract on the live chat screen
//! and the oracle's skill fixture, with the kit's mouse events. With the
//! mouse unused every screen equals the oracle's: the oracle shards say so.

use crate::kit;

use kit::{Owner, Screen};

/// A live chat screen, the composer empty.
fn chat() -> Screen {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.wait_for("› Ask Codex to do anything");
    s
}

/// The composer's prompt row (`›`), the last one.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›'))
        .unwrap_or(usize::MAX)
}

fn cell(row: usize) -> u16 {
    u16::try_from(row).unwrap_or(u16::MAX)
}

#[test]
fn a_click_in_the_draft_puts_the_caret_there() {
    let mut s = chat();
    s.type_text("hello world");
    let row = cell(prompt(&s));
    // the prompt takes two cells: x 7 is between `hello` and the space
    s.click(7, row);
    s.type_text("X");
    s.assert_row(prompt(&s), "› helloX world");
}

#[test]
fn a_drag_selects_and_typing_replaces_the_selection() {
    let mut s = chat();
    s.type_text("hello world");
    let row = cell(prompt(&s));
    s.drag(8, row, 13, row);
    s.type_text("X");
    s.assert_row(prompt(&s), "› hello X");
}

#[test]
fn a_double_click_selects_the_word_and_typing_replaces_it() {
    let mut s = chat();
    s.type_text("hello world");
    let row = cell(prompt(&s));
    // `world` starts at x 8 (the prompt takes two cells, `hello ` six)
    s.double_click(10, row);
    s.assert_style(8, usize::from(row), "reversed");
    s.assert_style(12, usize::from(row), "reversed");
    s.assert_style(7, usize::from(row), "!reversed");
    s.type_text("X");
    s.assert_row(prompt(&s), "› hello X");
}

#[test]
fn the_drag_is_drawn_reversed_as_the_oracles_textarea_does() {
    let mut s = chat();
    s.type_text("hello world");
    let row = usize::from(cell(prompt(&s)));
    s.drag(2, cell(row), 7, cell(row));
    s.assert_style(2, row, "reversed");
    s.assert_style(6, row, "reversed");
    s.assert_style(7, row, "!reversed");
}

// ADR-011: a click on a row of the composer's popup picks it as Enter does
#[test]
#[ignore = "pending(mount): MentionPopup is placed by session_page.rs (a Need of that file); the popup's rows are not on the page until the line is in"]
fn a_click_on_a_popup_row_picks_it() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "../oracle/fixtures/composer_popups/skills_rustdoc.jsonl"
        ))
        .open();
    s.type_text("$ru");
    s.click_text("rustdoc");
    s.assert_row(prompt(&s), "› $rustdoc");
}
