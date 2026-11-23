//! The text prompt of the small dialogs (oracle `bottom_pane/custom_prompt_view.rs`): what a
//! prompt asks for, what it was prefilled with and, for `/rename`, the one generated suggestion
//! that fills the field only while the user has not typed. The text field itself and what is
//! typed in it are the view's (a `TextEditingController`); the model keeps what outlives a
//! keystroke: which prompt this is, whether the user edited, the suggestion's state. The words
//! (`Rename thread`, `Name thread`, the placeholder, the two labels) are the view's.

/// What a prompt asks a text for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptKind {
    /// A prompt of a feature slice (ADR-012): the feature owns what its submission does.
    Feature(crate::features::FeaturePrompt),
}

/// The generated suggestion of a prompt (`with_text_suggestion`, `apply_text_suggestion`):
/// the one a feature asks for (`/rename`'s title) fills the field only while the user has not
/// typed.
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
}

impl Prompt {
    /// A feature's prompt, its field starting with `initial`; `suggesting`: a generated
    /// suggestion is awaited (`Feature` asks the host and answers with `suggested_for`).
    pub(crate) fn feature(
        prompt: crate::features::FeaturePrompt,
        initial: String,
        suggesting: bool,
    ) -> Self {
        Prompt {
            kind: PromptKind::Feature(prompt),
            initial,
            suggestion: if suggesting {
                Suggestion::Loading
            } else {
                Suggestion::Idle
            },
            edited: false,
        }
    }

    /// The field changed by a key or a paste: a suggestion no longer fills it.
    pub(crate) fn typed(&mut self) {
        self.edited = true;
    }

    /// The generator's answer for the feature prompt `prompt`: it fills the field only while
    /// this is still that prompt, it still waits, and the user has not typed
    /// (`apply_text_suggestion`).
    pub(crate) fn suggested_for(
        &mut self,
        prompt: &crate::features::FeaturePrompt,
        title: Option<String>,
    ) {
        let ours = matches!(&self.kind, PromptKind::Feature(open) if open == prompt);
        if ours && self.suggestion == Suggestion::Loading {
            self.suggestion = match title {
                Some(title) if !self.edited => Suggestion::Applied(title),
                _ => Suggestion::Idle,
            };
        }
    }
}

/// A prompt the user submitted, for the app to carry out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submitted {
    pub kind: PromptKind,
    pub text: String,
}
