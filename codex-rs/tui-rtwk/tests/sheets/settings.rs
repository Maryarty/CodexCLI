//! Screen tests of the `settings` slot (S15): the status-line and terminal
//! title setup sheets (`SetupSheet`) and the syntax theme picker (`ThemeSheet`). Expected text: the oracle's snapshots
//! are in `tests/oracle/small_dialogs_2.rs`; here are the behaviours the
//! snapshots do not show (keys, the filter, the preview, the mouse).
//! Oracle: `bottom_pane/{multi_select_picker,status_line_setup,title_setup}.rs`.
//! The domain tests of the catalogues are in `core/tests/settings.rs`, those of the drafts
//! (the checks, the order, the flips) in `src/sheets/settings/draft.rs`.

use rtwk_core::input::{Mods, MouseKind};

use codex_tui_core::app::AppEv;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::settings::{Edit, Ev as SettingsEv, Facts, MemoryFacts, Offer, Which};
use codex_tui_rtwk::app::UiApp;

use crate::common::{HEIGHT, Harness, WIDTH, ms, start_driven, start_sized};

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
// What the model keeps (the theme previewed on every move, kept by Enter,
// restored by Esc) is read below through `/theme`: a debug entry's model lives
// in its seeded scope, so these tests see its intents only as the sheet
// closing.

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
    assert!(cursor_row(&h).contains("1337"), "{}", h.t.screen());
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

// ---- the skills toggle (`skills_toggle_view.rs`): a checklist, each flip is the model's

