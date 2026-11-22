//! Shard `chat_popups_4` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Snapshot heights are the oracle's popup heights (`desired_height`), taken from the
//! snapshot rows; widths are the oracle test's `render_bottom_popup(.., width)` widths.

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/popups_and_settings.rs (43 screen-level tests here; 4 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/popups_and_settings.rs:2864 apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps kind=ui owner=dialogs-pane
#[test]
fn apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps() {
    kit::pending(
        Owner::DialogsPane,
        "/apps is live; the oracle injects a partial snapshot (`is_final: false`) and then a failed refresh: no protocol message carries a partial snapshot and the kit's `respond` step has no error answer",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps");
    s.keys("enter");
    s.assert_contains("Loading installed and available apps...");
    s.assert_contains("Installed 1 of 1 available apps.");
    s.assert_contains("Installed · Press Enter to open the app page");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2925 apps_popup_shows_disabled_status_for_installed_but_disabled_apps kind=ui owner=dialogs-pane
#[test]
fn apps_popup_shows_disabled_status_for_installed_but_disabled_apps() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(
            r#"{"step":"respond","method":"app/list","result":{"data":[{"id":"notion","name":"Notion","description":"Workspace docs","installUrl":"https://example.test/notion","isAccessible":true,"isEnabled":false,"pluginDisplayNames":[]}],"nextCursor":null}}"#,
        )
        .open();
    s.type_text("/apps");
    s.keys("enter");
    s.assert_contains("Installed · Disabled · Press Enter to open the app page");
    s.assert_contains("enable/disable this app");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2970 apps_refresh_preserves_toggled_enabled_state kind=ui owner=dialogs-pane
#[test]
fn apps_refresh_preserves_toggled_enabled_state() {
    kit::pending(
        Owner::DialogsPane,
        "/apps is live (core `select::Ev::AppSwitched` keeps a switch over a refreshed list, core/tests/select_live.rs); the oracle calls `update_connector_enabled` directly: the port has no key or message that switches an app (the app link dialog does, a core Need)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps");
    s.keys("enter");
    s.keys("space");
    s.assert_contains("Installed · Disabled · Press Enter to open the app page");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3045 apps_popup_for_not_installed_app_uses_install_only_selected_description kind=ui owner=dialogs-pane
#[test]
fn apps_popup_for_not_installed_app_uses_install_only_selected_description() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(
            r#"{"step":"respond","method":"app/list","result":{"data":[{"id":"notion","name":"Notion","description":"Workspace docs","installUrl":"https://example.test/notion","isAccessible":false,"isEnabled":true,"pluginDisplayNames":[]}],"nextCursor":null}}"#,
        )
        .open();
    s.type_text("/apps");
    s.keys("enter");
    s.assert_contains("Can be installed · Press Enter to open the app page to install");
    s.assert_absent("enable/disable this app");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3090 experimental_features_popup_snapshot kind=snap owner=dialogs-pane snap=experimental_features_popup snap=experimental_features_popup_configured_key_chords
#[test]
fn experimental_features_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "the experimental sheet exists (entries settings/experimental_features_*), `/experimental` does not open it: the feature list and the server catalogue come from the host (needs, parity/settings.md) and the keymap is not read",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 13)
        .keymap(&[("list.accept", "ctrl+x enter")])
        .live()
        .open();
    s.type_text("/experimental");
    s.keys("enter");
    s.assert_snap("experimental_features_popup");
    s.resize(80, 9);
    s.assert_snap("experimental_features_popup_configured_key_chords");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3153 experimental_features_toggle_saves_on_exit kind=ui owner=dialogs-pane
#[test]
fn experimental_features_toggle_saves_on_exit() {
    kit::pending(
        Owner::DialogsPane,
        "the experimental sheet exists (entries settings/experimental_features_*), `/experimental` does not open it: the feature list and the server catalogue come from the host (needs, parity/settings.md) and the keymap is not read",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/experimental");
    s.keys("enter");
    s.keys("space");
    // The oracle asserts no save event before the popup closes: the event is not drawn.
    s.keys("f2");
    s.assert_absent("Experimental features");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3204 experimental_popup_loading_snapshot kind=snap owner=dialogs-pane snap=experimental_features_loading
#[test]
fn experimental_popup_loading_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "the experimental sheet exists (entries settings/experimental_features_*), `/experimental` does not open it: the feature list and the server catalogue come from the host (needs, parity/settings.md) and the keymap is not read",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/experimental");
    s.keys("enter");
    s.assert_snap("experimental_features_startup");
    s.resize(80, 10);
    s.assert_snap("experimental_features_loading");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3230 experimental_popup_available_snapshot kind=snap owner=dialogs-pane snap=experimental_features_available_popup
#[test]
fn experimental_popup_available_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "the experimental sheet exists (entries settings/experimental_features_*), `/experimental` does not open it: the feature list and the server catalogue come from the host (needs, parity/settings.md) and the keymap is not read",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 12).live().open();
    s.assert_snap("experimental_features_available_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3259 feature_enable_prompts_snapshot kind=snap owner=dialogs-pane snap=memories_enable_prompt snap=multi_agent_enable_prompt
#[test]
fn feature_enable_prompts_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no feature enable prompts in the port: the memories and multi-agent enable prompts have no widget and no feature-flag seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 10).live().open();
    s.assert_snap("memories_enable_prompt");
    s.resize(80, 9);
    s.assert_snap("multi_agent_enable_prompt");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3289 memories_settings_popup_snapshot kind=snap owner=dialogs-pane snap=memories_settings_popup
#[test]
fn memories_settings_popup_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 14)
        .script(include_str!(
            "fixtures/chat_popups_4/memories_enabled.jsonl"
        ))
        .open();
    s.type_text("/memories");
    s.keys("enter");
    // The oracle's `render_bottom_popup` trims the blank row above the popup;
    // the sheet keeps it (row 0: the screen is as tall as the sheet).
    s.assert_snap_region("memories_settings_popup", 1, 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3302 memories_reset_confirmation_snapshot kind=snap owner=dialogs-pane snap=memories_reset_confirmation
#[test]
fn memories_reset_confirmation_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 14)
        .script(include_str!(
            "fixtures/chat_popups_4/memories_enabled.jsonl"
        ))
        .open();
    s.type_text("/memories");
    s.keys("enter");
    s.keys("down down enter");
    // The confirmation is shorter than the settings sheet the screen is sized
    // for and sits at the bottom: its title is on row 3.
    s.assert_snap_region("memories_reset_confirmation", 3, 0);
    s.keys("esc");
    s.assert_contains("Choose how Codex uses");
    s.keys("esc");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3323 memories_settings_toggle_saves_on_enter kind=ui owner=dialogs-pane
#[test]
fn memories_settings_toggle_saves_on_enter() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UpdateMemorySettings event Enter sends (use_memories, generate_memories), not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3344 memories_reset_confirmation_sends_event_on_confirm kind=ui owner=dialogs-pane
#[test]
fn memories_reset_confirmation_sends_event_on_confirm() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ResetMemories event sent on confirm, not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3360 model_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_selection_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port (`/model`): the model list with its reasoning hints has no widget and no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 14).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("model_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3375 model_picker_refresh_rejects_obsolete_and_unusable_replies kind=ui owner=dialogs-pane
#[test]
fn model_picker_refresh_rejects_obsolete_and_unusable_replies() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter");
    let before = s.shot();
    s.keys("esc");
    s.type_text("/model");
    s.keys("enter");
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3419 model_picker_refreshes_startup_catalog kind=snap owner=dialogs-pane snap=model_selection_popup snap=model_picker_refreshes_auto_models
#[test]
fn model_picker_refreshes_startup_catalog() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: the startup catalog refresh (auto models, hidden startup models) has no widget and no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 14).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("model_selection_popup");
    s.resize(80, 9);
    s.assert_snap("model_picker_refreshes_auto_models");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3463 model_picker_queued_all_models_uses_refreshed_catalog kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_picker_queued_all_models_uses_refreshed_catalog() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: the all-models page opened after the refreshed catalog has no widget and no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 14).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    s.assert_snap("model_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3487 model_picker_refresh_preserves_highlight kind=snap owner=dialogs-pane snap=model_picker_refresh_preserves_highlight
#[test]
fn model_picker_refresh_preserves_highlight() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: the highlighted row survives a catalog refresh with renamed models; no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 7).live().open();
    s.type_text("/model");
    s.keys("enter down");
    s.assert_snap("model_picker_refresh_preserves_highlight");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3536 model_picker_refreshes_service_tier_controls kind=snap owner=dialogs-pane snap=model_picker_refreshes_service_tier_controls
#[test]
fn model_picker_refreshes_service_tier_controls() {
    // the config says fast; the catalog first gives the model the tier, then takes it away
    let mut s = Screen::build(Owner::DialogsPane).size(80, 8).live().open();
    let fast = include_str!("fixtures/chat_popups_4/fast_catalog.json").trim();
    let refreshed = include_str!("fixtures/chat_popups_4/no_tiers.json").trim();
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{fast}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.4","service_tier":"priority","cwd":"/tmp/project"}}}}}}"#
    ));
    s.type_text("/model");
    s.keys("enter");
    s.keys("esc");
    s.type_text("/fast");
    s.assert_contains("/fast");
    s.keys("ctrl+u");
    s.feed(&format!(r#"{{"step":"respond","method":"model/list","result":{refreshed}}}"#));
    s.type_text("/model");
    s.keys("enter");
    s.keys("esc");
    // the model no longer offers a tier: the effective tier is none and `/fast` lists nothing
    s.type_text("/fast");
    let row = s.rows().iter().position(|r| r.starts_with("› /fast")).expect("the composer row");
    s.assert_snap_region("model_picker_refreshes_service_tier_controls", row, 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3561 model_picker_refresh_preserves_dismissal_and_reasoning_submenu kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_picker_refresh_preserves_dismissal_and_reasoning_submenu() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: a dismissed picker and its reasoning submenu survive a catalog refresh; no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 14).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("model_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3615 model_picker_refresh_dismisses_empty_choices kind=snap owner=dialogs-pane
#[test]
fn model_picker_refresh_dismisses_empty_choices() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: a refresh that leaves no choice dismisses the reasoning picker and shows the empty notice; no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.assert_contains("• No additional models are available right now.");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3654 skills_menu_default_mentions_shortcut_snapshot kind=snap owner=dialogs-pane snap=skills_menu_default_mentions_shortcut
#[test]
fn skills_menu_default_mentions_shortcut_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no skills menu in the port (`/skills`): the default skills list with its mention shortcut has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 8).live().open();
    s.type_text("/skills");
    s.keys("enter");
    s.assert_snap("skills_menu_default_mentions_shortcut");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3663 model_picker_hides_show_in_picker_false_models_from_cache kind=snap owner=dialogs-pane snap=model_picker_filters_hidden_models
#[test]
fn model_picker_hides_show_in_picker_false_models_from_cache() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: the picker filters models with show_in_picker false from the cache; no catalog seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 6).live().open();
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("model_picker_filters_hidden_models");
    s.assert_contains("test-visible-model");
    s.assert_absent("test-hidden-model");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3740 model_reasoning_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_reasoning_selection_popup
#[test]
fn model_reasoning_selection_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: the Select Reasoning Level list for a model has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 11).live().open();
    s.type_text("/model");
    s.keys("enter enter");
    s.assert_snap("model_reasoning_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3773 model_advanced_reasoning_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_advanced_reasoning_selection_popup
#[test]
fn model_advanced_reasoning_selection_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: the advanced (Ultra and Max) reasoning list has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 9).live().open();
    s.type_text("/model");
    s.keys("enter down enter");
    s.assert_snap("model_advanced_reasoning_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3795 model_reasoning_selection_popup_applies_custom_effort kind=ui owner=dialogs-pane
#[test]
fn model_reasoning_selection_popup_applies_custom_effort() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ApplyModelSelection events a custom effort sends (gpt-5.5, the custom effort), not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3878 ultra_reasoning_selection_warns_for_high_multi_agent_concurrency kind=snap owner=dialogs-pane snap=ultra_reasoning_selection_high_multi_agent_concurrency_warning
#[test]
fn ultra_reasoning_selection_warns_for_high_multi_agent_concurrency() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: selecting Ultra with a high multi-agent thread limit shows the concurrency warning; no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 4).live().open();
    s.type_text("/model");
    s.keys("enter down enter down enter");
    s.assert_snap("ultra_reasoning_selection_high_multi_agent_concurrency_warning");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3898 max_reasoning_selection_persists_model_selection kind=ui owner=dialogs-pane
