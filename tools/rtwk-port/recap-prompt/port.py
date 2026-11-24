"""Carry the instructions of the oracle's recap prompt (`codex-rs/context-fragments/src/
recap_prompt.rs`, `PROMPT_PREFIX`) into `codex-rs/tui-rtwk/core/src/features/recap/prompt.rs`
(core has no dependency on that crate). Rerun after the oracle moves:
python3 -I tools/rtwk-port/recap-prompt/port.py"""
import pathlib

root = pathlib.Path(__file__).resolve().parents[3]
src = (root / "codex-rs/context-fragments/src/recap_prompt.rs").read_text()
marker = 'const PROMPT_PREFIX: &str = r#"'
start = src.index(marker) + len(marker)
end = src.index('"#;', start)
prefix = src[start:end]
assert '"#' not in prefix
out = f'''//! The recap request's prompt (oracle `codex-context-fragments` `RecapPrompt`): the instructions
//! followed by the conversation excerpt, within the prompt's byte budget. The instructions are
//! carried by `tools/rtwk-port/recap-prompt/port.py`.

/// The prompt's budget in estimated tokens (`RecapPrompt::MAX_ESTIMATED_TOKENS`).
const MAX_ESTIMATED_TOKENS: usize = 8_192;
/// The shared four-bytes-per-token estimate (`approx_bytes_for_tokens`).
const APPROX_BYTES_PER_TOKEN: usize = 4;
/// Total UTF-8 bytes, including instructions and conversation labels (`RecapPrompt::MAX_BYTES`).
const MAX_BYTES: usize = MAX_ESTIMATED_TOKENS * APPROX_BYTES_PER_TOKEN;

const PROMPT_PREFIX: &str = r#"{prefix}"#;

/// Space available after the fixed instructions (`RecapPrompt::HISTORY_MAX_BYTES`).
pub const HISTORY_MAX_BYTES: usize = MAX_BYTES - PROMPT_PREFIX.len();

/// The prompt for `history`, which is cut at the budget (`RecapPrompt::new`, `body`).
pub fn prompt(history: &str) -> String {{
    let end = history.floor_char_boundary(HISTORY_MAX_BYTES.min(history.len()));
    format!("{{PROMPT_PREFIX}}{{}}", &history[..end])
}}
'''
(root / "codex-rs/tui-rtwk/core/src/features/recap/prompt.rs").write_text(out)
