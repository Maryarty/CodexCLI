//! Shard `cells_2` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: history_cell/messages_tests.rs (12 screen-level tests here; 3 state tests left to chat-live)
// ORACLE history_cell/messages_tests.rs:7 typed_prompt_markdown_keeps_the_prompt_gutter_and_links kind=snap owner=chat-live snap=typed_prompt_markdown_keeps_the_prompt_gutter_and_links
#[test]
fn typed_prompt_markdown_keeps_the_prompt_gutter_and_links() {
    kit::pending(
        Owner::ChatLive,
        "the transcript draws a user prompt as plain text rows (src/screens/chat/home.rs:328): no prompt gutter, markdown bold and italic, hyperlinks, or wrap at the prompt width",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(64, 6)
        .script(include_str!(
            "fixtures/cells_2/typed_prompt_markdown_keeps_the_prompt_gutter_and_links.jsonl"
        ))
        .open();
    s.assert_row(0, "› testing hello example documentation");
    s.assert_row(1, "  (https://example.test/docs)");
    s.assert_row(2, "");
    s.assert_row(3, "  • Try the command");
    s.assert_row(4, "");
    s.assert_row(5, "  echo '*literal*'");
    s.assert_text_style("testing", "bold");
    s.assert_text_style("hello", "italic");
    s.assert_link("example documentation", "https://example.test/docs");
    s.resize(20, 10);
    s.assert_row(0, "› testing hello");
    s.assert_row(1, "  example");
    s.assert_row(2, "  documentation");
    s.assert_row(3, "  (https://");
    s.assert_row(4, "  example.test/");
    s.assert_row(5, "  docs)");
    s.assert_row(6, "");
    s.assert_row(7, "  • Try the command");
    s.assert_row(8, "");
    s.assert_row(9, "  echo '*literal*'");
}

// ORACLE history_cell/messages_tests.rs:51 user_prompt_urls_keep_destinations_across_widths kind=snap owner=chat-live snap=user_prompt_urls_keep_destinations_across_widths
#[test]
fn user_prompt_urls_keep_destinations_across_widths() {
    kit::pending(
        Owner::ChatLive,
        "the transcript draws a user prompt as plain text rows (src/screens/chat/home.rs:328): no URL hyperlinks and no wrap at the prompt width; the oracle's per-width link text at width 24 needs wrapped links",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 4)
        .script(include_str!(
            "fixtures/cells_2/user_prompt_urls_keep_destinations_across_widths.jsonl"
        ))
        .open();
    s.assert_row(
        0,
        "› from this site, write out the ascii for solid octants 18 columns",
    );
    s.assert_row(
        1,
        "  https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_row(2, "  [Image #1] See (https://example.test/a).");
    s.assert_link(
        "https://example.test/terminal-link-wrapping/solid-octants",
        "https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_link("https://example.test/a", "https://example.test/a");
    s.resize(60, 4);
    s.assert_row(
        0,
        "› from this site, write out the ascii for solid octants 18",
    );
    s.assert_row(1, "  columns");
    s.assert_row(
        2,
        "  https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_row(3, "  [Image #1] See (https://example.test/a).");
    s.resize(24, 10);
    s.assert_row(0, "› from this site, write");
    s.assert_row(1, "  out the ascii for");
    s.assert_row(2, "  solid octants 18");
    s.assert_row(3, "  columns");
    s.assert_row(4, "  https://example.test/");
    s.assert_row(5, "  terminal-link-wrappin");
    s.assert_row(6, "  g/solid-octants");
    s.assert_row(7, "  [Image #1] See");
    s.assert_row(8, "  (https://example.test");
    s.assert_row(9, "  /a).");
}

// ORACLE history_cell/messages_tests.rs:107 local_image_only_user_message_remains_visible kind=snap owner=chat-live snap=local_image_only_user_message_remains_visible
#[test]
fn local_image_only_user_message_remains_visible() {
    let s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .script(include_str!(
            "fixtures/cells_2/local_image_only_user_message_remains_visible.jsonl"
        ))
        .open();
    let label = image_row(&s);
    s.assert_snap_region(
        "local_image_only_user_message_remains_visible",
        label - 1,
        0,
    );
}

// ORACLE history_cell/messages_tests.rs:121 mixed_image_labels_preserve_existing_placeholders_without_duplicates kind=snap owner=chat-live snap=mixed_image_labels_preserve_existing_placeholders_without_duplicates
#[test]
fn mixed_image_labels_preserve_existing_placeholders_without_duplicates() {
    let s = Screen::build(Owner::ChatLive)
        .size(40, 16)
        .script(include_str!(
            "fixtures/cells_2/mixed_image_labels_preserve_existing_placeholders_without_duplicates.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "mixed_image_labels_preserve_existing_placeholders_without_duplicates",
        image_row(&s) - 1,
        0,
    );
}

/// The row of the cell that starts with `start`: the oracle's snapshot is the
/// cell alone, the screen holds it among the chat's rows.
fn row_of(screen: &Screen, start: &str) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.starts_with(start))
        .unwrap_or_else(|| panic!("no row starts with `{start}`:\n{}", screen.text()))
}

fn image_row(screen: &Screen) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.contains("[Image #1]"))
        .filter(|&row| row > 0)
        .unwrap_or_else(|| {
            panic!(
                "no attachment with its top padding shown:\n{}",
                screen.text()
            )
        })
}

// ORACLE history_cell/messages_tests.rs:196 spoken_user_messages_have_a_red_chevron_without_changing_raw_text kind=snap owner=chat-live snap=spoken_user_prompt
#[test]
fn spoken_user_messages_have_a_red_chevron_without_changing_raw_text() {
    kit::pending(
        Owner::ChatLive,
        "no spoken user item: the wire carries no spoken flag, and the transcript draws every user item with the same plain prefix (src/screens/chat/home.rs:328); the red chevron is not drawn",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 3)
        .script(include_str!(
            "fixtures/cells_2/spoken_user_messages_have_a_red_chevron_without_changing_raw_text.jsonl"
        ))
        .open();
    s.assert_has_row("› hello from voice");
    // The oracle's Buffer dump (styles included): the chevron is red and bold.
    s.assert_snap_region("spoken_user_prompt", 0, 0);
    // Raw mode (the raw text `  hello from voice`) is the raw scrollback toggle, not built yet.
}

// ORACLE history_cell/messages_tests.rs:261 finalized_markdown_reuses_rendered_transcript_lines kind=ui owner=chat-live
#[test]
fn finalized_markdown_reuses_rendered_transcript_lines() {
    kit::pending(
        Owner::ChatLive,
        "a finalized agent message is drawn as plain text (src/screens/chat/home.rs:333), not the rich markdown cell with its transcript lines; the oracle's render-cache seed (replace_cached_lines) is internal and not reachable through the kit",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(48, 4)
        .script(include_str!(
            "fixtures/cells_2/finalized_markdown_reuses_rendered_transcript_lines.jsonl"
        ))
        .open();
    // The oracle compares the transcript lines with the display lines; on the screen the
    // display row is the one drawn (the bullet is the agent cell's prefix, as in the oracle's
    // file-citation snapshot).
    s.assert_has_row("• finalized markdown");
    let before = s.shot();
    s.resize(48, 4);
    s.assert_same_rows(&before);
}

// ORACLE history_cell/messages_tests.rs:278 finalized_assistant_file_citation_renders_as_local_path_snapshot kind=snap owner=chat-live
#[test]
fn finalized_assistant_file_citation_renders_as_local_path_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_2/finalized_assistant_file_citation_renders_as_local_path_snapshot.jsonl"
        ))
        .cwd("/tmp")
        .open();
    s.assert_has_row("• Generated Quarterly Report.xlsx.");
}

// ORACLE history_cell/messages_tests.rs:294 finalized_markdown_cache_misses_when_width_or_render_style_changes kind=ui owner=chat-live
#[test]
fn finalized_markdown_cache_misses_when_width_or_render_style_changes() {
    kit::pending(
        Owner::ChatLive,
        "a finalized agent message is drawn as plain text (src/screens/chat/home.rs:333), not the rich markdown cell; the oracle's cache-key edits (width, syntax theme, terminal fg) are internal and not reachable through the kit",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(48, 4)
        .script(include_str!(
            "fixtures/cells_2/finalized_markdown_cache_misses_when_width_or_render_style_changes.jsonl"
        ))
        .open();
    s.assert_has_row("• finalized markdown");
    // The visible half: after a re-render at the same width the rows are the same.
    let before = s.shot();
    s.resize(48, 4);
    s.assert_same_rows(&before);
}

// ORACLE history_cell/messages_tests.rs:316 raw_markdown_bypasses_the_rich_render_cache kind=ui owner=chat-live
#[test]
fn raw_markdown_bypasses_the_rich_render_cache() {
    kit::pending(
        Owner::ChatLive,
        "the rich markdown row is not drawn (src/screens/chat/home.rs:333); the raw view `finalized **markdown**` (toggle_raw_output, keymap/bindings.rs:343) is not built in the S11 transcript",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(48, 4)
        .script(include_str!(
            "fixtures/cells_2/raw_markdown_bypasses_the_rich_render_cache.jsonl"
        ))
        .open();
    s.assert_has_row("• finalized markdown");
    // Raw half, not ported: after the raw toggle the screen shows the source row
    // `finalized **markdown**` (the oracle's display_lines_for_mode(Raw)).
}

// ORACLE history_cell/messages_tests.rs:331 visualization_directives_are_not_cached kind=ui owner=chat-live
#[test]
fn visualization_directives_are_not_cached() {
    kit::na(
        Owner::ChatLive,
        "asserts that the rich render cache stays unset (`rendered_lines` is None) for visualization directives; no row or style is compared",
    );
}

// ORACLE history_cell/messages_tests.rs:345 spoken_artifacts_link_only_real_workspace_files_and_preserve_existing_urls kind=snap owner=chat-live
#[test]
fn spoken_artifacts_link_only_real_workspace_files_and_preserve_existing_urls() {
    kit::pending(
        Owner::ChatLive,
        "no spoken agent cell and no workspace-file link: the transcript draws the agent text plain (src/screens/chat/home.rs:333); the workspace file the oracle links is a tempdir the kit cannot create",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(90, 4)
        .script(include_str!(
            "fixtures/cells_2/spoken_artifacts_link_only_real_workspace_files_and_preserve_existing_urls.jsonl"
        ))
        .open();
    s.assert_has_row("• 中 src/lib.rs:42 and https://example.com");
    s.assert_link("https://example.com", "https://example.com");
    s.assert_text_style("src/lib.rs:42", "underlined");
    // Not ported: the `src/lib.rs:42` link destination is the canonical file URL of the
    // tempdir workspace (the oracle's expected_destination); the ordinary cell's link and
    // the bare `src/lib.rs` link (1 link at width 40) need a non-spoken seed the wire lacks.
}

// ORACLE history_cell/messages_tests.rs:416 spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file kind=snap owner=chat-live snap=spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file
#[test]
fn spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file() {
    kit::pending(
        Owner::ChatLive,
        "no spoken agent cell: the transcript draws the agent text plain (src/screens/chat/home.rs:333), and the workspace-file link is not made",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/cells_2/spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file.jsonl"
        ))
        .open();
    s.assert_has_row("• Updated src\\lib.rs:42");
    s.assert_text_style("src\\lib.rs:42", "underlined");
    // Not ported: the link destination is the canonical file URL of the tempdir workspace.
}

// ---- oracle: history_cell/patches_tests.rs (3 screen-level tests here; 1 state tests left to chat-live)
// ORACLE history_cell/patches_tests.rs:5 compact_patches_retain_full_changes_and_failure_details kind=snap owner=chat-live snap=compact_patches_retain_full_changes_and_failure_details
#[test]
fn compact_patches_retain_full_changes_and_failure_details() {
    kit::pending(
        Owner::ChatLive,
        "the failure diagnostics (`first`, `second`, `third`) are the oracle test's stderr: the app-server's fileChange item carries none, and the oracle's own live path then shows `(error details unavailable)` (chatwidget/tool_lifecycle.rs:173); the patch block passes",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 20)
        .script(include_str!(
            "fixtures/cells_2/compact_patches_retain_full_changes_and_failure_details.jsonl"
        ))
        .open();
    // The compact view (the default on the screen): the oracle's `patch` and `failure` blocks.
    s.assert_in_order(&[
        "• Edited 2 files (+6 -1)",
        "  └ existing.txt (+1 -1)",
        "    2 -before",
        "    2 +after",
        "  └ new.txt (+5 -0)",
        "    1 +one",
        "    2 +two",
        "    3 +three",
        "✘ Failed to apply patch",
        "  └ first",
        "    second",
        "    third",
    ]);
    // Not ported: the Full view (the transcript overlay) and the `missing diagnostics` block.
}

// ORACLE history_cell/patches_tests.rs:45 viewed_image_retains_original_path_in_details kind=ui owner=chat-live
#[test]
fn viewed_image_retains_original_path_in_details() {
    // The oracle's transcript lines (width 200) are what the control reveals in place.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 9)
        .script(include_str!(
            "fixtures/cells_2/viewed_image_retains_original_path_in_details.jsonl"
        ))
        .open();
    s.assert_has_row("• Viewed image example.png");
    s.click_text("+ Show details");
    s.assert_has_row("• Viewed image /workspace/assets/example.png");
    // Not ported: the raw row (raw scrollback, not built).
    let mut w = Screen::build(Owner::ChatLive)
        .size(80, 9)
        .script(include_str!(
            "fixtures/cells_2/viewed_image_retains_original_path_in_details_windows.jsonl"
        ))
        .open();
    w.assert_has_row("• Viewed image example.png");
    w.click_text("+ Show details");
    w.assert_has_row(r"• Viewed image C:\workspace\assets\example.png");
}

// ORACLE history_cell/patches_tests.rs:74 viewed_image_narrow_summary kind=snap owner=chat-live snap=viewed_image_narrow_summary
#[test]
fn viewed_image_narrow_summary() {
    let s = Screen::build(Owner::ChatLive)
        .size(32, 9)
        .script(include_str!(
            "fixtures/cells_2/viewed_image_narrow_summary.jsonl"
        ))
        .open();
    s.assert_snap_region("viewed_image_narrow_summary", row_of(&s, "• Viewed"), 0);
    // Not ported: the image link (columns 15..width) is the file URL of a real file the kit cannot create.
}

// ---- oracle: history_cell/plans_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/plans_tests.rs:5 finalized_plan_reuses_rendered_transcript_lines kind=ui owner=chat-live
#[test]
fn finalized_plan_reuses_rendered_transcript_lines() {
    kit::na(
        Owner::ChatLive,
        "D5: the oracle's check is the cell's render cache (the second call returns the first call's lines); no screen holds a cache, and the running turn's spinner ticks between two shots of the 48x4 screen, which shows only the bottom pane",
    );
}
// ORACLE history_cell/plans_tests.rs:28 finalized_plan_file_citation_renders_as_local_path_snapshot kind=snap owner=chat-live
#[test]
fn finalized_plan_file_citation_renders_as_local_path_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 13)
        .script(include_str!(
            "fixtures/cells_2/finalized_plan_file_citation_renders_as_local_path_snapshot.jsonl"
        ))
        .cwd("/tmp")
        .open();
    let top = row_of(&s, "• Proposed Plan");
    s.assert_row(top, "• Proposed Plan");
    s.assert_row(top + 1, " ");
    s.assert_row(top + 2, " ");
    s.assert_row(top + 3, "  • Quarterly Report.xlsx");
}

// ---- oracle: history_cell/request_user_input_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/request_user_input_tests.rs:9 completed_and_interrupted_results kind=snap owner=chat-live snap=completed_and_interrupted_results
#[test]
fn completed_and_interrupted_results() {
    kit::pending(
        Owner::ChatLive,
        "no request-user-input result cell and no seed for its answers or the interrupted state: the transcript carries no such item; the oracle also pins the default colours (240,240,240 on 24,24,24), which the kit cannot set",
    );
    let s = Screen::build(Owner::ChatLive).size(40, 5).live().open();
    s.assert_snap_section("completed_and_interrupted_results", "completed");
}

// ---- oracle: history_cell/search_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/search_tests.rs:5 web_action_labels_and_missing_details kind=snap owner=chat-live snap=web_action_labels_and_missing_details
#[test]
fn web_action_labels_and_missing_details() {
    // The snapshot is the six cells' display lines one after another; the chat
    // puts a blank row between two history cells (oracle app.rs), so each row
    // of the snapshot is a row of the screen, in order.
    let s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_2/web_action_labels_and_missing_details.jsonl"
        ))
        .open();
    let rows = [
        "• Opened https://example.com/docs",
        "• Opened page",
        "• Searched for 'needle' in https://example.com/docs",
        "• Searched for 'needle'",
        "• Searched page https://example.com/docs",
        "• Searched page",
    ];
    s.assert_in_order(&rows);
    for row in rows {
        s.assert_has_row(row);
    }
}

