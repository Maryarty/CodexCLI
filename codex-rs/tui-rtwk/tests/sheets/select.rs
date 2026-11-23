//! Screen tests of S14 (the generic selection dialog), two parameterisations:
//! the model picker and the permissions menu. Expected text: the oracle's
//! snapshots (`dialogs::oracle`). Oracle tests: `bottom_pane/list_selection_view.rs`
//! (`snapshot_model_picker_width_80`), `chatwidget/tests/permissions.rs`
//! (`approvals_selection_popup_snapshot`).

use rtwk_core::input::{Mods, MouseKind};

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

const CHATWIDGET: &str = "chatwidget/snapshots/codex_tui__chatwidget__tests__";

fn snap(name: &str) -> String {
    format!("{CHATWIDGET}{name}.snap")
}

/// The rows of the three menus of the `/model` family the oracle snapshots.
#[test]
fn the_quick_menu_is_the_oracle_s_refreshed_auto_menu() {
    let h = open(80, "models_quick");
    assert_rows(&h.t.screen(), &snap("model_picker_refreshes_auto_models"));
}

#[test]
fn the_reasoning_menu_is_the_oracle_s() {
    let h = open(80, "model_reasoning");
    assert_rows(&h.t.screen(), &snap("model_reasoning_selection_popup"));
}

#[test]
fn the_advanced_menu_is_the_oracle_s() {
    let h = open(80, "model_advanced");
    assert_rows(
        &h.t.screen(),
        &snap("model_advanced_reasoning_selection_popup"),
    );
}

#[test]
fn the_catalog_is_loading() {
    let h = open(80, "model_loading");
    let screen = h.t.screen();
    assert!(screen.contains("Loading models…"), "{screen}");
}

#[test]
fn the_plan_scope_prompt_names_the_reasoning_and_the_default_it_replaces() {
    let h = open(100, "model_plan_scope");
    let screen = h.t.screen();
    for text in [
        "Apply reasoning change",
        "Choose where to apply high reasoning.",
        "1. Apply to Plan mode override",
        "Always use high reasoning in Plan mode",
        "2. Apply to global default and Plan mode override",
        "mode override, replacing the current built-in",
        "Plan default (medium)",
    ] {
        assert!(screen.contains(text), "{text:?} not on\n{screen}");
    }
}

