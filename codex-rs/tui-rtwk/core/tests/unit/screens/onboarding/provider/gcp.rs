use codex_tui_core::screens::onboarding::auth::provider::Effect;
use codex_tui_core::screens::onboarding::auth::provider::Setup;
use codex_tui_core::screens::onboarding::auth::provider::gcp::*;

fn fields(project: &str, location: &str, credential: &str) -> Fields {
    Fields {
        project: project.into(),
        location: location.into(),
        credential_file: credential.into(),
    }
}

fn next(gcp: &mut Gcp, fields: Fields) -> Effect {
    gcp.apply(Intent::Next(fields), &mut || 7)
}

// gcp_tests.rs:16 gcp_menu_gating_and_setup_flow (the wizard's part)
#[test]
fn an_empty_project_is_refused_then_the_stages_follow() {
    let mut gcp = Gcp::default();
    assert_eq!(next(&mut gcp, fields("  ", "", "")), Effect::None);
    assert_eq!(gcp.error, Some(Error::EnterProject));
    assert_eq!(gcp.stage, Stage::Project);
    next(&mut gcp, fields("test-project", "", ""));
    assert_eq!((gcp.stage, gcp.error.clone()), (Stage::Location, None));
    next(&mut gcp, fields("test-project", "us-central1", ""));
    assert_eq!(gcp.stage, Stage::Credentials);
}

#[test]
fn esc_goes_one_stage_back_and_leaves_from_the_first() {
    let mut gcp = Gcp {
        stage: Stage::Credentials,
        error: Some(Error::EnterProject),
    };
    assert_eq!(gcp.back(), Effect::None);
    assert_eq!((gcp.stage, gcp.error.clone()), (Stage::Location, None));
    gcp.back();
    assert_eq!(gcp.back(), Effect::Leave);
}

#[test]
fn the_credential_stage_asks_for_the_setup_and_waits() {
    let mut gcp = Gcp {
        stage: Stage::Credentials,
        error: None,
    };
    let effect = next(&mut gcp, fields(" test-project ", "", " /adc.json "));
    assert_eq!(
        effect,
        Effect::Ask(
            7,
            Setup::Gcp(Params {
                project_id: "test-project".into(),
                location: None,
                credential_file: Some("/adc.json".into()),
            })
        )
    );
    assert_eq!(gcp.stage, Stage::Configuring(7));
    // No second submission and no cancel while the server works.
    assert_eq!(next(&mut gcp, fields("p", "", "")), Effect::None);
    assert_eq!(gcp.back(), Effect::None);
    assert_eq!(gcp.stage, Stage::Configuring(7));
    assert!(!gcp.is_text_entry());
}

#[test]
fn a_refusal_returns_to_the_credentials_and_a_stale_answer_is_dropped() {
    let mut gcp = Gcp {
        stage: Stage::Configuring(7),
        error: None,
    };
    assert!(!gcp.answer(6, Ok(())));
    assert_eq!(gcp.stage, Stage::Configuring(7));
    assert!(!gcp.answer(7, Err("must be absolute".into())));
    assert_eq!(gcp.stage, Stage::Credentials);
    assert_eq!(gcp.error, Some(Error::Server("must be absolute".into())));
    gcp.stage = Stage::Configuring(8);
    assert!(gcp.answer(8, Ok(())));
}
