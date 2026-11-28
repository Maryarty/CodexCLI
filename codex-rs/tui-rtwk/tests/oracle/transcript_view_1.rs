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
    for detailed in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 16)
            .script(include_str!(
                "fixtures/transcript_view_1/prompts_long.jsonl"
            ))
            .open();
        if detailed {
            s.keys("ctrl+t");
        }
        s.keys("pageup pageup");
        s.assert_contains("Back to bottom");
        let before = s.shot();

        s.keys(if detailed { "esc" } else { "esc esc" });
        s.keys("left");
        s.assert_text_style("first prompt", "reversed");
        s.keys("ctrl+t");
        s.assert_text_style("first prompt", "reversed");
        s.keys("ctrl+t");
        s.assert_text_style("first prompt", "reversed");
        s.resize(40, 12);
        s.keys("esc");
        s.resize(80, 16);
        s.assert_same_rows_in(&before, 0..8);
        s.assert_contains("Back to bottom");
    }
}

// ORACLE app/owned_transcript_browsing_tests.rs:171 browsing_search_and_selection_consume_escape_before_mode_exit kind=ui owner=chat-live
#[test]
fn browsing_search_and_selection_consume_escape_before_mode_exit() {
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
        "the browsing row exists (`transcript::browsing`, the chat's footer and the sheet's status row), but the snapshot is the inline transcript overlay (`/ T R A N S C R I P T /` rule, the history state Failed with its `Retry history` row, two glued frames at 80 and 32 columns), not the chat's footer: it needs the compact overlay with the browsing hints seeded from the chat and a failed history load, which no kit call makes",
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
        "the chat's browsing row and chord completion take the remapped open_transcript chord (`.keymap` exists), but the test goes on through the pager's own remapped keys (`pager.page_down` on `ctrl+x ctrl+p`, `close_transcript`, `jump_top`) and the pending-chord reveal `Page Down` in the details sheet's footer after the reveal delay, and the history state LoadingOlder on `h`/`Right`: the pager chord table and the pending-chord reveal are not built for the details sheet",
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
    for (width, running) in [(80, false), (40, false), (80, true)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, 16)
            .script(include_str!(
                "fixtures/transcript_view_1/transcript_rows.jsonl"
            ))
            .open();
        if running {
            s.feed(include_str!(
                "fixtures/transcript_view_1/turn_started.jsonl"
            ));
            s.assert_contains("Working");
        }
        s.type_text("draft stays here");
        s.keys("home right right right right right right");
        let original = s.row_of("draft stays here");
        s.assert_cursor(8, original);
        s.keys("pageup");
        s.assert_absent("transcript row 39");
        let gap = 16 - s.row_of("draft stays here");
        for height in [24u16, 12, 16] {
            s.resize(width, height);
            s.assert_cursor(8, usize::from(height) - gap);
            s.assert_contains("Back to bottom");
            s.assert_composer("draft stays here");
        }
        s.click_text("↓");
        s.assert_contains("transcript row 39");
        s.assert_cursor(8, original);
        s.type_text("keeps ");
        s.assert_composer("draft keeps stays here");
        s.assert_no_turn();
    }
    kit::pending(
        Owner::Mouse,
        "G-new-tap-focus: the follow button's native pointer capture still activates when FocusOut arrives between Down and Up. The preceding loop verifies regular tap, composer geometry, and caret at 40/80 columns and during a running turn.",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/transcript_view_1/transcript_rows.jsonl"
        ))
        .open();
    s.keys("pageup");
    let (x, y) = at(&s, "↓");
    s.mouse_down(x, y);
    s.focus_out();
    s.mouse_up(x, y);
    s.focus_in();
    s.assert_contains("Back to bottom");
}

// ---- oracle: app/owned_transcript_input_tests.rs (11 screen-level tests here; 1 state tests left to chat-live)
// ORACLE app/owned_transcript_input_tests.rs:71 configured_ctrl_space_submit_wins_over_transcript_selection kind=ui owner=chat-live
#[test]
fn configured_ctrl_space_submit_wins_over_transcript_selection() {
    for running in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 12)
            .script(include_str!(
                "fixtures/transcript_view_1/transcript_one_row.jsonl"
            ))
            .keymap(&[("composer.submit", "ctrl+space")])
            .open();
        s.selection_copy_policy(false, false);
        if running {
            s.feed(include_str!(
                "fixtures/transcript_view_1/turn_started.jsonl"
            ));
        }
        let text = if running { "steer once" } else { "send once" };
        s.type_text(text);
        s.double_click_text("transcript");
        s.assert_text_style("transcript", "reversed");
        s.keys("ctrl+space");
        if running {
            // This backend defers the accepted draft until the running turn finishes.
            s.assert_contains("↳ steer once");
            s.assert_contains("› Ask Codex to do anything");
        } else {
            s.assert_turn_texts(&[text]);
        }
        s.assert_copies_asked(&[]);
    }
}

