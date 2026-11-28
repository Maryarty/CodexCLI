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
        "the owned chat draws the pinned header (`pages::transcript_prompt_header`), but not in this frame: after copy mode's seven Up and five Down the first list item in view is the prompt's own (a debug read of `visible_prompt` gives row 1 of three, itself a prompt row; the screen shows `Worked for <1s` at its top, none of the prompt's text), which the port reads as a visible prompt, so the header yields; the oracle pins `› User text > user quote ``` user code ```` over `Latest plan` there. The header is decided by the first visible item, not by the first visible text row of the prompt",
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
        "G62: the pinned rtwk has a Mermaid parser but no diagram widget. The mounted code fallback and exact source copy are covered by transcript_kit::the_mermaid_code_fallback_copies_the_complete_fenced_source.",
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
    for pending in [false, true] {
        let mut s = chat(
            include_str!("fixtures/transcript_view_2/completion.jsonl"),
            80,
            8,
        );
        begin_copy_mode(&mut s);
        s.keys("enter");
        s.assert_copies_asked(&["**bold**"]);
        if pending {
            s.answer_copy_pending(7);
            s.assert_contains("Copying 8 chars");
        }
        s.answer_copy(Ok(()));
        s.assert_contains("Copied 8 chars to host clipboard");
        s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
        s.assert_contains("new output");
        s.assert_absent("Back to bottom");
        s.assert_copies_asked(&["**bold**"]);
    }
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
    const BODY: &str = "| Name | Value |\r\n|---|---|\r\n| alpha | beta |  \r\n";
    const MESSAGE: &str = "Intro\n\n```md\n| Name | Value |\n|---|---|\n| alpha | beta |  \n```\n\n```sh\necho done\n```\n";
    let mut s = chat(
        include_str!("fixtures/transcript_view_2/disabled_table.jsonl"),
        50,
        15,
    );
    s.rendering(kit::Renderers {
        tables: false,
        ..Default::default()
    });
    s.assert_in_order(&[
        "| Name | Value |",
        "|---|---|",
        "| alpha | beta |",
        "echo done",
    ]);
    let mut expected = Vec::new();
    for (keys, payload) in [
        ("enter", "echo done\r\n"),
        ("up enter", BODY),
        ("up up enter", MESSAGE),
    ] {
        begin_copy_mode(&mut s);
        s.keys(keys);
        expected.push(payload);
        s.assert_copies_asked(&expected);
        s.answer_copy(Ok(()));
    }
}

#[test]
fn a_loading_page_does_not_change_the_code_block_chosen_for_copy() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(64, 16)
        .owned_transcript()
        .replay(Vec::new())
        .open();
    let wanted = kit::agent_turn("recent", &["Latest reply\n\n```sh\nselected command\n```"]);
    let middle = kit::turns_of("history row", 0..15);
    s.history_page([vec![wanted], middle].concat(), Some("older-page"));
    s.keys("ctrl+home");
    s.assert_page_requests(2);

    begin_copy_mode(&mut s);
    s.keys("up");
    s.assert_text_style("selected command", "reversed");
    s.history_page(
        vec![kit::agent_turn(
            "older",
            &["Older reply\n\n```sh\nunrelated command\n```"],
        )],
        None,
    );
    s.resize(44, 16);
    s.keys("enter");
    s.assert_copies_asked(&["selected command\n"]);
    s.answer_copy(Ok(()));
}

