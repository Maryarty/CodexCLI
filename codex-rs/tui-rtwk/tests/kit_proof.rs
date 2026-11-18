//! The proof of the screen-test kit (`tests/kit/`): three oracle tests of
//! landed screens ported with it, and the kit's own tests. Every test here is
//! green; the pending, todo and n/a markers are shown by tests that expect
//! them.

mod common;
mod kit;

use kit::{Owner, Screen};

// ---- three oracle tests, ported (recipe: docs/rtwk-port/refs/screen-tests.md) ----

/// Oracle: `resume_picker.rs` `resume_search_error_snapshot`. Seed: the
/// resume screen's entry for a failed listing; the snapshot is the search-line
/// region (screen row 2, one column in) of an 80 x 13 screen.
#[test]
fn resume_search_error_snapshot() {
    Screen::build(Owner::RoutesLists)
        .size(80, 13)
        .entry("resume", "search_error")
        .open()
        .assert_snap_region("resume_picker_search_error", 2, 1);
}

/// Oracle: `resume_picker.rs` `mixed_backend_resume_table_snapshot`: the oracle
/// selects the second row; its snapshot is the list region of the screen (our
/// screen is 82 wide: the list starts two columns in, at screen row 6).
#[test]
fn mixed_backend_resume_table_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.keys("down");
    s.assert_snap_region("resume_picker_table", 6, 2);
}

/// Oracle: `resume_picker.rs` `hint_line_snapshot_uses_distributed_wide_footer`:
/// the footer region, at screen row 18.
#[test]
fn hint_line_snapshot_uses_distributed_wide_footer() {
    Screen::build(Owner::RoutesLists)
        .size(220, 22)
        .entry("resume", "footer_wide")
        .open()
        .assert_snap_region("resume_picker_footer_wide", 18, 0);
}

// ---- the verdict markers ----

#[test]
#[should_panic(expected = "PENDING[dialogs-pane]: the bottom pane does not draw this yet")]
fn pending_fails_with_its_owner_and_reason() {
    kit::pending(
        Owner::DialogsPane,
        "the bottom pane does not draw  this yet",
    );
    unreachable!("the body of a pending test is not reached outside a probe");
}

#[test]
#[should_panic(expected = "PENDING[ui-pane]: no screen directory `no_such_dir`")]
fn a_seed_that_does_not_exist_is_pending_not_a_crash() {
    drop(
        Screen::build(Owner::UiPane)
            .entry("no_such_dir", "x")
            .open(),
    );
}

#[test]
#[should_panic(expected = "PENDING[ui-pane]: no debug entry `status/no_such_entry`")]
fn a_missing_entry_is_pending_with_its_name() {
    drop(
        Screen::build(Owner::UiPane)
            .entry("status", "no_such_entry")
            .open(),
    );
}

#[test]
#[should_panic(expected = "TODO[port]")]
fn a_stub_is_todo() {
    kit::todo();
}

#[test]
fn na_passes_and_says_whose() {
    kit::na(Owner::SessionCore, "needs the real App to resume a thread");
}

// ---- the comparison ----

#[test]
#[should_panic(expected = "ours (right) differs from the oracle (left)")]
fn a_different_row_fails_and_is_never_loosened() {
    // The status card is not the resume table.
    Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("status", "reasoning_details")
        .open()
        .assert_snap("resume_picker_table");
}

#[test]
fn labels_of_keys_are_normalized_on_our_rows_only() {
    let none: Vec<String> = Vec::new();
    assert_eq!(
        kit::normalize_labels("shift+tab to cycle", &none),
        "⇧tab to cycle"
    );
    assert_eq!(kit::normalize_labels("ctrl + c quit", &none), "⌃c quit");
    assert_eq!(kit::normalize_labels("alt+x ", &none), "⌥x ");
    assert_eq!(kit::normalize_labels("shift-tab", &none), "shift-tab");
    assert_eq!(kit::normalize_labels("alternate", &none), "alternate");
    // The oracle's own text holds the word form: ours is left alone.
    let oracle = vec!["  ctrl + c to quit".to_string()];
    assert_eq!(
        kit::normalize_labels("ctrl + c quit", &oracle),
        "ctrl + c quit"
    );
}

#[test]
fn snapshots_are_found_by_test_name_or_file_stem() {
    let by_name = kit::snap_size("resume_picker_table");
    let by_stem = kit::snap_size("codex_tui__resume_picker__tests__resume_picker_table");
    assert_eq!(by_name, by_stem);
    assert!(by_name.0 > 40 && by_name.1 > 3, "{by_name:?}");
}

// ---- the seeds and inputs ----

/// A script feeds the live chat as a server would; the chat shows it.
#[test]
fn a_script_seeds_the_live_chat() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("kit/fixtures/answered.jsonl"))
        .open();
    s.wait_for("• Hello there");
    s.assert_contains("› hi");
    s.assert_absent("never shown");
}

/// Keys reach the screen: typing fills the composer.
#[test]
fn keys_and_text_reach_the_live_chat() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("draft");
    s.assert_contains("draft");
}

/// The mouse reaches the screen: a click on a row text and a wheel turn are
/// delivered without a crash; the effects are the slices' (their tests).
#[test]
fn mouse_helpers_deliver() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.click_text("Explain the codebase");
    s.double_click(3, 8);
    s.wheel_down(10, 8, 2);
    s.wheel_up(10, 8, 2);
    s.assert_contains("Fix resume picker timestamps");
}
