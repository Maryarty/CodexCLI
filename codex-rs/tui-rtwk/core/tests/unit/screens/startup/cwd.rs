use codex_tui_core::screens::startup::Action;
use codex_tui_core::screens::startup::CwdSelection;
use codex_tui_core::screens::startup::cwd::*;

fn setting(configured: Option<ResumeMode>) -> Setting {
    Setting {
        current: "/work/now".into(),
        configured,
        ..Setting::default()
    }
}

// session_resume.rs: matching_resume_cwd_skips_prompt_without_configured_mode
#[test]
fn the_same_directory_opens_as_the_session_has_it() {
    assert_eq!(
        plan(Action::Resume, &setting(None), Some("/work/now/")),
        Plan::Open(Some("/work/now/".into()))
    );
}

#[test]
fn different_directories_ask_with_both() {
    let Plan::Ask(prompt) = plan(Action::Fork, &setting(None), Some("/work/then")) else {
        panic!("a prompt is asked");
    };
    assert_eq!(prompt.action, Action::Fork);
    assert_eq!(
        (prompt.current.as_str(), prompt.session.as_str()),
        ("/work/now", "/work/then")
    );
    assert!(prompt.allow_remember_current);
}

// session_resume.rs: configured_resume_cwd_skips_prompt
#[test]
fn a_remembered_mode_skips_the_prompt() {
    assert_eq!(
        plan(
            Action::Resume,
            &setting(Some(ResumeMode::Session)),
            Some("/work/then")
        ),
        Plan::Open(Some("/work/then".into()))
    );
    assert_eq!(
        plan(
            Action::Resume,
            &setting(Some(ResumeMode::Current)),
            Some("/work/then")
        ),
        Plan::Open(Some("/work/now".into()))
    );
}

#[test]
fn cd_counts_as_current() {
    let setting = Setting {
        cwd_override: true,
        configured: Some(ResumeMode::Session),
        ..setting(None)
    };
    assert_eq!(
        plan(Action::Resume, &setting, Some("/work/then")),
        Plan::Open(Some("/work/now".into()))
    );
}

#[test]
fn a_session_with_no_recorded_directory() {
    assert_eq!(plan(Action::Resume, &setting(None), None), Plan::Open(None));
    assert!(matches!(
        plan(Action::Resume, &setting(Some(ResumeMode::Session)), None),
        Plan::Fail(message) if message.contains("recorded for the selected session")
    ));
}

// lib.rs: a remote workspace never prompts; "current" there needs `--cd`.
#[test]
fn a_remote_workspace_opens_in_the_current_directory() {
    let remote = Setting {
        remote: true,
        ..setting(None)
    };
    assert_eq!(
        plan(Action::Resume, &remote, Some("/elsewhere")),
        Plan::Open(Some("/work/now".into()))
    );
    let refused = Setting {
        configured: Some(ResumeMode::Current),
        ..remote
    };
    assert!(matches!(
        plan(Action::Resume, &refused, Some("/elsewhere")),
        Plan::Fail(message) if message.contains("requires `--cd`")
    ));
    let with_cd = Setting {
        cwd_override: true,
        ..refused
    };
    assert_eq!(
        plan(Action::Resume, &with_cd, Some("/elsewhere")),
        Plan::Open(Some("/work/now".into()))
    );
}

#[test]
fn a_selection_names_its_directory_and_what_it_remembers() {
    let Plan::Ask(prompt) = plan(Action::Resume, &setting(None), Some("/work/then")) else {
        panic!("a prompt is asked");
    };
    assert_eq!(chosen(&prompt, CwdSelection::Session), "/work/then");
    assert_eq!(chosen(&prompt, CwdSelection::Current), "/work/now");
    assert_eq!(
        chosen(&prompt, CwdSelection::CurrentAndRemember),
        "/work/now"
    );
    assert_eq!(
        remembered(CwdSelection::SessionAndRemember),
        Some(ResumeMode::Session)
    );
    assert_eq!(remembered(CwdSelection::Current), None);
}
