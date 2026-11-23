#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S15 (settings dialogs), one fact each: what the dialogs are offered (the
//! configuration panel `config_panel/config_panel_tests.rs`, the syntax theme picker
//! `theme_picker_model_tests.rs`, the status-line and title setups
//! `bottom_pane/status_line_setup.rs`, `title_setup.rs`, the keymap picker `keymap_setup.rs`:
//! `picker_content_snapshot`, the experimental features, the memories dialog and the hooks
//! browser), what the model keeps of them (the facts, the theme in force, the requests the
//! dialogs left), and the domain rules of the catalogues (`visible`, `preview`, `updates`,
//! `toggle`, `trust`).
//!
//! The cursor, the group tab, the search text, the checks, the order of a setup and the page of
//! the hooks browser are the sheets' own (ADR-013): `src/sheets/settings/draft.rs` and the
//! screen tests in `tests/sheets/settings.rs`.

use std::collections::BTreeSet;

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::settings::hooks::{Hook, HookEvent, HookTrust, Hooks};
use codex_tui_core::screens::settings::persist::Subject;
use codex_tui_core::screens::settings::{
    ConfigPanel, Dialog, Edit, Ev as SettingsEv, Experimental, Facts, Feature, FeaturesStatus,
    Intent as SettingsIntent, Memories, MemoryFacts, Offer, Section, SettingId, Setup, SetupKind,
    ThemePicker, USE_THEME_COLORS, Which, Written,
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
const MEMORIES: &str = include_str!("../../src/sheets/settings/fixtures/memories.jsonl");
const HOOKS_EVENTS: &str = include_str!("../../src/sheets/settings/fixtures/hooks_events.jsonl");
const HOOKS_REVIEW_MIXED: &str =
    include_str!("../../src/sheets/settings/fixtures/hooks_events_review_mixed.jsonl");
/// A session with nothing offered: the model of an app that has not opened a dialog yet.
const EMPTY: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn send(sandbox: &mut Sandbox, intent: SettingsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Settings(
        intent,
    ))));
}

fn answer(sandbox: &mut Sandbox, ev: SettingsEv) {
    sandbox.send(AppEv::Screen(ScreenEv::Settings(ev)));
}

fn model(sandbox: &Sandbox) -> &codex_tui_core::screens::settings::Model {
    &sandbox.app().screens().settings
}

fn offer(sandbox: &Sandbox, which: Which) -> Offer {
    model(sandbox)
        .offers
        .get(&which)
        .cloned()
        .unwrap_or_else(|| panic!("{which:?} is offered: {:?}", model(sandbox).offers.keys()))
}

fn panel(sandbox: &Sandbox) -> ConfigPanel {
    match offer(sandbox, Which::Config) {
        Offer::Config(panel) => panel,
        other => panic!("config panel expected, got {other:?}"),
    }
}

fn picker(sandbox: &Sandbox) -> ThemePicker {
    match offer(sandbox, Which::Theme) {
        Offer::Theme(picker) => picker,
        other => panic!("theme picker expected, got {other:?}"),
    }
}

fn setup(sandbox: &Sandbox, which: Which) -> Setup {
    match offer(sandbox, which) {
        Offer::Setup(setup) => setup,
        other => panic!("setup expected, got {other:?}"),
    }
}

fn experimental(sandbox: &Sandbox) -> Experimental {
    match offer(sandbox, Which::Experimental) {
        Offer::Experimental(dialog) => dialog,
        other => panic!("experimental features expected, got {other:?}"),
    }
}

fn memories(sandbox: &Sandbox) -> Memories {
    match offer(sandbox, Which::Memories) {
        Offer::Memories(dialog) => dialog,
        other => panic!("memories dialog expected, got {other:?}"),
    }
}

fn browser(sandbox: &Sandbox) -> Hooks {
    match offer(sandbox, Which::Hooks) {
        Offer::Hooks(browser) => browser,
        other => panic!("hooks browser expected, got {other:?}"),
    }
}

