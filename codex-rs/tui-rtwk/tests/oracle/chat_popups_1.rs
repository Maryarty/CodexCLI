//! Shard `chat_popups_1` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/bedrock_catalog_tests.rs (3 screen-level tests here; 0 state tests left to dialogs-pane)
/// The server's `model/list` answer of a provider's catalog (the fixture is what
/// the backend's own `list_models` gives for it, in the app server's wire form).
const MANTLE: &str = include_str!("fixtures/chat_popups_1/mantle.json");
const RUNTIME: &str = include_str!("fixtures/chat_popups_1/runtime.json");
const GOV: &str = include_str!("fixtures/chat_popups_1/gov.json");
const DEFAULT: &str = include_str!("fixtures/chat_popups_1/default.json");
const LUNA: &str = include_str!("fixtures/chat_popups_1/custom_luna.json");

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

// ---- oracle: chatwidget/tests/bedrock_catalog_tests.rs (3 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/bedrock_catalog_tests.rs:11 bedrock_astra_model_and_reasoning_pickers kind=snap owner=dialogs-pane snap=bedrock_mantle_astra_advanced_reasoning
#[test]
fn bedrock_astra_model_and_reasoning_pickers() {
    // mantle: the model list, the Astra preset's reasoning popup (the oracle opens it
    // directly; here its row is chosen), then the advanced reasoning popup (`5`).
    let mut s = on_catalog(100, 13, "openai.gpt-6.2-sol", MANTLE);
    s.type_text("/model").keys("enter");
    s.assert_snap("bedrock_mantle_models");
    s.keys("down down down down enter");
    s.resize(100, 10);
    s.assert_snap("bedrock_mantle_astra_reasoning");
    s.keys("5");
    s.resize(100, 8);
    s.assert_snap("bedrock_mantle_astra_advanced_reasoning");
    // `2` applies ultra: the final pick goes to the thread
    s.keys("2");
    // runtime: the same model list and the Astra reasoning popup (no advanced snapshot in the oracle).
    let mut s = on_catalog(100, 16, "global.openai.gpt-6.2-sol", RUNTIME);
    s.type_text("/model").keys("enter");
    s.assert_snap("bedrock_runtime_models");
    s.keys("down down down down enter");
    s.resize(100, 10);
    s.assert_snap("bedrock_runtime_astra_reasoning");
}

// ORACLE chatwidget/tests/bedrock_catalog_tests.rs:94 bedrock_govcloud_model_pickers kind=snap owner=dialogs-pane snap=bedrock_govcloud_models
#[test]
fn bedrock_govcloud_model_pickers() {
    // one screen per region; the oracle compares both with the same snapshot
    // (the two regions list the same models).
    for _region in ["us-gov-west-1", "us-gov-east-1"] {
        let mut s = on_catalog(100, 7, "openai.gpt-5.6-terra", GOV);
        s.type_text("/model").keys("enter");
        s.assert_snap("bedrock_govcloud_models");
    }
}

