//! The settings slice's host half (ADR-003: the model reads no files; the dialogs open from what
//! it was told): the config's `[tui] theme`, the custom `.tmTheme` files of
//! `{codex_home}/themes`, the memory tool's switches, the rows of the configuration panel and
//! the keymap picker's tabs, sent to the slice as its `Fact` at start and again after every
//! write of the config file. In Flutter this is the repository the settings sheets'
//! `FutureBuilder`s would read.

use std::path::Path;

use codex_config::types::{
    CopyOnSelect, NotificationCondition, ResumeCwdMode, RightClickPaste, SessionPickerViewMode,
};
use codex_core::config::Config;
use codex_features::Feature;
use codex_tui_core::features::settings::keymap;
use codex_tui_core::features::settings::{
    Fact, Facts, MemoryFacts, Row, Section, SettingId, Value,
};
use codex_tui_core::keymap::KeymapSpec;

use super::{Event, Feed, FeedCx};

/// The facts of the slice: at start, and after a write of the config file.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    if matches!(event, Event::Startup | Event::ConfigWritten) {
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
        out.push(Feed::fact(fact(
            cx.config,
            home.as_deref(),
            cx.alternate_screen,
        )));
    }
}

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
        memories: config.features.enabled(Feature::MemoryTool).then_some((
            config.memories.use_memories,
            config.memories.generate_memories,
        )),
    }
}

/// What the dialogs of this config read: the facts of `read`, the rows of the configuration panel
/// and the tabs of the keymap picker (`home` as in `read`). Sent at start and after every write.
fn fact(config: &Config, home: Option<&Path>, alternate_screen: bool) -> Fact {
    let read = read(config, home);
    Fact::Facts(Facts {
        syntax_theme: read.syntax_theme,
        custom_themes: read.custom_themes,
        themes_dir: read.themes_dir,
        memories: read
            .memories
            .map(|(use_memories, generate_memories)| MemoryFacts {
                use_memories,
                generate_memories,
            }),
        config_rows: config_rows(config, &PasteEnvironment::detect(), alternate_screen),
        keymap_tabs: crate::facts::keymap(config)
            .map(|resolved| keymap::picker(&resolved, keymap::Filter::default()).tabs)
            .unwrap_or_default(),
        keymap_spec: serde_json::to_value(&config.tui_keymap)
            .map(|value| KeymapSpec::from_json(&value))
            .unwrap_or_default(),
    })
}

/// What the machine says of right-click paste (`PasteEnvironment`): the terminal the oracle asks
/// for the primary clipboard and the VS Code detection; here the platform and the session.
pub(crate) struct PasteEnvironment {
    platform_default: bool,
    ssh: bool,
    wsl: bool,
    vscode: bool,
}

impl PasteEnvironment {
    pub(crate) fn detect() -> Self {
        Self {
            platform_default: cfg!(any(target_os = "windows", target_os = "linux")),
            ssh: std::env::var_os("SSH_TTY").is_some()
                || std::env::var_os("SSH_CONNECTION").is_some(),
            wsl: codex_utils_path::is_wsl(),
            vscode: std::env::var("TERM_PROGRAM")
                .is_ok_and(|program| program.eq_ignore_ascii_case("vscode")),
        }
    }

    /// The effective state of the setting, or why the terminal must handle the paste itself
    /// (`PasteEnvironment::status`).
    fn status(&self, mode: RightClickPaste, owned: bool) -> Result<bool, &'static str> {
        if mode == RightClickPaste::Off {
            return Ok(false);
        }
        if !owned {
            return Err("Unavailable in inline mode; use your terminal's paste command");
        }
        if self.ssh {
            return Err("Unavailable over SSH; use your terminal's paste command");
        }
        if self.vscode {
            return Err("VS Code handles pasting; use its terminal paste command");
        }
        if mode == RightClickPaste::Auto && self.wsl {
            return Err(
                "Auto cannot identify this WSL terminal; use On or your terminal's paste command",
            );
        }
        match mode {
            RightClickPaste::On if cfg!(target_os = "android") => Err("Unavailable on Android"),
            RightClickPaste::On => Ok(true),
            RightClickPaste::Auto => Ok(self.platform_default),
            RightClickPaste::Off => Ok(false),
        }
    }
}

const RIGHT_CLICK_PASTE: &str = "Paste when nothing is selected, with Auto following platform defaults and On also enabling macOS";

fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}

