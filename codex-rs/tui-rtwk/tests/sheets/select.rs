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

// Probe of rtwk gap G39b (`parity/select-dialog.md`). The oracle's snapshot
// `list_selection_col_width_mode_auto_visible_scroll` measures the label column
// over the rows in view (`ColumnWidthMode::AutoVisible`): `Item 1  desc 1` at
// the top, wide only once `Item 9` scrolls in. rtwk's `ListView` has no hook
// for its window at layout time, so the column is measured over every row (the
// `AutoAllRows` look) from the first frame. This pins that; when rtwk gives
// the window, the first frame turns narrow, this test fails, and the oracle
// shard's `snapshot_auto_visible_col_width_mode_scroll_behavior`
// (tests/oracle/picker.rs) loses its `pending(G39b)`. The second frame is the
// oracle's too.
#[test]
fn probe_g39b_auto_visible_measures_the_column_over_every_row() {
    let mut h = open_sized(96, 12, "col_width_auto_visible");
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

/// The tab strip of the dialog as the oracle's `rendered_strip` shows it: the
/// dialog insets its content by two cells, so a screen four cells wider than the
/// strip gives the strip that width, and four rows high (strip, blank, one
/// item, blank) it is the dialog and nothing of the page under it; the strip is
/// the first row.
fn strip_row(h: &Harness) -> String {
    let screen = h.t.screen();
    screen
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .skip(2)
        // The oracle's buffer dump writes a wide glyph and a blank for its second cell.
        .flat_map(|c| {
            if u32::from(c) >= 0x2e80 {
                vec![c, ' ']
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn open_clipping(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = crate::common::start_clipping(true, width, height);
    h.open_debug_row("select", Some(entry));
    h.t.pump_for(ms(800));
    h
}

// The oracle's `filled_tabs_window_around_active_tab` (selection_tabs_tests.rs):
// its snapshot `filled_tabs_window_around_active_tab`, one strip per active tab
// (0, 2, 6) at width 30.
#[test]
fn filled_tabs_window_around_active_tab() {
    let mut h = open_clipping(34, 4, "filled_tabs_seven");
    let mut strips = vec![strip_row(&h)];
    for _ in 0..2 {
        h.press("right");
    }
    h.t.pump_for(ms(100));
    strips.push(strip_row(&h));
    for _ in 0..4 {
        h.press("right");
    }
    h.t.pump_for(ms(100));
    strips.push(strip_row(&h));
    assert_eq!(
        strips,
        [
            " All   Common                \u{203a}",
            "\u{2039}  Common   Customized (3)   \u{203a}",
            "\u{2039}  App   Composer   Debug",
        ],
        "{}",
        h.t.screen()
    );
}

// The oracle's `filled_tabs_truncate_unicode_without_hiding_active_tab`: its
// snapshot, the strip of the second tab at widths 1, 4, 9 and 16.
#[test]
fn filled_tabs_truncate_unicode_without_hiding_active_tab() {
    let strips: Vec<String> = [1u16, 4, 9, 16]
        .iter()
        .map(|width| strip_row(&open_clipping(width + 4, 4, "filled_tabs_unicode")))
        .collect();
    assert_eq!(
        strips,
        [
            "\u{2026}",
            " \u{6587} \u{2026}",
            "\u{2039}  \u{6587} \u{2026}  \u{203a}",
            "\u{2039}  \u{6587} \u{4ef6} \u{7cfb} \u{7edf}  p\u{2026} \u{203a}",
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

// ---- rtwk's means, no window of the sheet's own (select-rtwk-list): the
// controllers (`ListController`, `SelectionController`, `TabController`) are
// the only holders of scroll, cursor and tab; the sheet and the core hold none.

#[test]
fn the_sheet_holds_no_window_cursor_or_tab_of_its_own() {
    let files = [
        (
            "src/widgets/picker/mod.rs",
            include_str!("../../src/widgets/picker/mod.rs"),
        ),
        (
            "src/widgets/picker/rows.rs",
            include_str!("../../src/widgets/picker/rows.rs"),
        ),
        (
            "core/src/screens/select/list.rs",
            include_str!("../../core/src/screens/select/list.rs"),
        ),
    ];
    // `Browse` and `List` take a tab as a parameter (a pure function of it):
    // only the sheet's own fields are state.
    let banned = [
        ("window_top", true),
        ("top: usize", true),
        ("rows[top", true),
        ("measured:", true),
        ("tab: Option<usize>,", false),
        ("item: Option<usize>,", false),
    ];
    let mut found = Vec::new();
    for (name, text) in files {
        let code = text.split("#[cfg(test)]").next().unwrap_or(text);
        for (line, text) in code.lines().enumerate() {
            let text = text.trim_start();
            if text.starts_with("//") {
                continue;
            }
            for (word, everywhere) in banned {
                if text.contains(word) && (everywhere || name.ends_with("picker/mod.rs")) {
                    found.push(format!("{name}:{}: {text}", line + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}
