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

#[test]
fn a_multi_select_flips_the_row_under_the_cursor_with_space_and_enter_confirms() {
    let mut h = open(60, "multi_select");
    let screen = h.t.screen();
    assert!(
        screen.contains("[x] Alpha") && screen.contains("[ ] Beta") && screen.contains("[x] Gamma"),
        "{screen}"
    );
    h.press("down");
    h.press("space");
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("[x] Alpha") && screen.contains("[x] Beta") && screen.contains("[x] Gamma"),
        "space flipped Beta: {screen}"
    );
    h.press("enter");
    h.t.pump_for(ms(800));
    let screen = h.t.screen();
    assert!(
        !screen.contains("[x] Alpha") && !screen.contains("[ ] Beta"),
        "enter confirmed: {screen}"
    );
}

#[test]
fn a_number_key_flips_its_row_of_a_multi_select_and_keeps_it_open() {
    let mut h = open(60, "multi_select");
    h.press("1");
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("[ ] Alpha") && screen.contains("Choose"),
        "{screen}"
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

/// Probe of rtwk gap G45 (`filled_tabs_window_around_active_tab`): rtwk's
/// `TabBar` scrolls by columns and cuts the tab at the edge; the oracle's
/// strip (`render_filled_tab_bar`) shows whole tabs only, keeps the active one,
/// and reserves two cells for each arrow (the probe asserts what rtwk draws:
/// the third tab whole where the oracle drops it, and at the end the cut `nd (4)`):
/// `" All   Common                ›"`,
/// `"‹  Common   Customized (3)   ›"`, `"‹  App   Composer   Debug"`.
#[test]
fn probe_g45_tab_bar_cuts_a_tab_at_the_edge() {
    use rtwk::controls::{TabBar, TabController};
    use rtwk_test::{Options, Tester};
    let labels = [
        "All",
        "Common",
        "Customized (3)",
        "Unbound (4)",
        "App",
        "Composer",
        "Debug",
    ];
    let strip = |active: usize| {
        let mut t = Tester::with(Options::new().size(30, 1));
        t.pump_widget(TabBar::new(
            TabController::with_initial_index(7, active),
            labels,
        ));
        t.screen()
            .lines()
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_string()
    };
    assert_eq!(
        [strip(0), strip(2), strip(6)],
        [
            " All  Common  Customized (3) \u{203a}".to_string(),
            " All  Common  Customized (3) \u{203a}".to_string(),
            "\u{2039}nd (4)  App  Composer  Debug".to_string(),
        ]
    );
}

// The oracle's `side_layout_width_half_uses_exact_split` as rows: at 120 cells
// the content is 116 wide, the preview half of what the gap leaves (57), and
// it starts at 120 - 2 - 57 = 61.
#[test]
fn a_preview_sits_beside_the_rows_when_it_fits() {
    let h = open_sized(120, 8, "side_content");
    let have = rows(&h);
    let at = |text: &str| have.iter().find_map(|row| row.find(text));
    assert_eq!(at("W"), Some(61), "the screen:\n{}", h.t.screen());
    assert_eq!(at("N"), None);
    assert_eq!(at("1. Item 1"), Some(4));
}

// ---- the dialogs that are this sheet with other words: their oracle snapshots
// (chatwidget/snapshots), at the width the snapshots were taken.

#[test]
fn skills_menu_snapshot() {
    let h = open(80, "skills_menu");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__skills_menu_default_mentions_shortcut.snap",
    );
}

#[test]
fn copy_picker_snapshot() {
    let h = open(80, "copy_picker");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__slash_copy_picker_status_fields.snap",
    );
}

#[test]
fn apps_loading_snapshot() {
    let h = open(80, "apps_loading");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__apps_popup_loading_state.snap",
    );
}

#[test]
fn worktrees_conversation_snapshot() {
    let h = open(80, "worktrees_conversation");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__worktrees_conversation_choices.snap",
    );
}

#[test]
fn worktrees_fork_snapshot() {
    let h = open(80, "worktrees_fork");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__worktrees_fork_choices.snap",
    );
}

#[test]
fn worktrees_new_snapshot() {
    let h = open(80, "worktrees_new");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__worktrees_new_choices.snap",
    );
}
