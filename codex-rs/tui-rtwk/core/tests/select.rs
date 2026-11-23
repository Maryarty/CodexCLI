#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S14: a menu is opened by a script's `screen` step, a pick is
//! a model fact and closes the menu (`list_selection_view.rs`: `dismiss_on_select`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::permissions::{Label, Preset, Reviewer};
use codex_tui_core::screens::select::{Intent as SelectIntent, Note};
use pretty_assertions::assert_eq;

const PERMISSIONS: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"permissions"}}"#;

fn pick(sandbox: &mut Sandbox, intent: SelectIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))));
}

#[test]
fn a_permission_pick_closes_the_menu_and_the_app_takes_it() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    pick(&mut sandbox, SelectIntent::Choose(":workspace".into()));
    let model = &sandbox.app().screens().select;
    // The app applies a permission pick at once (core/tests/select_live.rs
    // holds its asks); what stays is the line for the history.
    assert!(model.picked.is_empty());
    assert!(model.notes.is_empty(), "told at once");
    assert!(model.menu().is_none());
    // No thread is open: the line is a note of the chat screen.
    assert_eq!(
        sandbox.app().screens().chat.notes,
        vec![codex_tui_core::threads::Note::from(
            codex_tui_core::threads::NoteKey::Select(Note::PermissionsUpdated(Label::Mode {
                preset: Preset::Auto,
                reviewer: Reviewer::User
            }))
        )]
    );
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
