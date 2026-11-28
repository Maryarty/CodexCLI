use codex_tui_core::screens::cells::Daybreak;
use codex_tui_core::threads::daybreak::*;

fn catalog(model: &str, standard: bool, daybreak: bool) -> Access {
    Access {
        account: true,
        providers: vec!["openai".to_owned()],
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
        catalog("gpt-5.6", true, false).notice(Some("openai"), model, false),
        Daybreak::Apply
    );
    assert_eq!(
        catalog("gpt-5.6", true, true).notice(Some("openai"), model, false),
        Daybreak::Disabled
    );
    assert_eq!(
        catalog("gpt-5.6", false, true).notice(Some("openai"), model, false),
        Daybreak::Disabled
    );
    assert_eq!(
        catalog("gpt-5.6", true, true).notice(Some("openai"), model, true),
        Daybreak::Enabled
    );
}

#[test]
fn astra_without_a_daybreak_program_words_as_astra() {
    assert_eq!(
        catalog("gpt-6-astra", true, false).notice(Some("openai"), Some("gpt-6-astra"), false),
        Daybreak::Astra
    );
    // an unknown catalog never implies access
    let unknown = Access {
        account: true,
        providers: vec!["openai".to_owned()],
        models: Vec::new(),
    };
    assert_eq!(
        unknown.notice(Some("openai"), Some("gpt-6-astra"), false),
        Daybreak::Apply
    );
}

// An account that cannot enable Daybreak words as Apply whatever the thread says
// (`self.daybreak_enabled && can_enable_daybreak`).
#[test]
fn an_ineligible_account_words_as_apply() {
    let mut access = catalog("gpt-5.6", true, true);
    access.account = false;
    assert_eq!(
        access.notice(Some("openai"), Some("gpt-5.6"), true),
        Daybreak::Apply
    );
    assert_eq!(Access::default().notice(None, None, false), Daybreak::Apply);
}

// `config.model_provider_id` follows the thread's session and its settings updates: the account
// is eligible for the provider the thread runs on (`supports_cyber_access_programs`), and for no
// thread without one.
#[test]
fn eligibility_follows_the_provider_the_thread_runs_on() {
    let access = catalog("gpt-5.6", true, true);
    assert!(access.eligible(Some("openai")));
    assert!(!access.eligible(Some("server-updated")));
    assert!(!access.eligible(None));
    assert_eq!(
        access.notice(Some("server-updated"), Some("gpt-5.6"), false),
        Daybreak::Apply
    );
}

// Oracle `daybreak_command_description`: the entry is listed for a thread whose setting is on or
// whose provider the account is eligible for, and says what the catalog allows.
#[test]
fn the_entry_is_listed_by_the_setting_or_the_eligible_account() {
    let sol = catalog("gpt-5.6", true, true);
    assert_eq!(sol.row(false, Some("openai")), Some(Row::Enable));
    assert_eq!(sol.row(false, Some("other")), None);
    assert_eq!(sol.row(true, Some("other")), Some(Row::Disable));
    assert_eq!(
        catalog("gpt-5.6", true, false).row(false, Some("openai")),
        Some(Row::Learn)
    );
    let unknown = Access {
        models: Vec::new(),
        ..sol
    };
    assert_eq!(unknown.row(false, Some("openai")), Some(Row::Manage));
}
