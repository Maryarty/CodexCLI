//! `/model` in the session page (oracle `chatwidget/model_popups.rs`, `slash_dispatch.rs`): the
//! quick menu or the whole list over the composer, the reasoning menu of a model over it and the
//! advanced one over that, the `Loading models…` page while the catalog is empty. The pages are
//! the routes of a nested navigator in a sheet (ADR-013): Esc shows the page under one as it was
//! (the cursor on the row that opened the page above) and the first page closes the sheet; a pick
//! writes the defaults and the sheet is gone. The state rules of the family are
//! `core/tests/models.rs`; the refresh and provider flows are `tests/oracle/chat_popups_4.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codex_tui_core::app::Command;
use codex_tui_core::screens::{ScreenCommand, select};

use crate::kit::{Owner, Screen};

/// The server's `model/list` answer of the bundled catalog (`tests/oracle/chat_popups_4.rs`).
const BUNDLED: &str = include_str!("../oracle/fixtures/chat_popups_4/bundled.json");

/// The models of a catalog text.
fn models_of(catalog: &str) -> Vec<serde_json::Value> {
    let catalog: serde_json::Value = serde_json::from_str(catalog.trim()).expect("a catalog");
    catalog["data"].as_array().expect("data").clone()
}

/// One model of the bundled catalog.
fn bundled_model(id: &str) -> serde_json::Value {
    models_of(BUNDLED)
        .into_iter()
        .find(|m| m["id"] == id)
        .expect("a bundled model")
}

/// An auto mode with the one effort `low`: `gpt-5.5` renamed.
fn auto_mode(id: &str, name: &str, description: &str) -> serde_json::Value {
    let mut model = bundled_model("gpt-5.5");
    model["id"] = id.into();
    model["model"] = id.into();
    model["displayName"] = name.into();
    model["description"] = description.into();
    model["defaultReasoningEffort"] = "low".into();
    model["supportedReasoningEfforts"] =
        serde_json::json!([{"reasoningEffort": "low", "description": "Fast"}]);
    model
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

const STATUS: &str = r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5"}}}"#;

/// A live chat on `gpt-5.5` that holds `startup` as its catalog; the server has not answered yet.
fn on_catalog(height: u16, startup: &[serde_json::Value]) -> Screen {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, height)
        .live()
        .open();
    let presets: Vec<_> = startup.iter().map(preset_of).collect();
    let seed = serde_json::json!({"step":"screen","dir":"select","ev":{"kind":"catalog","presets":presets}});
    s.feed(&format!("{seed}\n{STATUS}"));
    s
}

/// A live chat on `gpt-5.5` whose catalog is still empty: `/model` waits for the server.
fn without_catalog() -> Screen {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.feed(STATUS);
    s
}

/// `/model` is typed and sent.
fn open_models(s: &mut Screen) {
    s.type_text("/model");
    s.keys("enter");
}

/// The `model/list` the app has asked last, still unanswered.
fn last_model_list(s: &Screen) -> Command {
    s.commands()
        .into_iter()
        .rev()
        .find(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Models { .. }),
                    ..
                }
            )
        })
        .expect("the app asked for the model list")
}

