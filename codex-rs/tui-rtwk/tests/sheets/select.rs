//! Screen tests of S14 (the generic selection dialog), two parameterisations:
//! the model picker and the permissions menu. Expected text: the oracle's
//! snapshots (`dialogs::oracle`). Oracle tests: `bottom_pane/list_selection_view.rs`
//! (`snapshot_model_picker_width_80`), `chatwidget/tests/permissions.rs`
//! (`approvals_selection_popup_snapshot`).

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::assert_rows;

fn open(width: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, 24);
    h.open_debug_row("select", Some(entry));
    h.t.pump_for(ms(800));
    h
}

#[test]
fn snapshot_model_picker_width_80() {
    let h = open(80, "model_picker");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__list_selection_view__tests__list_selection_model_picker_width_80.snap",
    );
}

#[test]
fn approvals_selection_popup_snapshot() {
    let h = open(80, "permissions_menu");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__approvals_selection_popup.snap",
    );
}

#[test]
fn enter_picks_the_highlighted_row_and_closes() {
    let mut h = open(100, "model_picker");
    h.press("down");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_number_key_picks_its_row() {
    let mut h = open(100, "permissions_menu");
    h.press("2");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn esc_closes_without_a_pick() {
    let mut h = open(100, "model_picker");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
    );
}

/// The rows of the screen, trailing blanks cut.
fn rows(h: &Harness) -> Vec<String> {
    h.t.screen()
        .lines()
        .map(|row| row.trim_end().to_string())
        .collect()
}

fn assert_exact(h: &Harness, want: &[&str]) {
    let mut have = rows(h);
    let mut want: Vec<String> = want.iter().map(|row| (*row).to_string()).collect();
    for list in [&mut have, &mut want] {
        while list.last().is_some_and(String::is_empty) {
            list.pop();
        }
    }
    assert_eq!(have, want, "the screen:\n{}", h.t.screen());
}

fn open_sized(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("select", Some(entry));
    h.t.pump_for(ms(800));
    h
}

// The oracle's snapshot `list_selection_col_width_mode_auto_visible_scroll`
// (bottom_pane/snapshots), its two frames: the kit cannot address them, so the
// rows are the snapshot's text.
#[test]
fn snapshot_auto_visible_col_width_mode_scroll_behavior() {
    let mut h = open_sized(96, 12, "col_width_auto_visible");
    assert_exact(
        &h,
        &[
            "  Debug",
            "",
            "",
            "\u{203a} 1. Item 1  desc 1",
            "  2. Item 2  desc 2",
            "  3. Item 3  desc 3",
            "  4. Item 4  desc 4",
            "  5. Item 5  desc 5",
            "  6. Item 6  desc 6",
            "  7. Item 7  desc 7",
            "  8. Item 8  desc 8",
            "\u{2193}",
        ],
    );
    for _ in 0..8 {
        h.press("down");
    }
    h.t.pump_for(ms(100));
    assert_exact(
        &h,
        &[
            "  Debug",
            "",
            "\u{2191}",
            "  2. Item 2                                         desc 2",
            "  3. Item 3                                         desc 3",
            "  4. Item 4                                         desc 4",
            "  5. Item 5                                         desc 5",
            "  6. Item 6                                         desc 6",
            "  7. Item 7                                         desc 7",
            "  8. Item 8                                         desc 8",
            "\u{203a} 9. Item 9 with an intentionally much longer name  desc 9",
            "",
        ],
    );
}

// The oracle's snapshot `list_selection_col_width_mode_auto_all_rows_scroll`.
#[test]
fn snapshot_auto_all_rows_col_width_mode_scroll_behavior() {
    let mut h = open_sized(96, 12, "col_width_auto_all_rows");
    assert_exact(
        &h,
        &[
            "  Debug",
            "",
            "",
            "\u{203a} 1. Item 1                                         desc 1",
            "  2. Item 2                                         desc 2",
            "  3. Item 3                                         desc 3",
            "  4. Item 4                                         desc 4",
            "  5. Item 5                                         desc 5",
            "  6. Item 6                                         desc 6",
            "  7. Item 7                                         desc 7",
            "  8. Item 8                                         desc 8",
            "\u{2193}",
        ],
    );
    for _ in 0..8 {
        h.press("down");
    }
    h.t.pump_for(ms(100));
    assert_exact(
        &h,
        &[
            "  Debug",
            "",
            "\u{2191}",
            "  2. Item 2                                         desc 2",
            "  3. Item 3                                         desc 3",
            "  4. Item 4                                         desc 4",
            "  5. Item 5                                         desc 5",
            "  6. Item 6                                         desc 6",
            "  7. Item 7                                         desc 7",
            "  8. Item 8                                         desc 8",
            "\u{203a} 9. Item 9 with an intentionally much longer name  desc 9",
            "",
        ],
    );
}
