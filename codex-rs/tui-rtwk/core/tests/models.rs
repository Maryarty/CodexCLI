#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the `/model` family of S14: the menu stack, the catalog and
//! the rules of `chatwidget/model_popups.rs` and `model_popup_state.rs` (a
//! choice opens the next menu or is final, Esc pops one menu, a catalog
//! refresh keeps what is open and drops what has nothing left to offer).
//! `core/tests/model.rs` is the app model's; this file is the picker's.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::select::models::{
    ALL_MODELS, ALL_MODES, Apply, Current, Effort, Heading, Kind, MORE_REASONING, ModelMenu,
    PLAN_ONLY, PlanMode, PlanSource, Preset,
};
use codex_tui_core::screens::select::{Ev, Intent as SelectIntent, Menu, Note, Picked, Selection};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use pretty_assertions::assert_eq;
use serde_json::json;

/// Seven models: three auto modes (one advertises Max), the oracle's
/// `gpt-5.5` with its five standard efforts, a model with one effort, a model
/// with only advanced efforts, and one the picker hides.
fn presets() -> Vec<Preset> {
    let list = json!([
        {"model": "codex-auto-thorough", "name": "Auto thorough", "description": "Deep",
         "default_effort": "high", "efforts": [{"effort": "high"}, {"effort": "max"}]},
        {"model": "codex-auto-balanced", "name": "Auto balanced", "description": "Balanced",
         "default_effort": "medium", "efforts": [{"effort": "medium"}]},
        {"model": "codex-auto-fast", "name": "Auto fast", "description": "Fast",
         "default_effort": "low", "efforts": [{"effort": "low"}]},
        {"model": "gpt-5.5", "name": "GPT-5.5", "description": "Legacy coding model.",
         "default_effort": "medium", "efforts": [
            {"effort": "low", "description": "Fast responses"},
            {"effort": "medium", "description": "Balances speed"},
            {"effort": "high", "description": "Greater depth"},
            {"effort": "xhigh", "description": "Extra depth"},
            {"effort": "persistent", "description": "Until put to sleep"}]},
        {"model": "gpt-sol", "name": "GPT-Sol", "description": "One effort",
         "default_effort": "high", "efforts": [{"effort": "high"}]},
        {"model": "gpt-astra", "name": "GPT-Astra", "description": "Advanced only",
         "default_effort": "max", "efforts": [{"effort": "max"}, {"effort": "ultra"}]},
        {"model": "gpt-hidden", "name": "Hidden", "show_in_picker": false,
         "default_effort": "medium", "efforts": [{"effort": "medium"}]},
    ]);
    serde_json::from_value(list).expect("presets")
}

fn sandbox(catalog: Vec<Preset>) -> Sandbox {
    let mut sandbox = Sandbox::play("").expect("plays");
    if !catalog.is_empty() {
        event(&mut sandbox, Ev::Loaded(catalog));
    }
    sandbox
}

fn event(sandbox: &mut Sandbox, ev: Ev) {
    sandbox.send(AppEv::Screen(ScreenEv::Select(ev)));
}

fn intent(sandbox: &mut Sandbox, intent: SelectIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))));
}

fn choose(sandbox: &mut Sandbox, id: &str) {
    intent(sandbox, SelectIntent::Choose(id.into()));
}

fn model(sandbox: &Sandbox) -> &codex_tui_core::screens::select::Model {
    &sandbox.app().screens().select
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    let page = model(sandbox).page().expect("a model menu");
    page.rows.into_iter().map(|row| row.id).collect()
}

fn depth(sandbox: &Sandbox) -> usize {
    model(sandbox).stack.len()
}

fn open(sandbox: &mut Sandbox, current: Current) {
    intent(sandbox, SelectIntent::OpenModels(current));
}

fn picked(sandbox: &Sandbox) -> Vec<Selection> {
    model(sandbox)
        .picked
        .iter()
        .map(|picked| match picked {
            Picked::Selection(selection) => selection.clone(),
            other => panic!("a selection, not {other:?}"),
        })
        .collect()
}