/// The config keys the app wrote, one list per `config/batchWrite`, oldest first.
fn writes(s: &Screen) -> Vec<Vec<(&'static str, Option<String>)>> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Write { edits }),
                ..
            } => Some(edits.into_iter().map(|e| (e.key, e.value)).collect()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_model_with_efforts_opens_its_reasoning_menu_and_esc_returns_to_the_row_that_opened_it() {
    let mut s = on_catalog(
        24,
        &[bundled_model("gpt-5.5"), bundled_model("gpt-5.6-terra")],
    );
    open_models(&mut s);
    s.assert_contains("Select Model and Effort");
    // the cursor starts on the current model, one key below it is the other
    s.keys("down");
    s.assert_contains("› 2. GPT-5.6-Terra");
    let page = s.shot();
    s.keys("enter");
    s.assert_contains("Select Reasoning Level for GPT-5.6-Terra");
    s.keys("esc");
    s.assert_absent("Select Reasoning Level");
    s.assert_same_rows(&page);
    s.assert_contains("› 2. GPT-5.6-Terra");
    assert!(writes(&s).is_empty());
}

#[test]
fn esc_on_the_first_page_closes_the_sheet() {
    let mut s = on_catalog(24, &[bundled_model("gpt-5.5")]);
    open_models(&mut s);
    s.assert_contains("Select Model and Effort");
    s.keys("esc");
    s.assert_absent("Select Model");
    assert!(writes(&s).is_empty());
}

#[test]
fn the_advanced_page_is_one_esc_deeper_and_each_esc_shows_the_page_under_it_as_it_was() {
    let mut s = on_catalog(24, &[bundled_model("gpt-6.2-sol")]);
    open_models(&mut s);
    let models = s.shot();
    s.keys("enter");
    s.assert_contains("Select Reasoning Level for GPT-6.2-Sol");
    let reasoning = s.shot();
    // a tap on `More reasoning…` opens the advanced page over the reasoning menu
    s.click_text("More reasoning");
    s.assert_contains("Advanced Reasoning");
    s.assert_contains("Consumes usage limits faster");
    s.assert_contains("Ultra");
    s.keys("esc");
    s.assert_absent("Advanced Reasoning");
    s.assert_same_rows(&reasoning);
    s.keys("esc");
    s.assert_absent("Select Reasoning Level");
    s.assert_same_rows(&models);
    s.keys("esc");
    s.assert_absent("Select Model");
    assert!(writes(&s).is_empty());
}

#[test]
fn an_effort_pick_writes_the_defaults_and_closes_the_sheet() {
    let mut s = on_catalog(24, &[bundled_model("gpt-5.5")]);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5","effort":"high"}}}"#);
    open_models(&mut s);
    s.keys("enter");
    // the cursor starts on the current effort
    s.assert_contains("› 3. High (current)");
    s.keys("down enter");
    assert_eq!(
        writes(&s),
        [[
            ("model", Some("gpt-5.5".to_owned())),
            ("model_reasoning_effort", Some("xhigh".to_owned())),
        ]]
    );
    s.assert_absent("Select Reasoning Level");
    s.assert_absent("Select Model");
}

#[test]
fn the_s_key_picks_for_this_session_only_and_writes_nothing() {
    let mut s = on_catalog(24, &[bundled_model("gpt-5.5")]);
    open_models(&mut s);
    s.keys("enter");
    s.assert_contains("s session");
    s.keys("s");
    assert!(writes(&s).is_empty(), "{:?}", writes(&s));
    s.assert_absent("Select Reasoning Level");
    s.assert_absent("Select Model");
}

#[test]
fn a_tap_on_a_model_opens_its_reasoning_menu_and_a_tap_on_an_effort_picks_it() {
    let mut s = on_catalog(
        24,
        &[bundled_model("gpt-5.5"), bundled_model("gpt-5.6-terra")],
    );
    open_models(&mut s);
    s.click_text("GPT-5.6-Terra");
    s.assert_contains("Select Reasoning Level for GPT-5.6-Terra");
    s.click_text("Extra high");
    assert_eq!(
        writes(&s),
        [[
            ("model", Some("gpt-5.6-terra".to_owned())),
            ("model_reasoning_effort", Some("xhigh".to_owned())),
        ]]
    );
    s.assert_absent("Select Reasoning Level");
    s.assert_absent("Select Model");
}

#[test]
fn the_quick_menu_names_the_auto_modes_and_a_tap_on_all_models_replaces_it_with_the_list() {
    let mut s = on_catalog(
        24,
        &[
            auto_mode("codex-auto-fast", "Auto fast", "Quick answers"),
            bundled_model("gpt-5.5"),
        ],
    );
    open_models(&mut s);
    s.assert_in_order(&[
        "Select Model",
        "Pick a quick auto mode or browse all models.",
        "Auto fast",
        "All models",
        "Choose a specific model",
    ]);
    s.click_text("All models");
    s.assert_contains("Select Model and Effort");
    s.assert_absent("Pick a quick auto mode");
    // the list took the quick menu's place: Esc closes the sheet
    s.keys("esc");
    s.assert_absent("Select Model");
    assert!(writes(&s).is_empty());
}

#[test]
fn a_tap_on_an_auto_mode_with_one_effort_picks_it_at_once() {
    let mut s = on_catalog(
        24,
        &[
            auto_mode("codex-auto-fast", "Auto fast", "Quick answers"),
            bundled_model("gpt-5.5"),
        ],
    );
    open_models(&mut s);
    s.click_text("Auto fast");
    assert_eq!(
        writes(&s),
        [[
            ("model", Some("codex-auto-fast".to_owned())),
            ("model_reasoning_effort", Some("low".to_owned())),
        ]]
    );
    s.assert_absent("Select Model");
}

#[test]
fn the_wheel_scrolls_a_long_model_list_and_a_tap_opens_the_reasoning_menu_of_a_row_it_brought_in() {
    let mut s = on_catalog(14, &models_of(BUNDLED));
    open_models(&mut s);
    // the cursor is on the current model at the bottom of the list: the first rows are above
    s.assert_contains("GPT-5.5 (current)");
    s.assert_absent("GPT-6.2-Sol");
    // the picker sits at the bottom of the 80x14 screen: row 8 is one of its rows
    s.wheel_up(10, 8, 20);
    s.assert_contains("GPT-6.2-Sol");
    s.click_text("GPT-6.2-Sol");
    s.assert_contains("Select Reasoning Level for GPT-6.2-Sol");
}

#[test]
fn loading_models_gives_way_to_the_list_the_server_answers() {
    let mut s = without_catalog();
    open_models(&mut s);
    s.assert_contains("Loading models…");
    let ask = last_model_list(&s);
    s.answer_request(
        &ask,
        serde_json::from_str(BUNDLED.trim()).expect("the bundled list"),
    );
    s.assert_absent("Loading models…");
    s.assert_contains("Select Model and Effort");
    s.assert_contains("GPT-5.5");
    // the page that took the loading page's place is the first one: Esc closes the sheet
    s.keys("esc");
    s.assert_absent("Select Model");
}

#[test]
fn a_list_the_server_refuses_closes_the_loading_page_and_the_history_says_so() {
    let mut s = without_catalog();
    open_models(&mut s);
    s.assert_contains("Loading models…");
    let ask = last_model_list(&s);
    s.answer_error(&ask, "model/list failed: 500 Internal Server Error");
    s.assert_absent("Loading models…");
    s.assert_contains("Could not load models. Try /model again.");
}

#[test]
fn esc_on_the_loading_page_closes_it_and_a_late_refusal_says_nothing() {
    let mut s = without_catalog();
    open_models(&mut s);
    s.assert_contains("Loading models…");
    s.keys("esc");
    s.assert_absent("Loading models…");
    let ask = last_model_list(&s);
    s.answer_error(&ask, "model/list failed: 500 Internal Server Error");
    // the user left the page: the failure is not told to them
    s.assert_absent("Could not load models");
}
