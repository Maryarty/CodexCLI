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
use codex_tui_core::screens::settings::hooks::{Hook, HookEvent, HookTrust, Hooks};
use codex_tui_core::screens::settings::{
    ConfigPanel, Edit, Ev as SettingsEv, Experimental, Feature, FeaturesStatus,
    Intent as SettingsIntent, MEMORIES_GENERATE, MEMORIES_USE, Memories, Open, Section, SettingId,
    Setup, SetupKind, USE_THEME_COLORS, Value, Written,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use pretty_assertions::assert_eq;

const CONFIG: &str = include_str!("../../src/sheets/settings/fixtures/config_panel.jsonl");
const THEME: &str = include_str!("../../src/sheets/settings/fixtures/theme_picker.jsonl");
const THEME_ALL: &str = include_str!("../../src/sheets/settings/fixtures/theme_picker_all.jsonl");
const STATUS_LINE: &str = include_str!("../../src/sheets/settings/fixtures/status_line.jsonl");
const TITLE: &str = include_str!("../../src/sheets/settings/fixtures/title.jsonl");
const SKILLS: &str = include_str!("../../src/sheets/settings/fixtures/skills_long_names.jsonl");
const EXPERIMENTAL_DISCOVERY: &str =
    include_str!("../../src/sheets/settings/fixtures/experimental_discovery.jsonl");
const EXPERIMENTAL_TWO: &str =
    include_str!("../../src/sheets/settings/fixtures/experimental_two.jsonl");
const EXPERIMENTAL_FAILED: &str =
    include_str!("../../src/sheets/settings/fixtures/experimental_save_failed.jsonl");
const EXPERIMENTAL_UNAVAILABLE: &str =
    include_str!("../../src/sheets/settings/fixtures/experimental_unavailable.jsonl");
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
            memories: None,
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

/// `skills_toggle_view.rs`: `apply_filter` (`match_skill`: the display name
/// first, then the skill's own name; best score first, ties by display name),
/// `toggle_selected` (`AppEvent::SetSkillEnabled`) and `close`
/// (`AppEvent::ManageSkillsClosed`).
#[test]
fn the_skills_search_matches_the_display_name_then_the_skill_name_and_each_flip_is_recorded() {
    let mut sandbox = Sandbox::play(SKILLS).expect("plays");
    let skills = setup(&sandbox);
    assert_eq!(skills.kind, SetupKind::Skills);
    assert_eq!(skills.visible("").len(), 2);
    // "completion" is in the second display name only
    assert_eq!(skills.visible("completion"), vec![1]);
    // "polish:" is in no display name, but in both skill names
    assert_eq!(skills.visible("polish:").len(), 2);
    assert_eq!(skills.visible("zzz"), Vec::<usize>::new());
    // no preview, no order
    assert_eq!(skills.preview(), "");

    let id = "polish:superpowers-verification-before-completion".to_string();
    send(&mut sandbox, SettingsIntent::Toggle(id.clone()));
    send(
        &mut sandbox,
        SettingsIntent::Move {
            id: id.clone(),
            delta: -1,
        },
    );
    // Enter does not save a skills setup
    send(&mut sandbox, SettingsIntent::Save);
    assert!(setup(&sandbox).items[1].enabled);
    assert_eq!(
        setup(&sandbox).items[1].id,
        id,
        "the skills keep their order"
    );
    send(&mut sandbox, SettingsIntent::Toggle(id.clone()));
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens().settings.open.is_none());
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![
            Edit::SkillToggled(id.clone(), true),
            Edit::SkillToggled(id, false),
            Edit::SkillsClosed,
        ]
    );
}

fn experimental(sandbox: &Sandbox) -> Experimental {
    match &sandbox.app().screens().settings.open {
        Some(Open::Experimental(dialog)) => dialog.clone(),
        other => panic!("experimental features expected, got {other:?}"),
    }
}

fn answer(sandbox: &mut Sandbox, ev: SettingsEv) {
    sandbox.send(AppEv::Screen(ScreenEv::Settings(ev)));
}

