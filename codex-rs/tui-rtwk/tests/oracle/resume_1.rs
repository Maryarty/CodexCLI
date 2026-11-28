//! Shard `resume_1` (screens S18, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The server's `thread/list` answer: one session of `cwd` on `branch` (none: no git info),
/// created and updated at the given Unix seconds.
fn listing(
    preview: &str,
    cwd: &str,
    branch: Option<&str>,
    created: u64,
    updated: u64,
) -> serde_json::Value {
    let id = "00000000-0000-0000-0000-000000000015";
    let git =
        branch.map(|name| serde_json::json!({"sha": null, "branch": name, "originUrl": null}));
    serde_json::json!({"nextCursor": null, "data": [{
        "id": id, "sessionId": id, "environments": null, "extra": null,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": preview, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai", "model": null,
        "reasoningEffort": null, "createdAt": created, "updatedAt": updated,
        "recencyAt": updated, "status": {"type": "idle"}, "path": "session.jsonl", "cwd": cwd,
        "cliVersion": "0.0.0", "originator": null, "source": "cli", "canAcceptDirectInput": true,
        "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": git,
        "name": null, "daybreakEnabled": null, "turns": []
    }]})
}

// ---- oracle: resume_picker.rs (32 screen-level tests here; 15 state tests left to routes-lists)
// ORACLE resume_picker.rs:4301 linked_worktree_sessions_follow_directory_scope kind=snap owner=routes-lists snap=resume_picker_linked_worktree
#[test]
fn linked_worktree_sessions_follow_directory_scope() {
    // The oracle hides the linked worktree's session until the filter moves to all checkouts
    // (`change_filter_mode(Next)`: the filter control, then right): the picker is the live one,
    // scoped to the primary checkout of a repository that has a linked one, and the server
    // lists the session of the linked checkout again when the filter changes.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(104, 12)
        .dir("resume")
        .clock(1_735_776_000)
        .open();
    s.feed_dir(
        r#"{"step":"screen","dir":"resume","ev":{"scope":{"cwd":"/repo/primary","repo_cwds":["/repo/primary","/repo/linked"]}}}"#,
    );
    s.server_answers(
        "thread/list",
        listing(
            "Linked session",
            "/repo/linked",
            None,
            1_735_689_600,
            1_735_689_600,
        ),
    );
    // the filter control has the focus (the oracle's `change_filter_mode(Next)`): Cwd -> Repo
    s.keys("right");
    // The list stands at row 3 (the title, the toolbar and the search line above it), 100 cells
    // wide between its two-cell insets (the oracle renders it at 100x12): its two list rows.
    s.assert_snap_region("resume_picker_linked_worktree", 3, 2);
}

// ORACLE resume_picker.rs:4564 local_db_first_falls_back_when_initial_db_page_is_empty kind=snap owner=routes-lists snap=resume_picker_db_fallback_transition
#[test]
fn local_db_first_falls_back_when_initial_db_page_is_empty() {
    // The oracle's two states, loading then the repaired rows; the screen shows
    // the second one (the listing of the fallback source is the loader's).
    let s = Screen::build(Owner::RoutesLists)
        .size(84, 24)
        .entry("resume", "db_fallback")
        .open();
    // The list is at row 6 of the screen, two cells in.
    s.snap("resume_picker_db_fallback_transition")
        .frame("repaired results")
        .at(6, 2)
        .assert();
}

// ORACLE resume_picker.rs:4658 local_db_first_accepts_db_thread_id_and_latest_history_mode kind=ui owner=routes-lists
#[test]
fn local_db_first_accepts_db_thread_id_and_latest_history_mode() {
    kit::na(
        Owner::RoutesLists,
        "asserts the selection that Enter returns (thread id, cwd, history mode), not rows: a state test",
    );
}

// ORACLE resume_picker.rs:4759 expanded_session_details_include_metadata kind=ui owner=routes-lists
#[test]
fn expanded_session_details_include_metadata() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(120, 24)
        .entry("resume", "expanded_details")
        .open();
    s.keys("ctrl+e");
    // The oracle renders the details of the expanded row at 120 cells; they are rows of the list.
    s.assert_contains(
        "Session:    feat(tui): add raw scrollback mode (019dabc1-0ef5-7431-b81c-03037f51f62c)",
    );
    s.assert_contains("Created:    17 minutes ago · 2026-05-02 14:31:08");
    s.assert_contains("Updated:    now · 2026-05-02 14:48:19");
    s.assert_contains("Directory:  /Users/felipe.coury/code/codex");
    s.assert_contains("Branch:     \u{e0a0} codex/raw-scrollback-mode");
    s.assert_contains("Conversation:");
}