#[test]
fn canonical_replacement_does_not_change_the_frozen_copy_source() {
    const ORIGINAL: &str = "A response\n\n```sh\nold command\n```\n\n```sh\nother command\n```\n";
    const REPLACEMENT: &str = "A response\n\n```sh\nother command\n```\n";
    let update = |source: &str| {
        serde_json::json!({"step":"notify","message":{"method":"item/completed","params":{
            "threadId":"thread-1","turnId":"turn-1","completedAtMs":1,
            "item":{"type":"agentMessage","id":"same","text":source,"phase":null}
        }}})
        .to_string()
    };
    for (choose_block, expected) in [(true, "old command\n"), (false, ORIGINAL.trim_end())] {
        let mut s = Screen::build(Owner::ChatLive)
            .owned_transcript()
            .size(65, 18)
            .live()
            .open();
        s.feed(&update(ORIGINAL));
        begin_copy_mode(&mut s);
        if choose_block {
            s.keys("up up");
            s.assert_contains("Copy code block");
        } else {
            s.assert_contains("Copy whole response");
        }
        s.assert_text_style("old command", "reversed");

        s.feed(&update(REPLACEMENT));
        s.resize(44, 18);
        s.assert_absent("old command");
        s.assert_text_style("other command", "!reversed");
        s.keys("enter");
        s.assert_copies_asked(&[expected]);
        s.answer_copy(Ok(()));
    }
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
        "the owned chat draws the pinned header (`pages::transcript_prompt_header`; row 0 is `› First prompt` as the oracle's), but copy mode's reveal of the target moves the list by the least scroll under the reserved row: `echo target` ends on row 3 (the frame), where the oracle keeps the screen put with the context below it and the target on row 5 (5 of 7)",
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
    for (enabled, clear) in [(false, false), (true, false), (true, true)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(20, 5)
            .script(include_str!("fixtures/transcript_view_2/sel_text.jsonl"))
            .view_only()
            .open();
        s.selection_copy_policy(enabled, clear);
        let row = u16::try_from(s.row_of("selected text")).unwrap();
        if !enabled {
            s.drag(2, row, 9, row);
            s.assert_text_style("selected", "reversed");
            s.assert_copies_asked(&[]);
            continue;
        }
        let mut copies = Vec::new();
        for delivery in [Ok(false), Err("clipboard unavailable"), Ok(true)] {
            s.mouse_down(2, row);
            s.mouse_drag(9, row);
            s.assert_copies_asked(&copies);
            s.mouse_up(9, row);
            copies.push("selected");
            s.assert_copies_asked(&copies);
            s.assert_text_style("selected", "reversed");
            match delivery {
                Ok(true) => s.answer_copy(Ok(())),
                Ok(false) => s.answer_copy_unconfirmed(),
                Err(error) => s.answer_copy(Err(error)),
            }
            let cleared = clear && delivery == Ok(true);
            s.assert_text_style("selected", if cleared { "!reversed" } else { "reversed" });
            s.wait(350);
        }
    }
}

// ORACLE transcript_view/copy_on_select_tests.rs:145 copy_on_select_copies_word_and_line_on_release_without_dragging kind=ui owner=chat-live
#[test]
fn copy_on_select_copies_word_and_line_on_release_without_dragging() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 5)
        .script(include_str!("fixtures/transcript_view_2/alpha.jsonl"))
        .view_only()
        .open();
    s.selection_copy_policy(true, false);
    let row = u16::try_from(s.row_of("alpha beta gamma")).unwrap();
    s.click(9, row);
    s.assert_copies_asked(&[]);
    s.mouse_down(9, row);
    s.assert_copies_asked(&[]);
    s.mouse_up(9, row);
    s.assert_text_style("beta", "reversed");
    s.assert_copies_asked(&["beta"]);
    s.click(9, row);
    s.assert_text_style("alpha beta gamma", "reversed");
    s.assert_copies_asked(&["beta", "alpha beta gamma"]);
}

// ORACLE transcript_view/copy_on_select_tests.rs:179 copy_on_select_preserves_link_activation_and_ignores_empty_drags kind=ui owner=chat-live
#[test]
fn copy_on_select_preserves_link_activation_and_ignores_empty_drags() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(36, 6)
        .script(include_str!("fixtures/transcript_view_2/link.jsonl"))
        .view_only()
        .open();
    s.selection_copy_policy(true, false);
    let row = u16::try_from(s.row_of("example.com")).unwrap();
    s.mouse_down(2, row);
    s.assert_opened_urls(&[]);
    s.mouse_up(2, row);
    s.assert_opened_urls(&["https://example.com/docs"]);
    s.wait(350);
    s.drag(2, row, 4, row);
    s.assert_text_style("exa", "reversed");
    s.assert_copies_asked(&["exa"]);
    s.wait(350);
    s.mouse_down(2, row);
    s.mouse_drag(4, row);
    s.mouse_drag(2, row);
    s.mouse_up(2, row);
    s.assert_text_style("exa", "!reversed");
    s.assert_copies_asked(&["exa"]);
    s.assert_opened_urls(&["https://example.com/docs"]);
}

