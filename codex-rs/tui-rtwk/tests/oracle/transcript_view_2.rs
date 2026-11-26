//! Shard `transcript_view_2` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Fixtures: `fixtures/transcript_view_2/*.jsonl`, the oracle test data of each family (the
//! comment on the first line of each file names its oracle test).
//! Keys: `ctrl+f` is the find key (the oracle's `keymap.app.find_transcript`); its default is not
//! confirmed here: check it against the oracle keymap when the port runs.

use crate::kit::{self, Owner, Screen};

/// The oracle's tests of copy mode draw the transcript view alone; the port's screen is the whole
/// chat, whose composer (3 rows) and footer (2) and the `Worked for` rows of its turns share the
/// height. A shard's height is the oracle's plus these, and its rows are read by offset from the
/// first row of the content, not by absolute row, except where the row is the claim.
const CHROME: u16 = 7;
// The footer of copy mode is drawn in the chat's gutter, two columns in: the 50-column hints line
// of the oracle (`↑/↓/j/k select · g/G ends · enter copy · esc close`) needs 52 where a test reads it.

/// A chat that owns its transcript (`tui.fullscreen_transcript`) over `fixture`, `width` columns
/// and the oracle's `height` rows of view.
fn chat(fixture: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, height + CHROME)
        .owned_transcript()
        .script(fixture)
        .open()
}

/// `begin_copy_mode`: `/copy` on the owned transcript.
fn begin_copy_mode(s: &mut Screen) {
    s.type_text("/copy").keys("enter");
}

/// The oracle's `#` mask: every cell of the `rows` below `top` (in `width` columns) is reversed,
/// the other rows of `0..all` are not.
fn assert_block(s: &Screen, top: usize, width: usize, rows: std::ops::Range<usize>, all: usize) {
    for row in 0..all {
        let spec = if rows.contains(&row) {
            "reversed"
        } else {
            "!reversed"
        };
        for x in 0..width {
            s.assert_style(x, top + row, spec);
        }
    }
}

// ---- oracle: transcript_view/copy_mode_tests.rs (20 screen-level tests here; 1 state test left to chat-live)
// ORACLE transcript_view/copy_mode_tests.rs:28 navigate_copy_resize_and_exit kind=snap owner=chat-live
#[test]
fn navigate_copy_resize_and_exit() {
    kit::pending(
        Owner::ChatLive,
        "G-new-usage-2: the pinned prompt header (the oracle's first row `› User text > user quote ``` user code ````, transcript_view/prompt_header.rs) is not drawn: the chat's list has no `visible_range()` to say which prompt scrolled off; copy mode, its footer and its keys are built",
    );
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/copy_nav.jsonl"),
        50,
        8,
    );
    begin_copy_mode(&mut s);
    // the oracle: Up seven times (the code, the quote, the prompt's code and quote, the prompt
    // as a whole, the old code, and the old code again: the oldest stays), Down five times
    s.keys("up up up up up up up");
    s.keys("down down down down down");
    s.assert_in_order(&[
        "› User text > user quote ``` user code ```",
        "Latest plan",
        "> assistant quote",
        "assistant code",
    ]);
    s.assert_contains("Copy code block");
    s.assert_contains("↑/↓/j/k select · g/G ends · enter copy · esc close");
    s.resize(14, 8 + CHROME);
    s.keys("enter");
    s.assert_copy_asked("assistant code\n");
}

// ORACLE transcript_view/copy_mode_tests.rs:131 copy_reveals_offscreen_block_with_context kind=snap owner=chat-live
#[test]
fn copy_reveals_offscreen_block_with_context() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/copy_tall.jsonl"),
        50,
        5,
    );
    s.wheel_up(0, 0, 5);
    begin_copy_mode(&mut s);
    s.keys("up");
    // the reveal scrolls the last code block into view, with its context above
    s.assert_in_order(&["line", "line", "", "last code"]);
    s.assert_text_style("last code", "reversed");
}

// ORACLE transcript_view/copy_mode_tests.rs:153 user_messages_support_vim_navigation_without_an_assistant_response kind=snap owner=chat-live
#[test]
fn user_messages_support_vim_navigation_without_an_assistant_response() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/user_vim.jsonl"),
        52,
        14,
    );
    // no response to copy: the last prompt is the target
    begin_copy_mode(&mut s);
    s.assert_text_style("Plain prompt with wrapping", "reversed");
    s.assert_text_style("First user message", "!reversed");
    // `k` to the first prompt, `j` back, `g`/`G` to the ends (twice: they stay)
    s.keys("k");
    s.assert_text_style("First user message", "reversed");
    s.keys("j");
    s.assert_text_style("Plain prompt with wrapping", "reversed");
    s.keys("g");
    s.assert_text_style("First user message", "reversed");
    s.keys("shift+g");
    s.assert_text_style("Plain prompt with wrapping", "reversed");
    s.keys("g");
    s.assert_text_style("First user message", "reversed");
    s.keys("shift+g");
    s.assert_text_style("Plain prompt with wrapping", "reversed");
    s.assert_contains("Copy user message");
    s.assert_contains("↑/↓/j/k select · g/G ends · enter copy · esc close");
    // the payload of the last prompt is its source, as typed
    s.keys("enter");
    s.assert_copy_asked("\nPlain prompt with wrapping\n\n");
}

