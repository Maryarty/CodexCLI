#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S14: a menu is opened by a script's `screen` step, a pick is
//! a model fact and closes the menu (`list_selection_view.rs`: `dismiss_on_select`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::{Access, Intent as SelectIntent, Menu, Picked};
use pretty_assertions::assert_eq;

const MODELS: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"models","rows":[{"name":"gpt-5.1-codex","description":"d1","current":true},{"name":"gpt-5.1-codex-mini","description":"d2"}]}}"#;

fn pick(sandbox: &mut Sandbox, intent: SelectIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))));
}

#[test]
fn a_models_menu_lists_its_rows_with_the_current_one() {
    let sandbox = Sandbox::play(MODELS).expect("plays");
    let Some(Menu::Models(rows)) = &sandbox.app().screens().select.menu else {
        panic!("a models menu");
    };
    assert_eq!(rows.len(), 2);
    assert!(rows[0].current && !rows[1].current);
}

#[test]
fn a_pick_is_recorded_and_closes_the_menu() {
    let mut sandbox = Sandbox::play(MODELS).expect("plays");
    pick(&mut sandbox, SelectIntent::Pick(1));
    let model = &sandbox.app().screens().select;
    assert_eq!(
        model.picked,
        vec![Picked::Model("gpt-5.1-codex-mini".into())]
    );
    assert!(model.menu.is_none());
}

#[test]
fn permissions_rows_pick_their_access() {
    let mut sandbox =
        Sandbox::play(r#"{"step":"screen","dir":"select","ev":{"kind":"permissions"}}"#)
            .expect("plays");
    pick(&mut sandbox, SelectIntent::Pick(1));
    assert_eq!(
        sandbox.app().screens().select.picked,
        vec![Picked::Access(Access::Full)]
    );
}

#[test]
fn a_pick_past_the_rows_changes_nothing() {
    let mut sandbox = Sandbox::play(MODELS).expect("plays");
    pick(&mut sandbox, SelectIntent::Pick(9));
    assert!(sandbox.app().screens().select.menu.is_some());
    assert!(sandbox.app().screens().select.picked.is_empty());
}

#[test]
fn close_drops_the_menu_without_a_pick() {
    let mut sandbox = Sandbox::play(MODELS).expect("plays");
    pick(&mut sandbox, SelectIntent::Close);
    assert!(sandbox.app().screens().select.menu.is_none());
    assert!(sandbox.app().screens().select.picked.is_empty());
}

const MULTI: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"list","multi":true,"items":[{"id":"a","name":"A","toggle":true,"checkbox":true},{"id":"b","name":"B","toggle":false,"checkbox":true},{"id":"c","name":"C","toggle":true,"checkbox":true}]}}"#;

#[test]
fn a_multi_select_answers_the_toggled_on_ids_in_list_order_and_closes() {
    let mut sandbox = Sandbox::play(MULTI).expect("plays");
    pick(&mut sandbox, SelectIntent::Toggle("b".into()));
    pick(&mut sandbox, SelectIntent::Toggle("a".into()));
    pick(&mut sandbox, SelectIntent::Confirm);
    let model = &sandbox.app().screens().select;
    assert_eq!(
        model.picked,
        vec![Picked::Items(vec!["b".into(), "c".into()])]
    );
    assert!(model.menu.is_none());
}

#[test]
fn a_multi_select_confirmed_with_nothing_on_answers_an_empty_list() {
    let mut sandbox = Sandbox::play(MULTI).expect("plays");
    for id in ["a", "c"] {
        pick(&mut sandbox, SelectIntent::Toggle(id.into()));
    }
    pick(&mut sandbox, SelectIntent::Confirm);
    assert_eq!(
        sandbox.app().screens().select.picked,
        vec![Picked::Items(Vec::new())]
    );
}

#[test]
fn a_confirm_of_a_list_that_is_not_multi_does_nothing() {
    let list = r#"{"step":"screen","dir":"select","ev":{"kind":"list","items":[{"id":"a","name":"A","toggle":true}]}}"#;
    let mut sandbox = Sandbox::play(list).expect("plays");
    pick(&mut sandbox, SelectIntent::Confirm);
    let model = &sandbox.app().screens().select;
    assert!(model.picked.is_empty());
    assert!(model.menu.is_some());
}