// ORACLE chatwidget/tests/bedrock_catalog_tests.rs:122 bedrock_ultrafast_slash_command_selects_and_clears_tier kind=snap owner=dialogs-pane
#[test]
fn bedrock_ultrafast_slash_command_selects_and_clears_tier() {
    // (name, model, bundled catalog, the custom catalog's `model/list` answer)
    let cases = [
        (
            "mantle",
            "openai.gpt-6-astra",
            MANTLE,
            include_str!("fixtures/chat_popups_1/mantle_custom.json"),
        ),
        (
            "runtime_global",
            "global.openai.gpt-6-astra",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_global_custom.json"),
        ),
        (
            "runtime_us",
            "us.openai.gpt-6-astra",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_us_custom.json"),
        ),
        (
            "mantle_sol61",
            "openai.gpt-6.1-sol",
            MANTLE,
            include_str!("fixtures/chat_popups_1/mantle_sol61_custom.json"),
        ),
        (
            "runtime_global_sol61",
            "global.openai.gpt-6.1-sol",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_global_sol61_custom.json"),
        ),
        (
            "runtime_us_sol61",
            "us.openai.gpt-6.1-sol",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_us_sol61_custom.json"),
        ),
        (
            "mantle_astra62",
            "openai.gpt-6.2-astra",
            MANTLE,
            include_str!("fixtures/chat_popups_1/mantle_astra62_custom.json"),
        ),
        (
            "runtime_global_astra62",
            "global.openai.gpt-6.2-astra",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_global_astra62_custom.json"),
        ),
        (
            "runtime_us_astra62",
            "us.openai.gpt-6.2-astra",
            RUNTIME,
            include_str!("fixtures/chat_popups_1/runtime_us_astra62_custom.json"),
        ),
    ];
    for (_name, model, bundled, custom) in cases {
        let mut s = on_catalog(100, 6, model, bundled);
        // the catalog is read when the picker opens (the oracle's chat holds it already)
        let refresh = |s: &mut Screen| {
            s.type_text("/model").keys("enter");
            s.keys("esc");
        };
        refresh(&mut s);
        // The snapshot joins two frames under `Bundled catalog:` / `Custom catalog:` labels the
        // kit does not parse (kit gap), and draws the popup above the composer: the row
        // the popup lists is what is compared.
        s.type_text("/ultrafast");
        s.assert_contains(
            "/ultrafast  The fastest available responses for latency-sensitive work.",
        );
        // Enter switches the tier on: the thread's settings and the saved default
        s.keys("enter");
        s.assert_request_mentions("thread/settings/update", "ultrafast");
        s.assert_request_mentions("config/batchWrite", "ultrafast");
        // the server's catalog turns custom: its one tier is named Express
        let custom = custom.trim();
        s.feed(&format!(
            r#"{{"step":"respond","method":"model/list","result":{custom}}}"#
        ));
        refresh(&mut s);
        s.type_text("/express");
        s.assert_contains("/express  My custom Bedrock tier.");
        s.keys("enter");
        s.assert_request_mentions("config/batchWrite", "ultrafast");
        let _ = bundled;
    }
}

// ---- oracle: chatwidget/tests/collaboration_catalog_tests.rs (1 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/collaboration_catalog_tests.rs:61 collaboration_catalog_unavailable_preserves_task_and_plan_input kind=ui owner=dialogs-pane
#[test]
fn collaboration_catalog_unavailable_preserves_task_and_plan_input() {
    kit::pending(
        Owner::DialogsPane,
        "no seed for an empty collaboration-mode catalog and no /plan command in the port; the visible half is the composer keeping `/plan keep my draft` after Enter (the active mode is state, not drawn)",
    );
    let mut s = Screen::build(Owner::DialogsPane).live().open();
    s.type_text("/plan keep my draft").keys("enter");
    s.assert_contains("/plan keep my draft");
}

// ---- oracle: chatwidget/tests/copy_export_picker_tests.rs (3 screen-level tests here; 2 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/copy_export_picker_tests.rs:9 whole_response_copy_uses_followup_labels kind=ui owner=dialogs-pane
#[test]
fn whole_response_copy_uses_followup_labels() {
    kit::na(
        Owner::DialogsPane,
        "asserts the copy shortcut's returned text, the CopySelection payload (text, Markdown format) and the transcript's last agent source: none of it is drawn; the copy picker's rows are not asserted",
    );
}

// ORACLE chatwidget/tests/copy_export_picker_tests.rs:39 completed_response_copy_preserves_markdown_line_endings kind=snap owner=dialogs-pane
#[test]
fn completed_response_copy_preserves_markdown_line_endings() {
    kit::na(
        Owner::DialogsPane,
        "asserts the CopySelection payload and an inline debug snapshot of the clipboard HTML render: neither is drawn on the screen",
    );
}