// ORACLE history_cell/search_tests.rs:42 web_action_compact_display_preserves_transcript_and_raw_details kind=snap owner=chat-live snap=web_action_compact_display_preserves_transcript_and_raw_details
#[test]
fn web_action_compact_display_preserves_transcript_and_raw_details() {
    let s = Screen::build(Owner::ChatLive)
        .size(32, 8)
        .script(include_str!(
            "fixtures/cells_2/web_action_compact_display_preserves_transcript_and_raw_details.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "web_action_compact_display_preserves_transcript_and_raw_details",
        row_of(&s, "• Searched"),
        0,
    );
    // Not ported: the transcript view at width 200 (`• Searched for '日本語 🦀 needle' in <url>`)
    // and the raw row, both in overlays not built yet.
}

// ORACLE history_cell/search_tests.rs:68 pending_web_action_and_legacy_query kind=snap owner=chat-live
#[test]
fn pending_web_action_and_legacy_query() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(include_str!(
            "fixtures/cells_2/pending_web_action_and_legacy_query.jsonl"
        ))
        .open();
    s.assert_has_row("• Browsing the web");
    // Not ported: the legacy raw row `Searched the web for old query` (raw scrollback, not built).
}

// ORACLE history_cell/search_tests.rs:83 batched_search_retains_each_query kind=snap owner=chat-live
#[test]
fn batched_search_retains_each_query() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(include_str!(
            "fixtures/cells_2/batched_search_retains_each_query.jsonl"
        ))
        .open();
    s.assert_has_row("• Searched the web for first query, second query");
    // Not ported: the raw row `Searched the web for first query, second query` (raw scrollback, not built).
}