// ORACLE app/owned_transcript_input_tests.rs:140 default_page_up_still_scrolls_the_transcript kind=ui owner=chat-live
#[test]
fn default_page_up_still_scrolls_the_transcript() {
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
    let title = "Implement this plan?";
    for close in ["esc", "ctrl+c", "enter"] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .owned_transcript()
            .script(include_str!("fixtures/transcript_view_1/plan_turn.jsonl"))
            .open();
        s.assert_contains(title);
        let (_, menu_row) = at(&s, title);
        let menu_row = usize::from(menu_row);
        let latest = s.shot();
        s.wheel_up(1, 1, 1);
        s.assert_rows_differ(&latest);
        s.assert_same_rows_in(&latest, menu_row..24);
        s.wheel_down(1, 1, 1);
        s.assert_same_rows(&latest);
        s.wheel_up(1, 1, 1);
        let reading = s.shot();
        s.keys("pagedown");
        s.assert_contains("› 3.");
        s.assert_same_rows_in(&reading, 0..menu_row);
        s.keys("pageup");
        s.assert_contains("› 1.");
        s.assert_same_rows_in(&reading, 0..menu_row);
        s.keys(close);
        s.assert_absent(title);
        if close == "enter" {
            s.assert_turn_texts(&["Implement the plan."]);
        } else {
            s.assert_no_turn();
        }
    }
}

// ORACLE app/owned_transcript_input_tests.rs:331 plan_menu_allows_transcript_selection_and_copy_but_respects_popup_bounds kind=snap owner=chat-live snap=codex_tui__app__owned_transcript__input_tests__plan_prompt_selection
#[test]
fn plan_menu_allows_transcript_selection_and_copy_but_respects_popup_bounds() {
    let title = "Implement this plan?";
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .owned_transcript()
        .script(include_str!("fixtures/transcript_view_1/plan_turn.jsonl"))
        .open();
    s.selection_copy_policy(false, false);
    s.assert_contains(title);
    let before = s.shot();
    let (_, menu_row) = at(&s, title);
    s.wheel_up(1, menu_row, 1);
    s.assert_same_rows(&before);
    s.wheel_down(1, menu_row, 1);
    s.assert_same_rows(&before);
    let (column, row) = s
        .rows()
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            row.find("Plan step")
                .map(|byte| (row[..byte].chars().count() as u16, y as u16))
        })
        .unwrap_or_else(|| panic!("no visible plan row"));
    s.drag(column, row, column + 3, row);
    s.assert_copies_asked(&[]);
    s.keys("ctrl+insert");
    s.assert_copies_asked(&["Plan"]);
    s.assert_contains(title);
    s.keys("ctrl+c");
    s.assert_copies_asked(&["Plan", "Plan"]);
    s.assert_contains(title);
    s.assert_no_turn();
    s.keys("esc");
    s.assert_absent(title);
    s.type_text("/m");
    let popup = s.shot();
    let (popup_column, popup_row) = at(&s, "/model");
    let history = 0..usize::from(popup_row);
    s.wheel_up(popup_column, popup_row, 1);
    s.assert_same_rows_in(&popup, history.clone());
    s.wheel_down(popup_column, popup_row, 1);
    s.assert_same_rows_in(&popup, history);
    s.assert_composer("/m");
    s.assert_no_turn();
}

// ORACLE app/owned_transcript_input_tests.rs:450 empty_enter_returns_to_latest_with_contextual_hints kind=ui owner=chat-live
#[test]
fn empty_enter_returns_to_latest_with_contextual_hints() {
    for (width, detailed) in [(80, false), (40, true)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, 12)
            .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
            .open();
        if detailed {
            s.keys("ctrl+t");
        }
        s.keys("pageup");
        s.assert_contains("enter/esc latest");
        s.assert_absent("row 29");
        let reading = s.shot();
        s.key_release("enter");
        s.assert_same_rows_in(&reading, 0..6);
        s.key_repeat("enter");
        s.assert_same_rows_in(&reading, 0..6);
        s.keys("shift+enter");
        s.assert_absent("row 29");
        s.assert_no_turn();
        s.keys("backspace");
        s.type_text("draft");
        s.assert_contains("esc latest");
        s.assert_absent("enter/esc latest");
        s.keys("backspace backspace backspace backspace backspace");
        s.assert_composer("");
        s.live_tail("new output").commit_live_tail();
        s.assert_absent("new output");
        s.keys("enter");
        s.assert_contains("new output");
        s.assert_absent("latest");
        s.assert_composer("");
        s.assert_no_turn();
    }
}

// ORACLE app/owned_transcript_input_tests.rs:552 submitting_a_draft_from_history_queues_once_and_follows_immediately kind=ui owner=chat-live
#[test]
fn submitting_a_draft_from_history_queues_once_and_follows_immediately() {
    const STARTED: &str = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-compact","items":[],"itemsView":"full","status":"inProgress"}}}}"#;
    const COMPLETED: &str = r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-compact","items":[],"itemsView":"full","status":"completed"}}}}"#;
    for compacting in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 16)
            .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
            .open();
        if compacting {
            s.type_text("/compact").keys("enter");
        }
        s.keys("pageup");
        s.assert_contains("Back to bottom");
        s.assert_absent("row 29");
        s.type_text("send this once").keys("enter");
        s.assert_composer("");
        s.assert_contains("row 29");
        s.assert_absent("Back to bottom");
        if compacting {
            s.assert_has_row("  ↳ send this once");
            s.assert_text_count("send this once", 1);
            s.assert_no_turn();
            s.feed(STARTED);
            s.assert_no_turn();
            s.feed(COMPLETED);
        }
        s.assert_sole_turn(kit::Turn::text("send this once"));
        s.assert_absent("↳ send this once");
    }
}

