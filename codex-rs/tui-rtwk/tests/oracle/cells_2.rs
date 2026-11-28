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
        "G73 code padded one cell (the fence row reads `   echo '*literal*'`, the oracle's `  echo '*literal*'`; at width 20 it wraps its last quote); G49 no break after a slash (the link destination at width 20 wraps `(https://example.` / `test/docs)`, the oracle's `(https://` / `example.test/` / `docs)`); the gutter, the bold, italic and link styles and width 64 equal",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(64, 20)
        .script(include_str!(
            "fixtures/cells_2/typed_prompt_markdown_keeps_the_prompt_gutter_and_links.jsonl"
        ))
        .open();
    // The oracle's snapshot is the cell alone: the screen holds it among the chat's rows.
    let y = row_of(&s, "› testing");
    s.assert_row(y, "› testing hello example documentation");
    s.assert_row(y + 1, "  (https://example.test/docs)");
    s.assert_row(y + 2, "");
    s.assert_row(y + 3, "  • Try the command");
    s.assert_row(y + 4, "");
    s.assert_row(y + 5, "  echo '*literal*'");
    s.assert_text_style("testing", "bold");
    s.assert_text_style("hello", "italic");
    s.assert_link("example documentation", "https://example.test/docs");
    s.resize(20, 24);
    let y = row_of(&s, "› testing");
    s.assert_row(y, "› testing hello");
    s.assert_row(y + 1, "  example");
    s.assert_row(y + 2, "  documentation");
    s.assert_row(y + 3, "  (https://");
    s.assert_row(y + 4, "  example.test/");
    s.assert_row(y + 5, "  docs)");
    s.assert_row(y + 6, "");
    s.assert_row(y + 7, "  • Try the command");
    s.assert_row(y + 8, "");
    s.assert_row(y + 9, "  echo '*literal*'");
}

// ORACLE history_cell/messages_tests.rs:51 user_prompt_urls_keep_destinations_across_widths kind=snap owner=chat-live snap=user_prompt_urls_keep_destinations_across_widths
#[test]
fn user_prompt_urls_keep_destinations_across_widths() {
    kit::pending(
        Owner::ChatLive,
        "G48 TextWrap::Word breaks after a hyphen inside the URL: at 120 columns `.../solid-` ends the prose row and `octants` follows, the oracle keeps the token whole on a row of its own and cuts it only at the width (`terminal-link-wrappin` / `g/solid-octants` at 24); G49 no break before a slash (`(https://example.test` / `/a).`); the link destinations, the gutter and the image label equal",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 20)
        .script(include_str!(
            "fixtures/cells_2/user_prompt_urls_keep_destinations_across_widths.jsonl"
        ))
        .open();
    let y = row_of(&s, "› from this site");
    s.assert_row(
        y,
        "› from this site, write out the ascii for solid octants 18 columns",
    );
    s.assert_row(
        y + 1,
        "  https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_row(y + 2, "  [Image #1] See (https://example.test/a).");
    s.assert_link(
        "https://example.test/terminal-link-wrapping/solid-octants",
        "https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_link("https://example.test/a", "https://example.test/a");
    s.resize(60, 20);
    let y = row_of(&s, "› from this site");
    s.assert_row(
        y,
        "› from this site, write out the ascii for solid octants 18",
    );
    s.assert_row(y + 1, "  columns");
    s.assert_row(
        y + 2,
        "  https://example.test/terminal-link-wrapping/solid-octants",
    );
    s.assert_row(y + 3, "  [Image #1] See (https://example.test/a).");
    s.resize(24, 24);
    let y = row_of(&s, "› from this site");
    s.assert_row(y, "› from this site, write");
    s.assert_row(y + 1, "  out the ascii for");
    s.assert_row(y + 2, "  solid octants 18");
    s.assert_row(y + 3, "  columns");
    s.assert_row(y + 4, "  https://example.test/");
    s.assert_row(y + 5, "  terminal-link-wrappin");
    s.assert_row(y + 6, "  g/solid-octants");
    s.assert_row(y + 7, "  [Image #1] See");
    s.assert_row(y + 8, "  (https://example.test");
    s.assert_row(y + 9, "  /a).");
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
    // The wire has no spoken flag: a spoken prompt is the voice caption of the user (the oracle's
    // `realtime_transcript_history_cell`), so the cell is made the way the app makes it.
    let mut s = Screen::build(Owner::ChatLive).size(40, 12).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"animations":false}}}"#);
    s.voice_available().type_text("/voice").keys("enter");
    let session = s.connect_voice();
    s.voice_caption(true, &session, "speech", "user", "  hello from voice");
    s.wait_for("hello from voice");
    s.assert_has_row("› hello from voice");
    // The oracle's Buffer dump (styles included): the chevron is red and bold, the whitespace the
    // speech began with is not drawn.
    let y = row_of(&s, "› hello from voice");
    s.assert_snap_region("spoken_user_prompt", y - 1, 0);
    s.assert_style(0, y, "fg=Red bold");
    // Raw mode (the raw text `  hello from voice`) is the raw scrollback toggle, not built yet.
}