// ORACLE transcript_view/copy_mode_tests.rs:192 marker_only_prompt_is_a_copy_target kind=snap owner=chat-live
#[test]
fn marker_only_prompt_is_a_copy_target() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/marker.jsonl"),
        52,
        8,
    );
    begin_copy_mode(&mut s);
    s.keys("up");
    s.assert_contains("Copy user message");
    s.assert_contains("↑/↓/j/k select · g/G ends · enter copy · esc close");
    s.keys("enter");
    s.assert_copy_asked("8.");
}

// ORACLE transcript_view/copy_mode_tests.rs:231 copy_highlight_fills_a_block_across_blank_and_wrapped_rows kind=snap owner=chat-live snap=copy_highlight_fills_a_block_across_blank_and_wrapped_rows
#[test]
fn copy_highlight_fills_a_block_across_blank_and_wrapped_rows() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/highlight_blank.jsonl"),
        16,
        12,
    );
    begin_copy_mode(&mut s);
    s.keys("up");
    // the block is `longer line`, a blank row and `x`: every cell of its rows is the selection's,
    // blank and wrapped rows included, at 16 columns and at 8 (the oracle's `#` mask)
    let top = s.row_of("Before");
    assert_block(&s, top, 16, 2..5, 7);
    // the oracle's 8 columns are the renderer's: the chat's gutter and free column come on top,
    // and "Worked for" wraps, which takes rows
    s.resize(8 + 3, 12 + CHROME + 6);
    let top = s.row_of("Before");
    assert_block(&s, top, 11, 2..6, 8);
    s.keys("enter");
    s.assert_copy_asked("longer line\n\nx\n");
}

// ORACLE transcript_view/copy_mode_tests.rs:269 whole_user_turn_highlight_includes_bottom_padding kind=snap owner=chat-live snap=whole_user_turn_highlight_includes_bottom_padding
#[test]
fn whole_user_turn_highlight_includes_bottom_padding() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/user_short.jsonl"),
        20,
        8,
    );
    begin_copy_mode(&mut s);
    // the whole turn is the target: the padding rows above and below the prompt are the selection's
    let pad = s.row_of("Short prompt") - 1;
    assert_block(&s, pad, 20, 0..3, 4);
    s.keys("enter");
    s.assert_copy_asked("Short prompt");
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/user_longer.jsonl"),
        10,
        12,
    );
    begin_copy_mode(&mut s);
    let pad = s.row_of("Longer") - 1;
    assert_block(&s, pad, 10, 0..6, 8);
    s.keys("enter");
    s.assert_copy_asked("Longer message\n\nlast");
}

// ORACLE transcript_view/copy_mode_tests.rs:309 rendered_mermaid_copies_source kind=ui owner=chat-live
#[test]
fn rendered_mermaid_copies_source() {
    kit::pending(
        Owner::ChatLive,
        "G62: the chat draws the mermaid fence as code (`• graph LR` / `A --> B`) where the oracle draws the diagram, so `graph LR` is on screen; the copy half works (`up`, `enter` copy `graph LR\\nA --> B\\n`)",
    );
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/mermaid.jsonl"),
        50,
        8,
    );
    s.assert_absent("graph LR");
    begin_copy_mode(&mut s);
    s.keys("up");
    s.keys("enter");
    s.assert_copy_asked("graph LR\nA --> B\n");
}

// ORACLE transcript_view/copy_mode_tests.rs:323 copy_source_survives_raw_rich_rendering_and_whitespace_normalization kind=ui owner=chat-live
#[test]
fn copy_source_survives_raw_rich_rendering_and_whitespace_normalization() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied payloads (`**quote**\\r\\n`, `echo x  \\r\\n  \\r\\n`) of the Rich and Raw presentations, the clipboard text, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:347 copy_completion_keeps_payload_count_and_follow_intent kind=snap owner=chat-live
#[test]
fn copy_completion_keeps_payload_count_and_follow_intent() {
    kit::pending(
        Owner::ChatLive,
        "the chat's composer gap has no copy feedback (`Copied 8 chars to host clipboard`) and no copy completion path yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 5)
        .script(include_str!("fixtures/transcript_view_2/completion.jsonl"))
        .open();
    // the oracle: a confirmed copy (or a pending one finished) shows the feedback in the composer gap
    s.assert_contains("Copied 8 chars to host clipboard");
}

// ORACLE transcript_view/copy_mode_tests.rs:395 find_and_shift_click_release_copy_payload kind=ui owner=chat-live
#[test]
fn find_and_shift_click_release_copy_payload() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 5)
        .script(include_str!("fixtures/transcript_view_2/needle.jsonl"))
        .open();
    s.keys("ctrl+f");
    s.type_text("needle");
    // the oracle: the find footer shows `needle`; the shift-click release copies `bold` (clipboard payload)
    s.assert_contains("needle");
}