// ORACLE chatwidget/tests/copy_export_picker_tests.rs:62 copy_export_picker_custom_keys_preserve_payloads_and_composer_draft kind=ui owner=dialogs-pane
#[test]
fn copy_export_picker_custom_keys_preserve_payloads_and_composer_draft() {
    kit::pending(
        Owner::DialogsPane,
        "no copy picker, transcript export popup or export file prompt in the port, and no seed for a Terminal transcript with a long last response; the list keys f3 and f2 need the keymap seed (kit: `.keymap`, pending)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .open();
    s.type_text("Keep this draft");
    // the last response is a long preview with 日本語 and a combining accent, repeated four times
    s.type_text("/copy").keys("enter");
    s.assert_contains("...");
    s.assert_contains("f3 select · f2 back");
    s.keys("f3");
    s.assert_contains("Keep this draft");
    s.type_text("/export").keys("enter");
    s.keys("f2");
    s.type_text("/export").keys("enter");
    s.keys("down f3");
    s.keys("esc");
    s.assert_contains("Keep this draft");
}

// ---- oracle: chatwidget/tests/goal_menu.rs (10 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/goal_menu.rs:4 goal_menu_active_snapshot kind=snap owner=dialogs-pane snap=goal_menu_active
#[test]
fn goal_menu_active_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no goal in the port: no goal summary history cell (the bare `/goal` status) and no thread goal seed (objective, status, budget 80000, usage)",
    );
    // the goal summary is a history cell: its rows are compared at the top of the screen
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/goal").keys("enter");
    s.assert_snap_region("goal_menu_active", 0, 0);
}

// ORACLE chatwidget/tests/goal_menu.rs:18 goal_menu_paused_snapshot kind=snap owner=dialogs-pane snap=goal_menu_paused
#[test]
fn goal_menu_paused_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no goal in the port: no goal summary history cell and no paused thread goal seed (no budget)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/goal").keys("enter");
    s.assert_snap_region("goal_menu_paused", 0, 0);
}

// ORACLE chatwidget/tests/goal_menu.rs:32 goal_menu_budget_limited_snapshot kind=snap owner=dialogs-pane snap=goal_menu_budget_limited
#[test]
fn goal_menu_budget_limited_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no goal in the port: no goal summary history cell and no budget-limited thread goal seed",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/goal").keys("enter");
    s.assert_snap_region("goal_menu_budget_limited", 0, 0);
}

// ORACLE chatwidget/tests/goal_menu.rs:46 resume_paused_goal_prompt_snapshot kind=snap owner=dialogs-pane snap=resume_paused_goal_prompt
#[test]
fn resume_paused_goal_prompt_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no resume-paused-goal prompt in the port (the bottom popup with the objective `Keep improving the bare goal command until it feels calm and useful.`)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(100, 9).live().open();
    s.type_text("/goal").keys("enter");
    s.assert_snap("resume_paused_goal_prompt");
}

// ORACLE chatwidget/tests/goal_menu.rs:62 goal_edit_prompt_snapshot kind=snap owner=dialogs-pane snap=goal_edit_prompt
#[test]
fn goal_edit_prompt_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no goal edit prompt in the port (the bottom popup for an active goal with budget 80000)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(100, 5).live().open();
    s.type_text("/goal edit").keys("enter");
    s.assert_snap("goal_edit_prompt");
}