// ORACLE transcript_view/copy_on_select_tests.rs:226 x11_release_preserves_primary_when_auto_copy_is_disabled kind=ui owner=chat-live
#[test]
fn x11_release_preserves_primary_when_auto_copy_is_disabled() {
    let text = "visit the café";
    for copy_on_select in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(20, 5)
            .script(include_str!("fixtures/transcript_view_2/cafe.jsonl"))
            .view_only()
            .open();
        s.selection_copy_policy(copy_on_select, false);
        s.primary_selection();
        let row = u16::try_from(s.row_of(text)).unwrap();
        s.mouse_down(2, row);
        s.mouse_drag(16, row);
        s.assert_copies_asked(&[]);
        s.mouse_up(16, row);
        s.assert_text_style(text, "reversed");
        s.assert_copies_asked(&[text]);
        s.assert_primary_copies_asked(&[(text, copy_on_select)]);
        s.mouse_up(16, row);
        s.assert_copies_asked(&[text]);
    }
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_2/rows60.jsonl"))
        .open();
    s.keys("pageup");
    s.assert_contains("Back to bottom");
    s.resize(40, 1);
    s.wheel_up(0, 0, 2);
    s.assert_absent("Back to bottom");
    s.assert_absent("latest");
}

// ORACLE transcript_view/follow_control_tests.rs:123 copying_selection_at_bottom_does_not_show_back_to_bottom kind=snap owner=chat-live snap=copying_selection_at_bottom_does_not_show_back_to_bottom
#[test]
fn copying_selection_at_bottom_does_not_show_back_to_bottom() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 14)
        .script(include_str!(
            "fixtures/transcript_view_2/needle_bottom.jsonl"
        ))
        .open();
    s.selection_copy_policy(false, true);
    s.assert_in_order(&["three", "four", "needle"]);
    s.double_click_text("needle");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["needle"]);
    s.answer_copy(Ok(()));
    s.assert_contains("Copied 6 chars");
    s.assert_text_style("needle", "!reversed");
    s.assert_absent("Back to bottom");
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(include_str!("fixtures/transcript_view_2/five.jsonl"))
        .open();
    s.keys("pageup");
    s.assert_contains("Back to bottom");
    s.resize(80, 17);
    s.assert_absent("Back to bottom");
    s.assert_in_order(&["one", "two", "three", "four", "five"]);
}

// ORACLE transcript_view/follow_control_tests.rs:252 control_tracks_pause_activity_hover_resize_and_click kind=snap owner=chat-live snap=follow_control
#[test]
fn control_tracks_pause_activity_hover_resize_and_click() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_2/rows60.jsonl"))
        .open();
    s.assert_absent("Back to bottom");
    s.keys("pageup");
    s.assert_contains("Back to bottom");
    s.assert_absent("New activity");
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.assert_contains("New activity · ↓ Back to bottom");
    let y = u16::try_from(s.row_of("Back to bottom")).expect("control row");
    s.mouse_drag(40, y);
    s.assert_style(40, usize::from(y), "reversed bold");
    s.resize(20, 12);
    s.assert_contains("↓ Bottom");
    s.click_text("↓ Bottom");
    s.assert_contains("new output");
    s.assert_absent("↓ Bottom");
}