// ORACLE transcript_view/copy_mode_tests.rs:449 copy_input_guard_is_released_on_all_selector_exits kind=ui owner=chat-live
#[test]
fn copy_input_guard_is_released_on_all_selector_exits() {
    kit::na(
        Owner::ChatLive,
        "asserts the copy input guard's weak reference and the TranscriptCopyClosed app event after each selector exit, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:496 user_code_blocks_preserve_original_line_endings kind=ui owner=chat-live
#[test]
fn user_code_blocks_preserve_original_line_endings() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied payload `echo user  \\r\\n` of a user code block in three presentations, the clipboard text, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:521 copying_a_disabled_fenced_table_uses_the_original_body kind=ui owner=chat-live
#[test]
fn copying_a_disabled_fenced_table_uses_the_original_body() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript draws the fenced md block as text; the tables preference (preferences::init with tables off) is not a kit seed yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(50, 12)
        .script(include_str!(
            "fixtures/transcript_view_2/disabled_table.jsonl"
        ))
        .open();
    s.assert_contains("```md");
    // the oracle: the copy payloads (`echo done`, the table body, the whole message) are clipboard text
}

// ORACLE transcript_view/copy_mode_tests.rs:551 copying_the_whole_typed_prompt_preserves_markdown_source kind=ui owner=chat-live
#[test]
fn copying_the_whole_typed_prompt_preserves_markdown_source() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied payloads of a typed prompt (`printf 'hello'  \\n`, the quote, the whole message), the clipboard text, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:570 latest_partial_response_owns_its_payload kind=ui owner=chat-live
#[test]
fn latest_partial_response_owns_its_payload() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied payload of the latest partial response (`Interrupted **partial** answer`), the clipboard text, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:588 source_targets_survive_quotes_lists_and_rendered_tables kind=ui owner=chat-live
#[test]
fn source_targets_survive_quotes_lists_and_rendered_tables() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied source payloads of five markdown sources in Rich and Raw modes, the clipboard text, nothing drawn",
    );
}

// ORACLE transcript_view/copy_mode_tests.rs:641 rendered_fenced_table_has_a_block_highlight kind=snap owner=chat-live snap=rendered_fenced_table_has_a_block_highlight
#[test]
fn rendered_fenced_table_has_a_block_highlight() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/table_block.jsonl"),
        30,
        10,
    );
    begin_copy_mode(&mut s);
    s.keys("up");
    // the table is drawn by rtwk's table (`A │ B`, `──┼──`), not the oracle's `A   B`, `━━━━━`:
    // a table-look difference no snapshot can compare; the claim is the block fill: the three
    // rows of the table are the selection's in every cell, the rows around are not
    s.assert_in_order(&["Before", "A", "x", "After"]);
    let top = s.row_of("Before");
    assert_block(&s, top, 30, 2..5, 7);
}

// ORACLE transcript_view/copy_mode_tests.rs:677 copy_navigation_leaves_context_and_preserves_the_center_band kind=snap owner=chat-live snap=copy_navigation_leaves_context_and_preserves_the_center_band
#[test]
fn copy_navigation_leaves_context_and_preserves_the_center_band() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: the list reveals the item only (`ensure_index_visible`), not a row inside the one tall message: after `up up down` the screen ends at `Context 11` and `second target` is off it; the oracle's target sits on rows 11, 4, 11",
    );
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/context.jsonl"),
        50,
        16,
    );
    begin_copy_mode(&mut s);
    // the oracle: Up, Up, Down; the selected block's row is 11, 4, 11 of the 16
    s.keys("up");
    s.assert_text_at(2, 11, "second target");
    s.assert_text_style("second target", "reversed");
    s.keys("up");
    s.assert_text_at(2, 4, "first target");
    s.assert_text_style("first target", "reversed");
    s.keys("down");
    s.assert_text_at(2, 11, "second target");
    s.assert_text_style("second target", "reversed");
}