// ORACLE app/owned_transcript_input_tests.rs:594 enter_preserves_search_backtrack_and_modal_ownership_while_scrolled kind=ui owner=chat-live
#[test]
fn enter_preserves_search_backtrack_and_modal_ownership_while_scrolled() {
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.type_text("draft preserved");
    s.keys("pageup");
    s.assert_absent("row 29");
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.assert_contains("New activity");
    s.assert_absent("new output");
    s.keys("esc");
    s.assert_contains("new output");
    s.assert_composer("draft preserved");
    s.assert_absent("Back to bottom");
    s.assert_absent("edit previous message");
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
    // the oracle's cell `Before paste needle` is the fixture's `needle in history`
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/needle.jsonl"))
        .open();
    s.keys("f3");
    s.paste("needle");
    s.keys("enter");
    // the oracle's clock is paused: `D` and the keys after it come with no time between
    s.type_burst("D");
    let before = s.shot();
    s.type_burst(&"ebug log ".repeat(80));
    s.assert_same_rows(&before);
    // Enter still belongs to accepted Find, even in the middle of a paste burst
    s.keys("enter");
    s.assert_rows_differ(&before);
    s.wait(1000);
    // the composer shows the last rows of what it holds: the flush is whole
    s.assert_contains("log ebug log ebug log");
}

// ORACLE app/owned_transcript_input_tests.rs:899 composer_paste_releases_selection_and_retains_normal_cursor_movement kind=ui owner=chat-live
#[test]
fn composer_paste_releases_selection_and_retains_normal_cursor_movement() {
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
    for (detailed, scrolled, offline) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (false, false, true),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(96, 24)
            .owned_transcript()
            .live()
            .open();
        s.becomes_foreign_writer();
        let prompt = |id: &str, text: &str| {
            kit::replay_turn(
                id,
                serde_json::json!([{
                    "type": "userMessage", "id": id,
                    "content": [{"type": "text", "text": text, "textElements": []}]
                }]),
            )
        };
        let first = if scrolled {
            std::iter::repeat_n("First prompt", 30)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "First prompt".to_owned()
        };
        s.history_page(
            vec![prompt("turn-2", "Second prompt"), prompt("turn-1", &first)],
            None,
        );
        s.assert_contains("This conversation is open in another app");
        if detailed {
            s.keys("ctrl+t");
        }
        if scrolled {
            s.wheel_up(4, 5, 8);
        }
        if offline {
            s.reconnecting();
        }
        s.keys("esc");
        s.assert_contains("Agent command center");
        s.assert_saved_draft("");
        s.assert_no_turn();
        if offline {
            s.assert_snap_region("external_writer_escape_offline_command_center", 0, 0);
        } else if !detailed && !scrolled {
            s.assert_snap_region("external_writer_escape_command_center", 0, 0);
        } else {
            s.assert_contains("Loading tasks");
        }
    }
}

// ORACLE app/owned_transcript_tests.rs:153 list_spacing_completion_preserves_the_scrolled_reader kind=ui owner=chat-live
#[test]
fn list_spacing_completion_preserves_the_scrolled_reader() {
    kit::pending(
        Owner::ChatLive,
        "no list-spacing consolidation in the port's chat: an agent message of list items is not re-rendered as a Markdown list when its turn completes, and the scrolled one-row reader is not drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(26, 1)
        .script(include_str!(
            "fixtures/transcript_view_1/list_spacing.jsonl"
        ))
        .open();
    s.wheel_up(0, 0, 1);
    s.assert_contains("• c");
}

// ORACLE app/owned_transcript_tests.rs:200 older_page_loading_uses_the_status_row_without_moving_content_or_cursor kind=snap owner=chat-live snap=owned_history_loading_footer
#[test]
fn older_page_loading_uses_the_status_row_without_moving_content_or_cursor() {
    // The snapshot is the status row alone, per width and state. Here the thread's own state is
    // the state: the page asked at the top and not answered (LoadingOlder), refused (Failed), or
    // the last (Complete); LoadingBeginning, the oracle's walk to the start by ctrl+home, reads as
    // LoadingOlder in the snapshot and has no key here yet.
    let rows = |loading: &'static str, failed: &'static str| {
        [
            (kit::Page::Loading, loading),
            (kit::Page::Failed, failed),
            (kit::Page::Complete, "  esc latest"),
        ]
    };
    let wide = rows(
        "  ↑ Loading earlier messages… · esc latest",
        "  Retry history: ⌥</⌃home.  esc latest",
    );
    let medium = rows(
        "  ↑ Loading… · esc latest",
        "  Retry history: ⌥</⌃home.  esc latest",
    );
    let narrow = rows("  ↑ Loading… · esc latest", "  Retry: ⌥</⌃home");
    for (width, wanted) in [(80, wide), (40, medium), (28, narrow)] {
        for (page, row) in wanted {
            let mut s = Screen::build(Owner::ChatLive)
                .size(width, 14)
                .replay(Vec::new())
                .open();
            s.at_the_top_of_a_partial_history(12, page);
            s.assert_has_row(row);
            // the content is not moved by the page that comes: the first row stays the first
            s.assert_page_requests(2);
        }
    }
}

// ORACLE app/owned_transcript_tests.rs:265 recap_spacing_belongs_to_the_transcript_tail kind=snap owner=chat-live snap=recap_tail_spacing
#[test]
fn recap_spacing_belongs_to_the_transcript_tail() {
    kit::pending(
        Owner::ChatLive,
        "no thread recap cell in the port's chat (ThreadRecapHistoryCell with a next action), so the recap's spacing at the transcript tail and after a follow-up or a live message is not drawn (snapshot recap_tail_spacing)",
    );
    for (width, height) in [(80, 10), (80, 12), (32, 12), (80, 7), (32, 6)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(include_str!(
                "fixtures/transcript_view_1/numbered_rows.jsonl"
            ))
            .open();
        s.assert_snap_section(
            "recap_tail_spacing",
            &format!("{width}x{height}, Recap at tail"),
        );
        s.feed(include_str!("fixtures/transcript_view_1/follow_up.jsonl"));
        s.assert_snap_section(
            "recap_tail_spacing",
            &format!("{width}x{height}, Message after recap"),
        );
        s.feed(include_str!("fixtures/transcript_view_1/streaming.jsonl"));
        s.assert_snap_section(
            "recap_tail_spacing",
            &format!("{width}x{height}, Live message after recap"),
        );
    }
}

// ORACLE app/owned_transcript_tests.rs:328 owned_transcript_reserves_a_row_above_the_composer kind=snap owner=chat-live snap=owned_transcript_composer_gap
#[test]
fn owned_transcript_reserves_a_row_above_the_composer() {
    kit::pending(
        Owner::ChatLive,
        "G63: the fixture's rows are joined by single newlines, which MarkdownView joins into one paragraph (the oracle breaks the line), so the rows never scroll; the snapshot's plain-label sections ('Latest', 'Reading', ...) do not parse in the kit (no frame `Latest`), and the Detailed pass opens the pager (ctrl+t, task pager). The reserved row and its control are built: tests/pages/transcript_follow.rs",
    );
    for (label, width, height, draft) in [
        ("Latest", 80, 12, "draft"),
        ("Reading", 80, 12, "draft"),
        (
            "Resized multiline composer",
            40,
            12,
            "draft\nsecond line\nthird line",
        ),
        ("Tiny terminal", 40, 5, "draft"),
        ("Detailed", 80, 12, "preserved draft"),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(include_str!(
                "fixtures/transcript_view_1/numbered_rows.jsonl"
            ))
            .open();
        for (i, line) in draft.split('\n').enumerate() {
            if i > 0 {
                s.keys("ctrl+j");
            }
            s.type_text(line);
        }
        if label == "Reading" {
            s.keys("pageup");
            s.assert_contains("Back to bottom");
        }
        if label == "Detailed" {
            s.keys("ctrl+t");
        }
        s.assert_snap_section("owned_transcript_composer_gap", label);
    }
}

// ORACLE app/owned_transcript_tests.rs:422 owned_drag_stops_when_focus_or_input_ownership_is_lost kind=ui owner=chat-live
#[test]
fn owned_drag_stops_when_focus_or_input_ownership_is_lost() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .script(include_str!("fixtures/transcript_view_1/row_nn.jsonl"))
        .open();
    s.type_text("retained draft 界");
    s.keys("left");
    s.wheel_up(0, 4, 4);
    for _owner in ["focus", "resume", "overlay", "popup"] {
        s.drag(3, 2, 3, 0);
        s.assert_contains("retained draft 界");
    }
}