fn labels(panel: &ConfigPanel, section: Section, query: &str) -> Vec<String> {
    panel
        .visible(section, query)
        .into_iter()
        .map(|index| panel.rows[index].label.clone())
        .collect()
}

// ---- what a seed or an event offers: the catalogue, and the ask for it

/// A dialog is offered together with the ask for it; the surface takes the ask once, the
/// catalogue stays (`Intent::Taken`).
#[test]
fn a_seeded_dialog_is_offered_and_asked_for_once() {
    for (script, which) in [
        (CONFIG, Which::Config),
        (THEME, Which::Theme),
        (STATUS_LINE, Which::StatusLine),
        (TITLE, Which::Title),
        (SKILLS, Which::Skills),
        (KEYMAP, Which::Keymap),
        (EXPERIMENTAL_DISCOVERY, Which::Experimental),
        (MEMORIES, Which::Memories),
        (HOOKS_EVENTS, Which::Hooks),
    ] {
        let mut sandbox = Sandbox::play(script).expect("plays");
        assert_eq!(model(&sandbox).requested, Some(which), "{which:?}");
        assert_eq!(offer(&sandbox, which).which(), which);
        send(&mut sandbox, SettingsIntent::Taken);
        assert_eq!(model(&sandbox).requested, None, "{which:?}");
        assert_eq!(offer(&sandbox, which).which(), which, "the catalogue stays");
    }
}

#[test]
fn bad_seeds_are_refused() {
    assert!(Sandbox::play(r#"{"step":"screen","dir":"settings","ev":{"kind":"nope"}}"#).is_err());
}

/// What a dialog asks of the app is a record, in order, that the app takes once; the model
/// changes nothing else for the plain ones.
#[test]
fn requests_are_recorded_in_order_and_the_app_takes_each_once() {
    let mut sandbox = Sandbox::play(CONFIG).expect("plays");
    let requests = vec![
        Edit::Toggled(SettingId::Tips, true),
        Edit::Editor(SettingId::Effects),
        Edit::TestNotification,
        Edit::KeymapPicked("Open Transcript".into()),
    ];
    for edit in &requests {
        send(&mut sandbox, SettingsIntent::Request(edit.clone()));
    }
    assert_eq!(model(&sandbox).edits, requests);
    assert_eq!(
        panel(&sandbox)
            .row(SettingId::Tips)
            .map(|row| row.value.clone()),
        Sandbox::play(CONFIG).ok().and_then(|fresh| panel(&fresh)
            .row(SettingId::Tips)
            .map(|row| row.value.clone())),
        "the catalogue is the host's: a request does not change it"
    );
}

// ---- the configuration panel (`config_panel/config_panel_tests.rs`)

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

// ---- the syntax theme picker (`theme_picker_model_tests.rs`)

const THEMES: [&str; 6] = ["ada", "babbage", "curie", "cushman", "dali", "davinci"];

/// The picker is offered on the theme in force; the preview changes the theme in force and
/// Enter's request keeps it (`Edit::ThemeSelected`).
#[test]
fn the_picker_opens_on_the_theme_in_force_and_the_preview_and_the_pick_change_it() {
    for (index, name) in THEMES.into_iter().enumerate() {
        let script = THEME.replace("\"current\":\"ada\"", &format!("\"current\":\"{name}\""));
        let mut sandbox = Sandbox::play(&script).expect("plays");
        let picker = picker(&sandbox);
        assert_eq!(picker.initial(), index);
        assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some(name));
        // A cursor on another theme previews it, and back.
        let other = THEMES[(index + 1) % THEMES.len()];
        send(
            &mut sandbox,
            SettingsIntent::SyntaxTheme(Some(other.into())),
        );
        assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some(other));
        send(&mut sandbox, SettingsIntent::SyntaxTheme(Some(name.into())));
        assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some(name));
        // Enter selects it.
        send(
            &mut sandbox,
            SettingsIntent::Request(Edit::ThemeSelected(name.into())),
        );
        assert_eq!(
            model(&sandbox).edits,
            vec![Edit::ThemeSelected(name.into())]
        );
        assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some(name));
    }
}