fn flags(sandbox: &Sandbox) -> Vec<(String, bool)> {
    experimental(sandbox)
        .features
        .iter()
        .map(|feature| (feature.key.clone(), feature.enabled))
        .collect()
}

/// `experimental_features_view.rs`: discovery keeps the server's order, a
/// toggle flips a writable row, Enter sends what differs from the config
/// and holds the dialog until the answer (`save_changes`, `pre_draw_tick`).
#[test]
fn experimental_features_discovered_toggled_and_saved() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    let dialog = experimental(&sandbox);
    assert_eq!(dialog.status, FeaturesStatus::Quiet);
    assert!(!dialog.loading);
    let keys: Vec<_> = dialog.features.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        ["network_proxy", "future_feature", "prevent_idle_sleep"]
    );

    // nothing changed: Enter closes without a write
    let mut idle = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    send(&mut idle, SettingsIntent::Save);
    assert!(idle.app().screens().settings.open.is_none());
    assert_eq!(idle.app().screens().settings.edits, vec![]);

    for key in ["future_feature", "network_proxy", "prevent_idle_sleep"] {
        send(&mut sandbox, SettingsIntent::Toggle(key.to_string()));
    }
    send(&mut sandbox, SettingsIntent::Save);
    let dialog = experimental(&sandbox);
    assert!(dialog.saving);
    assert_eq!(dialog.status, FeaturesStatus::Saving);
    let updates = vec![
        ("network_proxy".to_string(), false),
        ("future_feature".to_string(), false),
        ("prevent_idle_sleep".to_string(), false),
    ];
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::FeaturesSave(updates.clone())]
    );
    // a toggle while the write is on its way changes nothing
    send(&mut sandbox, SettingsIntent::Toggle("network_proxy".into()));
    assert_eq!(flags(&sandbox), updates);

    // the write was confirmed: the dialog closes
    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 0,
            result: Ok(Written {
                features: updates,
                warning: None,
            }),
        },
    );
    assert!(sandbox.app().screens().settings.open.is_none());
}

#[test]
fn a_failed_write_keeps_the_choices_and_enter_sends_them_again_while_esc_closes() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    send(&mut sandbox, SettingsIntent::Toggle("network_proxy".into()));
    send(&mut sandbox, SettingsIntent::Save);
    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 0,
            result: Err("Persistent write rejection".into()),
        },
    );
    let dialog = experimental(&sandbox);
    assert!(!dialog.saving);
    assert_eq!(
        dialog.status,
        FeaturesStatus::Message("Persistent write rejection".into())
    );
    assert_eq!(dialog.unconfirmed, ["network_proxy"]);
    // putting the value back still corrects a write that may have landed
    send(&mut sandbox, SettingsIntent::Toggle("network_proxy".into()));
    send(&mut sandbox, SettingsIntent::Save);
    assert_eq!(
        sandbox.app().screens().settings.edits.last(),
        Some(&Edit::FeaturesSave(vec![("network_proxy".into(), true)]))
    );
    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 0,
            result: Err("again".into()),
        },
    );
    // Esc after an unconfirmed write closes without another write
    let writes = sandbox.app().screens().settings.edits.len();
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens().settings.open.is_none());
    assert_eq!(sandbox.app().screens().settings.edits.len(), writes);

    // the seeded failure (the oracle's snapshot state)
    let failed = Sandbox::play(EXPERIMENTAL_FAILED).expect("plays");
    let dialog = experimental(&failed);
    assert_eq!(dialog.unconfirmed.len(), 3);
    assert_eq!(
        dialog.status,
        FeaturesStatus::Message("Saved but readback failed".into())
    );
}

#[test]
fn a_warning_keeps_the_dialog_open_and_esc_saves_what_changed() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_TWO).expect("plays");
    send(&mut sandbox, SettingsIntent::Toggle("last".into()));
    send(&mut sandbox, SettingsIntent::Save);
    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 0,
            result: Ok(Written {
                features: vec![("first".into(), true), ("last".into(), true)],
                warning: Some("Restart to apply".into()),
            }),
        },
    );
    let dialog = experimental(&sandbox);
    assert_eq!(
        dialog.status,
        FeaturesStatus::Message("Restart to apply".into())
    );
    assert!(dialog.unconfirmed.is_empty());
    assert_eq!(dialog.initial, [true, true]);
    // Esc with a change writes it and closes
    send(&mut sandbox, SettingsIntent::Toggle("first".into()));
    send(&mut sandbox, SettingsIntent::Close);
    assert!(sandbox.app().screens().settings.open.is_none());
    assert_eq!(
        sandbox.app().screens().settings.edits.last(),
        Some(&Edit::FeaturesSave(vec![("first".into(), false)]))
    );
}