/// The rows of the configuration panel (`config_panel::settings`), in the oracle's order, from
/// the config as it stands.
fn config_rows(config: &Config, paste: &PasteEnvironment, alternate_screen: bool) -> Vec<Row> {
    let owned = crate::terminal_mode::owns_transcript(config, alternate_screen);
    let copy = crate::effects::tui_settings(config).copy_on_select;
    let notifications = &config.tui_notifications;
    let enabled = !matches!(
        notifications.notifications,
        codex_config::types::Notifications::Enabled(false)
    );
    let custom_filters = matches!(
        notifications.notifications,
        codex_config::types::Notifications::Custom(_)
    );
    let right_click = paste.status(config.tui_right_click_paste, owned);
    let row = |id, section, label: &str, description: &str, keywords: &str, value| Row {
        id,
        section,
        label: label.to_owned(),
        description: description.to_owned(),
        keywords: keywords.to_owned(),
        value,
    };
    let choice = |label: &str| Value::Choice(label.to_owned());
    use Section::*;
    use SettingId as Id;
    let mut rows = vec![
        row(
            Id::Theme,
            Appearance,
            "Theme",
            "Choose a syntax color palette with a live preview",
            "colors dark light palette",
            choice(config.tui_theme.as_deref().unwrap_or("Automatic")),
        ),
        row(
            Id::Effects,
            Appearance,
            "Effects",
            "Configure animations and composer effects",
            "animations motion accessibility spinner starfield stars composer stars",
            Value::Configure,
        ),
        row(
            Id::Tips,
            Appearance,
            "Tips",
            "Show helpful shortcuts and suggestions",
            "welcome hints tooltips",
            Value::Toggle(config.show_tooltips),
        ),
        row(
            Id::StatusLineColors,
            Appearance,
            "Status line colors",
            "Use theme colors for the model, branch, and other status line details",
            "statusline footer colour",
            Value::Toggle(config.tui_status_line_use_colors),
        ),
        row(
            Id::StatusLineDisplay,
            Appearance,
            "Status line display",
            "Choose and reorder the information below the composer",
            "statusline footer model branch context",
            Value::Configure,
        ),
        row(
            Id::TerminalTitleDisplay,
            Appearance,
            "Terminal title display",
            "Choose and reorder the information in your terminal tab or window title",
            "title tab window name",
            Value::Configure,
        ),
        row(
            Id::Notifications,
            Section::Notifications,
            "Notifications",
            "Let your terminal notify you when a turn finishes or approval is needed",
            "alerts bell completion approval",
            if custom_filters {
                Value::Unavailable
            } else {
                Value::Toggle(enabled)
            },
        ),
        row(
            Id::NotifyWhen,
            Section::Notifications,
            "Notify when",
            "Notify only while you are away or while this terminal is focused too",
            "focus alerts background always unfocused",
            if enabled {
                choice(match notifications.condition {
                    NotificationCondition::Unfocused => "Unfocused",
                    NotificationCondition::Always => "Always",
                })
            } else {
                Value::Unavailable
            },
        ),
        row(
            Id::TestNotification,
            Section::Notifications,
            "Test notification",
            "Send in 5 seconds so you can switch apps before the notification appears",
            "alerts bell sound test",
            Value::Test,
        ),
        row(
            Id::KeyboardShortcuts,
            Input,
            "Keyboard shortcuts",
            "Browse actions and remap their keys",
            "keymap bindings hotkeys",
            Value::Configure,
        ),
        row(
            Id::StartInVimMode,
            Input,
            "Start in Vim mode",
            "Start new composers in Vim mode and use /vim for the current composer",
            "editor modal normal",
            Value::Toggle(config.tui_vim_mode_default),
        ),
        row(
            Id::PasteDetection,
            Input,
            "Paste detection",
            "Recognize pasted-text bursts and keep multiline input together",
            "burst clipboard multiline",
            Value::Toggle(!config.disable_paste_burst),
        ),
        row(
            Id::AutomaticRecaps,
            Sessions,
            "Automatic recaps",
            "Prepare recaps while the terminal is unfocused",
            "summary background",
            Value::Toggle(config.tui_auto_recap),
        ),
        row(
            Id::SessionPickerLayout,
            Sessions,
            "Session picker layout",
            "Choose tighter rows or a roomier list when resuming and forking conversations",
            "dense comfortable spacing resume fork",
            choice(match config.tui_session_picker_view {
                SessionPickerViewMode::Comfortable => "Comfortable",
                SessionPickerViewMode::Dense => "Dense",
            }),
        ),
        row(
            Id::ResumeDirectory,
            Sessions,
            "Resume directory",
            "Choose which working directory to use when the current directory and saved session differ",
            "cwd folder path fork ask",
            choice(match config.tui_resume_cwd {
                None => "Ask",
                Some(ResumeCwdMode::Current) => "Current directory",
                Some(ResumeCwdMode::Session) => "Session directory",
            }),
        ),
        row(
            Id::ModelProviders,
            Sessions,
            "Model providers",
            "Choose the default provider for new chats on the connected server",
            "provider gateway endpoint model default",
            choice("Choose"),
        ),
        row(
            Id::CopyOnSelect,
            TextSelection,
            "Copy on select",
            "Copy transcript, footer, and composer selections on release; Auto follows terminal defaults",
            "clipboard mouse selection automatic drag",
            choice(&match config.tui_copy_on_select {
                CopyOnSelect::Auto => format!("Auto · {}", on_off(copy)),
                CopyOnSelect::Always => "On".to_owned(),
                CopyOnSelect::Never => "Off".to_owned(),
            }),
        ),
        row(
            Id::DeselectAfterCopy,
            TextSelection,
            "Deselect after copy",
            "Remove read-only transcript and footer highlights after automatic copy succeeds",
            "clipboard deselect clear highlight automatic mouse selection",
            if copy {
                Value::Toggle(config.tui_copy_on_select_clear_selection)
            } else {
                Value::Unavailable
            },
        ),
        row(
            Id::RightClickPaste,
            TextSelection,
            "Right-click paste",
            right_click.err().unwrap_or(RIGHT_CLICK_PASTE),
            "clipboard mouse paste automatic",
            choice(&{
                let preference = match config.tui_right_click_paste {
                    RightClickPaste::Auto => "Auto",
                    RightClickPaste::On => "On",
                    RightClickPaste::Off => "Off",
                };
                match right_click {
                    Ok(on) if config.tui_right_click_paste == RightClickPaste::Auto => {
                        format!("Auto · {}", on_off(on))
                    }
                    Ok(_) => preference.to_owned(),
                    Err(_) => format!("{preference} · Unavailable"),
                }
            }),
        ),
    ];
    if owned {
        rows.push(row(
            Id::CollapseTurnDetails,
            Sessions,
            "Collapse turn details",
            "Summarize tool calls in the conversation",
            "collapse_turn_details fullscreen transcript expand history",
            Value::Toggle(config.tui_collapse_turn_details),
        ));
    }
    rows
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

    #[tokio::test]
    async fn config_rows_follow_the_buffer_at_startup_and_after_reload() -> anyhow::Result<()> {
        let home = tempfile::tempdir()?;
        let ssh = PasteEnvironment {
            platform_default: false,
            ssh: true,
            wsl: false,
            vscode: false,
        };
        let cases = [
            ("auto", false),
            ("auto", true),
            ("always", false),
            ("never", true),
        ];
        for (configured, alternate_screen) in cases {
            let config = codex_core::config::ConfigBuilder::default()
                .codex_home(home.path().to_owned())
                .fallback_cwd(Some(home.path().to_owned()))
                .cli_overrides(vec![
                    ("tui.fullscreen_transcript".into(), true.into()),
                    ("tui.alternate_screen".into(), configured.into()),
                    ("tui.right_click_paste".into(), "on".into()),
                ])
                .build()
                .await?;
            let rows = config_rows(&config, &ssh, alternate_screen);
            assert_eq!(
                rows.iter()
                    .any(|row| row.id == SettingId::CollapseTurnDetails),
                alternate_screen,
                "{configured}"
            );
            let paste = rows
                .iter()
                .find(|row| row.id == SettingId::RightClickPaste)
                .expect("right-click setting");
            let description = if alternate_screen {
                "Unavailable over SSH; use your terminal's paste command"
            } else {
                "Unavailable in inline mode; use your terminal's paste command"
            };
            assert_eq!(paste.description, description, "{configured}");
        }
        Ok(())
    }

    #[test]
    fn custom_themes_are_the_tmtheme_stems_in_order() {
        let dir = std::env::temp_dir().join(format!("rtwk-themes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["zen.tmTheme", "amber.tmTheme", "notes.txt"] {
            std::fs::File::create(dir.join(name)).unwrap();
        }
        assert_eq!(custom_themes(&dir), ["amber", "zen"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_themes_directory_offers_none() {
        assert!(custom_themes(Path::new("/nonexistent/rtwk-themes")).is_empty());
    }
}