/// Esc gives the picker's own original back; none is the config's theme.
#[test]
fn cancelling_the_theme_picker_puts_the_theme_in_force_back() {
    let mut sandbox = Sandbox::play(THEME).expect("plays");
    let original = picker(&sandbox).original;
    assert_eq!(original.as_deref(), Some("ada"));
    send(
        &mut sandbox,
        SettingsIntent::SyntaxTheme(Some("curie".into())),
    );
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("curie"));
    send(&mut sandbox, SettingsIntent::SyntaxTheme(original));
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("ada"));
    assert!(model(&sandbox).edits.is_empty());

    answer(
        &mut sandbox,
        SettingsEv::Facts(Facts {
            syntax_theme: Some("nord".into()),
            ..Facts::default()
        }),
    );
    send(
        &mut sandbox,
        SettingsIntent::SyntaxTheme(Some("curie".into())),
    );
    send(&mut sandbox, SettingsIntent::SyntaxTheme(None));
    assert_eq!(
        model(&sandbox).syntax_theme.as_deref(),
        Some("nord"),
        "none is the theme the config names"
    );
}

/// `theme_picker.rs` `SelectionItem::search_value`: the typed text matches a
/// name in any case; custom files and the directory come from the seed.
#[test]
fn the_theme_picker_filters_by_name_and_knows_the_custom_files() {
    let sandbox = Sandbox::play(THEME_ALL).expect("play");
    let picker = picker(&sandbox);
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
    let mut sandbox = Sandbox::play(EMPTY).expect("plays");
    answer(
        &mut sandbox,
        SettingsEv::Facts(Facts {
            syntax_theme: Some("Zebra".into()),
            custom_themes: vec!["Zebra".into(), "my-night".into(), "nord".into()],
            themes_dir: Some("~/.codex/themes".into()),
            memories: None,
        }),
    );
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("Zebra"));
    send(&mut sandbox, SettingsIntent::Open(Dialog::Theme));
    assert_eq!(model(&sandbox).requested, Some(Which::Theme));
    let picker = picker(&sandbox);
    assert_eq!(picker.themes.len(), 34, "32 bundled, 2 new custom");
    assert_eq!(
        picker.custom,
        vec!["Zebra".to_string(), "my-night".to_string()]
    );
    assert_eq!(picker.themes.first().map(String::as_str), Some("1337"));
    let at = |name: &str| picker.themes.iter().position(|theme| theme == name);
    assert!(at("monokai-extended-origin") < at("my-night"));
    assert!(at("my-night") < at("nord"));
    assert!(at("two-dark") < at("Zebra") && at("Zebra") < at("zenburn"));
    assert_eq!(picker.initial(), at("Zebra").unwrap_or(usize::MAX));
    assert_eq!(picker.themes_dir.as_deref(), Some("~/.codex/themes"));
    // facts that arrive while the picker previews do not undo the preview
    send(
        &mut sandbox,
        SettingsIntent::SyntaxTheme(Some("nord".into())),
    );
    answer(&mut sandbox, SettingsEv::Facts(Facts::default()));
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("nord"));
}

/// The config has what a dialog saved (`Saved`): the facts follow; a refused theme write gives
/// the theme of the config back (`restore_runtime_theme_from_config`).
#[test]
fn a_saved_theme_or_switches_are_facts_and_a_refused_theme_restores_the_config_s() {
    let mut sandbox = Sandbox::play(EMPTY).expect("plays");
    answer(
        &mut sandbox,
        SettingsEv::Facts(Facts {
            syntax_theme: Some("ada".into()),
            ..Facts::default()
        }),
    );
    send(
        &mut sandbox,
        SettingsIntent::Request(Edit::ThemeSelected("nord".into())),
    );
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("nord"));
    answer(
        &mut sandbox,
        SettingsEv::SaveFailed(Subject::Theme("nord".into())),
    );
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("ada"));
    send(
        &mut sandbox,
        SettingsIntent::Request(Edit::ThemeSelected("nord".into())),
    );
    answer(
        &mut sandbox,
        SettingsEv::Saved(Subject::Theme("nord".into())),
    );
    assert_eq!(model(&sandbox).facts.syntax_theme.as_deref(), Some("nord"));
    assert_eq!(model(&sandbox).syntax_theme.as_deref(), Some("nord"));

    let switches = MemoryFacts {
        use_memories: true,
        generate_memories: true,
    };
    answer(&mut sandbox, SettingsEv::Saved(Subject::Memories(switches)));
    assert_eq!(model(&sandbox).facts.memories, Some(switches));
}