// ORACLE transcript_view/follow_control_tests.rs:296 hidden_control_does_not_consume_clicks_and_drag_out_cancels kind=ui owner=chat-live
#[test]
fn hidden_control_does_not_consume_clicks_and_drag_out_cancels() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_2/rows60.jsonl"))
        .open();
    s.keys("pageup");
    let y = u16::try_from(s.row_of("Back to bottom")).expect("control row");
    s.mouse_down(40, y).mouse_up(40, 2);
    s.assert_contains("Back to bottom");
    s.keys("esc");
    s.assert_absent("Back to bottom");
    let tail = s.shot();
    s.click(40, y);
    s.assert_same_rows(&tail);
    s.type_text("composer still focused");
    s.assert_composer("composer still focused");
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
    for searching in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 14)
            .script(include_str!("fixtures/transcript_view_2/sel_hint.jsonl"))
            .open();
        s.selection_copy_policy(false, false);
        if searching {
            s.keys("f3");
            s.assert_contains("Find:");
            s.assert_contains("Type to find");
        }
        let row = s.row_of("é selection");
        let y = u16::try_from(row).expect("selection row");
        s.mouse_down(2, y);
        s.assert_absent("⌃c copy");
        s.assert_style(2, row, "!reversed");
        s.mouse_drag(3, y);
        s.assert_style(2, row, "reversed");
        s.assert_contains("⌃c copy");
        if searching {
            s.assert_contains("Find:");
            s.assert_absent("Type to find");
        }
        s.mouse_drag(2, y);
        s.mouse_up(2, y);
        s.assert_absent("⌃c copy");
        s.assert_copies_asked(&[]);
        if searching {
            s.assert_contains("Find:");
            s.assert_contains("Type to find");
        }
    }
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(94, 15)
        .replay(Vec::new())
        .open();
    s.at_the_top_of_a_partial_history(15, kit::Page::Loading);
    s.live_tail("new output");
    s.assert_contains("Loading earlier messages");
    s.assert_contains("New activity");
    s.assert_absent("new output");

    s.history_failed("earlier history unavailable");
    s.assert_absent("Loading earlier messages");
    s.assert_contains("Retry history");
    s.assert_contains("New activity");
    let retry = s.row_of("Retry history");
    let failed = s.shot();
    s.wait(500);
    s.assert_same_rows_in(&failed, retry..retry + 1);

    s.keys("ctrl+home");
    s.assert_page_requests(3);
    s.history_page(vec![kit::agent_turn("older-page", &["older result"])], None);
    s.assert_absent("Loading earlier messages");
    s.assert_absent("Retry history");
    s.assert_contains("New activity");
    s.keys("esc");
    s.assert_contains("new output");
    s.assert_absent("New activity");
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
    const PATCH: &str = include_str!("fixtures/transcript_view_2/patch_disclosure.jsonl");
    const APPROVAL: &str =
        include_str!("fixtures/transcript_view_2/patch_disclosure_approval.jsonl");
    const RESOLVED: &str =
        include_str!("fixtures/transcript_view_2/patch_disclosure_resolved.jsonl");
    const PROMPT: &str = "Would you like to make the following edits?";
    for scroll_before_click in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(48, 29).live().open();
        s.mouse_scroll_speed(Some(1.0));
        s.feed(PATCH);
        s.assert_in_order(&["approval-patch.txt", "+ Show details", "tail output 10"]);
        if scroll_before_click {
            s.wheel_up(1, 1, 1);
            s.assert_contains("Back to bottom");
        }
        s.feed(APPROVAL);
        s.assert_in_order(&["approval-patch.txt", "+ Show details", PROMPT]);
        s.click_text("+ Show details");
        // The approval takes the bottom rows, so the expanded control can sit just below
        // the manually held viewport. Scroll one row to reach it in the ordinary list.
        if s.grid().locate("Hide details").is_none() {
            s.wheel_down(1, 1, 1);
        }
        s.assert_in_order(&["approval-patch.txt", "patch line five", "Hide details"]);
        s.assert_contains(PROMPT);
        s.assert_no_resolved_requests();
        if !scroll_before_click {
            s.wheel_up(1, 1, 1);
        }
        let before = s.shot();
        s.feed(RESOLVED);
        s.assert_absent(PROMPT);
        s.assert_same_rows_in(&before, 0..3);
        s.assert_contains("Back to bottom");
        s.assert_no_resolved_requests();
    }
}

