//! The update run once the terminal is restored. Oracle: the CLI's
//! `run_update_action` (and `resolve_windows_update_command_from_path`).

use std::ffi::OsStr;
use std::path::PathBuf;

use codex_tui_rtwk::services::startup::UpdateAction;

/// Where an update runs: the CLI keeps project config and hooks out of the
/// package manager's reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateCwd {
    /// `/` (not Windows).
    Root,
    /// A fresh temporary directory (Windows).
    Fresh,
}

/// The process an update starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: UpdateCwd,
}

pub fn update_command(
    _action: UpdateAction,
    _windows: bool,
    _path_env: Option<&OsStr>,
) -> Result<UpdateCommand, String> {
    Err("not built".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory holding an executable file per name.
    fn bin_dir(names: &[&str]) -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        for name in names {
            let file = dir.path().join(name);
            std::fs::write(&file, "#!/bin/sh\n").expect("write");
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        dir
    }

    #[test]
    fn unix_updates_run_from_the_root_directory() {
        let plan = update_command(UpdateAction::NpmGlobalLatest, false, None).expect("a plan");
        assert_eq!(plan.program, PathBuf::from("npm"));
        assert_eq!(plan.args, ["install", "-g", "@openai/codex"]);
        assert_eq!(plan.cwd, UpdateCwd::Root);
    }

    #[test]
    fn windows_updates_run_in_a_fresh_directory_with_the_command_found_on_path() {
        let dir = bin_dir(&["npm"]);
        let plan = update_command(
            UpdateAction::NpmGlobalLatest,
            true,
            Some(dir.path().as_os_str()),
        )
        .expect("a plan");
        assert_eq!(plan.program, dir.path().join("npm"));
        assert_eq!(plan.args, ["install", "-g", "@openai/codex"]);
        assert_eq!(plan.cwd, UpdateCwd::Fresh);
    }

    #[test]
    fn the_windows_standalone_installer_goes_through_powershell_exe() {
        let dir = bin_dir(&["powershell.exe", "powershell"]);
        let plan = update_command(
            UpdateAction::StandaloneWindows,
            true,
            Some(dir.path().as_os_str()),
        )
        .expect("a plan");
        assert_eq!(plan.program, dir.path().join("powershell.exe"));
        assert_eq!(plan.args[0], "-ExecutionPolicy");
    }

    #[test]
    fn windows_ignores_relative_path_entries() {
        let dir = bin_dir(&["npm"]);
        let path = std::env::join_paths(["bin".as_ref(), dir.path().as_os_str()]).expect("join");
        let plan = update_command(UpdateAction::NpmGlobalLatest, true, Some(&path)).expect("plan");
        assert_eq!(plan.program, dir.path().join("npm"));
        let only_relative = update_command(
            UpdateAction::NpmGlobalLatest,
            true,
            Some(OsStr::new("bin:tools")),
        );
        assert_eq!(
            only_relative.expect_err("no absolute entry"),
            "Could not find an absolute update command `npm` on PATH. Please update manually: https://developers.openai.com/codex/cli/"
        );
    }

    #[test]
    fn windows_without_a_path_or_the_command_fails_with_the_oracles_words() {
        assert_eq!(
            update_command(UpdateAction::NpmGlobalLatest, true, None).expect_err("no PATH"),
            "PATH is not set"
        );
        let empty = bin_dir(&[]);
        let error = update_command(
            UpdateAction::NpmGlobalLatest,
            true,
            Some(empty.path().as_os_str()),
        )
        .expect_err("no npm");
        assert!(error.contains("npm"), "{error}");
    }
}
