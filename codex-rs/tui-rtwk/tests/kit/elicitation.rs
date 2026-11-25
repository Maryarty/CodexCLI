//! What an oracle test of the app-link and approval dialogs expects of the answers the app gave
//! the server: the elicitation actions, the directory reads, the switch written before the
//! accept. A shard names no command and writes no `assert!`; each is spelled once here.

use codex_tui_core::app::Command;
use codex_tui_core::screens::{ScreenCommand, select};

use super::Screen;

impl Screen {
    /// The app answered no server request yet.
    pub fn assert_no_resolved_requests(&self) {
        let got = self.resolved_requests();
        assert!(got.is_empty(), "answers sent: {got:?}\n{}", self.text());
    }

    /// The server requests were answered with exactly the elicitation `actions`, oldest first.
    pub fn assert_resolved_actions(&self, actions: &[&str]) {
        let got: Vec<String> = self
            .resolved_requests()
            .iter()
            .map(|result| result["action"].as_str().unwrap_or_default().to_owned())
            .collect();
        pretty_assertions::assert_eq!(got, actions, "{}", self.text());
    }

    /// The app asked for the directory of apps `n` times.
    pub fn assert_directory_reads(&self, n: usize) {
        let got = self
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    Command::Screen {
                        command: ScreenCommand::Select(select::Command::Apps { .. }),
                        ..
                    }
                )
            })
            .count();
        pretty_assertions::assert_eq!(got, n, "directory reads");
    }

    /// The app switched the app `id` on, and only then answered the request.
    pub fn assert_app_enabled_then_resolved(&self, id: &str) {
        let asked = self.commands();
        let written = asked.iter().position(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::SetApp { id: got, enabled: true }),
                    ..
                } if got == id
            )
        });
        let accepted = asked
            .iter()
            .position(|command| matches!(command, Command::ResolveRequest { .. }));
        assert!(written.is_some() && written < accepted, "{asked:?}");
    }

    /// The app gave the backend `n` commands that start with `name`.
    pub fn assert_commands_named(&self, name: &str, n: usize) {
        pretty_assertions::assert_eq!(self.commands_named(name).len(), n, "{name} commands");
    }
}