fn selection(model: &str, effort: Effort, apply: Apply) -> Selection {
    Selection {
        model: model.into(),
        effort: Some(effort),
        apply,
        warning: None,
    }
}

fn head(sandbox: &Sandbox) -> &ModelMenu {
    match model(sandbox).menu() {
        Some(Menu::Model(menu)) => menu,
        other => panic!("a model menu, not {other:?}"),
    }
}

fn all_models(sandbox: &mut Sandbox) {
    open(sandbox, Current::new("gpt-5.5"));
    choose(sandbox, ALL_MODELS);
}

// -- opening, loading, requests

#[test]
fn an_empty_catalog_opens_the_loading_menu_and_asks_for_the_models() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    assert!(matches!(head(&s), ModelMenu::Loading(_)));
    assert_eq!(model(&s).page().expect("page").heading, Heading::Loading);
    assert!(model(&s).catalog.pending.is_some());
}

#[test]
fn the_answer_replaces_the_loading_menu_with_the_quick_menu() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let request = model(&s).catalog.pending.expect("asked");
    event(
        &mut s,
        Ev::Fetched {
            request,
            result: Ok(presets()),
        },
    );
    assert_eq!(depth(&s), 1);
    assert!(matches!(head(&s), ModelMenu::Models(_)));
    assert_eq!(model(&s).catalog.presets, presets());
    assert_eq!(model(&s).catalog.pending, None);
}

#[test]
fn a_failed_answer_drops_the_loading_menu_with_a_note() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let request = model(&s).catalog.pending.expect("asked");
    event(
        &mut s,
        Ev::Fetched {
            request,
            result: Err("down".into()),
        },
    );
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, vec![Note::ModelsNotLoaded]);
}

#[test]
fn an_answer_of_an_older_request_is_dropped() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let old = model(&s).catalog.pending.expect("asked");
    intent(&mut s, SelectIntent::Close);
    open(&mut s, Current::new("gpt-5.5"));
    let current = model(&s).catalog.pending.expect("asked again");
    assert_ne!(old, current);
    event(
        &mut s,
        Ev::Fetched {
            request: old,
            result: Ok(presets()),
        },
    );
    assert!(model(&s).catalog.presets.is_empty());
    assert!(matches!(head(&s), ModelMenu::Loading(_)));
    assert_eq!(model(&s).catalog.pending, Some(current));
}

#[test]
fn an_empty_or_unchanged_answer_changes_nothing() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    let before = ids(&s);
    for result in [Ok(Vec::new()), Ok(presets()), Err("down".to_string())] {
        let request = model(&s).catalog.pending.expect("asked");
        event(&mut s, Ev::Fetched { request, result });
        assert_eq!(model(&s).catalog.presets, presets());
        assert_eq!(ids(&s), before);
        assert!(model(&s).notes.is_empty());
        open(&mut s, Current::new("gpt-5.5"));
    }
}

// -- the lists

#[test]
fn the_quick_menu_lists_the_auto_modes_in_order_then_all_models() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    assert_eq!(
        ids(&s),
        [
            "codex-auto-fast",
            "codex-auto-balanced",
            "codex-auto-thorough",
            ALL_MODELS
        ]
    );
    let page = model(&s).page().expect("page");
    assert_eq!(page.heading, Heading::Quick);
    // None of the auto modes is in use: "All models" is the current row.
    let current: Vec<_> = page
        .rows
        .iter()
        .filter(|row| row.current)
        .map(|row| &row.id)
        .collect();
    assert_eq!(current, [ALL_MODELS]);
}

#[test]
fn the_auto_mode_in_use_is_the_current_row() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("codex-auto-balanced"));
    let page = model(&s).page().expect("page");
    let current: Vec<_> = page
        .rows
        .iter()
        .filter(|row| row.current)
        .map(|row| &row.id)
        .collect();
    assert_eq!(current, ["codex-auto-balanced"]);
}