// ORACLE history_cell/messages_tests.rs:261 finalized_markdown_reuses_rendered_transcript_lines kind=ui owner=chat-live
#[test]
fn finalized_markdown_reuses_rendered_transcript_lines() {
    kit::na(
        Owner::ChatLive,
        "asserts the cell's render cache (its transcript lines equal the cached display lines): internal state, nothing is drawn differently; the drawn half is checked below",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(48, 10)
        .script(include_str!(
            "fixtures/cells_2/finalized_markdown_reuses_rendered_transcript_lines.jsonl"
        ))
        .open();
    // The oracle compares the transcript lines with the display lines; on the screen the
    // display row is the one drawn (the bullet is the agent cell's prefix, as in the oracle's
    // file-citation snapshot).
    s.assert_has_row("• finalized markdown");
    let before = s.shot();
    s.resize(40, 10);
    s.resize(48, 10);
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
    kit::na(
        Owner::ChatLive,
        "asserts the cell's render-cache key (a cache built for another width, syntax theme or terminal foreground misses): internal state, nothing is drawn differently; the drawn half is checked below",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(48, 10)
        .script(include_str!(
            "fixtures/cells_2/finalized_markdown_cache_misses_when_width_or_render_style_changes.jsonl"
        ))
        .open();
    s.assert_has_row("• finalized markdown");
    // The visible half: after a re-render at the same width the rows are the same.
    let before = s.shot();
    s.resize(40, 10);
    s.resize(48, 10);
    s.assert_same_rows(&before);
}

// ORACLE history_cell/messages_tests.rs:316 raw_markdown_bypasses_the_rich_render_cache kind=ui owner=chat-live
#[test]
fn raw_markdown_bypasses_the_rich_render_cache() {
    // 14 rows, not the oracle's 10: our toggle note ("Raw output mode on", two rows at 48
    // columns) joins the history below the cell, which the 7-row history of a 10-row
    // terminal cannot hold with the cell above it.
    let mut s = Screen::build(Owner::ChatLive)
        .size(48, 14)
        .script(include_str!(
            "fixtures/cells_2/raw_markdown_bypasses_the_rich_render_cache.jsonl"
        ))
        .open();
    s.assert_has_row("• finalized markdown");
    // Raw half: after the raw toggle the screen shows the source row.
    s.keys("alt+r");
    s.assert_has_row("finalized **markdown**");
    s.assert_absent("• finalized markdown");
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
    // The oracle's workspace is a tempdir with `src/lib.rs`; a spoken answer is the assistant's
    // voice caption (`new_spoken`).
    let workspace = kit::workspace(&[("src/lib.rs", "fn example() {}\n")]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(90, 12)
        .workspace(&workspace)
        .live()
        .open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"animations":false}}}"#);
    s.voice_available().type_text("/voice").keys("enter");
    let session = s.connect_voice();
    s.voice_caption(
        true,
        &session,
        "speech",
        "assistant",
        "中 src/lib.rs:42 and https://example.com",
    );
    s.wait_for("• 中 src/lib.rs:42 and https://example.com");
    s.wait_for_link("src/lib.rs:42", &workspace.url("src/lib.rs"));
    s.assert_has_row("• 中 src/lib.rs:42 and https://example.com");
    s.assert_link("https://example.com", "https://example.com");
    s.assert_text_style("src/lib.rs:42", "underlined");
    // only a file the workspace has is linked: the oracle validates the path on the disk
    s.voice_caption(
        true,
        &session,
        "speech-2",
        "assistant",
        "see src/missing.rs:7 and src/lib.rs",
    );
    s.wait_for("see src/missing.rs:7 and src/lib.rs");
    s.wait_for_link("src/lib.rs", &workspace.url("src/lib.rs"));
    s.assert_no_link("src/missing.rs:7");
}

