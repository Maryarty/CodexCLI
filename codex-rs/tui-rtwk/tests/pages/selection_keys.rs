// test root: pages
//! Mouse selection followed by ordinary transcript keyboard input.

use crate::kit::{Owner, Screen};

#[test]
fn plain_and_shifted_arrows_move_an_existing_mouse_range_and_copy_its_new_end() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 7)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/sl_modified.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("beta");
    s.assert_text_style("beta", "reversed");

    s.keys("shift+left");
    s.assert_text_style("bet", "reversed");
    s.key_repeat("left");
    s.assert_text_style("be", "reversed");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["be"]);
}

#[test]
fn typing_clears_a_mouse_selection_and_the_composer_receives_the_key() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 12)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/sl_modified.jsonl"
        ))
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("beta");
    s.assert_text_style("beta", "reversed");
    s.keys("z");
    s.assert_text_style("beta", "!reversed");
    s.assert_contains("› z");
    s.assert_copies_asked(&[]);
}

#[test]
fn arrow_extension_keeps_graphemes_and_the_column_across_a_short_line() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 12)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/sl_keyboard.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    let row = s.row_of("1234567890") as u16 - 2;
    s.mouse_down(0, row);
    s.mouse_drag(3, row);
    s.mouse_up(3, row);
    s.keys("right right");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["A👨‍👩‍👧‍👦界e\u{301}"]);
    s.keys("down down");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["A👨‍👩‍👧‍👦界e\u{301}", "A👨‍👩‍👧‍👦界e\u{301}Z\nx\n123456"]);
    s.keys("up up");
    s.keys("ctrl+c");
    s.assert_copies_asked(&[
        "A👨‍👩‍👧‍👦界e\u{301}",
        "A👨‍👩‍👧‍👦界e\u{301}Z\nx\n123456",
        "A👨‍👩‍👧‍👦界e\u{301}",
    ]);
}

#[test]
fn clipboard_keys_only_clear_the_readers_selection_on_confirmation() {
    for key in ["enter", "ctrl+c", "super+c", "ctrl+shift+c", "ctrl+insert"] {
        for searching in [false, true] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(32, 8)
                .script(include_str!(
                    "../oracle/fixtures/transcript_view_4/sl_command_c.jsonl"
                ))
                .view_only()
                .open();
            s.selection_copy_policy(false, true);
            if searching {
                s.keys("/");
            }
            s.double_click_text("selected");
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected"]);
            s.answer_copy(Err("clipboard unavailable"));
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected", "selected"]);
            s.answer_copy_unconfirmed();
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected", "selected", "selected"]);
            s.answer_copy(Ok(()));
            s.assert_text_style("selected", "!reversed");
        }
    }
}
