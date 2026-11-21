#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S15 (settings dialogs), one variant each: the configuration
//! panel (`config_panel/config_panel_tests.rs`), the syntax theme picker
//! (`theme_picker_model_tests.rs`), the status-line and title setups
//! (`bottom_pane/status_line_setup.rs`, `title_setup.rs` tests) and the
//! keymap picker (`keymap_setup.rs`: `picker_content_snapshot`).
//!
//! The cursor, the group tab and the search text are controllers of the view
//! (`SelectionController`, `TabController`, `TextEditingController`); here is
//! what they leave to the model: what a group or a search shows
//! (`visible`), and what an intent changes.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::settings::{
    ConfigPanel, Edit, Intent as SettingsIntent, Open, Section, SettingId, Setup, SetupKind,
    USE_THEME_COLORS, Value,
};
use pretty_assertions::assert_eq;

const CONFIG: &str = include_str!("../../src/sheets/settings/fixtures/config_panel.jsonl");
const THEME: &str = include_str!("../../src/sheets/settings/fixtures/theme_picker.jsonl");
const THEME_ALL: &str = include_str!("../../src/sheets/settings/fixtures/theme_picker_all.jsonl");
const STATUS_LINE: &str = include_str!("../../src/sheets/settings/fixtures/status_line.jsonl");
const TITLE: &str = include_str!("../../src/sheets/settings/fixtures/title.jsonl");
const KEYMAP: &str = include_str!("../../src/sheets/settings/fixtures/keymap.jsonl");

fn send(sandbox: &mut Sandbox, intent: SettingsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Settings(
        intent,
    ))));
}

fn panel(sandbox: &Sandbox) -> ConfigPanel {
    match &sandbox.app().screens().settings.open {
        Some(Open::Config(panel)) => panel.clone(),
        other => panic!("config panel expected, got {other:?}"),
    }
}

fn setup(sandbox: &Sandbox) -> Setup {
    match &sandbox.app().screens().settings.open {
        Some(Open::Setup(setup)) => setup.clone(),
        other => panic!("setup expected, got {other:?}"),
    }
}

fn labels(panel: &ConfigPanel, section: Section, query: &str) -> Vec<String> {
    panel
        .visible(section, query)
        .into_iter()
        .map(|index| panel.rows[index].label.clone())
        .collect()
}

#[test]
fn config_panel_groups_hold_the_oracles_rows() {
    let sandbox = Sandbox::play(CONFIG).expect("plays");
    let panel = panel(&sandbox);
    let counts = [6, 3, 4, 5, 3];
    for (section, count) in Section::ALL.into_iter().zip(counts) {
        assert_eq!(panel.visible(section, "").len(), count, "{section:?}");
    }
    // A search covers every group at once and ignores the tab.
    let found = panel.visible(Section::Appearance, "notify when");
    assert_eq!(found.len(), 1);
    assert_eq!(panel.rows[found[0]].id, SettingId::NotifyWhen);
    assert_eq!(
        panel.visible(Section::TextSelection, "notify when"),
        found,
        "the same rows in whichever group"
    );
}

#[test]
fn search_finds_settings_across_groups() {
    let sandbox = Sandbox::play(CONFIG).expect("plays");
    assert_eq!(
        labels(&panel(&sandbox), Section::Sessions, "notify when"),
        vec!["Notify when".to_string()]
    );
    let found = panel(&sandbox).visible(Section::Appearance, "composer stars");
    assert_eq!(panel(&sandbox).rows[found[0]].id, SettingId::Effects);
}

#[test]
fn search_matches_displayed_values() {
    let script = CONFIG.replace("\"Automatic\"", "\"Dracula\"");
    let sandbox = Sandbox::play(&script).expect("plays");
    let panel = panel(&sandbox);
    let found = panel.visible(Section::Appearance, "Dracula");
    assert_eq!(panel.rows[found[0]].id, SettingId::Theme);
}

#[test]
fn activating_a_toggle_row_flips_it_and_asks_the_app_to_persist_it() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Activate(SettingId::Tips));
    let panel = panel(&sandbox);
    assert_eq!(
        panel.row(SettingId::Tips).unwrap().value,
        Value::Toggle(true)
    );
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::Toggled(SettingId::Tips, true)]
    );
}

