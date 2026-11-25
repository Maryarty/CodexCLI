//! Shard `textarea` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// Reasons shared by several tests: what the port's chat composer does not have yet.
const WRAP: &str = "G65: rtwk TextArea wraps and places the caret differently from the oracle's textarea at full rows (a caret after a full row, wide and half-width graphemes), on hanging spaces and tabs, and does not clear its rect's cells or keep its scroll as the oracle does";
const LINK: &str = "the port's composer draws no OSC 8 links for URLs (rtwk-editing has no hyperlink annotation) and keeps no link cache to reuse";
const MOUSE: &str = "the bare text area takes the mouse (a drag selects, a tap places the caret), but these rows are the oracle's wrap of the rect (G65), and its rect offsets (a 1-row rect at y 1, a resize mid-test) have no kit seed yet";

const URL_PR: &str = "https://github.com/openai/codex/pull/20252";
const URL_LONG: &str = "https://example.test/alpha/beta/gamma/delta/epsilon";
/// The oracle's maximum draft length (codex-protocol user_input.rs:11).
const MAX_USER_INPUT_TEXT_CHARS: usize = 1 << 20;

/// A draft pasted into the composer's text area alone (the oracle's bare
/// `TextArea` at the test's rect), `width` by `height` cells.
fn draft(width: u16, height: u16, text: &str) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .entry("composer", "textarea")
        .open();
    s.paste(text);
    s
}

/// As `draft`, with the user's bindings (`(action, key)` pairs of the keymap config).
fn draft_keyed(
    width: u16,
    height: u16,
    text: &str,
    bindings: &'static [(&'static str, &'static str)],
) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .entry("composer", "textarea")
        .keymap(bindings)
        .open();
    s.paste(text);
    s
}

/// The oracle's `vim_composer`: the draft pasted, Vim on (Normal mode), the caret on its first cell.
/// The kit has no Vim seed: `global.toggle_vim_mode` is bound to F9 (no default key).
fn vim_draft(width: u16, height: u16, text: &str) -> Screen {
    let mut s = draft_keyed(width, height, text, &[("global.toggle_vim_mode", "f9")]);
    s.keys("f9 g g");
    s
}

/// Moves the caret from byte `from` to byte `to` of an ASCII draft, one key per byte.
fn step(s: &mut Screen, from: usize, to: usize) {
    if to < from {
        s.keys(&vec!["left"; from - to].join(" "));
    } else if to > from {
        s.keys(&vec!["right"; to - from].join(" "));
    }
}

// ---- oracle: bottom_pane/textarea.rs (33 screen-level tests here; 71 state tests left to chat-live)
// ORACLE bottom_pane/textarea.rs:2544 delete_backward_removes_thai_marks_one_at_a_time kind=snap owner=chat-live snap=textarea_thai_backspace_preserves_remaining_marks
#[test]
fn delete_backward_removes_thai_marks_one_at_a_time() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: an 8x1 rect, "ที่" with one backspace shows "ที" (the snapshot); two more leave "ท", then "".
    let mut s = draft(8, 1, "ที่");
    s.keys("backspace");
    s.assert_snap("textarea_thai_backspace_preserves_remaining_marks");
    s.keys("backspace");
    s.assert_has_row("ท");
    s.keys("backspace");
    s.assert_absent("ท");
}

// ORACLE bottom_pane/textarea.rs:3077 vim_e_advances_from_each_word_end kind=snap owner=chat-live snap=vim_e_advances_from_each_word_end
#[test]
fn vim_e_advances_from_each_word_end() {
    // "alpha beta gamma" with the caret after "alph" (byte 4) in Normal mode, three `e` presses,
    // one state each: the oracle's snapshot joins the three states, each with a caret row; the
    // kit takes the caret from the screen's cursor
    let mut s = vim_draft(80, 24, "alpha beta gamma");
    step(&mut s, 0, 4);
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("alpha beta gamma"))
        .expect("the draft's row");
    // `e` from the last cell of a word goes to the end of the next one and stays at the end
    for caret in [9, 15, 15] {
        s.keys("e");
        s.assert_row(row, "alpha beta gamma");
        s.assert_cursor(caret, row);
    }
}

// ORACLE bottom_pane/textarea.rs:3149 vim_linewise_yank_pastes_below_current_line kind=ui owner=chat-live
#[test]
fn vim_linewise_yank_pastes_below_current_line() {
    // oracle: "abc\n123\nxyz" with the caret on byte 1 (line 1); `yy` then `p` pastes "abc" below line 1.
    let mut s = vim_draft(80, 24, "abc\n123\nxyz");
    s.keys("k k 0 l y y p");
    s.assert_in_order(&["abc", "abc", "123", "xyz"]);
}

// ORACLE bottom_pane/textarea.rs:3232 kill_current_line_removes_current_line_linewise kind=ui owner=chat-live
#[test]
fn kill_current_line_removes_current_line_linewise() {
    // oracle: "abc\ndef\nghi" with the caret on byte 5 (line 2): kill_current_line removes "def" and its newline.
    // The method has no default key: `editor.kill_whole_line` is bound to ctrl+u here, as the oracle's keymap test does.
    let mut s = draft_keyed(
        80,
        24,
        "abc\ndef\nghi",
        &[("editor.kill_whole_line", "ctrl+u")],
    );
    step(&mut s, 11, 5);
    s.keys("ctrl+u");
    s.assert_in_order(&["abc", "ghi"]);
    s.assert_absent("def");
}