// ---- oracle: history_cell/separators_tests.rs (9 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/separators_tests.rs:22 completion_label_shows_known_durations_including_short_turns kind=snap owner=chat-live
#[test]
fn completion_label_shows_known_durations_including_short_turns() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript: the chat draws no `Worked for …` label (the turn's end is not a row); and the viewer's today is the real clock, which the kit cannot set, so the time-only label cannot be seeded",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/cells_2/completion_label_shows_known_durations_including_short_turns.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "2:32 PM",
        "Worked for <1s • 2:32 PM",
        "Worked for 12s • 2:32 PM",
        "Worked for 1m 0s • 2:32 PM",
        "Worked for 1m 1s • 2:32 PM",
        "Worked for 2m 5s • 2:32 PM",
        "Worked for 1h 0m 5s • 2:32 PM",
    ]);
}

// ORACLE history_cell/separators_tests.rs:54 completion_label_includes_date_when_viewed_on_another_day kind=snap owner=chat-live
#[test]
fn completion_label_includes_date_when_viewed_on_another_day() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript; the 'viewed on another day' date needs the viewer's today set a day (and a year) after the completion, which the kit's clock cannot seed",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 2)
        .script(include_str!(
            "fixtures/cells_2/completion_label_includes_date_when_viewed_on_another_day.jsonl"
        ))
        .open();
    s.assert_contains("Worked for 2m 5s • Sep 6 at 2:32 PM");
    // Not ported: the next-year label `Worked for 2m 5s • Sep 6, 2000 at 2:32 PM` (viewer in 2001).
}