#[test]
fn activating_a_choice_row_opens_its_editor() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Activate(SettingId::Effects));
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::Editor(SettingId::Effects)]
    );
    assert!(
        sandbox.app().screens().settings.open.is_some(),
        "the panel is not complete"
    );
}

#[test]
fn activating_the_test_row_sends_the_test_notification() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::Activate(SettingId::TestNotification),
    );
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::TestNotification]
    );
}

#[test]
fn esc_closes_the_panel() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens().settings.open.is_none());
}

const THEMES: [&str; 6] = ["ada", "babbage", "curie", "cushman", "dali", "davinci"];

#[test]
fn model_theme_files_preview_select_and_restore() {
    for (index, name) in THEMES.into_iter().enumerate() {
        let script = THEME.replace("\"current\":\"ada\"", &format!("\"current\":\"{name}\""));
        let mut sandbox = Sandbox::play(&script).expect("plays");
        let picker = match &sandbox.app().screens().settings.open {
            Some(Open::Theme(picker)) => picker.clone(),
            other => panic!("theme picker expected, got {other:?}"),
        };
        // The picker opens on the configured theme.
        assert_eq!(picker.initial(), index);
        // A cursor on another theme previews it, and back.
        let other = THEMES[(index + 1) % THEMES.len()];
        send(&mut sandbox, SettingsIntent::Preview(other.into()));
        assert_eq!(
            sandbox.app().screens().settings.syntax_theme.as_deref(),
            Some(other)
        );
        send(&mut sandbox, SettingsIntent::Preview(name.into()));
        assert_eq!(
            sandbox.app().screens().settings.syntax_theme.as_deref(),
            Some(name)
        );
        // Enter selects it.
        send(&mut sandbox, SettingsIntent::Pick(name.into()));
        assert_eq!(
            sandbox.app().screens().settings.edits,
            vec![Edit::ThemeSelected(name.to_string())]
        );
        assert!(sandbox.app().screens().settings.open.is_none());
    }
}

#[test]
fn cancelling_the_theme_picker_restores_the_theme_in_force() {
    let mut sandbox = Sandbox::play(THEME).expect("plays");
    send(&mut sandbox, SettingsIntent::Preview("curie".into()));
    assert_eq!(
        sandbox.app().screens().settings.syntax_theme.as_deref(),
        Some("curie")
    );
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(
        sandbox.app().screens().settings.syntax_theme.as_deref(),
        Some("ada")
    );
    assert!(sandbox.app().screens().settings.edits.is_empty());
    assert!(sandbox.app().screens().settings.open.is_none());
}

/// `theme_picker.rs` `SelectionItem::search_value`: the typed text matches a
/// name in any case; custom files and the directory come from the seed.
#[test]
fn the_theme_picker_filters_by_name_and_knows_the_custom_files() {
    let sandbox = Sandbox::play(THEME_ALL).expect("play");
    let Some(Open::Theme(picker)) = &sandbox.app().screens().settings.open else {
        panic!("theme picker open");
    };
    assert_eq!(picker.themes.len(), 33);
    let names = |query: &str| -> Vec<&str> {
        picker
            .visible(query)
            .into_iter()
            .map(|at| picker.themes[at].as_str())
            .collect()
    };
    assert_eq!(names("NIGHT"), vec!["my-night"]);
    assert_eq!(
        names("coldark"),
        vec!["coldark-cold", "coldark-dark"],
        "builtin order kept"
    );
    assert!(names("zzz").is_empty());
    assert_eq!(names("").len(), 33);
    assert!(picker.is_custom("my-night"));
    assert!(!picker.is_custom("nord"));
    assert_eq!(picker.initial(), 24, "the row of the theme in force");
    assert_eq!(picker.themes_dir.as_deref(), Some("~/.codex/themes"));
}