#[test]
fn without_an_auto_mode_the_quick_menu_is_the_list_of_all_models() {
    let others: Vec<Preset> = presets().into_iter().filter(|p| !p.is_auto()).collect();
    let mut s = sandbox(others);
    open(&mut s, Current::new("gpt-5.5"));
    assert_eq!(ids(&s), ["gpt-5.5", "gpt-sol", "gpt-astra"]);
    assert_eq!(model(&s).page().expect("page").heading, Heading::All);
}

#[test]
fn models_the_picker_hides_are_in_no_list() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    assert!(!ids(&s).contains(&"gpt-hidden".to_string()));
    assert_eq!(ids(&s), ["gpt-5.5", "gpt-sol", "gpt-astra"]);
}

#[test]
fn all_models_replaces_the_quick_menu() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    assert_eq!(depth(&s), 1);
    assert!(matches!(head(&s), ModelMenu::AllModels(_)));
    assert_eq!(model(&s).page().expect("page").heading, Heading::All);
}

#[test]
fn an_all_models_row_has_the_session_key_only_with_one_standard_effort() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    let rows = model(&s).page().expect("page").rows;
    let session: Vec<_> = rows
        .iter()
        .map(|row| (row.id.as_str(), row.session))
        .collect();
    assert_eq!(
        session,
        [("gpt-5.5", false), ("gpt-sol", true), ("gpt-astra", false)]
    );
}

// -- the reasoning menu

#[test]
fn a_model_with_several_efforts_opens_its_menu_above_the_list_and_esc_comes_back() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    assert_eq!(depth(&s), 2);
    assert!(matches!(head(&s), ModelMenu::Reasoning { .. }));
    intent(&mut s, SelectIntent::Close);
    assert_eq!(depth(&s), 1);
    assert!(matches!(head(&s), ModelMenu::AllModels(_)));
    assert!(model(&s).picked.is_empty());
}

#[test]
fn a_model_with_one_effort_is_picked_without_a_second_menu() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-sol");
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("gpt-sol", Effort::High, Apply::Selection)]
    );
}

#[test]
fn an_effort_pick_is_final_and_closes_every_menu_below_it() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    choose(&mut s, "high");
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("gpt-5.5", Effort::High, Apply::Selection)]
    );
}

#[test]
fn the_reasoning_menu_marks_the_default_and_the_current_effort() {
    let mut s = sandbox(presets());
    let mut current = Current::new("gpt-5.5");
    current.effort = Some(Effort::High);
    open(&mut s, current);
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    let page = model(&s).page().expect("page");
    assert_eq!(
        page.heading,
        Heading::Reasoning {
            model: "GPT-5.5".into()
        }
    );
    assert_eq!(ids(&s), ["low", "medium", "high", "xhigh", "persistent"]);
    // The cursor starts on the current effort.
    assert_eq!(page.initial, Some(2));
    let flags: Vec<_> = page
        .rows
        .iter()
        .map(|row| match &row.kind {
            Kind::Effort { default, .. } => (*default, row.current),
            other => panic!("an effort row, not {other:?}"),
        })
        .collect();
    assert_eq!(
        flags,
        [
            (false, false),
            (true, false),
            (false, true),
            (false, false),
            (false, false)
        ]
    );
}

#[test]
fn another_model_starts_on_its_default_effort_and_marks_none_current() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-sol"));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    let page = model(&s).page().expect("page");
    assert_eq!(page.initial, Some(1));
    assert!(page.rows.iter().all(|row| !row.current));
}

#[test]
fn a_high_effort_of_the_codex_models_warns_on_the_highlighted_row() {
    let mut codex = presets();
    codex[3].model = "gpt-5.1-codex-max".into();
    let mut s = sandbox(codex);
    open(&mut s, Current::new("gpt-5.5"));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.1-codex-max");
    let warns: Vec<_> = model(&s)
        .page()
        .expect("page")
        .rows
        .iter()
        .filter_map(|row| match &row.kind {
            Kind::Effort { warns: true, .. } => Some(row.id.clone()),
            _ => None,
        })
        .collect();
    // The extra high effort when the model has it, not the high one.
    assert_eq!(warns, ["xhigh"]);
}