// ORACLE history_cell/separators_tests.rs:69 completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon kind=snap owner=chat-live
#[test]
fn completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript, and the time-only label needs the viewer's today to be the completion day, which the kit's real clock cannot be set to",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/cells_2/completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon.jsonl"
        ))
        .open();
    s.assert_in_order(&["12:32 AM", "12:32 PM", "3:32 PM"]);
}

// ORACLE history_cell/separators_tests.rs:85 completion_uses_twenty_four_hour_time_with_dates_and_wrapping kind=snap owner=chat-live snap=completion_uses_twenty_four_hour_time_with_dates_and_wrapping
#[test]
fn completion_uses_twenty_four_hour_time_with_dates_and_wrapping() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript; the 24-hour clock format is a setting the kit cannot seed, and the today / tomorrow / next-year viewer dates need a clock seam",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(24, 15)
        .script(include_str!(
            "fixtures/cells_2/completion_uses_twenty_four_hour_time_with_dates_and_wrapping.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completion_uses_twenty_four_hour_time_with_dates_and_wrapping",
        0,
        0,
    );
}

// ORACLE history_cell/separators_tests.rs:118 completion_without_metadata_has_no_visible_or_raw_lines kind=ui owner=chat-live
#[test]
fn completion_without_metadata_has_no_visible_or_raw_lines() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no completion separator to leave out (src/screens/chat/home.rs draws no turn end): the test pins that a completion without elapsed time and timestamp adds no row",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/cells_2/completion_without_metadata_has_no_visible_or_raw_lines.jsonl"
        ))
        .open();
    s.assert_absent("Worked for");
    s.assert_absent("PM");
    s.assert_absent("AM");
    // Not ported: the raw lines (none) in raw scrollback.
}

