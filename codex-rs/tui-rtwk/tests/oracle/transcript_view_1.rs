//! Shard `transcript_view_1` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The first cell of `text` on the screen, as (column, row); (0, 0) when it is not drawn.
fn at(s: &Screen, text: &str) -> (u16, u16) {
    s.rows()
        .iter()
        .enumerate()
        .find_map(|(y, row)| row.find(text).map(|x| (x as u16, y as u16)))
        .unwrap_or((0, 0))
}

// ---- oracle: app/owned_transcript_browsing_tests.rs (6 screen-level tests here; 2 state tests left to chat-live)
// ORACLE app/owned_transcript_browsing_tests.rs:11 provider_error_escape_opens_resume_picker kind=ui owner=chat-live
#[test]
fn provider_error_escape_opens_resume_picker() {
    kit::pending(
        Owner::ChatLive,
        "no provider-error seed: a model-provider mismatch (show_provider_error) has no kit call; Esc after it opens the resume picker and keeps the draft",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.type_text("unsent draft");
    s.keys("esc");
    s.assert_contains("unsent draft");
}

// ORACLE app/owned_transcript_browsing_tests.rs:89 escape_restores_reading_origin_after_details_navigation_and_resize kind=ui owner=chat-live
#[test]
fn escape_restores_reading_origin_after_details_navigation_and_resize() {
    kit::pending(
        Owner::ChatLive,
        "no transcript browsing: Esc primes it, ctrl+t toggles the details, and Esc restores the reading origin after a resize; the browsing and its origin are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/transcript_view_1/prompts_long.jsonl"
        ))
        .open();
    s.wheel_up(0, 4, 4);
    let before = s.shot();
    s.keys("esc esc");
    s.keys("ctrl+t");
    s.assert_contains("first prompt");
    s.keys("left");
    s.assert_contains("first prompt");
    s.keys("ctrl+t ctrl+t");
    s.resize(40, 12);
    s.keys("esc");
    s.resize(80, 16);
    s.assert_same_rows_in(&before, 0..8);
}

// ORACLE app/owned_transcript_browsing_tests.rs:171 browsing_search_and_selection_consume_escape_before_mode_exit kind=ui owner=chat-live
#[test]
fn browsing_search_and_selection_consume_escape_before_mode_exit() {
    kit::pending(
        Owner::ChatLive,
        "no transcript browsing with find (F3) and keyboard selection: the search bar and the selection are not built, so Esc does not yet close them before browsing",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("esc esc");
    s.keys("f3");
    s.type_text("first");
    s.keys("enter");
    s.keys("ctrl+space shift+right");
    s.keys("esc");
    s.keys("esc");
    s.keys("esc");
    s.assert_contains("first prompt");
}

// ORACLE app/owned_transcript_browsing_tests.rs:226 browsing_footer_adapts_to_width kind=snap owner=chat-live
#[test]
fn browsing_footer_adapts_to_width() {
    kit::pending(
        Owner::ChatLive,
        "no transcript browsing footer: two Esc on the prompt history do not draw the browsing footer (snapshot browsing_footer, 80 and 32 columns); the failed-history retry row needs a failed history load, which no kit call makes",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("esc esc");
    s.assert_snap_section("browsing_footer", "80 columns");
    s.resize(32, 8);
    s.assert_snap_section("browsing_footer", "32 columns");
}

// ORACLE app/owned_transcript_browsing_tests.rs:326 browsing_details_use_the_remapped_chord_without_cancelling_preview kind=ui owner=chat-live
#[test]
fn browsing_details_use_the_remapped_chord_without_cancelling_preview() {
    kit::pending(
        Owner::ChatLive,
        "no keymap seed: the port reads no key bindings from a config (the keymap slice), so the remapped open_transcript chord (ctrl+x ctrl+t, left f6, enter f6) and the pending-chord reveal are not set",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/transcript_view_1/first_and_long_second.jsonl"
        ))
        .keymap(&[("global.open_transcript", "ctrl+x ctrl+t")])
        .open();
    s.keys("esc esc");
    s.assert_contains("⌃x ⌃t");
    s.keys("ctrl+x ctrl+t");
    s.keys("left f6");
    s.keys("enter f6");
    s.keys("ctrl+x ctrl+p");
    s.assert_contains("Page Down");
    s.assert_absent("first prompt");
    s.assert_contains("second prompt");
}

// ORACLE app/owned_transcript_browsing_tests.rs:558 browsing_arrows_and_vim_keys_navigate_without_editing_the_draft kind=ui owner=chat-live
#[test]
fn browsing_arrows_and_vim_keys_navigate_without_editing_the_draft() {
    kit::pending(
        Owner::ChatLive,
        "no transcript browsing selection: Esc primes it, and the arrows and vim keys move the selected prompt while the composer draft stays; the browsing selection is not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/transcript_view_1/prompts_long.jsonl"
        ))
        .open();
    s.keys("esc esc");
    s.type_text("untouched draft");
    let before = s.shot();
    s.keys("up k");
    s.assert_rows_differ(&before);
    s.keys("down j h l left right");
    s.assert_contains("untouched draft");
}

