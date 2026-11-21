//! Screen tests of the `settings` slot (S15): the status-line and terminal
//! title setup sheets (`SetupSheet`). Expected text: the oracle's snapshots
//! are in `tests/oracle/small_dialogs_2.rs`; here are the behaviours the
//! snapshots do not show (keys, the filter, the preview, the mouse).
//! Oracle: `bottom_pane/{multi_select_picker,status_line_setup,title_setup}.rs`.
//! The state tests of the other dialogs (config, theme, keymap) are in
//! `core/tests/settings.rs`.

use crate::common::{Harness, ms, start_sized};

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 80, 24);
    h.open_debug_row("settings", Some(entry));
    h.t.pump_for(ms(800));
    h
}

/// The screen row (y) and the column (x) where `text` starts.
fn find(h: &Harness, text: &str) -> (u16, u16) {
    let screen = h.t.screen();
    for (y, line) in screen.lines().enumerate() {
        if let Some(byte) = line.find(text) {
            let x = line[..byte].chars().count();
            return (
                u16::try_from(y).unwrap_or(u16::MAX),
                u16::try_from(x).unwrap_or(u16::MAX),
            );
        }
    }
    panic!("{text} is not on screen:\n{screen}");
}

/// The selected row's text (the `›` marker row), trimmed.
fn cursor_row(h: &Harness) -> String {
    let screen = h.t.screen();
    let rows: Vec<&str> = screen.lines().filter(|l| l.contains('›')).collect();
    assert_eq!(rows.len(), 1, "one cursor row\n{screen}");
    rows[0].trim().to_string()
}

#[test]
fn the_sheet_opens_on_the_theme_row_with_the_runtime_preview() {
    let h = open("status_line_preview");
    let screen = h.t.screen();
    assert!(screen.contains("Configure Status Line"), "{screen}");
    assert!(screen.contains("› [x] Use theme colors"), "{screen}");
    assert!(
        screen.contains("gpt-5-codex · ~/codex-rs · jif/statusline-preview"),
        "{screen}"
    );
}

#[test]
fn space_toggles_the_item_and_the_preview_follows() {
    let mut h = open("status_line_preview");
    assert!(h.t.screen().contains("gpt-5-codex · ~/codex-rs"));
    // the first row is the theme-colors switch; the items follow it
    h.press("down");
    h.press("space");
    let screen = h.t.screen();
    assert!(screen.contains("› [ ] model"), "{screen}");
    assert!(
        !screen.contains("gpt-5-codex · ~/codex-rs"),
        "the preview drops the item\n{screen}"
    );
    h.press("space");
    assert!(h.t.screen().contains("› [x] model"), "{}", h.t.screen());
}

#[test]
fn the_arrows_move_the_cursor_and_skip_nothing_in_the_item_rows() {
    let mut h = open("status_line_preview");
    let first = cursor_row(&h);
    h.press("down");
    let second = cursor_row(&h);
    assert_ne!(first, second);
    h.press("up");
    assert_eq!(cursor_row(&h), first);
    h.press("end");
    let last = cursor_row(&h);
    assert_ne!(last, first);
    h.press("home");
    assert_eq!(cursor_row(&h), first);
}

#[test]
fn right_moves_the_item_down_the_order_and_left_back() {
    let mut h = open("status_line_preview");
    // model, current-dir, git-branch are the selected order.
    h.press("down");
    h.press("right");
    let screen = h.t.screen();
    assert!(
        screen.contains("~/codex-rs · gpt-5-codex · jif/statusline-preview"),
        "{screen}"
    );
    h.press("left");
    assert!(
        h.t.screen()
            .contains("gpt-5-codex · ~/codex-rs · jif/statusline-preview"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn typing_filters_and_left_right_stay_with_the_search_field() {
    let mut h = open("status_line_preview");
    for key in "branch".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("git-branch"), "{screen}");
    assert!(!screen.contains("current-dir"), "{screen}");
    let preview = "gpt-5-codex · ~/codex-rs · jif/statusline-preview";
    assert!(screen.contains(preview), "{screen}");
    // no reorder while a query stands (the oracle's `reorder` guard)
    h.press("left");
    h.press("right");
    assert!(h.t.screen().contains(preview), "{}", h.t.screen());
}

#[test]
fn enter_saves_and_closes() {
    let mut h = open("status_line_preview");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Configure Status Line"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn esc_cancels_and_closes() {
    let mut h = open("status_line_preview");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Configure Status Line"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_title_sheet_lists_its_own_items() {
    let h = open("terminal_title");
    let screen = h.t.screen();
    assert!(screen.contains("Configure Terminal Title"), "{screen}");
    assert!(screen.contains("project-name"), "{screen}");
}

/// ADR-011 (beyond the oracle): a click on an item puts the cursor there, a
/// second click on the item the cursor is on toggles it.
#[test]
fn a_click_moves_the_cursor_and_a_second_click_toggles() {
    let mut h = open("status_line_preview");
    let (y, x) = find(&h, "current-dir");
    h.t.click(x, y);
    h.settle();
    assert!(cursor_row(&h).contains("current-dir"), "{}", h.t.screen());
    assert!(cursor_row(&h).contains("[x]"), "{}", h.t.screen());
    h.t.click(x, y);
    h.settle();
    assert!(cursor_row(&h).contains("[ ]"), "{}", h.t.screen());
}