// ORACLE history_cell/separators_tests.rs:126 completion_without_timestamp_retains_known_elapsed_duration kind=snap owner=chat-live
#[test]
fn completion_without_timestamp_retains_known_elapsed_duration() {
    kit::pending(
        Owner::ChatLive,
        "the test pins the raw text `Worked for 2m 5s` of a completion without timestamp; that text is drawn only in the raw scrollback (toggle_raw_output, keymap/bindings.rs:343), not built in the S11 transcript",
    );
    let _s = Screen::build(Owner::ChatLive)
        .size(80, 3)
        .script(include_str!(
            "fixtures/cells_2/completion_without_timestamp_retains_known_elapsed_duration.jsonl"
        ))
        .open();
    // Not ported: the raw row `Worked for 2m 5s` in the raw scrollback.
}

// ORACLE history_cell/separators_tests.rs:135 completion_wraps_metadata_and_preserves_unwrapped_raw_text kind=snap owner=chat-live snap=completion_wraps_metadata_and_preserves_unwrapped_raw_text
#[test]
fn completion_wraps_metadata_and_preserves_unwrapped_raw_text() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript, and no runtime-metrics seed (3 tool calls, 2.5s) the chat reads from the turn; the viewer date needs a clock seam",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(24, 6)
        .script(include_str!(
            "fixtures/cells_2/completion_wraps_metadata_and_preserves_unwrapped_raw_text.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "Worked for 2m 5s • Sep",
        "6, 2000 at 2:32 PM •",
        "Local tools: 3 calls",
        "(2.5s)",
    ]);
    // Not ported: the raw line (unwrapped) in raw scrollback, and the width loop 0, 1, 5, 24
    // (no line wider than the width).
}