#[test]
fn advanced_efforts_sit_behind_a_more_reasoning_row() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-astra");
    // Only advanced efforts: the menu has the one row.
    assert_eq!(depth(&s), 2);
    assert_eq!(ids(&s), [MORE_REASONING]);
    choose(&mut s, MORE_REASONING);
    assert_eq!(depth(&s), 3);
    assert!(matches!(head(&s), ModelMenu::Advanced { .. }));
    // Ultra is the last row.
    assert_eq!(ids(&s), ["max", "ultra"]);
    choose(&mut s, "ultra");
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("gpt-astra", Effort::Ultra, Apply::Advanced)]
    );
}

#[test]
fn esc_in_the_advanced_menu_returns_one_level_at_a_time() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-astra");
    choose(&mut s, MORE_REASONING);
    intent(&mut s, SelectIntent::Close);
    assert!(matches!(head(&s), ModelMenu::Reasoning { .. }));
    intent(&mut s, SelectIntent::Close);
    assert!(matches!(head(&s), ModelMenu::AllModels(_)));
    intent(&mut s, SelectIntent::Close);
    assert_eq!(depth(&s), 0);
    assert!(model(&s).picked.is_empty());
}

#[test]
fn an_auto_mode_picks_its_default_effort() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    choose(&mut s, "codex-auto-fast");
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("codex-auto-fast", Effort::Low, Apply::Selection)]
    );
}

#[test]
fn an_auto_mode_that_advertises_an_advanced_effort_opens_the_reasoning_menu() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    choose(&mut s, "codex-auto-thorough");
    assert_eq!(depth(&s), 2);
    assert!(matches!(head(&s), ModelMenu::Reasoning { .. }));
    // Esc returns to the quick menu, a pick closes both.
    intent(&mut s, SelectIntent::Close);
    assert!(matches!(head(&s), ModelMenu::Models(_)));
    choose(&mut s, "codex-auto-thorough");
    choose(&mut s, "high");
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection(
            "codex-auto-thorough",
            Effort::High,
            Apply::Selection
        )]
    );
}

// -- the session key

#[test]
fn the_session_key_picks_for_this_session_only() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    intent(&mut s, SelectIntent::Secondary("high".into()));
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("gpt-5.5", Effort::High, Apply::Session)]
    );
}

#[test]
fn the_session_key_works_on_a_one_effort_model_and_an_auto_mode() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    intent(
        &mut s,
        SelectIntent::Secondary("codex-auto-balanced".into()),
    );
    assert_eq!(
        picked(&s),
        [selection(
            "codex-auto-balanced",
            Effort::Medium,
            Apply::Session
        )]
    );
    all_models(&mut s);
    intent(&mut s, SelectIntent::Secondary("gpt-sol".into()));
    assert_eq!(
        picked(&s)[1],
        selection("gpt-sol", Effort::High, Apply::Session)
    );
}

#[test]
fn rows_without_a_session_key_ignore_it() {
    let mut s = sandbox(presets());
    open(&mut s, Current::new("gpt-5.5"));
    // All models, and an auto mode that needs its own menu.
    intent(&mut s, SelectIntent::Secondary(ALL_MODELS.into()));
    intent(
        &mut s,
        SelectIntent::Secondary("codex-auto-thorough".into()),
    );
    assert_eq!(depth(&s), 1);
    assert!(model(&s).picked.is_empty());
}

// -- the Ultra warning

#[test]
fn an_ultra_pick_warns_when_the_session_allows_many_threads() {
    for (threads, warning) in [(Some(8), Some(8)), (Some(7), None), (None, None)] {
        let mut s = sandbox(presets());
        let mut current = Current::new("gpt-astra");
        current.threads = threads;
        open(&mut s, current);
        choose(&mut s, ALL_MODELS);
        choose(&mut s, "gpt-astra");
        choose(&mut s, MORE_REASONING);
        choose(&mut s, "ultra");
        let mut want = selection("gpt-astra", Effort::Ultra, Apply::Advanced);
        want.warning = warning;
        assert_eq!(picked(&s), [want], "threads {threads:?}");
    }
}

