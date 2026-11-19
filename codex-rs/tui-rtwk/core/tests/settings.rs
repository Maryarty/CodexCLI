#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S15 (settings dialogs), one variant each: the configuration
//! panel (`config_panel/config_panel_tests.rs`), the syntax theme picker
//! (`theme_picker_model_tests.rs`), the status-line setup
//! (`bottom_pane/status_line_setup.rs` tests) and the keymap picker
//! (`keymap_setup.rs`: `picker_content_snapshot`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::settings::{
    Edit, Intent as SettingsIntent, Open, Section, SettingId, USE_THEME_COLORS, Value,
};
use pretty_assertions::assert_eq;

const CONFIG: &str = include_str!("../../src/sheets/settings/fixtures/config_panel.jsonl");
const THEME: &str = include_str!("../../src/sheets/settings/fixtures/theme_picker.jsonl");
const STATUS_LINE: &str = include_str!("../../src/sheets/settings/fixtures/status_line.jsonl");
const KEYMAP: &str = include_str!("../../src/sheets/settings/fixtures/keymap.jsonl");

fn send(sandbox: &mut Sandbox, intent: SettingsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Settings(
        intent,
    ))));
}

fn panel(sandbox: &Sandbox) -> codex_tui_core::screens::settings::ConfigPanel {
    match &sandbox.app().screens.settings.open {
        Some(Open::Config(panel)) => panel.clone(),
        other => panic!("config panel expected, got {other:?}"),
    }
}

fn setup(sandbox: &Sandbox) -> codex_tui_core::screens::settings::StatusLineSetup {
    match &sandbox.app().screens.settings.open {
        Some(Open::StatusLine(setup)) => setup.clone(),
        other => panic!("status line setup expected, got {other:?}"),
    }
}

fn labels(sandbox: &Sandbox) -> Vec<String> {
    let panel = panel(sandbox);
    panel
        .visible()
        .into_iter()
        .map(|index| panel.rows[index].label.clone())
        .collect()
}

#[test]
fn config_panel_renders_each_group_at_a_stable_height() {
    // The rows of each group, in the oracle's order; Right past the last
    // group wraps to the first.
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    let counts = [6, 3, 4, 5, 3];
    for (tab, count) in counts.into_iter().enumerate() {
        assert_eq!(panel(&sandbox).tab, tab);
        assert_eq!(panel(&sandbox).section(), Section::ALL[tab]);
        assert_eq!(panel(&sandbox).visible().len(), count, "group {tab}");
        send(&mut sandbox, SettingsIntent::Move(1));
        send(&mut sandbox, SettingsIntent::Move(-1));
        assert_eq!(panel(&sandbox).cursor, 0);
        send(&mut sandbox, SettingsIntent::Tab(1));
    }
    assert_eq!(panel(&sandbox).tab, 0);
    send(&mut sandbox, SettingsIntent::Move(-1));
    assert_eq!(panel(&sandbox).cursor, 5, "Up from the first row wraps");
}

#[test]
fn search_finds_settings_across_groups_and_preserves_the_selected_group() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Search("notify when".into()));
    assert_eq!(labels(&sandbox), vec!["Notify when".to_string()]);

    send(&mut sandbox, SettingsIntent::Tab(1));
    assert_eq!(panel(&sandbox).tab, 1);
    assert_eq!(panel(&sandbox).query, "notify when");
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(panel(&sandbox).tab, 1);
    assert_eq!(panel(&sandbox).query, "");

    send(
        &mut sandbox,
        SettingsIntent::Search("composer stars".into()),
    );
    send(&mut sandbox, SettingsIntent::Activate);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::Editor(SettingId::Effects)]
    );
    assert!(
        sandbox.app().screens.settings.open.is_some(),
        "the panel is not complete"
    );
}

#[test]
fn search_matches_displayed_values() {
    let script = CONFIG.replace("\"Automatic\"", "\"Dracula\"");
    let mut sandbox = Sandbox::play(&script).expect("plays");
    send(&mut sandbox, SettingsIntent::Search("Dracula".into()));
    send(&mut sandbox, SettingsIntent::Activate);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::Editor(SettingId::Theme)]
    );
}

#[test]
fn enter_flips_a_toggle_row_and_asks_the_app_to_persist_it() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Move(2));
    send(&mut sandbox, SettingsIntent::Activate);
    let panel = panel(&sandbox);
    assert_eq!(panel.rows[2].id, SettingId::Tips);
    assert_eq!(panel.rows[2].value, Value::Toggle(true));
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::Toggled(SettingId::Tips, true)]
    );
}

#[test]
fn enter_on_the_test_row_sends_the_test_notification() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Tab(1));
    send(&mut sandbox, SettingsIntent::Move(2));
    send(&mut sandbox, SettingsIntent::Activate);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::TestNotification]
    );
}

