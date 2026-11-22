//! Shard `pane_footer_1` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The braille starfield dots of the oracle's sparkle (`sparkle_field.rs`, `DOTS`).
const DOTS: [&str; 8] = ["⠁", "⠂", "⠄", "⠈", "⠐", "⠠", "⡀", "⢀"];

/// No starfield dot is on the screen (each glyph is absent).
fn assert_no_dots(s: &Screen) {
    for dot in DOTS {
        s.assert_absent(dot);
    }
}

/// The first row that contains `text`: where a footer or a status row of the oracle sits.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or(usize::MAX)
}

const FIND_REASON: &str = "the transcript Find footer (`Find: needle` above the composer, its cursor, the copy flash) is not built: the chat footer draws only the history-search row, and no key or seed opens a find footer";
const SPARKLE_REASON: &str = "the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) is not built: the chat draws no dots and no seed marks a fresh task";
const HELP_REASON: &str = "the help itself is built (the `?` overlay above the composer); what these two wait for is the composer sparkle under it (braille dots in the placeholder, step 7 of chunk-status: no sparkle yet)";
const HELP_STATUS: &str = include_str!("fixtures/pane_footer_1/help_status.jsonl");
const EXTENSION_REASON: &str = "the status line of this test is a ui_tree `extension` (a `FooterContent` that only the oracle's tests and previews build; no native item makes such a row): the person decides (brief section 7), the behaviour it checks is re-expressed with native items in tests/pages/chat_status.rs";
const PROVIDER_REASON: &str = "a provider status (the bold `codex-internal`, an extension row) beside Plan mode: no collaboration-mode fact in StatusFacts (core/src/screens/chat.rs, chunk-live's) and no extension status";
const THUMBNAIL_REASON: &str = "the composer's image thumbnails between the footer rows and the extra status (composer chunk) over an extension status row";
const LINK_REASON: &str = "the native status carries a link (`PR #42`): no pull-request fact (git summary in StatusFacts, chunk-live's; the host probe is a need), and Plan mode needs the collaboration fact";
const AGENT_REASON: &str = "the viewed agent's label joins the status (`build ready · frontend`): no agent-label fact in StatusFacts (chunk-live's)";
const ACTIVITY_REASON: &str = "the passive activity hint (`ctrl+o t/f4 inspect activity`) is not in the footer: the activity rows are step 5 of chunk-status";
const MODE_REASON: &str = "the Plan mode indicator and its cycle hint (and the side-conversation label): no collaboration-mode / side facts in StatusFacts (chunk-live's); the footer's fit for them is done (footer/hints.rs unit tests)";
const GOAL_REASON: &str = "the goal indicator (`Pursuing goal (40K / 50K)`): no goal fact (ThreadPart.goal from thread/goal/updated, chunk-live's)";
const QUEUE_REASON: &str = "the status under hints, a flash and the queued messages (`persistent_status_and_messages`): the queue preview of the oracle's layout and the footer flash are steps 2/5 of chunk-status, not yet built";
const POPUP_REASON: &str = "the status stays under the composer while the popup is open (page test), but the popup's own rows (`/model` descriptions, the selection marker) are the popups chunk's widget and differ from the oracle's Buffer snapshot";
const WARN_REASON: &str = "the warning notice (`⚠ N warnings · /warnings to view`) is not built in the composer footer: no seed sets a warning count";
const EFFORT_REASON: &str = "the reasoning-effort footer (the `U L T R A` ignition, the status row, the Ultra prompt accent) is not built: no seed sets a reasoning effort";

// ---- oracle: bottom_pane/chat_composer/footer_state_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:62 retained_command_suggestions_yield_paint_and_cursor_to_find kind=snap owner=ui-pane snap=find_hides_retained_command_suggestions
#[test]
fn retained_command_suggestions_yield_paint_and_cursor_to_find() {
    kit::pending(Owner::UiPane, FIND_REASON);
    // the oracle types `/` under a Find footer (`Find: needle`, cursor at column 8) and
    // renders it at 60x8, 24x8 and 12x3; the footer is set by the test, no key reaches it
    let mut s = Screen::build(Owner::UiPane).size(60, 8).live().open();
    s.type_text("/");
    s.assert_contains("Find: needle");
    s.assert_contains("Enter next · Esc close");
    // the snapshot holds the three frames; the kit compares one frame of the snapshot
    s.assert_snap("find_hides_retained_command_suggestions");
    s.resize(24, 8);
    s.resize(12, 3);
}

// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:110 copying_from_find_preserves_the_query_above_selection_feedback kind=snap owner=ui-pane snap=transcript_find_copy_footer
#[test]
fn copying_from_find_preserves_the_query_above_selection_feedback() {
    kit::pending(Owner::UiPane, FIND_REASON);
    // the Find footer (`Find: needle`, `Ctrl+C copy · Esc clear selection`) and the 3 s flash
    // `Copied selection to clipboard` are set by the test; the flash keeps the query above it
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("Find: needle");
    s.assert_contains("Copied selection to clipboard");
    s.assert_snap("transcript_find_copy_footer");
}

// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:134 interactive_query_keeps_focus_over_underlying_hints_but_empty_override_wins kind=ui owner=ui-pane
#[test]
fn interactive_query_keeps_focus_over_underlying_hints_but_empty_override_wins() {
    kit::pending(Owner::UiPane, FIND_REASON);
    // the interactive Find footer (cursor at 12) keeps the frame the same when the test sets a
    // hint override (`ctrl+c quit`); an empty override shows the composer's own footer again
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    let expected = s.shot();
    s.assert_contains("Find: needle");
    s.assert_same_rows(&expected);
}

// ---- oracle: bottom_pane/chat_composer/sparkle_tests.rs (19 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:114 sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor kind=snap owner=ui-pane snap=sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor
#[test]
fn sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a fresh Astra task: the dots fill the free cells of the composer, never the placeholder or
    // the cursor; the first typed character clears them (the oracle draws at 40 and 80 columns)
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_snap("sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor");
    let plain = s.shot();
    s.wait(2_000);
    s.assert_rows_differ(&plain);
    s.type_text("x");
    assert_no_dots(&s);
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:176 sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds kind=snap owner=ui-pane snap=sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds
#[test]
fn sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a fresh Astra task: the last frame before the fade (14 s), the fading frame (14.5 s) and
    // the finished frame (15 s); the fade adds no dot, and no dot is left at the deadline
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.wait(14_000);
    s.wait(500);
    s.wait(500);
    assert_no_dots(&s);
    s.assert_snap("sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:227 the_first_buffered_character_and_empty_escape_clear_visible_stars kind=ui owner=ui-pane
#[test]
fn the_first_buffered_character_and_empty_escape_clear_visible_stars() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // on a fresh Astra task each of these keys clears the visible dots: `x`, `?`, Esc, Left
    for key in ["x", "?", "esc", "left"] {
        let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
        s.keys(key);
        assert_no_dots(&s);
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:256 shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not kind=snap owner=ui-pane snap=shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not
#[test]
fn shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not() {
    kit::pending(Owner::UiPane, HELP_REASON);
    // an unused Astra sparkle: `?` opens the shortcut help (dots hidden), `?` again closes it;
    // the literal `?` of a paste does not end the sparkle; then `/model` selects Astra
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("?");
    s.assert_snap("shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not");
    s.keys("?");
    s.paste("?");
    s.assert_contains("?");
    s.keys("backspace");
    s.type_text("/model");
    s.keys("enter");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:372 shortcut_help_hides_visible_sparkles_without_restarting_their_deadline kind=snap owner=ui-pane snap=shortcut_help_hides_visible_sparkles_without_restarting_their_deadline
#[test]
fn shortcut_help_hides_visible_sparkles_without_restarting_their_deadline() {
    kit::pending(Owner::UiPane, HELP_REASON);
    // a visible Astra sparkle: `?` at 4 s hides it and opens the help; the 15 s deadline is not
    // restarted, so the dots stay hidden after the help closes at 15 s
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.wait(4_000);
    s.keys("?");
    assert_no_dots(&s);
    s.wait(11_000);
    s.keys("?");
    assert_no_dots(&s);
    s.assert_snap("shortcut_help_hides_visible_sparkles_without_restarting_their_deadline");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:459 disconnected_sparkle_edits_are_tracked_between_renders kind=snap owner=ui-pane snap=disconnected_sparkle_edits_are_tracked_between_renders
#[test]
fn disconnected_sparkle_edits_are_tracked_between_renders() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // the oracle edits a disconnected composer without rendering (restricted keys) and erases:
    // the dots return only for an untouched draft; `gpt-5.5` shows none at all
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("x");
    s.keys("backspace");
    s.assert_snap("disconnected_sparkle_edits_are_tracked_between_renders");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:529 a_fresh_astra_response_waits_for_the_pending_command_to_leave kind=snap owner=ui-pane snap=a_fresh_astra_response_waits_for_the_pending_command_to_leave
#[test]
fn a_fresh_astra_response_waits_for_the_pending_command_to_leave() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // `/status` typed on a fresh Astra task shows no dots while it is pending; submitting it
    // (or erasing it) is the completion the oracle snapshots
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("/status");
    assert_no_dots(&s);
    s.keys("enter");
    assert_no_dots(&s);
    s.assert_snap("a_fresh_astra_response_waits_for_the_pending_command_to_leave");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:574 an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared kind=ui owner=ui-pane
#[test]
fn an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // an attached image (the test attaches it; no key does) ends the sparkle; ctrl+c clears the
    // composer and the sparkle stays ended after Astra is selected
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("ctrl+c");
    assert_no_dots(&s);
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:591 history_search_acceptance_and_cancellation_update_sparkles_before_rendering kind=snap owner=ui-pane snap=history_search_acceptance_and_cancellation_update_sparkles_before_rendering
#[test]
fn history_search_acceptance_and_cancellation_update_sparkles_before_rendering() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // history `git status` (the test records it; no seed does): ctrl+r then `git`; enter accepts
    // (the sparkle ends), esc and ctrl+c cancel (it stays)
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("ctrl+r");
    s.type_text("git");
    s.keys("enter");
    s.assert_snap("history_search_acceptance_and_cancellation_update_sparkles_before_rendering");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:637 canceled_history_search_restores_visible_stars_only_before_the_original_deadline kind=snap owner=ui-pane snap=canceled_history_search_restores_visible_stars_only_before_the_original_deadline
#[test]
fn canceled_history_search_restores_visible_stars_only_before_the_original_deadline() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // ctrl+r, `st` then a pasted `ored` over `stored draft`; esc at 4 s restores the dots, and
    // when the 15 s deadline passes while the search is open they do not come back
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.wait(4_000);
    s.keys("ctrl+r");
    s.type_text("st");
    s.paste("ored");
    assert_no_dots(&s);
    s.keys("esc");
    s.assert_snap(
        "canceled_history_search_restores_visible_stars_only_before_the_original_deadline",
    );
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:701 history_search_preserves_a_held_sparkle_command_without_reclassifying_it kind=snap owner=ui-pane snap=history_search_preserves_a_held_sparkle_command_without_reclassifying_it
#[test]
fn history_search_preserves_a_held_sparkle_command_without_reclassifying_it() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a held `/` (paste burst) then ctrl+r and a pasted `history`: the command is kept, not
    // reclassified; esc (or ctrl+c) then backspace leaves the `/`, and `/model` selects Astra
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("/");
    s.keys("ctrl+r");
    s.paste("history");
    assert_no_dots(&s);
    s.keys("esc");
    s.keys("backspace");
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("history_search_preserves_a_held_sparkle_command_without_reclassifying_it");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:762 cancelling_history_search_preserves_a_typed_command_despite_attachment_previews kind=ui owner=ui-pane
#[test]
fn cancelling_history_search_preserves_a_typed_command_despite_attachment_previews() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // `/status` typed, ctrl+r, `history` (an entry with an image: the test records it); esc
    // restores `/status` with no previews, and enter runs the Status command
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("/status");
    s.keys("ctrl+r");
    s.type_text("history");
    s.keys("esc");
    s.assert_contains("/status");
    s.keys("enter");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:791 fresh_draft_replacement_ends_search_and_dismisses_sparkles kind=ui owner=ui-pane
#[test]
fn fresh_draft_replacement_ends_search_and_dismisses_sparkles() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // ctrl+r opens the search; the test replaces the draft with `external draft` (no key does),
    // which ends the search; esc then leaves `external draft` in the composer
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("ctrl+r");
    s.keys("esc");
    s.assert_contains("external draft");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:806 arbitrary_commands_preserve_a_never_used_fresh_opportunity kind=ui owner=ui-pane
#[test]
fn arbitrary_commands_preserve_a_never_used_fresh_opportunity() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a fresh Astra task: `/status`, `/diff`, `/pwd` run and keep the opportunity; `/model`
    // (or `/mo` completed by tab) selects Astra and the sparkle shows again
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    for command in ["/status", "/diff", "/pwd"] {
        s.type_text(command);
        s.keys("enter");
        assert_no_dots(&s);
    }
    s.type_text("/mo");
    s.keys("tab");
    s.keys("enter");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:855 canceled_commands_remain_eligible_but_inserted_content_and_ordinary_paste_do_not kind=snap owner=ui-pane snap=cleared_recovered_draft_keeps_sparkle_dismissed
#[test]
fn canceled_commands_remain_eligible_but_inserted_content_and_ordinary_paste_do_not() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a canceled `/sta` (four backspaces) keeps the sparkle eligible for Astra; an inserted
    // `/mention` (enter, then `@`) and an ordinary paste do not; the recovered draft (the test
    // appends it, no key does) keeps the sparkle dismissed after clearing
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("/sta");
    s.keys("backspace backspace backspace backspace");
    s.type_text("/mention");
    s.keys("enter");
    s.type_text("@");
    s.assert_snap("cleared_recovered_draft_keeps_sparkle_dismissed");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:913 review_regression_canceling_a_held_slash_restores_sparkle_command_tracking kind=snap owner=ui-pane snap=review_regression_canceling_a_held_slash_restores_sparkle_command_tracking
#[test]
fn review_regression_canceling_a_held_slash_restores_sparkle_command_tracking() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a held `/` canceled at once by backspace restores the sparkle tracking; `/model` then
    // selects Astra and the dots show (for gpt-6-astra, not for gpt-5.5)
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("/");
    s.keys("backspace");
    s.assert_snap("review_regression_canceling_a_held_slash_restores_sparkle_command_tracking");
    s.type_text("/model");
    s.keys("enter");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:954 buffered_commands_and_read_only_inline_arguments_preserve_eligibility kind=ui owner=ui-pane
#[test]
fn buffered_commands_and_read_only_inline_arguments_preserve_eligibility() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a buffered `/status` (flushed after 1 s) runs with enter; the inline `/rename task name`
    // runs too; the test then selects Astra and the dots show again
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("/status");
    s.wait(1_000);
    s.keys("enter");
    s.type_text("/rename task name");
    s.keys("enter");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:992 commands_hide_a_started_sparkle_without_restarting_its_deadline kind=ui owner=ui-pane
#[test]
fn commands_hide_a_started_sparkle_without_restarting_its_deadline() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a started Astra sparkle: typing `/status` at 5 s hides it, enter at 6 s shows it again
    // (a command does not restart the deadline); `/diff` hides it at 15 s and enter ends it
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.wait(5_000);
    s.type_text("/status");
    assert_no_dots(&s);
    s.keys("enter");
    s.wait(1_000);
    s.type_text("/diff");
    s.wait(9_000);
    assert_no_dots(&s);
    s.keys("enter");
    s.wait(1_000);
    assert_no_dots(&s);
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:1046 focus_startup_work_and_configuration_do_not_extend_a_started_deadline kind=ui owner=ui-pane
#[test]
fn focus_startup_work_and_configuration_do_not_extend_a_started_deadline() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // a task running hides the dots; once it ends at 15 s they show; the terminal focus loss at
    // 16 s hides them and the focus back at 30 s does not restart the deadline; a narrow composer
    // (25 columns) keeps the dots off the draft
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    assert_no_dots(&s);
    s.wait(15_000);
    s.wait(1_000);
    assert_no_dots(&s);
    s.wait(14_000);
    assert_no_dots(&s);
}

// ---- oracle: bottom_pane/chat_composer/status_surface_tests.rs (26 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:27 extension_status_line_clips_links_to_footer_layout kind=ui owner=ui-pane
#[test]
fn extension_status_line_clips_links_to_footer_layout() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the status `plain · PR #42` (the PR a link to https://example.com/pr/42) clips to `PR#` at
    // 12 columns and shows `PR #42` at 40; an agent label shows only when the status is cleared
    let mut s = Screen::build(Owner::UiPane).size(40, 8).live().open();
    s.assert_contains("plain");
    s.assert_link("PR #42", "https://example.com/pr/42");
    s.assert_absent("…");
    s.resize(12, 8);
    s.assert_contains("PR#");
    s.assert_absent("…");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:131 extension_uses_the_full_width_after_resize kind=ui owner=ui-pane
#[test]
fn extension_uses_the_full_width_after_resize() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the status `Ready PR 12345` (the number is a link) on one row at 16 columns, two rows at
    // 15 in fullscreen; the warning `⚠` sits at the bottom-right corner after collaboration modes
    let mut s = Screen::build(Owner::UiPane).size(16, 6).live().open();
    s.assert_contains("Ready PR 12345");
    s.resize(15, 6);
    s.assert_contains("12345");
    s.assert_contains("⚠");
    s.resize(27, 6);
    s.assert_contains("Plan mode");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:174 provider_status_and_hints_use_separate_rows kind=snap owner=ui-pane snap=provider_status_and_hints_use_separate_rows
#[test]
fn provider_status_and_hints_use_separate_rows() {
    kit::pending(Owner::UiPane, PROVIDER_REASON);
    // the bold `codex-internal` status and the hints sit on separate rows at 84 columns, with
    // 2 warnings and Plan mode; the warning `⚠` stays on the footer row at 22 and at 4 rows
    let mut s = Screen::build(Owner::UiPane).size(84, 22).live().open();
    s.assert_snap("provider_status_and_hints_use_separate_rows");
    s.resize(84, 4);
    s.assert_contains("⚠");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:205 status_rows_respect_host_budget_and_yield_to_the_draft kind=snap owner=ui-pane snap=status_rows_respect_host_budget_and_yield_to_the_draft
#[test]
fn status_rows_respect_host_budget_and_yield_to_the_draft() {
    // the status `one two three four five` at 7 columns shows `one` and `two…` when the frame
    // caps the height; the draft `a b c` keeps its rows. The oracle's unit test hands the
    // composer `max_height = height`; the app hands it `composer_height_limit` (two thirds of the
    // frame, at least 8), so the cap (a quarter of that, two to four rows) is 2 at 7 to 14 rows
    // and 4 at 24: the page is held to what the app does.
    let mut s = Screen::build(Owner::UiPane)
        .size(7, 7)
        .script(include_str!(
            "fixtures/pane_footer_1/status_one_to_five.jsonl"
        ))
        .open();
    s.paste("a\nb\nc");
    // the oracle's snapshot (rows between `|`, a format `oracle_snap` does not read yet: the kit
    // owner's file), row by row
    for (row, text) in ["", "› a", "  b", "  c", "", "  one", "  two…"]
        .iter()
        .enumerate()
    {
        s.assert_row(row, text);
    }
    for height in [10, 14] {
        s.resize(7, height);
        s.assert_in_order(&["one", "two…"]);
        s.assert_absent("three");
    }
    s.resize(7, 24);
    s.assert_in_order(&["one", "two", "three", "four…"]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:269 sidebar_focus_keeps_dim_extension_status_rows kind=snap owner=ui-pane
#[test]
fn sidebar_focus_keeps_dim_extension_status_rows() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the draft `draft` above four status rows; unfocused (sidebar focus) the warning is hidden
    // and the rows dim, the last one shortened to `three…` (the oracle's inline snapshot)
    let mut s = Screen::build(Owner::UiPane).size(40, 12).live().open();
    s.type_text("draft");
    s.assert_in_order(&["one", "two", "three", "four"]);
    s.assert_absent("⚠");
    s.assert_in_order(&["one", "two", "three…"]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:316 thumbnails_fit_after_basic_footer_rows_and_before_extra_status kind=snap owner=ui-pane snap=thumbnails_fit_after_basic_footer_rows_and_before_extra_status
#[test]
fn thumbnails_fit_after_basic_footer_rows_and_before_extra_status() {
    kit::pending(Owner::UiPane, THUMBNAIL_REASON);
    // the attached image `preview.png` (the test attaches it; no key does) shows thumbnails
    // between the basic footer rows and the extra status, at caps 4 to 10 rows, in fullscreen
    let mut s = Screen::build(Owner::UiPane).size(32, 9).live().open();
    s.type_text("draft");
    s.assert_snap("thumbnails_fit_after_basic_footer_rows_and_before_extra_status");
    s.resize(32, 10);
    s.assert_contains("draft");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:416 extension_status_paints_with_the_current_theme_at_the_same_width kind=ui owner=ui-pane
#[test]
fn extension_status_paints_with_the_current_theme_at_the_same_width() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the status `λ` in the primary colour: red under the first syntax theme, green under the
    // second (the test switches the theme; no key does) at the same width of 24 columns
    let s = Screen::build(Owner::UiPane).size(24, 4).live().open();
    s.assert_text_style("λ", "fg=Red");
    s.assert_text_style("λ", "fg=Green");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:460 extension_status_keeps_its_rows_when_effort_changes kind=ui owner=ui-pane
#[test]
fn extension_status_keeps_its_rows_when_effort_changes() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the status `one two three` at 7 columns keeps its rows through an effort change to Max
    // (the test sets the effort; no key does): three rows fullscreen, `two…` in scrollback
    let s = Screen::build(Owner::UiPane).size(7, 12).live().open();
    s.assert_in_order(&["one", "two", "three"]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:489 native_status_wraps_links_within_the_host_budget kind=snap owner=ui-pane snap=native_status_wraps_links_within_the_host_budget
#[test]
fn native_status_wraps_links_within_the_host_budget() {
    kit::pending(Owner::UiPane, LINK_REASON);
    // the native status `لا PR #42 build green …` (a link) wraps within 18 columns: 4 rows
    // fullscreen, 2 in scrollback; with Plan mode 3 and 1 rows
    let s = Screen::build(Owner::UiPane).size(18, 20).live().open();
    s.assert_snap("native_status_wraps_links_within_the_host_budget");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:542 native_wrap_tracks_agent_label_and_content_changes_at_the_same_width kind=ui owner=ui-pane
#[test]
fn native_wrap_tracks_agent_label_and_content_changes_at_the_same_width() {
    kit::pending(Owner::UiPane, AGENT_REASON);
    // the status `build ready` at 16 columns wraps to `build ready ·` and the agent label; the
    // test changes the label and the status (no key does) and the wrap follows at the same width
    let mut s = Screen::build(Owner::UiPane).size(16, 14).live().open();
    s.assert_row(0, "build ready ·");
    s.assert_row(1, "frontend");
    // the test then sets the label `backend` and the status `tests green` (no key does)
    s.assert_row(0, "tests green ·");
    s.assert_row(1, "backend");
    s.resize(36, 14);
    s.assert_row(0, "tests green · backend");
    s.resize(16, 14);
    s.assert_row(0, "tests green ·");
    s.assert_row(1, "backend");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:571 native_effort_transition_restores_wrapping_after_its_final_frame kind=ui owner=ui-pane
#[test]
fn native_effort_transition_restores_wrapping_after_its_final_frame() {
    kit::pending(Owner::UiPane, EFFORT_REASON);
    // the status `build ready tests passed deploy queued` at 14 columns is one row during the
    // effort animation; after its final frame (10 s) it wraps again and `tests` shows
    let mut s = Screen::build(Owner::UiPane).size(14, 20).live().open();
    s.assert_contains("build ready");
    s.wait(10_000);
    s.assert_contains("tests");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:651 passive_activity_keeps_shortcuts_only_when_the_complete_hint_fits kind=ui owner=ui-pane
#[test]
fn passive_activity_keeps_shortcuts_only_when_the_complete_hint_fits() {
    kit::pending(Owner::UiPane, ACTIVITY_REASON);
    // the passive activity hint `ctrl+o t/f4 inspect activity` keeps `· ? shortcuts` at 80
    // columns and drops it at 40; an interactive footer or no shortcut key drops it at 80
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("ctrl+o t/f4 inspect activity · ? shortcuts");
    s.resize(40, 24);
    s.assert_contains("ctrl+o t/f4 inspect activity");
    s.assert_absent("? shortcuts");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:683 fullscreen_plan_indicator_keeps_the_cycle_hint_when_it_fits kind=snap owner=ui-pane snap=fullscreen_plan_cycle_hint
#[test]
fn fullscreen_plan_indicator_keeps_the_cycle_hint_when_it_fits() {
    kit::pending(Owner::UiPane, MODE_REASON);
    // Plan mode shows `Plan mode (⇧tab to cycle)` at 100 columns while idle; narrow (37) it keeps
    // `Plan mode` on the last row beside the status; running it has no ⇧tab; a `Search: verify`
    // footer keeps it too
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    s.assert_contains("Plan mode (⇧tab to cycle)");
    s.assert_snap("fullscreen_plan_cycle_hint");
    s.resize(37, 24);
    s.assert_contains("project · PR #42 · reviewer");
    s.assert_contains("Plan mode");
    s.resize(100, 24);
    s.assert_contains("Search: verify");
    s.assert_contains("Plan mode");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:771 footer_layout_in_fullscreen_and_scrollback kind=snap owner=ui-pane snap=empty snap=footer_layout_in_fullscreen_and_scrollback
#[test]
fn footer_layout_in_fullscreen_and_scrollback() {
    kit::pending(Owner::UiPane, MODE_REASON);
    // the footer rows at 120x2, 60x2 and 31x3 in fullscreen, and in scrollback at 31x4 and 31x1,
    // with Plan mode, the ide context, the vim mode and 2 warnings; a draft (`draft`) shifts them
    let mut s = Screen::build(Owner::UiPane).size(120, 5).live().open();
    s.assert_snap("footer_layout_in_fullscreen_and_scrollback");
    s.resize(31, 6);
    s.type_text("draft");
    s.assert_contains("Plan mode");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:830 active_side_action_keeps_plan_expanded_on_the_other_row kind=ui owner=ui-pane
#[test]
fn active_side_action_keeps_plan_expanded_on_the_other_row() {
    kit::pending(Owner::UiPane, MODE_REASON);
    // a side conversation: `Side from main thread · ctrl+/ to switch · ctrl+c to close` is the
    // last row and `Plan mode (⇧tab to cycle)` sits on the row above it, at 63 columns
    let s = Screen::build(Owner::UiPane).size(63, 6).live().open();
    s.assert_in_order(&[
        "Plan mode (⇧tab to cycle)",
        "Side from main thread · ctrl+/ to switch · ctrl+c to close",
    ]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:852 oversized_goal_yields_to_an_eligible_whole_hint_with_status_disabled kind=ui owner=ui-pane
#[test]
fn oversized_goal_yields_to_an_eligible_whole_hint_with_status_disabled() {
    kit::pending(Owner::UiPane, GOAL_REASON);
    // status disabled, a 100-token context window: `? for shortcuts` at 18 columns and
    // `100% context left` at 20; a paused goal (`/goal resume`) wins at 80 columns
    let mut s = Screen::build(Owner::UiPane).size(18, 4).live().open();
    s.assert_contains("? for shortcuts");
    s.resize(20, 4);
    s.assert_contains("100% context left");
    s.resize(80, 4);
    s.assert_contains("Goal paused (/goal resume)");
}

/// The oracle's snapshot of the composer alone against our rows from the help's title on: the
/// page keeps the reader's gap (one blank row, `composer_gap`) above the prompt box that the
/// snapshot lacks, and the prompt row carries the page's own placeholder (the oracle's is
/// `Ask Codex`).
fn assert_help_snap(s: &Screen, name: &str) {
    assert_help_rows(s, &kit::snap_rows(name, ""), name);
}

/// `assert_help_snap` against the rows of one frame the oracle joined of several.
fn assert_help_rows(s: &Screen, oracle: &[String], name: &str) {
    let at = |rows: &[String], text: &str| {
        rows.iter()
            .position(|row| row.contains(text))
            .unwrap_or_else(|| panic!("no row with `{text}`"))
    };
    let ours = s.rows();
    let (o_help, o_prompt) = (at(oracle, "Keyboard shortcuts"), at(oracle, "Ask Codex"));
    let (help, prompt) = (at(&ours, "Keyboard shortcuts"), at(&ours, "Ask Codex"));
    let mut got: Vec<String> = ours[help..]
        .iter()
        .map(|row| row.trim_end().to_string())
        .collect();
    // customize, the gap, the box's blank row, the prompt: the gap is two rows above the prompt
    got.remove(prompt - help - 2);
    let want = &oracle[o_help..];
    let at_prompt = o_prompt - o_help;
    assert!(
        got[at_prompt].starts_with(&want[at_prompt]),
        "{name}: the prompt row `{}` does not start with the oracle's `{}`",
        got[at_prompt],
        want[at_prompt]
    );
    got[at_prompt] = want[at_prompt].clone();
    pretty_assertions::assert_eq!(
        want.join("\n"),
        got.join("\n"),
        "{name}: ours (right) differs from the oracle (left)"
    );
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:883 shortcut_help_stays_above_the_prompt_and_bottom_status kind=snap owner=ui-pane snap=shortcut
#[test]
fn shortcut_help_stays_above_the_prompt_and_bottom_status() {
    // `?` opens the help above the prompt (`Keyboard shortcuts` above `Ask Codex`), the close row
    // `? / esc close` is the last row; the oracle renders at the height the composer wants, so
    // the narrow frame (30 rows of help) gets the rows to show them all
    let mut s = Screen::build(Owner::UiPane)
        .size(130, 24)
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    s.assert_in_order(&["Keyboard shortcuts", "Ask Codex"]);
    assert_help_snap(&s, "shortcut_help_above_wide");
    s.resize(40, 40);
    assert_help_snap(&s, "shortcut_help_above_narrow");
    s.resize(80, 12);
    s.assert_contains("? / esc close");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:929 shortcut_help_keeps_bottom_anchored_composer_status_and_cursor_stable kind=ui owner=ui-pane
#[test]
fn shortcut_help_keeps_bottom_anchored_composer_status_and_cursor_stable() {
    // at 40x12 the composer, its status and the cursor keep the same geometry while `?` opens the
    // help and `esc` closes it
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 12)
        .script(HELP_STATUS)
        .open();
    let before = s.shot();
    s.keys("?");
    s.keys("esc");
    s.assert_same_rows(&before);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:966 shortcut_help_keeps_input_and_close_hint_visible_when_height_is_tiny kind=ui owner=ui-pane
#[test]
fn shortcut_help_keeps_input_and_close_hint_visible_when_height_is_tiny() {
    // the help at 40 columns, heights 4 to 7: the last row is `? / esc close` and the input stays
    // visible; with the status on and off (the test sets it; no key does)
    // (opened taller: a resize to the size it has repaints nothing)
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 8)
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    for height in [4u16, 5, 6, 7] {
        s.resize(40, height);
        s.assert_row(usize::from(height) - 1, "  ? / esc close");
    }
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1002 shortcut_help_close_row_uses_runtime_binding_and_narrow_fallback kind=ui owner=ui-pane
#[test]
fn shortcut_help_close_row_uses_runtime_binding_and_narrow_fallback() {
    // the help close row names the runtime binding of the shortcut toggle (`⌃x ⌃⇧⌥f12`) at 80
    // columns and falls back to `esc close` at 22 (the test sets the binding; no key does)
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 8)
        .keymap(&[("composer.toggle_shortcuts", "ctrl-x ctrl-shift-alt-f12")])
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    // the oracle trims the row; ours keeps the footer's two-cell indent
    s.assert_row(7, "  ⌃x ⌃⇧⌥f12 / esc close");
    s.resize(22, 8);
    s.assert_row(7, "  esc close");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1031 status_survives_hints_feedback_and_queue kind=snap owner=ui-pane snap=persistent_status_and_messages
#[test]
fn status_survives_hints_feedback_and_queue() {
    kit::pending(Owner::UiPane, QUEUE_REASON);
    // the status `MODEL · ~/project · Context 20% used` and `Plan mode` persist through the
    // shortcut overlay, a copy flash, a hint override, a running task and the file and skill
    // popups (the test sets the flash and the override; no key does)
    let mut s = Screen::build(Owner::UiPane).size(64, 24).live().open();
    s.assert_snap("persistent_status_and_messages");
    s.keys("?");
    s.assert_contains("MODEL · ~/project · Context 20% used");
    s.assert_contains("Plan mode");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1064 transcript_search_keeps_status_and_query_cursor kind=ui owner=ui-pane
#[test]
fn transcript_search_keeps_status_and_query_cursor() {
    kit::pending(Owner::UiPane, FIND_REASON);
    // the Find footer (`Find: needle`, cursor at 8) at 48 columns: heights 4, 5 and 6 keep its
    // `F` at column 2 and the cursor at column 10 of its row
    let mut s = Screen::build(Owner::UiPane).size(48, 4).live().open();
    let footer = row_of(&s, "Find:");
    s.assert_text_at(2, footer, "F");
    s.assert_cursor(10, footer);
    s.resize(48, 5);
    s.assert_text_at(2, row_of(&s, "Find:"), "F");
    s.resize(48, 6);
    s.assert_text_at(2, row_of(&s, "Find:"), "F");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1098 input_history_search_uses_the_final_row kind=ui owner=ui-pane
#[test]
fn input_history_search_uses_the_final_row() {
    kit::pending(Owner::UiPane, SPARKLE_REASON);
    // ctrl+r at 48 columns puts the cursor on the final row of the screen; the kit has no
    // row-only cursor assert, so the row is the last one of `rows()` (a kit gap, see the report)
    let mut s = Screen::build(Owner::UiPane).size(48, 6).live().open();
    s.keys("ctrl+r");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1109 slash_suggestions_take_space_before_status_expands kind=snap owner=ui-pane snap=slash_suggestions_take_space_before_status_expands
#[test]
fn slash_suggestions_take_space_before_status_expands() {
    kit::pending(Owner::UiPane, EXTENSION_REASON);
    // the status `one two three four` shows `one…` while the `/m` command popup takes the space
    // above the composer, at 80 columns and 7 rows
    let mut s = Screen::build(Owner::UiPane).size(80, 7).live().open();
    s.type_text("/m");
    s.assert_row_starts_with(0, "one…");
    s.assert_snap("slash_suggestions_take_space_before_status_expands");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1130 slash_popup_preserves_footer_and_selection_style kind=snap owner=ui-pane snap=slash_popup_footer_wide
#[test]
fn slash_popup_preserves_footer_and_selection_style() {
    kit::pending(Owner::UiPane, POPUP_REASON);
    // the `/m` popup with Down selected keeps the status `model · high · fast` below it at 80
    // and 32 columns; the transcript footer `transcript status` sits on the last row
    let mut s = Screen::build(Owner::UiPane).size(80, 12).live().open();
    s.type_text("/m");
    s.keys("down");
    s.assert_snap("slash_popup_footer_wide");
    s.resize(32, 12);
    s.assert_contains("model · high · fast");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1198 activity_shortcut_help_follows_runtime_binding kind=snap owner=ui-pane snap=activity_shortcut_help_follows_runtime_binding
#[test]
fn activity_shortcut_help_follows_runtime_binding() {
    // the help at 130 columns names the activity key from the runtime keymap: `f4`, `ctrl-x t`,
    // and no key at all (the row is gone); the test sets the keymap (`global.focus_activity`),
    // no key does. The oracle joined its three renders in one snapshot with `---` lines.
    const NAME: &str = "activity_shortcut_help_follows_runtime_binding";
    let oracle = kit::snap_rows(NAME, "");
    let frames: Vec<&[String]> = oracle.split(|row| row == "---").collect();
    assert_eq!(frames.len(), 3, "the snapshot holds three renders");
    let keymaps: [&'static [(&'static str, &'static str)]; 3] = [
        &[("global.focus_activity", "f4")],
        &[("global.focus_activity", "ctrl-x t")],
        &[("global.focus_activity", "")],
    ];
    for (keymap, frame) in keymaps.into_iter().zip(frames) {
        let mut s = Screen::build(Owner::UiPane)
            .size(130, 24)
            .script(HELP_STATUS)
            .keymap(keymap)
            .open();
        s.keys("?");
        assert_help_rows(&s, frame, NAME);
    }
}

// ---- oracle: bottom_pane/chat_composer/warning_notice_tests.rs (4 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:57 warning_notice_styles kind=snap owner=ui-pane snap=warning_notice_styles
#[test]
fn warning_notice_styles() {
    kit::pending(Owner::UiPane, WARN_REASON);
    // the draft `keep this draft` and 3 warnings at 80 columns, on the test's default colours;
    // the snapshot is a Buffer dump with the notice's styles
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.type_text("keep this draft");
    s.assert_snap("warning_notice_styles");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:71 light_warning_notice_clears_inherited_dimming kind=ui owner=ui-pane
#[test]
fn light_warning_notice_clears_inherited_dimming() {
    kit::pending(Owner::UiPane, WARN_REASON);
    // on a light background the notice's `3` at column 2 is not dimmed, the notice is 20
    // columns wide (the test sets the palette and the dim; the kit has no palette seed)
    let s = Screen::build(Owner::UiPane).size(20, 1).live().open();
    s.assert_text_at(2, 0, "3");
    s.assert_style(2, 0, "!dim");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:102 warning_notice_respects_shortcuts_and_interactive_hints kind=ui owner=ui-pane
#[test]
fn warning_notice_respects_shortcuts_and_interactive_hints() {
    kit::pending(Owner::UiPane, WARN_REASON);
    // 2 warnings at 64 columns: `⚠ 2 warnings · /warnings to view`, or `f12 to view` when the
    // shortcut is set (the test sets it); a `/` draft or the search hides the notice
    let mut s = Screen::build(Owner::UiPane).size(64, 24).live().open();
    s.assert_contains("⚠ 2 warnings · /warnings to view");
    s.type_text("/");
    s.assert_absent("⚠ 2 warnings");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:139 warnings_yield_to_queue_controls_and_complete_navigation_hints kind=ui owner=ui-pane
#[test]
fn warnings_yield_to_queue_controls_and_complete_navigation_hints() {
    kit::pending(Owner::UiPane, WARN_REASON);
    // a queued draft with a task running shows no notice; with the hint `esc latest` and 4
    // warnings the notice shows at 20 columns, and not at 12
    let mut s = Screen::build(Owner::UiPane).size(32, 6).live().open();
    s.type_text("queued draft");
    s.assert_absent("⚠ 4 warnings");
    s.resize(20, 6);
    s.assert_contains("esc latest");
    s.assert_contains("⚠");
    s.resize(12, 6);
    s.assert_absent("⚠");
}

// ---- oracle: bottom_pane/chat_composer_effort_tests.rs (3 screen-level tests here; 4 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer_effort_tests.rs:123 effort_transition_never_replaces_a_footer_flash kind=ui owner=ui-pane
#[test]
fn effort_transition_never_replaces_a_footer_flash() {
    kit::pending(Owner::UiPane, EFFORT_REASON);
    // the flash `saved` (1 s) on the footer row at 60x6 is kept through the effort transitions
    // High then Ultra; the footer never shows `U L T R A` (the test sets them; no key does)
    let s = Screen::build(Owner::UiPane).size(60, 6).live().open();
    s.assert_contains("saved");
    s.assert_absent("U L T R A");
}

// ORACLE bottom_pane/chat_composer_effort_tests.rs:154 effort_transition_keeps_the_full_footer_row kind=snap owner=ui-pane snap=codex_tui__bottom_pane__chat_composer__effort_tests__effort_transition_keeps_the_full_footer_row
#[test]
fn effort_transition_keeps_the_full_footer_row() {
    kit::pending(Owner::UiPane, EFFORT_REASON);
    // the two bottom rows at 36 columns: `gpt-5.4 ultra · feature-branch` and the Plan-mode row
    // (the test sets the effort and the indicator; no key does)
    let s = Screen::build(Owner::UiPane).size(36, 6).live().open();
    s.assert_snap_region("effort_transition_keeps_the_full_footer_row", 4, 0);
}

// ORACLE bottom_pane/chat_composer_effort_tests.rs:188 ultra_accent_upgrades_prompt_glyph kind=snap owner=ui-pane snap=ultra_accent_upgrades_prompt_glyph
#[test]
fn ultra_accent_upgrades_prompt_glyph() {
    kit::pending(Owner::UiPane, EFFORT_REASON);
    // the prompt glyph `›` takes the Ultra accent at 60 columns (the test sets Ultra; no key does)
    let s = Screen::build(Owner::UiPane).size(60, 24).live().open();
    s.assert_snap("ultra_accent_upgrades_prompt_glyph");
}