// ---- oracle: app/owned_transcript_follow_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE app/owned_transcript_follow_tests.rs:11 follow_control_click_preserves_draft_caret_and_composer_geometry kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__follow_tests__running_copy_feedback_spacing snap=codex_tui__app__owned_transcript__follow_tests__running_follow_control snap=codex_tui__app__owned_transcript__follow_tests__running_follow_control_short_terminal snap=codex_tui__app__owned_transcript__follow_tests__running_without_composer_hint
#[test]
fn follow_control_click_preserves_draft_caret_and_composer_geometry() {
    kit::pending(
        Owner::ChatLive,
        "no transcript follow control: the return control (↓) of a scrolled transcript, the running turn and the copy feedback spacing are not drawn by the port's chat yet; the mouse click on ↓ and the resize geometry are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/transcript_view_1/transcript_rows.jsonl"
        ))
        .open();
    s.type_text("draft stays here");
    s.feed(include_str!(
        "fixtures/transcript_view_1/turn_started.jsonl"
    ));
    s.assert_snap("running_without_composer_hint");
    s.wheel_up(0, 4, 4);
    s.assert_contains("draft stays here");
    s.resize(80, 9);
    s.assert_snap("running_follow_control_short_terminal");
    s.resize(80, 12);
    s.assert_snap("running_follow_control");
    s.click_text("↓");
    s.assert_contains("draft stays here");
    s.assert_snap_section("running_copy_feedback_spacing", "80 columns, 12 rows");
}

// ---- oracle: app/owned_transcript_input_tests.rs (11 screen-level tests here; 1 state tests left to chat-live)
// ORACLE app/owned_transcript_input_tests.rs:71 configured_ctrl_space_submit_wins_over_transcript_selection kind=ui owner=chat-live
#[test]
fn configured_ctrl_space_submit_wins_over_transcript_selection() {
    kit::pending(
        Owner::ChatLive,
        "no keymap seed: composer.submit = ctrl-space, f9 is read from a config the port does not load (the keymap slice), so the configured submit chord cannot be set",
    );
    for running in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 12)
            .script(include_str!(
                "fixtures/transcript_view_1/transcript_one_row.jsonl"
            ))
            .keymap(&[("composer.submit", "ctrl+space")])
            .open();
        if running {
            s.feed(include_str!(
                "fixtures/transcript_view_1/turn_started.jsonl"
            ));
        }
        let text = if running { "steer once" } else { "send once" };
        s.type_text(text);
        s.keys("ctrl+space");
        s.assert_contains(text);
    }
}

// ORACLE app/owned_transcript_input_tests.rs:140 default_page_up_still_scrolls_the_transcript kind=ui owner=chat-live
#[test]
fn default_page_up_still_scrolls_the_transcript() {
    kit::pending(
        Owner::ChatLive,
        "no owned transcript: the 30 rows of a plain cell do not scroll with PageUp in the port's chat yet (no transcript view, no follow state drawn)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/transcript_view_1/transcript_rows.jsonl"
        ))
        .open();
    let before = s.shot();
    s.keys("pageup");
    s.assert_rows_differ(&before);
}