#[test]
fn esc_closes_the_panel_when_no_search_is_typed() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens.settings.open.is_none());
}

#[test]
fn model_theme_files_preview_select_and_restore() {
    for name in ["ada", "babbage", "curie", "cushman", "dali", "davinci"] {
        let script = THEME.replace("\"ada\"}", &format!("\"{name}\"}}"));
        let mut sandbox = Sandbox::play(&script).expect("plays");
        let themes = ["ada", "babbage", "curie", "cushman", "dali", "davinci"];
        let index = themes.iter().position(|theme| *theme == name).unwrap();
        let picker = match &sandbox.app().screens.settings.open {
            Some(Open::Theme(picker)) => picker.clone(),
            other => panic!("theme picker expected, got {other:?}"),
        };
        // The picker opens on the configured theme.
        assert_eq!(picker.cursor, index);
        assert_eq!(picker.themes[index], name);
        // Moving previews the theme under the cursor and back.
        send(&mut sandbox, SettingsIntent::Move(1));
        let other = themes[(index + 1) % themes.len()];
        assert_eq!(
            sandbox.app().screens.settings.syntax_theme.as_deref(),
            Some(other)
        );
        send(&mut sandbox, SettingsIntent::Move(-1));
        assert_eq!(
            sandbox.app().screens.settings.syntax_theme.as_deref(),
            Some(name)
        );
        // Enter selects it.
        send(&mut sandbox, SettingsIntent::Activate);
        assert_eq!(
            sandbox.app().screens.settings.edits,
            vec![Edit::ThemeSelected(name.to_string())]
        );
        assert!(sandbox.app().screens.settings.open.is_none());
    }
}

#[test]
fn cancelling_the_theme_picker_restores_the_theme_in_force() {
    let mut sandbox = Sandbox::play(THEME).expect("plays");
    send(&mut sandbox, SettingsIntent::Move(2));
    assert_eq!(
        sandbox.app().screens.settings.syntax_theme.as_deref(),
        Some("curie")
    );
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(
        sandbox.app().screens.settings.syntax_theme.as_deref(),
        Some("ada")
    );
    assert!(sandbox.app().screens.settings.edits.is_empty());
    assert!(sandbox.app().screens.settings.open.is_none());
}

fn ids(sandbox: &Sandbox) -> Vec<(String, bool)> {
    setup(sandbox)
        .items
        .into_iter()
        .map(|item| (item.id, item.enabled))
        .collect()
}

#[test]
fn setup_view_snapshot_uses_runtime_preview_values() {
    let sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    let want: Vec<(String, bool)> = [
        (USE_THEME_COLORS, true),
        ("model", true),
        ("current-dir", true),
        ("git-branch", true),
        ("model-with-reasoning", false),
        ("reasoning", false),
        ("project-name", false),
        ("weekly-limit", false),
    ]
    .into_iter()
    .map(|(id, on)| (id.to_string(), on))
    .collect();
    assert_eq!(ids(&sandbox), want);
    assert_eq!(
        setup(&sandbox).preview(),
        "gpt-5-codex · ~/codex-rs · jif/statusline-preview"
    );
    assert_eq!(setup(&sandbox).cursor, 0);
}

#[test]
fn preview_uses_placeholders_when_runtime_values_are_missing() {
    let script = STATUS_LINE
        .replace(
            "\"selected\":[\"model\",\"current-dir\",\"git-branch\"]",
            "\"selected\":[\"model\",\"git-branch\"]",
        )
        .replace(
            "\"values\":{\"model\":\"gpt-5-codex\",\"current-dir\":\"~/codex-rs\",\"git-branch\":\"jif/statusline-preview\",\"weekly-limit\":\"weekly 82% left\"}",
            "\"values\":{\"model\":\"gpt-5\"}",
        );
    let sandbox = Sandbox::play(&script).expect("plays");
    assert_eq!(setup(&sandbox).preview(), "gpt-5 · feat/awesome-feature");
}

#[test]
fn space_toggles_an_item_and_the_preview_follows() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    for _ in 0..4 {
        send(&mut sandbox, SettingsIntent::Move(1));
    }
    send(&mut sandbox, SettingsIntent::Toggle);
    assert_eq!(
        setup(&sandbox).preview(),
        "gpt-5-codex · ~/codex-rs · jif/statusline-preview · gpt-5.2-codex medium"
    );
}