// ORACLE transcript_view/footer_tests.rs:483 activity_focus_keeps_controls_without_passive_hints kind=snap owner=chat-live snap=activity_focus_keeps_controls_without_passive_hints
#[test]
fn activity_focus_keeps_controls_without_passive_hints() {
    const LIVE: &str = include_str!("fixtures/transcript_view_2/activity_disclosure_live.jsonl");
    const COMMIT: &str =
        include_str!("fixtures/transcript_view_2/activity_disclosure_commit.jsonl");
    for bare in [false, true] {
        let open = Screen::build(Owner::ChatLive).size(82, 18).live();
        let mut s = if bare { open.view_compact() } else { open }.open();
        s.feed(LIVE);
        s.assert_contains("+ Show details");
        s.assert_absent("hidden detail");
        s.assert_absent("enter expand");
        if bare {
            s.assert_absent("for shortcuts");
        } else {
            s.assert_contains("for shortcuts");
        }
        s.assert_text_style("Running", "!underlined");
        s.keys("f4");
        s.assert_text_style("Running", "underlined");
        s.assert_contains("↑ previous · ↓ next · enter expand · esc back");
        s.assert_absent("for shortcuts");
        let indent = if bare { 0 } else { 2 };
        s.resize(32 + indent, 18);
        s.assert_contains("↓ next · enter expand · esc back");
        s.assert_absent("↑ previous");
        s.resize(20 + indent, 18);
        s.assert_contains("esc back");
        s.assert_absent("enter expand");
        s.resize(82, 18);
        s.keys("enter");
        s.assert_contains("echo hidden detail");
        s.assert_contains("Hide details");
        s.assert_contains("enter collapse");
        s.feed(COMMIT);
        s.assert_text_style("Ran", "underlined");
        s.assert_contains("echo hidden detail");
        s.assert_contains("enter collapse");
        s.keys("enter");
        s.assert_contains("+ Show details");
        s.assert_absent("hidden detail");
        s.keys("esc");
        s.assert_text_style("Ran", "!underlined");
        s.assert_absent("esc back");
    }
    let bindings: [&[(&str, &str)]; 3] = [
        &[("global.focus_activity", "ctrl-x t")],
        &[
            ("global.focus_activity", "ctrl-x t"),
            ("global.focus_activity", "f4"),
        ],
        &[("global.focus_activity", "")],
    ];
    for keymap in bindings {
        let mut s = Screen::build(Owner::ChatLive)
            .size(82, 18)
            .live()
            .keymap(keymap)
            .open();
        s.feed(LIVE);
        s.feed(COMMIT);
        s.assert_contains("+ Show details");
        s.assert_text_style("Ran", "!underlined");
        s.assert_has_row("  ← for agents · ? for shortcuts");
        s.assert_absent("enter expand");
        s.assert_absent("esc back");
    }
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
fn wheel_reader(overlay: bool, speed: Option<f64>) -> Screen {
    let open = Screen::build(Owner::ChatLive).size(60, 16).live();
    let mut s = if overlay { open.view_overlay() } else { open }.open();
    if speed.is_some() {
        s.mouse_scroll_speed(speed);
    }
    let rows = (1..=120)
        .map(|row| format!("row {row:03}"))
        .collect::<Vec<_>>()
        .join("  \n");
    s.live_tail(&rows);
    s.keys("pageup pageup");
    s
}

fn first_wheel_row(s: &Screen) -> i32 {
    s.rows()
        .iter()
        .find_map(|row| {
            row.trim()
                .trim_start_matches('•')
                .trim()
                .strip_prefix("row ")?
                .parse()
                .ok()
        })
        .unwrap_or_else(|| panic!("no numbered transcript row:\n{}", s.text()))
}

// ORACLE transcript_view/input_tests.rs:8 mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement kind=snap owner=chat-live snap=mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement
#[test]
fn mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement() {
    for overlay in [false, true] {
        for (speed, deltas) in [
            (None, &[-3, 3][..]),
            (Some(0.5), &[0, -1, 0, 0, 1]),
            (Some(1.5), &[-1, -2, 1, 2]),
            (Some(3.0), &[-3, 3]),
        ] {
            let mut s = wheel_reader(overlay, speed);
            let mut expected = first_wheel_row(&s);
            for (i, &delta) in deltas.iter().enumerate() {
                if i < deltas.len().div_ceil(2) {
                    s.wheel_up(5, 5, 1);
                } else {
                    s.wheel_down(5, 5, 1);
                }
                expected += delta;
                kit::assert_same(
                    first_wheel_row(&s),
                    expected,
                    &format!("overlay {overlay}, speed {speed:?}, event {i}"),
                );
            }
        }
    }
}

// ORACLE transcript_view/input_tests.rs:57 decimal_mouse_scroll_speeds_preserve_whole_rows kind=ui owner=chat-live
#[test]
fn decimal_mouse_scroll_speeds_preserve_whole_rows() {
    for (speed, tenths) in [(0.1, 1), (0.3, 3)] {
        let mut s = wheel_reader(true, Some(speed));
        let start = first_wheel_row(&s);
        for (up, direction) in [(true, -1), (false, 1)] {
            let origin = first_wheel_row(&s);
            for event in 1..=100 {
                if up {
                    s.wheel_up(5, 5, 1);
                } else {
                    s.wheel_down(5, 5, 1);
                }
                kit::assert_same(
                    first_wheel_row(&s),
                    origin + direction * (event * tenths / 10),
                    &format!("speed {speed}, direction {direction}, event {event}"),
                );
            }
        }
        kit::assert_same(first_wheel_row(&s), start, "back to the starting row");
    }
}

// ORACLE transcript_view/input_tests.rs:134 hover_links_match_wrapped_click_targets_and_refresh_after_scrolling kind=ui owner=chat-live
#[test]
fn hover_links_match_wrapped_click_targets_and_refresh_after_scrolling() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 8)
        .script(include_str!("fixtures/transcript_view_2/links.jsonl"))
        .view_only()
        .open();
    s.keys("ctrl+home");
    let url = "https://example.com";
    let cells = s.wrapped_link_cells(url);
    for &(x, y) in &cells {
        s.ctrl_click(x, y);
    }
    let expected = vec![url; cells.len()];
    s.assert_opened_urls(&expected);
    s.wheel_down(1, 1, 20);
    for (x, y) in cells {
        s.ctrl_click(x, y);
    }
    s.assert_opened_urls(&expected);
}

