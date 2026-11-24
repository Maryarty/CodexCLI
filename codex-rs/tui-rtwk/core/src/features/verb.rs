//! The public verbs (ADR-012 "public verbs", arch-pillars P9): what a feature may ask of the app
//! that another owner carries out. A short whitelist and a verb, never a gesture: the app
//! (`app/verbs.rs`) routes each to the one that owns it, so a slice does not hand a message to the
//! UI for the UI to hand on (the old `Wish::{Status, Model}` relay). A wish is for showing
//! something once; a verb is for changing something.

use crate::screens::chat::StatusOverride;
use crate::screens::select::models::Selection;
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
}