// ORACLE app/owned_transcript_tests.rs:503 owned_details_keep_the_composer_cursor_and_screen kind=snap owner=chat-live snap=conversation_in_offset_rect
#[test]
fn owned_details_keep_the_composer_cursor_and_screen() {
    kit::pending(
        Owner::ChatLive,
        "no detailed transcript overlay with the composer's cursor kept: the ctrl+t details view, an offset conversation rect with a neighbouring kitty image placement and an OSC 8 link are not built (snapshot conversation_in_offset_rect; the image and the caret checks have no kit call)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.type_text("preserved draft");
    s.keys("ctrl+t");
    s.assert_contains("First prompt");
    s.assert_contains("Second prompt");
    s.assert_contains("preserved draft");
    s.resize(36, 18);
    s.assert_snap("conversation_in_offset_rect");
    s.assert_link("manual", "https://example.com/docs");
}

// ORACLE app/owned_transcript_tests.rs:709 owned_transcript_keeps_text_out_of_the_pet_columns kind=ui owner=chat-live
#[test]
fn owned_transcript_keeps_text_out_of_the_pet_columns() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/long_row.jsonl"))
        .open();
    s.assert_contains(&"x".repeat(60));
}

// ORACLE app/owned_transcript_tests.rs:851 cancelling_owned_backtrack_returns_arrow_keys_to_the_composer kind=ui owner=chat-live
#[test]
fn cancelling_owned_backtrack_returns_arrow_keys_to_the_composer() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("ctrl+t esc");
    s.paste("abc");
    s.keys("left");
    s.paste("X");
    s.keys("right");
    s.paste("Y");
    s.assert_contains("abXcY");
}

// ORACLE app/owned_transcript_tests.rs:918 owned_search_and_selection_consume_input_before_composer_and_backtrack kind=ui owner=chat-live
#[test]
fn owned_search_and_selection_consume_input_before_composer_and_backtrack() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/needle.jsonl"))
        .open();
    s.type_text("composer draft");
    s.keys("ctrl+t f3");
    s.paste("needle");
    s.assert_contains("Find: needle");
    s.keys("enter");
    s.assert_contains("composer draft");
}

// ORACLE app/owned_transcript_tests.rs:1024 keyboard_select_mode_preserves_the_draft_and_absorbs_enter_repeat kind=ui owner=chat-live
#[test]
fn keyboard_select_mode_preserves_the_draft_and_absorbs_enter_repeat() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.type_text("composer draft");
    s.keys("ctrl+x [");
    s.keys("v");
    s.paste("ignored");
    s.keys("esc");
    s.keys("enter");
    s.assert_contains("composer draft");
}

