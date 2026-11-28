//! Which paths a spoken answer names are files of the workspace (oracle
//! `terminal_hyperlinks.rs` `TrustedWorkspaceFile::validate`): an existing, inert source file
//! that stays inside the workspace after every link is followed. The file is looked at here, in
//! a service, never in a view.

use std::path::Path;

use codex_tui_core::app::effects::WorkspaceFile;
use url::Url;

const SAFE_EXTENSIONS: &[&str] = &[
    "rs", "go", "c", "cc", "cpp", "h", "hpp", "java", "kt", "swift", "toml", "json", "yaml", "yml",
    "md", "txt", "css",
];

/// The `paths` that are files of the local workspace `cwd`, each with its `file:` URL.
pub fn files(cwd: &str, paths: &[String]) -> Vec<WorkspaceFile> {
    paths
        .iter()
        .filter_map(|path| {
            let url = validate(Path::new(cwd), path)?;
            Some(WorkspaceFile {
                path: path.clone(),
                url,
            })
        })
        .collect()
}

fn validate(workspace: &Path, candidate: &str) -> Option<String> {
    if candidate.is_empty()
        || candidate.len() > 512
        || candidate.contains(':')
        || candidate.chars().any(char::is_control)
    {
        return None;
    }
    let normalized = candidate.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute()
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
        || !SAFE_EXTENSIONS.contains(&path.extension()?.to_str()?)
    {
        return None;
    }
    let workspace = workspace.canonicalize().ok()?;
    let target = workspace.join(path).canonicalize().ok()?;
    let relative = target.strip_prefix(&workspace).ok()?;
    if !target.metadata().ok()?.is_file()
        || target.extension() != path.extension()
        || relative
            .iter()
            .any(|part| part.to_string_lossy().starts_with('.'))
    {
        return None;
    }
    Url::from_file_path(target).ok().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::files;

    #[test]
    fn only_a_real_source_file_inside_the_workspace_is_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.path().join("run.sh"), "").unwrap();
        let cwd = dir.path().to_string_lossy().into_owned();
        let asked = [
            "src/lib.rs",
            "src\\lib.rs",
            "src/missing.rs",
            "run.sh",
            "../lib.rs",
            ".hidden/x.rs",
        ]
        .map(str::to_owned);
        let found = files(&cwd, &asked);
        let paths: Vec<_> = found.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["src/lib.rs", "src\\lib.rs"]);
        assert!(found[0].url.starts_with("file://"));
        assert!(found[0].url.ends_with("/src/lib.rs"));
    }
}