/// `list_available_themes`: the bundled names and the custom files sorted
/// together without regard to case, a custom file named like a bundled theme
/// adds nothing; the picker opens on the theme the config names.
#[test]
fn opening_the_theme_picker_builds_it_from_the_facts() {
    use codex_tui_core::app::AppFx;
    use codex_tui_core::screens::settings::{Dialog, Ev, Facts, Model, apply, apply_intent};
    let mut fx = AppFx::default();
    let mut model = Model::default();
    apply(
        &mut model,
        Ev::Facts(Facts {
            syntax_theme: Some("Zebra".into()),
            custom_themes: vec!["Zebra".into(), "my-night".into(), "nord".into()],
            themes_dir: Some("~/.codex/themes".into()),
        }),
        &mut fx,
    );
    assert_eq!(model.syntax_theme.as_deref(), Some("Zebra"));
    apply_intent(&mut model, SettingsIntent::Open(Dialog::Theme), &mut fx);
    let Some(Open::Theme(picker)) = &model.open else {
        panic!("theme picker open");
    };
    assert_eq!(picker.themes.len(), 34, "32 bundled, 2 new custom");
    assert_eq!(
        picker.custom,
        vec!["Zebra".to_string(), "my-night".to_string()]
    );
    assert_eq!(picker.themes.first().map(String::as_str), Some("1337"));
    // "my-night" sorts between "monokai-extended-origin" and "nord"; "Zebra" after "zenburn"? no: z-e-b < z-e-n
    let at = |name: &str| picker.themes.iter().position(|theme| theme == name);
    assert!(at("monokai-extended-origin") < at("my-night"));
    assert!(at("my-night") < at("nord"));
    assert!(at("two-dark") < at("Zebra") && at("Zebra") < at("zenburn"));
    assert_eq!(picker.initial(), at("Zebra").unwrap_or(usize::MAX));
    assert_eq!(picker.themes_dir.as_deref(), Some("~/.codex/themes"));
    // facts that arrive while the picker previews do not undo the preview
    apply_intent(&mut model, SettingsIntent::Preview("nord".into()), &mut fx);
    apply(&mut model, Ev::Facts(Facts::default()), &mut fx);
    assert_eq!(model.syntax_theme.as_deref(), Some("nord"));
}

#[test]
fn an_unknown_theme_is_ignored() {
    let mut sandbox = Sandbox::play(THEME).expect("plays");
    send(&mut sandbox, SettingsIntent::Pick("nope".into()));
    assert!(sandbox.app().screens().settings.edits.is_empty());
    assert!(sandbox.app().screens().settings.open.is_some());
}

fn ids(sandbox: &Sandbox) -> Vec<(String, bool)> {
    setup(sandbox)
        .items
        .into_iter()
        .map(|item| (item.id, item.enabled))
        .collect()
}

fn order(sandbox: &Sandbox) -> Vec<String> {
    ids(sandbox).into_iter().map(|(id, _)| id).collect()
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
    assert_eq!(setup(&sandbox).kind, SetupKind::StatusLine);
    assert_eq!(
        setup(&sandbox).preview(),
        "gpt-5-codex · ~/codex-rs · jif/statusline-preview"
    );
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
fn toggling_an_item_changes_the_preview() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::Toggle("model-with-reasoning".into()),
    );
    assert_eq!(
        setup(&sandbox).preview(),
        "gpt-5-codex · ~/codex-rs · jif/statusline-preview · gpt-5.2-codex medium"
    );
}

#[test]
fn moving_reorders_the_orderable_items_and_stops_at_the_fixed_row() {
    let mut sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    // The fixed "use theme colors" row neither moves nor is crossed.
    send(
        &mut sandbox,
        SettingsIntent::Move {
            id: USE_THEME_COLORS.into(),
            delta: 1,
        },
    );
    assert_eq!(order(&sandbox)[0], USE_THEME_COLORS);
    send(
        &mut sandbox,
        SettingsIntent::Move {
            id: "model".into(),
            delta: -1,
        },
    );
    assert_eq!(order(&sandbox)[1], "model");
    send(
        &mut sandbox,
        SettingsIntent::Move {
            id: "model".into(),
            delta: 1,
        },
    );
    assert_eq!(
        &order(&sandbox)[1..4],
        ["current-dir", "model", "git-branch"]
    );
    assert_eq!(
        setup(&sandbox).preview(),
        "~/codex-rs · gpt-5-codex · jif/statusline-preview"
    );
}

