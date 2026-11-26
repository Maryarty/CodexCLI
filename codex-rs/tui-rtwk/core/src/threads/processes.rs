//! Unified exec, the thread's side (oracle `chatwidget/command_lifecycle.rs`,
//! `chatwidget/exec_state.rs`): the commands that run in the background as terminals, the
//! last lines each printed, and the wait streak of empty polls. What is drawn from it is the
//! UI's: the footer line (`ThreadModel::terminals`), the `/ps` cell (`features::processes`), the
//! status row `Waiting for background terminal` (`ThreadModel::terminal_wait`) and the
//! interaction cells ([`Notice::TerminalInteraction`], detailed transcript only).
//!
//! The processes are kept over an interrupt and a completed turn: only `/stop`
//! (`ThreadModel::clean_terminals`), the end of their own command and a restart of the thread
//! take them away. The wait streak is the one thing a turn's end settles: a completed turn writes
//! it as one `Waited for background terminal` entry, an interrupted or failed one drops it
//! (oracle `on_task_complete` flushes, `finalize_turn` clears).

use super::{
    Item, ItemKey, Notice, ThreadModel, ToolKind, ToolStatus, TurnEnd, TurnId, apply_appended,
};
use crate::screens::approvals::strip_shell_wrapper;

/// How many of a process's last output lines the `/ps` cell lists (oracle `MAX_RECENT_CHUNKS`).
pub const RECENT_CHUNKS: usize = 3;

/// One unified-exec session that runs in the background (oracle `UnifiedExecProcessSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// The server's process id, else the item's id.
    pub key: String,
    /// The id of the item that started it: its output deltas name it.
    pub call_id: String,
    /// The command as the user reads it, the shell wrapper stripped.
    pub command: String,
    /// The last non-empty lines of its output, the newest last (at most [`RECENT_CHUNKS`]).
    pub recent: Vec<String>,
}

/// A terminal the server lists (`thread/backgroundTerminals/list`), as it listed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub item: String,
    pub process: String,
    pub command: String,
}

/// The empty polls of one process that follow each other (oracle `UnifiedExecWaitStreak`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitStreak {
    pub process: String,
    /// The command of the process, once it is known and not empty.
    pub command: Option<String>,
}

/// What the thread knows of its background processes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Processes {
    terminals: Vec<Terminal>,
    streak: Option<WaitStreak>,
    /// A restore asked of the server (oracle `pending_terminal_restore`): its id and the items
    /// whose command ended while the list was in flight.
    restoring: Option<(u32, Vec<String>)>,
    restore_seq: u32,
}

impl Processes {
    pub(super) fn terminals(&self) -> &[Terminal] {
        &self.terminals
    }

    pub(super) fn streak(&self) -> Option<&WaitStreak> {
        self.streak.as_ref()
    }

    /// `/stop`: the processes are gone (oracle `clear_background_terminals`).
    pub(super) fn clear(&mut self) {
        self.restoring = None;
        self.terminals.clear();
    }

    /// A resumed thread asks for its live terminals (oracle `begin_background_terminal_restore`):
    /// a second ask makes the answer of the first stale.
    pub(super) fn begin_restore(&mut self) -> u32 {
        self.restore_seq += 1;
        self.restoring = Some((self.restore_seq, Vec::new()));
        self.restore_seq
    }

    /// The server's list arrived (oracle `restore_background_terminals`): a terminal whose item
    /// ended meanwhile or whose process is held already is not added; the command stays as
    /// listed. An answer of a restore that is not the pending one is dropped.
    pub(super) fn restore(&mut self, id: u32, listed: Vec<Listed>) {
        let Some((_, ended)) = self.restoring.take_if(|(pending, _)| *pending == id) else {
            return;
        };
        for terminal in listed {
            if ended.contains(&terminal.item)
                || self.terminals.iter().any(|t| t.key == terminal.process)
            {
                continue;
            }
            self.terminals.push(Terminal {
                key: terminal.process,
                call_id: terminal.item,
                command: terminal.command,
                recent: Vec::new(),
            });
        }
    }

    /// The command as the user reads it (oracle `strip_bash_lc_and_escape`).
    fn display(command: &str) -> String {
        strip_shell_wrapper(command)
    }