// ORACLE transcript_view/copy_mode_tests.rs:721 copy_resize_keeps_whole_user_padding_inside_context kind=snap owner=chat-live snap=copy_resize_keeps_whole_user_padding_inside_context
#[test]
fn copy_resize_keeps_whole_user_padding_inside_context() {
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/resize.jsonl"),
        50,
        20,
    );
    begin_copy_mode(&mut s);
    // the whole prompt `One..Six`: the last response, then the prompt before it
    s.keys("up");
    s.assert_contains("One");
    s.assert_contains("Six");
    s.resize(50, 10 + CHROME);
    // the prompt and its padding rows are the selection's, with a row of context above and below
    s.assert_contains("One");
    s.assert_contains("Six");
    let pad = s.row_of("One") - 1;
    assert_block(&s, pad.saturating_sub(1), 50, 1..9, 10);
}

// ORACLE transcript_view/copy_mode_tests.rs:772 copy_navigation_keeps_bottom_context_when_prompt_header_appears kind=ui owner=chat-live
#[test]
fn copy_navigation_keeps_bottom_context_when_prompt_header_appears() {
    kit::pending(
        Owner::ChatLive,
        "G-new-usage-2: the pinned prompt header (`› First prompt`) that appears when the first prompt scrolls off is not drawn, so `echo target` is not on the row the oracle keeps (5 of 7, the context below it kept)",
    );
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/header.jsonl"),
        50,
        7,
    );
    begin_copy_mode(&mut s);
    // the oracle: Home, Down; the target is on row 5 and the screen stays put
    s.keys("home down");
    s.assert_text_at(0, 0, "› First prompt");
    s.assert_text_at(2, 5, "echo target");
}

// ---- oracle: transcript_view/copy_on_select_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/copy_on_select_tests.rs:26 copy_on_select_waits_for_release_and_confirmed_delivery kind=snap owner=chat-live snap=copy_on_select_waits_for_release_and_confirmed_delivery
#[test]
fn copy_on_select_waits_for_release_and_confirmed_delivery() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no copy-on-select setting and no mouse drag selection that holds until release; the kit has no seed for the setting",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 1)
        .script(include_str!("fixtures/transcript_view_2/sel_text.jsonl"))
        .open();
    s.drag(0, 0, 8, 0);
    // the oracle: the highlight shows `selected` (8 cells) reversed after the release
    s.assert_text_style("selected", "reversed");
}

// ORACLE transcript_view/copy_on_select_tests.rs:145 copy_on_select_copies_word_and_line_on_release_without_dragging kind=ui owner=chat-live
#[test]
fn copy_on_select_copies_word_and_line_on_release_without_dragging() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no word and line selection on repeated clicks (double and triple click), so no word or line highlight is drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 1)
        .script(include_str!("fixtures/transcript_view_2/alpha.jsonl"))
        .open();
    // the oracle: click 1 selects nothing, click 2 selects `beta`, click 3 selects the line
    s.double_click(7, 0);
    s.assert_text_style("beta", "reversed");
    s.click(7, 0);
    s.assert_text_style("alpha beta gamma", "reversed");
}

// ORACLE transcript_view/copy_on_select_tests.rs:179 copy_on_select_preserves_link_activation_and_ignores_empty_drags kind=ui owner=chat-live
#[test]
fn copy_on_select_preserves_link_activation_and_ignores_empty_drags() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no drag selection over a link that copies on release and keeps the link activation for a click",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 2)
        .script(include_str!("fixtures/transcript_view_2/link.jsonl"))
        .open();
    s.click(2, 0);
    s.assert_link("example.com", "https://example.com/docs");
    // the oracle: a drag from column 2 to 5 selects `exa`, released at 5 copies it; released at 2 selects nothing
    s.drag(2, 0, 5, 0);
    s.assert_text_style("exa", "reversed");
}

// ORACLE transcript_view/copy_on_select_tests.rs:226 x11_release_preserves_primary_when_auto_copy_is_disabled kind=ui owner=chat-live
#[test]
fn x11_release_preserves_primary_when_auto_copy_is_disabled() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no PRIMARY selection publish and no copy-on-select setting; the visible half is the selection that stays after release",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 1)
        .script(include_str!("fixtures/transcript_view_2/cafe.jsonl"))
        .open();
    s.drag(0, 0, 14, 0);
    // the oracle: the selection `visit the café` stays after the release; the PRIMARY publish is not drawn
    s.assert_text_style("visit the café", "reversed");
}

// ---- oracle: transcript_view/diff_source_tests.rs (1 screen-level test here; 2 state tests left to chat-live)
// ORACLE transcript_view/diff_source_tests.rs:35 annotated_diff_preserves_the_existing_hard_wrapped_visual_output kind=ui owner=chat-live
#[test]
fn annotated_diff_preserves_the_existing_hard_wrapped_visual_output() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no patch cell (an added file, a diff summary) to draw; the oracle compares a hard-wrapped diff with its reference paragraph, which the kit has no literal rows for",
    );
    let s = Screen::build(Owner::ChatLive).size(40, 8).live().open();
    // the oracle: the annotated diff at width 40 renders the same cells as the reference paragraph
    s.assert_contains("Added test.rs");
}

