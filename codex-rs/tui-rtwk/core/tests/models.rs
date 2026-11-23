#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the `/model` family of S14: the catalog, the shows and the pure rules of
//! `chatwidget/model_popups.rs` and `model_popup_state.rs` (a choice opens the next page or is
//! final, a catalog refresh keeps what is open and drops what has nothing left to offer).
//! The pages are the sheet's (ADR-013: a nested navigator, `tests/pages/models.rs` is its test),
//! so a test here plays the sheet as a fold of the two pure answers the model gives it
//! (`ModelMenu::choose` leads from a page to the next, `ModelMenu::next` is what the catalog makes
//! of one) over the pages the shows open; `Esc` is the sheet's pop.
//! `core/tests/model.rs` is the app model's; this file is the picker's.

use codex_tui_core::app::AppFx;
use codex_tui_core::screens::select::models::{
    ALL_MODELS, ALL_MODES, Answer, Apply, Current, Effort, GaveUp, Heading, Kind, MORE_REASONING,
    ModelMenu, Next, Onward, PLAN_ONLY, PlanMode, PlanSource, Preset,
};
use codex_tui_core::screens::select::{
    Ev, Intent as SelectIntent, Note, Opening, Picked, Selection,
};
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

/// The picker's own model: these are state tests of the screen, not of the
/// app that answers its asks (`core/tests/select_live.rs` is that). `pages` are the sheet's
/// (the navigator's routes), played by the helpers below.
#[derive(Default)]
struct Sandbox {
    model: codex_tui_core::screens::select::Model,
    fx: AppFx,
    pages: Vec<ModelMenu>,
}

impl Sandbox {
    /// The picker's model after a script's events (`play` of the app).
    fn play(script: &str) -> Result<Self, String> {
        let played = codex_tui_core::fixtures::sandbox::Sandbox::play(script)
            .map_err(|error| format!("{error:?}"))?;
        let mut sandbox = Self {
            model: played.app().screens().select.clone(),
            fx: AppFx::default(),
            pages: Vec::new(),
        };
        present(&mut sandbox);
        Ok(sandbox)
    }
}

fn sandbox(catalog: Vec<Preset>) -> Sandbox {
    let mut sandbox = Sandbox::default();
    if !catalog.is_empty() {
        event(&mut sandbox, Ev::Loaded(catalog));
    }
    sandbox
}

/// The presenter: a sheet opens on the menu of every show, and the show is said to be shown.
fn present(sandbox: &mut Sandbox) {
    let shows: Vec<_> = sandbox.model.shows.pending().cloned().collect();
    for show in shows {
        if let Opening::Models(menu) = show.opening {
            sandbox.pages = vec![menu];
        }
        intent(sandbox, SelectIntent::Shown(show.seq));
    }
}

/// The pages look at the catalog after it changed (`ModelMenu::next`): the loading page goes on
/// with its answer, a list left with no choice says so and leaves, the provider closes the sheet.
fn follow(sandbox: &mut Sandbox) {
    let mut at = 0;
    while at < sandbox.pages.len() {
        match sandbox.pages[at].next(&sandbox.model.catalog) {
            None => at += 1,
            Some(Next::Close) => sandbox.pages.clear(),
            Some(Next::Replace(menu)) => {
                sandbox.pages[at] = *menu;
                at += 1;
            }
            Some(Next::GiveUp(why)) => {
                intent(sandbox, SelectIntent::GiveUp(why));
                sandbox.pages.remove(at);
            }
        }
    }
}

fn event(sandbox: &mut Sandbox, ev: Ev) {
    codex_tui_core::screens::select::apply(&mut sandbox.model, ev, &mut sandbox.fx);
    follow(sandbox);
}

fn intent(sandbox: &mut Sandbox, intent: SelectIntent) {
    codex_tui_core::screens::select::apply_intent(&mut sandbox.model, intent, &mut sandbox.fx);
}