// ORACLE history_cell/messages_tests.rs:416 spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file kind=snap owner=chat-live snap=spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file
#[test]
fn spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file() {
    let workspace = kit::workspace(&[("src/lib.rs", "fn example() {}\n")]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .workspace(&workspace)
        .live()
        .open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"animations":false}}}"#);
    s.voice_available().type_text("/voice").keys("enter");
    let session = s.connect_voice();
    s.voice_caption(
        true,
        &session,
        "speech",
        "assistant",
        "Updated src\\lib.rs:42",
    );
    s.wait_for("• Updated src\\lib.rs:42");
    // the text keeps its backslash, the link is the one file URL of the workspace
    s.wait_for_link("src\\lib.rs:42", &workspace.url("src/lib.rs"));
    s.assert_has_row("• Updated src\\lib.rs:42");
    s.assert_text_style("src\\lib.rs:42", "underlined");
}

// ---- oracle: history_cell/patches_tests.rs (3 screen-level tests here; 1 state tests left to chat-live)
// ORACLE history_cell/patches_tests.rs:5 compact_patches_retain_full_changes_and_failure_details kind=snap owner=chat-live snap=compact_patches_retain_full_changes_and_failure_details
#[test]
fn compact_patches_retain_full_changes_and_failure_details() {
    kit::na(
        Owner::ChatLive,
        "the failure diagnostics (`first`, `second`, `third`) are the oracle test's own stderr handed to `new_patch_apply_failure`: the app-server's fileChange item carries none, and the oracle's live path (chatwidget/tool_lifecycle.rs:173) shows `(error details unavailable)`, which is what the port draws; the patch block and that failure row are checked below",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 24)
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
        "  └ (error details unavailable)",
    ]);
    // The Full view (the details the control reveals in place) keeps every changed line.
    s.click_text("+ Show details");
    s.assert_in_order(&[
        "  └ existing.txt (+1 -1)",
        "    1  context",
        "    2 -before",
        "    2 +after",
        "    3  tail",
        "  └ new.txt (+5 -0)",
        "    1 +one",
        "    4 +four",
        "    5 +five",
        "✘ Failed to apply patch",
        "  └ (error details unavailable)",
    ]);
    // Not ported: the `failure` block (its stderr is not on the wire).
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
    // The exchange as the user has it: option 1 and a typed note, submitted; the transcript keeps
    // the result cell (the oracle test builds the cell directly and snapshots it twice). The
    // `interrupted` frame is the cell with its flag set and no answers: the oracle's own code
    // never sets the flag (`submit_answers`, `submit_empty_auto_resolution`), so no exchange
    // reaches it; the cell's rows for that frame are checked in tests/pages/cells.rs.
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 24)
        // the oracle's `with_test_default_colors`
        .terminal_colors(0xf0_f0_f0, 0x18_18_18)
        .script(include_str!(
            "fixtures/cells_2/completed_and_interrupted_results.jsonl"
        ))
        .open();
    s.keys("tab");
    s.type_text("Preserve the behavior");
    s.keys("enter");
    let top = s
        .rows()
        .iter()
        .position(|row| row.starts_with("• Questions"))
        .unwrap_or_else(|| panic!("no result cell:\n{}", s.text()));
    // The snapshot's rows (frame `completed`), then the styles of its runs. The kit's frame
    // compare also reads the blank cells' styles from the terminal, where the presenter leaves
    // a dim or bold attribute off a blank (G46, `assert_painted`): so the runs are asserted by
    // their text.
    let rows = [
        "• Questions 1/1 answered",
        "  • Which approach should we use for the",
        "    next step?",
        "    answer: Keep it small",
        "    note: Preserve the behavior",
    ];
    for (offset, row) in rows.into_iter().enumerate() {
        s.assert_row(top + offset, row);
    }
    s.assert_text_style("Questions", "bold");
    s.assert_text_style("1/1 answered", "dim");
    s.assert_text_style("answer:", "dim");
    s.assert_text_style("note:", "dim");
    s.assert_text_style("Keep it small", "fg=Rgb(99, 168, 248) !dim");
    s.assert_text_style("Preserve the behavior", "fg=Rgb(99, 168, 248) !dim");
    s.assert_text_style("Which approach", "!dim !bold");
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
// The fixtures seed the viewer's today (the agents clock the pane reads) and the clock setting
// (`cells` clock: 12 or 24 hour, zone UTC, so no label depends on the machine's zone); a turn is
// its message and the separator below it. The raw halves of the oracle's asserts (`raw_lines`)
// are the raw scrollback's (step 11, `cells/raw.rs`).
// Epochs of the fixtures: 968250720 is 2000-09-06 14:32 UTC, 968337120 the next day, 999873120
// 2001-09-07 14:32 UTC.