// ORACLE bottom_pane/textarea.rs:3245 kill_current_line_keeps_previous_newline_for_final_line kind=ui owner=chat-live
#[test]
fn kill_current_line_keeps_previous_newline_for_final_line() {
    // oracle: "abc\ndef" with the caret on byte 5: kill_current_line leaves "abc" and its newline.
    let mut s = draft_keyed(80, 24, "abc\ndef", &[("editor.kill_whole_line", "ctrl+u")]);
    step(&mut s, 7, 5);
    s.keys("ctrl+u");
    s.assert_has_row("abc");
    s.assert_absent("def");
}

// ORACLE bottom_pane/textarea.rs:3258 kill_whole_line_keymap_dispatch_uses_linewise_kill kind=ui owner=chat-live
#[test]
fn kill_whole_line_keymap_dispatch_uses_linewise_kill() {
    // oracle: "abc\ndef\nghi" with the caret on byte 5 and ctrl+u bound to kill_whole_line: the line "def" goes.
    let mut s = draft_keyed(
        80,
        24,
        "abc\ndef\nghi",
        &[("editor.kill_whole_line", "ctrl+u")],
    );
    step(&mut s, 11, 5);
    s.keys("ctrl+u");
    s.assert_in_order(&["abc", "ghi"]);
    s.assert_absent("def");
}

// ORACLE bottom_pane/textarea.rs:3840 wrapping_and_cursor_positions kind=ui owner=chat-live
#[test]
fn wrapping_and_cursor_positions() {
    // oracle: "hello world here" in a 6-wide rect wraps to rows "hello", "world", "here"; the caret on "world" is on row 1.
    let mut s = draft(6, 10, "hello world here");
    s.assert_in_order(&["hello", "world", "here"]);
    step(&mut s, 16, 6);
    s.assert_cursor(0, 1);
}

// ORACLE bottom_pane/textarea.rs:3868 cursor_wraps_after_filling_the_final_line kind=ui owner=chat-live
#[test]
fn cursor_wraps_after_filling_the_final_line() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: each text fills the first row of a 4x2 rect at (2, 3); the caret after it is at (2, 4).
    for text in ["abad", "界界", "abｶﾞ"] {
        let s = draft(4, 2, text);
        // the bare area sits at (0, 0): the oracle's (2, 4) less its rect's (2, 3)
        s.assert_cursor(2 - 2, 4 - 3);
    }
}

// ORACLE bottom_pane/textarea.rs:3880 full_non_final_lines_reserve_visible_cursor_rows kind=ui owner=chat-live
#[test]
fn full_non_final_lines_reserve_visible_cursor_rows() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: each text with "\nef" in a 4x3 rect: the caret on the newline is at (0, 1), after it at (0, 2).
    for text in ["abad\nef", "界界\nef", "abｶﾞ\nef"] {
        let mut s = draft(4, 3, text);
        step(&mut s, text.len(), text.len() - 3);
        s.assert_cursor(0, 1);
        step(&mut s, text.len() - 3, text.len() - 2);
        s.assert_cursor(0, 2);
    }
}

// ORACLE bottom_pane/textarea.rs:3897 full_non_final_line_cursor_snapshot kind=snap owner=chat-live snap=textarea_full_non_final_line_has_a_visible_cursor_row
#[test]
fn full_non_final_line_cursor_snapshot() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad\nef" with the caret on byte 4 (the newline) in a 4x3 rect: the snapshot shows the cursor row.
    let mut s = draft(4, 3, "abad\nef");
    step(&mut s, 7, 4);
    s.assert_snap("textarea_full_non_final_line_has_a_visible_cursor_row");
}

// ORACLE bottom_pane/textarea.rs:3916 trailing_spaces_wrap_without_letting_the_cursor_escape kind=snap owner=chat-live snap=textarea_trailing_spaces_wrap_without_cursor_overflow
#[test]
fn trailing_spaces_wrap_without_letting_the_cursor_escape() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad" and eight spaces in a 5x3 rect, the caret at the end: three rows, the caret stays inside.
    let s = draft(5, 3, "abad        ");
    s.assert_snap("textarea_trailing_spaces_wrap_without_cursor_overflow");
}

// ORACLE bottom_pane/textarea.rs:3936 separator_after_full_line_hangs_before_following_text kind=snap owner=chat-live snap=textarea_separator_after_full_line_hangs_before_following_text
#[test]
fn separator_after_full_line_hangs_before_following_text() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad a" in a 4x2 rect with the caret at the end is at (1, 1); the same holds for the three other texts.
    let s = draft(4, 2, "abad a");
    s.assert_cursor(1, 1);
    s.assert_snap("textarea_separator_after_full_line_hangs_before_following_text");
    for text in ["界界 a", "abｶﾞ a", "abad\ta"] {
        let s = draft(4, 2, text);
        s.assert_cursor(1, 1);
    }
}

// ORACLE bottom_pane/textarea.rs:3965 breakable_unicode_space_hangs_before_following_text kind=snap owner=chat-live snap=textarea_breakable_unicode_space_hangs_before_following_text
#[test]
fn breakable_unicode_space_hangs_before_following_text() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad" + U+3000 + "abcde" in a 4x3 rect: the caret at the end is at (1, 2).
    let s = draft(4, 3, "abad\u{3000}abcde");
    s.assert_cursor(1, 2);
    s.assert_snap("textarea_breakable_unicode_space_hangs_before_following_text");
}