// ORACLE app/owned_transcript_input_tests.rs:239 plan_menu_allows_transcript_wheel_scrolling_and_keeps_keyboard_ownership kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__input_tests__plan_prompt_scrolled
#[test]
fn plan_menu_allows_transcript_wheel_scrolling_and_keeps_keyboard_ownership() {
    kit::pending(
        Owner::ChatLive,
        "no plan-mode turn in the port's chat: a completed plan item (40 steps) does not open the plan menu, so the menu's wheel scrolling and its keys are not built",
    );
    for close in ["esc", "enter"] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(include_str!("fixtures/transcript_view_1/plan_turn.jsonl"))
            .open();
        let latest = s.shot();
        s.wheel_up(1, 1, 1);
        s.assert_rows_differ(&latest);
        s.wheel_down(1, 1, 1);
        s.assert_same_rows(&latest);
        s.wheel_up(1, 1, 1);
        if close == "esc" {
            s.assert_snap("plan_prompt_scrolled");
        }
        s.keys("pagedown");
        s.assert_contains("› 3.");
        s.keys("pageup");
        s.assert_contains("› 1.");
        s.keys(close);
        if close == "enter" {
            s.assert_contains("Implement the plan.");
        } else {
            s.assert_absent("Implement the plan.");
        }
    }
}

// ORACLE app/owned_transcript_input_tests.rs:331 plan_menu_allows_transcript_selection_and_copy_but_respects_popup_bounds kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__input_tests__plan_prompt_selection
#[test]
fn plan_menu_allows_transcript_selection_and_copy_but_respects_popup_bounds() {
    kit::pending(
        Owner::ChatLive,
        "no plan-mode turn in the port's chat, and no mouse selection of transcript text with copy (ctrl+insert) and the plan menu's popup bounds; the selection snapshot marks the reversed cells, which the screen compare does not read",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/plan_turn.jsonl"))
        .open();
    let before = s.shot();
    s.wheel_up(1, 20, 1);
    s.assert_same_rows(&before);
    s.wheel_down(1, 20, 1);
    s.assert_same_rows(&before);
    let (x, y) = at(&s, "Plan step");
    s.drag(x, y, x + 4, y);
    s.keys("ctrl+insert");
    s.assert_snap("plan_prompt_selection");
    s.keys("esc");
    s.type_text("/m");
    let popup = s.shot();
    let (_, popup_row) = at(&s, "› /model");
    s.wheel_up(1, popup_row, 1);
    s.assert_same_rows(&popup);
    s.wheel_down(1, popup_row, 1);
    s.assert_same_rows(&popup);
}

// ORACLE app/owned_transcript_input_tests.rs:450 empty_enter_returns_to_latest_with_contextual_hints kind=ui owner=chat-live
#[test]
fn empty_enter_returns_to_latest_with_contextual_hints() {
    kit::pending(
        Owner::ChatLive,
        "no owned transcript view: the 'enter/esc latest' hint while scrolled, the draft's 'esc latest' hint and the Enter that returns to latest are not built",
    );
    for (width, detailed) in [(80, false), (40, true)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, 12)
            .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
            .open();
        s.wheel_up(0, 4, 4);
        if detailed {
            s.keys("ctrl+t");
        }
        s.assert_contains("enter/esc latest");
        s.keys("shift+enter");
        s.type_text("draft");
        s.assert_contains("esc latest");
        s.assert_absent("enter/esc latest");
        s.keys("backspace backspace backspace backspace backspace");
        s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
        s.keys("enter");
        s.assert_contains("new output");
        s.assert_absent("latest");
    }
}

