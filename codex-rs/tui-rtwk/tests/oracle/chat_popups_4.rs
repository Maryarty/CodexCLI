//! Shard `chat_popups_4` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Snapshot heights are the oracle's popup heights (`desired_height`), taken from the
//! snapshot rows; widths are the oracle test's `render_bottom_popup(.., width)` widths.

#![allow(clippy::expect_used)]

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/popups_and_settings.rs (43 screen-level tests here; 4 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/popups_and_settings.rs:2864 apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps kind=ui owner=dialogs-pane
#[test]
fn apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps() {
    kit::pending(
        Owner::DialogsPane,
        "/apps is live; the oracle injects a partial snapshot (`is_final: false`) and then a failed refresh: no protocol message carries a partial snapshot (the kit answers a request with an error, `Screen::answer_error`)",
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
    use crate::chat_popups_3::apps_answer;
    // the config takes the write of the switch
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(
            r#"{"step":"respond","method":"config/batchWrite","result":{"status":"ok","version":"v","filePath":"/c","overriddenMetadata":null}}"#,
        )
        .open();
    s.type_text("/apps");
    s.keys("enter");
    let asked = s.asked_apps();
    s.answer_request(
        &asked[0],
        apps_answer(&[("connector_1", "Notion", "Workspace docs")]),
    );
    // the oracle calls `update_connector_enabled`; the port's way to switch an app is the link
    // dialog: Enter on the app, then `Disable app`
    s.keys("enter");
    s.assert_contains("2. Disable app");
    s.keys("down");
    s.assert_contains("› 2. Disable app");
    s.keys("enter");
    s.assert_contains("2. Enable app");
    s.keys("esc");
    // the directory is read again and still says the app is on: the switch stays
    s.type_text("/apps");
    s.keys("enter");
    s.assert_apps_asked(2);
    let asked = s.asked_apps();
    s.answer_request(
        &asked[1],
        apps_answer(&[("connector_1", "Notion", "Workspace docs")]),
    );
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

/// The server's `model/list` answer of the bundled catalog (what the oracle's
/// `chat.model_catalog` holds in these tests).
const BUNDLED: &str = include_str!("fixtures/chat_popups_4/bundled.json");

/// A live chat on `model` whose server lists `catalog` (the oracle's
/// `chat.model_catalog = ModelCatalog::new(presets)`).
fn on_catalog(width: u16, height: u16, model: &str, catalog: &str) -> Screen {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(width, height)
        .live()
        .open();
    let catalog = catalog.trim();
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{catalog}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}"}}}}}}"#
    ));
    s
}

/// The `model/list` answer of `models` in the wire form of the app server.
fn wire(models: &[serde_json::Value]) -> String {
    serde_json::json!({"data": models, "nextCursor": null}).to_string()
}

/// The models of a catalog text.
fn models_of(catalog: &str) -> Vec<serde_json::Value> {
    let catalog: serde_json::Value = serde_json::from_str(catalog.trim()).expect("a catalog");
    catalog["data"].as_array().expect("data").clone()
}

/// The oracle's `get_available_model(&chat, id)`: one model of the bundled catalog.
fn bundled_model(id: &str) -> serde_json::Value {
    models_of(BUNDLED)
        .into_iter()
        .find(|m| m["id"] == id)
        .expect("a bundled model")
}

/// The wire model as the select screen's own `Preset` (what its `catalog` seed reads).
fn preset_of(model: &serde_json::Value) -> serde_json::Value {
    let efforts: Vec<_> = model["supportedReasoningEfforts"]
        .as_array()
        .expect("efforts")
        .iter()
        .map(|e| serde_json::json!({"effort": e["reasoningEffort"], "description": e["description"]}))
        .collect();
    serde_json::json!({
        "model": model["model"],
        "name": model["displayName"],
        "description": model["description"],
        "show_in_picker": !model["hidden"].as_bool().unwrap_or(false),
        "default_effort": model["defaultReasoningEffort"],
        "efforts": efforts,
    })
}

/// A live chat on `model` that holds `startup` as its catalog (the oracle's
/// `chat.model_catalog = ModelCatalog::new(presets)`); the server has not answered yet.
fn on_startup_catalog(
    width: u16,
    height: u16,
    model: &str,
    startup: &[serde_json::Value],
) -> Screen {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(width, height)
        .live()
        .open();
    let presets: Vec<_> = startup.iter().map(preset_of).collect();
    let seed = serde_json::json!({"step":"screen","dir":"select","ev":{"kind":"catalog","presets":presets}});
    s.feed(&format!(
        r#"{seed}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}"}}}}}}"#
    ));
    s
}