// ORACLE bottom_pane/textarea.rs:3986 leading_space_reflows_across_following_wrapped_fragments kind=snap owner=chat-live snap=textarea_leading_space_reflows_across_following_wrapped_fragments
#[test]
fn leading_space_reflows_across_following_wrapped_fragments() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad abcde wxyz" in a 4x5 rect: the carets 11, 14 and 15 are at (0, 3), (3, 3) and (0, 4).
    let mut s = draft(4, 5, "abad abcde wxyz");
    let mut at = 15;
    for (cursor, x, y) in [(11, 0, 3), (14, 3, 3), (15, 0, 4)] {
        step(&mut s, at, cursor);
        at = cursor;
        s.assert_cursor(x, y);
    }
    // oracle: "abad abcde xy zz" in a 4x4 rect: the carets listed, then the snapshot at the last caret (16).
    let mut s = draft(4, 4, "abad abcde xy zz");
    let mut at = 16;
    for (cursor, x, y) in [
        (4, 0, 1),
        (5, 0, 1),
        (8, 3, 1),
        (9, 0, 2),
        (10, 1, 2),
        (11, 2, 2),
        (12, 3, 2),
        (13, 0, 3),
        (14, 0, 3),
        (16, 2, 3),
    ] {
        step(&mut s, at, cursor);
        at = cursor;
        s.assert_cursor(x, y);
    }
    s.assert_snap("textarea_leading_space_reflows_across_following_wrapped_fragments");
}

// ORACLE bottom_pane/textarea.rs:4029 reflow_keeps_fitting_remainder_on_current_row kind=snap owner=chat-live snap=textarea_reflow_keeps_fitting_remainder_on_current_row
#[test]
fn reflow_keeps_fitting_remainder_on_current_row() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: four texts, each with the caret at its end in a rect of its own width and height, with the caret cells listed; then "p     a b" in a 5x2 rect is the snapshot.
    for (text, width, height, x, y) in [
        ("  -", 1, 4, 0, 3),
        ("p    a b", 3, 3, 0, 2),
        ("p     a b", 5, 2, 3, 1),
        (" aaaa a   ", 3, 4, 1, 3),
    ] {
        let s = draft(width, height, text);
        s.assert_cursor(x, y);
    }
    let s = draft(5, 2, "p     a b");
    s.assert_snap("textarea_reflow_keeps_fitting_remainder_on_current_row");
}

// ORACLE bottom_pane/textarea.rs:4068 reflow_preserves_hyphenated_word_breakpoint_when_rendering kind=snap owner=chat-live snap=textarea_reflow_preserves_hyphenated_word_breakpoint
#[test]
fn reflow_preserves_hyphenated_word_breakpoint_when_rendering() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "a foo-barbaz" in a 10x2 rect: the hyphen is a break point, and the snapshot shows the wrap.
    let s = draft(10, 2, "a foo-barbaz");
    s.assert_snap("textarea_reflow_preserves_hyphenated_word_breakpoint");
}

// ORACLE bottom_pane/textarea.rs:4087 space_runs_hang_before_later_text kind=ui owner=chat-live
#[test]
fn space_runs_hang_before_later_text() {
    // oracle: "abad     next" in a 4x3 rect: every caret from byte 4 to 9 (the space run) is at (0, 1).
    let mut s = draft(4, 3, "abad     next");
    let mut at = 13;
    for cursor in 4..=9 {
        step(&mut s, at, cursor);
        at = cursor;
        s.assert_cursor(0, 1);
    }
}

// ORACLE bottom_pane/textarea.rs:4100 full_final_line_scrolls_to_its_visible_cursor_row kind=snap owner=chat-live snap=textarea_full_final_line_scrolls_to_visible_cursor_row
#[test]
fn full_final_line_scrolls_to_its_visible_cursor_row() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abcdefgh" in a 4x2 rect with the caret at the end: the view scrolls to the caret's row; the snapshot shows the scroll and the cursor.
    let s = draft(4, 2, "abcdefgh");
    s.assert_snap("textarea_full_final_line_scrolls_to_visible_cursor_row");
}

// ORACLE bottom_pane/textarea.rs:4125 text_rendering_does_not_overwrite_cells_outside_the_textarea kind=ui owner=chat-live
#[test]
fn text_rendering_does_not_overwrite_cells_outside_the_textarea() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad     next" in a 4x3 rect over an 8x3 buffer whose columns 4.. are red: the cell at column 4 keeps its red style.
    let s = draft(8, 3, "abad     next");
    s.assert_style(4, 0, "fg=Red");
}

// ORACLE bottom_pane/textarea.rs:4140 styled_text_rendering_does_not_overwrite_cells_outside_the_textarea kind=snap owner=chat-live snap=textarea_styled_overlays_stay_inside_the_textarea
#[test]
fn styled_text_rendering_does_not_overwrite_cells_outside_the_textarea() {
    kit::pending(
        Owner::ChatLive,
        &format!(
            "{WRAP}; no element or highlight seed: the port's composer makes an atomic element only from a pasted attachment, and no seed sets a highlight range"
        ),
    );
    // oracle: "abad     next" in a 4x3 rect over a red "KEEP" at column 4, once with the element 0..9 and once with a magenta highlight 0..9; the snapshot joins both frames, and one screen is one frame in the kit.
    let s = draft(8, 3, "abad     next");
    s.assert_snap("textarea_styled_overlays_stay_inside_the_textarea");
}

// ORACLE bottom_pane/textarea.rs:4198 render_highlights_apply_style_without_mutating_text kind=ui owner=chat-live
#[test]
fn render_highlights_apply_style_without_mutating_text() {
    kit::pending(
        Owner::ChatLive,
        "no highlight seed: the port's composer applies no highlight range to its text",
    );
    // oracle: "hello world" in a 20x1 rect with "world" (bytes 6..11) reversed; the text is unchanged. The oracle's check that column 0 is not reversed has no kit form and is not written.
    let s = draft(20, 1, "hello world");
    s.assert_contains("hello world");
    s.assert_text_style("world", "reversed");
}