// ORACLE resume_picker.rs:4806 footer_prioritizes_active_sort_timestamp kind=ui owner=routes-lists
#[test]
fn footer_prioritizes_active_sort_timestamp() {
    let s = Screen::build(Owner::RoutesLists)
        .size(84, 12)
        .entry("resume", "footer_times")
        .open();
    // Updated sort (the default): the footer leads with the updated time.
    s.assert_contains("  3h ago");
    s.assert_absent("created 5h ago");
    s.assert_in_order(&["⌁ tmp/codex", " main"]);
    // Created sort: the sort change reloads the list, which only a live picker has a server
    // for: the fake backend lists the session (updated 3h, created 5h before the clock) and
    // the footer leads with the created time.
    let now: u64 = 1_777_723_200;
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 12)
        .dir("resume")
        .clock(i64::try_from(now).unwrap_or_default())
        .open();
    s.server_answers(
        "thread/list",
        listing(
            "Footer row",
            "/tmp/codex",
            Some("main"),
            now - 5 * 3600,
            now - 3 * 3600,
        ),
    );
    s.keys("tab tab right");
    s.assert_contains("  5h ago");
    s.assert_absent("updated 3h ago");
    s.assert_in_order(&["⌁ /tmp/codex", " main"]);
}

// ORACLE resume_picker.rs:4837 footer_marks_missing_branch kind=ui owner=routes-lists
#[test]
fn footer_marks_missing_branch() {
    let s = Screen::build(Owner::RoutesLists)
        .size(84, 12)
        .entry("resume", "footer_no_branch")
        .open();
    s.assert_contains("⌁ /tmp/codex");
    s.assert_contains(" no branch");
    s.assert_in_order(&["⌁ /tmp/codex", " no branch"]);
}

// ORACLE resume_picker.rs:4856 footer_branch_expands_when_line_has_room kind=ui owner=routes-lists
#[test]
fn footer_branch_expands_when_line_has_room() {
    let s = Screen::build(Owner::RoutesLists)
        .size(144, 12)
        .entry("resume", "footer_long_branch")
        .open();
    s.assert_contains("etraut/animations-false-improvements");
}

// ORACLE resume_picker.rs:4873 footer_cwd_truncates_to_responsive_column kind=ui owner=routes-lists
#[test]
fn footer_cwd_truncates_to_responsive_column() {
    let s = Screen::build(Owner::RoutesLists)
        .size(84, 12)
        .entry("resume", "footer_long_cwd")
        .open();
    s.assert_absent(
        "~/code/codex.owner-extremely-long-worktree-name-that-needs-truncating/codex-rs",
    );
    s.assert_contains("⌁ ~/code/codex.");
    s.assert_contains("...");
    s.assert_contains(" owner/branch");
}

// ORACLE resume_picker.rs:4895 footer_omits_cwd_when_hidden kind=ui owner=routes-lists
#[test]
fn footer_omits_cwd_when_hidden() {
    let s = Screen::build(Owner::RoutesLists)
        .size(84, 12)
        .entry("resume", "footer_no_cwd")
        .open();
    s.assert_contains("4h ago");
    s.assert_contains(" owner/branch");
    s.assert_absent("⌁");
    s.assert_absent("~/code");
}

// ORACLE resume_picker.rs:5037 mixed_backend_resume_table_snapshot kind=snap owner=routes-lists snap=resume_picker_table
#[test]
fn mixed_backend_resume_table_snapshot() {
    // Our screen is 84 wide (the list is the screen less its two-cell inset on each side); the
    // oracle's test selects the second row.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("down");
    s.assert_snap_region("resume_picker_table", 6, 2);
}

// ORACLE resume_picker.rs:5139 resume_search_error_snapshot kind=snap owner=routes-lists snap=resume_picker_search_error
#[test]
fn resume_search_error_snapshot() {
    Screen::build(Owner::RoutesLists)
        .size(80, 13)
        .entry("resume", "search_error")
        .open()
        .assert_snap_region("resume_picker_search_error", 2, 1);
}

