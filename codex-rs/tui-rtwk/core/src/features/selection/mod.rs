//! Copying what the reader selected in the transcript as a feature slice (ADR-012; oracle
//! `transcript_view/selection.rs` `copy_selected_text_with`, `finish_copy`,
//! `composer_gap.rs` `show_copy_feedback`): the view hands the text of a selection to the app,
//! the app asks the host's clipboard service for it, and the answer is the feedback the
//! composer gap shows (`Copied 8 chars to host clipboard`, `Copying…`, `Copy failed · try
//! again`).
//!
//! The selection itself (its range, its highlight, its words and lines) is the view's: rtwk's
//! `SelectionArea` owns it. What the app keeps is the one thing that outlives the selection,
//! the result of the last copy, which belongs to the composer gap and survives a change or the
//! end of the selection (oracle: "Feedback also belongs to composer copies and survives
//! selection changes"). Its five seconds of life are the gap's (a timer of the view).

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;

/// The marker type of the feature.
pub struct Selection;

/// What the view asks of the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Put `text`, what the reader selected, where `destination` says (oracle `Tui::copy_selection`
    /// with a `SelectionCopy`).
    Copy {
        text: String,
        format: CopyFormat,
        destination: CopyDestination,
    },
}

/// What a copy was for: how many characters it carried (the oracle's feedback counts what
/// was copied, not what is selected).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    pub characters: usize,
}

/// What the last copy came to, until the next one: the composer gap words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feedback {
    /// How many answers have been taken in: a view tells a new answer from the one it has
    /// shown (a pending copy that ends is a new answer, with its own five seconds).
    pub version: u64,
    /// How the copy was delivered; the host's failure text is the view's to word, so only
    /// that it failed is kept (oracle `Result<CopyStatus, ()>`).
    pub result: Result<CopyStatus, ()>,
    pub characters: usize,
}

/// The app-scoped state: the feedback of the last copy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub feedback: Option<Feedback>,
    answers: u64,
}

impl Feature for Selection {
    const NAME: &'static str = "selection";

    type State = State;
    type ThreadPart = ();
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Menu = Never<Self>;
    type Pick = Never<Self>;
    type Prompt = Never<Self>;
    type Note = Never<Self>;
    type Fact = Never<Self>;

    fn intent(_state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Copy {
            text,
            format,
            destination,
        } = intent;
        let characters = text.chars().count();
        cx.effect(
            Ask::Copy {
                text,
                format,
                destination,
            },
            Ctx { characters },
        );
    }

    fn effect_done(state: &mut State, ctx: Ctx, done: Done, _cx: &mut AppCx<'_>) {
        let Done::Copy(result) = done else {
            return;
        };
        let result = result.map_err(|_| ());
        // A second copy while one is under way is refused by the host as busy; the feedback
        // of the one under way stays (oracle `show_copy_feedback`).
        if result == Ok(CopyStatus::Busy)
            && state
                .feedback
                .as_ref()
                .is_some_and(|feedback| matches!(feedback.result, Ok(CopyStatus::Pending(_))))
        {
            return;
        }
        state.answers += 1;
        state.feedback = Some(Feedback {
            version: state.answers,
            result,
            characters: ctx.characters,
        });
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