#[test]
fn answers_of_another_open_and_read_only_rows_change_nothing() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_UNAVAILABLE).expect("plays");
    assert_eq!(experimental(&sandbox).status, FeaturesStatus::Unavailable);
    // the catalogue of an earlier open of the dialog is stale: the open
    // that waits for its own keeps waiting
    answer(
        &mut sandbox,
        SettingsEv::Opened(Open::Experimental(Experimental::new(vec![], true, 8))),
    );
    answer(
        &mut sandbox,
        SettingsEv::FeaturesDiscovered {
            generation: 7,
            features: Some(vec![]),
        },
    );
    let waiting = experimental(&sandbox);
    assert!(waiting.loading);
    assert_eq!(waiting.status, FeaturesStatus::Loading);
    answer(
        &mut sandbox,
        SettingsEv::Opened(Open::Experimental(Experimental::new(vec![], false, 0))),
    );
    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 7,
            result: Ok(Written {
                features: vec![],
                warning: None,
            }),
        },
    );
    assert!(sandbox.app().screens().settings.open.is_some());

    let read_only = Feature {
        key: "pinned".into(),
        name: "Pinned".into(),
        description: String::new(),
        enabled: true,
        writable: false,
    };
    let mut dialog = Experimental::new(vec![read_only], false, 0);
    dialog.status = FeaturesStatus::Quiet;
    let mut sandbox = Sandbox::play(EXPERIMENTAL_TWO).expect("plays");
    answer(&mut sandbox, SettingsEv::Opened(Open::Experimental(dialog)));
    send(&mut sandbox, SettingsIntent::Toggle("pinned".into()));
    assert!(experimental(&sandbox).features[0].enabled);
    send(&mut sandbox, SettingsIntent::Save);
    assert!(sandbox.app().screens().settings.open.is_none());
    assert_eq!(sandbox.app().screens().settings.edits, vec![]);
}

// ---- the memories dialog (`bottom_pane/memories_settings_view.rs`,
// `chatwidget/tests/popups_and_settings.rs:3323..3344`)

const MEMORIES: &str = include_str!("../../src/sheets/settings/fixtures/memories.jsonl");

fn memories(sandbox: &Sandbox) -> Memories {
    match &sandbox.app().screens().settings.open {
        Some(Open::Memories(dialog)) => dialog.clone(),
        other => panic!("memories dialog expected, got {other:?}"),
    }
}

/// Space flips a switch (`toggle_selected`); Enter on one sends both switches
/// as they stand and closes (`UpdateMemorySettings`). The confirmation's
/// intents do nothing before it is asked for.
#[test]
fn memories_toggles_flip_and_enter_saves_both_switches() {
    let mut sandbox = Sandbox::play(MEMORIES).expect("plays");
    assert_eq!(
        memories(&sandbox),
        Memories {
            use_memories: true,
            generate_memories: false,
            confirming: false
        }
    );
    send(&mut sandbox, SettingsIntent::Reset);
    send(&mut sandbox, SettingsIntent::Toggle("unknown".to_string()));
    send(
        &mut sandbox,
        SettingsIntent::Toggle(MEMORIES_GENERATE.to_string()),
    );
    send(
        &mut sandbox,
        SettingsIntent::Toggle(MEMORIES_USE.to_string()),
    );
    assert_eq!(
        memories(&sandbox),
        Memories {
            use_memories: false,
            generate_memories: true,
            confirming: false
        }
    );
    assert_eq!(sandbox.app().screens().settings.edits, vec![]);
    send(&mut sandbox, SettingsIntent::Save);
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::MemoriesSaved {
            use_memories: false,
            generate_memories: true
        }]
    );
    assert_eq!(sandbox.app().screens().settings.open, None);
}

