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
    let h = open(100, "model_picker");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__list_selection_view__tests__list_selection_model_picker_width_80.snap",
    );
}

#[test]
fn approvals_selection_popup_snapshot() {
    let h = open(100, "permissions_menu");
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