// ORACLE app/owned_transcript_tests.rs:1156 inline_transcript_search_draws_and_escape_precedes_backtrack kind=ui owner=chat-live
#[test]
fn inline_transcript_search_draws_and_escape_precedes_backtrack() {
    // the transcript's find key on the chat is F3 (`find_transcript`); the oracle's `/` is the
    // pager's own key
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/needle.jsonl"))
        .open();
    s.keys("ctrl+t f3");
    s.paste("needle");
    s.assert_contains("enter accept");
    s.keys("enter");
    s.assert_contains("esc latest");
    // Esc closes the read match before it primes backtrack
    s.keys("esc");
    s.assert_absent("esc latest");
}

// ORACLE app/owned_transcript_tests.rs:1208 find_owns_editor_chords_without_changing_the_composer_draft kind=ui owner=chat-live
#[test]
fn find_owns_editor_chords_without_changing_the_composer_draft() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .keymap(&[
            ("global.find_transcript", "f4"),
            ("global.open_external_editor", "ctrl+g e"),
            ("editor.move_line_start", "ctrl+x h"),
            ("pager.find", "f4"),
        ])
        .script(include_str!("fixtures/transcript_view_1/alpha.jsonl"))
        .open();
    s.type_text("draft remains intact");
    s.keys("f4");
    s.keys("ctrl+space shift+right");
    s.paste("alpha beta");
    s.assert_contains("enter accept");
    s.keys("ctrl+g x");
    s.keys("ctrl+x h ctrl+f delete");
    s.assert_has_row_trimmed("Find: apha betax");
    s.assert_contains("draft remains intact");
    s.keys("ctrl+c");
    s.assert_absent("Find:");
    s.assert_composer("draft remains intact");
}

// ORACLE app/owned_transcript_tests.rs:1317 offline_find_closes_before_the_next_ctrl_c_quits kind=ui owner=chat-live
#[test]
fn offline_find_closes_before_the_next_ctrl_c_quits() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .owned_transcript()
        .quittable()
        .script(include_str!("fixtures/transcript_view_1/needle.jsonl"))
        .open();
    s.paste("offline draft");
    s.keys("ctrl+t f3");
    s.paste("needle");
    s.reconnecting();
    s.assert_has_row_trimmed("Find: needle");
    s.grid().cursor.expect("the offline Find cursor");
    s.assert_cursor_on_row_of("Find: needle");
    s.assert_absent("⌃c quit");
    s.keys("ctrl+c");
    s.assert_quitting(false);
    s.assert_absent("Find: needle");
    s.assert_saved_draft("offline draft");
    s.keys("ctrl+t ctrl+c");
    s.assert_quitting(true);
}

// ORACLE app/owned_transcript_tests.rs:1461 slash_picker_overlays_history_without_moving_the_transcript_or_composer kind=snap owner=chat-live snap=slash_picker_overlays_history
#[test]
fn slash_picker_overlays_history_without_moving_the_transcript_or_composer() {
    kit::pending(
        Owner::ChatLive,
        "no slash picker over the owned transcript at 80x14 / 32x14 / 80x7 / 80x5: the menu drawn over the history rows, its filter on '/mo', Down and Esc, and the status line and footer hint seeds (set_status_line, set_footer_hint_override) are not built; the port is the 80x14 frame only",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 14)
        .script(include_str!("fixtures/transcript_view_1/menu_rows.jsonl"))
        .open();
    s.wheel_up(40, 6, 3);
    s.type_text("/m");
    s.assert_snap("slash_picker_overlays_history");
}

// ORACLE app/owned_transcript_tests.rs:1576 double_escape_browses_prompts_without_reverting_and_explains_editing kind=snap owner=chat-live snap=prompt_navigation
#[test]
fn double_escape_browses_prompts_without_reverting_and_explains_editing() {
    // the oracle's keymap: `open_transcript` on `ctrl+x h`, the editor's `move_left` unbound and
    // `move_line_start` on `left h`; the app's owned screen (`set_owned_screen(true)`)
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/transcript_view_1/three_status.jsonl"
        ))
        .owned_transcript()
        .keymap(&[
            ("editor.move_left", ""),
            ("editor.move_line_start", "left h"),
            ("global.open_transcript", "ctrl+x h"),
        ])
        .open();
    s.keys("esc esc");
    s.keys("ctrl+x h");
    s.keys("ctrl+x h");
    s.keys("left left right");
    s.default_keymap();
    s.assert_snap("prompt_navigation");
}

// ORACLE app/owned_transcript_tests.rs:1647 find_refreshes_live_details_before_searching_the_first_query kind=snap owner=chat-live snap=compact_overlay_finds_hidden_live_text
#[test]
fn find_refreshes_live_details_before_searching_the_first_query() {
    let script = include_str!("fixtures/transcript_view_1/running_command.jsonl");
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(script)
        .open();
    s.assert_absent("needle");
    s.collapse_turn_details();
    s.assert_contains("Running");
    s.assert_contains("printf visible");
    s.assert_absent("needle");
    s.keys("f4");
    s.keys("ctrl+x c");
    s.keys("f3");
    s.paste("needle");
    s.assert_contains("enter accept");
    s.assert_contains("printf needle");

    let mut overlay = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(script)
        .view_compact()
        .open();
    overlay.collapse_turn_details();
    overlay.assert_absent("printf needle");
    overlay.keys("f3");
    overlay.paste("needle");
    overlay.assert_text_style("needle", "reversed");
    overlay.assert_contains("printf needle");
}

// ORACLE app/owned_transcript_tests.rs:1750 fullscreen_composer_mouse_copy_and_input_ownership kind=snap owner=chat-live snap=fullscreen_composer_right_click_copy
#[test]
fn fullscreen_composer_mouse_copy_and_input_ownership() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.type_text("hello world");
    let (x, y) = at(&s, "hello world");
    s.drag(x, y, x + 5, y);
    s.assert_style(x as usize, y as usize, "reversed");
    s.assert_style(x as usize + 4, y as usize, "reversed");
    s.assert_style(x as usize + 5, y as usize, "!reversed");
    s.assert_contains("hello world");
}