// -- Plan mode

fn plan(model: &str, effort: Option<Effort>) -> PlanMode {
    PlanMode {
        model: model.into(),
        effort: effort.clone(),
        source: PlanSource::Default(effort),
    }
}

fn in_plan_mode(effort: Effort) -> Current {
    let mut current = Current::new("gpt-5.5");
    current.effort = Some(effort.clone());
    current.plan = Some(plan("gpt-5.5", Some(effort)));
    current
}

#[test]
fn in_plan_mode_a_changed_effort_asks_where_it_applies() {
    let mut s = sandbox(presets());
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    choose(&mut s, "high");
    assert!(model(&s).picked.is_empty());
    assert_eq!(depth(&s), 1);
    let page = model(&s).page().expect("page");
    assert_eq!(
        page.heading,
        Heading::PlanScope {
            effort: Some(Effort::High),
            source: PlanSource::Default(Some(Effort::Medium)),
        }
    );
    assert_eq!(ids(&s), [PLAN_ONLY, ALL_MODES]);
    choose(&mut s, PLAN_ONLY);
    assert_eq!(depth(&s), 0);
    assert_eq!(
        picked(&s),
        [selection("gpt-5.5", Effort::High, Apply::PlanOnly)]
    );
}

#[test]
fn the_scope_prompt_can_apply_to_every_mode_and_esc_drops_it() {
    let mut s = sandbox(presets());
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    choose(&mut s, "low");
    choose(&mut s, ALL_MODES);
    assert_eq!(
        picked(&s),
        [selection("gpt-5.5", Effort::Low, Apply::AllModes)]
    );
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    choose(&mut s, "low");
    intent(&mut s, SelectIntent::Close);
    assert_eq!(depth(&s), 0);
    assert_eq!(picked(&s).len(), 1);
}

#[test]
fn a_pick_that_changes_nothing_in_plan_mode_is_applied_at_once() {
    let mut s = sandbox(presets());
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    choose(&mut s, "medium");
    assert_eq!(
        picked(&s),
        [selection("gpt-5.5", Effort::Medium, Apply::Selection)]
    );
}

#[test]
fn another_model_or_the_session_key_never_asks_for_a_scope() {
    let mut s = sandbox(presets());
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-sol");
    assert_eq!(
        picked(&s),
        [selection("gpt-sol", Effort::High, Apply::Selection)]
    );
    open(&mut s, in_plan_mode(Effort::Medium));
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    intent(&mut s, SelectIntent::Secondary("low".into()));
    assert_eq!(
        picked(&s)[1],
        selection("gpt-5.5", Effort::Low, Apply::Session)
    );
}

#[test]
fn in_plan_mode_the_plan_override_is_the_effort_the_menu_starts_on() {
    let mut s = sandbox(presets());
    let mut current = in_plan_mode(Effort::Medium);
    if let Some(plan) = &mut current.plan {
        plan.source = PlanSource::Override(Effort::Low);
    }
    open(&mut s, current);
    choose(&mut s, ALL_MODELS);
    choose(&mut s, "gpt-5.5");
    let page = model(&s).page().expect("page");
    assert_eq!(page.initial, Some(0));
    assert!(page.rows[0].current);
}

// -- refresh

#[test]
fn a_refresh_leaves_the_menu_above_a_list_as_it_was_opened() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    let before = model(&s).page().expect("page");
    let mut renamed = presets();
    renamed[3].description = "Updated".into();
    let request = model(&s).catalog.pending.expect("asked");
    event(
        &mut s,
        Ev::Fetched {
            request,
            result: Ok(renamed.clone()),
        },
    );
    assert_eq!(model(&s).catalog.presets, renamed);
    assert_eq!(depth(&s), 2);
    assert_eq!(model(&s).page().expect("page"), before);
    // The parent shows the new catalog.
    intent(&mut s, SelectIntent::Close);
    let rows = model(&s).page().expect("page").rows;
    assert_eq!(rows[0].description, "Updated");
}

