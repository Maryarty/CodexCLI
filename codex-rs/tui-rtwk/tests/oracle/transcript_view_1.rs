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
        "G63: the fixture's rows are joined by single newlines, which MarkdownView joins into one paragraph (the oracle breaks the line), so the rows never scroll; the kit also has no bytes for shift+enter (no kitty keyboard protocol) and the 40-wide pass opens the detailed view (ctrl+t: the pager, task pager). The hints and the Enter back are built: tests/pages/transcript_follow.rs",
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
        "G63: the fixture's rows are joined by single newlines, which MarkdownView joins into one paragraph (the oracle breaks the line), so the rows never scroll; the snapshot's plain-label sections ('before', 'after') do not parse in the kit (no frame `before`). The 'New activity · esc latest' hint and the Esc back are built: tests/pages/transcript_follow.rs",
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
    kit::pending(
        Owner::ChatLive,
        "G-new-external-writer-state: Esc and Left leave the read-only chat for the command center (built); the snapshots differ by what the kit cannot stage: the oracle's list is still `Loading tasks…` (the foreign_writer seed lists the thread, which the chat needs to open it) and the offline variant needs a lost connection (`Reconnecting…`, stale list), for which the kit has no step",
    );
    for (detailed, scrolled, offline) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (false, false, true),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(96, 24)
            .foreign_writer("thread-foreign")
            .live()
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
    kit::pending(
        Owner::ChatLive,
        "G-new-offline-composer-screen: no offline state (reconnect pause) on the owned transcript with a Find query: the 'Find: needle' query row, the hidden '⌃c quit' hint, and ctrl+c closing Find before quitting are not built; the quit exit is not visible",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_1/needle.jsonl"))
        .open();
    s.type_text("offline draft");
    s.keys("ctrl+t /");
    s.paste("needle");
    s.assert_has_row("Find: needle");
    s.assert_absent("⌃c quit");
    s.keys("ctrl+c");
    s.assert_contains("offline draft");
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
    kit::pending(
        Owner::ChatLive,
        "no Esc-Esc prompt browsing on the owned transcript at 80x16, no keymap override (open_transcript on ctrl+x h, editor move_left unbound, move_line_start on left h; .keymap is pending in the kit) and no editing explanation row; the prompt_navigation frame is the target",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!("fixtures/transcript_view_1/three.jsonl"))
        .open();
    s.keys("esc esc");
    s.keys("ctrl+x h");
    s.keys("ctrl+x h");
    s.keys("left left right");
    s.assert_snap("prompt_navigation");
}

// ORACLE app/owned_transcript_tests.rs:1647 find_refreshes_live_details_before_searching_the_first_query kind=snap owner=chat-live snap=compact_overlay_finds_hidden_live_text
#[test]
fn find_refreshes_live_details_before_searching_the_first_query() {
    kit::pending(
        Owner::ChatLive,
        "f-find-tool-text-2 (G-new-find-room-text, folded turn details): with `collapse_turn_details` the running command is the fold row `▸ Running printf visible …` (`FoldRow`, `turn_details.rs` `Kind::Tools`), whose nodes are not the item's, so Find gives no text for it and says `No matches` (the oracle reveals the folded call, `search_presentation.rs:104`); the footer `enter accept` and `printf needle` are the first half, the second half (the Ctrl+T overlay in the compact presentation with its own Find, snapshot `compact_overlay_finds_hidden_live_text`) needs the sheet over the compact presentation",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/transcript_view_1/running_command.jsonl"
        ))
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
        "no fullscreen composer over a 40-row history at 60x24 that follows the caret: the external-editor edit (apply_external_edit to 'Edited') and the same-length caret restore have no kit call, and the multi-frame snapshot fullscreen_prompt_wheel is not compared; the wheel over the composer, the Left/Right caret and the paste at the caret are ported",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!("fixtures/transcript_view_1/history40.jsonl"))
        .open();
    let draft = (1..=35)
        .map(|n| format!("Prompt {n:02} 界"))
        .collect::<Vec<_>>()
        .join("\n");
    s.paste(&draft);
    let (x, y) = at(&s, "Prompt 35");
    s.wheel_up(x, y, 3);
    s.assert_absent("Prompt 35");
    s.assert_contains("History 40");
    s.keys("left");
    s.assert_contains("Prompt 35");
    s.keys("right");
    s.paste(" pasted here");
    s.assert_contains("pasted here");
}

