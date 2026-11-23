//! The text prompt of the small dialogs (oracle `bottom_pane/custom_prompt_view.rs`): what a
//! prompt asks for, what it was prefilled with and, for `/rename`, the one generated suggestion
//! that fills the field only while the user has not typed. The text field itself and what is
//! typed in it are the view's (a `TextEditingController`); the model keeps what outlives a
//! keystroke: which prompt this is, whether the user edited, the suggestion's state. The words
//! (`Rename thread`, `Name thread`, the placeholder, the two labels) are the view's.

use crate::ids::ThreadId;

/// The effect requests of the prompt carry this bit: the UI's own request numbers never do.
pub const REQUEST_BIT: u64 = 1 << 60;

/// What a prompt asks a text for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptKind {
    /// `/rename`: a name for `thread`.
    Rename { thread: ThreadId },
    /// `/export`, "Save to file": the filename the transcript is written to (oracle
    /// `show_transcript_export_file_prompt`). The conversation is the one in use when it is
    /// submitted.
    ExportFile,
}

/// The generated suggestion of a prompt (`with_text_suggestion`, `apply_text_suggestion`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Suggestion {
    /// None was asked for, or its answer did not apply (nothing came out, or the user typed
    /// first): no label.
    #[default]
    Idle,
    /// `Generating a title suggestion…`: the answer is awaited.
    Loading,
    /// The text to put in the field (`Suggested from this conversation`).
    Applied(String),
}

/// An open text prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    /// The text the field starts with: the thread's name, empty when it has none (the title is
    /// `Rename thread` with a name, else `Name thread`).
    pub initial: String,
    pub suggestion: Suggestion,
    /// The user typed or pasted since the prompt opened.
    pub edited: bool,
    /// The request the suggestion's answer must carry.
    request: Option<u64>,
}

impl Prompt {
    /// The rename prompt of `thread`; `existing` is its name, `request` the ask of a
    /// suggestion (none without a thread to read the conversation from).
    pub(crate) fn rename(thread: ThreadId, existing: Option<&str>, request: Option<u64>) -> Self {
        Prompt {
            kind: PromptKind::Rename { thread },
            initial: existing.filter(|name| !name.is_empty()).unwrap_or_default().to_owned(),
            suggestion: if request.is_some() {
                Suggestion::Loading
            } else {
                Suggestion::Idle
            },
            edited: false,
            request,
        }
    }

    /// The export prompt, prefilled with the filename for the conversation in use
    /// (`codex-session-{thread}.md`, `codex-session.md` before a thread exists).
    pub(crate) fn export_file(thread: Option<&ThreadId>) -> Self {
        Prompt {
            kind: PromptKind::ExportFile,
            // A filename, not prose: the oracle's own (`codex-session-{thread}.md`).
            initial: thread.map_or_else(
                || "codex-session.md".to_owned(),
                |thread| ["codex-session-", thread.as_str(), ".md"].concat(),
            ),
            suggestion: Suggestion::Idle,
            edited: false,
            request: None,
        }
    }

    /// The thread already has a name (the title says `Rename`).
    pub fn renaming(&self) -> bool {
        !self.initial.is_empty()
    }

    /// The field changed by a key or a paste: a suggestion no longer fills it.
    pub(crate) fn typed(&mut self) {
        self.edited = true;
    }

    /// Whether the answer `request` is the suggestion this prompt waits for.
    pub fn waits_for(&self, request: u64) -> bool {
        self.request == Some(request) && self.suggestion == Suggestion::Loading
    }

    /// The generator's answer: it fills the field only while the user has not typed
    /// (`apply_text_suggestion`).
    pub(crate) fn suggested(&mut self, title: Option<String>) {
        self.request = None;
        self.suggestion = match title {
            Some(title) if !self.edited => Suggestion::Applied(title),
            _ => Suggestion::Idle,
        };
    }
}

/// A prompt the user submitted, for the app to carry out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submitted {
    pub kind: PromptKind,
    pub text: String,
}

/// `normalize_thread_name`: the trimmed name, none when nothing is left.
pub fn normalize_thread_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
