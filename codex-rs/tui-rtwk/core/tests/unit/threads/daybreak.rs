use codex_tui_core::screens::cells::Daybreak;
use codex_tui_core::threads::daybreak::*;

fn catalog(model: &str, standard: bool, daybreak: bool) -> Access {
    Access {
        eligible: true,
        models: vec![ModelAccess {
            model: model.to_owned(),
            programs: Some(Programs { standard, daybreak }),
        }],
    }
}

// Oracle `daybreak_tests.rs`: Standard only words as Apply, a Daybreak program as Disabled,
// and the setting on as Enabled.
#[test]
fn the_setting_and_the_catalog_choose_the_wording() {
    let model = Some("gpt-5.6");
    assert_eq!(
        catalog("gpt-5.6", true, false).notice(model, false),
        Daybreak::Apply
    );
    assert_eq!(
        catalog("gpt-5.6", true, true).notice(model, false),
        Daybreak::Disabled
    );
    assert_eq!(
        catalog("gpt-5.6", false, true).notice(model, false),
        Daybreak::Disabled
    );
    assert_eq!(
        catalog("gpt-5.6", true, true).notice(model, true),
        Daybreak::Enabled
    );
}

#[test]
fn astra_without_a_daybreak_program_words_as_astra() {
    assert_eq!(
        catalog("gpt-6-astra", true, false).notice(Some("gpt-6-astra"), false),
        Daybreak::Astra
    );
    // an unknown catalog never implies access
    let unknown = Access {
        eligible: true,
        models: Vec::new(),
    };
    assert_eq!(unknown.notice(Some("gpt-6-astra"), false), Daybreak::Apply);
}

// An account that cannot enable Daybreak words as Apply whatever the thread says
// (`self.daybreak_enabled && can_enable_daybreak`).
#[test]
fn an_ineligible_account_words_as_apply() {
    let mut access = catalog("gpt-5.6", true, true);
    access.eligible = false;
    assert_eq!(access.notice(Some("gpt-5.6"), true), Daybreak::Apply);
    assert_eq!(Access::default().notice(None, false), Daybreak::Apply);
}
