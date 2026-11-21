//! The update run once the terminal is restored. Oracle: the CLI's
//! `run_update_action` (and `resolve_windows_update_command_from_path`).

use std::ffi::OsStr;
use std::path::PathBuf;

use codex_tui_rtwk::services::startup::UpdateAction;
use codex_tui_rtwk::services::startup::run_update_process;

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

/// The CLI's `run_update_action` builds the command per platform: on
/// Windows the program is found on the absolute `PATH` entries only
/// (`resolve_windows_update_command_from_path`), the standalone installer
/// goes through `powershell.exe`, and the update runs in a fresh directory;
/// elsewhere it runs from `/` with WSL-mapped paths.
pub fn update_command(
    action: UpdateAction,
    windows: bool,
    path_env: Option<&OsStr>,
) -> Result<UpdateCommand, String> {
    let (program, args) = action.command_args();
    if !windows {
        return Ok(UpdateCommand {
            program: normalize_for_wsl(program).into(),
            args: args.iter().map(|arg| normalize_for_wsl(arg)).collect(),
            cwd: UpdateCwd::Root,
        });
    }
    let program = if action == UpdateAction::StandaloneWindows {
        // The arguments hold PowerShell metacharacters: PATHEXT must not
        // pick a batch shim for this action.
        "powershell.exe"
    } else {
        program
    };
    let path_env = path_env.ok_or_else(|| "PATH is not set".to_owned())?;
    let absolute =
        std::env::join_paths(std::env::split_paths(path_env).filter(|path| path.is_absolute()))
            .map_err(|error| error.to_string())?;
    if absolute.is_empty() {
        return Err(format!(
            "Could not find an absolute update command `{program}` on PATH. Please update manually: https://developers.openai.com/codex/cli/"
        ));
    }
    let found = which::which_in_global(program, Some(&absolute))
        .map_err(|error| error.to_string())?
        .next()
        .ok_or_else(|| format!("could not find update command `{program}` on PATH"))?;
    Ok(UpdateCommand {
        program: found,
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        cwd: UpdateCwd::Fresh,
    })
}

/// The CLI's `wsl_paths::normalize_for_wsl`: under WSL a Windows drive path
/// becomes its `/mnt/<drive>/...` mount; anything else is unchanged.
fn normalize_for_wsl(value: &str) -> String {
    if !codex_utils_path::is_wsl() {
        return value.to_owned();
    }
    let bytes = value.as_bytes();
    if bytes.len() < 3
        || bytes[1] != b':'
        || !(bytes[2] == b'\\' || bytes[2] == b'/')
        || !bytes[0].is_ascii_alphabetic()
    {
        return value.to_owned();
    }
    let drive = (bytes[0] as char).to_ascii_lowercase();
    let tail = value[3..].replace('\\', "/");
    if tail.is_empty() {
        format!("/mnt/{drive}")
    } else {
        format!("/mnt/{drive}/{tail}")
    }
}

/// The CLI's `run_update_action`: after the terminal is restored, the update
/// runs where no project config steers it.
pub fn run_update(action: UpdateAction) -> Result<(), String> {
    let command = action.command_str();
    let path_env = std::env::var_os("PATH");
    let plan = update_command(action, cfg!(windows), path_env.as_deref())?;
    // Kept alive until the update ends.
    let fresh = match plan.cwd {
        UpdateCwd::Root => None,
        UpdateCwd::Fresh => Some(tempfile::tempdir().map_err(|error| error.to_string())?),
    };
    let cwd = fresh
        .as_ref()
        .map_or(std::path::Path::new("/"), |dir| dir.path());
    run_update_process(&command, &plan.program, &plan.args, cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory holding an executable per name.
    fn bin_dir(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for name in names {
            // A link to an executable is one.
            std::os::unix::fs::symlink("/bin/sh", dir.path().join(name)).expect("link");
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