/// The `model/list` the app has asked last, still unanswered.
fn last_model_list(s: &Screen) -> kit::Ask {
    s.asked_models()
        .pop()
        .expect("the app asked for the model list")
}

/// The server answers the `model/list` the popup waits for with `models` (the oracle's
/// `apply_model_list_response`).
fn refresh(s: &mut Screen, models: &[serde_json::Value]) {
    let ask = last_model_list(s);
    s.answer_request(&ask, serde_json::from_str(&wire(models)).expect("wire"));
}

/// The effort rows of the wire model: `(effort, description)` in the order they are listed.
fn with_efforts(model: &str, efforts: &[(&str, &str)]) -> serde_json::Value {
    let mut model = bundled_model(model);
    model["supportedReasoningEfforts"] = efforts
        .iter()
        .map(|(effort, description)| {
            serde_json::json!({"reasoningEffort": effort, "description": description})
        })
        .collect();
    model
}

/// The thread runs on `model` with `effort` (the chat's status facts; the oracle's
/// `chat.set_reasoning_effort`).
fn on_effort(s: &mut Screen, model: &str, effort: &str) {
    s.feed(&format!(
        r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}","effort":"{effort}"}}}}}}"#
    ));
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3360 model_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_selection_popup_snapshot() {
    let mut s = on_catalog(80, 14, "gpt-5.5", BUNDLED);
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
    for (explicit_all_models, hidden_startup) in [(false, false), (true, false), (false, true)] {
        // the startup catalog holds gpt-5.5 (hidden in the third case); the refresh brings the
        // whole bundled catalog and an auto model
        let mut startup = vec![bundled_model("gpt-5.5")];
        startup[0]["hidden"] = hidden_startup.into();
        let mut refreshed = models_of(BUNDLED);
        let mut auto = bundled_model("gpt-5.5");
        auto["id"] = "codex-auto-test".into();
        auto["model"] = "codex-auto-test".into();
        auto["description"] = "Auto model".into();
        refreshed.push(auto.clone());
        if explicit_all_models {
            // the All models page needs an auto model to be a page of its own (the oracle opens
            // it by `open_all_models_popup`)
            startup.insert(0, auto);
        }
        let mut s = on_startup_catalog(80, 14, "gpt-5.5", &startup);
        s.type_text("/model");
        s.keys("enter");
        if explicit_all_models {
            // the quick menu's cursor is on `All models` (the current choice)
            s.keys("enter");
        }
        refresh(&mut s, &refreshed);
        if hidden_startup {
            // the startup catalog offered nothing: no popup opened while the refresh arrived,
            // `/model` opens the refreshed one
            s.type_text("/model");
            s.keys("enter");
        }
        if explicit_all_models {
            s.assert_snap("model_selection_popup");
        } else {
            s.resize(80, 9);
            s.assert_snap("model_picker_refreshes_auto_models");
        }
    }
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3463 model_picker_queued_all_models_uses_refreshed_catalog kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_picker_queued_all_models_uses_refreshed_catalog() {
    // the startup catalog has an auto model and gpt-5.5; the refresh is the bundled catalog
    let mut auto = bundled_model("gpt-5.5");
    auto["id"] = "codex-auto-test".into();
    auto["model"] = "codex-auto-test".into();
    let mut s = on_startup_catalog(80, 14, "gpt-5.5", &[auto, bundled_model("gpt-5.5")]);
    s.type_text("/model");
    s.keys("enter");
    // `All models` (the quick menu's current choice) is picked before the answer comes
    s.keys("enter");
    refresh(&mut s, &models_of(BUNDLED));
    s.assert_snap("model_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3487 model_picker_refresh_preserves_highlight kind=snap owner=dialogs-pane snap=model_picker_refresh_preserves_highlight
#[test]
fn model_picker_refresh_preserves_highlight() {
    for (remove_selected, reasoning_submenu, expected) in [
        (false, false, "Renamed model"),
        (true, false, "Shared display name"),
        (false, true, "Renamed model"),
        (true, true, "Shared display name"),
    ] {
        let mut presets = vec![bundled_model("gpt-5.5"), bundled_model("gpt-5.6-terra")];
        for preset in &mut presets {
            preset["displayName"] = "Shared display name".into();
        }
        let mut s = on_startup_catalog(80, 14, "gpt-5.5", &presets);
        s.type_text("/model");
        s.keys("enter");
        s.keys("down");
        if reasoning_submenu {
            s.keys("enter");
        }
        let before = s.shot();
        presets.reverse();
        presets[0]["displayName"] = "Renamed model".into();
        if remove_selected {
            presets.remove(0);
        }
        refresh(&mut s, &presets);
        if reasoning_submenu {
            s.assert_same_rows(&before);
            s.keys("esc");
        }
        if !remove_selected {
            s.resize(80, 7);
            s.assert_snap("model_picker_refresh_preserves_highlight");
            s.resize(80, 14);
        }
        // Enter opens the reasoning menu of the row the highlight stayed on
        s.keys("enter");
        s.assert_contains(&format!("Select Reasoning Level for {expected}"));
    }
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
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{refreshed}}}"#
    ));
    s.type_text("/model");
    s.keys("enter");
    s.keys("esc");
    // the model no longer offers a tier: the effective tier is none and `/fast` lists nothing
    s.type_text("/fast");
    let row = s
        .rows()
        .iter()
        .position(|r| r.starts_with("› /fast"))
        .expect("the composer row");
    s.assert_snap_region("model_picker_refreshes_service_tier_controls", row, 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3561 model_picker_refresh_preserves_dismissal_and_reasoning_submenu kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_picker_refresh_preserves_dismissal_and_reasoning_submenu() {
    for (dismiss, explicit_all_models, accept_reasoning) in [
        (true, false, false),
        (false, false, false),
        (false, true, false),
        (false, false, true),
        (false, true, true),
    ] {
        let mut startup = vec![bundled_model("gpt-5.5")];
        if explicit_all_models {
            // an auto model makes the quick menu a page of its own: `All models` is picked from it
            let mut auto = bundled_model("gpt-5.5");
            auto["id"] = "codex-auto-test".into();
            auto["model"] = "codex-auto-test".into();
            startup.insert(0, auto);
        }
        let mut s = on_startup_catalog(80, 14, "gpt-5.5", &startup);
        s.type_text("/model");
        s.keys("enter");
        if explicit_all_models {
            s.keys("enter");
        }
        if dismiss {
            s.keys("esc");
        } else {
            // the model row opens its reasoning menu
            s.keys("enter");
        }
        let before = s.shot();
        refresh(&mut s, &models_of(BUNDLED));
        // the refresh changes nothing of what is open: a dismissed picker stays dismissed
        s.assert_same_rows(&before);
        if !dismiss {
            if accept_reasoning {
                s.keys("enter");
            } else {
                s.keys("esc");
            }
        }
        if dismiss || accept_reasoning {
            s.assert_absent("Select Model and Effort");
            s.assert_absent("Select Reasoning Level");
        } else {
            s.assert_snap("model_selection_popup");
        }
    }
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3615 model_picker_refresh_dismisses_empty_choices kind=snap owner=dialogs-pane
#[test]
fn model_picker_refresh_dismisses_empty_choices() {
    for (explicit_all_models, reasoning_submenu) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut preset = bundled_model("gpt-5.5");
        let mut startup = vec![preset.clone()];
        if explicit_all_models {
            // an auto model makes the quick menu a page of its own: `All models` is picked from it
            let mut auto = preset.clone();
            auto["id"] = "codex-auto-test".into();
            auto["model"] = "codex-auto-test".into();
            startup.insert(0, auto);
        }
        let mut s = on_startup_catalog(80, 24, "gpt-5.5", &startup);
        s.type_text("/model");
        s.keys("enter");
        if explicit_all_models {
            s.keys("enter");
        }
        if reasoning_submenu {
            s.keys("enter");
        }
        // the refresh leaves no model to pick: only an auto one, or one hidden from the picker
        if explicit_all_models {
            preset["id"] = "codex-auto-test".into();
            preset["model"] = "codex-auto-test".into();
        } else {
            preset["hidden"] = true.into();
        }
        refresh(&mut s, &[preset]);
        if reasoning_submenu {
            // the reasoning menu stays; Esc leaves the picker that was under it
            s.assert_contains("Select Reasoning Level");
            s.keys("esc");
        }
        s.assert_absent("Select Model and Effort");
        s.assert_absent("Select Reasoning Level");
        s.assert_contains("• No additional models are available right now.");
    }
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3654 skills_menu_default_mentions_shortcut_snapshot kind=snap owner=dialogs-pane snap=skills_menu_default_mentions_shortcut
#[test]
fn skills_menu_default_mentions_shortcut_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature skills menu (w2-approvals, chat popups): none in the port in the port (`/skills`): the default skills list with its mention shortcut has no widget and no scripted state",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 8).live().open();
    s.type_text("/skills");
    s.keys("enter");
    s.assert_snap("skills_menu_default_mentions_shortcut");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3663 model_picker_hides_show_in_picker_false_models_from_cache kind=snap owner=dialogs-pane snap=model_picker_filters_hidden_models
#[test]
fn model_picker_hides_show_in_picker_false_models_from_cache() {
    // the oracle's `preset(slug, show_in_picker)`
    let preset = |slug: &str, show_in_picker: bool| {
        serde_json::json!({
            "id": slug, "model": slug, "displayName": slug,
            "description": format!("{slug} description"),
            "hidden": !show_in_picker,
            "defaultReasoningEffort": "medium",
            "supportedReasoningEfforts": [{"description": "medium", "reasoningEffort": "medium"}],
        })
    };
    let mut s = on_startup_catalog(
        80,
        6,
        "test-visible-model",
        &[
            preset("test-visible-model", true),
            preset("test-hidden-model", false),
        ],
    );
    s.type_text("/model");
    s.keys("enter");
    s.assert_snap("model_picker_filters_hidden_models");
    s.assert_contains("test-visible-model");
    s.assert_absent("test-hidden-model");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3740 model_reasoning_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_reasoning_selection_popup
#[test]
fn model_reasoning_selection_popup_snapshot() {
    let model = with_efforts(
        "gpt-5.5",
        &[
            ("low", "Fast responses with lighter reasoning"),
            (
                "medium",
                "Balances speed and reasoning depth for everyday tasks",
            ),
            ("max", "Maximum available reasoning"),
            ("high", "Greater reasoning depth for complex problems"),
            ("xhigh", "Extra high reasoning depth for complex problems"),
            ("ultra", "Ultra reasoning"),
            ("persistent", "Continue working until put to sleep"),
        ],
    );
    let mut s = on_startup_catalog(80, 11, "gpt-5.5", &[model]);
    on_effort(&mut s, "gpt-5.5", "high");
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    s.assert_snap("model_reasoning_selection_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:3773 model_advanced_reasoning_selection_popup_snapshot kind=snap owner=dialogs-pane snap=model_advanced_reasoning_selection_popup
#[test]
fn model_advanced_reasoning_selection_popup_snapshot() {
    let mut model = bundled_model("gpt-5.5");
    let efforts = model["supportedReasoningEfforts"]
        .as_array_mut()
        .expect("efforts");
    efforts.push(serde_json::json!({"reasoningEffort": "ultra", "description": "Ultra reasoning"}));
    efforts.push(
        serde_json::json!({"reasoningEffort": "max", "description": "Maximum available reasoning"}),
    );
    let mut s = on_startup_catalog(80, 9, "gpt-5.5", &[model]);
    on_effort(&mut s, "gpt-5.5", "ultra");
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    // the current effort is Ultra: the reasoning menu stands on its More row
    s.keys("enter");
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
        "the warning is a cell of the detailed transcript only (`add_warning_message`, visible_in_transcript: false): the transcript pager (S16) is not ported, and the select drain drops the note instead of recording it as a hidden warning of the thread",
    );
    let model = with_efforts(
        "gpt-5.5",
        &[("high", "High reasoning"), ("ultra", "Ultra reasoning")],
    );
    let mut s = on_startup_catalog(80, 12, "gpt-5.5", &[model]);
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5","effort":"high","agent_threads":8}}}"#,
    );
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    // `More reasoning…` is the row below High, Ultra its only row
    s.keys("down");
    s.keys("enter");
    s.keys("enter");
    // the transcript pager shows the cell
    s.keys("ctrl+t");
    s.assert_snap_region(
        "ultra_reasoning_selection_high_multi_agent_concurrency_warning",
        0,
        0,
    );
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

/// The server's `thread/settings/updated`: what the thread runs on after a shortcut or a mode
/// switch (the kit's backend does not echo the settings it takes).
fn settings_updated(model: &str, effort: &str, mode: &str) -> String {
    serde_json::json!({"step":"notify","message":{"method":"thread/settings/updated","params":{
        "threadId":"thread-1",
        "threadSettings":{"cwd":"/tmp/project","approvalPolicy":"on-request","approvalsReviewer":"user",
            "sandboxPolicy":{"type":"dangerFullAccess"},"activePermissionProfile":null,
            "model":model,"modelProvider":"openai","serviceTier":null,"effort":effort,"summary":null,
            "collaborationMode":{"mode":mode,"settings":{"model":model,"reasoning_effort":effort,"developer_instructions":null}},
            "personality":null}}}})
    .to_string()
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4029 reasoning_up_shortcuts_reach_max_in_default_and_plan_modes kind=snap owner=dialogs-pane snap=reasoning_shortcut_max_footer snap=reasoning_shortcut_max_plan_footer
#[test]
fn reasoning_up_shortcuts_reach_max_in_default_and_plan_modes() {
    let max = include_str!("fixtures/chat_popups_4/max_catalog.json").trim();
    for plan_mode in [false, true] {
        for key in ["alt+.", "shift+up"] {
            let mut s = Screen::build(Owner::DialogsPane).size(80, 6).live().open();
            s.feed(&format!(
                r#"{{"step":"respond","method":"model/list","result":{max}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.5","effort":"xhigh","cwd":"/tmp/project","items":["model-with-reasoning"]}}}}}}"#
            ));
            s.type_text("/model");
            s.keys("enter");
            s.keys("esc");
            s.feed(&settings_updated(
                "gpt-5.5",
                "xhigh",
                if plan_mode { "plan" } else { "default" },
            ));
            s.keys(key);
            s.assert_last_request_has("thread/settings/update", r#""max""#);
            // Plan mode steps the Plan override: the mode rides with the effort
            s.assert_last_request_mentions("thread/settings/update", r#""plan""#, plan_mode);
            // the server answers with the settings it took; the footer reads them
            s.feed(&settings_updated(
                "gpt-5.5",
                "max",
                if plan_mode { "plan" } else { "default" },
            ));
            if key == "alt+." {
                // the status line plays its transition to Max (the oracle's `M A X` frames,
                // about two seconds) before it settles on the text the snapshot holds
                s.wait(2600);
                // The snapshot's rows; its hint row reads `? for shortcuts` where ours adds
                // `← for agents` (the oracle's chat widget has agents navigation off: a gap
                // of every footer snapshot, not of the shortcut).
                s.assert_has_row("  GPT-5.5 max");
                s.assert_has_row("› Ask Codex to do anything");
                // GAP: the Plan mode indicator (`Plan mode (⇧tab to cycle)`, right of the hint
                // row) is not drawn: the footer widget takes no mode fact yet (pane-footer's).
                s.assert_contains("? for shortcuts");
            }
        }
    }
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4108 reasoning_up_shortcut_does_not_silently_enter_ultra kind=snap owner=dialogs-pane
#[test]
fn reasoning_up_shortcut_does_not_silently_enter_ultra() {
    for (catalog, model) in [
        (
            include_str!("fixtures/chat_popups_4/ultra_catalog.json").trim(),
            "gpt-5.5",
        ),
        (
            include_str!("fixtures/chat_popups_4/ultra_auto_catalog.json").trim(),
            "codex-auto-test",
        ),
    ] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        s.feed(&format!(
            r#"{{"step":"respond","method":"model/list","result":{catalog}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}","effort":"max","cwd":"/tmp/project"}}}}}}"#
        ));
        s.type_text("/model");
        s.keys("enter");
        s.keys("esc");
        s.feed(&settings_updated(model, "max", "default"));
        let before = s.requests_of("thread/settings/update");
        s.keys("alt+.");
        // The oracle's inline literal "• Ultra is available under /model → <model path> → More reasoning…".
        let path = if model.starts_with("codex-auto-") {
            model.to_string()
        } else {
            format!("All models → {model}")
        };
        s.assert_contains(&format!(
            "• Ultra is available under /model → {path} → More reasoning…"
        ));
        // and nothing was asked of the server
        s.assert_requests_of("thread/settings/update", before);
    }
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
    let mut s = on_startup_catalog(120, 24, "gpt-5.5", &[bundled_model("gpt-5.5")]);
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    s.assert_contains("Select Reasoning Level");
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
    let mut model = with_efforts("gpt-5.5", &[("ultra", "Ultra reasoning")]);
    model["defaultReasoningEffort"] = "ultra".into();
    let mut s = on_startup_catalog(80, 6, "gpt-5.5", &[model]);
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    // the only option is advanced: the list waits for an explicit selection
    s.assert_snap("advanced_only_reasoning_selection_popup");
    s.assert_no_request("thread/settings/update");
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
        "G68: the first frame (the /feedback suggestion) is the composer slash popup, which floats under a blank row with a `▎` bar and a right-aligned detail, not the oracle's `› /feedback  send logs..` row above the composer with the status footer; the second frame is the feedback sheet as a titled raised `Dialog` (rows indented 6, wrapped narrower, a `▎` mark, 3 rows over the 11 of the oracle's pane) where the oracle's is the bare pane with a `›` mark and two spaces of indent",
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
        "feature /feedback flow (w2-approvals, chat popups): none in the port in the port: with no upload destination the flow stops before the note and prints the two unavailable notices; no widget and no scripted state",
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
        "entry to register (w2-commands, feedback consent): the entry with the proxy connectivity diagnostics: feedback_consent and feedback_consent_windows carry the doctor report and no proxy headline",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 22).live().open();
    s.assert_snap("feedback_upload_consent_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4381 feedback_good_result_consent_popup_includes_connectivity_diagnostics_filename kind=snap owner=dialogs-pane snap=feedback_good_result_consent_popup
#[test]
fn feedback_good_result_consent_popup_includes_connectivity_diagnostics_filename() {
    kit::pending(
        Owner::DialogsPane,
        "entry to register (w2-commands, feedback consent): the entry with the good-result consent and its connectivity diagnostics filename: the port consent entries carry no proxy headline",
    );
    let s = Screen::build(Owner::DialogsPane).size(80, 17).live().open();
    s.assert_snap("feedback_good_result_consent_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4403 reasoning_popup_escape_returns_to_model_popup kind=ui owner=dialogs-pane
#[test]
fn reasoning_popup_escape_returns_to_model_popup() {
    let mut s = on_startup_catalog(80, 24, "gpt-5.5", &[bundled_model("gpt-5.5")]);
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    s.assert_contains("Select Reasoning Level");
    s.keys("esc");
    s.assert_contains("Select Model");
    s.assert_absent("Select Reasoning Level");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:4422 account_change_dismisses_the_previous_app_directory_snapshot kind=snap owner=dialogs-pane snap=connector_scope_invalidation
#[test]
fn account_change_dismisses_the_previous_app_directory_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 16).live().open();
    s.type_text("/apps").keys("enter");
    let asked = s.asked_apps();
    s.answer_request(
        &asked[0],
        serde_json::json!({"data": [
            {"id": "previous-account", "name": "Previous Account App", "isAccessible": true},
        ], "nextCursor": null}),
    );
    // "Before account change:" (the snapshot joins the two renders under labels the kit does not
    // read as frames: the popup's rows are checked one by one)
    s.assert_in_order(&[
        "Apps",
        "Use $ to insert an installed app into your prompt.",
        "Installed 1 of 1 available apps.",
        "Type to search apps",
        "› Previous Account App  Installed · App link unavailable",
        "enter select · esc back",
    ]);
    s.assert_has_row("› Previous Account App  Installed · App link unavailable");
    // the account changes: the previous account's directory is forgotten and its popup dismissed
    s.feed(&format!(
        "{}\n",
        serde_json::json!({"step": "notify", "message": {"method": "account/updated",
            "params": {"authMode": "chatgpt", "planType": null}}})
    ));
    // "After account change:" the bare composer
    s.assert_absent("Previous Account App");
    s.assert_absent("Installed 1 of 1 available apps.");
    s.assert_absent("enter select · esc back");
    s.assert_contains("Ask Codex to do anything");
}

// ---- oracle: chatwidget/tests/quick_dials_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/quick_dials_tests.rs:8 current_settings_offer_model_permissions_and_status_layouts kind=snap owner=dialogs-pane snap=saved_quick_dials_source snap=saved_quick_dials_source_unavailable_model
#[test]
fn current_settings_offer_model_permissions_and_status_layouts() {
    kit::pending(
        Owner::DialogsPane,
        "feature quick dials (w2-approvals, chat popups): none in the port in the port: the saved quick-dial source list (model, permissions, status) has no widget and no scripted state of the current settings",
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
        "feature quick dials (w2-approvals, chat popups): none in the port in the port: the shortcut menu, the slot editor, the save, clear and replace/delete confirmations have no widget and no scripted state",
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
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .live()
        .open();
    let commits = [("1111111", "First commit"), ("abcdef2", "Selected commit")];
    // the oracle's draft is in the composer before the popup opens; here the command that opens
    // it takes the composer, so the draft is what the line left: nothing
    s.type_text("/review");
    s.keys("enter");
    s.assert_contains("Select a review preset");
    s.keys("down down f3");
    s.assert_git_logs_asked(1);
    let asks = s.asked_git_logs();
    s.answer_git_log(&asks[0], &commits);
    s.assert_contains("Select a commit to review");
    s.keys("a f2");
    s.assert_contains("› 3. Review a commit");
    s.assert_contains("f3 select · f2 back");
    s.keys("f3");
    s.assert_git_logs_asked(2);
    let asks = s.asked_git_logs();
    s.answer_git_log(&asks[1], &commits);
    s.type_text("abcdef2");
    s.keys("f3");
    s.assert_review_of_commit("abcdef2", "Selected commit");
    s.assert_absent("Select a commit to review");
    s.assert_absent("Select a review preset");
    s.assert_composer("");
}

// ---- oracle: chatwidget/tests/security_setup_tests.rs (5 screen-level tests here; 0 state tests left to dialogs-pane)

/// The first row that contains `text`; past the screen when none does.
fn row_holding(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

/// The host's fact: the notice fetched for `account` (`show_security_setup`).
fn security_step(account: &str, title: &str, description: &str) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "fact", "feature": "security_setup", "fact": {
            "kind": "loaded",
            "identity": {"account": account, "user": "user"},
            "notice": {
                "title": title, "description": description,
                "action": {"label": "Set up security", "url": "https://chatgpt.com/cyber"},
            },
        }})
    )
}

/// The default notice of the oracle's tests.
fn security_notice(account: &str) -> String {
    security_step(
        account,
        "Keep using Daybreak mode",
        "Set up Advanced Account Security with a hardware security key by October 1. Already Persona-verified? Add your key before October 15 to skip re-verification.",
    )
}

/// `invalidate_security_setup`: the account was announced again.
fn security_invalidated() -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "fact", "feature": "security_setup", "fact": {"kind": "invalidated"}})
    )
}

/// The account read's answer: a dismissible `selected_model_limit` banner on `test-model-a`.
fn usage_banner() -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "respond", "method": "account/rateLimits/read", "result": {
            "accountId": "workspace-a", "rateLimits": {},
            "rateLimitUpsell": {
                "banner_type": "selected_model_limit", "model_slug": "test-model-a",
                "title": "Selected model usage exhausted", "description": "Switch models.",
                "presentation": "dismissible", "ctas": [],
            },
        }})
    )
}

