//! The words of `/ps` (oracle `history_cell/exec.rs` `UnifiedExecProcessesCell`). This is the
//! text of the cell at an unbounded width, as one note; the cell that cuts a command to its
//! width and to 80 graphemes, and lists 16 processes at most, is the page's (UI LEFT).

use codex_tui_core::features::processes::Note;

/// The most processes the cell lists (oracle `max_processes`).
const MAX_PROCESSES: usize = 16;

/// The words of the note: the heading and one row per terminal with its last output lines.
pub fn note(note: &Note) -> String {
    let mut lines = vec!["Background terminals".to_owned(), String::new()];
    if note.terminals.is_empty() {
        lines.push("  • No background terminals running.".to_owned());
        return lines.join("\n");
    }
    for terminal in note.terminals.iter().take(MAX_PROCESSES) {
        lines.push(format!("  • {}", terminal.command.replace('\n', "↵")));
        for (index, chunk) in terminal.recent.iter().enumerate() {
            let prefix = if index == 0 { "    ↳ " } else { "      " };
            lines.push(format!("{prefix}{chunk}"));
        }
    }
    let remaining = note.terminals.len().saturating_sub(MAX_PROCESSES);
    if remaining > 0 {
        lines.push(format!("  • ... and {remaining} more running"));
    }
    lines.join("\n")
}
