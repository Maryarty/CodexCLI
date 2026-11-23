//! The host's side of the settings dialogs (ADR-003: the model reads no files; the dialogs open
//! from what it was told). `read` is the config's `[tui] theme`, the custom `.tmTheme` files of
//! `{codex_home}/themes` and the memory tool's switches, which `in_process` sends to the feature
//! as its facts at start and again after every write; `write` is the answer to `Ask::ConfigWrite`
//! (the oracle's `write_config_batch`): the edits go to the config file through the app server
//! and, once it has them, the config is loaded again so the facts the session reads follow. In
//! Flutter this is the repository the settings sheets' `FutureBuilder`s would read.

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_core::config::Config;
use codex_features::Feature;
use codex_tui_core::app::effects::{ConfigEdit, Done};
use codex_tui_core::protocol::config::{config_write, config_written};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::{Startup, Target};

/// What the config and the disk say of the dialogs' facts.
pub(crate) struct Read {
    /// `[tui] theme` of the config: the syntax theme in force.
    pub syntax_theme: Option<String>,
    /// The `.tmTheme` files of `{codex_home}/themes` (file stems), alphabetical.
    pub custom_themes: Vec<String>,
    /// That directory as the subtitle names it (`~/.codex/themes`); none when it is not below
    /// the home directory.
    pub themes_dir: Option<String>,
    /// The memory tool's `use_memories` and `generate_memories`; none: it is off.
    pub memories: Option<(bool, bool)>,
}

/// The facts of the dialogs of `config`; `home` is the user's home directory (the themes
/// directory is named `~/…` below it).
pub(crate) fn read(config: &Config, home: Option<&Path>) -> Read {
    let themes = config.codex_home.join("themes");
    let shown = crate::facts::directory_display(&themes, home);
    Read {
        syntax_theme: config.tui_theme.clone(),
        custom_themes: custom_themes(&themes),
        themes_dir: shown.starts_with('~').then_some(shown),
        memories: config
            .features
            .enabled(Feature::MemoryTool)
            .then_some((config.memories.use_memories, config.memories.generate_memories)),
    }
}

/// The stems of the `.tmTheme` files in `dir`, alphabetical (`theme_picker::list`).
fn custom_themes(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("tmTheme"))
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// How a write of the config file went.
pub(crate) struct Written {
    /// The answer to the ask.
    pub done: Done,
    /// The config as it stands once the file has the edits: loaded again, for the facts that
    /// follow. None: the write failed, the workspace is remote (its config is not this
    /// machine's) or the config could not be loaded again.
    pub config: Option<Arc<Config>>,
}

/// `Ask::ConfigWrite`: `config/batchWrite` of `edits`, then the session's config again.
pub(crate) async fn write<F, Fut>(
    edits: Vec<ConfigEdit>,
    startup: &Startup,
    ids: &Ids,
    send: F,
) -> Written
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let done = match send(config_write(&edits, ids.next())).await {
        Ok(result) => Done::Config(config_written(result)),
        Err(error) => Done::Config(Err(error.message)),
    };
    let config = if matches!(done, Done::Config(Ok(()))) && startup.target != Target::Remote {
        reload(startup).await
    } else {
        None
    };
    Written { done, config }
}

/// The session's config loaded again and kept as the one in force (`ReloadConfig` does the same
/// at launch; a write of a dialog changes no provider, so no server restarts).
async fn reload(startup: &Startup) -> Option<Arc<Config>> {
    let mut slot = startup.current.write().await;
    let config = Arc::new((startup.reload)().await.ok()?);
    *slot = config.clone();
    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_themes_are_the_tmtheme_stems_in_order() {
        let dir = std::env::temp_dir().join(format!("rtwk-themes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["zen.tmTheme", "amber.tmTheme", "notes.txt"] {
            std::fs::write(dir.join(name), "").unwrap();
        }
        assert_eq!(custom_themes(&dir), ["amber", "zen"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_themes_directory_offers_none() {
        assert!(custom_themes(Path::new("/nonexistent/rtwk-themes")).is_empty());
    }
}