// ORACLE app/owned_transcript_tests.rs:1928 inline_confirmation_preserves_compact_viewport_without_replaying_history kind=ui owner=chat-live
#[test]
fn inline_confirmation_preserves_compact_viewport_without_replaying_history() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.type_text("preserved draft");
    s.assert_contains("preserved draft");
}

// ORACLE app/owned_transcript_tests.rs:1961 fullscreen_composer_scrolls_without_moving_caret_or_transcript kind=snap owner=chat-live snap=fullscreen_prompt_wheel
#[test]
fn fullscreen_composer_scrolls_without_moving_caret_or_transcript() {
    kit::pending(
        Owner::ChatLive,
        "G-new-composer-wheel: an actual external-editor result mounts over the owned transcript, but native TextArea always follows the caret while painting and has no independent wheel viewport. Three wheel-up events at the caret leave Prompt 35 visible.",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .owned_transcript()
        .script(include_str!("fixtures/transcript_view_1/history40.jsonl"))
        .open();
    let draft = (1..=35)
        .map(|n| format!("Prompt {n:02} 界"))
        .collect::<Vec<_>>()
        .join("\n");
    let browse = |s: &mut Screen, last: &str, history| {
        let (x, y) = (14, s.row_of(last));
        s.assert_cursor(x, y);
        for _ in 0..3 {
            s.wheel_up(x as u16, y as u16, 1);
        }
        s.assert_absent(last);
        s.assert_row(history, "  History 40");
        s.assert_cursor_hidden();
    };
    s.keys("ctrl+g");
    s.answer_editor("", &draft);
    let history = s.row_of("History 40");
    browse(&mut s, "Prompt 35", history);
    s.keys("left");
    s.assert_cursor(12, s.row_of("Prompt 35"));
    s.keys("right");
    browse(&mut s, "Prompt 35", history);
    let edited = draft.replace("Prompt", "Edited");
    s.keys("ctrl+g");
    s.answer_editor(&draft, &edited);
    browse(&mut s, "Edited 35", history);
    s.paste(" pasted here");
    s.assert_contains("Edited 35 界 pasted here");
    s.assert_row(history, "  History 40");
    s.resize(40, 10);
    s.assert_contains("History 40");
    s.assert_contains("Edited 35 界 pasted here");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text(&format!("{edited} pasted here")));
}

// ORACLE app/owned_transcript_tests.rs:2055 composer_thumbnails_follow_draft_deletion_and_resize kind=snap owner=chat-live snap=composer_thumbnail_strip
#[test]
fn composer_thumbnails_follow_draft_deletion_and_resize() {
    let clock = |millis| format!("{{\"step\":\"clock\",\"millis\":{millis}}}\n");
    for owned in [false, true] {
        let portrait = kit::image_file_sized("composer-portrait.png", 4, 400);
        let panorama = kit::image_file_sized("composer-panorama.png", 400, 1);
        let open = Screen::build(Owner::ChatLive).size(20, 24).kitty();
        let mut s = if owned {
            open.owned_transcript()
        } else {
            open.inline(&[])
        }
        .open();
        s.attach_image_path(&portrait)
            .keys("home")
            .attach_image_path(&panorama)
            .keys("end")
            .attach_image_path(&panorama);
        s.assert_thumbnails(20, &[(6, 1), (1, 3)]);
        s.assert_contains("+1");

        s.resize(60, 24);
        s.assert_thumbnails(60, &[(6, 1), (1, 3), (6, 1)]);
        s.assert_absent("+1");
        s.keys("home delete");
        s.assert_thumbnails(60, &[(1, 3), (6, 1)]);

        let earlier = s.thumbnail_bytes(0);
        kit::rewrite_image_file(&portrait, 4, 400);
        s.feed(&clock(2_000_000_000_000_i64));
        s.assert_thumbnails(60, &[(1, 3), (6, 1)]);
        s.assert_thumbnail_changed(0, earlier);

        s.resize(60, 5);
        s.assert_thumbnails(60, &[]);
        s.assert_contains("[Image #1][Image #2]");
        s.keys("home delete");
        s.assert_contains("[Image #1]");
        s.assert_absent("[Image #2]");
        s.resize(60, 24);
        s.assert_thumbnails(60, &[(6, 1)]);

        let open = Screen::build(Owner::ChatLive).size(60, 24);
        let mut s = if owned {
            open.owned_transcript()
        } else {
            open.inline(&[])
        }
        .open();
        s.attach_image_path(&portrait).attach_image_path(&panorama);
        s.assert_thumbnails(60, &[]);
        s.assert_contains("[Image #1][Image #2]");
        s.keys("home delete");
        s.assert_absent("[Image #2]");
        kit::remove_artifact_file(&portrait);
        kit::remove_artifact_file(&panorama);
    }
}