/// A row of the page on top is picked with the primary key (`s`: `session`), and the step the
/// model answers is what the sheet does with its pages.
fn pick(sandbox: &mut Sandbox, id: &str, session: bool) {
    let step = head(sandbox).choose(&sandbox.model.catalog, id, session);
    match step {
        Onward::None => {}
        Onward::Push(menu) => sandbox.pages.push(menu),
        Onward::Replace(menu) => {
            sandbox.pages.pop();
            sandbox.pages.push(menu);
        }
        Onward::Done(selection) => {
            intent(sandbox, SelectIntent::Settle(selection));
            sandbox.pages.clear();
        }
        Onward::Prompt(menu) => sandbox.pages = vec![menu],
    }
}

fn choose(sandbox: &mut Sandbox, id: &str) {
    pick(sandbox, id, false);
}

fn secondary(sandbox: &mut Sandbox, id: &str) {
    pick(sandbox, id, true);
}

/// Esc: the page on top goes, the one under it shows again.
fn esc(sandbox: &mut Sandbox) {
    sandbox.pages.pop();
}

fn model(sandbox: &Sandbox) -> &codex_tui_core::screens::select::Model {
    &sandbox.model
}

/// The rows of the page on top.
fn page_of(sandbox: &Sandbox) -> codex_tui_core::screens::select::models::Page {
    head(sandbox).page(&model(sandbox).catalog)
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    page_of(sandbox)
        .rows
        .into_iter()
        .map(|row| row.id)
        .collect()
}

fn depth(sandbox: &Sandbox) -> usize {
    sandbox.pages.len()
}

fn open(sandbox: &mut Sandbox, current: Current) {
    intent(sandbox, SelectIntent::OpenModels(current));
    present(sandbox);
    follow(sandbox);
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
    sandbox.pages.last().expect("a page of the sheet")
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
    // The show carries the request it waits for: the answer is read under it.
    let ModelMenu::Loading { request, .. } = head(&s) else {
        panic!("loading, not {:?}", head(&s))
    };
    assert_eq!(Some(*request), model(&s).catalog.pending);
    assert_eq!(page_of(&s).heading, Heading::Loading);
    assert_eq!(model(&s).catalog.answered, None);
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
    assert_eq!(model(&s).catalog.answered, Some((request, Answer::Listed)));
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
    assert_eq!(model(&s).catalog.answered, Some((request, Answer::Failed)));
}

#[test]
fn a_failure_after_the_loading_page_was_left_says_nothing() {
    // Only the sheet knows the user is still looking at `Loading models…` (Esc left it): the
    // model keeps the fact, the note is the page's verb (`Intent::GiveUp`).
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let request = model(&s).catalog.pending.expect("asked");
    esc(&mut s);
    event(
        &mut s,
        Ev::Fetched {
            request,
            result: Err("down".into()),
        },
    );
    assert!(model(&s).notes.is_empty());
    assert_eq!(model(&s).catalog.answered, Some((request, Answer::Failed)));
}

#[test]
fn an_answer_with_no_choice_ends_the_loading_page_with_the_note_of_an_empty_list() {
    let mut hidden = presets();
    for preset in &mut hidden {
        preset.show_in_picker = false;
    }
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let request = model(&s).catalog.pending.expect("asked");
    event(
        &mut s,
        Ev::Fetched {
            request,
            result: Ok(hidden),
        },
    );
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, vec![Note::NoAdditionalModels]);
}

#[test]
fn an_answer_of_an_older_request_is_dropped() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let old = model(&s).catalog.pending.expect("asked");
    esc(&mut s);
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
    assert!(matches!(head(&s), ModelMenu::Loading { .. }));
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
    let page = page_of(&s);
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
    let page = page_of(&s);
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
    assert_eq!(page_of(&s).heading, Heading::All);
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
    assert_eq!(page_of(&s).heading, Heading::All);
}