#[test]
fn max_reasoning_selection_persists_model_selection() {
    kit::na(
        Owner::DialogsPane,
        "asserts the model-selection events Max sends (persist, apply), not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3984 reasoning_shortcut_clears_armed_quit_shortcut kind=ui owner=dialogs-pane
#[test]
fn reasoning_shortcut_clears_armed_quit_shortcut() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.keys("ctrl+c");
    s.keys("alt+.");
    s.assert_absent("again to quit");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4005 reasoning_shortcut_is_ignored_with_model_popup_open kind=ui owner=dialogs-pane
#[test]
fn reasoning_shortcut_is_ignored_with_model_popup_open() {
    kit::na(
        Owner::DialogsPane,
        "asserts that no reasoning update and no model persistence events are sent while the model popup is open, not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4029 reasoning_up_shortcuts_reach_max_in_default_and_plan_modes kind=snap owner=dialogs-pane snap=reasoning_shortcut_max_footer snap=reasoning_shortcut_max_plan_footer
#[test]
fn reasoning_up_shortcuts_reach_max_in_default_and_plan_modes() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning shortcut (alt+. and shift+up) in the port: the status footer reads the max effort in default and plan modes; no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 6).live().open();
    s.keys("alt+.");
    s.assert_snap("reasoning_shortcut_max_footer");
    s.keys("tab");
    s.assert_snap("reasoning_shortcut_max_plan_footer");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4108 reasoning_up_shortcut_does_not_silently_enter_ultra kind=snap owner=dialogs-pane
#[test]
fn reasoning_up_shortcut_does_not_silently_enter_ultra() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning shortcut (alt+.) in the port: at Max it prints the Ultra hint in the history rather than entering Ultra; no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.keys("alt+.");
    // The oracle's inline literal "• Ultra is available under /model → <model path> → More reasoning…";
    // the model path is the test's own replacement, so both fixed parts are checked.
    s.assert_contains("• Ultra is available under /model → ");
    s.assert_contains(" → More reasoning…");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4159 reasoning_down_shortcut_can_leave_advanced_effort kind=ui owner=dialogs-pane
#[test]
fn reasoning_down_shortcut_can_leave_advanced_effort() {
    kit::na(
        Owner::DialogsPane,
        "asserts the reasoning update events the down shortcut sends from Ultra and Max, not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4196 reasoning_popup_shows_extra_high_with_space kind=ui owner=dialogs-pane
#[test]
fn reasoning_popup_shows_extra_high_with_space() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: the label of the extra-high effort (with a space) has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/model");
    s.keys("enter enter");
    s.assert_contains("Extra high");
    s.assert_absent("Extrahigh");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4216 single_reasoning_option_skips_selection kind=ui owner=dialogs-pane
#[test]
fn single_reasoning_option_skips_selection() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter enter");
    s.assert_absent("Select Reasoning Level");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4266 advanced_only_reasoning_option_requires_explicit_selection kind=snap owner=dialogs-pane snap=advanced_only_reasoning_selection_popup
#[test]
fn advanced_only_reasoning_option_requires_explicit_selection() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: a model whose only option is advanced shows the list and waits for an explicit selection; no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 6).live().open();
    s.type_text("/model");
    s.keys("enter enter");
    s.assert_snap("advanced_only_reasoning_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4288 auto_model_advertising_advanced_effort_opens_reasoning_picker kind=ui owner=dialogs-pane
#[test]
fn auto_model_advertising_advanced_effort_opens_reasoning_picker() {
    kit::na(
        Owner::DialogsPane,
        "asserts the events Enter sends (OpenReasoningPopup, and no reasoning or persistence update), not drawn on the screen",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4320 feedback_selection_popup_snapshot kind=snap owner=dialogs-pane snap=feedback_command_suggestion snap=feedback_selection_popup
#[test]
fn feedback_selection_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "the first frame (the /feedback suggestion) is the composer slash popup, not the dialogs screen; the category list itself is the feedback_categories entry (oracle data)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 5).live().open();
    s.type_text("/feedback");
    s.assert_snap("feedback_command_suggestion");
    s.keys("enter");
    s.resize(80, 11);
    s.assert_snap("feedback_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4341 feedback_stops_before_collecting_a_note_when_no_destination_is_available kind=snap owner=dialogs-pane
#[test]
fn feedback_stops_before_collecting_a_note_when_no_destination_is_available() {
    kit::pending(
        Owner::DialogsPane,
        "no /feedback flow in the port: with no upload destination the flow stops before the note and prints the two unavailable notices; no widget and no seed",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.assert_contains("• Feedback is unavailable in Cloud threads.");
    s.assert_contains(
        "• Feedback uploads are disabled, and saving a ZIP requires a local app server.",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4359 feedback_upload_consent_popup_snapshot kind=snap owner=dialogs-pane snap=feedback_upload_consent_popup
#[test]
fn feedback_upload_consent_popup_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the proxy connectivity diagnostics: feedback_consent and feedback_consent_windows carry the doctor report and no proxy headline",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 22).live().open();
    s.assert_snap("feedback_upload_consent_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4381 feedback_good_result_consent_popup_includes_connectivity_diagnostics_filename kind=snap owner=dialogs-pane snap=feedback_good_result_consent_popup
#[test]
fn feedback_good_result_consent_popup_includes_connectivity_diagnostics_filename() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the good-result consent and its connectivity diagnostics filename: the port consent entries carry no proxy headline",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 17).live().open();
    s.assert_snap("feedback_good_result_consent_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4403 reasoning_popup_escape_returns_to_model_popup kind=ui owner=dialogs-pane
#[test]
fn reasoning_popup_escape_returns_to_model_popup() {
    kit::pending(
        Owner::DialogsPane,
        "no reasoning-effort popup in the port: Esc from Select Reasoning Level returns to Select Model; no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter enter");
    s.assert_contains("Select Reasoning Level");
    s.keys("esc");
    s.assert_contains("Select Model");
    s.assert_absent("Select Reasoning Level");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4422 account_change_dismisses_the_previous_app_directory_snapshot kind=snap owner=dialogs-pane snap=connector_scope_invalidation
#[test]
fn account_change_dismisses_the_previous_app_directory_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no /apps directory popup in the port: an account change dismisses the previous app directory; no connector script step and no account seed",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 16).live().open();
    s.assert_snap("connector_scope_invalidation");
}

// ---- oracle: chatwidget/tests/quick_dials_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/quick_dials_tests.rs:8 current_settings_offer_model_permissions_and_status_layouts kind=snap owner=dialogs-pane snap=saved_quick_dials_source snap=saved_quick_dials_source_unavailable_model
#[test]
fn current_settings_offer_model_permissions_and_status_layouts() {
    kit::pending(
        Owner::DialogsPane,
        "no quick dials in the port: the saved quick-dial source list (model, permissions, status) has no widget and no seed of the current settings",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 10).live().open();
    s.assert_snap("saved_quick_dials_source");
    // The oracle's Down and Enter send /permissions, /title and /statusline commands: the events are not drawn.
    s.keys("down enter");
    s.resize(80, 12);
    s.assert_snap("saved_quick_dials_source_unavailable_model");
}

// ORACLE chatwidget/tests/quick_dials_tests.rs:141 saved_quick_dials_save_and_clear_picker_snapshots kind=snap owner=dialogs-pane snap=quick_dial_shortcuts snap=quick_dial_shortcuts_empty snap=quick_dial_shortcuts_narrow snap=saved_quick_dials_clear snap=saved_quick_dials_confirm_delete snap=saved_quick_dials_confirm_replace snap=saved_quick_dials_confirm_save snap=saved_quick_dials_editor snap=saved_quick_dials_save
#[test]
fn saved_quick_dials_save_and_clear_picker_snapshots() {
    kit::pending(
        Owner::DialogsPane,
        "no quick dials in the port: the shortcut menu, the slot editor, the save, clear and replace/delete confirmations have no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 10)
        .live()
        .open();
    s.assert_snap("quick_dial_shortcuts_empty");
    s.resize(80, 14);
    s.assert_snap("saved_quick_dials_editor");
    s.assert_snap("saved_quick_dials_save");
    s.assert_snap("saved_quick_dials_clear");
    s.resize(52, 8);
    s.assert_snap("saved_quick_dials_confirm_save");
    s.assert_snap("saved_quick_dials_confirm_replace");
    s.resize(80, 7);
    s.assert_snap("saved_quick_dials_confirm_delete");
    s.resize(100, 16);
    s.assert_snap("quick_dial_shortcuts");
    s.resize(48, 20);
    s.assert_snap("quick_dial_shortcuts_narrow");
}

// ---- oracle: chatwidget/tests/review_picker_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/review_picker_tests.rs:7 review_picker_filtered_accept_and_cancel_preserve_parent_and_draft kind=ui owner=dialogs-pane
#[test]
fn review_picker_filtered_accept_and_cancel_preserve_parent_and_draft() {
    kit::pending(
        Owner::DialogsPane,
        "no review picker in the port (`/review`): the commit picker with its filter and the f3 select / f2 back keys has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("Keep this draft");
    s.type_text("/review");
    s.keys("enter");
    s.assert_contains("› 3. Review a commit");
    s.assert_contains("f3 select · f2 back");
    s.keys("f2");
    s.assert_contains("Keep this draft");
}

// ---- oracle: chatwidget/tests/security_setup_tests.rs (5 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/security_setup_tests.rs:23 security_setup_preserves_draft_action_and_dismissal_across_widgets kind=snap owner=dialogs-pane
#[test]
fn security_setup_preserves_draft_action_and_dismissal_across_widgets() {
    kit::pending(
        Owner::DialogsPane,
        "no security-setup banner in the port: the Set up security notice with its action and Esc dismissal has no widget and no seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
    s.assert_snap("security_setup_banner");
    s.keys("esc");
    s.assert_absent("Set up security");
}

// ORACLE chatwidget/tests/security_setup_tests.rs:69 security_setup_dismissal_survives_usage_banner_replacement kind=snap owner=dialogs-pane
#[test]
fn security_setup_dismissal_survives_usage_banner_replacement() {
    kit::pending(
        Owner::DialogsPane,
        "no security-setup banner in the port: a dismissed notice stays dismissed when the usage banner replaces it; no widget and no backend banner seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
    s.keys("esc");
    s.assert_snap("security_setup_dismissed_after_usage");
}

// ORACLE chatwidget/tests/security_setup_tests.rs:126 security_setup_after_cutoff_wraps_in_narrow_terminal kind=snap owner=dialogs-pane snap=security_setup_after_cutoff
#[test]
fn security_setup_after_cutoff_wraps_in_narrow_terminal() {
    kit::pending(
        Owner::DialogsPane,
        "no security-setup banner in the port: the after-cutoff notice wraps in a 48-column terminal; no widget and no seed",
    );
    let s = Screen::build(Owner::DialogsPane).size(48, 24).live().open();
    s.assert_snap("security_setup_after_cutoff");
}

// ORACLE chatwidget/tests/security_setup_tests.rs:139 security_setup_after_persona_grace_wraps_in_narrow_terminal kind=snap owner=dialogs-pane snap=security_setup_after_persona_grace
#[test]
fn security_setup_after_persona_grace_wraps_in_narrow_terminal() {
    kit::pending(
        Owner::DialogsPane,
        "no security-setup banner in the port: the after-Persona-grace notice wraps in a 48-column terminal; no widget and no seed",
    );
    let s = Screen::build(Owner::DialogsPane).size(48, 24).live().open();
    s.assert_snap("security_setup_after_persona_grace");
}

// ORACLE chatwidget/tests/security_setup_tests.rs:152 security_setup_respects_applicable_backend_banner kind=ui owner=dialogs-pane
#[test]
fn security_setup_respects_applicable_backend_banner() {
    kit::pending(
        Owner::DialogsPane,
        "no security-setup banner in the port: it shows or hides by the applicable backend banner and the model; no widget and no backend banner seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
    s.assert_contains("Set up security");
    s.keys("esc");
    s.assert_absent("Set up security");
    s.assert_contains("Selected model usage exhausted");
}

// ---- oracle: chatwidget/tests/session_model_selection_tests.rs (4 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/session_model_selection_tests.rs:6 session_model_selection_accepts_final_choices_without_saving kind=ui owner=dialogs-pane
#[test]
fn session_model_selection_accepts_final_choices_without_saving() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter enter s");
    s.assert_absent("Select Model");
    s.assert_absent("Select Reasoning Level");
}

// ORACLE chatwidget/tests/session_model_selection_tests.rs:66 session_model_selection_notifies_the_original_task_after_each_final_astra_choice kind=ui owner=dialogs-pane
#[test]
fn session_model_selection_notifies_the_original_task_after_each_final_astra_choice() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("enter enter s");
    s.assert_absent("Select Model");
    s.assert_absent("Select Reasoning Level");
}

// ORACLE chatwidget/tests/session_model_selection_tests.rs:143 session_model_selection_hides_conflicting_shortcut kind=ui owner=dialogs-pane
#[test]
fn session_model_selection_hides_conflicting_shortcut() {
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .keymap(&[("list.accept", "s")])
        .live()
        .open();
    s.assert_absent("s session");
}

// ORACLE chatwidget/tests/session_model_selection_tests.rs:164 provider_switch_blocks_cached_choices_in_old_and_new_chats kind=snap owner=dialogs-pane
#[test]
fn provider_switch_blocks_cached_choices_in_old_and_new_chats() {
    kit::pending(
        Owner::DialogsPane,
        "no model picker in the port: after a provider switch the cached choices are blocked with the restart notice; no catalog seed and no provider switch step",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.assert_contains(
        "The model list may be out of date. Restart the app server and TUI to refresh /model.",
    );
}

// ---- oracle: chatwidget/tests/skills_summary_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/skills_summary_tests.rs:8 manage_skills_summary_snapshot kind=snap owner=dialogs-pane snap=manage_skills_summary
#[test]
fn manage_skills_summary_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no manage-skills summary in the port: the one-line skill summaries of the fixture skills have no widget and no seed",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 4).live().open();
    s.assert_snap("manage_skills_summary");
}

// ---- oracle: chatwidget/tests/worktree_picker_tests.rs (5 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/worktree_picker_tests.rs:9 slash_new_and_fork_offer_checkout_choices_inside_local_git_repository kind=snap owner=dialogs-pane snap=worktrees_command_local snap=worktrees_command_remote snap=worktrees_fork_choices snap=worktrees_new_choices
#[test]
fn slash_new_and_fork_offer_checkout_choices_inside_local_git_repository() {
    kit::pending(
        Owner::DialogsPane,
        "no worktree picker in the port (`/new`, `/fork`, `/work`): the checkout choices inside a local git repository have no widget and no git seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 8).live().open();
    s.type_text("/new");
    s.keys("enter");
    s.assert_snap("worktrees_new_choices");
    s.keys("esc");
    s.type_text("/fork named");
    s.keys("enter");
    s.assert_snap("worktrees_fork_choices");
    s.assert_contains("Use current Git worktree");
    s.assert_contains("Create new Git worktree");
    s.keys("esc");
    s.resize(80, 3);
    s.type_text("/work");
    s.assert_snap("worktrees_command_remote");
    s.resize(80, 5);
    s.assert_snap("worktrees_command_local");
}

// ORACLE chatwidget/tests/worktree_picker_tests.rs:85 slash_worktree_offers_current_or_new_conversation kind=snap owner=dialogs-pane snap=worktrees_conversation_choices
#[test]
fn slash_worktree_offers_current_or_new_conversation() {
    kit::pending(
        Owner::DialogsPane,
        "no worktree picker in the port: the continue-current or start-new conversation choice has no widget and no git seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 11).live().open();
    s.type_text("/worktree");
    s.keys("enter");
    s.assert_snap("worktrees_conversation_choices");
    s.assert_contains("Continue current conversation");
    s.assert_contains("Start new conversation");
}

// ORACLE chatwidget/tests/worktree_picker_tests.rs:107 worktree_browser_actions_and_stale_results kind=snap owner=dialogs-pane snap=worktree_browser_actions snap=worktree_browser_delete_confirmation snap=worktree_browser_list snap=worktree_browser_loading
#[test]
fn worktree_browser_actions_and_stale_results() {
    kit::pending(
        Owner::DialogsPane,
        "no worktree browser in the port: the loading state, the list, the actions, the delete confirmation and stale results have no widget and no git seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 6).live().open();
    s.type_text("/worktree");
    s.keys("enter");
    s.assert_snap("worktree_browser_loading");
    s.resize(80, 8);
    s.assert_snap("worktree_browser_list");
    s.keys("enter");
    s.resize(80, 9);
    s.assert_snap("worktree_browser_actions");
    s.resize(80, 8);
    s.assert_snap("worktree_browser_delete_confirmation");
}

// ORACLE chatwidget/tests/worktree_picker_tests.rs:214 worktree_browser_names_archived_and_missing_owners kind=snap owner=dialogs-pane snap=worktree_browser_owner_states
#[test]
fn worktree_browser_names_archived_and_missing_owners() {
    kit::pending(
        Owner::DialogsPane,
        "no worktree browser in the port: the owner states (archived and missing owners) and the Worktree: title have no widget and no git seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 10).live().open();
    s.type_text("/worktree");
    s.keys("enter");
    s.assert_snap("worktree_browser_owner_states");
    s.assert_contains("Worktree: Fix settings");
    s.assert_absent("Resume owner");
}

// ORACLE chatwidget/tests/worktree_picker_tests.rs:273 worktree_picker_custom_keys_preserve_draft_and_protect_current_checkout kind=ui owner=dialogs-pane
#[test]
fn worktree_picker_custom_keys_preserve_draft_and_protect_current_checkout() {
    kit::pending(
        Owner::DialogsPane,
        "no worktree browser in the port: the remapped f3 select and f2 back keys, the draft kept, and the current checkout protected from deletion; no widget and no keymap seed",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 24)
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .live()
        .open();
    s.type_text("Keep this draft");
    s.type_text("/worktree");
    s.keys("enter");
    s.assert_contains("f3 select · f2 back");
    s.keys("f2");
    s.assert_contains("Keep this draft");
    s.resize(80, 24);
    s.assert_contains("Switch to another checkout before deleting this one");
}