// ORACLE resume_picker.rs:5256 hint_line_compacts_on_narrow_width kind=ui owner=routes-lists
#[test]
fn hint_line_compacts_on_narrow_width() {
    let s = Screen::build(Owner::RoutesLists)
        .size(119, 12)
        .entry("resume", "footer_narrow")
        .open();
    s.assert_contains("esc new");
    s.assert_contains("tab focus");
    s.assert_contains("←/→ option");
    s.assert_contains("⌃o dense");
    s.assert_contains("⌃t preview");
    s.assert_contains("⌃e exp");
    s.assert_absent("focus sort/filter");
}

// ORACLE resume_picker.rs:5280 hint_line_snapshot_uses_distributed_wide_footer kind=snap owner=routes-lists snap=resume_picker_footer_wide
#[test]
fn hint_line_snapshot_uses_distributed_wide_footer() {
    Screen::build(Owner::RoutesLists)
        .size(220, 22)
        .entry("resume", "footer_wide")
        .open()
        .assert_snap_region("resume_picker_footer_wide", 18, 0);
}

// ORACLE resume_picker.rs:5299 hint_line_snapshot_uses_compact_footer kind=snap owner=routes-lists snap=resume_picker_footer_compact
#[test]
fn hint_line_snapshot_uses_compact_footer() {
    // The oracle's query `picker` and dense density, at 96 cells; the footer is the
    // screen's bottom four rows (row 18 of 22), its three snapshot rows.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(96, 22)
        .entry("resume", "footer_wide")
        .open();
    s.type_text("picker");
    s.keys("ctrl+o");
    s.assert_snap_region("resume_picker_footer_compact", 18, 0);
}

// ORACLE resume_picker.rs:5320 hint_line_preserves_primary_action_labels_when_very_narrow kind=ui owner=routes-lists
#[test]
fn hint_line_preserves_primary_action_labels_when_very_narrow() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(38, 12)
        .entry("resume", "footer_very_narrow")
        .open();
    s.keys("ctrl+o");
    // The oracle's two hint rows, each within the width.
    s.assert_has_row(" enter resume   esc new   ⌃c quit");
    s.assert_has_row(" ⌃o comfy   ⌃t preview   ↑/↓ browse");
}

// ORACLE resume_picker.rs:5349 hint_line_shows_loading_transcript_mode kind=ui owner=routes-lists
#[test]
fn hint_line_shows_loading_transcript_mode() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    s.assert_contains("loading transcript");
    s.assert_contains("⌃c quit");
    s.assert_absent("enter");
}

// ORACLE resume_picker.rs:5516 ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn ctrl_o_toggles_density_without_typing_into_search() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.type_text("pick");
    s.assert_contains("⌃o dense");
    s.keys("ctrl+o");
    // Dense now (the footer offers the way back), and the search row still holds "pick".
    s.assert_contains("⌃o comfy");
    s.assert_contains("pick");
}

// ORACLE resume_picker.rs:5539 ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn ctrl_t_requests_selected_session_transcript() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    // The request sent to the loader is the loader's; the screen shows the loading mode.
    s.assert_contains("loading transcript");
}

// ORACLE resume_picker.rs:5586 transcript_loading_consumes_picker_input kind=ui owner=routes-lists
#[test]
fn transcript_loading_consumes_picker_input() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    let before = s.shot();
    // While a transcript loads, down and typed characters change nothing on the screen.
    s.keys("down");
    s.assert_same_rows(&before);
    s.type_text("x");
    s.assert_same_rows(&before);
}

// ORACLE resume_picker.rs:5645 escape_cancels_transcript_loading_and_restores_picker_navigation kind=ui owner=routes-lists
#[test]
fn escape_cancels_transcript_loading_and_restores_picker_navigation() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    s.assert_contains("loading transcript");
    s.keys("esc");
    s.assert_absent("loading transcript");
    // Navigation is back: down moves the selection to the second row.
    let before = s.shot();
    s.keys("down");
    s.assert_rows_differ(&before);
}

// ORACLE resume_picker.rs:5702 transcript_loading_still_allows_ctrl_c_exit kind=ui owner=routes-lists
#[test]
fn transcript_loading_still_allows_ctrl_c_exit() {
    kit::na(
        Owner::RoutesLists,
        "asserts the Exit selection that ctrl+c returns, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5724 transcript_loading_overlay_snapshot kind=snap owner=routes-lists snap=resume_picker_transcript_loading_overlay
#[test]
fn transcript_loading_overlay_snapshot() {
    // The oracle's 80-cell list with the loading overlay on the second row; the
    // list is at row 6 of the screen and seven rows high at 17 rows (the oracle renders seven).
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 17)
        .entry("resume", "transcript_loading")
        .open();
    s.keys("ctrl+t");
    s.assert_snap_region("resume_picker_transcript_loading_overlay", 6, 2);
}

// ORACLE resume_picker.rs:5793 raw_ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn raw_ctrl_t_requests_selected_session_transcript() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    s.assert_contains("loading transcript");
}