#[test]
fn an_all_models_row_has_the_session_key_only_with_one_standard_effort() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    let rows = page_of(&s).rows;
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
    esc(&mut s);
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
    let page = page_of(&s);
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
    let page = page_of(&s);
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
    let warns: Vec<_> = page_of(&s)
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
    esc(&mut s);
    assert!(matches!(head(&s), ModelMenu::Reasoning { .. }));
    esc(&mut s);
    assert!(matches!(head(&s), ModelMenu::AllModels(_)));
    esc(&mut s);
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
    esc(&mut s);
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
    secondary(&mut s, "high");
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
    secondary(&mut s, "codex-auto-balanced");
    assert_eq!(
        picked(&s),
        [selection(
            "codex-auto-balanced",
            Effort::Medium,
            Apply::Session
        )]
    );
    all_models(&mut s);
    secondary(&mut s, "gpt-sol");
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
    secondary(&mut s, ALL_MODELS);
    secondary(&mut s, "codex-auto-thorough");
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
    let page = page_of(&s);
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
    esc(&mut s);
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
    secondary(&mut s, "low");
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
    let page = page_of(&s);
    assert_eq!(page.initial, Some(0));
    assert!(page.rows[0].current);
}

// -- refresh

#[test]
fn a_refresh_leaves_the_menu_above_a_list_as_it_was_opened() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    let before = page_of(&s);
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
    assert_eq!(page_of(&s), before);
    // The parent shows the new catalog.
    esc(&mut s);
    let rows = page_of(&s).rows;
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
    esc(&mut s);
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

// -- the provider write

#[test]
fn a_provider_write_closes_every_page_and_drops_the_answer_in_flight() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    assert_eq!(depth(&s), 2);
    assert!(model(&s).catalog.pending.is_some());
    event(
        &mut s,
        Ev::Provider(codex_tui_core::screens::select::Provider::Changed),
    );
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).catalog.pending, None);
    assert!(model(&s).notes.is_empty());
}

#[test]
fn a_show_not_shown_yet_is_forgotten_when_the_provider_is_written() {
    let mut s = sandbox(presets());
    intent(&mut s, SelectIntent::OpenModels(Current::new("gpt-5.5")));
    assert_eq!(model(&s).shows.pending().count(), 1);
    event(
        &mut s,
        Ev::Provider(codex_tui_core::screens::select::Provider::Saving),
    );
    assert_eq!(model(&s).shows.pending().count(), 0);
}

#[test]
fn after_a_provider_write_model_opens_no_sheet_and_says_why() {
    let mut s = sandbox(presets());
    event(
        &mut s,
        Ev::Provider(codex_tui_core::screens::select::Provider::Changed),
    );
    open(&mut s, Current::new("gpt-5.5"));
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).shows.pending().count(), 0);
    assert_eq!(
        model(&s).notes,
        vec![Note::ModelPickerUnavailable(
            codex_tui_core::screens::select::Provider::Changed
        )]
    );
}

// -- what a page does with the catalog's facts

#[test]
fn a_page_acts_on_the_answer_of_its_own_request_only() {
    let mut s = sandbox(Vec::new());
    open(&mut s, Current::new("gpt-5.5"));
    let loading = head(&s).clone();
    let ModelMenu::Loading { request, .. } = &loading else {
        panic!("loading")
    };
    let mut catalog = model(&s).catalog.clone();
    assert_eq!(loading.next(&catalog), None);
    catalog.answered = Some((request + 1, Answer::Failed));
    assert_eq!(loading.next(&catalog), None);
    catalog.answered = Some((*request, Answer::Failed));
    assert_eq!(
        loading.next(&catalog),
        Some(Next::GiveUp(GaveUp::NotLoaded))
    );
    catalog.answered = Some((*request, Answer::Listed));
    catalog.presets = presets();
    assert_eq!(
        loading.next(&catalog),
        Some(Next::Replace(Box::new(ModelMenu::Models(Current::new(
            "gpt-5.5"
        )))))
    );
}