/// The rows of `labels` are on the screen, each exactly, in this order.
fn assert_labels(s: &Screen, labels: &[&str]) {
    for label in labels {
        s.assert_has_row(label);
    }
    s.assert_in_order(labels);
}

// ORACLE history_cell/separators_tests.rs:22 completion_label_shows_known_durations_including_short_turns kind=snap owner=chat-live
#[test]
fn completion_label_shows_known_durations_including_short_turns() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 40)
        .script(include_str!(
            "fixtures/cells_2/completion_label_shows_known_durations_including_short_turns.jsonl"
        ))
        .open();
    assert_labels(
        &s,
        &[
            "  2:32 PM",
            "  Worked for <1s • 2:32 PM",
            "  Worked for 12s • 2:32 PM",
            "  Worked for 1m 0s • 2:32 PM",
            "  Worked for 1m 1s • 2:32 PM",
            "  Worked for 2m 5s • 2:32 PM",
            "  Worked for 1h 0m 5s • 2:32 PM",
        ],
    );
}

// ORACLE history_cell/separators_tests.rs:54 completion_label_includes_date_when_viewed_on_another_day kind=snap owner=chat-live
#[test]
fn completion_label_includes_date_when_viewed_on_another_day() {
    let tomorrow = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(include_str!(
            "fixtures/cells_2/completion_label_includes_date_when_viewed_on_another_day.jsonl"
        ))
        .open();
    tomorrow.assert_has_row("  Worked for 2m 5s • Sep 6 at 2:32 PM");
    let next_year = Screen::build(Owner::ChatLive)
        .size(80, 8)
        .script(concat!(
            include_str!(
                "fixtures/cells_2/completion_label_includes_date_when_viewed_on_another_day.jsonl"
            ),
            "\n",
            r#"{"step":"screen","dir":"agents","ev":{"clock":999873120}}"#
        ))
        .open();
    next_year.assert_has_row("  Worked for 2m 5s • Sep 6, 2000 at 2:32 PM");
}

// ORACLE history_cell/separators_tests.rs:69 completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon kind=snap owner=chat-live
#[test]
fn completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/cells_2/completion_uses_twelve_hour_time_at_midnight_noon_and_afternoon.jsonl"
        ))
        .open();
    assert_labels(&s, &["  12:32 AM", "  12:32 PM", "  3:32 PM"]);
}

