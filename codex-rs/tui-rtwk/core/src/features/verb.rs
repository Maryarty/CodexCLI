//! The public verbs (ADR-012 "public verbs", arch-pillars P9): what a feature may ask of the app
//! that another owner carries out. A short whitelist and a verb, never a gesture: the app
//! (`app/verbs.rs`) routes each to the one that owns it, so a slice does not hand a message to the
//! UI for the UI to hand on (the old `Wish::{Status, Model}` relay). A wish is for showing
//! something once; a verb is for changing something.

use crate::ids::ThreadId;
use crate::rpc::Address;
use crate::screens::chat::{Nudge, StatusOverride};
use crate::screens::select::models::{Effort, Selection};
use crate::screens::select::permissions;

/// What a feature asks of an owner outside it (`AppCx::verb`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicVerb {
    /// `/title` and `/statusline` with items: the status surfaces this session shows (the chat
    /// screen's; nothing is written to the config).
    StatusOverride(StatusOverride),
    /// `/model <name>`: the model and effort picked, for this session or saved (the app's model
    /// pick, as the sheet's own pick is carried out). A `--speed` is checked by the arguments and
    /// not carried (the pick has no tier).
    PickModel(Selection),
    /// A `/permissions` pick applied once its precondition is met (the Windows sandbox is set
    /// up): the app's permissions pick, as the sheet's own pick is carried out.
    PickPermissions(permissions::Selection),
    /// The model and effort the thread's next turns use, saved nowhere and said nowhere (oracle
    /// `override_turn_context` with a model and an effort, then `UpdateModel` and
    /// `UpdateReasoningEffort`: the rate limit prompt's "Switch to {model}"). `PickModel` for
    /// this session only says its line; this one is for a feature whose own prompt is the
    /// answer.
    OverrideTurn {
        thread: ThreadId,
        address: Address,
        model: String,
        effort: Option<Effort>,
    },
    /// The workspace owner is e-mailed (`account/sendAddCreditsNudgeEmail`): the app's one nudge
    /// path, the banner's notify-owner action's too, with its one request in flight and its note
    /// in the conversation (`finish_add_credits_nudge_email_request`).
    NotifyOwner(Nudge),
}