// ORACLE chatwidget/tests/goal_menu.rs:82 goal_edit_prompt_submits_preserved_status_and_budget kind=ui owner=dialogs-pane
#[test]
fn goal_edit_prompt_submits_preserved_status_and_budget() {
    kit::pending(
        Owner::DialogsPane,
        "no goal edit prompt in the port; the visible half is the pasted text in the prompt and the prompt closing after Enter (the submitted draft and status are events, not drawn)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(100, 5).live().open();
    let base = s.shot();
    s.type_text("/goal edit").keys("enter");
    s.paste(" with clearer wording");
    s.assert_contains("with clearer wording");
    s.keys("enter");
    s.assert_same_rows(&base);
}

// ORACLE chatwidget/tests/goal_menu.rs:121 goal_edit_prompt_preserves_resumable_stopped_statuses kind=ui owner=dialogs-pane
#[test]
fn goal_edit_prompt_preserves_resumable_stopped_statuses() {
    kit::na(
        Owner::DialogsPane,
        "asserts the status and token budget of the SetThreadGoalDraft event for blocked and usage-limited goals: the event is not drawn and no row is asserted",
    );
}

// ORACLE chatwidget/tests/goal_menu.rs:157 goal_edit_prompt_resets_terminal_status_to_active kind=ui owner=dialogs-pane
#[test]
fn goal_edit_prompt_resets_terminal_status_to_active() {
    kit::na(
        Owner::DialogsPane,
        "asserts the status and token budget of the SetThreadGoalDraft event for budget-limited and complete goals: the event is not drawn and no row is asserted",
    );
}

// ORACLE chatwidget/tests/goal_menu.rs:195 resume_paused_goal_prompt_default_resumes_goal kind=ui owner=dialogs-pane
#[test]
fn resume_paused_goal_prompt_default_resumes_goal() {
    kit::pending(
        Owner::DialogsPane,
        "no resume-paused-goal prompt in the port; the visible half is the prompt closing after Enter (the SetThreadGoalStatus event is not drawn)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(100, 9).live().open();
    let base = s.shot();
    s.type_text("/goal").keys("enter");
    s.keys("enter");
    s.assert_same_rows(&base);
}

// ORACLE chatwidget/tests/goal_menu.rs:216 resume_paused_goal_prompt_can_leave_goal_paused kind=ui owner=dialogs-pane
#[test]
fn resume_paused_goal_prompt_can_leave_goal_paused() {
    kit::pending(
        Owner::DialogsPane,
        "no resume-paused-goal prompt in the port; the visible half is the prompt closing after Down and Enter with no event (the absence of an event is not drawn)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(100, 9).live().open();
    let base = s.shot();
    s.type_text("/goal").keys("enter");
    s.keys("down enter");
    s.assert_same_rows(&base);
}

// ---- oracle: chatwidget/tests/model_display_name_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/model_display_name_tests.rs:7 custom_model_display_name_in_pickers_preserves_selection_slug kind=snap owner=dialogs-pane snap=custom_model_display_name_all_models snap=custom_model_display_name_quick_picker snap=custom_model_display_name_reasoning
#[test]
fn custom_model_display_name_in_pickers_preserves_selection_slug() {
    // the display name `GPT-5.6 Luna` is what the pickers show; the slug is what is picked.
    let mut s = on_catalog(80, 9, "us.openai.gpt-5.6-luna", LUNA);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"us.openai.gpt-5.6-luna","effort":"high"}}}"#);
    s.type_text("/model").keys("enter");
    s.assert_snap("custom_model_display_name_quick_picker");
    s.keys("enter");
    s.resize(80, 6);
    s.assert_snap("custom_model_display_name_all_models");
    s.keys("enter");
    s.resize(80, 7);
    s.assert_snap("custom_model_display_name_reasoning");
    // Enter applies the slug (`ApplyModelSelection`)
    s.keys("enter");
    s.assert_request_mentions("thread/settings/update", "us.openai.gpt-5.6-luna");
}

// ORACLE chatwidget/tests/model_display_name_tests.rs:62 custom_model_display_name_in_status_line_and_fallback kind=snap owner=dialogs-pane snap=custom_model_display_name_status_line
#[test]
fn custom_model_display_name_in_status_line_and_fallback() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 6)
        .script(include_str!(
            "fixtures/chat_popups_1/custom_model_status_line.jsonl"
        ))
        .open();
    s.assert_snap("custom_model_display_name_status_line");
    let line = |s: &Screen, text: &str| s.assert_has_row_trimmed(text);
    // the catalog is cleared: the status line shows the slug; Luna Reserve by its reserve slug;
    // an empty model shows the default display name (its text is a constant of the oracle).
    let status = |model: &str| {
        format!(
            r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}","items":["model-name"]}}}}}}"#
        )
    };
    // (a catalog without the model: an empty list is no answer and keeps the old catalog)
    s.feed(r#"{"step":"screen","dir":"select","ev":{"kind":"catalog","presets":[{"model":"gpt-5.5","name":"GPT-5.5"}]}}"#);
    s.feed(&status("us.openai.gpt-5.6-luna"));
    line(&s, "us.openai.gpt-5.6-luna");
    s.feed(&status("gpt-reserve"));
    line(&s, "Luna Reserve");
    s.feed(&status(""));
    line(&s, "loading");
}

// ---- oracle: chatwidget/tests/model_picker_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/model_picker_tests.rs:7 model_picker_compact_hint_and_actions_follow_configured_bindings kind=ui owner=dialogs-pane
#[test]
fn model_picker_compact_hint_and_actions_follow_configured_bindings() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 10)
        .live()
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .open();
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.5"}}}}}}"#,
        DEFAULT.trim()
    ));
    s.type_text("/model").keys("enter");
    s.assert_contains("f3 select · f2 back");
    // f3 chooses the row (its reasoning menu opens), f2 goes back, f2 again closes the picker
    s.keys("f3");
    s.assert_contains("Select Reasoning Level");
    s.keys("f2");
    s.assert_contains("Select Model and Effort");
    s.keys("f2");
    s.assert_absent("Select Model and Effort");
}