// ORACLE app/owned_transcript_input_tests.rs:552 submitting_a_draft_from_history_queues_once_and_follows_immediately kind=ui owner=chat-live
#[test]
fn submitting_a_draft_from_history_queues_once_and_follows_immediately() {
    kit::pending(
        Owner::ChatLive,
        "no owned transcript view: a draft typed while scrolled up is not submitted with Enter and followed at once; the queued message and the follow are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.wheel_up(0, 4, 4);
    s.type_text("send this once");
    s.keys("enter");
    s.assert_contains("send this once");
    s.assert_absent("enter/esc latest");
}

// ORACLE app/owned_transcript_input_tests.rs:594 enter_preserves_search_backtrack_and_modal_ownership_while_scrolled kind=ui owner=chat-live
#[test]
fn enter_preserves_search_backtrack_and_modal_ownership_while_scrolled() {
    kit::pending(
        Owner::ChatLive,
        "no owned transcript view: Enter while searching, Esc with an empty selection anchor, the backtrack priming and the open modals (remote images, the 'Enable subagents?' prompt) over a scrolled transcript are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.wheel_up(0, 4, 4);
    s.keys("f3");
    s.keys("enter");
    s.keys("esc");
    s.keys("ctrl+space");
    s.keys("enter");
    s.keys("esc");
    s.assert_contains("row 29");
}

// ORACLE app/owned_transcript_input_tests.rs:687 escape_returns_to_latest_without_changing_the_composer_draft kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__input_tests__escape_returns_to_latest
#[test]
fn escape_returns_to_latest_without_changing_the_composer_draft() {
    kit::pending(
        Owner::ChatLive,
        "no owned transcript view: the 'New activity · esc latest' hint with new output while scrolled, and Esc returning to latest with the draft kept (snapshot escape_returns_to_latest) are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.type_text("draft preserved");
    s.wheel_up(0, 4, 4);
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.assert_contains("New activity · esc latest");
    s.assert_snap_section("escape_returns_to_latest", "before");
    s.keys("esc");
    s.assert_contains("draft preserved");
    s.assert_snap_section("escape_returns_to_latest", "after");
}

// ORACLE app/owned_transcript_input_tests.rs:762 raw_multiline_paste_defers_rendering_and_submits_once kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__input_tests__raw_paste_burst_render
#[test]
fn raw_multiline_paste_defers_rendering_and_submits_once() {
    kit::pending(
        Owner::ChatLive,
        "no paste burst on the chat composer: raw keys of a multi-line paste (Enter, Tab, wide glyphs) are not held back until the draw, and the 'Before paste' cell is not seeded; the 'During paste' / 'After flush' snapshot needs the deferred render",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("D");
    let before = s.shot();
    s.type_text(&format!(
        "{}end",
        "ebug\t한국어\tASCII\t0123456789\n".repeat(80)
    ));
    s.assert_same_rows(&before);
    s.assert_snap_section("raw_paste_burst_render", "During paste:");
    s.wait(1000);
    s.assert_contains("After paste");
    s.assert_snap_section("raw_paste_burst_render", "After flush:");
    s.keys("enter");
}

// ORACLE app/owned_transcript_input_tests.rs:835 accepted_find_defers_paste_rendering_but_keeps_enter kind=ui owner=chat-live
#[test]
fn accepted_find_defers_paste_rendering_but_keeps_enter() {
    kit::pending(
        Owner::ChatLive,
        "no transcript find on the chat: F3 with a pasted needle, Enter accepting it, and a paste burst held back under the accepted find are not built (the 'Before paste needle' cell is not seeded)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.keys("f3");
    s.paste("needle");
    s.keys("enter");
    s.type_text("D");
    let before = s.shot();
    s.type_text(&"ebug log ".repeat(80));
    s.assert_same_rows(&before);
    s.keys("enter");
    s.assert_rows_differ(&before);
    s.wait(1000);
    s.assert_contains("ebug log");
}

// ORACLE app/owned_transcript_input_tests.rs:899 composer_paste_releases_selection_and_retains_normal_cursor_movement kind=ui owner=chat-live
#[test]
fn composer_paste_releases_selection_and_retains_normal_cursor_movement() {
    kit::pending(
        Owner::ChatLive,
        "no transcript selection to release: the oracle's select_transcript helper starts a selection, which the port's chat has no key for (select_transcript is unbound by default); a paste into the composer then has to release it and keep the cursor movement",
    );
    for detailed in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
        s.type_text("draft ");
        if detailed {
            s.keys("ctrl+t");
        }
        s.paste("café\r\nbeta\rgamma");
        s.assert_contains("draft café");
        s.assert_contains("gamma");
        s.keys("left");
        s.paste("X");
        s.assert_contains("gammXa");
    }
}

// ---- oracle: app/owned_transcript_tests.rs (23 screen-level tests here; 4 state tests left to chat-live)
// ORACLE app/owned_transcript_tests.rs:78 external_writer_escape_returns_to_command_center_without_editing kind=snap owner=chat-live snap=external_writer_escape_command_center snap=external_writer_escape_offline_command_center
#[test]
fn external_writer_escape_returns_to_command_center_without_editing() {
    kit::pending(
        Owner::ChatLive,
        "no external-writer thread: a remote app-server target showing the external writer (the agent command center, S21) and the offline reconnect overview are not built; the snapshots are the command-center popup only (96 columns)",
    );
    for (detailed, scrolled, offline) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (false, false, true),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(96, 24)
            .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
            .open();
        if detailed {
            s.keys("ctrl+t");
        }
        if scrolled {
            s.wheel_up(0, 4, 4);
        }
        s.keys("esc");
        s.assert_absent("draft");
        if offline {
            s.assert_snap("external_writer_escape_offline_command_center");
        } else if !detailed && !scrolled {
            s.assert_snap("external_writer_escape_command_center");
        }
    }
}

// ORACLE app/owned_transcript_tests.rs:153 list_spacing_completion_preserves_the_scrolled_reader kind=ui owner=chat-live
#[test]
fn list_spacing_completion_preserves_the_scrolled_reader() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:200 older_page_loading_uses_the_status_row_without_moving_content_or_cursor kind=snap owner=chat-live snap=owned_history_loading_footer
#[test]
fn older_page_loading_uses_the_status_row_without_moving_content_or_cursor() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:265 recap_spacing_belongs_to_the_transcript_tail kind=snap owner=chat-live snap=recap_tail_spacing
#[test]
fn recap_spacing_belongs_to_the_transcript_tail() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:328 owned_transcript_reserves_a_row_above_the_composer kind=snap owner=chat-live snap=owned_transcript_composer_gap
#[test]
fn owned_transcript_reserves_a_row_above_the_composer() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:422 owned_drag_stops_when_focus_or_input_ownership_is_lost kind=ui owner=chat-live
#[test]
fn owned_drag_stops_when_focus_or_input_ownership_is_lost() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:503 owned_details_keep_the_composer_cursor_and_screen kind=snap owner=chat-live snap=conversation_in_offset_rect
#[test]
fn owned_details_keep_the_composer_cursor_and_screen() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:709 owned_transcript_keeps_text_out_of_the_pet_columns kind=ui owner=chat-live
#[test]
fn owned_transcript_keeps_text_out_of_the_pet_columns() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:851 cancelling_owned_backtrack_returns_arrow_keys_to_the_composer kind=ui owner=chat-live
#[test]
fn cancelling_owned_backtrack_returns_arrow_keys_to_the_composer() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:918 owned_search_and_selection_consume_input_before_composer_and_backtrack kind=ui owner=chat-live
#[test]
fn owned_search_and_selection_consume_input_before_composer_and_backtrack() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1024 keyboard_select_mode_preserves_the_draft_and_absorbs_enter_repeat kind=ui owner=chat-live
#[test]
fn keyboard_select_mode_preserves_the_draft_and_absorbs_enter_repeat() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1156 inline_transcript_search_draws_and_escape_precedes_backtrack kind=ui owner=chat-live
#[test]
fn inline_transcript_search_draws_and_escape_precedes_backtrack() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1208 find_owns_editor_chords_without_changing_the_composer_draft kind=ui owner=chat-live
#[test]
fn find_owns_editor_chords_without_changing_the_composer_draft() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1317 offline_find_closes_before_the_next_ctrl_c_quits kind=ui owner=chat-live
#[test]
fn offline_find_closes_before_the_next_ctrl_c_quits() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1461 slash_picker_overlays_history_without_moving_the_transcript_or_composer kind=snap owner=chat-live snap=slash_picker_overlays_history
#[test]
fn slash_picker_overlays_history_without_moving_the_transcript_or_composer() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1576 double_escape_browses_prompts_without_reverting_and_explains_editing kind=snap owner=chat-live snap=prompt_navigation
#[test]
fn double_escape_browses_prompts_without_reverting_and_explains_editing() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1647 find_refreshes_live_details_before_searching_the_first_query kind=snap owner=chat-live snap=compact_overlay_finds_hidden_live_text
#[test]
fn find_refreshes_live_details_before_searching_the_first_query() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1750 fullscreen_composer_mouse_copy_and_input_ownership kind=snap owner=chat-live snap=fullscreen_composer_right_click_copy
#[test]
fn fullscreen_composer_mouse_copy_and_input_ownership() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1928 inline_confirmation_preserves_compact_viewport_without_replaying_history kind=ui owner=chat-live
#[test]
fn inline_confirmation_preserves_compact_viewport_without_replaying_history() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:1961 fullscreen_composer_scrolls_without_moving_caret_or_transcript kind=snap owner=chat-live snap=fullscreen_prompt_wheel
#[test]
fn fullscreen_composer_scrolls_without_moving_caret_or_transcript() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:2055 composer_thumbnails_follow_draft_deletion_and_resize kind=snap owner=chat-live snap=composer_thumbnail_strip
#[test]
fn composer_thumbnails_follow_draft_deletion_and_resize() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:2158 fullscreen_composer_remote_images_leave_an_editable_prompt_row kind=snap owner=chat-live snap=fullscreen_prompt_remote_images
#[test]
fn fullscreen_composer_remote_images_leave_an_editable_prompt_row() {
    kit::todo();
}

// ORACLE app/owned_transcript_tests.rs:2212 footer_selection_defers_warning_clicks_and_preserves_the_draft kind=snap owner=chat-live snap=fullscreen_footer_selection
#[test]
fn footer_selection_defers_warning_clicks_and_preserves_the_draft() {
    kit::todo();
}

// ---- oracle: transcript_view/activity_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/activity_tests.rs:23 visible_updates_and_empty_tail_cells_do_not_report_activity kind=ui owner=chat-live
#[test]
fn visible_updates_and_empty_tail_cells_do_not_report_activity() {
    kit::todo();
}

// ORACLE transcript_view/activity_tests.rs:40 unchanged_live_revisions_do_not_report_activity_but_hidden_text_does kind=ui owner=chat-live
#[test]
fn unchanged_live_revisions_do_not_report_activity_but_hidden_text_does() {
    kit::todo();
}

// ORACLE transcript_view/activity_tests.rs:65 escape_closes_selection_and_search_before_returning_to_latest kind=ui owner=chat-live
#[test]
fn escape_closes_selection_and_search_before_returning_to_latest() {
    kit::todo();
}

// ---- oracle: transcript_view/bookmark_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/bookmark_tests.rs:8 cancel_restores_both_positions_after_resize_and_pagination kind=ui owner=chat-live
#[test]
fn cancel_restores_both_positions_after_resize_and_pagination() {
    kit::todo();
}

// ORACLE transcript_view/bookmark_tests.rs:38 following_origin_returns_to_new_output_in_original_presentation kind=ui owner=chat-live
#[test]
fn following_origin_returns_to_new_output_in_original_presentation() {
    kit::todo();
}

// ORACLE transcript_view/bookmark_tests.rs:57 cancel_restores_a_group_replaced_during_browsing kind=ui owner=chat-live
#[test]
fn cancel_restores_a_group_replaced_during_browsing() {
    kit::todo();
}

// ---- oracle: transcript_view/composer_gap_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/composer_gap_tests.rs:14 pending_copy_feedback_survives_expiry_and_selection_replacement kind=snap owner=chat-live snap=pending_copy_feedback_survives_expiry_and_selection_replacement
#[test]
fn pending_copy_feedback_survives_expiry_and_selection_replacement() {
    kit::todo();
}

// ORACLE transcript_view/composer_gap_tests.rs:85 copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery kind=snap owner=chat-live snap=copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery
#[test]
fn copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery() {
    kit::todo();
}

// ORACLE transcript_view/composer_gap_tests.rs:118 feedback_releases_navigation_targets_and_expiry_restores_them kind=ui owner=chat-live
#[test]
fn feedback_releases_navigation_targets_and_expiry_restores_them() {
    kit::todo();
}

// ORACLE transcript_view/composer_gap_tests.rs:149 tip_links_follow_alignment_and_release_stale_targets kind=ui owner=chat-live
#[test]
fn tip_links_follow_alignment_and_release_stale_targets() {
    kit::todo();
}
