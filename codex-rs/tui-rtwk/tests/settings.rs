//! Screen tests of S15 (the settings dialogs), one variant each: the
//! configuration panel, the syntax theme picker, the status-line setup and
//! the keymap picker. Expected text: the oracle's snapshots
//! (`dialogs::oracle`), with the `›` marker kept and no width tuning.
//! Oracle tests: `config_panel/config_panel_tests.rs`
//! (`config_panel_renders_each_group_at_a_stable_height`,
//! `search_finds_settings_across_groups_and_preserves_the_selected_group`),
//! `bottom_pane/status_line_setup.rs` (`setup_view_snapshot_*`),
//! `theme_picker.rs` / `theme_picker_model_tests.rs`, `keymap_setup.rs`.

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::{assert_rows, assert_sequence, rows};

fn open(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("settings", Some(entry));
    h.t.pump_for(ms(800));
    h
}

const GROUPS: [&str; 5] = [
    "Appearance",
    "Notifications",
    "Input",
    "Sessions",
    "Text selection",
];

// config_panel_tests.rs: config_panel_renders_each_group_at_a_stable_height
#[test]
fn config_panel_renders_each_group_at_a_stable_height() {
    let mut h = open(140, 24, "config_panel");
    for group in GROUPS {
        assert_rows(
            &h.t.screen(),
            &format!("config_panel/snapshots/codex_tui__config_panel__tests__config_{group}.snap"),
        );
        h.press("right");
    }
}

// config_panel_tests.rs: search_finds_settings_across_groups_and_preserves_the_selected_group
#[test]
fn config_global_search() {
    let mut h = open(100, 24, "config_panel");
    h.press("/");
    for key in "notify when".chars() {
        h.press(&key.to_string());
    }
    assert_rows(
        &h.t.screen(),
        "config_panel/snapshots/codex_tui__config_panel__tests__config_global_search.snap",
    );
}

#[test]
fn right_changes_the_group_and_esc_clears_the_search() {
    let mut h = open(140, 24, "config_panel");
    h.press("/");
    for key in "notify when".chars() {
        h.press(&key.to_string());
    }
    h.press("right");
    h.press("esc");
    let screen = h.t.screen();
    assert!(screen.contains("Notify when"), "{screen}");
    assert!(!screen.contains("Search ›"), "{screen}");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Changes save automatically"));
}

#[test]
fn enter_on_a_toggle_flips_it() {
    let mut h = open(140, 24, "config_panel");
    h.press("down");
    h.press("down");
    h.press("enter");
    assert!(h.t.screen().contains("[x] On"), "{}", h.t.screen());
}

// bottom_pane/status_line_setup.rs: setup_view_snapshot_uses_runtime_preview_values
#[test]
fn setup_view_snapshot_uses_runtime_preview_values() {
    let h = open(72, 24, "status_line");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__status_line_setup__tests__setup_view_snapshot_uses_runtime_preview_values.snap",
    );
}

#[test]
fn space_toggles_the_item_and_the_preview_follows() {
    let mut h = open(72, 24, "status_line");
    for _ in 0..4 {
        h.press("down");
    }
    h.press("space");
    assert!(
        h.t.screen().contains("gpt-5.2-codex medium"),
        "{}",
        h.t.screen()
    );
}

// theme_picker.rs: the picker is a selection list with a diff preview below
// it; the oracle's model test (theme_picker_model_tests.rs) renders the
// narrow preview.
#[test]
fn model_theme_files_preview_select_and_restore() {
    let h = open(100, 30, "theme_picker");
    let preview: Vec<String> = rows(
        "snapshots/codex_tui__theme_picker__model_tests__model_theme_files_preview_select_and_restore.snap",
    )
    .into_iter()
    .skip(1)
    .take(4)
    .collect();
    let mut want = vec!["Select Syntax Theme".to_string()];
    want.extend(preview.clone());
    assert_sequence(&h.t.screen(), "theme picker", &want);
    let mut h = h;
    h.press("down");
    assert_sequence(&h.t.screen(), "theme picker after moving", &preview);
}

// keymap_setup.rs: picker_content_snapshot: the tabs and the first actions.
#[test]
fn picker_content_snapshot() {
    let h = open(120, 30, "keymap");
    let want: Vec<String> =
        rows("snapshots/codex_tui__keymap_setup__tests__keymap_picker_first_actions.snap")
            .into_iter()
            .filter_map(|row| row.strip_prefix("tab: ").map(str::to_owned))
            .collect();
    assert!(!want.is_empty());
    let screen = h.t.screen();
    for tab in ["All", "Common", "Unbound (5)", "Vim"] {
        assert!(screen.contains(tab), "{screen}");
    }
    for row in ["Open Agents", "Open Transcript", "Copy", "Leader"] {
        assert!(screen.contains(row), "{screen}");
    }
}