/// Enter on the reset row opens the confirmation, which takes neither
/// toggles nor a save; Esc or "Go back" (`Close`) returns to the switches and
/// closes only from there; "Reset all memories" sends `ResetMemories` and
/// closes (`save`, `cancel`, `close_reset_confirmation`).
#[test]
fn memories_reset_is_confirmed_then_sent_and_close_goes_back_first() {
    let mut sandbox = Sandbox::play(MEMORIES).expect("plays");
    send(&mut sandbox, SettingsIntent::ConfirmReset);
    assert!(memories(&sandbox).confirming);
    send(
        &mut sandbox,
        SettingsIntent::Toggle(MEMORIES_USE.to_string()),
    );
    send(&mut sandbox, SettingsIntent::Save);
    assert!(memories(&sandbox).use_memories);
    assert!(memories(&sandbox).confirming);
    send(&mut sandbox, SettingsIntent::Close);
    assert!(!memories(&sandbox).confirming);
    send(&mut sandbox, SettingsIntent::Reset);
    assert!(sandbox.app().screens().settings.open.is_some());
    send(&mut sandbox, SettingsIntent::ConfirmReset);
    send(&mut sandbox, SettingsIntent::Reset);
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::MemoriesReset]
    );
    assert_eq!(sandbox.app().screens().settings.open, None);
    // Esc from the switches closes with nothing sent
    let mut sandbox = Sandbox::play(MEMORIES).expect("plays");
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(sandbox.app().screens().settings.open, None);
    assert_eq!(sandbox.app().screens().settings.edits, vec![]);
}

/// `/memories` builds the dialog from the facts (`open_memories_popup`: the
/// switches of `config.memories` when the memory tool is on).
#[test]
fn slash_memories_opens_from_the_facts_when_the_memory_tool_is_on() {
    use codex_tui_core::screens::settings::{Dialog, Facts, MemoryFacts};
    let mut sandbox = Sandbox::play(MEMORIES).expect("plays");
    send(&mut sandbox, SettingsIntent::Close);
    send(&mut sandbox, SettingsIntent::Open(Dialog::Memories));
    assert_eq!(sandbox.app().screens().settings.open, None);
    answer(
        &mut sandbox,
        SettingsEv::Facts(Facts {
            memories: Some(MemoryFacts {
                use_memories: false,
                generate_memories: true,
            }),
            ..Facts::default()
        }),
    );
    send(&mut sandbox, SettingsIntent::Open(Dialog::Memories));
    assert_eq!(
        memories(&sandbox),
        Memories {
            use_memories: false,
            generate_memories: true,
            confirming: false
        }
    );
}

// ---- the hooks browser (`bottom_pane/hooks_browser_view.rs`: the state
// tests at 1561..1700)

const HOOKS_EVENTS: &str = include_str!("../../src/sheets/settings/fixtures/hooks_events.jsonl");
const HOOKS_REVIEW_MIXED: &str =
    include_str!("../../src/sheets/settings/fixtures/hooks_events_review_mixed.jsonl");

fn browser(sandbox: &Sandbox) -> Hooks {
    match &sandbox.app().screens().settings.open {
        Some(Open::Hooks(browser)) => browser.clone(),
        other => panic!("hooks browser expected, got {other:?}"),
    }
}

fn hook(key: &str, event: &str, trust: &str) -> Hook {
    serde_json::from_value(serde_json::json!({
        "key": key, "event": event, "handler": "command", "command": "x",
        "source": "user", "enabled": false, "trust": trust,
    }))
    .expect("a hook")
}