// ORACLE history_cell/separators_tests.rs:168 completion_renders_with_dim_default_colors kind=ui owner=chat-live
#[test]
fn completion_renders_with_dim_default_colors() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript: the dim, default-coloured separator row is not drawn; the viewer's today is the real clock, which the kit cannot set",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 1)
        .script(include_str!(
            "fixtures/cells_2/completion_renders_with_dim_default_colors.jsonl"
        ))
        .open();
    // The oracle's display row `  2:32 PM` (nine cells) is Reset on Reset and DIM in every cell.
    for x in 0..9 {
        s.assert_style(x, 0, "fg=Reset bg=Reset dim");
    }
}

// ORACLE history_cell/separators_tests.rs:192 completion_rendering_uses_the_captured_display_date kind=snap owner=chat-live
#[test]
fn completion_rendering_uses_the_captured_display_date() {
    kit::pending(
        Owner::ChatLive,
        "no completion separator in the S11 transcript: the time-only `  2:32 PM` row is not drawn; it needs the viewer's today to be the captured display date, which the kit's real clock cannot set",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 2)
        .script(include_str!(
            "fixtures/cells_2/completion_rendering_uses_the_captured_display_date.jsonl"
        ))
        .open();
    s.assert_has_row("  2:32 PM");
    // Not ported: the raw line `2:32 PM` (raw scrollback, not built).
}