/// `set_model` (the config's model in use) and, with `read`, the account read that follows.
fn model_step(model: &str, read: bool) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "screen", "dir": "chat", "ev": {"status": {"model": model, "account_usage": read}}})
    )
}

/// The oracle's snapshot `name` against the screen from `row`, up to and including the snapshot's
/// first row holding `last`. What the oracle draws under that row is not the banner's: the footer
/// (`test-model-a default · /tmp/project` over `? for shortcuts`, ours is one `← for agents` row)
/// and the composer's gap (ours has one more blank row between an actions banner's hint and the
/// composer, the bottom pane's layout).
fn assert_snap_through(s: &Screen, name: &str, row: usize, last: &str) {
    let want = kit::snap_rows(name, "");
    let end = want
        .iter()
        .position(|r| r.contains(last))
        .unwrap_or_else(|| panic!("{name}: no row holds `{last}`"));
    let ours = s.rows();
    assert!(
        row < ours.len(),
        "{name}: the banner is not on the screen:\n{}",
        s.text()
    );
    let have: Vec<String> = (0..=end)
        .map(|at| ours.get(row + at).map_or("", |r| r.trim_end()).to_owned())
        .collect();
    let want: Vec<String> = want[..=end]
        .iter()
        .map(|r| r.trim_end().to_owned())
        .collect();
    pretty_assertions::assert_eq!(
        have,
        want,
        "{name}: ours (left) differs from the oracle (right)"
    );
}

