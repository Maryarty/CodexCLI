"""Carry the oracle's recap history excerpting (`codex-rs/tui/src/app/recap_history.rs`, from
`recap_history` on) into `codex-rs/tui-rtwk/core/src/features/recap/history.rs`: the cells it
reads become the thread's messages. Rerun after the oracle moves:
python3 -I tools/rtwk-port/recap-history/port.py"""
import pathlib

root = pathlib.Path(__file__).resolve().parents[3]
src = (root / "codex-rs/tui/src/app/recap_history.rs").read_text()
body = src[src.index("pub(super) fn recap_history"):]
body = body.replace(
    "pub(super) fn recap_history(cells: &[Arc<dyn HistoryCell>]) -> String {\n    let exchanges = recent_exchanges(cells);",
    "pub fn recap_history(messages: &[Message]) -> String {\n    let exchanges = recent_exchanges(messages);",
)
body = body.replace(
    "fn recent_exchanges(cells: &[Arc<dyn HistoryCell>]) -> Vec<Exchange> {",
    "fn recent_exchanges(messages: &[Message]) -> Vec<Exchange> {",
)
old_loop = body[body.index("    for cell in cells.iter().rev() {"):body.index("        // In reverse order")]
new_loop = """    for message in messages.iter().rev() {
        let (is_user, text) = match message {
            Message::User(text) => (true, text),
            Message::Assistant(text) => (false, text),
        };
        let mut content = text.trim().to_owned();
        if content.is_empty() {
            continue;
        }
"""
body = body.replace(old_loop, new_loop)
body = body.replace("RecapPrompt::HISTORY_MAX_BYTES", "HISTORY_MAX_BYTES")
header = '''//! What the recap sees of the conversation (oracle `app/recap_history.rs`, carried by
//! `tools/rtwk-port/recap-history/port.py`): recent visible exchanges without tools, reasoning or
//! another thread's history. A newer unanswered request is retained once; excerpts preserve both
//! ends within a byte limit. The oracle reads its history cells (`UserHistoryCell`,
//! `AgentMarkdownCell`, `AgentMessageCell`, text trimmed); here they are the thread's messages.

use super::prompt::HISTORY_MAX_BYTES;

/// One visible message of the conversation, as the history cells show its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    User(String),
    Assistant(String),
}

pub const RECAP_HISTORY_MAX_TURNS: usize = 8;
const OMITTED_HISTORY: &str = "[Earlier exchanges omitted]\\n\\n";
const EXCERPT_MARKER: &str = "\\n[... excerpted ...]\\n";

'''
(root / "codex-rs/tui-rtwk/core/src/features/recap/history.rs").write_text(header + body)
