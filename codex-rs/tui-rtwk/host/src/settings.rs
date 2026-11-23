//! The host's side of the settings dialogs: the config's `[tui] theme`, the custom `.tmTheme`
//! files of `{codex_home}/themes` and the memory tool's switches, read once at start and sent as
//! `SettingsEv::Facts` (ADR-003: the model reads no files; the sheets open from what it was
//! told). In Flutter this is the repository the settings sheets' `FutureBuilder`s would read.

use std::path::Path;

use codex_core::config::Config;
use codex_features::Feature;
use codex_tui_core::screens::settings::{Facts, MemoryFacts};

/// The facts of the dialogs of `config`; `home` is the user's home directory (the themes
/// directory is named `~/…` below it).
pub(crate) fn facts(config: &Config, home: Option<&Path>) -> Facts {
    let themes = config.codex_home.join("themes");
    let shown = crate::facts::directory_display(&themes, home);
    Facts {
        syntax_theme: config.tui_theme.clone(),
        custom_themes: custom_themes(&themes),
        themes_dir: shown.starts_with('~').then_some(shown),
        memories: config
            .features
            .enabled(Feature::MemoryTool)
            .then_some(MemoryFacts {
                use_memories: config.memories.use_memories,
                generate_memories: config.memories.generate_memories,
            }),
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