/// The events page counts what is installed, what is active (on, and managed
/// or trusted) and what waits for a review, and starts on the first event
/// with a hook to review (`HooksBrowserView::new`'s `initial_event`).
#[test]
fn hooks_events_count_installed_active_and_review_and_start_on_the_first_review() {
    let sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    let browser = browser(&sandbox);
    let row = |event| {
        let row = browser.rows()[HookEvent::index(event)];
        (row.installed, row.active, row.needs_review)
    };
    assert_eq!(row(HookEvent::PreToolUse), (2, 1, 1));
    assert_eq!(row(HookEvent::Stop), (1, 0, 1));
    assert_eq!(row(HookEvent::PostToolUse), (0, 0, 0));
    assert_eq!(browser.review_total(), 2);
    assert_eq!(browser.initial_event(), 0);

    let late = Hooks::new(
        vec![
            hook("a", "pre_tool_use", "trusted"),
            hook("b", "stop", "modified"),
        ],
        vec![],
        vec![],
    );
    assert_eq!(late.initial_event(), HookEvent::Stop.index());
    assert_eq!(Hooks::new(vec![], vec![], vec![]).initial_event(), 0);
}

/// An event opens its page once; Esc goes back to the events and closes only
/// from there; Ctrl+C (`HookClose`) closes from either page.
#[test]
fn hooks_an_event_opens_its_page_esc_goes_back_then_closes_and_ctrl_c_closes_from_either() {
    let mut sandbox = Sandbox::play(HOOKS_EVENTS).expect("plays");
    assert_eq!(browser(&sandbox).page, None);
    send(
        &mut sandbox,
        SettingsIntent::HookOpen(HookEvent::PreToolUse),
    );
    assert_eq!(browser(&sandbox).page, Some(HookEvent::PreToolUse));
    // Another event while a page is open: not taken.
    send(&mut sandbox, SettingsIntent::HookOpen(HookEvent::Stop));
    assert_eq!(browser(&sandbox).page, Some(HookEvent::PreToolUse));
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(browser(&sandbox).page, None);
    send(&mut sandbox, SettingsIntent::Close);
    assert_eq!(sandbox.app().screens().settings.open, None);

    let mut sandbox = Sandbox::play(HOOKS_EVENTS).expect("plays");
    send(&mut sandbox, SettingsIntent::HookClose);
    assert_eq!(sandbox.app().screens().settings.open, None);
    let mut sandbox = Sandbox::play(HOOKS_EVENTS).expect("plays");
    send(&mut sandbox, SettingsIntent::HookOpen(HookEvent::Stop));
    send(&mut sandbox, SettingsIntent::HookClose);
    assert_eq!(sandbox.app().screens().settings.open, None);
}

/// A switch is recorded (`SetHookEnabled`); managed hooks and those waiting
/// for a review stay as they are (`toggle_selected_hook`), as an unknown key.
#[test]
fn hooks_toggle_flips_a_user_hook_and_leaves_managed_and_review_needed_ones() {
    let mut sandbox = Sandbox::play(HOOKS_EVENTS).expect("plays");
    let enabled = |sandbox: &Sandbox, key: &str| {
        browser(sandbox)
            .hooks
            .iter()
            .find(|hook| hook.key == key)
            .map(|hook| hook.enabled)
    };
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("path:user-config".to_string()),
    );
    assert_eq!(enabled(&sandbox, "path:user-config"), Some(true));
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("path:managed".to_string()),
    );
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("unknown".to_string()),
    );
    assert_eq!(enabled(&sandbox, "path:managed"), Some(true));
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::HookEnabled {
            key: "path:user-config".to_string(),
            enabled: true
        }]
    );
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("path:user-config".to_string()),
    );
    assert_eq!(enabled(&sandbox, "path:user-config"), Some(false));

    let mut sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("path:untrusted".to_string()),
    );
    send(
        &mut sandbox,
        SettingsIntent::HookToggle("path:modified".to_string()),
    );
    assert_eq!(sandbox.app().screens().settings.edits, vec![]);
}

/// `t` trusts the hook waiting for a review and leaves its enablement; a
/// trusted one records nothing (`trust_selected_hook`).
#[test]
fn hooks_trust_vouches_for_one_hook_and_keeps_its_enablement() {
    let mut sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::HookTrust("path:trusted".to_string()),
    );
    assert_eq!(sandbox.app().screens().settings.edits, vec![]);
    send(
        &mut sandbox,
        SettingsIntent::HookTrust("path:untrusted".to_string()),
    );
    let after = browser(&sandbox);
    let untrusted = after
        .hooks
        .iter()
        .find(|hook| hook.key == "path:untrusted")
        .expect("the hook");
    assert_eq!(
        (untrusted.trust, untrusted.enabled),
        (HookTrust::Trusted, false)
    );
    assert_eq!(after.review_total(), 1);
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::HookTrusted {
            key: "path:untrusted".to_string(),
            current_hash: "sha256:current".to_string()
        }]
    );
}