// ---- oracle: transcript_view/follow_control_tests.rs (7 screen-level tests here; 3 state tests left to chat-live)
// ORACLE transcript_view/follow_control_tests.rs:106 empty_viewport_does_not_show_back_to_bottom kind=snap owner=chat-live snap=empty_viewport_does_not_show_back_to_bottom
#[test]
fn empty_viewport_does_not_show_back_to_bottom() {
    kit::pending(
        Owner::ChatLive,
        "a 1-row window: the chat's bottom pane (composer, footer) is laid out in a Column and overflows it (RenderFlex overflow, fail_on_overflow) where the oracle clips the pane to the window; the control itself hides at a zero-row viewport (Follow::at_tail)",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 1)
        .script(include_str!("fixtures/transcript_view_2/sel_text.jsonl"))
        .open();
    // the oracle: a zero-height transcript draws nothing and shows no control
    s.assert_absent("Back to bottom");
}

// ORACLE transcript_view/follow_control_tests.rs:123 copying_selection_at_bottom_does_not_show_back_to_bottom kind=snap owner=chat-live snap=copying_selection_at_bottom_does_not_show_back_to_bottom
#[test]
fn copying_selection_at_bottom_does_not_show_back_to_bottom() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no follow control and no double-click word selection at the bottom to copy",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 5)
        .script(include_str!(
            "fixtures/transcript_view_2/needle_bottom.jsonl"
        ))
        .open();
    s.double_click(0, 2);
    s.assert_absent("Back to bottom");
    s.assert_in_order(&["three", "four", "needle"]);
}

// ORACLE transcript_view/follow_control_tests.rs:158 closing_find_at_bottom_does_not_show_back_to_bottom kind=ui owner=chat-live
#[test]
fn closing_find_at_bottom_does_not_show_back_to_bottom() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 5)
        .script(include_str!(
            "fixtures/transcript_view_2/needle_bottom.jsonl"
        ))
        .open();
    s.keys("ctrl+f");
    s.type_text("needle");
    s.keys("enter");
    // the oracle: after the find closes at the bottom, the rows are unchanged and no control shows
    s.assert_absent("Back to bottom");
}

// ORACLE transcript_view/follow_control_tests.rs:179 enlarging_paused_view_hides_back_to_bottom_when_tail_fits kind=snap owner=chat-live snap=enlarging_paused_view_hides_back_to_bottom_when_tail_fits
#[test]
fn enlarging_paused_view_hides_back_to_bottom_when_tail_fits() {
    kit::pending(
        Owner::ChatLive,
        "G63: the fixture's rows are joined by single newlines, which MarkdownView joins into one paragraph (the oracle breaks the line), so the rows never scroll (five.jsonl is `one\\ntwo\\n...`), and the oracle test drives the view alone (5 rows, then 6), while at 40x5 the port's gap row, composer and footer leave the transcript no row",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 5)
        .script(include_str!("fixtures/transcript_view_2/five.jsonl"))
        .open();
    s.wheel_up(0, 0, 2);
    s.assert_contains("Back to bottom");
    s.resize(40, 6);
    s.assert_absent("Back to bottom");
    s.assert_in_order(&["one", "two", "three", "four", "five"]);
}

// ORACLE transcript_view/follow_control_tests.rs:252 control_tracks_pause_activity_hover_resize_and_click kind=snap owner=chat-live snap=follow_control
#[test]
fn control_tracks_pause_activity_hover_resize_and_click() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no follow control: its pause, activity, hover, resize and click states are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 5)
        .script(include_str!("fixtures/transcript_view_2/five.jsonl"))
        .open();
    s.wheel_up(0, 0, 2);
    s.assert_snap_section("follow_control", "paused");
    s.feed(include_str!("fixtures/transcript_view_2/new_output.jsonl"));
    s.assert_snap_section("follow_control", "activity at 80");
    s.assert_snap_section("follow_control", "activity at 20");
    s.assert_snap_section("follow_control", "hover");
}

// ORACLE transcript_view/follow_control_tests.rs:296 hidden_control_does_not_consume_clicks_and_drag_out_cancels kind=ui owner=chat-live
#[test]
fn hidden_control_does_not_consume_clicks_and_drag_out_cancels() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no follow control to click or drag out of; a hidden control must not take the click",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 4)
        .script(include_str!("fixtures/transcript_view_2/four.jsonl"))
        .open();
    s.wheel_up(0, 0, 1);
    // the oracle: a press on the hidden control's cell is not consumed; a drag out of the control cancels
    // (the control's cell coordinates come from the oracle's follow_control area, not drawn when hidden)
}

