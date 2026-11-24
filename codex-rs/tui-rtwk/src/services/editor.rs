//! The external editor's runner (oracle `external_editor.rs`): the draft is written to a temporary
//! file in an editor directory the sandbox policy does not let the model write to, `$VISUAL` or
//! `$EDITOR` runs on it, and the file's text is read back. In Flutter this is the repository
//! behind `Ask::Editor`. The host gives it the codex home and the sandbox policy of its config. Handing the terminal over to the editor (leaving the alternate
//! screen and raw mode, restoring them after) is the owner of the terminal's: `run` is called
//! inside that handoff.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_tui_core::app::effects::EditorFailure;
use tempfile::Builder;
use tokio::process::Command;

/// Why no editor command came out of the environment.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EditorError {
    /// Neither `VISUAL` nor `EDITOR` is set.
    Missing,
    /// The variable does not parse as a command line.
    ParseFailed,
    /// It parses to nothing.
    Empty,
}

impl EditorError {
    fn failure(self) -> EditorFailure {
        match self {
            EditorError::Missing => EditorFailure::Missing,
            EditorError::ParseFailed => {
                EditorFailure::Failed("failed to parse editor command".into())
            }
            EditorError::Empty => EditorFailure::Failed("editor command is empty".into()),
        }
    }
}

/// The editor command from the environment, `VISUAL` before `EDITOR`.
pub(crate) fn resolve_editor_command() -> Result<Vec<String>, EditorError> {
    let raw = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| EditorError::Missing)?;
    let parts = split(&raw).ok_or(EditorError::ParseFailed)?;
    if parts.is_empty() {
        return Err(EditorError::Empty);
    }
    Ok(parts)
}

/// A command line split into words as a POSIX shell does for quotes and backslashes (the oracle's
/// `shlex::split`): `None` for an unterminated quote.
fn split(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            '\'' => {
                started = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => word.push(c),
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            c @ ('"' | '\\' | '$' | '`') => word.push(c),
                            '\n' => {}
                            c => {
                                word.push('\\');
                                word.push(c);
                            }
                        },
                        c => word.push(c),
                    }
                }
            }
            '\\' => {
                started = true;
                match chars.next()? {
                    '\n' => {}
                    c => word.push(c),
                }
            }
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        words.push(word);
    }
    Some(words)
}

/// The directory the temporary file lives in: `editor/` under the first of the candidate homes
/// that the policy does not let the model write to, without symbolic links.
pub(crate) fn editor_directory(
    candidate_homes: &[&Path],
    policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> Result<PathBuf, String> {
    let mut error = "editor directory must not be writable".to_owned();
    let mut rejected_writable = false;
    for home in candidate_homes {
        let canonical_home = match dunce::canonicalize(home) {
            Ok(path) => path,
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {
                let (Some(parent), Some(name)) = (home.parent(), home.file_name()) else {
                    error = "editor directory has no parent".to_owned();
                    continue;
                };
                match dunce::canonicalize(parent) {
                    Ok(parent) => parent.join(name),
                    Err(failure) => {
                        error = failure.to_string();
                        continue;
                    }
                }
            }
            Err(failure) => {
                error = failure.to_string();
                continue;
            }
        };
        let directory = canonical_home.join("editor");
        let logical = home.join("editor");
        if !policy.has_full_disk_write_access()
            && [&logical, &directory].into_iter().any(|directory| {
                let Some(parent) = directory.parent() else {
                    return true;
                };
                policy.can_write_local_path_with_cwd(directory, cwd)
                    || policy.can_write_local_path_with_cwd(parent, cwd)
                    || policy.get_writable_roots_with_cwd(cwd).iter().any(|root| {
                        root.is_path_writable(directory)
                            || root.root.as_path().starts_with(directory)
                    })
            })
        {
            error = "editor directory must not be writable".to_owned();
            rejected_writable = true;
            continue;
        }
        if let Err(failure) = fs::create_dir_all(&directory) {
            error = failure.to_string();
            continue;
        }
        match dunce::canonicalize(&directory) {
            Ok(path) if path == directory => return Ok(directory),
            Ok(_) => error = "editor directory must not contain symbolic links".to_owned(),
            Err(failure) => error = failure.to_string(),
        }
    }
    if rejected_writable {
        Err("editor directory must not be writable".to_owned())
    } else {
        Err(error)
    }
}

/// Writes `seed` to a temporary file, runs the editor command on it and returns the file's text.
pub(crate) async fn run_editor(
    seed: &str,
    editor: &[String],
    codex_home: &Path,
    policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> Result<String, String> {
    let Some((program, args)) = editor.split_first() else {
        return Err("editor command is empty".to_owned());
    };
    let default_home = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex"));
    let project_home = cwd.join(".codex");
    let mut homes: Vec<&Path> = vec![codex_home];
    homes.extend(default_home.as_deref());
    homes.push(&project_home);
    let directory = editor_directory(&homes, policy, cwd)?;
    // A `TempPath` at once, so no file handle stays open (Windows).
    let path = Builder::new()
        .suffix(".md")
        .tempfile_in(directory)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    fs::write(&path, seed).map_err(|e| e.to_string())?;
    let status = Command::new(program)
        .args(args)
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("editor exited with status {status}"));
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// `Ask::Editor` answered: the editor of the environment on `draft`, with the codex home and
/// the sandbox policy the host read from its config; `cwd` is the thread's directory (empty:
/// `default_cwd`).
pub async fn ask(
    codex_home: &Path,
    policy: &FileSystemSandboxPolicy,
    default_cwd: &Path,
    draft: String,
    cwd: String,
) -> Result<String, EditorFailure> {
    let editor = resolve_editor_command().map_err(EditorError::failure)?;
    let cwd = if cwd.is_empty() {
        default_cwd.to_path_buf()
    } else {
        PathBuf::from(cwd)
    };
    run_editor(&draft, &editor, codex_home, policy, &cwd)
        .await
        .map_err(EditorFailure::Failed)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    type Out = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn a_command_line_splits_like_a_shell() {
        assert_eq!(
            split(r#"code --wait "my file" 'a b' c\ d"#),
            Some(vec![
                "code".to_owned(),
                "--wait".to_owned(),
                "my file".to_owned(),
                "a b".to_owned(),
                "c d".to_owned()
            ])
        );
        assert_eq!(split("vim 'open"), None);
        assert_eq!(split("  "), Some(Vec::new()));
    }

    #[tokio::test]
    async fn the_editor_runs_on_a_file_that_holds_the_draft() -> Out {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        let script = dir.path().join("edit.sh");
        fs::write(&script, "#!/bin/sh\nprintf \"edited\" > \"$1\"\n")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
        let editor = vec![script.to_string_lossy().to_string()];
        let policy = FileSystemSandboxPolicy::read_only();
        let text = run_editor("seed", &editor, dir.path(), &policy, dir.path()).await?;
        assert_eq!(text, "edited");
        Ok(())
    }
}
