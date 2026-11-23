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