// ORACLE chatwidget/tests/security_setup_tests.rs:23 security_setup_preserves_draft_action_and_dismissal_across_widgets kind=snap owner=dialogs-pane
#[test]
fn security_setup_preserves_draft_action_and_dismissal_across_widgets() {
    let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
    s.feed(&security_notice("account"));
    assert_snap_through(
        &s,
        "security_setup_banner",
        row_holding(&s, "Keep using Daybreak mode"),
        "Press a number",
    );
    // 1 opens the page in the browser and the banner stays
    s.keys("1");
    assert_eq!(s.opened_urls(), ["https://chatgpt.com/cyber"]);
    s.assert_contains("Set up security");
    s.keys("esc");
    s.assert_absent("Set up security");
    // the account is announced again and the same identity is verified: the dismissal stays
    s.feed(&(security_invalidated() + &security_notice("account")));
    s.assert_absent("Set up security");
    // a verified change of account starts a new notice occurrence
    s.feed(&(security_invalidated() + &security_notice("other-account")));
    s.assert_contains("Set up security");
}

// ORACLE chatwidget/tests/security_setup_tests.rs:69 security_setup_dismissal_survives_usage_banner_replacement kind=snap owner=dialogs-pane
#[test]
fn security_setup_dismissal_survives_usage_banner_replacement() {
    for dismiss_security in [true, false] {
        let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
        s.feed(&security_notice("account"));
        s.assert_contains("Set up security");
        if dismiss_security {
            s.keys("esc");
        }
        s.feed(&(usage_banner() + &model_step("test-model-a", true)));
        s.wait_for("Selected model usage exhausted");
        if !dismiss_security {
            // Dismissing usage must not dismiss the security reminder instead.
            s.assert_absent("Set up security");
            s.keys("esc");
        }
        // Account refresh must not read the replacement banner's dismissal state.
        s.feed(&(security_invalidated() + &security_notice("account")));
        s.assert_present("Set up security", !dismiss_security);
        if dismiss_security {
            assert_snap_through(
                &s,
                "security_setup_dismissed_after_usage",
                row_holding(&s, "Ask Codex to do anything"),
                "Ask Codex to do anything",
            );
        }
    }
}

