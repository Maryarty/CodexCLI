use pretty_assertions::assert_eq;

use codex_tui_rtwk::services::transcript_file::*;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

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