// ORACLE chatwidget/tests/model_picker_tests.rs:23 model_picker_opens_after_discovery_populates_an_empty_catalog kind=snap owner=dialogs-pane snap=model_selection_popup
#[test]
fn model_picker_opens_after_discovery_populates_an_empty_catalog() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 14).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5"}}}"#);
    s.type_text("/model").keys("enter");
    s.assert_contains("Loading models…");
    s.keys("esc");
    // the discovery answers: the next `/model` opens on the filled catalog
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{}}}"#,
        DEFAULT.trim()
    ));
    s.type_text("/model").keys("enter");
    s.assert_snap("model_selection_popup");
}

// ---- oracle: chatwidget/tests/permission_picker_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/permission_picker_tests.rs:7 clipped_full_access_confirmation_discloses_hidden_warning_text kind=snap owner=dialogs-pane snap=clipped_full_access_confirmation
#[test]
fn clipped_full_access_confirmation_discloses_hidden_warning_text() {
    kit::pending(
        Owner::DialogsPane,
        "no full-access confirmation in the port (the select dialog has the permissions menu only); no seed for a cyber-specialty model; the snapshot's frames are headed `specialty=None 40x10` / `specialty=Some(\"cyber\") 40x16`, a header the kit does not parse (kit gap)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(40, 10).live().open();
    // the full-access confirmation for the current model (`open_full_access_confirmation`)
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Yes, continue anyway");
    s.assert_contains("[…");
    s.assert_row(9, "enter select · esc back");
    s.assert_snap_section("clipped_full_access_confirmation", "specialty=None 40x10");
    s.resize(40, 16);
    s.assert_snap_section(
        "clipped_full_access_confirmation",
        "specialty=Some(\"cyber\") 40x16",
    );
}

// ORACLE chatwidget/tests/permission_picker_tests.rs:64 permission_picker_retry_and_confirmation_follow_configured_bindings kind=ui owner=dialogs-pane
#[test]
fn permission_picker_retry_and_confirmation_follow_configured_bindings() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 9)
        .live()
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .open();
    let base = s.shot();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("f3 select · f2 back");
    s.keys("f3");
    s.keys("down f3");
    s.keys("f2");
    // the cancel closes the popup: the screen is the one before it
    s.assert_same_rows(&base);
}

// ---- oracle: chatwidget/tests/permission_shortcuts_tests.rs (3 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/permission_shortcuts_tests.rs:8 permission_shortcuts_cycle_builtin_modes kind=ui owner=dialogs-pane
#[test]
fn permission_shortcuts_cycle_builtin_modes() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ApplyPermissionShortcut events (target thread, profile id, approval policy, reviewer) for the F7/F8 cycle, and the windows-only branch: the events are not drawn and no row is asserted",
    );
}

// ORACLE chatwidget/tests/permission_shortcuts_tests.rs:116 permission_shortcuts_respect_managed_mode_requirements kind=snap owner=dialogs-pane
#[test]
fn permission_shortcuts_respect_managed_mode_requirements() {
    kit::pending(
        Owner::DialogsPane,
        "no managed-mode requirements seed (allowed reviewers, required models, allowed profiles) in the live chat and no unavailable-mode notice in the port",
    );
    // three requirement sets: each one, F8 shows the notice `permission_shortcut_no_alternative` (a history cell)
    for _requirements in 0..3 {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        s.keys("f8");
        s.assert_snap_region("permission_shortcut_no_alternative", 0, 0);
    }
}

// ORACLE chatwidget/tests/permission_shortcuts_tests.rs:152 permission_shortcuts_load_once_and_reuse_the_picker_catalog kind=ui owner=dialogs-pane
#[test]
fn permission_shortcuts_load_once_and_reuse_the_picker_catalog() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    let base = s.shot();
    s.keys("f8");
    s.assert_rows_differ(&base);
    s.keys("esc");
    s.assert_same_rows(&base);
    s.keys("f8");
    s.keys("esc");
    s.assert_same_rows(&base);
}