#[test]
fn a_refresh_that_leaves_no_choice_dismisses_the_list_with_a_note() {
    let only: Vec<Preset> = presets()
        .into_iter()
        .filter(|p| p.model == "gpt-5.5")
        .collect();
    let mut s = sandbox(only.clone());
    open(&mut s, Current::new("gpt-5.5"));
    assert_eq!(ids(&s), ["gpt-5.5"]);
    let mut hidden = only;
    hidden[0].show_in_picker = false;
    // Hidden: nothing left, but a catalog with no visible model is still a
    // catalog (only an empty one is ignored).
    event(&mut s, Ev::Loaded(hidden));
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, vec![Note::NoAdditionalModels]);
}

#[test]
fn the_reasoning_menu_outlives_its_dismissed_parent() {
    let only: Vec<Preset> = presets()
        .into_iter()
        .filter(|p| p.model == "gpt-5.5")
        .collect();
    let mut s = sandbox(only.clone());
    open(&mut s, Current::new("gpt-5.5"));
    choose(&mut s, "gpt-5.5");
    assert_eq!(depth(&s), 2);
    let mut hidden = only;
    hidden[0].show_in_picker = false;
    event(&mut s, Ev::Loaded(hidden));
    // The parent went, the child stays; Esc then leaves nothing.
    assert_eq!(depth(&s), 1);
    assert!(matches!(head(&s), ModelMenu::Reasoning { .. }));
    assert_eq!(model(&s).notes, vec![Note::NoAdditionalModels]);
    intent(&mut s, SelectIntent::Close);
    assert_eq!(depth(&s), 0);
}

#[test]
fn an_all_models_list_that_turns_auto_only_is_dismissed() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    let mut auto_only = presets();
    auto_only.retain(Preset::is_auto);
    event(&mut s, Ev::Loaded(auto_only));
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, vec![Note::NoAdditionalModels]);
}

#[test]
fn a_catalog_with_no_choice_opens_no_menu_and_says_so() {
    let mut hidden = presets();
    for preset in &mut hidden {
        preset.show_in_picker = false;
    }
    let mut s = sandbox(hidden);
    open(&mut s, Current::new("gpt-5.5"));
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, vec![Note::NoAdditionalModels]);
}

// -- the wire

#[test]
fn a_model_list_answer_becomes_presets_with_the_hidden_flag_inverted() {
    let answer = json!({"data": [{
        "id": "1", "model": "gpt-5.5", "upgrade": null, "upgradeInfo": null,
        "availabilityNux": null, "displayName": "GPT-5.5", "description": "d",
        "hidden": true,
        "supportedReasoningEfforts": [{"reasoningEffort": "low", "description": "fast"}],
        "defaultReasoningEffort": "low", "multiAgentVersion": null, "isDefault": false
    }], "nextCursor": null});
    let presets = codex_tui_core::screens::select::models::presets(answer).expect("reads");
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].model, "gpt-5.5");
    assert_eq!(presets[0].name, "GPT-5.5");
    assert!(!presets[0].show_in_picker);
    assert_eq!(presets[0].efforts[0].effort, Effort::Low);
    assert_eq!(presets[0].efforts[0].description, "fast");
}

#[test]
fn a_menu_a_script_seeds_is_the_only_one_on_the_stack() {
    let mut s = Sandbox::play(
        r#"{"step":"screen","dir":"select","ev":{"kind":"catalog","presets":[{"model":"m","name":"M"}]}}
{"step":"screen","dir":"select","ev":{"kind":"models","current":{"model":"m"}}}"#,
    )
    .expect("plays");
    assert_eq!(depth(&s), 1);
    assert_eq!(ids(&s), ["m"]);
    intent(&mut s, SelectIntent::Close);
    assert_eq!(depth(&s), 0);
}