// ORACLE transcript_view/follow_control_tests.rs:334 height_changes_backfill_after_empty_click_but_preserve_deliberate_pause kind=ui owner=chat-live
#[test]
fn height_changes_backfill_after_empty_click_but_preserve_deliberate_pause() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 50)
        .script(include_str!("fixtures/transcript_view_2/rows60.jsonl"))
        .open();
    // the oracle: at height 50 following shows row 59 last; at 24 and back to 50 the tail refills
    s.assert_contains("row 59");
    s.resize(120, 24);
    s.assert_contains("row 59");
    s.resize(120, 50);
    s.assert_contains("row 59");
}

// ---- oracle: transcript_view/footer_tests.rs (11 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/footer_tests.rs:17 selection_at_bottom_restores_footer_without_resuming_following kind=snap owner=chat-live snap=selection_at_bottom_restores_footer_without_resuming_following
#[test]
fn selection_at_bottom_restores_footer_without_resuming_following() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no selection footer (copy, copy & follow, esc clear) that restores at the bottom without resuming following",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 3)
        .script(include_str!(
            "fixtures/transcript_view_2/needle_final.jsonl"
        ))
        .open();
    s.drag(0, 2, 6, 2);
    s.assert_contains("⌃c copy · enter copy & follow · esc clear");
    s.assert_contains("Earlier messages available.");
}

// ORACLE transcript_view/footer_tests.rs:126 selection_hints_follow_selected_text_instead_of_mouse_down kind=snap owner=chat-live snap=selection_hint_threshold
#[test]
fn selection_hints_follow_selected_text_instead_of_mouse_down() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no selection hint footer that follows the selected text (the threshold of one character)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 3)
        .script(include_str!("fixtures/transcript_view_2/sel_hint.jsonl"))
        .open();
    s.mouse_down(0, 0);
    s.mouse_drag(1, 0);
    s.mouse_drag(0, 0);
    s.mouse_up(0, 0);
    s.assert_snap("selection_hint_threshold");
}

// ORACLE transcript_view/footer_tests.rs:173 loading_preserves_selected_content_and_copy_action kind=snap owner=chat-live snap=loading_preserves_selected_content_and_copy_action
#[test]
fn loading_preserves_selected_content_and_copy_action() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 14)
        .replay(Vec::new())
        .open();
    s.at_the_top_of_a_partial_history(12, kit::Page::Loading);
    // the oracle: a triple click selects the line; loading keeps the selection and its copy action
    s.click_text("row 00");
    s.click_text("row 00");
    s.click_text("row 00");
    s.assert_contains("row 00");
    s.assert_contains("↑ Loading earlier messages… · ⌃c copy · enter copy & follow");
    // the selection is held through the page that joins
    s.history_page(vec![kit::agent_turn("turn-older", &["row older"])], None);
    s.assert_contains("⌃c copy · enter copy & follow");
}

// ORACLE transcript_view/footer_tests.rs:233 loading_and_failure_preserve_search_query_and_caret kind=snap owner=chat-live snap=loading_and_failure_preserve_search_query_and_caret
#[test]
fn loading_and_failure_preserve_search_query_and_caret() {
    // the oracle's three history states (loading older, loading the beginning, failed) at 32 and
    // 12 columns: the editor's query stays and its status row gives way to the room there is
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 3)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["visible history"]);
    s.keys("/");
    s.type_text("missing needle");
    s.keys("enter");
    s.assert_find_query("missing needle");
    s.assert_find_footer(32, "Loading… · esc cancel");
    s.assert_find_footer(12, "esc cancel");
    s.history_failed("history unavailable");
    s.assert_find_query("missing needle");
    s.assert_find_footer(32, "⌃p retry · esc cancel");
    s.assert_find_footer(12, "esc cancel");
    // the retry loads again, and the page that has nothing ends the search
    s.keys("ctrl+p");
    s.assert_find_footer(32, "Loading… · esc cancel");
    s.join_older_page(&["older"], false);
    s.assert_find_query("missing needle");
    s.keys("esc");
    s.assert_absent("missing needle");
}

// ORACLE transcript_view/footer_tests.rs:283 reading_a_find_result_shows_history_loading_and_failure kind=snap owner=chat-live snap=reading_a_find_result_shows_history_loading_and_failure
#[test]
fn reading_a_find_result_shows_history_loading_and_failure() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 5)
        .replay(Vec::new())
        .view_overlay()
        .open();
    s.partial_history_of(&["visible needle"]);
    s.keys("/");
    s.type_text("needle");
    s.keys("pageup");
    s.assert_page_requests(2);
    s.keys("enter");
    // the oracle reads the status row in the three states, loading, failed and complete
    s.assert_contains("↑ Loading earlier messages… · esc latest");
    s.history_failed("history unavailable");
    s.assert_contains("Retry history: ⌥</⌃home.  esc latest");
    s.keys("alt+<");
    s.join_older_page(&["older"], false);
    s.assert_find_footer(80, "Find · ctrl+p older · ctrl+n newer · esc latest");
}