// ORACLE app/owned_transcript_tests.rs:2158 fullscreen_composer_remote_images_leave_an_editable_prompt_row kind=snap owner=chat-live snap=fullscreen_prompt_remote_images
#[test]
fn fullscreen_composer_remote_images_leave_an_editable_prompt_row() {
    let urls: Vec<_> = (1..=12)
        .map(|n| format!("https://example.com/image-{n}.png"))
        .collect();
    let all: Vec<_> = urls.iter().map(String::as_str).collect();
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .owned_transcript()
        .kitty()
        .live()
        .remote_images(&all)
        .open();
    s.live_tail("History stays visible").commit_live_tail();
    s.keys("ctrl+g");
    s.answer_editor("", "Edit this prompt");
    let history = s.row_of("History stays visible");
    let prompt = s.row_of("Edit this prompt");
    s.assert_screen_row_then(
        "  [Image #1]",
        &["  [Image #2]", "  [Image #3]", "  [Image #4]"],
    );
    s.assert_absent("[Image #5]");
    s.keys("home up");
    s.assert_screen_row_then(
        "  [Image #9]",
        &["  [Image #10]", "  [Image #11]", "  [Image #12]"],
    );
    s.assert_text_style("[Image #12]", "reversed");
    s.assert_row(prompt, "› Edit this prompt");
    s.keys("up");
    s.assert_absent("[Image #12]");
    s.assert_text_style("[Image #11]", "reversed");
    s.keys("delete");
    s.assert_text_style("[Image #11]", "reversed");
    s.keys("down");
    s.assert_screen_row_then(
        "  [Image #1]",
        &["  [Image #2]", "  [Image #3]", "  [Image #4]"],
    );
    s.assert_cursor(2, prompt);
    s.assert_row_of("Edit this prompt", &["›"], &[]);
    s.assert_row(history, "• History stays visible");
    s.keys("end");
    s.type_text(" now");
    let local = kit::image_file_sized("fullscreen-composer-remote-and-local.png", 400, 1);
    s.attach_image_path(&local);
    s.assert_contains("Edit this prompt now[Image #12]");
    s.assert_thumbnails(60, &[]);
    s.resize(60, 36);
    s.assert_thumbnails(60, &[(6, 1)]);
    s.assert_has_row("  [Image #11]");
    s.resize(60, 12);
    s.assert_thumbnails(60, &[]);
    s.assert_row(history, "• History stays visible");
    s.assert_absent("  [Image #5]");
    s.keys("backspace");
    kit::remove_artifact_file(&local);
    let remaining: Vec<_> = all
        .into_iter()
        .enumerate()
        .filter_map(|(index, url)| (index != 10).then_some(url))
        .collect();
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("Edit this prompt now").remote_images(&remaining));
}

// ORACLE app/owned_transcript_tests.rs:2212 footer_selection_defers_warning_clicks_and_preserves_the_draft kind=snap owner=chat-live snap=fullscreen_footer_selection
#[test]
fn footer_selection_defers_warning_clicks_and_preserves_the_draft() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.type_text("draft stays here");
    s.assert_contains("draft stays here");
}

// ---- oracle: transcript_view/activity_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/activity_tests.rs:23 visible_updates_and_empty_tail_cells_do_not_report_activity kind=ui owner=chat-live
#[test]
fn visible_updates_and_empty_tail_cells_do_not_report_activity() {
    kit::pending(
        Owner::ChatLive,
        "passes only while G63 keeps the fixture's rows from scrolling (MarkdownView joins the single newlines into one paragraph); with the rows broken the scrolled-up reader has the new output below its viewport (the port shows New activity) and the test expects the text on the screen",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.wheel_up(40, 10, 3);
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.assert_contains("new output");
}

// ORACLE transcript_view/activity_tests.rs:40 unchanged_live_revisions_do_not_report_activity_but_hidden_text_does kind=ui owner=chat-live
#[test]
fn unchanged_live_revisions_do_not_report_activity_but_hidden_text_does() {
    kit::na(
        Owner::ChatLive,
        "asserts the unseen_activity flag after sync_live_tail revisions 1, 2, 3, not a drawn cell; the footer it drives is not asserted by this test",
    );
}

// ORACLE transcript_view/activity_tests.rs:65 escape_closes_selection_and_search_before_returning_to_latest kind=ui owner=chat-live
#[test]
fn escape_closes_selection_and_search_before_returning_to_latest() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.wheel_up(40, 10, 1);
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.keys("esc esc esc");
    s.assert_absent("new output ");
}

// ---- oracle: transcript_view/bookmark_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/bookmark_tests.rs:8 cancel_restores_both_positions_after_resize_and_pagination kind=ui owner=chat-live
#[test]
fn cancel_restores_both_positions_after_resize_and_pagination() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .replay(Vec::new())
        .open();
    s.partial_history(20);
    s.type_text("bookmarked draft");
    s.keys("pageup pageup");
    s.assert_absent("row 19");
    let before = s.shot();
    s.keys("ctrl+t");
    s.assert_composer("bookmarked draft");
    s.resize(36, 10);
    s.join_older_page(&["older page"], false);
    s.assert_page_requests(2);
    s.live_tail("new response");
    s.resize(80, 16);
    s.keys("ctrl+t");
    s.assert_same_rows_in(&before, 0..7);
    s.assert_composer("bookmarked draft");
    s.assert_absent("new response");
}

// ORACLE transcript_view/bookmark_tests.rs:38 following_origin_returns_to_new_output_in_original_presentation kind=ui owner=chat-live
#[test]
fn following_origin_returns_to_new_output_in_original_presentation() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/transcript_view_1/prompts_long.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_absent("Back to bottom");
    s.keys("esc left");
    s.assert_text_style("first prompt", "reversed");
    s.live_tail("new output");
    s.assert_absent("new output");
    s.keys("esc");
    s.assert_contains("new output");
    s.assert_absent("Back to bottom");
    s.assert_absent("first prompt");
}

