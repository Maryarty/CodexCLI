//! The `/export` file (oracle `app/transcript_export.rs`, `write_transcript`):
//! the conversation as Markdown written to the path the user named, relative
//! to the workspace, under `~`, or absolute. A file that exists is never
//! overwritten, and the content is written whole or not at all (a temporary
//! file in the destination directory, then a no-clobber rename).
//!
//! The Markdown itself is core's (built from the loaded turns); this only
//! writes it. The failure texts are the oracle's.

use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

/// Write `markdown` to `requested`: `~/...` is under the home directory, a
/// relative path is under `cwd`. Returns the file written.
pub fn write_transcript(
    home: Option<PathBuf>,
    cwd: &Path,
    requested: &Path,
    markdown: &str,
) -> Result<PathBuf, String> {
    let path = if let Ok(relative) = requested.strip_prefix("~") {
        home.ok_or_else(|| "could not determine the home directory".to_string())?
            .join(relative)
    } else if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        cwd.join(requested)
    };
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(cwd))
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    file.write_all(markdown.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    file.persist_noclobber(&path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    Ok(path)
}

/// [`write_transcript`] off the async workers, under the user's home
/// directory. The answer of the `/export` command: the path, or the text of
/// the failure.
pub async fn export(cwd: PathBuf, requested: PathBuf, markdown: String) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || {
        write_transcript(std::env::home_dir(), &cwd, &requested, &markdown)
    })
    .await
    .map_err(|error| format!("the export did not finish: {error}"))?
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn a_relative_path_is_under_the_workspace_and_an_existing_file_is_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();

        let path = write_transcript(
            None,
            directory.path(),
            "conversation.md".as_ref(),
            "# First export\n",
        )
        .unwrap();

        assert_eq!(path, directory.path().join("conversation.md"));
        assert!(write_transcript(None, directory.path(), &path, "# Second export\n").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# First export\n");
    }

    #[test]
    fn an_absolute_path_ignores_the_workspace() {
        let workspace = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let target = elsewhere.path().join("out.md");

        let path = write_transcript(None, workspace.path(), &target, "# Export\n").unwrap();

        assert_eq!(path, target);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "# Export\n");
    }

    #[test]
    fn a_tilde_path_is_under_the_home_directory() {
        let home = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();

        let path = write_transcript(
            Some(home.path().to_owned()),
            workspace.path(),
            Path::new("~").join("notes.md").as_path(),
            "# Export\n",
        )
        .unwrap();

        assert_eq!(path, home.path().join("notes.md"));
    }

    #[test]
    fn a_tilde_path_without_a_home_directory_is_refused() {
        let workspace = tempfile::tempdir().unwrap();

        assert_eq!(
            write_transcript(None, workspace.path(), "~/notes.md".as_ref(), "x"),
            Err("could not determine the home directory".to_string())
        );
    }

    #[test]
    fn a_missing_parent_directory_names_the_path_it_could_not_create() {
        let workspace = tempfile::tempdir().unwrap();
        let missing = workspace.path().join("no-such-dir").join("out.md");

        let error = write_transcript(None, workspace.path(), &missing, "x").unwrap_err();

        assert!(
            error.starts_with(&format!("could not create {}: ", missing.display())),
            "{error}"
        );
    }

    #[tokio::test]
    async fn the_async_entry_writes_under_the_workspace() {
        let workspace = tempfile::tempdir().unwrap();

        let path = export(
            workspace.path().to_owned(),
            PathBuf::from("chat.md"),
            "# Codex conversation\n".to_owned(),
        )
        .await
        .unwrap();

        assert_eq!(path, workspace.path().join("chat.md"));
    }
}
