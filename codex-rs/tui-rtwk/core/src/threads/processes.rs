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
        self.terminals.clear();
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