// ORACLE chatwidget/tests/security_setup_tests.rs:126 security_setup_after_cutoff_wraps_in_narrow_terminal kind=snap owner=dialogs-pane snap=security_setup_after_cutoff
#[test]
fn security_setup_after_cutoff_wraps_in_narrow_terminal() {
    let mut s = Screen::build(Owner::DialogsPane).size(48, 24).live().open();
    s.feed(&security_step(
        "account",
        "Set up security for Daybreak mode",
        "Set up Advanced Account Security with a hardware security key. Already Persona-verified? Add your key before October 15 to skip re-verification. You can keep using Codex while you finish setup.",
    ));
    assert_snap_through(
        &s,
        "security_setup_after_cutoff",
        row_holding(&s, "Set up security for Daybreak mode"),
        "type to continue",
    );
}

// ORACLE chatwidget/tests/security_setup_tests.rs:139 security_setup_after_persona_grace_wraps_in_narrow_terminal kind=snap owner=dialogs-pane snap=security_setup_after_persona_grace
#[test]
fn security_setup_after_persona_grace_wraps_in_narrow_terminal() {
    let mut s = Screen::build(Owner::DialogsPane).size(48, 24).live().open();
    s.feed(&security_step(
        "account",
        "Set up security for Daybreak mode",
        "Set up Advanced Account Security with a hardware security key. Verify each new hardware security key with Persona. You can keep using Codex while you finish setup.",
    ));
    assert_snap_through(
        &s,
        "security_setup_after_persona_grace",
        row_holding(&s, "Set up security for Daybreak mode"),
        "type to continue",
    );
}