// ---- oracle: history_cell/session_transcript_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/session_transcript_tests.rs:15 wheel_at_hidden_session_header_keeps_the_loaded_message_anchor kind=ui owner=chat-live
#[test]
fn wheel_at_hidden_session_header_keeps_the_loaded_message_anchor() {
    kit::pending(
        Owner::ChatLive,
        "no script step answers `thread/turns/list`, so a thread never gets a next cursor (the header is hidden only while `ThreadModel::can_load_history`), and the wheel at the top of the chat does not page older turns in or keep the anchor (the chat-transcript slice: TranscriptView history states)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!(
            "fixtures/cells_2/wheel_at_hidden_session_header_keeps_the_loaded_message_anchor.jsonl"
        ))
        .open();
    s.wheel_up(0, 0, 1);
    s.wheel_up(0, 0, 1);
    let before = s.shot();
    s.feed(include_str!(
        "fixtures/cells_2/wheel_at_hidden_session_header_keeps_the_loaded_message_anchor_older.jsonl"
    ));
    // One wheel gesture must not keep requesting older pages, and the loaded page leaves the
    // rows where they were.
    s.assert_same_rows(&before);
}

// ORACLE history_cell/session_transcript_tests.rs:64 session_information_waits_for_older_pages_and_reappears_after_completion kind=ui owner=chat-live
#[test]
fn session_information_waits_for_older_pages_and_reappears_after_completion() {
    kit::pending(
        Owner::ChatLive,
        "the header is hidden while older pages exist (`can_load_history`, the oracle's Partial) and shown when the history is complete, but the kit cannot give a thread a next cursor (no `thread/turns/list` script step) and core has no LoadingOlder, LoadingBeginning or Failed states (the chat-transcript slice); the row to compare is the pane's `OpenAI Codex` header, the oracle text `session announcement` is its test stand-in",
    );
    let s = Screen::build(Owner::ChatLive).size(30, 2).live().open();
    // The Complete state: the session announcement is the first row. The oracle's other
    // states draw nothing until their history is complete (not reachable through the kit yet).
    s.assert_row(0, "session announcement");
    s.assert_row(1, "");
}
