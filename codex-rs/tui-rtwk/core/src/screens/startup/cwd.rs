//! The working directory a resumed or forked session opens in. Oracle:
//! `session_resume.rs` (`effective_resume_cwd_mode`,
//! `resolve_cwd_for_resume_or_fork`, `cwds_differ`) and the cwd of the
//! selection in `lib.rs` (`resolve_cwd_for_resume_or_fork_selection`).
//!
//! Pure: the host reads what the oracle reads from the config and the command
//! line (`Setting`), the app reads the session (`thread/read`), and the plan
//! says whether the session opens as it is, a prompt asks, or the launch ends.

use super::{Action, CwdPrompt, CwdSelection};

/// `tui.resume_cwd` (`ResumeCwdMode`): the answer the user remembered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeMode {
    /// Always the directory the app was started in.
    Current,
    /// Always the directory the session recorded.
    Session,
}

/// What the host reads of the config and the command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Setting {
    /// The directory the app was started in (`config.cwd`).
    pub current: String,
    /// `tui.resume_cwd`.
    pub configured: Option<ResumeMode>,
    /// `--cd` was given: it counts as `tui.resume_cwd = "current"`
    /// (`effective_resume_cwd_mode`).
    pub cwd_override: bool,
    /// A remote workspace or environment: the server's directories are not
    /// this machine's (`uses_remote_workspace_or_environment`).
    pub remote: bool,
}

impl Setting {
    /// `effective_resume_cwd_mode`.
    pub fn mode(&self) -> Option<ResumeMode> {
        if self.cwd_override {
            Some(ResumeMode::Current)
        } else {
            self.configured
        }
    }
}

/// What opening the session asks of the launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// Open in this directory (`None`: the one the server recorded).
    Open(Option<String>),
    /// The directories differ and nothing was remembered: the cwd prompt.
    Ask(CwdPrompt),
    /// The launch ends with this message.
    Fail(String),
}

/// `resolve_cwd_for_resume_or_fork_selection`. `session` is the directory the
/// session recorded (`thread.cwd`), none where it could not be read.
pub fn plan(action: Action, setting: &Setting, session: Option<&str>) -> Plan {
    let mode = setting.mode();
    if setting.remote && !setting.cwd_override && mode == Some(ResumeMode::Current) {
        return Plan::Fail(
            "`tui.resume_cwd = \"current\"` requires `--cd` when using a remote workspace"
                .to_owned(),
        );
    }
    if setting.remote {
        return Plan::Open(Some(setting.current.clone()));
    }
    if mode == Some(ResumeMode::Current) {
        return Plan::Open(Some(setting.current.clone()));
    }
    let Some(session) = session else {
        return match mode {
            Some(ResumeMode::Session) => Plan::Fail(
                "failed to determine the working directory recorded for the selected session"
                    .to_owned(),
            ),
            _ => Plan::Open(None),
        };
    };
    if mode == Some(ResumeMode::Session) || same_directory(&setting.current, session) {
        return Plan::Open(Some(session.to_owned()));
    }
    Plan::Ask(CwdPrompt {
        action,
        current: setting.current.clone(),
        session: session.to_owned(),
        remembered_current: setting.current.clone(),
        allow_remember_current: !setting.remote || setting.cwd_override,
    })
}

/// The directory a selection opens the session in.
pub fn chosen(prompt: &CwdPrompt, selection: CwdSelection) -> &str {
    match selection {
        CwdSelection::Current => &prompt.current,
        CwdSelection::CurrentAndRemember => &prompt.remembered_current,
        CwdSelection::Session | CwdSelection::SessionAndRemember => &prompt.session,
    }
}

/// `CwdSelection::remembered_mode`: what a selection writes to the config.
pub fn remembered(selection: CwdSelection) -> Option<ResumeMode> {
    match selection {
        CwdSelection::CurrentAndRemember => Some(ResumeMode::Current),
        CwdSelection::SessionAndRemember => Some(ResumeMode::Session),
        CwdSelection::Current | CwdSelection::Session => None,
    }
}

/// `cwds_differ` negated: the same directory, a trailing separator aside (the
/// oracle also resolves links, which a core with no files cannot).
fn same_directory(left: &str, right: &str) -> bool {
    let trim = |path: &str| {
        let trimmed = path.trim_end_matches(['/', '\\']);
        if trimmed.is_empty() { path.to_owned() } else { trimmed.to_owned() }
    };
    trim(left) == trim(right)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            plan(Action::Resume, &setting(Some(ResumeMode::Session)), Some("/work/then")),
            Plan::Open(Some("/work/then".into()))
        );
        assert_eq!(
            plan(Action::Resume, &setting(Some(ResumeMode::Current)), Some("/work/then")),
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
        assert_eq!(chosen(&prompt, CwdSelection::CurrentAndRemember), "/work/now");
        assert_eq!(remembered(CwdSelection::SessionAndRemember), Some(ResumeMode::Session));
        assert_eq!(remembered(CwdSelection::Current), None);
    }
}