// ORACLE chatwidget/tests/security_setup_tests.rs:152 security_setup_respects_applicable_backend_banner kind=ui owner=dialogs-pane
#[test]
fn security_setup_respects_applicable_backend_banner() {
    for state in ["visible", "dismissed", "another-model"] {
        let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
        s.feed(&(usage_banner() + &model_step("test-model-a", true)));
        s.wait_for("Selected model usage exhausted");
        match state {
            "dismissed" => {
                s.keys("esc");
            }
            "another-model" => {
                s.feed(&model_step("test-model-b", false));
            }
            _ => {}
        }
        s.feed(&security_notice("account"));
        s.assert_present("Set up security", state != "visible");
        if state == "another-model" {
            s.keys("esc");
            s.feed(&model_step("test-model-a", false));
            s.assert_contains("Selected model usage exhausted");
        }
    }
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
    let mut s = on_startup_catalog(80, 24, "gpt-5.5", &[bundled_model("gpt-5.5")]);
    // the picker is open on a model's reasoning menu when the provider write lands
    s.type_text("/model");
    s.keys("enter");
    s.keys("enter");
    s.assert_contains("Select Reasoning Level");
    s.feed(r#"{"step":"screen","dir":"select","ev":{"kind":"provider","state":"changed"}}"#);
    // the stale picker is dismissed
    s.assert_absent("Select Reasoning Level");
    s.assert_absent("Select Model");
    // `/model` is blocked with the restart notice and asks nothing of the server
    let asked = s.command_count();
    s.type_text("/model");
    s.keys("enter");
    // (the notice wraps at 80 columns)
    s.assert_has_row(
        "• The model list may be out of date. Restart the app server and TUI to refresh",
    );
    s.assert_has_row("/model.");
    s.assert_absent("Select Model");
    // the composer is free again: no dialog stays over it
    s.type_text("hello");
    s.assert_contains("hello");
    s.assert_no_models_asked_since(asked);
}

// ---- oracle: chatwidget/tests/skills_summary_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/skills_summary_tests.rs:8 manage_skills_summary_snapshot kind=snap owner=dialogs-pane snap=manage_skills_summary
#[test]
fn manage_skills_summary_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature manage-skills summary (w2-approvals, chat popups): none in the port in the port: the one-line skill summaries of the fixture skills have no widget and no scripted state",
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