// ORACLE resume_picker.rs:5833 ctrl_t_on_row_without_thread_id_shows_inline_error kind=ui owner=routes-lists
#[test]
fn ctrl_t_on_row_without_thread_id_shows_inline_error() {
    kit::na(
        Owner::RoutesLists,
        "a listed row always carries its thread id (core `Row.id` is not optional, the protocol's thread has one), so a row without one cannot exist in the port; the transcript of a thread that is not listed is core's `a_transcript_is_asked_for_and_marked_loaded` and the missing-transcript problem `Problem::NoTranscript`",
    );
}

// ORACLE resume_picker.rs:5869 loaded_transcript_waits_for_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn loaded_transcript_waits_for_loading_frame_before_opening_overlay() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle opens the transcript pager (`Overlay::Transcript`, S16 `pager`, not built) on the frame after the loading frame, routes the keymap's chords in it and acknowledges the unread revision; the picker's part (ctrl+t marks the transcript loading and draws the loading overlay, the loaded transcript is state) is the oracle shard `transcript_loading_overlay_snapshot` and core's `a_transcript_is_asked_for_and_marked_loaded`",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    // The loading frame is drawn first; the overlay opens on the frame after it.
    s.assert_contains("loading transcript");
    s.wait(100);
    s.assert_absent("loading transcript");
    s.assert_contains("BAtail");
    // (the chord routing of ctrl+space and the leader keymap is state: not asserted here)
}

// ORACLE resume_picker.rs:6020 cached_transcript_still_shows_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn cached_transcript_still_shows_loading_frame_before_opening_overlay() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle opens the transcript pager (`Overlay::Transcript`, S16 `pager`, not built) on the frame after the loading frame, routes the keymap's chords in it and acknowledges the unread revision; the picker's part (ctrl+t marks the transcript loading and draws the loading overlay, the loaded transcript is state) is the oracle shard `transcript_loading_overlay_snapshot` and core's `a_transcript_is_asked_for_and_marked_loaded`",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+t");
    // Even a cached transcript shows the loading frame, then the overlay.
    s.assert_contains("loading transcript");
    s.wait(100);
    s.assert_absent("loading transcript");
}

// ORACLE resume_picker.rs:6071 ctrl_o_persists_density_preference kind=ui owner=routes-lists
#[test]
fn ctrl_o_persists_density_preference() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+o");
    // Visible half: the density is dense (the saved preference is not drawn).
    s.assert_contains("⌃o comfy");
}

// ORACLE resume_picker.rs:6104 ctrl_o_keeps_toggled_density_when_persistence_fails kind=ui owner=routes-lists
#[test]
fn ctrl_o_keeps_toggled_density_when_persistence_fails() {
    kit::pending(
        Owner::RoutesLists,
        "no seed with a view-mode preference store that fails to save",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+o");
    s.assert_contains("⌃o comfy");
    // The failed save is shown as the inline error.
    s.assert_contains("preference");
}

// ORACLE resume_picker.rs:6139 raw_ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn raw_ctrl_o_toggles_density_without_typing_into_search() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.type_text("pick");
    s.keys("ctrl+o");
    s.assert_contains("⌃o comfy");
    s.assert_contains("pick");
}

// ORACLE resume_picker.rs:6162 space_appends_to_search_query kind=ui owner=routes-lists
#[test]
fn space_appends_to_search_query() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.type_text("resize");
    s.keys("space");
    s.type_text("r");
    // The query is "resize r" and no row is expanded.
    s.assert_contains("resize r");
    s.assert_absent("Conversation:");
}

// ORACLE resume_picker.rs:6189 ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn ctrl_e_toggles_selected_session_expansion() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+e");
    // The selected row is expanded (its details are shown) ...
    s.assert_contains("Conversation:");
    s.keys("ctrl+e");
    // ... and a second ctrl+e collapses it.
    s.assert_absent("Conversation:");
}

// ORACLE resume_picker.rs:6240 raw_ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn raw_ctrl_e_toggles_selected_session_expansion() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.keys("ctrl+e");
    s.assert_contains("Conversation:");
}