#[test]
fn only_the_lists_give_themselves_up_when_nothing_is_left_to_offer() {
    let mut s = sandbox(presets());
    all_models(&mut s);
    choose(&mut s, "gpt-5.5");
    let mut hidden = presets();
    for preset in &mut hidden {
        preset.show_in_picker = false;
    }
    let mut catalog = model(&s).catalog.clone();
    catalog.presets = hidden;
    let empty = Some(Next::GiveUp(GaveUp::Empty { reserve: false }));
    assert_eq!(s.pages[0].next(&catalog), empty);
    assert_eq!(
        ModelMenu::Models(Current::new("gpt-5.5")).next(&catalog),
        empty
    );
    // The pages over a list keep the model they were opened on.
    assert_eq!(s.pages[1].next(&catalog), None);
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
fn a_menu_a_script_seeds_is_a_show_the_sheet_opens_on() {
    let mut s = Sandbox::play(
        r#"{"step":"screen","dir":"select","ev":{"kind":"catalog","presets":[{"model":"m","name":"M"}]}}
{"step":"screen","dir":"select","ev":{"kind":"models","current":{"model":"m"}}}"#,
    )
    .expect("plays");
    assert_eq!(depth(&s), 1);
    assert_eq!(ids(&s), ["m"]);
    // the presenter said it showed it: nothing is left to show
    assert_eq!(model(&s).shows.pending().count(), 0);
    esc(&mut s);
    assert_eq!(depth(&s), 0);
}

fn reasoning(
    sandbox: &mut Sandbox,
    model: &str,
    effort: Option<Effort>,
    direction: Direction,
    plan: bool,
) {
    intent(
        sandbox,
        SelectIntent::Reasoning {
            direction,
            current: Current {
                effort,
                ..Current::new(model)
            },
            plan,
            started: true,
        },
    );
}

use codex_tui_core::screens::select::reasoning::Direction;

#[test]
fn a_reasoning_step_walks_the_advertised_efforts_with_the_advanced_ones_last() {
    let mut s = sandbox(presets());
    // gpt-5.5 advertises low, medium, high, xhigh and persistent
    reasoning(
        &mut s,
        "gpt-5.5",
        Some(Effort::Medium),
        Direction::Raise,
        false,
    );
    reasoning(
        &mut s,
        "gpt-5.5",
        Some(Effort::Medium),
        Direction::Lower,
        false,
    );
    assert_eq!(
        model(&s).picked,
        [
            Picked::Effort {
                model: "gpt-5.5".into(),
                effort: Effort::High,
                plan: false
            },
            Picked::Effort {
                model: "gpt-5.5".into(),
                effort: Effort::Low,
                plan: false
            },
        ]
    );
}

#[test]
fn an_unset_effort_anchors_to_the_model_default() {
    let mut s = sandbox(presets());
    reasoning(&mut s, "gpt-5.5", None, Direction::Raise, true);
    assert_eq!(
        model(&s).picked,
        [Picked::Effort {
            model: "gpt-5.5".into(),
            effort: Effort::High,
            plan: true
        }]
    );
}

#[test]
fn a_raise_stops_before_ultra_and_says_where_it_is() {
    let mut s = sandbox(presets());
    // gpt-astra: max then ultra
    reasoning(
        &mut s,
        "gpt-astra",
        Some(Effort::Max),
        Direction::Raise,
        false,
    );
    assert!(model(&s).picked.is_empty());
    assert_eq!(model(&s).notes, [Note::UltraAvailable("gpt-astra".into())]);
    // and lowering out of Ultra is a step
    reasoning(
        &mut s,
        "gpt-astra",
        Some(Effort::Ultra),
        Direction::Lower,
        false,
    );
    assert_eq!(
        model(&s).picked,
        [Picked::Effort {
            model: "gpt-astra".into(),
            effort: Effort::Max,
            plan: false
        }]
    );
}

#[test]
fn the_ends_and_the_unknowns_change_nothing() {
    let mut s = sandbox(presets());
    reasoning(
        &mut s,
        "gpt-sol",
        Some(Effort::High),
        Direction::Raise,
        false,
    );
    reasoning(
        &mut s,
        "gpt-sol",
        Some(Effort::High),
        Direction::Lower,
        false,
    );
    reasoning(&mut s, "no-such-model", None, Direction::Raise, false);
    assert!(model(&s).picked.is_empty());
    assert_eq!(
        model(&s).notes,
        [
            Note::ReasoningBound {
                direction: Direction::Raise,
                effort: Effort::High
            },
            Note::ReasoningBound {
                direction: Direction::Lower,
                effort: Effort::High
            },
            Note::ReasoningUnavailable("no-such-model".into()),
        ]
    );
}

#[test]
fn a_shortcut_before_the_session_is_configured_says_so() {
    let mut s = sandbox(presets());
    intent(
        &mut s,
        SelectIntent::Reasoning {
            direction: Direction::Raise,
            current: Current::new("gpt-5.5"),
            plan: false,
            started: false,
        },
    );
    assert!(model(&s).picked.is_empty());
    assert_eq!(model(&s).notes, [Note::ReasoningDisabled]);
}

#[test]
fn a_service_tier_command_toggles_against_the_tier_the_thread_runs_on() {
    let mut s = sandbox(presets());
    intent(
        &mut s,
        SelectIntent::ServiceTier {
            id: "priority".into(),
            current: None,
        },
    );
    intent(
        &mut s,
        SelectIntent::ServiceTier {
            id: "priority".into(),
            current: Some("priority".into()),
        },
    );
    assert_eq!(
        model(&s).picked,
        [
            Picked::Tier("priority".into()),
            Picked::Tier("default".into())
        ]
    );
}

#[test]
fn a_model_list_answer_carries_the_tiers_of_each_model() {
    let answer = json!({"data": [{
        "id": "1", "model": "gpt-5.4", "upgrade": null, "upgradeInfo": null,
        "availabilityNux": null, "displayName": "gpt-5.4", "description": "d", "hidden": false,
        "supportedReasoningEfforts": [{"reasoningEffort": "low", "description": "fast"}],
        "defaultReasoningEffort": "low", "isDefault": true,
        "serviceTiers": [{"id": "priority", "name": "Fast", "description": "2x speed"}],
        "defaultServiceTier": "priority"
    }], "nextCursor": null});
    let presets = codex_tui_core::screens::select::models::presets(answer).expect("reads");
    assert_eq!(presets[0].service_tiers.len(), 1);
    assert_eq!(presets[0].service_tiers[0].id, "priority");
    assert_eq!(presets[0].service_tiers[0].name, "Fast");
    assert_eq!(presets[0].default_service_tier.as_deref(), Some("priority"));
}

fn reserve_current(normal: Option<&str>) -> Current {
    Current {
        reserve: Some(codex_tui_core::screens::select::models::ReserveOnly {
            normal_model: normal.map(str::to_owned),
        }),
        ..Current::new("gpt-reserve")
    }
}

#[test]
fn the_reserve_only_menu_offers_the_normal_model_it_borrows_and_nothing_else() {
    let mut s = sandbox(presets());
    open(&mut s, reserve_current(Some("gpt-5.5")));
    let page = page_of(&s);
    assert_eq!(
        page.heading,
        codex_tui_core::screens::select::models::Heading::Reserve
    );
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].id, "gpt-reserve");
    assert_eq!(page.rows[0].name, "GPT-5.5");
    assert!(page.rows[0].current);
}