// ORACLE transcript_view/input_tests.rs:178 shift_click_extends_a_double_clicked_word_in_both_directions kind=snap owner=chat-live snap=shift_click_extends_a_double_clicked_word_in_both_directions
#[test]
fn shift_click_extends_a_double_clicked_word_in_both_directions() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-point: the pinned SelectionArea does not expose a pointer cell as a SelectionPoint or recognize Shift-click to extend the existing word range",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 8)
        .script(include_str!("fixtures/transcript_view_2/alpha.jsonl"))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    let row = u16::try_from(s.row_of("alpha beta gamma")).unwrap();
    s.double_click_text("beta");
    s.assert_text_style("beta", "reversed");
    s.shift_click(3, row);
    s.assert_text_style("alpha beta", "reversed");
    s.shift_click(15, row);
    s.assert_text_style("beta gamma", "reversed");
    s.assert_snap("shift_click_extends_a_double_clicked_word_in_both_directions");
}

// ORACLE transcript_view/input_tests.rs:270 ordinary_click_opens_bare_and_markdown_links_on_release kind=ui owner=chat-live
#[test]
fn ordinary_click_opens_bare_and_markdown_links_on_release() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 14)
        .script(include_str!(
            "fixtures/transcript_view_2/clickable_web.jsonl"
        ))
        .view_only()
        .open();
    let mut opened = Vec::new();
    for (label, url) in [
        ("https://example.org/plain", "https://example.org/plain"),
        ("docs", "https://example.com/docs"),
    ] {
        s.assert_link(label, url);
        let y = u16::try_from(s.row_of(label)).unwrap();
        s.mouse_down(3, y);
        s.assert_opened_urls(&opened);
        s.mouse_up(3, y);
        opened.push(url);
        s.assert_opened_urls(&opened);
    }
}

