// test root: pages
//! Mouse selection followed by ordinary transcript keyboard input.

use crate::kit::{Owner, Screen};

#[test]
fn an_immediate_copy_answer_preserves_the_same_text_selected_in_another_row() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(42, 12)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["same first", "same second"]);
    s.selection_copy_policy(false, false);
    let first = u16::try_from(s.row_of("same first")).expect("first row");
    let second = u16::try_from(s.row_of("same second")).expect("second row");
    s.double_click(3, first);
    s.assert_style(3, usize::from(first), "reversed");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["same"]);
    s.double_click(3, second);
    s.assert_style(3, usize::from(second), "reversed");
    s.answer_copy(Ok(()));
    s.assert_style(3, usize::from(second), "reversed");
}

#[test]
fn returning_the_keyboard_range_to_its_original_end_keeps_it_after_an_old_copy() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 7)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/sl_modified.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("beta");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["beta"]);
    s.answer_copy_pending(1);
    s.keys("shift+left");
    s.assert_text_style("bet", "reversed");
    s.keys("right");
    s.assert_text_style("beta", "reversed");
    s.answer_copy(Ok(()));
    s.assert_text_style("beta", "reversed");
}

#[test]
fn wheel_navigation_away_and_back_invalidates_clear_and_follow() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 12)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_5/rows40.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.keys("ctrl+home");
    let row = u16::try_from(s.row_of("row 05")).expect("selected row");
    s.drag(2, row, 7, row);
    s.keys("enter");
    s.assert_copies_asked(&["row 05"]);
    s.answer_copy_pending(1);
    let before = s.shot();
    s.wheel_down(3, row, 1);
    s.assert_rows_differ(&before);
    s.wheel_up(3, row, 1);
    s.assert_same_rows(&before);
    s.assert_text_style("row 05", "reversed");
    s.answer_copy(Ok(()));
    s.assert_same_rows(&before);
    s.assert_text_style("row 05", "reversed");
}

#[test]
fn copy_and_follow_takes_the_current_reader_to_the_tail_on_confirmation() {
    for pending in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 12)
            .script(include_str!(
                "../oracle/fixtures/transcript_view_5/rows40.jsonl"
            ))
            .view_only()
            .open();
        s.selection_copy_policy(false, false);
        s.keys("ctrl+home");
        let row = u16::try_from(s.row_of("row 05")).expect("selected row");
        s.drag(2, row, 7, row);
        s.keys("enter");
        s.assert_copies_asked(&["row 05"]);
        if pending {
            s.answer_copy_pending(1);
        }
        s.assert_text_style("row 05", "reversed");
        s.assert_absent("row 39");
        s.answer_copy(Ok(()));
        s.assert_contains("row 39");
        s.assert_absent("row 05");
    }
}

#[test]
fn hiding_a_reader_leaves_feedback_but_retires_its_selection_effects() {
    const APPROVAL: &str = r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","itemId":"call-1","approvalId":"call-1","environmentId":null,"command":"echo hello","cwd":"/tmp","reason":"need confirmation","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;
    for confirm_while_hidden in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(72, 20)
            .script(include_str!(
                "../oracle/fixtures/transcript_view_4/sl_pending.jsonl"
            ))
            .owned_transcript()
            .open();
        s.selection_copy_policy(false, false);
        s.double_click_text("selected");
        s.keys("ctrl+c");
        s.assert_copies_asked(&["selected"]);
        s.answer_copy_pending(1);
        s.feed(APPROVAL);
        s.assert_contains("need confirmation");
        if !confirm_while_hidden {
            s.keys("escape");
            s.assert_absent("need confirmation");
            s.assert_text_style("selected", "reversed");
        }
        s.answer_copy(Ok(()));
        if confirm_while_hidden {
            s.keys("escape");
            s.assert_absent("need confirmation");
        }
        s.assert_contains("Copied 8 chars to host clipboard");
        s.assert_text_style("selected", "reversed");
    }
}

#[test]
fn toggling_the_detailed_transcript_twice_retires_its_earlier_copy() {
    const SCENE: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"reasoning","text":"quiet thought"}
{"step":"entry","kind":"agent","text":"the answer"}"#;
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 20)
        .script(SCENE)
        .owned_transcript()
        .open();
    s.selection_copy_policy(false, false);
    s.assert_absent("quiet thought");
    s.double_click_text("answer");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["answer"]);
    s.answer_copy_pending(1);
    s.keys("ctrl+t");
    s.assert_contains("quiet thought");
    s.keys("ctrl+t");
    s.assert_absent("quiet thought");
    s.assert_text_style("answer", "!reversed");
    s.wait(350);
    s.double_click_text("answer");
    s.assert_text_style("answer", "reversed");
    s.answer_copy(Ok(()));
    s.assert_contains("Copied 6 chars to host clipboard");
    s.assert_text_style("answer", "reversed");
}

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
fn an_older_history_page_keeps_the_keyboard_selection_on_its_original_row() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(42, 16)
        .replay(Vec::new())
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.partial_history_of(&["preceding message", "canonical text"]);
    s.double_click_text("canonical");
    s.keys("shift+left ctrl+c");
    s.assert_text_style("canonica", "reversed");
    s.assert_copies_asked(&["canonica"]);

    s.join_older_page(&["historical message"], false);
    s.assert_contains("historical message");
    s.assert_text_style("canonica", "reversed");
    s.assert_text_style("preceding", "!reversed");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["canonica", "canonica"]);
    s.keys("right ctrl+c");
    s.assert_copies_asked(&["canonica", "canonica", "canonical"]);
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