// ORACLE history_cell/separators_tests.rs:85 completion_uses_twenty_four_hour_time_with_dates_and_wrapping kind=snap owner=chat-live snap=completion_uses_twenty_four_hour_time_with_dates_and_wrapping
#[test]
fn completion_uses_twenty_four_hour_time_with_dates_and_wrapping() {
    // The oracle's snapshot joins nine labels (three hours, each seen today, tomorrow and next
    // year) and the three that wrap at width 24: one screen shows one viewer's day.
    let fixture = include_str!(
        "fixtures/cells_2/completion_uses_twenty_four_hour_time_with_dates_and_wrapping.jsonl"
    );
    let today = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(fixture)
        .open();
    assert_labels(
        &today,
        &[
            "  Worked for 2m 5s • 00:25",
            "  Worked for 2m 5s • 12:25",
            "  Worked for 2m 5s • 23:25",
        ],
    );
    let tomorrow = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(concat!(
            include_str!(
                "fixtures/cells_2/completion_uses_twenty_four_hour_time_with_dates_and_wrapping.jsonl"
            ),
            "\n",
            r#"{"step":"screen","dir":"agents","ev":{"clock":968337120}}"#
        ))
        .open();
    assert_labels(
        &tomorrow,
        &[
            "  Worked for 2m 5s • Sep 6 at 00:25",
            "  Worked for 2m 5s • Sep 6 at 12:25",
            "  Worked for 2m 5s • Sep 6 at 23:25",
        ],
    );
    let next_year = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(concat!(
            include_str!(
                "fixtures/cells_2/completion_uses_twenty_four_hour_time_with_dates_and_wrapping.jsonl"
            ),
            "\n",
            r#"{"step":"screen","dir":"agents","ev":{"clock":999873120}}"#
        ))
        .open();
    assert_labels(
        &next_year,
        &[
            "  Worked for 2m 5s • Sep 6, 2000 at 00:25",
            "  Worked for 2m 5s • Sep 6, 2000 at 12:25",
            "  Worked for 2m 5s • Sep 6, 2000 at 23:25",
        ],
    );
    let wrapped = Screen::build(Owner::ChatLive)
        .size(24, 24)
        .script(concat!(
            include_str!(
                "fixtures/cells_2/completion_uses_twenty_four_hour_time_with_dates_and_wrapping.jsonl"
            ),
            "\n",
            r#"{"step":"screen","dir":"agents","ev":{"clock":999873120}}"#
        ))
        .open();
    assert_labels(
        &wrapped,
        &[
            "  Worked for 2m 5s • Sep",
            "  6, 2000 at 00:25",
            "  Worked for 2m 5s • Sep",
            "  6, 2000 at 12:25",
            "  Worked for 2m 5s • Sep",
            "  6, 2000 at 23:25",
        ],
    );
}

// ORACLE history_cell/separators_tests.rs:118 completion_without_metadata_has_no_visible_or_raw_lines kind=ui owner=chat-live
#[test]
fn completion_without_metadata_has_no_visible_or_raw_lines() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_2/completion_without_metadata_has_no_visible_or_raw_lines.jsonl"
        ))
        .open();
    // The turn is there; no row says how long it took or when it ended.
    s.assert_contains("Done.");
    s.assert_absent("Worked for");
    s.assert_absent("PM");
    s.assert_absent("AM");
}

// ORACLE history_cell/separators_tests.rs:126 completion_without_timestamp_retains_known_elapsed_duration kind=snap owner=chat-live
#[test]
fn completion_without_timestamp_retains_known_elapsed_duration() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_2/completion_without_timestamp_retains_known_elapsed_duration.jsonl"
        ))
        .open();
    // The oracle pins the raw row `Worked for 2m 5s`; the display row is the same text, indented.
    s.assert_has_row("  Worked for 2m 5s");
    s.assert_absent("•  ");
}

// ORACLE history_cell/separators_tests.rs:135 completion_wraps_metadata_and_preserves_unwrapped_raw_text kind=snap owner=chat-live snap=completion_wraps_metadata_and_preserves_unwrapped_raw_text
#[test]
fn completion_wraps_metadata_and_preserves_unwrapped_raw_text() {
    let s = Screen::build(Owner::ChatLive)
        .size(24, 16)
        .script(include_str!(
            "fixtures/cells_2/completion_wraps_metadata_and_preserves_unwrapped_raw_text.jsonl"
        ))
        .open();
    assert_labels(
        &s,
        &[
            "  Worked for 2m 5s • Sep",
            "  6, 2000 at 2:32 PM •",
            "  Local tools: 3 calls",
            "  (2.5s)",
        ],
    );
    // The raw line (unwrapped) is the raw scrollback's (step 11). The oracle's width loop (0, 1,
    // 5, 24: no line wider than the width) is a property of the screen's width here.
}