#[test]
fn enter_saves_the_enabled_items_and_the_color_preference() {
    for use_theme_colors in [false, true] {
        let script = STATUS_LINE.replace(
            "\"use_theme_colors\":true",
            &format!("\"use_theme_colors\":{use_theme_colors}"),
        );
        let mut sandbox = Sandbox::play(&script).expect("plays");
        send(&mut sandbox, SettingsIntent::Save);
        assert_eq!(
            sandbox.app().screens().settings.edits,
            vec![Edit::SetupSaved {
                kind: SetupKind::StatusLine,
                items: vec!["model".into(), "current-dir".into(), "git-branch".into()],
                use_theme_colors,
            }]
        );
        assert!(sandbox.app().screens().settings.open.is_none());
    }
}

#[test]
fn escape_cancels_the_setup() {
    for (script, kind) in [
        (STATUS_LINE, SetupKind::StatusLine),
        (TITLE, SetupKind::Title),
    ] {
        let mut sandbox = Sandbox::play(script).expect("plays");
        send(&mut sandbox, SettingsIntent::Close);
        assert_eq!(
            sandbox.app().screens().settings.edits,
            vec![Edit::SetupCancelled(kind)]
        );
        assert!(sandbox.app().screens().settings.open.is_none());
    }
}

#[test]
fn a_search_ranks_by_fuzzy_score_then_name() {
    let sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    let setup = setup(&sandbox);
    let names = |query: &str| -> Vec<String> {
        setup
            .visible(query)
            .into_iter()
            .map(|index| setup.items[index].id.clone())
            .collect()
    };
    assert_eq!(names("branch"), vec!["git-branch"]);
    assert_eq!(names("").len(), setup.items.len());
    // `m-w-r`: a subsequence of model-with-reasoning only.
    assert_eq!(names("mwr"), vec!["model-with-reasoning"]);
    assert!(names("zzz").is_empty());
}

#[test]
fn the_title_setup_starts_with_the_selected_items_and_previews_them() {
    let sandbox = Sandbox::play(TITLE).expect("plays");
    let setup = setup(&sandbox);
    assert_eq!(setup.kind, SetupKind::Title);
    assert_eq!(
        &order(&sandbox)[..8],
        [
            "project-name",
            "activity",
            "run-state",
            "thread-title",
            "app-name",
            "current-dir",
            "thread-name",
            "git-branch",
        ]
    );
    assert!(!setup.items.iter().any(|item| item.id == USE_THEME_COLORS));
    assert_eq!(
        setup.preview(),
        "[ ! ] Action Required | my-project | Working | thread title"
    );
}

#[test]
fn a_title_without_the_activity_joins_with_bars_and_without_the_prefix() {
    let mut sandbox = Sandbox::play(TITLE).expect("plays");
    send(&mut sandbox, SettingsIntent::Toggle("activity".into()));
    assert_eq!(
        setup(&sandbox).preview(),
        "my-project | Working | thread title"
    );
    for id in ["run-state", "thread-title", "project-name"] {
        send(&mut sandbox, SettingsIntent::Toggle(id.into()));
    }
    assert_eq!(setup(&sandbox).preview(), "");
}

/// The text of the oracle's `keymap_picker_first_actions` snapshot.
fn keymap_dump(sandbox: &Sandbox) -> String {
    let picker = match &sandbox.app().screens().settings.open {
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
fn the_keymap_picker_filters_a_tab_and_picks_an_action() {
    let mut sandbox = Sandbox::play(KEYMAP).expect("plays");
    let picker = match &sandbox.app().screens().settings.open {
        Some(Open::Keymap(picker)) => picker.clone(),
        other => panic!("keymap picker expected, got {other:?}"),
    };
    let names: Vec<&str> = picker
        .visible(0, "transcript")
        .into_iter()
        .map(|index| picker.tabs[0].items[index].name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["Open Transcript", "Find Transcript", "Focus Activity"]
    );
    send(
        &mut sandbox,
        SettingsIntent::KeymapPick("Open Transcript".into()),
    );
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::KeymapPicked("Open Transcript".into())]
    );
    assert!(sandbox.app().screens().settings.open.is_some());
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens().settings.open.is_none());
}

#[test]
fn bad_seeds_are_refused() {
    assert!(Sandbox::play(r#"{"step":"screen","dir":"settings","ev":{"kind":"nope"}}"#).is_err());
}
