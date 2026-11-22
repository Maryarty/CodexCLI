//! The restricted composer (`chat_composer/reconnect.rs`
//! `RestrictedInputMode`): the draft stays editable while the thread cannot
//! take a turn, and Enter runs only the local commands that need none.
//! Pure: which commands pass in which mode. The keys and the draft's
//! expansion are the view's.

use crate::commands::{SlashCommand, parse_slash_name};

/// Why the composer is restricted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restricted {
    /// The connection to the server is lost (reconnecting, or given up).
    Disconnected,
    /// The thread is not attached to a live stream: recovery commands pass.
    UnavailableThread,
    /// A cloud switch owns navigation; the draft is handed over to it and
    /// Enter cannot submit.
    DraftHandoff,
}

impl Restricted {
    /// Whether Enter on `text` (pastes expanded) runs the command it is:
    /// a single line `/name` or `/name args`, the args only for a command
    /// that takes them inline, and the command one that passes here. A
    /// disconnected draft runs `/warnings`, `/config`, `/quit` and `/exit`;
    /// an unavailable thread also the commands that need no writable thread;
    /// a handoff none.
    pub fn dispatches(self, text: &str) -> bool {
        if self == Restricted::DraftHandoff {
            return false;
        }
        let text = text.trim();
        if text.lines().count() != 1 {
            return false;
        }
        let Some((name, rest, _)) = parse_slash_name(text) else {
            return false;
        };
        let Some(command) = SlashCommand::parse(name) else {
            return false;
        };
        if !rest.is_empty() && !command.supports_inline_args() {
            return false;
        }
        use SlashCommand::*;
        matches!(command, Warnings | Config | Quit | Exit)
            || self == Restricted::UnavailableThread && command.available_when_thread_unavailable()
    }

    /// Whether a paste is expanded where it lands: an offline draft shows
    /// what Enter would run at once (the app sends a Null key after the
    /// paste); the other modes expand on the next key.
    pub fn expands_on_paste(self) -> bool {
        self == Restricted::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unavailable_thread_runs_the_recovery_and_local_commands() {
        use SlashCommand::*;
        for command in [
            New, Clear, Resume, Agents, MultiAgents, Quit, Exit, Status, DebugConfig, Pwd,
            Rollout, Copy, Raw,
        ] {
            let line = ["/", command.command()].concat();
            assert!(Restricted::UnavailableThread.dispatches(&line), "{line}");
        }
    }

    #[test]
    fn inline_args_pass_for_commands_that_take_them() {
        let unavailable = Restricted::UnavailableThread;
        for line in ["/new recovery", "/clear recovery", "/resume saved", "/raw on"] {
            assert!(unavailable.dispatches(line), "{line}");
        }
        for line in ["/status extra", "/config now", "/quit now"] {
            assert!(!unavailable.dispatches(line), "{line}");
        }
    }

    #[test]
    fn a_disconnected_draft_runs_only_the_four_local_commands() {
        for line in ["/warnings", "/config", "/quit", "/exit"] {
            assert!(Restricted::Disconnected.dispatches(line), "{line}");
        }
        for line in ["/status", "/new", "/clear", "/raw on", "/review", "/compact"] {
            assert!(!Restricted::Disconnected.dispatches(line), "{line}");
        }
    }

    #[test]
    fn prompts_and_several_lines_stay_in_the_draft() {
        for mode in [Restricted::Disconnected, Restricted::UnavailableThread] {
            for line in [
                "keep my draft",
                "!echo hello",
                "/rename changed",
                "/unknown",
                "/status\nkeep my draft",
                "/clear recovery\nkeep my draft",
                "/new recovery\nkeep my draft",
                "/resume saved\nkeep my draft",
                "",
            ] {
                assert!(!mode.dispatches(line), "{mode:?} {line:?}");
            }
        }
    }

    #[test]
    fn a_handoff_runs_nothing() {
        for line in ["/quit", "/config", "/status", "hello"] {
            assert!(!Restricted::DraftHandoff.dispatches(line), "{line}");
        }
    }
}