#[test]
fn enter_on_a_model_with_efforts_opens_its_reasoning_menu_and_esc_comes_back() {
    let mut h = open(100, "model_picker");
    let chosen = rows(&h)
        .into_iter()
        .find(|row| row.contains('›'))
        .expect("a cursor row");
    h.press("enter");
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(
        screen.contains("Select Reasoning Level for gpt-5.1-codex"),
        "{screen}"
    );
    h.press("esc");
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(screen.contains("Select Model and Effort"), "{screen}");
    assert!(!screen.contains("Select Reasoning Level"), "{screen}");
    // the parent keeps its highlight: the row that opened the menu above it
    let back = rows(&h)
        .into_iter()
        .find(|row| row.contains('›'))
        .expect("a cursor row");
    assert_eq!(back, chosen, "{screen}");
    // The second Esc leaves the picker.
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_high_effort_of_a_codex_model_warns_while_highlighted() {
    let mut h = open(100, "model_picker");
    h.press("enter");
    h.t.pump_for(ms(300));
    assert!(
        !h.t.screen().contains("Plus plan rate limits"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.t.pump_for(ms(100));
    assert!(
        h.t.screen()
            .contains("⚠ High reasoning effort can quickly consume Plus plan rate limits"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_model_row_with_the_session_key_says_so_in_the_footer() {
    let mut h = open(100, "model_picker");
    h.press("down");
    h.t.pump_for(ms(100));
    assert!(
        h.t.screen()
            .contains("enter default · s session · esc back"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_tap_on_a_model_with_efforts_opens_its_reasoning_menu() {
    let mut h = open(100, "model_picker");
    let (x, y) = cell_of(&h, "gpt-5.1-codex (current)");
    h.t.click(x, y);
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(
        screen.contains("Select Reasoning Level for gpt-5.1-codex"),
        "{screen}"
    );
    // A tap on an effort picks it: the whole stack closes.
    let (x, y) = cell_of(&h, "Medium (default)");
    h.t.click(x, y);
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Reasoning Level"),
        "{}",
        h.t.screen()
    );
    assert!(
        !h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
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
    h.press("1");
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

/// The cell of the first screen row holding `needle`.
fn cell_of(h: &Harness, needle: &str) -> (u16, u16) {
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{screen}"));
    (line.find(needle).map_or(0, |at| at as u16), y as u16)
}

/// ADR-011: a tap on a row of a multi-select flips it and the dialog stays.
#[test]
fn a_tap_flips_a_row_of_a_multi_select_and_keeps_it_open() {
    let mut h = open(60, "multi_select");
    let (x, y) = cell_of(&h, "[ ] Beta");
    h.t.click(x, y);
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("[x] Beta") && screen.contains("Choose"),
        "{screen}"
    );
}

/// ADR-011: a tap on a row of a plain list picks it and the dialog closes.
#[test]
fn a_tap_on_a_row_picks_it_and_closes_the_dialog() {
    let mut h = open(80, "model_picker");
    assert!(h.t.screen().contains("Select Model and Effort"));
    let (x, y) = cell_of(&h, "gpt-5.1-codex-mini");
    h.t.click(x, y);
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011: the wheel scrolls the rows of a list longer than its window.
#[test]
fn the_wheel_scrolls_the_rows_of_a_long_list() {
    let mut h = open(60, "twelve_items");
    assert!(!h.t.screen().contains("Item 11"), "{}", h.t.screen());
    let (x, y) = cell_of(&h, "Item 3");
    for _ in 0..6 {
        h.t.mouse(MouseKind::WheelDown, x, y, 0, Mods::NONE);
    }
    h.t.pump_for(ms(100));
    assert!(h.t.screen().contains("Item 11"), "{}", h.t.screen());
}

/// The index of the screen row that is exactly `text` (trailing blanks cut).
fn row_of(h: &Harness, text: &str) -> usize {
    rows(h)
        .iter()
        .position(|row| row == text)
        .unwrap_or_else(|| panic!("no row `{text}` on:\n{}", h.t.screen()))
}

const RULE: &str = "  ───────────────────────";

#[test]
fn a_section_break_is_a_rule_row_of_its_own_between_two_rows() {
    let h = open(60, "section_break");
    let at = row_of(&h, "› [ ] theme-colors");
    let rows = rows(&h);
    assert_eq!(rows[at + 1], RULE);
    assert_eq!(rows[at + 2], "  [ ] model");
}

#[test]
fn the_ordering_keys_swap_orderable_rows_and_the_cursor_follows_the_moved_row() {
    let mut h = open(60, "multi_orderable_two");
    assert!(row_of(&h, "› [ ] model") < row_of(&h, "  [ ] branch"));
    h.press("ctrl+l");
    h.t.pump_for(ms(100));
    assert!(row_of(&h, "  [ ] branch") < row_of(&h, "› [ ] model"));
    h.press("right");
    h.t.pump_for(ms(100));
    assert!(
        row_of(&h, "  [ ] branch") < row_of(&h, "› [ ] model"),
        "the end stays"
    );
    h.press("ctrl+h");
    h.t.pump_for(ms(100));
    assert!(row_of(&h, "› [ ] model") < row_of(&h, "  [ ] branch"));
}

#[test]
fn a_fixed_row_neither_moves_nor_is_crossed() {
    let mut h = open(60, "multi_orderable");
    h.press("ctrl+l");
    h.t.pump_for(ms(100));
    let order = |h: &Harness| {
        ["theme-colors", "model", "branch"].map(|name| {
            rows(h)
                .iter()
                .position(|row| row.ends_with(&format!("[ ] {name}")))
                .unwrap_or(usize::MAX)
        })
    };
    assert!(
        order(&h)[0] < order(&h)[1] && order(&h)[1] < order(&h)[2],
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.press("ctrl+h");
    h.t.pump_for(ms(100));
    assert!(
        order(&h)[0] < order(&h)[1] && order(&h)[1] < order(&h)[2],
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_plain_letter_feeds_the_search_of_a_multi_select() {
    let mut h = open(60, "multi_alpha_jupiter");
    h.press("j");
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("› [ ] jupiter") && !screen.contains("[ ] alpha"),
        "{screen}"
    );
}

#[test]
fn the_preview_line_names_the_toggled_on_rows_in_order() {
    let mut h = open(80, "multi_appearance");
    assert!(
        rows(&h).iter().any(|row| row == "  Field 0"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.press("space");
    h.t.pump_for(ms(100));
    assert!(
        rows(&h).iter().any(|row| row == "  Field 0 · Field 1"),
        "{}",
        h.t.screen()
    );
    h.press("ctrl+h");
    h.t.pump_for(ms(100));
    assert!(
        rows(&h).iter().any(|row| row == "  Field 1 · Field 0"),
        "a move reorders the preview too: {}",
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

/// The app link dialog (oracle `bottom_pane/app_link_view.rs`): the rows of an installed app.
#[test]
fn the_link_dialog_of_an_installed_app_is_the_oracle_s() {
    let h = open(80, "app_link_installed");
    let rows = rows(&h);
    let at = rows
        .iter()
        .position(|row| row == "  Notion")
        .expect("title");
    assert_eq!(rows[at + 1], "  Workspace docs");
    assert_eq!(rows[at + 2], "");
    assert_eq!(rows[at + 3], "  Use $ to insert this app into the prompt.");
    assert_eq!(rows[at + 4], "");
    assert_eq!(rows[at + 5], "  Manage this app in your browser.");
    assert_eq!(
        rows[at + 6],
        "  Newly installed apps can take a few minutes to appear in /apps."
    );
    assert_eq!(rows[at + 7], "");
    assert_eq!(rows[at + 8], "");
    assert_eq!(rows[at + 9], "› 1. Manage on ChatGPT");
    assert_eq!(rows[at + 10], "  2. Disable app");
    assert_eq!(rows[at + 11], "  3. Back");
    assert_eq!(rows[at + 12], "  tab / ↑ ↓ move · enter select · esc close");
}

/// An app that is not installed: Install on ChatGPT and Back, and the install text.
#[test]
fn the_link_dialog_of_an_app_to_install_offers_install_and_back() {
    let h = open(80, "app_link_install");
    let screen = h.t.screen();
    assert!(screen.contains("› 1. Install on ChatGPT"), "{screen}");
    assert!(screen.contains("  2. Back"), "{screen}");
    assert!(
        screen.contains("Install this app in your browser, then reload Codex."),
        "{screen}"
    );
    assert!(
        screen.contains("After installed, use $ to insert this app into the prompt."),
        "{screen}"
    );
    assert!(!screen.contains("Use $ to insert this app"), "{screen}");
    assert!(!screen.contains("Disable app"), "{screen}");
}

/// Opening the page of an app that is not installed goes on to the confirmation: the setup URL
/// (a link), `I already Installed it` and Back, which returns to the first screen.
#[test]
fn the_install_page_goes_on_to_the_confirmation_and_back_returns() {
    let mut h = open(80, "app_link_install");
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("Finish App Setup"), "{screen}");
    assert!(
        screen.contains("Complete app setup on ChatGPT in the browser window that just opened."),
        "{screen}"
    );
    assert!(screen.contains("Setup URL:"), "{screen}");
    assert!(screen.contains("https://example.test/linear"), "{screen}");
    assert!(screen.contains("› 1. I already Installed it"), "{screen}");
    assert!(screen.contains("  2. Back"), "{screen}");
    h.press("2");
    let screen = h.t.screen();
    assert!(screen.contains("› 1. Install on ChatGPT"), "{screen}");
    assert!(!screen.contains("Finish App Setup"), "{screen}");
}

/// The digit of an action picks it (`2` is Disable app); Esc closes the dialog.
#[test]
fn a_digit_switches_the_app_and_esc_closes_the_dialog() {
    let mut h = open(80, "app_link_installed");
    h.press("2");
    let screen = h.t.screen();
    assert!(screen.contains("2. Enable app"), "{screen}");
    assert!(!screen.contains("Disable app"), "{screen}");
    h.press("esc");
    assert!(!h.t.screen().contains("Notion"), "{}", h.t.screen());
}

/// The cursor stops at the ends (the oracle does not wrap), Tab and `j` go on.
#[test]
fn the_cursor_stops_at_the_ends_of_the_actions() {
    let mut h = open(80, "app_link_installed");
    h.press("up");
    assert!(h.t.screen().contains("› 1. Manage on ChatGPT"));
    h.press("tab");
    h.press("j");
    assert!(h.t.screen().contains("› 3. Back"), "{}", h.t.screen());
    h.press("down");
    assert!(h.t.screen().contains("› 3. Back"), "{}", h.t.screen());
    h.press("k");
    assert!(
        h.t.screen().contains("› 2. Disable app"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011: a tap on a row moves the cursor to it, a tap on the cursor's row does it.
#[test]
fn a_tap_on_an_action_moves_the_cursor_and_a_second_tap_does_it() {
    let mut h = open(80, "app_link_installed");
    let (x, y) = cell_of(&h, "Disable app");
    h.t.click(x, y);
    h.t.pump_for(ms(200));
    assert!(
        h.t.screen().contains("› 2. Disable app"),
        "{}",
        h.t.screen()
    );
    h.t.click(x, y);
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("2. Enable app"), "{}", h.t.screen());
}

/// Back closes the dialog.
#[test]
fn back_closes_the_link_dialog() {
    let mut h = open(80, "app_link_installed");
    h.press("3");
    assert!(!h.t.screen().contains("Notion"), "{}", h.t.screen());
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
            "src/widgets/picker/list.rs",
            include_str!("../../src/widgets/picker/list.rs"),
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

// ---- the pane at the heights of the oracle's tests (Picker `Pane`, core
// `select::layout`): the whole screen equals one frame of the oracle's
// snapshot, which the oracle joined of several.

/// The rows of the frame whose first line is `header` in the oracle snapshot
/// `path` (relative to `codex-rs/tui/src`): up to the next frame, the
/// separator row between frames dropped.
#[allow(clippy::disallowed_methods)]
fn frame(path: &str, header: &str) -> Vec<String> {
    let full = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tui/src")
        .join(path);
    let text = std::fs::read_to_string(&full)
        .unwrap_or_else(|err| panic!("oracle snapshot {}: {err}", full.display()));
    let body = text.splitn(3, "---\n").nth(2).unwrap_or_default();
    let starts =
        |line: &str| line.starts_with("height=") || line.ends_with(':') && line.contains('×');
    if header.is_empty() {
        // A snapshot of one screen: every row.
        let mut rows: Vec<String> = body
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        return rows;
    }
    let mut lines = body.lines().skip_while(|line| *line != header).skip(1);
    let mut rows: Vec<String> = Vec::new();
    for line in lines.by_ref() {
        if line == "---" || starts(line) {
            break;
        }
        rows.push(line.trim_end().to_string());
    }
    // `\n\n` between two frames leaves one blank row after the last.
    if rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

fn sized(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("select", Some(entry));
    h.t.pump_for(ms(800));
    h
}

fn rows_of(h: &Harness) -> Vec<String> {
    h.t.screen()
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

const SHORT_BROWSER: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__list_selection_view__picker_tests__short_browser_keeps_header_tabs_search_and_footer.snap";

#[test]
fn the_browser_keeps_header_tabs_search_and_footer_at_short_heights() {
    for (width, height) in [(48, 6), (48, 7), (48, 13), (24, 6), (24, 7)] {
        let h = sized(width, height, "browser_keymap");
        assert_eq!(
            rows_of(&h),
            frame(SHORT_BROWSER, &format!("{width}×{height}:")),
            "{width}x{height}"
        );
    }
}

const MULTI_SHORT: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__multi_select_picker__tests__short_picker_preserves_selected_row_when_overflow_spacers_yield.snap";

#[test]
fn the_multi_select_yields_its_spacers_before_the_selected_row() {
    for height in 1..=7 {
        let mut h = sized(80, height, "multi_fields_twelve");
        h.press("end");
        h.t.pump_for(ms(300));
        assert_eq!(
            rows_of(&h),
            frame(MULTI_SHORT, &format!("height={height}")),
            "height {height}"
        );
    }
}

/// `multi_select_picker.rs` `picker_appearance_renders_checkboxes_preview_and_overflow`: the three
/// frames of the oracle's snapshot (rows only: the dump's cell styles say `bg: Reset`, the debug host
/// under the sheet paints its own background), each at the height the oracle drew it in. The rule row
/// is a disabled row of its own that sizes the label column (`SECTION_BREAK_ROW`), so the details
/// start at column 27. The oracle measures that column over the rows in the window (AutoVisible):
/// once the rule scrolls out (frame 2) its details sit at column 17 there; ours keeps column 27, the
/// open G39b (the rows below are ours, equal to the oracle's but for those spaces).
#[test]
fn the_multi_select_appearance_matches_the_oracle_in_three_frames() {
    let mut h = sized(48, 17, "multi_appearance");
    assert_eq!(
        rows_of(&h),
        [
            "",
            "  Status line",
            "",
            "  Type to search",
            "",
            "› [x] Field 0              Status detail",
            "  ───────────────────────",
            "  [ ] Field 1              Status detail",
            "  [ ] Field 2              Status detail",
            "  [ ] Field 3              Status detail",
            "  [ ] Field 4              Status detail",
            "  [ ] Field 5              Status detail",
            "  [ ] Field 6              Status detail",
            "↓",
            "  Field 0",
            "  space toggle · ←/→ reorder · enter save · esc",
            "  cancel",
        ],
        "First page"
    );
    for key in ["end", "space", "left"] {
        h.press(key);
        h.t.pump_for(ms(100));
    }
    assert_eq!(
        rows_of(&h),
        [
            "",
            "  Status line",
            "",
            "  Type to search",
            "↑",
            "  [ ] Field 4              Status detail",
            "  [ ] Field 5              Status detail",
            "  [ ] Field 6              Status detail",
            "  [ ] Field 7              Status detail",
            "  [ ] Field 8              Status detail",
            "  [ ] Field 9              Status detail",
            "› [x] Field 11             Status detail",
            "  [ ] Field 10             Status detail",
            "",
            "  Field 0 · Field 11",
            "  space toggle · ←/→ reorder · enter save · esc",
            "  cancel",
        ],
        "Last page"
    );
    h.press("z");
    h.t.pump_for(ms(100));
    // the sheet is as tall as its rows (10): the debug page shows around it
    let shown = rows_of(&h);
    let top = shown
        .iter()
        .position(|row| row == "  Status line")
        .unwrap_or(1)
        - 1;
    assert_eq!(
        shown[top..top + 10],
        [
            "",
            "  Status line",
            "",
            "  z",
            "",
            "no matches",
            "",
            "  Field 0 · Field 11",
            "  space toggle · ←/→ reorder · enter save · esc",
            "  cancel",
        ],
        "No matches"
    );
}

const BROWSER_PAGE: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__list_selection_view__picker_tests__browser_page_navigation_uses_rows_that_fit.snap";

#[test]
fn page_down_moves_by_the_rows_that_fit() {
    let mut h = sized(60, 18, "browser_keymap");
    h.press("pagedown");
    h.t.pump_for(ms(300));
    assert_eq!(rows_of(&h), frame(BROWSER_PAGE, ""));
}

#[test]
fn shrinking_the_browser_keeps_the_selected_row_visible() {
    for entry in ["browser_single_line", "browser_wrapped"] {
        let mut h = sized(60, 24, entry);
        for _ in 0..7 {
            h.press("down");
        }
        h.t.resize(60, 13);
        h.t.pump_for(ms(300));
        let screen = h.t.screen();
        let focused = screen.lines().find(|line| line.starts_with('›'));
        assert!(
            focused.is_some_and(|line| line.contains("Action 07")),
            "{entry}:\n{screen}"
        );
    }
}

/// The description column hides below 24 cells and the rows keep their whole
/// width: at 48 cells the names stand alone, at 49 the details come back
/// (`description_column_visibility`; the oracle's test draws raw rows: the
/// second one carries its number `2.` by hand, while a disabled row of the
/// list has none; the reason is bare under this layout, as
/// `selection_row_layout.rs` `combined_description` says).
#[test]
fn the_description_column_hides_when_it_would_be_narrow() {
    for width in [48u16, 49, 80] {
        let h = sized(width, 2, "description_hide_when_narrow");
        let rows = rows_of(&h);
        let (first, second) = if width >= 49 {
            (
                format!("› 1. 日本語{}Details", " ".repeat(14)),
                "     Blocked (disabled)  Unavailable".to_string(),
            )
        } else {
            (
                "› 1. 日本語".to_string(),
                "     Blocked (disabled)".to_string(),
            )
        };
        assert_eq!(rows, [first, second], "width {width}");
    }
}

/// An entry opened under the user's keymap, as the kit's `.keymap(..)` does.
fn keyed(width: u16, height: u16, entry: &str, keys: &[(&str, &str)]) -> Harness {
    let mut h = crate::common::start_driven(true, width, height);
    h.connect();
    h.set_keymap(keys);
    h.open_debug_row("select", Some(entry));
    h.advance(800);
    h
}

fn screen_of(h: &Harness) -> String {
    h.t.screen()
}

/// `list_selection_view.rs:2025`: a printable key bound to accept accepts,
/// whether or not a search line is there to take it as a letter.
#[test]
fn a_printable_accept_key_wins_over_the_search_line() {
    let mut h = keyed(60, 6, "accept_alternative", &[("list.accept", "z")]);
    assert!(screen_of(&h).contains("Selection"));
    h.press("z");
    assert!(!screen_of(&h).contains('z'), "{}", screen_of(&h));
}

/// `list_selection_view.rs:2550`: a key the keymap binds to a list action is
/// that action's, not a row's toggle. (A searchable list draws no numbers.)
#[test]
fn a_configured_list_binding_moves_before_it_toggles() {
    let mut h = keyed(60, 6, "toggle_then_next", &[("list.move_down", "tab")]);
    h.press("tab");
    assert!(screen_of(&h).contains("› Next"), "{}", screen_of(&h));
}

/// `list_selection_view.rs:2598`: space bound to cancel cancels, it does not
/// flip the checkbox.
#[test]
fn space_bound_to_cancel_closes_instead_of_flipping_the_checkbox() {
    let mut h = keyed(60, 6, "animations_checkbox", &[("list.cancel", "space")]);
    assert!(screen_of(&h).contains("Animations"));
    h.press("space");
    assert!(!screen_of(&h).contains("Animations"), "{}", screen_of(&h));
}

/// Without a binding for it, space still flips the checkbox.
#[test]
fn space_flips_the_checkbox_when_the_keymap_leaves_it_free() {
    let mut h = keyed(60, 6, "animations_checkbox", &[]);
    assert!(
        screen_of(&h).contains("[x] Animations"),
        "{}",
        screen_of(&h)
    );
    h.press("space");
    assert!(
        screen_of(&h).contains("[ ] Animations"),
        "{}",
        screen_of(&h)
    );
}

/// `list_selection_view.rs:2875, :2907, :2938`: ctrl+p and ctrl+n are the
/// keymap's, not the search line's: unbound they do nothing, rebound they
/// move the other way.
#[test]
fn ctrl_p_and_ctrl_n_follow_the_keymap_under_a_search_line() {
    let mut h = keyed(
        60,
        6,
        "first_second_searchable_second",
        &[("list.move_up", "")],
    );
    h.press("ctrl+p");
    assert!(screen_of(&h).contains("› Second"), "{}", screen_of(&h));
    let mut h = keyed(60, 6, "first_second_searchable", &[("list.move_down", "")]);
    h.press("ctrl+n");
    assert!(screen_of(&h).contains("› First"), "{}", screen_of(&h));
    let mut h = keyed(
        60,
        6,
        "first_second_searchable",
        &[("list.move_up", ""), ("list.move_down", "ctrl+p")],
    );
    h.press("ctrl+p");
    assert!(screen_of(&h).contains("› Second"), "{}", screen_of(&h));
}

/// The oracle's search query takes plain characters and Backspace only: the
/// text field's own chords (Ctrl+U to the start, Ctrl+K, Ctrl+W, Alt+B ...) leave
/// it as it is, unless the keymap binds them.
#[test]
fn a_search_line_ignores_the_text_fields_edit_chords() {
    let mut h = keyed(60, 6, "first_second_searchable", &[]);
    h.t.type_text("fir");
    for chord in ["ctrl+u", "ctrl+k", "ctrl+w", "ctrl+a", "alt+b", "alt+d"] {
        h.press(chord);
        assert!(
            rows_of(&h).contains(&"  fir".to_string()),
            "{chord} edited the query:\n{}",
            screen_of(&h)
        );
    }
    h.press("backspace");
    assert!(
        rows_of(&h).contains(&"  fi".to_string()),
        "{}",
        screen_of(&h)
    );
}

/// `list_selection_view.rs:2969`: page and jump keys are the keymap's.
#[test]
fn page_and_jump_keys_follow_the_keymap() {
    // 60x12 shows eight rows between the arrows: a page is eight rows, as
    // the oracle's `MAX_POPUP_ROWS` when it is not drawn smaller.
    let mut h = keyed(
        60,
        12,
        "twelve_items",
        &[
            ("list.page_down", "ctrl+d"),
            ("list.page_up", "ctrl+u"),
            ("list.jump_bottom", "ctrl+e"),
            ("list.jump_top", "ctrl+a"),
        ],
    );
    for (key, row) in [
        ("pagedown", "› 1. Item 0"),
        ("ctrl+d", "› 9. Item 8"),
        ("ctrl+u", "› 1. Item 0"),
        ("ctrl+e", "› 12. Item 11"),
        ("ctrl+a", "› 1. Item 0"),
    ] {
        h.press(key);
        assert!(screen_of(&h).contains(row), "{key}:\n{}", screen_of(&h));
    }
}

/// `picker_hint_tests.rs`: the standard footer names the keys the keymap
/// gives accept and cancel, and the cancel key closes the list.
#[test]
fn the_standard_footer_follows_the_configured_bindings() {
    for entry in ["hint_default", "hint_picker"] {
        let mut h = keyed(
            60,
            7,
            entry,
            &[("list.accept", "f3"), ("list.cancel", "f2")],
        );
        assert!(
            screen_of(&h).contains("f3 select · f2 back"),
            "{entry}:\n{}",
            screen_of(&h)
        );
        h.press("f2");
        assert!(!screen_of(&h).contains("Continue"), "{entry}");
    }
    let h = keyed(60, 7, "hint_default", &[]);
    assert!(
        screen_of(&h).contains("enter select · esc back"),
        "{}",
        screen_of(&h)
    );
}

/// The cell of the first `text` on the screen, as (column, row).
fn find(h: &Harness, text: &str) -> (usize, usize) {
    h.t.screen()
        .lines()
        .enumerate()
        .find_map(|(y, line)| line.find(text).map(|at| (line[..at].chars().count(), y)))
        .unwrap_or_else(|| panic!("`{text}` is not on the screen:\n{}", h.t.screen()))
}

/// `selection_popup_common.rs:986`: the runs of a highlighted row keep their
/// roles: the mnemonic underlined and bold with the accent, the status in the
/// success colour, italic and bold, the description in the accent but not bold.
#[test]
fn the_runs_of_a_highlighted_row_keep_their_roles() {
    use rtwk_core::style::{Attrs, Color};
    let h = keyed(48, 1, "semantic_spans", &[]);
    let (l, y) = find(&h, "L");
    let (ready, _) = find(&h, "Ready");
    let (details, _) = find(&h, "secondary");
    let (name, _) = find(&h, "provider");
    let key = h.t.cell(l, y);
    assert!(
        key.attrs.intersects(Attrs::BOLD | Attrs::UNDERLINE),
        "{key:?}"
    );
    assert!(key.attrs.intersects(Attrs::UNDERLINE), "{key:?}");
    let status = h.t.cell(ready, y);
    assert_eq!(status.fg, Color::GREEN, "{status:?}");
    assert!(status.attrs.intersects(Attrs::BOLD), "{status:?}");
    assert!(status.attrs.intersects(Attrs::ITALIC), "{status:?}");
    assert!(!h.t.cell(name, y).attrs.intersects(Attrs::UNDERLINE));
    assert!(!h.t.cell(details, y).attrs.intersects(Attrs::BOLD));
    assert_eq!(h.t.cell(details, y).fg, h.t.cell(l, y).fg);
}

/// `selection_popup_common.rs:1089`: the highlighted row is bold in the
/// accent; a row beside it is plain.
#[test]
fn the_highlighted_row_is_bold_in_the_accent() {
    use rtwk_core::style::Attrs;
    let h = keyed(16, 1, "selected_row", &[]);
    let mark = h.t.cell(0, 0);
    assert!(mark.attrs.intersects(Attrs::BOLD), "{mark:?}");
    assert_ne!(mark.fg, rtwk_core::style::Color::DEFAULT, "{mark:?}");
}

/// `selection_popup_common.rs:965`: a line break in a name keeps the second
/// line under the first, one in a description stays a break in its column.
#[test]
fn line_breaks_in_a_name_and_a_description_stay_breaks() {
    let h = keyed(24, 4, "hard_breaks", &[]);
    let screen = screen_of(&h);
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[0].ends_with("first"), "{screen}");
    assert_eq!(rows[1].trim_end(), "  second", "{screen}");
    let column = |row: &str, text: &str| row.find(text).map(|at| row[..at].chars().count());
    assert!(
        rows[2].contains("x") && rows[2].contains("alpha"),
        "{screen}"
    );
    assert_eq!(
        column(rows[3], "beta"),
        column(rows[2], "alpha"),
        "{screen}"
    );
}

/// `selection_picker_tests.rs` `shared_menu_presentation_at_wide_and_narrow_sizes`:
/// the two frames of the oracle's snapshot, a described row's long label cut
/// with an ellipsis beside its description, a disabled row's reason bare, the
/// label wrapped when the description column hides. The oracle draws the view
/// into the whole area (footer on the last row); the dialog is as tall as its
/// content, so the rows are compared from the blank row above the title on,
/// and the footer row last.
#[test]
fn the_shared_menu_matches_the_oracle_at_both_sizes() {
    let frames: [(u16, u16, &[&str]); 2] = [
        (
            80,
            24,
            &[
                "",
                "  Choose an option",
                "  A menu with descriptions and disabled choices.",
                "",
                "",
                "› 1. Recommended option                                 The description column",
                "                                                        is visible when it fits.",
                "  2. A long option label that still wraps at narrow wi… Additional details",
                "     Unavailable (disabled)                             This option needs a",
                "                                                        connected server.",
            ],
        ),
        (
            40,
            16,
            &[
                "",
                "  Choose an option",
                "  A menu with descriptions and",
                "  disabled choices.",
                "",
                "",
                "› 1. Recommended option",
                "  2. A long option label that still",
                "     wraps at narrow widths",
                "     Unavailable (disabled)",
            ],
        ),
    ];
    for (width, height, expected) in frames {
        let h = keyed(width, height, "shared_menu", &[]);
        let screen = screen_of(&h);
        let rows: Vec<&str> = screen.lines().map(str::trim_end).collect();
        let title = rows
            .iter()
            .position(|r| *r == "  Choose an option")
            .unwrap_or(1);
        let from = title - 1;
        assert_eq!(
            &rows[from..from + expected.len()],
            expected,
            "{width}x{height}:\n{screen}"
        );
        assert!(
            rows.contains(&"  enter select · esc back"),
            "{width}x{height}:\n{screen}"
        );
    }
}

// ---- the /permissions family (chatwidget/permission_popups.rs, permissions_menu.rs,
// permission_discovery.rs): the rows of the oracle's snapshots, the keys and the taps.

#[test]
fn the_profile_menus_are_the_oracle_s_at_80_columns() {
    for (entry, name) in [
        ("profile_permissions", "profile_permissions_selection_popup"),
        (
            "profile_permissions_disallowed_full_access",
            "profile_permissions_selection_popup_with_disallowed_full_access",
        ),
        (
            "profile_permissions_custom",
            "profile_permissions_selection_popup_with_custom_profiles",
        ),
        (
            "permission_discovery_loading",
            "permission_discovery_loading",
        ),
        (
            "permission_discovery_failure",
            "permission_discovery_failure",
        ),
    ] {
        let h = open(80, entry);
        assert_rows(&h.t.screen(), &snap(name));
    }
}

#[test]
fn the_remote_catalog_disables_what_the_requirements_leave_out_at_110_columns() {
    let h = open(110, "permission_discovery_remote");
    assert_rows(&h.t.screen(), &snap("permission_discovery_remote"));
}

#[test]
fn an_empty_catalog_keeps_only_the_active_profile_at_100_columns() {
    let h = open(100, "permission_discovery_empty");
    assert_rows(&h.t.screen(), &snap("permission_discovery_empty"));
}

#[test]
fn approve_for_me_in_use_is_marked_at_120_columns() {
    let h = open(120, "permissions_unnamed_server_profile");
    assert_rows(&h.t.screen(), &snap("permissions_unnamed_server_profile"));
}

#[test]
fn a_preset_the_config_forbids_shows_the_configs_reason_and_is_skipped() {
    let mut h = open(80, "permissions_constrained");
    let screen = h.t.screen();
    assert!(
        screen.contains("Full Access (disabled)")
            && screen.contains("(disabled: this message should be printed in the"),
        "{screen}"
    );
    // Ten Downs: the cursor never lands on the disabled row, and Enter
    // still picks the enabled one: the menu closes.
    for _ in 0..10 {
        h.press("down");
    }
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
}

/// Full access asks first: the second row opens the confirmation, `Cancel`
/// opens the menu again and Esc closes.
#[test]
fn full_access_asks_before_it_applies() {
    let mut h = open(100, "permissions_menu");
    h.press("2");
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    for text in [
        "Enable full access?",
        "When Codex runs with full access, it can edit any file on your computer",
        "1. Yes, continue anyway",
        "Apply full access for this session",
        "2. Cancel",
    ] {
        assert!(screen.contains(text), "{text}\n{screen}");
    }
    h.press("2");
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(
        screen.contains("Update Model Permissions") && !screen.contains("Enable full access?"),
        "{screen}"
    );
    h.press("2");
    h.t.pump_for(ms(300));
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Enable full access?")
            && !h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn continuing_the_confirmation_closes_the_dialog() {
    let mut h = open(100, "permissions_menu");
    h.press("2");
    h.t.pump_for(ms(300));
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Enable full access?"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_cyber_model_s_confirmation_recommends_what_is_available() {
    let h = open(120, "permissions_cyber_full_access");
    let screen = h.t.screen();
    assert!(
        screen.contains("Cyber models carry a higher risk of dangerous actions.")
            && screen.contains("We strongly recommend selecting \"Approve for me\" instead"),
        "{screen}"
    );
}

/// ADR-011: a tap on Retry asks for the catalog again (the menu is the
/// loading row until the answer), a tap on a disabled row does nothing.
#[test]
fn a_tap_on_retry_asks_again_and_a_tap_on_a_disabled_row_does_nothing() {
    let mut h = open(80, "permission_discovery_failure");
    let (x, y) = cell_of(&h, "Retry");
    h.t.click(x, y);
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(
        screen.contains("Update Model Permissions") && !screen.contains("Failed to load"),
        "{screen}"
    );
    let mut h = open(110, "permission_discovery_remote");
    let (x, y) = cell_of(&h, "Approve for me");
    h.t.click(x, y);
    h.t.pump_for(ms(300));
    assert!(
        h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011: a tap on a profile of the server picks it and closes the dialog;
/// a tap on Full Access opens the confirmation.
#[test]
fn a_tap_on_a_profile_picks_it_and_a_tap_on_full_access_asks() {
    let mut h = open(80, "profile_permissions_custom");
    let (x, y) = cell_of(&h, "locked-down");
    h.t.click(x, y);
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
    let mut h = open(80, "profile_permissions_custom");
    let (x, y) = cell_of(&h, "Full Access");
    h.t.click(x, y);
    h.t.pump_for(ms(300));
    assert!(
        h.t.screen().contains("Enable full access?"),
        "{}",
        h.t.screen()
    );
}

/// `open_full_access_confirmation`: the header is a paragraph, the title bold,
/// the sentence about the risk plain and the caution in red, not the dim
/// subtitle of the other menus; a cyber model's warning is red on its first
/// sentence.
#[test]
fn the_confirmation_header_is_bold_then_plain_then_red() {
    use rtwk_core::style::Attrs;
    let mut h = open(100, "permissions_menu");
    h.press("2");
    h.t.pump_for(ms(300));
    let (x, y) = find(&h, "Enable full access?");
    assert!(h.t.cell(x, y).attrs.intersects(Attrs::BOLD));
    let (body, by) = find(&h, "When Codex runs");
    assert!(!h.t.cell(body, by).attrs.intersects(Attrs::DIM));
    let (caution, cy) = find(&h, "Exercise caution");
    let red = h.t.cell(caution, cy).fg;
    assert_ne!(red, rtwk_core::style::Color::DEFAULT);
    assert_ne!(red, h.t.cell(body, by).fg);

    let h = open(120, "permissions_cyber_full_access");
    let (risk, ry) = find(&h, "Cyber models carry");
    let (plain, py) = find(&h, "Ensure proper safeguards");
    assert_ne!(h.t.cell(risk, ry).fg, h.t.cell(plain, py).fg);
    assert_eq!(h.t.cell(risk, ry).fg, red);
}