// ORACLE app/owned_transcript_tests.rs:2055 composer_thumbnails_follow_draft_deletion_and_resize kind=snap owner=chat-live snap=composer_thumbnail_strip
#[test]
fn composer_thumbnails_follow_draft_deletion_and_resize() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment seed (attach_image of a portrait and a banner, Home/End/Delete over the draft) and no kitty image placements in the kit: the thumbnail strip at 20x24 and 60x24, the '+1' count and the '[Image #1][Image #2]' text fallback at insufficient height or without terminal support are not built; the 6-row strip snapshot composer_thumbnail_strip needs the placement row, so it is not compared yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.assert_contains("+1");
}

// ORACLE app/owned_transcript_tests.rs:2158 fullscreen_composer_remote_images_leave_an_editable_prompt_row kind=snap owner=chat-live snap=fullscreen_prompt_remote_images
#[test]
fn fullscreen_composer_remote_images_leave_an_editable_prompt_row() {
    kit::pending(
        Owner::ChatLive,
        "no remote image seed (set_remote_image_urls of twelve urls) at 60x12: the '[Image #N]' rows in the composer and the Home/Up/Delete/Down editing across them are not built; the multi-frame snapshot fullscreen_prompt_remote_images is not compared",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.type_text("Edit this prompt");
    s.assert_contains("Edit this prompt");
    s.keys("home up");
    s.assert_contains("[Image #12]");
    s.keys("up");
    s.assert_contains("[Image #11]");
    s.keys("delete");
    s.assert_contains("[Image #11]");
    s.keys("down");
    s.assert_contains("Edit this prompt");
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
    kit::pending(
        Owner::ChatLive,
        "no browsing bookmark through the kit: jump_to_entry to the first prompt, a detailed toggle, a resize to 12x4, an older page inserted and restore_bookmark have no key path to the 30x3 view; the visible part is that the restored rows equal the rows before",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("esc esc left left");
    let before = s.shot();
    s.keys("ctrl+t");
    s.resize(12, 4);
    s.keys("esc");
    s.resize(30, 3);
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/bookmark_tests.rs:38 following_origin_returns_to_new_output_in_original_presentation kind=ui owner=chat-live
#[test]
fn following_origin_returns_to_new_output_in_original_presentation() {
    kit::pending(
        Owner::ChatLive,
        "no detailed origin with a follow-state restore on the chat screen: the bookmark taken while detailed and the restore that returns to following in the original (compact) presentation have no key path; the visible part is the 'new output' row after the restore",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 2)
        .script(include_str!("fixtures/transcript_view_1/two_prompts.jsonl"))
        .open();
    s.keys("ctrl+t");
    s.keys("esc");
    s.feed(include_str!("fixtures/transcript_view_1/new_output.jsonl"));
    s.keys("esc");
    s.assert_contains("new output");
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
    kit::pending(
        Owner::ChatLive,
        "no copy feedback in the composer gap through the kit: a copy result (Pending, Busy, setup failed) needs the clipboard seam, and the gap row frames 'Pending after ten seconds' and 'Setup failed' are not drawn by the kit yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.drag(0, 0, 5, 0);
    s.assert_contains("selected text");
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
    kit::pending(
        Owner::ChatLive,
        "G63: the fixture's rows are joined by single newlines, which MarkdownView joins into one paragraph (the oracle breaks the line), so the rows never scroll; with the rows broken (hard breaks) the test passes",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!("fixtures/transcript_view_1/rows_30.jsonl"))
        .open();
    s.wheel_up(40, 10, 2);
    s.assert_contains("Back to bottom");
}

// ORACLE transcript_view/composer_gap_tests.rs:149 tip_links_follow_alignment_and_release_stale_targets kind=ui owner=chat-live
#[test]
fn tip_links_follow_alignment_and_release_stale_targets() {
    kit::pending(
        Owner::ChatLive,
        "no tip line with a hyperlink (Tip: plus the link text 文档 to https://example.com/docs) in the composer gap, and no ctrl-click (the kit's click has no modifiers): the link cell, the link target and the stale-target release after a feedback expiry are not built",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(30, 4)
        .script(include_str!("fixtures/transcript_view_1/select.jsonl"))
        .open();
    s.assert_link("文档", "https://example.com/docs");
}