// ORACLE bottom_pane/textarea.rs:4235 tabs_render_as_spaces_and_align_with_cursor_snapshot kind=snap owner=chat-live snap=textarea_tabs_render_as_spaces_and_align_with_cursor
#[test]
fn tabs_render_as_spaces_and_align_with_cursor_snapshot() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: a line with tabs and an emoji in a 100x1 terminal: the tabs draw as spaces and the caret aligns with them (the snapshot).
    let s = draft(
        100,
        1,
        "❌\tSimulation\tformatter[large/dataset.py]\t7.4 ms\t8.1 ms\t-8.29%",
    );
    s.assert_snap("textarea_tabs_render_as_spaces_and_align_with_cursor");
}

// ORACLE bottom_pane/textarea.rs:4262 tabs_wrap_at_their_rendered_width kind=ui owner=chat-live
#[test]
fn tabs_wrap_at_their_rendered_width() {
    // oracle: "1234\t5" in a 5x2 rect: the tab fills to column 4 and "5" wraps to row 1: the caret is at (1, 1) and cell (0, 1) reads "5".
    let s = draft(5, 2, "1234\t5");
    s.assert_cursor(1, 1);
    s.assert_text_at(0, 1, "5");
}

// ORACLE bottom_pane/textarea.rs:4277 halfwidth_sound_marks_wrap_and_align_with_the_cursor kind=snap owner=chat-live snap=textarea_halfwidth_sound_marks_wrap_and_align_with_cursor
#[test]
fn halfwidth_sound_marks_wrap_and_align_with_the_cursor() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: three half-width sound-mark texts, each with the caret at its end in its own rect: the caret and the cells listed. The oracle joins the three frames in one snapshot; one screen is one frame in the kit, so the snapshot is not compared here.
    for (text, width, height, x, y, cells) in [
        ("12ｶﾞx", 4, 2, 1, 1, vec![(2, 0, "ｶﾞ"), (0, 1, "x")]),
        ("abﾊﾟc", 3, 3, 0, 2, vec![(0, 1, "ﾊﾟ"), (2, 1, "c")]),
        ("a ﾞb", 2, 3, 0, 2, vec![(0, 1, "ﾞ"), (1, 1, "b")]),
    ] {
        let s = draft(width, height, text);
        s.assert_cursor(x, y);
        for (cx, cy, expected) in cells {
            s.assert_text_at(cx, cy, expected);
        }
    }
}

// ORACLE bottom_pane/textarea.rs:4327 masked_graphemes_align_with_the_cursor kind=snap owner=chat-live snap=textarea_masked_graphemes_align_with_cursor
#[test]
fn masked_graphemes_align_with_the_cursor() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "界ﾞa" in a 4x2 rect with the caret at the end: the caret is at (0, 1); the snapshot shows the masked grapheme.
    let s = draft(4, 2, "界ﾞa");
    s.assert_cursor(0, 1);
    s.assert_snap("textarea_masked_graphemes_align_with_cursor");
}

// ORACLE bottom_pane/textarea.rs:4351 overwide_halfwidth_sound_marks_keep_the_final_cursor_visible kind=ui owner=chat-live
#[test]
fn overwide_halfwidth_sound_marks_keep_the_final_cursor_visible() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "ｶﾞﾞab" and "界ﾞab" in a 2x3 rect: the caret at the end is at (0, 2); after the first grapheme it is at (0, 1).
    for grapheme in ["ｶﾞﾞ", "界ﾞ"] {
        let mut s = draft(2, 3, &format!("{grapheme}ab"));
        s.assert_cursor(0, 2);
        step(&mut s, grapheme.len() + 2, grapheme.len());
        s.assert_cursor(0, 1);
    }
}

// ORACLE bottom_pane/textarea.rs:4368 tall_draft_height_saturates_at_the_terminal_limit kind=ui owner=chat-live
#[test]
fn tall_draft_height_saturates_at_the_terminal_limit() {
    // oracle: 65535 newlines and an "x" in a 32x2 rect: the "x" is drawn at (0, 1) and the caret at (1, 1).
    let text = "\n".repeat(usize::from(u16::MAX)) + "x";
    let s = draft(32, 2, &text);
    s.assert_text_at(0, 1, "x");
    s.assert_cursor(1, 1);
}

// ORACLE bottom_pane/textarea.rs:4390 cursor_pos_with_state_basic_and_scroll_behaviors kind=ui owner=chat-live
#[test]
fn cursor_pos_with_state_basic_and_scroll_behaviors() {
    // oracle: case 1, "hello world" with the caret on byte 3 in a rect at (2, 5): the caret is at (5, 5) whatever the state's scroll. Cases 2 and 3 assert the row only (bottom row, then top row); the kit has no row-only caret form, so they are not written here.
    let mut s = draft(20, 3, "hello world");
    step(&mut s, 11, 3);
    // the bare area sits at (0, 0): the oracle's (5, 5) less its rect's (2, 5)
    s.assert_cursor(5 - 2, 5 - 5);
}

// ORACLE bottom_pane/textarea.rs:4438 wrapped_navigation_across_visual_lines kind=ui owner=chat-live
#[test]
fn wrapped_navigation_across_visual_lines() {
    // oracle: "abcdefghij" at width 4 wraps to "abcd", "efgh", "ij": from byte 0 down is byte 4, at (0, 1) in a 4x10 rect. From byte 6, up keeps the column (byte 2) and down returns to byte 6; down again clamps to the end. The last three caret cells are derived from those byte asserts.
    let mut s = draft(4, 10, "abcdefghij");
    step(&mut s, 10, 0);
    s.keys("down");
    s.assert_cursor(0, 1);
    step(&mut s, 4, 6);
    s.keys("up");
    s.assert_cursor(2, 0);
    s.keys("down");
    s.assert_cursor(2, 1);
    s.keys("down");
    s.assert_cursor(2, 2);
}