#[test]
fn the_reserve_effort_is_the_tasks_alone() {
    let mut s = sandbox(presets());
    open(&mut s, reserve_current(Some("gpt-5.5")));
    choose(&mut s, "gpt-reserve");
    // the reasoning menu of the borrowed model, under the Reserve's slug
    assert_eq!(depth(&s), 2);
    choose(&mut s, "high");
    assert_eq!(depth(&s), 0);
    let Some(Picked::Selection(selection)) = model(&s).picked.last() else {
        panic!("a selection: {:?}", model(&s).picked)
    };
    assert_eq!(selection.model, "gpt-reserve");
    assert_eq!(selection.effort, Some(Effort::High));
    assert_eq!(selection.apply, Apply::Reserve);
    // it patches the thread's effort and keeps the model and the saved defaults
    let effects = codex_tui_core::screens::select::persist::effects(selection);
    let patch = effects.patch.expect("a patch");
    assert_eq!((patch.model, patch.effort), (None, Some(Effort::High)));
    assert!(effects.edits.is_empty());
}

#[test]
fn a_reserve_without_its_model_in_the_catalog_says_so() {
    let mut s = sandbox(presets());
    open(&mut s, reserve_current(Some("gpt-nowhere")));
    assert_eq!(depth(&s), 0);
    assert_eq!(model(&s).notes, [Note::ReserveUnavailable]);
}