    /// A unified-exec command began (oracle `track_unified_exec_process_begin`): a process of its
    /// key, a restarted one with a clean output.
    fn begin(&mut self, call_id: &str, process: Option<&str>, command: &str) {
        let key = process.unwrap_or(call_id).to_owned();
        let command = Self::display(command);
        match self.terminals.iter_mut().find(|t| t.key == key) {
            Some(terminal) => {
                terminal.call_id = call_id.to_owned();
                terminal.command = command;
                terminal.recent.clear();
            }
            None => self.terminals.push(Terminal {
                key,
                call_id: call_id.to_owned(),
                command,
                recent: Vec::new(),
            }),
        }
    }

    /// Its command ended (oracle `track_unified_exec_process_end`).
    fn end(&mut self, call_id: &str, process: Option<&str>) {
        if let Some((_, ended)) = &mut self.restoring {
            ended.push(call_id.to_owned());
        }
        let key = process.unwrap_or(call_id);
        self.terminals.retain(|terminal| terminal.key != key);
    }

    /// Output of the command `call_id` (oracle `track_unified_exec_output_chunk`).
    fn output(&mut self, call_id: &str, chunk: &str) {
        let Some(terminal) = self.terminals.iter_mut().find(|t| t.call_id == call_id) else {
            return;
        };
        terminal.recent.extend(
            chunk
                .lines()
                .map(str::trim_end)
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
        let surplus = terminal.recent.len().saturating_sub(RECENT_CHUNKS);
        terminal.recent.drain(..surplus);
    }

    fn command_of(&self, process: &str) -> Option<String> {
        self.terminals
            .iter()
            .find(|terminal| terminal.key == process)
            .map(|terminal| terminal.command.clone())
    }
}

/// A unified-exec command that runs is a background terminal until its item completes (oracle
/// `track_unified_exec_process_begin` / `_end`); the end of the command of the wait streak's
/// process settles the streak first (oracle `on_command_execution_completed`).
pub(super) fn track(model: &mut ThreadModel, key: &ItemKey, item: &Item) {
    let Item::Tool(tool) = item else {
        return;
    };
    let ToolKind::Shell {
        command,
        process_id,
        unified_exec: true,
        ..
    } = &tool.kind
    else {
        return;
    };
    let call = key.item.as_str();
    let process = process_id.as_deref();
    if tool.status == ToolStatus::Running {
        model.processes.begin(call, process, command);
        return;
    }
    if let Some(process) = process
        && model
            .processes
            .streak
            .as_ref()
            .is_some_and(|streak| streak.process == process)
    {
        flush(model, Some(key.turn.clone()));
    }
    model.processes.end(call, process);
}

/// A unified-exec startup command that is not a standard tool call draws no entry of its own
/// (oracle `on_command_execution_started`: it returns before any cell unless the command parses
/// to known actions, the footer and the wait row say it runs), and its end after its turn
/// finished draws none either (`on_command_execution_completed`: `if !is_task_running`). While
/// the turn runs the end draws the `Ran` cell as any command's. The process facts are kept
/// ([`track`] runs first), only the transcript entry is skipped.
pub(super) fn draws_nothing(item: &Item, turn_finished: bool) -> bool {
    let Item::Tool(tool) = item else {
        return false;
    };
    let ToolKind::Shell {
        actions,
        unified_exec: true,
        ..
    } = &tool.kind
    else {
        return false;
    };
    if tool.status == ToolStatus::Running {
        return actions.is_empty();
    }
    turn_finished
}

/// Output of a command: the last lines of a background terminal's (oracle
/// `on_exec_command_output_delta`).
pub(super) fn output(model: &mut ThreadModel, key: &ItemKey, delta: &str) {
    model.processes.output(key.item.as_str(), delta);
}

/// `item/commandExecution/terminalInteraction` (oracle `on_terminal_interaction`): the model
/// polled the process (empty `stdin`) or wrote to it. A turn must run; a poll of a process not
/// known is nothing.
pub(super) fn interaction(model: &mut ThreadModel, process: &str, stdin: String) {
    if !model.is_running() {
        return;
    }
    let command = model.processes.command_of(process);
    if stdin.is_empty() && command.is_none() {
        return;
    }
    if stdin.is_empty() {
        let command = command.filter(|command| !command.is_empty());
        match &mut model.processes.streak {
            Some(streak) if streak.process == process => {
                if streak.command.is_none() {
                    streak.command = command;
                }
            }
            _ => {
                flush(model, None);
                model.processes.streak = Some(WaitStreak {
                    process: process.to_owned(),
                    command,
                });
            }
        }
        return;
    }
    if model
        .processes
        .streak
        .as_ref()
        .is_some_and(|streak| streak.process == process)
    {
        flush(model, None);
    }
    apply_appended(
        model,
        None,
        Item::Notice(Notice::TerminalInteraction { command, stdin }),
    );
}

/// The streak is over: it is one `Waited for background terminal` entry of the transcript
/// (oracle `flush_unified_exec_wait_streak`), in `turn`, else the running one.
pub(super) fn flush(model: &mut ThreadModel, turn: Option<TurnId>) {
    let Some(streak) = model.processes.streak.take() else {
        return;
    };
    apply_appended(
        model,
        turn,
        Item::Notice(Notice::TerminalInteraction {
            command: streak.command,
            stdin: String::new(),
        }),
    );
}

/// A turn ended: a completed one writes the streak, any other drops it. The processes stay.
pub(super) fn turn_ended(model: &mut ThreadModel, turn: &TurnId, end: TurnEnd) {
    match end {
        TurnEnd::Completed => flush(model, Some(turn.clone())),
        TurnEnd::Interrupted | TurnEnd::Failed => model.processes.streak = None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn listed(item: &str, process: &str, command: &str) -> Listed {
        Listed {
            item: item.to_owned(),
            process: process.to_owned(),
            command: command.to_owned(),
        }
    }

    fn commands(processes: &Processes) -> Vec<String> {
        processes
            .terminals()
            .iter()
            .map(|terminal| terminal.command.clone())
            .collect()
    }

    #[test]
    fn restore_keeps_a_listed_terminal() {
        let mut processes = Processes::default();
        let id = processes.begin_restore();
        processes.restore(id, vec![listed("c1", "43", "sleep 200")]);
        assert_eq!(commands(&processes), ["sleep 200"]);
        assert_eq!(processes.terminals()[0].key, "43");
        assert_eq!(processes.terminals()[0].call_id, "c1");
    }

    #[test]
    fn restore_skips_the_terminal_that_ended_while_pending() {
        let mut processes = Processes::default();
        let id = processes.begin_restore();
        processes.end("c2", Some("44"));
        processes.restore(
            id,
            vec![
                listed("c1", "43", "sleep 200"),
                listed("c2", "44", "sleep 300"),
            ],
        );
        assert_eq!(commands(&processes), ["sleep 200"]);
    }

    #[test]
    fn restore_skips_a_process_already_held() {
        let mut processes = Processes::default();
        processes.begin("live", Some("43"), "sleep 200");
        let id = processes.begin_restore();
        processes.restore(id, vec![listed("c1", "43", "sleep 200 again")]);
        assert_eq!(commands(&processes), ["sleep 200"]);
    }

    #[test]
    fn a_second_restore_makes_the_first_answer_stale() {
        let mut processes = Processes::default();
        let first = processes.begin_restore();
        let second = processes.begin_restore();
        processes.restore(first, vec![listed("c1", "43", "sleep 200")]);
        assert_eq!(commands(&processes), Vec::<String>::new());
        processes.restore(second, vec![listed("c1", "43", "sleep 200")]);
        assert_eq!(commands(&processes), ["sleep 200"]);
    }

    #[test]
    fn clean_forgets_a_pending_restore() {
        let mut processes = Processes::default();
        let id = processes.begin_restore();
        processes.clear();
        processes.restore(id, vec![listed("c1", "43", "sleep 200")]);
        assert_eq!(commands(&processes), Vec::<String>::new());
    }

    #[test]
    fn restore_keeps_the_listed_command_unstripped() {
        let mut processes = Processes::default();
        let id = processes.begin_restore();
        processes.restore(id, vec![listed("c1", "43", "bash -lc 'sleep 200'")]);
        assert_eq!(commands(&processes), ["bash -lc 'sleep 200'"]);
    }
}
