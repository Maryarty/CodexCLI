//! What the settings lists (`/experimental`, `/skills`, `/hooks`, `/memories`) leave to the host:
//! the edits of the config file the app asked for (`Ask::ConfigWrite`), as the oracle's tests
//! read them from the app events (`SaveExperimentalFeatures`, `UpdateMemorySettings`).

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask;

use super::Screen;

impl Screen {
    /// The edits of the config file the app asked the host for, oldest write first: each a
    /// `(key path, value)` list.
    pub fn config_writes(&self) -> Vec<Vec<(String, serde_json::Value)>> {
        self.commands()
            .into_iter()
            .filter_map(|c| match c {
                Command::Effect {
                    ask: Ask::ConfigWrite { edits },
                    ..
                } => Some(
                    edits
                        .into_iter()
                        .map(|edit| (edit.key_path, edit.value))
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    /// The app has asked for no write of the config file (the oracle: no event until the popup
    /// is saved).
    pub fn assert_no_config_write(&self) {
        pretty_assertions::assert_eq!(
            self.config_writes(),
            Vec::<Vec<(String, serde_json::Value)>>::new(),
            "the writes of the config file"
        );
    }

    /// The app asked for one write of the config file, of exactly these edits.
    pub fn assert_config_write(&self, edits: &[(&str, serde_json::Value)]) {
        let want: Vec<(String, serde_json::Value)> = edits
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect();
        pretty_assertions::assert_eq!(
            self.config_writes(),
            vec![want],
            "the writes of the config file"
        );
    }
}