// ORACLE history_cell/separators_tests.rs:168 completion_renders_with_dim_default_colors kind=ui owner=chat-live
#[test]
fn completion_renders_with_dim_default_colors() {
    // The oracle's cell has no duration: a restored completion (no `turn/started` in the script,
    // as the sibling test below), so the row reads `  2:32 PM` alone.
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_2/completion_rendering_uses_the_captured_display_date.jsonl"
        ))
        .open();
    let Some(y) = s
        .rows()
        .iter()
        .position(|row| row.trim_end() == "  2:32 PM")
    else {
        panic!("no separator row:\n{}", s.text());
    };
    // The oracle's display row `  2:32 PM` (nine cells) is Reset on Reset and DIM in every cell.
    for x in 0..9 {
        s.assert_style(x, y, "fg=Reset bg=Reset");
        s.assert_painted(x, y, "dim");
    }
}

// ORACLE history_cell/separators_tests.rs:192 completion_rendering_uses_the_captured_display_date kind=snap owner=chat-live
#[test]
fn completion_rendering_uses_the_captured_display_date() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_2/completion_rendering_uses_the_captured_display_date.jsonl"
        ))
        .open();
    s.assert_has_row("  2:32 PM");
    // The raw line `2:32 PM` is the raw scrollback's (step 11).
}

// ---- oracle: history_cell/session_transcript_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/session_transcript_tests.rs:15 wheel_at_hidden_session_header_keeps_the_loaded_message_anchor kind=ui owner=chat-live
#[test]
fn wheel_at_hidden_session_header_keeps_the_loaded_message_anchor() {
    // The oracle's view is 3 rows of the transcript alone; the chat is 12 rows with its composer.
    // The thread is a replayed one whose server has older turns behind a cursor; the wheel at the
    // top of what is loaded asks for them, the answer joins above.
    let recent: Vec<_> = (0..6)
        .rev()
        .map(|n| kit::agent_turn(&format!("recent-{n}"), &[&format!("recent row {n}")]))
        .collect();
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 12)
        .replay(Vec::new())
        .open();
    s.wait_for("Ask Codex to do anything");
    s.history_page(recent, Some("cursor-1"));
    // one wheel gesture after the other to the top of what is loaded
    for _ in 0..5 {
        s.wheel_up(0, 0, 10);
    }
    // the session header is hidden while older pages exist
    s.assert_absent("OpenAI Codex");
    let before = s.shot();
    let older: Vec<_> = (0..3)
        .rev()
        .map(|n| kit::agent_turn(&format!("older-{n}"), &[&format!("older row {n}")]))
        .collect();
    // One wheel gesture must not keep requesting older pages, and the loaded page leaves the
    // rows where they were.
    s.assert_page_requests(2);
    s.history_page(older, None);
    s.assert_same_rows_in(&before, 0..8);
    s.assert_page_requests(2);
    for _ in 0..4 {
        s.wheel_up(0, 0, 10);
    }
    s.assert_contains("older row 0");
}

// ORACLE history_cell/session_transcript_tests.rs:64 session_information_waits_for_older_pages_and_reappears_after_completion kind=ui owner=chat-live
#[test]
fn session_information_waits_for_older_pages_and_reappears_after_completion() {
    // The oracle's stand-in `session announcement` is the pane's `OpenAI Codex` header: hidden
    // while older pages exist (Partial, and while one is Loading), the first row once the
    // history is Complete.
    let rows = |n: usize| -> Vec<_> {
        (0..n)
            .rev()
            .map(|n| kit::agent_turn(&format!("turn-{n}"), &[&format!("row {n}")]))
            .collect()
    };
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 20)
        .replay(Vec::new())
        .open();
    s.wait_for("Ask Codex to do anything");
    s.history_page(rows(2), Some("cursor-1"));
    s.assert_absent("OpenAI Codex");
    // the wheel at the top asks for the next page: loading, the header is still not there
    s.wheel_up(0, 0, 10);
    s.assert_page_requests(2);
    s.assert_absent("OpenAI Codex");
    s.history_page(rows(2), None);
    s.wheel_up(0, 0, 10);
    s.assert_first_row(">_ OpenAI Codex (v0.170.0)");
}