// ---- the status-line and title setups (`status_line_setup.rs`, `title_setup.rs`)

fn ids(sandbox: &Sandbox, which: Which) -> Vec<(String, bool)> {
    setup(sandbox, which)
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
    assert_eq!(ids(&sandbox, Which::StatusLine), want);
    assert_eq!(
        setup(&sandbox, Which::StatusLine).kind,
        SetupKind::StatusLine
    );
    assert_eq!(
        setup(&sandbox, Which::StatusLine).preview(),
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
    assert_eq!(
        setup(&sandbox, Which::StatusLine).preview(),
        "gpt-5 · feat/awesome-feature"
    );
}

#[test]
fn a_search_ranks_by_fuzzy_score_then_name() {
    let sandbox = Sandbox::play(STATUS_LINE).expect("plays");
    let setup = setup(&sandbox, Which::StatusLine);
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
    let title = setup(&sandbox, Which::Title);
    assert_eq!(title.kind, SetupKind::Title);
    let order: Vec<String> = ids(&sandbox, Which::Title)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        &order[..8],
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
    assert!(!title.items.iter().any(|item| item.id == USE_THEME_COLORS));
    assert_eq!(
        title.preview(),
        "[ ! ] Action Required | my-project | Working | thread title"
    );
}

/// `skills_toggle_view.rs`: `apply_filter` (`match_skill`: the display name
/// first, then the skill's own name; best score first, ties by display name).
#[test]
fn the_skills_search_matches_the_display_name_then_the_skill_name() {
    let sandbox = Sandbox::play(SKILLS).expect("plays");
    let skills = setup(&sandbox, Which::Skills);
    assert_eq!(skills.kind, SetupKind::Skills);
    assert_eq!(skills.visible("").len(), 2);
    // "completion" is in the second display name only
    assert_eq!(skills.visible("completion"), vec![1]);
    // "polish:" is in no display name, but in both skill names
    assert_eq!(skills.visible("polish:").len(), 2);
    assert_eq!(skills.visible("zzz"), Vec::<usize>::new());
    // no preview, no order
    assert_eq!(skills.preview(), "");
    assert!(skills.items.iter().all(|item| !item.orderable));
}

// ---- the keymap picker (`keymap_setup.rs`)

/// The text of the oracle's `keymap_picker_first_actions` snapshot.
fn keymap_dump(sandbox: &Sandbox) -> String {
    let picker = match offer(sandbox, Which::Keymap) {
        Offer::Keymap(picker) => picker,
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
fn the_keymap_picker_filters_a_tab() {
    let sandbox = Sandbox::play(KEYMAP).expect("plays");
    let Offer::Keymap(picker) = offer(&sandbox, Which::Keymap) else {
        panic!("keymap picker offered");
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
}

// ---- the experimental features (`experimental_features_view.rs`)

fn flipped(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

/// Discovery keeps the server's order and leaves the dialog quiet.
#[test]
fn experimental_features_discovered_keep_the_servers_order() {
    let sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    let dialog = experimental(&sandbox);
    assert_eq!(dialog.status, FeaturesStatus::Quiet);
    assert!(!dialog.loading);
    let keys: Vec<_> = dialog.features.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        ["network_proxy", "future_feature", "prevent_idle_sleep"]
    );
}

/// `save_changes`: what is sent is what the user flipped, writable rows only, in the dialog's
/// order; a key whose write did not confirm is sent again with the value the user has now.
#[test]
fn the_updates_are_the_flipped_writable_features_and_the_unconfirmed_ones() {
    let sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    let dialog = experimental(&sandbox);
    assert_eq!(dialog.updates(&flipped(&[])), vec![]);
    assert_eq!(
        dialog.updates(&flipped(&["prevent_idle_sleep", "network_proxy"])),
        vec![
            ("network_proxy".to_string(), false),
            ("prevent_idle_sleep".to_string(), false),
        ],
    );
    let feature = &dialog.features[0];
    assert!(dialog.enabled(feature, &flipped(&[])));
    assert!(!dialog.enabled(feature, &flipped(&["network_proxy"])));

    let read_only = Feature {
        key: "pinned".into(),
        name: "Pinned".into(),
        description: String::new(),
        enabled: true,
        writable: false,
    };
    let pinned = Experimental::new(vec![read_only], false, 0);
    assert_eq!(pinned.updates(&flipped(&["pinned"])), vec![]);

    // a write that did not confirm: putting the value back still corrects it
    let mut unconfirmed = dialog;
    unconfirmed.unconfirmed = vec!["network_proxy".into()];
    assert_eq!(
        unconfirmed.updates(&flipped(&[])),
        vec![("network_proxy".to_string(), true)]
    );
}

/// A write request holds the dialog until the answer (`pre_draw_tick`): it saves, remembers the
/// keys as unconfirmed, and a confirmed answer carries the values the config reads now.
#[test]
fn a_write_request_marks_the_dialog_saving_and_the_confirmation_settles_it() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    let updates = vec![
        ("network_proxy".to_string(), false),
        ("future_feature".to_string(), false),
        ("prevent_idle_sleep".to_string(), false),
    ];
    send(
        &mut sandbox,
        SettingsIntent::Request(Edit::FeaturesSave(updates.clone())),
    );
    let dialog = experimental(&sandbox);
    assert!(dialog.saving);
    assert_eq!(dialog.status, FeaturesStatus::Saving);
    assert_eq!(
        dialog.unconfirmed,
        ["network_proxy", "future_feature", "prevent_idle_sleep"]
    );
    assert_eq!(
        model(&sandbox).edits,
        vec![Edit::FeaturesSave(updates.clone())]
    );

    answer(
        &mut sandbox,
        SettingsEv::FeaturesSaved {
            generation: 0,
            result: Ok(Written {
                features: updates.clone(),
                warning: None,
            }),
        },
    );
    let dialog = experimental(&sandbox);
    assert!(!dialog.saving);
    assert_eq!(dialog.status, FeaturesStatus::Quiet);
    assert!(dialog.unconfirmed.is_empty());
    assert_eq!(dialog.confirmed, 1);
    let now: Vec<(String, bool)> = dialog
        .features
        .iter()
        .map(|feature| (feature.key.clone(), feature.enabled))
        .collect();
    assert_eq!(now, updates);
}

#[test]
fn a_failed_write_keeps_the_keys_unconfirmed_and_says_why() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_DISCOVERY).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::Request(Edit::FeaturesSave(vec![("network_proxy".into(), false)])),
    );
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
    assert_eq!(dialog.confirmed, 0);

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
fn a_warning_keeps_the_dialog_with_the_written_values() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_TWO).expect("plays");
    send(
        &mut sandbox,
        SettingsIntent::Request(Edit::FeaturesSave(vec![("last".into(), true)])),
    );
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
    assert_eq!(
        dialog
            .features
            .iter()
            .map(|feature| feature.enabled)
            .collect::<Vec<_>>(),
        [true, true]
    );
}