#[test]
fn arrows_reorder_the_orderable_items_and_stop_at_the_fixed_row() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    // On the fixed "use theme colors" row nothing moves.
    send(&mut sandbox, SettingsIntent::Reorder(1));
    assert_eq!(ids(&sandbox)[0].0, USE_THEME_COLORS);
    send(&mut sandbox, SettingsIntent::Move(1));
    // "model" cannot move up past the fixed row.
    send(&mut sandbox, SettingsIntent::Reorder(-1));
    assert_eq!(ids(&sandbox)[1].0, "model");
    send(&mut sandbox, SettingsIntent::Reorder(1));
    let order: Vec<String> = ids(&sandbox).into_iter().map(|(id, _)| id).collect();
    assert_eq!(&order[1..4], ["current-dir", "model", "git-branch"]);
    assert_eq!(
        setup(&sandbox).cursor,
        2,
        "the cursor follows the moved row"
    );
    assert_eq!(
        setup(&sandbox).preview(),
        "~/codex-rs · gpt-5-codex · jif/statusline-preview"
    );
}

#[test]
fn picker_previews_the_pr_link_and_preserves_the_color_preference() {
    for use_theme_colors in [false, true] {
        let script = STATUS_LINE.replace(
            "\"use_theme_colors\":true",
            &format!("\"use_theme_colors\":{use_theme_colors}"),
        );
        let mut sandbox = Sandbox::play(&script).expect("plays");
        send(&mut sandbox, SettingsIntent::Activate);
        assert_eq!(
            sandbox.app().screens.settings.edits,
            vec![Edit::StatusLineSaved {
                items: vec!["model".into(), "current-dir".into(), "git-branch".into()],
                use_theme_colors,
            }]
        );
        assert!(sandbox.app().screens.settings.open.is_none());
    }
}

#[test]
fn escape_cancels_the_status_line_setup() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::StatusLineCancelled]
    );
    assert!(sandbox.app().screens.settings.open.is_none());
}

#[test]
fn typing_filters_the_status_line_items_and_blocks_reordering() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    send(&mut sandbox, SettingsIntent::Search("branch".into()));
    assert_eq!(setup(&sandbox).visible().len(), 1);
    send(&mut sandbox, SettingsIntent::Reorder(-1));
    let order: Vec<String> = ids(&sandbox).into_iter().map(|(id, _)| id).collect();
    assert_eq!(&order[1..4], ["model", "current-dir", "git-branch"]);
}

/// The text of the oracle's `keymap_picker_first_actions` snapshot.
fn keymap_dump(sandbox: &Sandbox) -> String {
    let picker = match &sandbox.app().screens.settings.open {
        Some(Open::Keymap(picker)) => picker.clone(),
        other => panic!("keymap picker expected, got {other:?}"),
    };
    let mut lines: Vec<String> = picker
        .tabs
        .iter()
        .map(|tab| format!("tab: {} ({} selectable)", tab.label, tab.selectable))
        .collect();
    lines.extend(
        picker.tabs[0]
            .items
            .iter()
            .take(13)
            .map(|item| format!("{} | {} | {}", item.name, item.binding, item.search)),
    );
    lines.join("\n")
}

#[test]
fn picker_content_snapshot() {
    let sandbox = Sandbox::play(KEYMAP).expect("plays");
    let text = include_str!(
        "../../../tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_first_actions.snap"
    );
    let want = text.splitn(3, "---\n").nth(2).expect("body").trim_end();
    assert_eq!(keymap_dump(&sandbox), want);
}

#[test]
fn the_keymap_picker_switches_tabs_and_picks_an_action() {
    let mut sandbox = Sandbox::play(KEYMAP).expect("plays");
    send(&mut sandbox, SettingsIntent::Move(1));
    send(&mut sandbox, SettingsIntent::Activate);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::KeymapPicked("Open Transcript".into())]
    );
    send(&mut sandbox, SettingsIntent::Tab(1));
    match &sandbox.app().screens.settings.open {
        Some(Open::Keymap(picker)) => assert_eq!(picker.tab, 1),
        other => panic!("keymap picker expected, got {other:?}"),
    }
}

#[test]
fn typing_filters_the_keymap_picker_and_esc_clears_it() {
    let mut sandbox = Sandbox::play(KEYMAP).expect("plays");
    send(&mut sandbox, SettingsIntent::Search("transcript".into()));
    let picker = match &sandbox.app().screens.settings.open {
        Some(Open::Keymap(picker)) => picker.clone(),
        other => panic!("keymap picker expected, got {other:?}"),
    };
    let names: Vec<&str> = picker
        .visible()
        .into_iter()
        .map(|index| picker.items()[index].name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["Open Transcript", "Find Transcript", "Focus Activity"]
    );
    send(&mut sandbox, SettingsIntent::Activate);
    assert_eq!(
        sandbox.app().screens.settings.edits,
        vec![Edit::KeymapPicked("Open Transcript".into())]
    );
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens.settings.open.is_some());
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens.settings.open.is_none());
}

#[test]
fn bad_seeds_are_refused() {
    assert!(Sandbox::play(r#"{"step":"screen","dir":"settings","ev":{"kind":"nope"}}"#).is_err());
}
