//! Which dialog a slash command opens (`Args`).

use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::settings::Which;

/// What opens a settings dialog: its kind, as the slash command names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    pub which: Which,
    /// `/keymap debug`: the keymap editor opens on the keypress inspector.
    pub inspect: bool,
}

impl Args {
    /// The dialog `command` opens (`/theme`, `/memories`, `/statusline`, `/title`, `/skills`,
    /// `/config`, `/experimental`, `/keymap`; `/tui` is the configuration panel).
    pub fn of(command: SlashCommand) -> Args {
        Args {
            inspect: false,
            which: match command {
                SlashCommand::Theme => Which::Theme,
                SlashCommand::Memories => Which::Memories,
                SlashCommand::Statusline => Which::StatusLine,
                SlashCommand::Title => Which::Title,
                SlashCommand::Skills => Which::Skills,
                SlashCommand::Experimental => Which::Experimental,
                SlashCommand::Keymap => Which::Keymap,
                _ => Which::Config,
            },
        }
    }

    /// The dialog opens on the keypress inspector (`/keymap debug`).
    pub fn inspecting(mut self) -> Args {
        self.inspect = self.which == Which::Keymap;
        self
    }
}
