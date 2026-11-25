//! The Markdown of `/export` (oracle `transcript_export.rs`): `# Codex conversation`, then a
//! `## User` / `## Assistant` / `## Plan` / `## Reasoning` section per cell with text. Handed to
//! the app once (`CodexApp::with_transcript_text`); a conversation with none of them has
//! nothing to export.

use codex_tui_core::app::TranscriptText;
use codex_tui_core::threads::{Item, ThreadModel};

/// The renderer the app is built with.
pub fn renderer() -> TranscriptText {
    TranscriptText::new(markdown)
}

fn markdown(model: &ThreadModel) -> Option<String> {
    let transcript = model.transcript();
    let mut sections = Vec::new();
    for index in 0..transcript.len() {
        let Some((_, item)) = transcript.get(index) else {
            continue;
        };
        let (title, text) = match item {
            Item::User(prompt) => ("User", prompt.text.as_str()),
            Item::Agent { text, .. } => ("Assistant", text.as_str()),
            Item::Plan { text } => ("Plan", text.as_str()),
            Item::Reasoning { summary, .. } => ("Reasoning", summary.as_str()),
            _ => continue,
        };
        if !text.trim().is_empty() {
            sections.push(["## ", title, "\n\n", text.trim_end()].concat());
        }
    }
    (!sections.is_empty())
        .then(|| ["# Codex conversation\n\n", &sections.join("\n\n"), "\n"].concat())
}