// ORACLE bottom_pane/textarea.rs:4476 cursor_pos_with_state_after_movements kind=ui owner=chat-live
#[test]
fn cursor_pos_with_state_after_movements() {
    // oracle: "abcdefghij" at width 4 in a 4x2 rect: from the start the caret is (0, 0); down to the second visual line (0, 1); down again the view scrolls and the caret stays on the bottom row (0, 1); up is the top row (0, 0); from byte 2, down keeps column 2: (2, 0) then (2, 1).
    let mut s = draft(4, 2, "abcdefghij");
    step(&mut s, 10, 0);
    s.assert_cursor(0, 0);
    s.keys("down");
    s.assert_cursor(0, 1);
    s.keys("down");
    s.assert_cursor(0, 1);
    s.keys("up");
    s.assert_cursor(0, 0);
    step(&mut s, 4, 2);
    s.assert_cursor(2, 0);
    s.keys("down");
    s.assert_cursor(2, 1);
}

// ORACLE bottom_pane/textarea.rs:4520 wrapped_navigation_with_newlines_and_spaces kind=ui owner=chat-live
#[test]
fn wrapped_navigation_with_newlines_and_spaces() {
    // oracle: "word1  word2\nword3" at width 6 with the caret on byte 8 (column 1 of word2): up is byte 1, down returns to byte 8, down crosses the newline to byte 14. The caret cells are derived from the wrapped rows.
    let mut s = draft(6, 10, "word1  word2\nword3");
    step(&mut s, 18, 8);
    s.keys("up");
    s.assert_cursor(1, 0);
    s.keys("down");
    s.assert_cursor(1, 1);
    s.keys("down");
    s.assert_cursor(1, 2);
}

// ORACLE bottom_pane/textarea.rs:4545 wrapped_navigation_with_wide_graphemes kind=ui owner=chat-live
#[test]
fn wrapped_navigation_with_wide_graphemes() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: four thumbs-up (two cells each) at width 3, the caret after the second: down moves on, and up returns to the caret after the second. The caret cell (2, 1) is derived: one emoji per row at width 3.
    let mut s = draft(3, 4, "👍👍👍👍");
    step(&mut s, 16, 8);
    s.keys("down");
    s.keys("up");
    s.assert_cursor(2, 1);
}

// ORACLE bottom_pane/textarea.rs:4565 fuzz_textarea_randomized kind=ui owner=chat-live
#[test]
fn fuzz_textarea_randomized() {
    kit::na(
        Owner::ChatLive,
        "asserts invariants of random edits on the state (char boundaries, element payloads, scroll over 500 cases): no fixed row or caret is asserted",
    );
}

// ---- oracle: bottom_pane/textarea/editing_tests.rs (1 screen-level tests here; 2 state tests left to chat-live)
// ORACLE bottom_pane/textarea/editing_tests.rs:8 target_replaces_forward_and_backward_selections kind=ui owner=chat-live
#[test]
fn target_replaces_forward_and_backward_selections() {
    // oracle: "abcdef" in a 20x2 rect; a drag from byte 1 to byte 4 (and back) selects "bcd", and inserting "界" there gives "a界ef". The coordinates are the oracle's rect origin; the composer's text origin differs (prompt column).
    for (anchor, cursor) in [(1, 4), (4, 1)] {
        let mut s = draft(20, 2, "abcdef");
        s.drag(anchor, 0, cursor, 0);
        s.type_text("界");
        s.assert_contains("a界ef");
    }
}