// ORACLE transcript_view/bookmark_tests.rs:57 cancel_restores_a_group_replaced_during_browsing kind=ui owner=chat-live
#[test]
fn cancel_restores_a_group_replaced_during_browsing() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 2)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("esc esc left");
    let before = s.shot();
    s.feed(include_str!("fixtures/transcript_view_1/follow_up.jsonl"));
    s.keys("esc");
    s.assert_same_rows(&before);
}

// ---- oracle: transcript_view/composer_gap_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/composer_gap_tests.rs:14 pending_copy_feedback_survives_expiry_and_selection_replacement kind=snap owner=chat-live snap=pending_copy_feedback_survives_expiry_and_selection_replacement
#[test]
fn pending_copy_feedback_survives_expiry_and_selection_replacement() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.selection_copy_policy(false, true);
    s.double_click_text("select");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["select"]);
    s.answer_copy_pending(1);
    s.wait(10_000);
    s.assert_contains("Copying 6 chars");
    s.answer_copy(Err("setup timed out"));
    s.assert_contains("Copy failed");
    s.assert_text_style("select", "reversed");

    s.keys("ctrl+c");
    s.assert_copies_asked(&["select", "select"]);
    s.answer_copy_pending(2);
    let (x, y) = at(&s, "transcript");
    s.wait(500);
    s.double_click(x, y);
    s.assert_text_style("transcript", "reversed");
    s.answer_copy(Ok(()));
    s.assert_contains("Copied 6 chars");
    s.assert_text_style("transcript", "reversed");
}

// ORACLE transcript_view/composer_gap_tests.rs:85 copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery kind=snap owner=chat-live snap=copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery
#[test]
fn copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.assert_contains("select this transcript");
}

// ORACLE transcript_view/composer_gap_tests.rs:118 feedback_releases_navigation_targets_and_expiry_restores_them kind=ui owner=chat-live
#[test]
fn feedback_releases_navigation_targets_and_expiry_restores_them() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.selection_copy_policy(false, false);
    s.keys("pageup");
    let (control_x, control_y) = at(&s, "Back to bottom");
    s.assert_absent("row 29");
    let (x, y) = at(&s, "row 16");
    s.drag(x, y, x + 5, y);
    s.keys("ctrl+c");
    s.assert_copies_asked(&["row 16"]);
    s.answer_copy_unconfirmed();
    s.assert_absent("Back to bottom");
    s.click(control_x, control_y);
    s.assert_absent("row 29");
    s.wait(5_500);
    s.assert_contains("Back to bottom");
    s.click_text("Back to bottom");
    s.assert_contains("row 29");
    s.assert_absent("Back to bottom");
}

// ORACLE transcript_view/composer_gap_tests.rs:149 tip_links_follow_alignment_and_release_stale_targets kind=ui owner=chat-live
#[test]
fn tip_links_follow_alignment_and_release_stale_targets() {
    const DOCS: &str = "https://example.com/docs";
    const TIP: (&str, &str, &str) = ("Tip: ", "文档", DOCS);
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 14)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.selection_copy_policy(false, false);
    let set_tip = s.composer_gap_hint(TIP);
    let row = s.row_of("Tip:");
    let y = u16::try_from(row).unwrap();
    s.assert_row(row, "                    Tip: 文档");
    s.assert_span_linked("Tip:", 5, 4, DOCS, true);
    let mut opened = Vec::new();
    for x in 25..29 {
        s.ctrl_down(x, y);
        s.assert_opened_urls(&opened);
        s.ctrl_up(x, y);
        opened.push(DOCS);
        s.assert_opened_urls(&opened);
        s.super_click(x, y);
        opened.push(DOCS);
        s.assert_opened_urls(&opened);
    }
    s.click(26, y);
    s.ctrl_up(26, y);
    s.ctrl_click(24, y);
    s.ctrl_click(29, y);
    s.ctrl_click(26, y - 1);
    s.ctrl_down(26, y).mouse_drag(24, y).mouse_drag(26, y);
    s.ctrl_up(26, y);
    for moved_to in [26, 2] {
        s.ctrl_down(26, y).pointer_move(moved_to, y);
        s.ctrl_up(26, y);
    }
    s.assert_opened_urls(&opened);
    s.assert_copies_asked(&[]);

    for tip in [None, Some(("Other tip", "", ""))] {
        s.ctrl_down(26, y);
        set_tip(tip);
        s.wait(1);
        s.assert_link_targets(&[]);
        s.ctrl_up(26, y);
        s.ctrl_click(26, y);
        s.assert_opened_urls(&opened);
        set_tip(Some(TIP));
        s.wait(1);
        s.assert_link("文档", DOCS);
    }
    s.resize(8, 14);
    s.assert_link_targets(&[]);
    s.resize(40, 14);
    s.assert_row(s.row_of("Tip:"), "                              Tip: 文档");
    s.ctrl_click(26, y);
    s.assert_opened_urls(&opened);
    s.resize(30, 14);

    s.double_click_text("select this transcript");
    s.keys("ctrl+c");
    s.assert_copy_asked("select");
    s.keys("esc");
    s.assert_link("文档", DOCS);
    let y = u16::try_from(s.row_of("Tip:")).unwrap();
    s.ctrl_down(26, y);
    s.answer_copy(Ok(()));
    s.assert_contains("Copied 6 chars");
    s.assert_link_targets(&[]);
    s.wait(5_500);
    s.assert_link("文档", DOCS);
    s.ctrl_up(26, y);
    s.assert_opened_urls(&opened);
    s.ctrl_click(26, y);
    opened.push(DOCS);
    s.assert_opened_urls(&opened);
}
