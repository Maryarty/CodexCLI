#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S14: a menu is opened by a script's `screen` step, a pick is
//! a model fact and closes the menu (`list_selection_view.rs`: `dismiss_on_select`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::permissions::{Preset, Reviewer, Selection};
use codex_tui_core::screens::select::{Intent as SelectIntent, Menu, Picked};
use pretty_assertions::assert_eq;

const PERMISSIONS: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"permissions"}}"#;

fn pick(sandbox: &mut Sandbox, intent: SelectIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))));
}

#[test]
fn a_pick_is_recorded_and_closes_the_menu() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    pick(&mut sandbox, SelectIntent::Choose(":workspace".into()));
    let model = &sandbox.app().screens().select;
    assert_eq!(
        model.picked,
        vec![Picked::Permissions(Selection::Preset {
            preset: Preset::Auto,
            reviewer: Reviewer::User
        })]
    );
    assert!(model.menu().is_none());
}

#[test]
fn a_choice_that_is_no_row_changes_nothing() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    pick(&mut sandbox, SelectIntent::Choose("nothing".into()));
    assert!(sandbox.app().screens().select.menu().is_some());
    assert!(sandbox.app().screens().select.picked.is_empty());
}

#[test]
fn close_drops_the_menu_without_a_pick() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    pick(&mut sandbox, SelectIntent::Close);
    assert!(sandbox.app().screens().select.menu().is_none());
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
    assert!(model.menu().is_none());
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
    assert!(model.menu().is_some());
}

const ORDERABLE: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"list","multi":true,"ordering":true,"items":[{"id":"theme","name":"theme-colors","toggle":true,"checkbox":true,"section_break_after":true},{"id":"model","name":"model","toggle":true,"checkbox":true,"orderable":true},{"id":"branch","name":"branch","toggle":false,"checkbox":true,"orderable":true}]}}"#;

fn ids(sandbox: &Sandbox) -> Vec<String> {
    let Some(Menu::List(list)) = sandbox.app().screens().select.menu() else {
        panic!("a list menu");
    };
    list.items.iter().map(|item| item.id.clone()).collect()
}

#[test]
fn an_orderable_item_swaps_with_its_orderable_neighbour() {
    let mut sandbox = Sandbox::play(ORDERABLE).expect("plays");
    pick(&mut sandbox, SelectIntent::Reorder("model".into(), true));
    assert_eq!(ids(&sandbox), ["theme", "branch", "model"]);
    pick(&mut sandbox, SelectIntent::Reorder("model".into(), false));
    assert_eq!(ids(&sandbox), ["theme", "model", "branch"]);
}

#[test]
fn a_non_orderable_item_neither_moves_nor_is_crossed() {
    let mut sandbox = Sandbox::play(ORDERABLE).expect("plays");
    pick(&mut sandbox, SelectIntent::Reorder("theme".into(), true));
    pick(&mut sandbox, SelectIntent::Reorder("model".into(), false));
    pick(&mut sandbox, SelectIntent::Reorder("branch".into(), true));
    assert_eq!(ids(&sandbox), ["theme", "model", "branch"]);
}

#[test]
fn a_reordered_multi_select_answers_in_the_new_order() {
    let mut sandbox = Sandbox::play(ORDERABLE).expect("plays");
    pick(&mut sandbox, SelectIntent::Toggle("branch".into()));
    pick(&mut sandbox, SelectIntent::Reorder("branch".into(), false));
    pick(&mut sandbox, SelectIntent::Confirm);
    assert_eq!(
        sandbox.app().screens().select.picked,
        vec![Picked::Items(vec![
            "theme".into(),
            "branch".into(),
            "model".into()
        ])]
    );
}
