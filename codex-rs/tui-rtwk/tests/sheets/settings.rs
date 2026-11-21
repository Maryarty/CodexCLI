//! Screen tests of the `settings` slot (S15): the status-line and terminal
//! title setup sheets (`SetupSheet`) and the syntax theme picker (`ThemeSheet`). Expected text: the oracle's snapshots
//! are in `tests/oracle/small_dialogs_2.rs`; here are the behaviours the
//! snapshots do not show (keys, the filter, the preview, the mouse).
//! Oracle: `bottom_pane/{multi_select_picker,status_line_setup,title_setup}.rs`.
//! The state tests of the other dialogs (config, theme, keymap) are in
//! `core/tests/settings.rs`.

use rtwk_core::input::{Mods, MouseKind};

use crate::common::{Harness, ms, start_sized};

fn open(entry: &str) -> Harness {
    open_sized(entry, 80, 24)
}

fn open_sized(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
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

// ---- the theme picker (`ThemeSheet`): oracle `theme_picker.rs`,
// `bottom_pane/list_selection_view.rs` ----
// What the intents do to the model (the theme previewed on every move, kept
// by Enter, restored by Esc) is `core/tests/settings.rs`: a debug entry's
// model lives in its seeded scope, so these tests see the intents only as
// the sheet closing.

#[test]
fn the_theme_picker_in_a_narrow_terminal_stacks_the_preview_below_the_rows() {
    let h = open("theme_picker");
    let screen = h.t.screen();
    assert!(screen.contains("Select Syntax Theme"), "{screen}");
    // 76 content cells hold the custom directory's sentence
    assert!(
        screen.contains("Custom .tmTheme files can be added to the ~/.codex/themes directory."),
        "{screen}"
    );
    assert!(screen.contains("› ada (current)"), "{screen}");
    assert!(screen.contains("Type to filter themes..."), "{screen}");
    // the compact sample: its number column, the sign column, the code
    assert!(
        screen.contains("12  fn greet(name: &str) -> String {"),
        "{screen}"
    );
    assert!(
        screen.contains("13 -    format!(\"Hello, {}!\", name)"),
        "{screen}"
    );
    assert!(
        screen.contains("13 +    format!(\"Hello, {name}!\")"),
        "{screen}"
    );
    assert!(
        screen.contains("Press enter to confirm or esc to go back"),
        "{screen}"
    );
}

#[test]
fn the_theme_picker_in_a_wide_terminal_puts_the_preview_beside_the_rows() {
    let h = open_sized("theme_picker", 100, 30);
    let screen = h.t.screen();
    // the list is 46 cells wide there: the long sentence falls back
    assert!(
        screen.contains("Move up/down to live preview themes"),
        "{screen}"
    );
    let (_, list_x) = find(&h, "› ada (current)");
    let (title_y, _) = find(&h, "Select Syntax Theme");
    let (sample_y, sample_x) = find(&h, "fn summarize(users: &[User]) -> String {");
    assert!(
        sample_x > list_x + 40,
        "the preview is on the right\n{screen}"
    );
    // centred in the sheet's height as the oracle's `centered_offset` has it:
    // the sheet is 13 rows (a blank row to the second mark), the sample 8,
    // five free: a frame row, then (5 - 2) / 2 more
    assert_eq!(sample_y, title_y + 1, "centred\n{screen}");
    // all eight lines of the wide sample, one row each (a long line is cut,
    // not wrapped), numbers 31..36
    assert!(screen.contains("  31  fn summarize"), "{screen}");
    assert!(screen.contains("35 +        .trim()"), "{screen}");
    assert!(screen.contains("36  }"), "{screen}");
    let sample_rows = screen
        .lines()
        .filter(|line| {
            let right: String = line.chars().skip(52).collect();
            right.trim_start().starts_with(|c: char| c.is_ascii_digit())
        })
        .count();
    assert_eq!(sample_rows, 8, "{screen}");
    assert!(
        !screen.contains("12  fn greet"),
        "no stacked sample\n{screen}"
    );
}

#[test]
fn the_arrows_move_the_theme_cursor_and_esc_closes() {
    let mut h = open("theme_picker");
    h.press("down");
    assert!(cursor_row(&h).contains("babbage"), "{}", h.t.screen());
    h.press("down");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Syntax Theme"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn enter_keeps_the_theme_under_the_cursor() {
    let mut h = open("theme_picker");
    h.press("down");
    h.press("down");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Syntax Theme"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn typing_filters_the_themes_and_enter_with_no_row_cancels() {
    let mut h = open("theme_picker");
    for key in "dav".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("davinci"), "{screen}");
    assert!(!screen.contains("babbage"), "{screen}");
    // the cursor went to the first row left
    assert!(cursor_row(&h).contains("davinci"), "{screen}");
    for key in "xyz".chars() {
        h.press(&key.to_string());
    }
    assert!(h.t.screen().contains("no matches"), "{}", h.t.screen());
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Syntax Theme"),
        "no row: the picker cancels\n{}",
        h.t.screen()
    );
}

#[test]
fn a_long_theme_list_scrolls_and_marks_the_custom_file() {
    let mut h = open("theme_picker_all");
    let screen = h.t.screen();
    assert!(screen.contains("› nord (current)"), "{screen}");
    // `nord` is row 25 of 33: the window scrolled to it
    assert!(!screen.contains("ansi"), "{screen}");
    h.press("end");
    let screen = h.t.screen();
    assert!(screen.contains("› my-night (custom)"), "{screen}");
    h.press("home");
    assert!(cursor_row(&h).contains("ansi"), "{}", h.t.screen());
}

/// ADR-011 (beyond the oracle): a click on a theme puts the cursor there (the
/// preview follows), a second click on it keeps it; the wheel scrolls the rows.
#[test]
fn a_click_previews_a_theme_a_second_click_keeps_it_and_the_wheel_scrolls() {
    let mut h = open("theme_picker");
    let (y, x) = find(&h, "cushman");
    h.t.click(x, y);
    h.settle();
    assert!(cursor_row(&h).contains("cushman"), "{}", h.t.screen());
    h.t.click(x, y);
    h.settle();
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Syntax Theme"),
        "{}",
        h.t.screen()
    );

    let mut h = open("theme_picker_all");
    let (y, x) = find(&h, "› nord");
    assert!(!h.t.screen().contains("ansi"), "{}", h.t.screen());
    for _ in 0..3 {
        h.t.mouse(MouseKind::WheelUp, x, y, 0, Mods::NONE);
    }
    h.t.pump_for(ms(100));
    assert!(
        h.t.screen().contains("gruvbox-dark"),
        "the wheel scrolled up\n{}",
        h.t.screen()
    );
}
