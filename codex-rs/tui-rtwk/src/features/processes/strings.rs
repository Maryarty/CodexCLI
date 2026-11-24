//! The words of `/ps` (oracle `history_cell/exec.rs` `new_unified_exec_processes_output`,
//! `UnifiedExecProcessesCell`). The text of the cell at an unbounded width, as one note (the raw
//! mode's and a selection's text), and the pieces `cell.rs` lays out at the width it is given.

use codex_tui_core::features::processes::Note;

/// The typed command the cell echoes first (the oracle's magenta `/ps` plain cell).
pub const COMMAND: &str = "/ps";
/// The heading of the list.
pub const HEADING: &str = "Background terminals";
/// The row of an empty list.
pub const NONE: &str = "  • No background terminals running.";
/// The mark of a process row and of the `... and N more running` row.
pub const PROCESS_MARK: &str = "  • ";
/// The mark of the first output chunk of a process and of the ones after it.
pub const CHUNK_FIRST: &str = "    ↳ ";
pub const CHUNK_NEXT: &str = "      ";
/// What stands for a command or a chunk the width cuts.
pub const CUT: &str = " [...]";
/// A newline of a command (the cell is one row per process).
pub const NEWLINE: &str = "↵";

/// The most processes the cell lists (oracle `max_processes`).
pub const MAX_PROCESSES: usize = 16;
/// The most graphemes of a command the cell shows (oracle `max_graphemes`).
pub const MAX_GRAPHEMES: usize = 80;

/// The row that says how many processes the list leaves out.
pub fn more(remaining: usize) -> String {
    format!("... and {remaining} more running")
}

/// The words of the note: the echoed command, the heading and one row per terminal with its last
/// output lines.
pub fn note(note: &Note) -> String {
    let mut lines = vec![
        COMMAND.to_owned(),
        String::new(),
        HEADING.to_owned(),
        String::new(),
    ];
    if note.terminals.is_empty() {
        lines.push(NONE.to_owned());
        return lines.join("\n");
    }
    for terminal in note.terminals.iter().take(MAX_PROCESSES) {
        lines.push(format!(
            "{PROCESS_MARK}{}",
            terminal.command.replace('\n', NEWLINE)
        ));
        for (index, chunk) in terminal.recent.iter().enumerate() {
            let prefix = if index == 0 { CHUNK_FIRST } else { CHUNK_NEXT };
            lines.push(format!("{prefix}{chunk}"));
        }
    }
    let remaining = note.terminals.len().saturating_sub(MAX_PROCESSES);
    if remaining > 0 {
        lines.push(format!("{PROCESS_MARK}{}", more(remaining)));
    }
    lines.join("\n")
}
