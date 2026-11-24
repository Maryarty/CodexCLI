//! The seed form of the screen: a fixture script settles the loads directly.

use codex_protocol::account::PlanType;
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::usage::{Load, LoadError, Model, Note, Section, UsageChoice, View};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn play(steps: &[&str]) -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{}\n", steps.join("\n"))).expect("plays")
}

/// What the page would choose over the seed the fixture left in the model.
fn chosen(model: &Model) -> UsageChoice {
    match &model.seeded {
        Some(seed) => UsageChoice::seeded(seed, model),
        None => UsageChoice::default(),
    }
}

#[test]
fn a_fresh_model_has_no_account_and_loads_nothing() {
    let sandbox = play(&[]);
    let model = &sandbox.app().screens().usage;
    assert!(model.account == Load::Unavailable);
    assert!(model.visible_sections().is_empty());
    assert_eq!(chosen(model).section, Section::Summary);
}

#[test]
fn a_seed_settles_the_account_the_reports_and_the_profile() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"usage","ev":{
            "account":{"plan":"plus","label":"Example User","today":"2026-09-09"},
            "section":"usage",
            "histories":[{"section":"usage","load":{"ready":{"unit":"tokens","updated_at":null,"data":[]}}},
                         {"section":"activity","load":"loading"},
                         {"section":"skills","load":{"error":"temporary failure"}}],
            "profile":{"ready":{"stats":{"lifetime_tokens":142300000000}}}
        }}"#
        .replace('\n', " ")
        .as_str(),
    ]);
    let model = &sandbox.app().screens().usage;
    assert_eq!(model.account.ready().copied(), Some(PlanType::Plus));
    let sel = chosen(model);
    let view = View::new(model, &sel);
    assert_eq!(sel.section, Section::Usage);
    assert_eq!(model.end_date.to_string(), "2026-09-09");
    assert!(view.history(Section::Usage).ready().is_some());
    assert!(view.history(Section::Activity).is_loading());
    assert_eq!(
        view.history(Section::Skills).note(),
        Some(Note::Error(LoadError::Host("temporary failure".to_owned())))
    );
    assert_eq!(
        model.profile.ready().unwrap().stats.lifetime_tokens,
        Some(142_300_000_000)
    );
}

#[test]
fn a_seed_of_a_section_the_account_hides_falls_back_to_the_first() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"usage","ev":{"account":{"plan":"plus","today":"2026-09-09"},"section":"credits"}}"#,
    ]);
    assert_eq!(chosen(&sandbox.app().screens().usage).section, Section::Summary);
}

#[test]
fn an_unknown_key_in_a_seed_is_refused() {
    let err = Sandbox::play(&format!(
        "{SESSION}\n{}\n",
        r#"{"step":"screen","dir":"usage","ev":{"acount":{}}}"#
    ));
    assert!(err.is_err());
}