// ---- oracle: bottom_pane/textarea/hyperlinks_tests.rs (12 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:87 wrapped_url_fragments_keep_the_complete_destination kind=snap owner=chat-live snap=wrapped_textarea_url_keeps_original_appearance
#[test]
fn wrapped_url_fragments_keep_the_complete_destination() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: "Fix CI on <url>" in a 26x5 rect: every cell of the url links to it, and the visible rows are the snapshot.
    let s = draft(26, 5, &format!("Fix CI on {URL_PR}"));
    s.assert_link(URL_PR, URL_PR);
    s.assert_snap("wrapped_textarea_url_keeps_original_appearance");
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:115 composer_wrapped_url_fragments_keep_the_complete_destination kind=ui owner=chat-live
#[test]
fn composer_wrapped_url_fragments_keep_the_complete_destination() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: the chat composer with "Fix CI on <url>" in a 34x8 rect: every cell of the url links to it. The port's chat composer is its one textarea: it draws its prompt and footer, and draws no links.
    let s = draft(34, 8, &format!("Fix CI on {URL_PR}"));
    s.assert_link(URL_PR, URL_PR);
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:139 scrolled_url_fragments_keep_the_offscreen_destination kind=ui owner=chat-live
#[test]
fn scrolled_url_fragments_keep_the_offscreen_destination() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: a url longer than a 16x2 rect scrolls: the whole url is not on the screen, and the visible cells link to it. The kit has no form for "the visible text is a suffix of the url" (the oracle's ends_with check), so that check is not written.
    let s = draft(16, 2, URL_LONG);
    s.assert_absent(URL_LONG);
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:149 long_drafts_reuse_hyperlink_detection_across_cursor_redraws kind=ui owner=chat-live
#[test]
fn long_drafts_reuse_hyperlink_detection_across_cursor_redraws() {
    kit::na(
        Owner::ChatLive,
        "asserts the identity of the hyperlink cache across cursor redraws (pointer equality and cache contents): a state test, no row or caret is asserted",
    );
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:170 maximum_length_urls_render_without_osc8_annotations kind=snap owner=chat-live snap=maximum_length_url_without_annotations
#[test]
fn maximum_length_urls_render_without_osc8_annotations() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: a url of the maximum draft length (MAX_USER_INPUT_TEXT_CHARS) in an 80x2 rect has no link cells; the snapshot shows the rows.
    let text = format!(
        "https://example.test/{}",
        "a".repeat(MAX_USER_INPUT_TEXT_CHARS - "https://example.test/".len())
    );
    let s = draft(80, 2, &text);
    s.assert_no_link("https://example.test/");
    s.assert_snap("maximum_length_url_without_annotations");
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:188 many_urls_render_with_the_complete_destination kind=ui owner=chat-live
#[test]
fn many_urls_render_with_the_complete_destination() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: "https://x.io " fifty thousand times in an 80x2 rect: the view is scrolled, and the destination links to itself.
    let text = "https://x.io ".repeat(50_000);
    let s = draft(80, 2, &text);
    s.assert_link("https://x.io", "https://x.io");
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:211 joined_emoji_preserve_complete_url_cell_ranges kind=snap owner=chat-live snap=joined_emoji_url_cell_ranges
#[test]
fn joined_emoji_preserve_complete_url_cell_ranges() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: a joined emoji before each url, at widths 64, 24 and 16 in six rows: the url's cells link to it. The oracle joins the cell ranges in one snapshot; one screen is one frame here, so the links are compared per screen.
    for url in ["https://example.test/path", "https://example.test/👩‍💻/path"] {
        for width in [64, 24, 16] {
            let s = draft(width, 6, &format!("👩‍💻 {url} tail"));
            s.assert_link(url, url);
        }
    }
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:239 unicode_whitespace_separates_url_destinations kind=snap owner=chat-live snap=unicode_whitespace_url_destinations
#[test]
fn unicode_whitespace_separates_url_destinations() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: two urls with each unicode separator between them, at widths 48 and 20 in four rows: each url's cells link to its own destination. The oracle joins the frames in one snapshot; one screen is one frame here.
    let (first, second) = ("https://one.test/a", "https://two.test/b");
    for separator in ['\u{2003}', '\u{a0}', '\u{2007}', '\u{202f}', '\u{2028}'] {
        for width in [48, 20] {
            let s = draft(width, 4, &format!("{first}{separator}{second}"));
            s.assert_link(first, first);
            s.assert_link(second, second);
        }
    }
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:271 hyperlink_cache_is_invalidated_when_text_changes kind=ui owner=chat-live
#[test]
fn hyperlink_cache_is_invalidated_when_text_changes() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: "https://one.test/before" replaced by "https://two.test/after" in a 40x1 rect: the new url links to its own destination.
    let mut s = draft(40, 1, "https://one.test/before");
    s.keys(&vec!["backspace"; 23].join(" "));
    s.type_text("https://two.test/after");
    s.assert_link("https://two.test/after", "https://two.test/after");
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:286 distinct_urls_respect_punctuation_wide_prefixes_and_tabs kind=ui owner=chat-live
#[test]
fn distinct_urls_respect_punctuation_wide_prefixes_and_tabs() {
    kit::pending(Owner::ChatLive, LINK);
    // oracle: "界\t(<first>),\n<second>!" in a 48x2 rect: the first url starts at (4, 0) and the second at (0, 1), with the punctuation outside the link. The kit's link check covers the two urls; the cell columns are not written.
    let (first, second) = ("https://one.test/a", "https://two.test/b");
    let s = draft(48, 2, &format!("界\t({first}),\n{second}!"));
    s.assert_link(first, first);
    s.assert_link(second, second);
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:308 url_hyperlinks_preserve_existing_highlight_styles kind=ui owner=chat-live
#[test]
fn url_hyperlinks_preserve_existing_highlight_styles() {
    kit::pending(
        Owner::ChatLive,
        &format!("{LINK}; no highlight seed: the port's composer applies no highlight range"),
    );
    // oracle: "https://example.test/highlight" in a 40x1 rect with a magenta highlight over the url: the cells keep the magenta foreground and the link.
    let s = draft(40, 1, "https://example.test/highlight");
    s.assert_link(
        "https://example.test/highlight",
        "https://example.test/highlight",
    );
    s.assert_text_style("https://example.test/highlight", "fg=Magenta");
}

// ORACLE bottom_pane/textarea/hyperlinks_tests.rs:337 masked_url_input_never_exposes_hyperlink_destinations kind=ui owner=chat-live
#[test]
fn masked_url_input_never_exposes_hyperlink_destinations() {
    kit::pending(
        Owner::ChatLive,
        &format!("{LINK}; no masked-input mode: the port's composer has no masked draft"),
    );
    // oracle: "https://example.test/secret" in masked input: no cell links, and the url is not drawn.
    let s = draft(40, 1, "https://example.test/secret");
    s.assert_absent("https://example.test/secret");
    s.assert_no_link("https://example.test/secret");
}

// ---- oracle: bottom_pane/textarea/mouse_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/textarea/mouse_tests.rs:30 wheel_at_edges_preserves_caret_following_or_browsing_after_resize kind=snap owner=chat-live snap=wheel_at_edges_after_resize
#[test]
fn wheel_at_edges_preserves_caret_following_or_browsing_after_resize() {
    kit::pending(Owner::ChatLive, MOUSE);
    // oracle: six lines in a 14x4 rect: wheel events at the edges, then resizes to 14x2 and 7x4, with and without browsing; the snapshot joins the frames. One screen is one frame in the kit, so the last frame is compared.
    let mut s = draft(
        14,
        4,
        "line one abc\nline two abc\nline three ab\nline four abc\nline five abc\nline six abc",
    );
    s.wheel_up(0, 0, 1);
    s.wheel_up(0, 0, 1);
    s.assert_snap("wheel_at_edges_after_resize");
}

// ORACLE bottom_pane/textarea/mouse_tests.rs:88 mouse_click_maps_wrapping_unicode_tabs_and_empty_lines kind=ui owner=chat-live
#[test]
fn mouse_click_maps_wrapping_unicode_tabs_and_empty_lines() {
    kit::pending(Owner::ChatLive, MOUSE);
    // oracle: "ab 界e\u{301}👩‍💻\n\nx\ty" in an 8x8 rect at (3, 2): a click on the wide 界 (both cells) places the caret before it, so "!" inserts there: "ab !界e\u{301}👩‍💻". The click cells are the oracle's rect; the composer's text origin differs.
    let mut s = draft(8, 8, "ab 界e\u{301}👩‍💻\n\nx\ty");
    s.click(7, 2);
    s.type_text("!");
    s.assert_contains("ab !界e\u{301}👩‍💻");
}

// ORACLE bottom_pane/textarea/mouse_tests.rs:113 mouse_drag_highlights_and_edits_wrapped_selection kind=snap owner=chat-live snap=mouse_wrapped_selection
#[test]
fn mouse_drag_highlights_and_edits_wrapped_selection() {
    kit::pending(Owner::ChatLive, MOUSE);
    // oracle: "hello world!" in an 8x1 rect at row 1: a drag selects the wrapped part and the key edits it: "X" gives "hXld!", and the delete, backward-word, forward-word and kill-line keys give "hld!". The oracle's vim cases (plain keys with vim on) are not written here.
    for (start, end, key, expected) in [
        ((1, 1), (3, 2), "X", "hXld!"),
        ((3, 1), (1, 0), "backspace", "hld!"),
        ((1, 1), (3, 2), "delete", "hld!"),
        ((3, 1), (1, 0), "ctrl+backspace", "hld!"),
        ((1, 1), (3, 2), "ctrl+delete", "hld!"),
        ((1, 1), (3, 2), "ctrl+k", "hld!"),
    ] {
        let mut s = draft(8, 1, "hello world!");
        s.drag(start.0, start.1, end.0, end.1);
        s.keys(key);
        s.assert_contains(expected);
    }
}

// ORACLE bottom_pane/textarea/mouse_tests.rs:180 mouse_multiclick_preserves_the_initial_word_or_wrapped_line_when_reversing kind=ui owner=chat-live
#[test]
fn mouse_multiclick_preserves_the_initial_word_or_wrapped_line_when_reversing() {
    kit::pending(Owner::ChatLive, MOUSE);
    // oracle: "one two six\nnext" in an 8x4 rect: a double click on "two" selects "two", and a drag back to column 1 selects "one two"; a triple click selects the line "one two six\n", and the drag back selects the same line. The selection highlight is not compared here.
    for clicks in [2, 3] {
        let mut s = draft(8, 4, "one two six\nnext");
        for _ in 0..clicks {
            s.click(5, 0);
        }
        s.drag(5, 0, 1, 0);
        s.assert_contains("one two six");
    }
}

// ---- oracle: bottom_pane/textarea/vim_commands_tests.rs (3 screen-level tests here; 30 state tests left to chat-live)
// ORACLE bottom_pane/textarea/vim_commands_tests.rs:695 editor_commands_have_visual_snapshot_coverage kind=snap owner=chat-live
#[test]
fn editor_commands_have_visual_snapshot_coverage() {
    // oracle: "alpha beta\ngamma delta" at the start in vim normal mode: `rZ` gives "Zlpha beta" on the first row; `cw` then gives " beta" with "gamma delta" below.
    let mut s = vim_draft(80, 24, "alpha beta\ngamma delta");
    s.keys("0 r Z");
    s.assert_in_order(&["Zlpha beta", "gamma delta"]);
    s.keys("c w");
    s.assert_in_order(&[" beta", "gamma delta"]);
}

// ORACLE bottom_pane/textarea/vim_commands_tests.rs:716 dot_repeat_has_visual_snapshot_coverage kind=snap owner=chat-live
#[test]
fn dot_repeat_has_visual_snapshot_coverage() {
    // oracle: "alpha beta\ngamma delta" at the start in vim normal mode: `dw` leaves "beta\ngamma delta", and `.` repeats it, leaving "gamma delta".
    let mut s = vim_draft(80, 24, "alpha beta\ngamma delta");
    s.keys("0 d w");
    s.assert_in_order(&["beta", "gamma delta"]);
    s.keys(".");
    s.assert_in_order(&["gamma delta"]);
}

// ORACLE bottom_pane/textarea/vim_commands_tests.rs:737 find_and_navigation_have_visual_snapshot_coverage kind=snap owner=chat-live
#[test]
fn find_and_navigation_have_visual_snapshot_coverage() {
    // oracle: "alpha beta\ngamma delta" at the start in vim normal mode: the motions tb, fb, Ta, Fa, G and gg move the caret and leave the text. The carets of the inline snapshot are not compared (the composer's text origin differs).
    let mut s = vim_draft(80, 24, "alpha beta\ngamma delta");
    s.keys("0");
    for command in ["t b", "f b", "T a", "F a", "G", "g g"] {
        s.keys(command);
        s.assert_in_order(&["alpha beta", "gamma delta"]);
    }
}

// ---- oracle: bottom_pane/textarea/wrapping_tests.rs (6 screen-level tests here; 8 state tests left to chat-live)
// ORACLE bottom_pane/textarea/wrapping_tests.rs:25 vertical_navigation_preserves_destination_tab_columns kind=ui owner=chat-live
#[test]
fn vertical_navigation_preserves_destination_tab_columns() {
    // oracle: "a\tb\nxyz" in an 8x2 rect with the caret on byte 5: up is byte 1 (after the "a") at (1, 0), and down is byte 5 at (1, 1).
    let mut s = draft(8, 2, "a\tb\nxyz");
    step(&mut s, 7, 5);
    s.keys("up");
    s.assert_cursor(1, 0);
    s.keys("down");
    s.assert_cursor(1, 1);
}

// ORACLE bottom_pane/textarea/wrapping_tests.rs:38 mandatory_breaks_keep_distinct_cursor_positions kind=snap owner=chat-live snap=mandatory_break_cursor
#[test]
fn mandatory_breaks_keep_distinct_cursor_positions() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abad" + each mandatory break + "next" in a 4x3 rect: the rows are "abad", "<break>nex", "t"; the caret at byte 4 is at (0, 1), after the break at (1, 1), and left returns. The U+2028 case is the snapshot.
    for separator in ["\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}", "\r"] {
        let text = format!("abad{separator}next");
        let row = format!("{separator}nex");
        let mut s = draft(4, 3, &text);
        s.assert_in_order(&["abad", row.as_str(), "t"]);
        for _ in 0..5 {
            s.keys("left");
        }
        s.assert_cursor(0, 1);
        s.keys("right");
        s.assert_cursor(1, 1);
        s.keys("left");
        s.assert_cursor(0, 1);
    }
    let s = draft(4, 3, "abad\u{2028}next");
    s.assert_snap("mandatory_break_cursor");
}

// ORACLE bottom_pane/textarea/wrapping_tests.rs:147 hanging_spaces_preserve_source_editing_and_visual_navigation kind=ui owner=chat-live
#[test]
fn hanging_spaces_preserve_source_editing_and_visual_navigation() {
    // oracle: "abad  next" in a 4x3 rect: the carets 4..=6 are all at (0, 1); left is byte 5, up is byte 0, down is byte 6; one backspace leaves "abad next" and another "abadnext", the caret at (0, 1) each time.
    let mut s = draft(4, 3, "abad  next");
    let mut at = 10;
    for pos in 4..=6 {
        step(&mut s, at, pos);
        at = pos;
        s.assert_cursor(0, 1);
    }
    s.keys("left");
    s.keys("up");
    s.keys("down");
    s.keys("backspace");
    s.assert_cursor(0, 1);
    s.keys("backspace");
    s.assert_cursor(0, 1);
    s.assert_in_order(&["abad", "next"]);
}

// ORACLE bottom_pane/textarea/wrapping_tests.rs:174 hanging_tabs_keep_cursor_scrolling_and_navigation_aligned kind=snap owner=chat-live snap=hanging_tab_cursor_and_scroll
#[test]
fn hanging_tabs_keep_cursor_scrolling_and_navigation_aligned() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abc\t a" in a 4x2 rect: the carets 0..=6 are at (0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (0, 1), (1, 1); the caret on byte 4 scrolls the view to row 0 (snapshot); up is (0, 0), and down is (0, 1).
    let mut s = draft(4, 2, "abc\t a");
    let mut at = 6;
    for (pos, x, y) in [
        (0, 0, 0),
        (1, 1, 0),
        (2, 2, 0),
        (3, 3, 0),
        (4, 0, 1),
        (5, 0, 1),
        (6, 1, 1),
    ] {
        step(&mut s, at, pos);
        at = pos;
        s.assert_cursor(x, y);
    }
    step(&mut s, at, 4);
    s.assert_cursor(0, 0);
    s.assert_snap("hanging_tab_cursor_and_scroll");
    s.keys("up");
    s.assert_cursor(0, 0);
    s.keys("down");
    s.assert_cursor(0, 1);
}

// ORACLE bottom_pane/textarea/wrapping_tests.rs:217 vertical_navigation_clamps_saved_column_after_resize kind=snap owner=chat-live snap=vertical_navigation_after_resize
#[test]
fn vertical_navigation_clamps_saved_column_after_resize() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: "abcdefghij\nabcd  xyz": up at width 10 is byte 10 at (0, 1); resized to width 4 the same byte is at (2, 2); down is byte 14 at (3, 3), down again byte 20 at (3, 4), and up byte 14 at (3, 3). The snapshot shows the last width.
    let mut s = draft(10, 6, "abcdefghij\nabcd  xyz");
    s.keys("up");
    s.assert_cursor(0, 1);
    s.resize(4, 6);
    s.assert_cursor(2, 2);
    s.keys("down");
    s.assert_cursor(3, 3);
    s.keys("down");
    s.assert_cursor(3, 4);
    s.keys("up");
    s.assert_cursor(3, 3);
    s.assert_snap("vertical_navigation_after_resize");
}

// ORACLE bottom_pane/textarea/wrapping_tests.rs:272 typing_after_eol_spaces_reflows_without_changing_text kind=snap owner=chat-live snap=typing_after_eol_spaces
#[test]
fn typing_after_eol_spaces_reflows_without_changing_text() {
    kit::pending(Owner::ChatLive, WRAP);
    // oracle: typing in a 4x2 rect: "abad" leaves the caret at (0, 1), three spaces at (3, 1), "x" at (1, 1), and one backspace at (3, 1); the text is "abad   ". The snapshot joins the four frames; one screen is one frame in the kit.
    let mut s = draft(4, 2, "");
    s.type_text("abad");
    s.assert_cursor(0, 1);
    s.type_text("   ");
    s.assert_cursor(3, 1);
    s.type_text("x");
    s.assert_cursor(1, 1);
    s.keys("backspace");
    s.assert_cursor(3, 1);
    s.assert_contains("abad");
}