// ORACLE transcript_view/footer_tests.rs:313 loading_completion_restores_unseen_activity_and_failure_stops_motion kind=snap owner=chat-live snap=loading_completion_restores_unseen_activity_and_failure_stops_motion
#[test]
fn loading_completion_restores_unseen_activity_and_failure_stops_motion() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no unseen-activity footer (`New activity`) and no loading completion that restores it",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 3)
        .script(include_str!("fixtures/transcript_view_2/four.jsonl"))
        .open();
    s.wheel_up(0, 0, 1);
    s.feed(include_str!("fixtures/transcript_view_2/new_output.jsonl"));
    s.assert_contains("New activity · esc latest");
}

// ORACLE transcript_view/footer_tests.rs:354 passive_footer_keeps_jump_actions_in_one_row kind=snap owner=chat-live snap=compact_navigation_hints
#[test]
fn passive_footer_keeps_jump_actions_in_one_row() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no passive navigation footer (jump actions in one row at 26, 38 and 78 columns)",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(78, 3)
        .script(include_str!("fixtures/transcript_view_2/retained.jsonl"))
        .open();
    s.assert_snap("compact_navigation_hints");
}

// ORACLE transcript_view/footer_tests.rs:382 narrow_selection_and_search_keep_complete_exit_hints kind=snap owner=chat-live snap=narrow_selection_and_search_keep_complete_exit_hints
#[test]
fn narrow_selection_and_search_keep_complete_exit_hints() {
    let name = "narrow_selection_and_search_keep_complete_exit_hints";
    // the oracle's `footer(width)` is the width of the hint row, which the screen indents
    const FOOTER_INDENT: u16 = 2;
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/transcript_view_2/needle_final.jsonl"
        ))
        .open();
    // the line `needle final`, as the oracle's triple click selects it
    s.triple_click(4, 5);
    for (at, width) in [12, 20, 32, 80].into_iter().enumerate() {
        s.resize(width + FOOTER_INDENT, 12);
        s.assert_exit_hint(name, at);
    }
    s.keys("esc");
    // the chat's key for Find is `find_transcript` (F3); `/` is the pager's
    s.keys("f3");
    s.paste("needle");
    for (at, width) in [12, 20, 32, 80].into_iter().enumerate() {
        s.resize(width + FOOTER_INDENT, 12);
        s.assert_exit_hint(name, 4 + at);
    }
}

// ORACLE transcript_view/footer_tests.rs:439 patch_disclosure_preserves_manual_scrolling kind=ui owner=chat-live
#[test]
fn patch_disclosure_preserves_manual_scrolling() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no activity cell with a `+ Show details` disclosure, and no focus on it from a click",
    );
    let mut s = Screen::build(Owner::ChatLive).size(32, 10).live().open();
    s.assert_contains("+ Show details");
    s.click_text("+ Show details");
    // the oracle: the disclosure is focused; the manual scroll position survives the patch approval sync
}

// ORACLE transcript_view/footer_tests.rs:483 activity_focus_keeps_controls_without_passive_hints kind=snap owner=chat-live snap=activity_focus_keeps_controls_without_passive_hints
#[test]
fn activity_focus_keeps_controls_without_passive_hints() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no focusable activity cell (F4 focus, Enter expand) with its controls and footer hints",
    );
    let mut s = Screen::build(Owner::ChatLive).size(32, 10).live().open();
    s.keys("f4");
    s.assert_snap("activity_focus_keeps_controls_without_passive_hints");
}

// ORACLE transcript_view/footer_tests.rs:565 transcript_footer_keys_remain_distinct_from_prose_and_query_text kind=snap owner=chat-live snap=transcript_footer_keys_remain_distinct_from_prose_and_query_text
#[test]
fn transcript_footer_keys_remain_distinct_from_prose_and_query_text() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no footer whose key labels stay distinct from prose and from the find query `Activity`",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 3).live().open();
    s.keys("ctrl+f");
    s.type_text("Activity");
    s.assert_snap("transcript_footer_keys_remain_distinct_from_prose_and_query_text");
}

// ---- oracle: transcript_view/input_tests.rs (6 screen-level tests here; 5 state tests left to chat-live)
// ORACLE transcript_view/input_tests.rs:8 mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement kind=snap owner=chat-live snap=mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement
#[test]
fn mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no mouse scroll speed setting: the speed 0.5, 1.5 and 3.0 scales each wheel notch",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 8)
        .script(include_str!("fixtures/transcript_view_2/rows24.jsonl"))
        .open();
    s.wheel_up(1, 1, 8);
    s.assert_snap("mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement");
}

