#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S24 (the `/pets` picker): `pets/picker.rs` tests and the
//! selection events of `build_pet_picker_params`.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::pets::{Choice, DISABLED, Intent as PetsIntent, Picker, Preview};
use pretty_assertions::assert_eq;

const PLAIN: &str = include_str!("../../src/sheets/pets/fixtures/picker.jsonl");
const CUSTOM: &str = include_str!("../../src/sheets/pets/fixtures/picker_custom.jsonl");
const DISABLED_FIXTURE: &str = include_str!("../../src/sheets/pets/fixtures/picker_disabled.jsonl");

fn send(sandbox: &mut Sandbox, intent: PetsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Pets(intent))));
}

fn picker(sandbox: &Sandbox) -> Picker {
    sandbox
        .app()
        .screens()
        .pets
        .open
        .clone()
        .expect("the picker is open")
}

fn selectors(picker: &Picker) -> Vec<&str> {
    picker
        .pets
        .iter()
        .map(|pet| pet.selector.as_str())
        .collect()
}

#[test]
fn picker_lists_app_bundled_and_custom_pets() {
    let sandbox = Sandbox::play(CUSTOM).expect("plays");
    let picker = picker(&sandbox);
    // Disabled first, then by display name: BSOD, Chefito, Codex, ...
    assert_eq!(
        selectors(&picker),
        vec![
            DISABLED,
            "bsod",
            "custom:chefito",
            "codex",
            "dewey",
            "fireball",
            "null-signal",
            "rocky",
            "seedy",
            "stacky"
        ]
    );
    assert_eq!(picker.initial(), 2);
    assert_eq!(picker.pets[2].name, "Chefito");
}

#[test]
fn picker_preselects_codex_without_marking_it_current_when_no_pet_is_configured() {
    let sandbox = Sandbox::play(PLAIN).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 2);
    assert_eq!(picker.pets[2].name, "Codex");
    assert!(!picker.is_current(&picker.pets[2]));
}

#[test]
fn picker_marks_disabled_pet_as_current() {
    let sandbox = Sandbox::play(DISABLED_FIXTURE).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 0);
    assert_eq!(picker.pets[0].selector, DISABLED);
    assert_eq!(picker.pets[0].description, None);
    assert!(picker.is_current(&picker.pets[0]));
}

#[test]
fn a_custom_pet_is_current_through_its_legacy_selector() {
    let script = CUSTOM.replace("\"current\":\"custom:chefito\"", "\"current\":\"chefito\"");
    let sandbox = Sandbox::play(&script).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 2);
    assert!(picker.is_current(&picker.pets[2]));
}

#[test]
fn opening_and_moving_request_the_preview_of_the_row() {
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    assert_eq!(sandbox.app().screens().pets.previews, vec!["codex"]);
    assert_eq!(
        picker(&sandbox).preview,
        Some(("codex".to_string(), Preview::Loading))
    );
    // The cursor is the widget's: it tells the model which row it is on.
    send(&mut sandbox, PetsIntent::Highlight("dewey".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey"]
    );
    send(&mut sandbox, PetsIntent::Highlight("dewey".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey"]
    );
    send(&mut sandbox, PetsIntent::Highlight("bsod".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey", "bsod"]
    );
}

#[test]
fn the_preview_becomes_ready_only_for_the_row_it_was_asked_for() {
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::pets::Ev;
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::Highlight("dewey".into()));
    sandbox.send(AppEv::Screen(ScreenEv::Pets(Ev::PreviewReady(
        "codex".into(),
    ))));
    assert_eq!(
        picker(&sandbox).preview,
        Some(("dewey".to_string(), Preview::Loading))
    );
    sandbox.send(AppEv::Screen(ScreenEv::Pets(Ev::PreviewReady(
        "dewey".into(),
    ))));
    assert_eq!(
        picker(&sandbox).preview,
        Some(("dewey".to_string(), Preview::Ready))
    );
}

#[test]
fn enter_selects_the_pet_and_closes() {
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::Pick("codex".into()));
    assert_eq!(
        sandbox.app().screens().pets.chosen,
        vec![Choice::Selected("codex".into())]
    );
    assert!(sandbox.app().screens().pets.open.is_none());
}

#[test]
fn enter_on_the_disable_row_disables_pets() {
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::Pick(DISABLED.into()));
    assert_eq!(sandbox.app().screens().pets.chosen, vec![Choice::Disabled]);
}

#[test]
fn typing_filters_the_pets_and_the_disable_row_answers_to_its_words() {
    let sandbox = Sandbox::play(PLAIN).expect("plays");
    // The search text is the widget's; the model filters by it on request.
    let picker_now = picker(&sandbox);
    assert_eq!(picker_now.visible("dew").len(), 1);
    assert_eq!(
        picker_now.pets[picker_now.visible("dew")[0]].selector,
        "dewey"
    );
    assert_eq!(
        picker_now.pets[picker_now.visible("hidden")[0]].selector,
        DISABLED
    );
    assert_eq!(picker_now.visible("").len(), picker_now.pets.len());
}

#[test]
fn close_drops_the_picker_without_a_choice() {
    // Esc clears a typed search first: that is the widget's, the model sees
    // only the Close that follows.
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::Close);
    assert!(sandbox.app().screens().pets.open.is_none());
    assert!(sandbox.app().screens().pets.chosen.is_empty());
}

#[test]
fn bad_seeds_are_refused() {
    assert!(Sandbox::play(r#"{"step":"screen","dir":"pets","ev":{"kind":"nope"}}"#).is_err());
}