// ORACLE transcript_view/input_tests.rs:310 selection_mouse_does_not_open_links kind=ui owner=chat-live
#[test]
fn selection_mouse_does_not_open_links() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .script(include_str!("fixtures/transcript_view_2/docs.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.assert_contains("enter/q close");
    let row = u16::try_from(s.row_of("docs")).unwrap();
    s.click(3, row);
    s.ctrl_click(3, row);
    s.assert_opened_urls(&[]);
    s.assert_contains("enter/q close");
    s.keys("q");
    s.wait(350);
    s.click(3, u16::try_from(s.row_of("docs")).unwrap());
    s.assert_opened_urls(&["https://example.com/docs"]);
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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artifact.png");
    image::RgbaImage::from_pixel(480, 160, image::Rgba([255, 0, 0, 255]))
        .save(&path)
        .unwrap();
    let link = url::Url::from_file_path(dunce::canonicalize(&path).unwrap()).unwrap();
    let notify = |item| {
        serde_json::json!({"step": "notify", "message": {"method": "item/completed", "params": {
            "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0, "item": item,
        }}})
    };
    let script = format!(
        "{}\n{}\n",
        notify(serde_json::json!({"type":"imageView","id":"view-1","path":path})),
        notify(serde_json::json!({
            "type":"imageGeneration","id":"img-1","status":"completed",
            "revisedPrompt":"A red rectangle","failure":null,"savedPath":path,
            "generationId":null,"result":"iVBORw0KGgo=",
        })),
    );
    for graphics in [false, true] {
        let builder = Screen::build(Owner::ChatLive)
            .size(140, 40)
            .live()
            .owned_transcript();
        let mut s = if graphics { builder.kitty() } else { builder }.open();
        s.local_image(&path);
        s.feed(&script);
        s.assert_link("artifact.png", link.as_str());
        s.assert_no_link("Viewed image");
        s.assert_image_rows(if graphics { &["Saved to:"] } else { &[] }, 140);

        s.click_text("+ Show details");
        s.assert_link(path.to_str().unwrap(), link.as_str());
        s.assert_image_rows(
            if graphics {
                &["− Hide details", "Saved to:"]
            } else {
                &[]
            },
            140,
        );

        s.selection_copy_policy(true, false);
        let (x, y) = s.find("Viewed image");
        let (end_x, end_y) = s.find("A red rectangle");
        s.drag(x, y, end_x + 14, end_y);
        s.assert_copy_asked(&format!(
            "Viewed image {}\n\n• Generated Image:\n  └ A red rectangle",
            path.display()
        ));

        s.resize(8, 40);
        s.assert_image_rows(&[], 8);
        s.resize(100, 40);
        s.assert_image_rows(
            if graphics {
                &["− Hide details", "Saved to:"]
            } else {
                &[]
            },
            100,
        );
        s.click_text("− Hide details");
        s.assert_image_rows(if graphics { &["Saved to:"] } else { &[] }, 100);
        if graphics {
            s.keys("alt+r");
            s.assert_image_rows(&[], 100);
            kit::remove_artifact_file(&path);
            s.keys("alt+r");
            s.click_text("+ Show details");
            s.assert_contains(path.to_str().unwrap());
            s.assert_no_link(path.to_str().unwrap());
            s.assert_contains("A red rectangle");
            s.assert_image_rows(&[], 100);
        }
    }
}
