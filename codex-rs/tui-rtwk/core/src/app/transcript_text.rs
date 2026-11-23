//! What renders a thread's conversation as the Markdown of `/export`: the UI's (the cells' text
//! is the UI's), handed to the app once (`CodexApp::with_transcript_text`) as a pure function of
//! a thread's model. A feature reads the text through `AppView::transcript_text` and never holds
//! the renderer or the model (ADR-012).

use std::rc::Rc;

use crate::threads::ThreadModel;

/// What renders a thread's conversation as the Markdown of an export: `# Codex conversation`,
/// then a `## User` / `## Assistant` / `## Plan` / `## Reasoning` / `## Activity` section per
/// cell. `None` is a conversation with nothing in it to export.
#[derive(Clone)]
pub struct TranscriptText(Rc<Render>);

type Render = dyn Fn(&ThreadModel) -> Option<String>;

impl TranscriptText {
    pub fn new(render: impl Fn(&ThreadModel) -> Option<String> + 'static) -> Self {
        Self(Rc::new(render))
    }

    pub(crate) fn render(&self, model: &ThreadModel) -> Option<String> {
        (self.0)(model)
    }
}

impl std::fmt::Debug for TranscriptText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TranscriptText")
    }
}