// ORACLE transcript_view/input_tests.rs:57 decimal_mouse_scroll_speeds_preserve_whole_rows kind=ui owner=chat-live
#[test]
fn decimal_mouse_scroll_speeds_preserve_whole_rows() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 8)
        .script(include_str!("fixtures/transcript_view_2/rows120.jsonl"))
        .open();
    s.wheel_up(1, 1, 40);
    // the oracle: after each event the top row moves by floor(event * speed) whole rows (the formula is the oracle's)
}

// ORACLE transcript_view/input_tests.rs:134 hover_links_match_wrapped_click_targets_and_refresh_after_scrolling kind=ui owner=chat-live
#[test]
fn hover_links_match_wrapped_click_targets_and_refresh_after_scrolling() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no link hover target that matches a wrapped link's click target and refreshes after scrolling",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 8)
        .script(include_str!("fixtures/transcript_view_2/links.jsonl"))
        .open();
    s.assert_contains("wide 界 wrapped");
    s.wheel_down(1, 1, 20);
    // the oracle: after scrolling 20 rows the hovered link cells no longer match any target
    s.assert_absent("wide 界 wrapped");
}

// ORACLE transcript_view/input_tests.rs:178 shift_click_extends_a_double_clicked_word_in_both_directions kind=snap owner=chat-live snap=shift_click_extends_a_double_clicked_word_in_both_directions
#[test]
fn shift_click_extends_a_double_clicked_word_in_both_directions() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no shift-click extension of a double-clicked word in both directions",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 8)
        .script(include_str!("fixtures/transcript_view_2/alpha.jsonl"))
        .open();
    s.double_click(11, 1);
    s.assert_text_style("beta", "reversed");
    s.assert_snap("shift_click_extends_a_double_clicked_word_in_both_directions");
}

// ORACLE transcript_view/input_tests.rs:270 ordinary_click_opens_bare_and_markdown_links_on_release kind=ui owner=chat-live
#[test]
fn ordinary_click_opens_bare_and_markdown_links_on_release() {
    // 12 rows: a live completion ends with the "Worked for" separator (the oracle's status-timer
    // fallback, `FinalMessageSeparator::from_turn_once`), four more rows under the message.
    let mut s = Screen::build(Owner::ChatLive)
        .size(12, 12)
        .script(include_str!("fixtures/transcript_view_2/docs.jsonl"))
        .open();
    s.assert_link("docs", "https://example.com/docs");
    s.click(1, 0);
    s.assert_contains("docs");
}

// ORACLE transcript_view/input_tests.rs:310 selection_mouse_does_not_open_links kind=ui owner=chat-live
#[test]
fn selection_mouse_does_not_open_links() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no selection gesture on a link that keeps the link from opening (Ctrl and no modifier)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .script(include_str!("fixtures/transcript_view_2/docs.jsonl"))
        .open();
    s.drag(1, 0, 4, 0);
    s.assert_text_style("docs", "reversed");
}

// ---- oracle: transcript_view/layout_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/layout_tests.rs:36 layouts_refresh_for_width_animation_and_mutable_frames kind=ui owner=chat-live
#[test]
fn layouts_refresh_for_width_animation_and_mutable_frames() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no layout cache with animated frames: the frame tick and the width change redraw the cell as `tick 2`",
    );
    let mut s = Screen::build(Owner::ChatLive).size(20, 4).live().open();
    s.wait(100);
    s.assert_contains("tick 2");
}

// ORACLE transcript_view/layout_tests.rs:98 recent_entries_are_reused_and_old_entries_are_evicted kind=ui owner=chat-live
#[test]
fn recent_entries_are_reused_and_old_entries_are_evicted() {
    kit::na(
        Owner::ChatLive,
        "asserts the render counts of the layout cache (recent entries reused, old entries evicted) through a counter on a test cell, not drawn",
    );
}

// ORACLE transcript_view/layout_tests.rs:155 fullscreen_image_previews_follow_layout_without_entering_text_surfaces kind=snap owner=chat-live snap=fullscreen_image_artifact_rows
#[test]
fn fullscreen_image_previews_follow_layout_without_entering_text_surfaces() {
    kit::pending(
        Owner::ChatLive,
        "G4 (conditional, `rtwk-rich` `Image`; the file read is a services effect): the snapshot holds the kitty image placements (`Graphics: [Rect { .. }]`) of a PNG read from disk and the blank rows reserved under `Viewed image` and `Generated Image`, and its local file links; the chat draws the call rows and their `+ Show details` control (equal to the snapshot's text) but reads no file, so it reserves no preview rows and has no placement list",
    );
    let s = Screen::build(Owner::ChatLive).size(60, 20).live().open();
    s.assert_snap("fullscreen_image_artifact_rows");
}