#[test]
fn space_and_enter_flip_the_skill_under_the_cursor_and_arrows_stay_inert() {
    let mut h = open("skills_basic");
    assert!(
        h.t.screen().contains("› [x] Repo Scout"),
        "{}",
        h.t.screen()
    );
    h.press("space");
    assert!(
        h.t.screen().contains("› [ ] Repo Scout"),
        "{}",
        h.t.screen()
    );
    // Enter does what Space does on the skills (it saves nothing else)
    h.press("enter");
    assert!(
        h.t.screen().contains("› [x] Repo Scout"),
        "{}",
        h.t.screen()
    );
    // no order to change: Right/Left leave the rows where they are
    h.press("right");
    h.press("left");
    let screen = h.t.screen();
    let (scout, writer) = (find(&h, "Repo Scout").0, find(&h, "Changelog Writer").0);
    assert!(scout < writer, "{screen}");
    h.press("down");
    h.press("space");
    assert!(
        h.t.screen().contains("› [x] Changelog Writer"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn typing_filters_the_skills_by_display_name_then_by_skill_name_and_esc_closes() {
    let mut h = open("skills_basic");
    for key in "scout".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("› [x] Repo Scout"), "{screen}");
    assert!(!screen.contains("Changelog Writer"), "{screen}");
    // a query only the skill's own name matches (`changelog_writer`)
    for _ in 0..5 {
        h.press("backspace");
    }
    for key in "log_w".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("Changelog Writer"), "{screen}");
    assert!(!screen.contains("Repo Scout"), "{screen}");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(
        !h.t.screen().contains("Enable/Disable Skills"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011 (beyond the oracle): a tap puts the cursor on a skill, a second
/// tap flips it.
#[test]
fn a_tap_moves_the_cursor_to_a_skill_and_a_second_tap_flips_it() {
    let mut h = open("skills_basic");
    let (y, x) = find(&h, "Changelog Writer");
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [ ] Changelog Writer"),
        "{}",
        h.t.screen()
    );
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [x] Changelog Writer"),
        "{}",
        h.t.screen()
    );
}

// ---- the experimental features (`experimental_features_view.rs`): a checklist
// that writes what changed

#[test]
fn space_flips_the_feature_under_the_cursor_the_cursor_wraps_and_enter_sends_the_write() {
    let mut h = open("experimental_features_two");
    assert!(
        h.t.screen().contains("› [x] First feature"),
        "{}",
        h.t.screen()
    );
    h.press("space");
    assert!(
        h.t.screen().contains("› [ ] First feature"),
        "{}",
        h.t.screen()
    );
    h.press("up");
    assert!(
        h.t.screen().contains("› [ ] Last feature"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    assert!(
        h.t.screen().contains("› [ ] First feature"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.press("space");
    assert!(
        h.t.screen().contains("› [x] Last feature"),
        "{}",
        h.t.screen()
    );
    // Enter sends the write and holds the sheet until it is answered; the
    // rows are inert meanwhile
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("Saving experimental features"), "{screen}");
    assert!(screen.contains("Saving… Closing this popup will not cancel the write."));
    h.press("space");
    assert!(
        h.t.screen().contains("› [x] Last feature"),
        "{}",
        h.t.screen()
    );
    // Esc closes at any time
    h.press("esc");
    assert!(
        !h.t.screen().contains("Experimental features"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_empty_catalogue_leaves_the_keys_inert_and_enter_closes_without_a_write() {
    let mut h = open("experimental_features_empty");
    h.press("down");
    h.press("space");
    let screen = h.t.screen();
    assert!(
        screen.contains("No experimental features available for now"),
        "{screen}"
    );
    assert!(!screen.contains('›'), "{screen}");
    h.press("enter");
    assert!(
        !h.t.screen().contains("Experimental features"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011 (beyond the oracle): a tap puts the cursor on a feature, a second
/// tap flips it.
#[test]
fn a_tap_moves_the_cursor_to_a_feature_and_a_second_tap_flips_it() {
    let mut h = open("experimental_features_two");
    let (y, x) = find(&h, "Last feature");
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [ ] Last feature"),
        "{}",
        h.t.screen()
    );
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [x] Last feature"),
        "{}",
        h.t.screen()
    );
    // the first row is untouched
    assert!(
        h.t.screen().contains("  [x] First feature"),
        "{}",
        h.t.screen()
    );
}

// ---- the memories dialog (`memories_settings_view.rs`): two switches, the
// reset row and its confirmation

/// The edits the model recorded (what the dialog asked the app to do): the
/// real model, so the dialogs under test are opened by `/memories`.
fn edits(h: &Harness) -> Vec<Edit> {
    h.t.read_app(|app: &UiApp| app.core.screens().settings.edits.clone())
}

/// `/memories` with the memory tool on, use on and generate off
/// (`popups_and_settings.rs:3289`).
/// The `/memories` command on a driven harness: a key's intent travels to the
/// shell and back, and on the shared runtime that hop takes real time (a
/// press followed by another before the model answered read a stale
/// `confirming`); here the runtime runs only when the test lets it, and
/// `press` runs it until the app and the shell are quiet.
fn memories_command() -> Harness {
    let mut h = start_driven(false, WIDTH, HEIGHT);
    h.wait_idle();
    h.inject([AppEv::Screen(ScreenEv::Settings(SettingsEv::Facts(
        Facts {
            memories: Some(MemoryFacts {
                use_memories: true,
                generate_memories: false,
            }),
            ..Facts::default()
        },
    )))]);
    h.t.type_text("/memories");
    h.press("enter");
    h.wait_for("the dialog", |screen| {
        screen.contains("Choose how Codex uses")
    });
    h.advance(600);
    h
}

#[test]
fn space_flips_a_switch_enter_saves_both_and_closes() {
    let mut h = memories_command();
    assert!(
        h.t.screen().contains("› [x] Use memories"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.press("space");
    assert!(
        h.t.screen().contains("› [x] Generate memories"),
        "{}",
        h.t.screen()
    );
    h.press("enter");
    assert!(!h.t.screen().contains("Memories"), "{}", h.t.screen());
    assert_eq!(
        edits(&h),
        vec![Edit::MemoriesSaved {
            use_memories: true,
            generate_memories: true
        }]
    );
}

#[test]
fn space_on_the_reset_row_flips_nothing_and_the_cursor_goes_round_the_ends() {
    let mut h = open("memories_settings");
    h.press("up");
    assert!(
        h.t.screen().contains("› Reset all memories"),
        "{}",
        h.t.screen()
    );
    h.press("space");
    let screen = h.t.screen();
    assert!(screen.contains("[x] Use memories"), "{screen}");
    assert!(screen.contains("[ ] Generate memories"), "{screen}");
    h.press("down");
    assert!(
        h.t.screen().contains("› [x] Use memories"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn enter_on_the_reset_row_asks_and_go_back_returns_with_the_cursor_on_the_reset_row() {
    let mut h = memories_command();
    h.press("up");
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("Reset all memories?"), "{screen}");
    assert!(
        screen.contains("› Reset all memories  Delete local"),
        "{screen}"
    );
    // Space is no key of the confirmation
    h.press("space");
    assert!(
        h.t.screen().contains("Reset all memories?"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("Choose how Codex uses"), "{screen}");
    assert!(screen.contains("› Reset all memories"), "{screen}");
    assert_eq!(edits(&h), vec![]);
}

#[test]
fn esc_goes_back_from_the_confirmation_then_closes_and_the_next_confirmation_starts_on_its_first_choice()
 {
    let mut h = memories_command();
    h.press("up");
    h.press("enter");
    h.press("down");
    h.press("esc");
    assert!(
        h.t.screen().contains("Choose how Codex uses"),
        "{}",
        h.t.screen()
    );
    h.press("enter");
    assert!(
        h.t.screen().contains("› Reset all memories  Delete local"),
        "{}",
        h.t.screen()
    );
    h.press("esc");
    h.press("esc");
    assert!(!h.t.screen().contains("Memories"), "{}", h.t.screen());
    assert_eq!(edits(&h), vec![]);
}

#[test]
fn enter_on_reset_all_memories_sends_the_reset_and_closes() {
    let mut h = memories_command();
    h.press("up");
    h.press("enter");
    h.press("enter");
    assert!(
        !h.t.screen().contains("Reset all memories?"),
        "{}",
        h.t.screen()
    );
    assert_eq!(edits(&h), vec![Edit::MemoriesReset]);
}

/// ADR-011 (beyond the oracle): a tap puts the cursor on a row, a second tap
/// does what Space (a switch) or Enter (the reset row, a choice) does.
#[test]
fn a_tap_moves_the_cursor_a_second_tap_flips_a_switch_asks_to_reset_or_chooses() {
    let mut h = open("memories_settings");
    let (y, x) = find(&h, "Generate memories");
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [ ] Generate memories"),
        "{}",
        h.t.screen()
    );
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("› [x] Generate memories"),
        "{}",
        h.t.screen()
    );
    let (y, x) = find(&h, "Reset all memories");
    h.t.click(x, y);
    h.settle();
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("Reset all memories?"),
        "{}",
        h.t.screen()
    );
    let (y, x) = find(&h, "Go back");
    h.t.click(x, y);
    h.settle();
    assert!(h.t.screen().contains("› Go back"), "{}", h.t.screen());
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("Choose how Codex uses"),
        "{}",
        h.t.screen()
    );
}

// ---- /theme through the app: the model builds the picker from its facts,
// so these tests read the real model (a debug entry's lives in its seeded
// scope) ----

fn theme_command() -> Harness {
    let mut h = start_driven(false, WIDTH, HEIGHT);
    h.wait_idle();
    h.t.type_text("/theme");
    h.press("enter");
    h.wait_for("the picker", |screen| {
        screen.contains("Select Syntax Theme")
    });
    h.advance(600);
    h
}

fn syntax_theme(h: &Harness) -> Option<String> {
    h.t.read_app(|app: &UiApp| app.core.screens().settings.syntax_theme.clone())
}

#[test]
fn slash_theme_opens_the_picker_over_the_bundled_themes() {
    let h = theme_command();
    let screen = h.t.screen();
    // The config names no theme: the cursor starts on the terminal's default theme, which the
    // row says is in force (`build_theme_picker_params`: `effective_name`).
    assert!(screen.contains("› catppuccin-mocha (current)"), "{screen}");
    assert!(screen.contains("base16-ocean-light"), "{screen}");
    let themes =
        h.t.read_app(
            |app: &UiApp| match app.core.screens().settings.offers.get(&Which::Theme) {
                Some(Offer::Theme(picker)) => picker.themes.len(),
                _ => 0,
            },
        );
    assert_eq!(themes, 32);
}

#[test]
fn the_cursor_previews_in_the_model_and_esc_restores_the_theme_in_force() {
    let mut h = theme_command();
    assert_eq!(syntax_theme(&h), None);
    h.press("down");
    h.wait_until("the model previews coldark-cold", |h| {
        syntax_theme(h).as_deref() == Some("coldark-cold")
    });
    h.press("down");
    h.wait_until("the model previews coldark-dark", |h| {
        syntax_theme(h).as_deref() == Some("coldark-dark")
    });
    h.press("esc");
    h.wait_until("Esc restores the theme in force", |h| {
        syntax_theme(h).is_none()
    });
    assert!(!h.t.screen().contains("Select Syntax Theme"));
}

#[test]
fn enter_picks_the_theme_under_the_cursor_and_asks_for_it_to_be_saved() {
    let mut h = theme_command();
    h.press("down");
    h.press("down");
    h.press("enter");
    h.wait_until("the picked theme is in force and recorded", |h| {
        syntax_theme(h).as_deref() == Some("coldark-dark")
            && h.t
                .read_app(|app: &UiApp| !app.core.screens().settings.edits.is_empty())
    });
    let edits =
        h.t.read_app(|app: &UiApp| app.core.screens().settings.edits.clone());
    assert_eq!(edits, vec![Edit::ThemeSelected("coldark-dark".to_string())]);
    assert!(!h.t.screen().contains("Select Syntax Theme"));
}

// ---- the hooks browser (`hooks_browser_view.rs`): two pages over the model's
// hooks; the cursors and the scroll are the view's controllers

/// The browser opened from the debug menu on a driven harness: a key's intent
/// travels to the shell and back, and the model's answer is on the screen
/// before the test reads it.
fn hooks(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_driven(true, width, height);
    h.open_debug_row("settings", Some(entry));
    h.advance(800);
    h
}

#[test]
fn hooks_keys_open_an_event_toggle_its_hooks_and_esc_goes_back_then_closes() {
    let mut h = hooks("hooks_events", 112, 24);
    let screen = h.t.screen();
    assert!(
        screen.contains("Lifecycle hooks from config and enabled plugins."),
        "{screen}"
    );
    assert!(screen.contains("enter details · esc close"), "{screen}");
    assert!(cursor_row(&h).starts_with("› PreToolUse"), "{screen}");
    h.press("down");
    assert!(
        cursor_row(&h).starts_with("› PermissionRequest"),
        "{}",
        h.t.screen()
    );
    h.press("up");
    h.press("up");
    assert!(
        cursor_row(&h).starts_with("› PostModelRequest"),
        "up wraps to the last: {}",
        h.t.screen()
    );
    h.press("down");
    assert!(
        cursor_row(&h).starts_with("› PreToolUse"),
        "{}",
        h.t.screen()
    );

    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("PreToolUse hooks"), "{screen}");
    assert!(screen.contains("space/enter toggle · esc back"), "{screen}");
    assert!(cursor_row(&h).starts_with("› [x]"), "{screen}");
    h.press("space");
    assert!(cursor_row(&h).starts_with("› [ ]"), "{}", h.t.screen());
    h.press("down");
    h.press("enter");
    assert!(cursor_row(&h).starts_with("› [x]"), "{}", h.t.screen());

    // Esc: back on the events page, on the event it came from; Esc: closed.
    h.press("esc");
    assert!(
        cursor_row(&h).starts_with("› PreToolUse"),
        "{}",
        h.t.screen()
    );
    assert!(
        h.t.screen().contains("enter details · esc close"),
        "{}",
        h.t.screen()
    );
    h.press("esc");
    assert!(
        !h.t.screen().contains("Lifecycle hooks"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn hooks_ctrl_c_closes_from_either_page() {
    let mut h = hooks("hooks_events", 112, 24);
    h.press("ctrl+c");
    assert!(
        !h.t.screen().contains("Lifecycle hooks"),
        "{}",
        h.t.screen()
    );
    let mut h = hooks("hooks_events", 112, 24);
    h.press("enter");
    assert!(
        h.t.screen().contains("PreToolUse hooks"),
        "{}",
        h.t.screen()
    );
    h.press("ctrl+c");
    assert!(
        !h.t.screen().contains("PreToolUse hooks"),
        "{}",
        h.t.screen()
    );
    assert!(
        !h.t.screen().contains("Lifecycle hooks"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn hooks_t_trusts_every_hook_to_review_from_the_events_and_one_from_its_page() {
    let mut h = hooks("hooks_events_review_mixed", 112, 24);
    assert!(
        h.t.screen()
            .contains("t trust all · enter review · esc close"),
        "{}",
        h.t.screen()
    );
    h.press("t");
    assert!(
        h.t.screen().contains("enter details · esc close"),
        "nothing is left to review: {}",
        h.t.screen()
    );

    let mut h = hooks("hooks_review_needed", 112, 24);
    h.press("enter");
    assert!(
        h.t.screen().contains("t trust · esc back"),
        "{}",
        h.t.screen()
    );
    // Space and Enter leave a hook waiting for a review as it is.
    h.press("space");
    h.press("enter");
    h.press("t");
    assert!(
        h.t.screen().contains("space/enter toggle · esc back"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn hooks_the_cursor_walks_a_long_page_by_the_window_and_to_its_ends() {
    let mut h = hooks("hooks_many", 112, 30);
    h.press("enter");
    assert!(
        h.t.screen().contains("↓"),
        "rows below the window: {}",
        h.t.screen()
    );
    assert!(!h.t.screen().contains("↑"), "{}", h.t.screen());
    h.press("end");
    assert!(cursor_row(&h).contains("11"), "{}", h.t.screen());
    assert!(
        h.t.screen().contains("↑"),
        "rows above the window: {}",
        h.t.screen()
    );
    assert!(!h.t.screen().contains("↓"), "{}", h.t.screen());
    h.press("home");
    assert!(cursor_row(&h).contains(" 1 "), "{}", h.t.screen());
    h.press("pagedown");
    assert!(
        cursor_row(&h).contains(" 9 "),
        "a page is the window: {}",
        h.t.screen()
    );
    h.press("pageup");
    assert!(cursor_row(&h).contains(" 1 "), "{}", h.t.screen());
}

#[test]
fn hooks_a_tap_moves_the_cursor_to_an_event_and_a_second_tap_opens_it() {
    let mut h = hooks("hooks_events", 112, 24);
    let (y, x) = find(&h, "PermissionRequest");
    h.t.click(x, y);
    h.advance(100);
    assert!(
        cursor_row(&h).starts_with("› PermissionRequest"),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("Lifecycle hooks"), "{}", h.t.screen());
    h.t.click(x, y);
    h.advance(100);
    assert!(
        h.t.screen().contains("PermissionRequest hooks"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn hooks_a_tap_moves_the_cursor_to_a_hook_and_a_second_tap_toggles_it_and_the_wheel_scrolls() {
    let mut h = hooks("hooks_events", 112, 24);
    h.press("enter");
    let (y, x) = find(&h, "[ ]");
    h.t.click(x + 6, y);
    h.advance(100);
    assert!(cursor_row(&h).starts_with("› [ ]"), "{}", h.t.screen());
    h.t.click(x + 6, y);
    h.advance(100);
    assert!(cursor_row(&h).starts_with("› [x]"), "{}", h.t.screen());

    let mut h = hooks("hooks_many", 112, 30);
    h.press("enter");
    let (y, x) = find(&h, "Check tool 3");
    assert!(
        h.t.screen().contains("↓") && !h.t.screen().contains("↑"),
        "{}",
        h.t.screen()
    );
    for _ in 0..4 {
        h.t.mouse(MouseKind::WheelDown, x, y, 0, Mods::NONE);
    }
    h.advance(100);
    assert!(
        h.t.screen().contains("↑"),
        "the wheel scrolled the window: {}",
        h.t.screen()
    );
    assert!(!h.t.screen().contains("↓"), "{}", h.t.screen());
}