/// An answer of another open of the dialog is stale; the open that waits for its own keeps
/// waiting.
#[test]
fn answers_of_another_open_change_nothing() {
    let mut sandbox = Sandbox::play(EXPERIMENTAL_UNAVAILABLE).expect("plays");
    assert_eq!(experimental(&sandbox).status, FeaturesStatus::Unavailable);
    answer(
        &mut sandbox,
        SettingsEv::Offered(Offer::Experimental(Experimental::new(vec![], true, 8))),
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
        SettingsEv::Offered(Offer::Experimental(Experimental::new(vec![], false, 0))),
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
    assert_eq!(experimental(&sandbox).confirmed, 0);
    assert_eq!(experimental(&sandbox).status, FeaturesStatus::Quiet);
}

// ---- the memories dialog (`bottom_pane/memories_settings_view.rs`,
// `chatwidget/tests/popups_and_settings.rs:3323..3344`)

#[test]
fn the_memories_dialog_is_offered_with_the_switches_of_the_config() {
    let sandbox = Sandbox::play(MEMORIES).expect("plays");
    assert_eq!(
        memories(&sandbox),
        Memories {
            use_memories: true,
            generate_memories: false,
        }
    );
}

/// `/memories` builds the dialog from the facts (`open_memories_popup`: the
/// switches of `config.memories` when the memory tool is on).
#[test]
fn slash_memories_opens_from_the_facts_when_the_memory_tool_is_on() {
    let mut sandbox = Sandbox::play(EMPTY).expect("plays");
    send(&mut sandbox, SettingsIntent::Open(Dialog::Memories));
    assert_eq!(model(&sandbox).requested, None);
    assert!(model(&sandbox).offers.is_empty());
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
    assert_eq!(model(&sandbox).requested, Some(Which::Memories));
    assert_eq!(
        memories(&sandbox),
        Memories {
            use_memories: false,
            generate_memories: true,
        }
    );
}

// ---- the hooks browser (`bottom_pane/hooks_browser_view.rs`: the state
// tests at 1561..1700)

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

/// A switch flips a user hook; managed hooks and those waiting for a review stay as they are
/// (`toggle_selected_hook`), as an unknown key.
#[test]
fn hooks_toggle_flips_a_user_hook_and_leaves_managed_and_review_needed_ones() {
    let sandbox = Sandbox::play(HOOKS_EVENTS).expect("plays");
    let mut hooks = browser(&sandbox);
    assert_eq!(hooks.toggle("path:user-config"), Some(true));
    assert_eq!(hooks.toggle("path:managed"), None);
    assert_eq!(hooks.toggle("unknown"), None);
    assert_eq!(hooks.toggle("path:user-config"), Some(false));

    let sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    let mut hooks = browser(&sandbox);
    assert_eq!(hooks.toggle("path:untrusted"), None);
    assert_eq!(hooks.toggle("path:modified"), None);
}

/// `t` trusts the hook waiting for a review and leaves its enablement; a trusted one vouches
/// for nothing (`trust_selected_hook`).
#[test]
fn hooks_trust_vouches_for_one_hook_and_keeps_its_enablement() {
    let sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    let mut hooks = browser(&sandbox);
    assert_eq!(hooks.trust("path:trusted"), None);
    assert_eq!(
        hooks.trust("path:untrusted"),
        Some("sha256:current".to_string())
    );
    let untrusted = hooks
        .hooks
        .iter()
        .find(|hook| hook.key == "path:untrusted")
        .expect("the hook");
    assert_eq!(
        (untrusted.trust, untrusted.enabled),
        (HookTrust::Trusted, false)
    );
    assert_eq!(hooks.review_total(), 1);
}

/// `t` on the events page trusts every hook waiting for a review, in one
/// update; with none to review it vouches for nothing (`trust_all_hooks`).
#[test]
fn hooks_trust_all_vouches_for_every_review_needed_hook_in_one_update() {
    let sandbox = Sandbox::play(HOOKS_REVIEW_MIXED).expect("plays");
    let mut hooks = browser(&sandbox);
    assert_eq!(
        hooks.trust_all(),
        vec![
            ("path:untrusted".to_string(), "sha256:current".to_string()),
            ("path:modified".to_string(), "sha256:current".to_string()),
        ]
    );
    assert_eq!(hooks.review_total(), 0);
    assert_eq!(hooks.trust_all(), vec![]);
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
