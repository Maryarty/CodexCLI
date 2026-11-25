//! The empty conversation moves to a Cloud backend (oracle `cloud_switch_wait.rs`): the draft
//! stays editable and nothing is submitted while Cloud prepares the destination, the footer says
//! `Connecting to Cloud…`, and a refused start says `Cloud conversation did not open · draft
//! kept` for the frame that heard it. The state is the app's (`SessionSlice::cloud_switch`,
//! derived from the start's ticket); the page words it and restricts its composer.

use codex_tui_core::app::{CloudSwitch, SessionSlice};
use codex_tui_core::screens::composer::restricted::Restricted;

use crate::pages::session::strings::Strings;

/// The footer's hint items while the switch is on (`show_cloud_switch_hint`): the label with no
/// action while connecting, the label and `draft kept` after a refusal.
pub(super) fn hint(session: &SessionSlice, strings: &Strings) -> Option<Vec<(String, String)>> {
    let (label, action) = match session.cloud_switch? {
        CloudSwitch::Connecting => (strings.cloud_connecting, ""),
        CloudSwitch::Failed => (strings.cloud_failed, strings.cloud_failed_action),
    };
    Some(vec![(label.to_owned(), action.to_owned())])
}

/// The composer edits and submits nothing while Cloud is asked (`handle_restricted_key` with
/// `RestrictedInputMode::DraftHandoff`): Enter, Tab and the queue key are swallowed. A refusal
/// restricts nothing: the draft is the user's again.
pub(super) fn restricted(session: &SessionSlice) -> Option<Restricted> {
    (session.cloud_switch == Some(CloudSwitch::Connecting)).then_some(Restricted::DraftHandoff)
}
