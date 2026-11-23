#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S14: a sheet is asked for by a script's `screen` step, a pick is a verb of
//! the sheet and a model fact (`list_selection_view.rs`: `dismiss_on_select`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::permissions::{
    Chosen, Label, PermissionsMenu, Preset, Reviewer,
};
use codex_tui_core::screens::select::{Intent as SelectIntent, Note, Opening};
use pretty_assertions::assert_eq;

const PERMISSIONS: &str = r#"{"step":"screen","dir":"select","ev":{"kind":"permissions"}}"#;

fn pick(sandbox: &mut Sandbox, intent: SelectIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))));
}

/// The menu the script's step asked a sheet for.
fn asked(sandbox: &Sandbox) -> PermissionsMenu {
    sandbox
        .app()
        .screens()
        .select
        .shows
        .pending()
        .find_map(|show| match &show.opening {
            Opening::Permissions(menu) => Some(menu.clone()),
            _ => None,
        })
        .expect("a permissions sheet is asked for")
}

/// What a sheet on the menu does for a row: the pure rule of the menu.
fn choose(sandbox: &Sandbox, id: &str) -> Option<Chosen> {
    asked(sandbox).choose(&sandbox.app().screens().select.profiles, id)
}

#[test]
fn a_permission_pick_is_a_verb_and_the_app_takes_it() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    let Some(Chosen::Done(selection)) = choose(&sandbox, ":workspace") else {
        panic!("a pick of the preset is final");
    };
    pick(&mut sandbox, SelectIntent::SetPermissions(selection));
    let model = &sandbox.app().screens().select;
    // The app applies a permission pick at once (core/tests/select_live.rs
    // holds its asks); what stays is the line for the history.
    assert!(model.picked.is_empty());
    assert!(model.notes.is_empty(), "told at once");
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
    let sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    assert_eq!(choose(&sandbox, "nothing"), None);
    assert!(sandbox.app().screens().select.picked.is_empty());
}

#[test]
fn leaving_the_sheet_picks_nothing() {
    let mut sandbox = Sandbox::play(PERMISSIONS).expect("plays");
    pick(&mut sandbox, SelectIntent::AbandonPermissions);
    assert!(!sandbox.app().screens().select.profiles.inline_pending());
    assert!(sandbox.app().screens().select.picked.is_empty());
}