/// `t` on the events page trusts every hook waiting for a review, in one
/// edit; with none to review it records nothing (`trust_all_hooks`).
#[test]
fn hooks_trust_all_vouches_for_every_review_needed_hook_in_one_edit() {
    let mut sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    send(&mut sandbox, SettingsIntent::HookTrustAll);
    assert_eq!(browser(&sandbox).review_total(), 0);
    assert_eq!(
        sandbox.app().screens().settings.edits,
        vec![Edit::HooksTrusted(vec![
            ("path:untrusted".to_string(), "sha256:current".to_string()),
            ("path:modified".to_string(), "sha256:current".to_string()),
        ])]
    );
    send(&mut sandbox, SettingsIntent::HookTrustAll);
    assert_eq!(sandbox.app().screens().settings.edits.len(), 1);
}

/// What the dialogs write (`config_update.rs`, `app/event_dispatch.rs`): the key paths and
/// values of each edit that is a write of the config file.
mod writes {
    use codex_tui_core::screens::settings::persist::{Subject, write_of};
    use codex_tui_core::screens::settings::{Edit, Model, SetupKind};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn keys(edit: &Edit) -> Vec<(String, serde_json::Value)> {
        write_of(edit)
            .expect("a write")
            .edits
            .into_iter()
            .map(|edit| (edit.key_path, edit.value))
            .collect()
    }

    #[test]
    fn the_status_line_writes_its_items_and_the_colour_switch() {
        let edit = Edit::SetupSaved {
            kind: SetupKind::StatusLine,
            items: vec!["model".into(), "git-branch".into()],
            use_theme_colors: false,
        };
        assert_eq!(
            keys(&edit),
            vec![
                ("tui.status_line".to_owned(), json!(["model", "git-branch"])),
                ("tui.status_line_use_colors".to_owned(), json!(false)),
            ]
        );
        assert_eq!(
            write_of(&edit).map(|write| write.subject),
            Some(Subject::StatusLine {
                items: vec!["model".into(), "git-branch".into()],
                use_theme_colors: false,
            })
        );
    }

    #[test]
    fn an_empty_title_is_written_as_an_empty_list() {
        let edit = Edit::SetupSaved {
            kind: SetupKind::Title,
            items: Vec::new(),
            use_theme_colors: true,
        };
        assert_eq!(
            keys(&edit),
            vec![("tui.terminal_title".to_owned(), json!([]))]
        );
    }

    #[test]
    fn the_theme_and_the_memory_switches_write_their_keys() {
        assert_eq!(
            keys(&Edit::ThemeSelected("nord".into())),
            vec![("tui.theme".to_owned(), json!("nord"))]
        );
        assert_eq!(
            keys(&Edit::MemoriesSaved {
                use_memories: true,
                generate_memories: false,
            }),
            vec![
                ("memories.use_memories".to_owned(), json!(true)),
                ("memories.generate_memories".to_owned(), json!(false)),
            ]
        );
    }

    #[test]
    fn edits_with_a_request_of_their_own_are_no_config_write() {
        for edit in [
            Edit::TestNotification,
            Edit::MemoriesReset,
            Edit::SetupCancelled(SetupKind::Title),
            Edit::KeymapPicked("composer.submit".into()),
        ] {
            assert_eq!(write_of(&edit), None, "{edit:?}");
        }
    }

    #[test]
    fn the_app_takes_each_edit_once() {
        let mut model = Model::default();
        model.edits.push(Edit::TestNotification);
        model.edits.push(Edit::ThemeSelected("nord".into()));
        assert_eq!(model.take_edits().len(), 2);
        assert_eq!(model.take_edits(), Vec::new());
        model.edits.push(Edit::MemoriesReset);
        assert_eq!(model.take_edits(), vec![Edit::MemoriesReset]);
    }
}
