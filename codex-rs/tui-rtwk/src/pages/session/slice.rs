//! What the chat page draws of the screens' states (ADR-013: a view selects the slice it draws,
//! never the whole `ScreenStates`, so a change of another screen's model does not wake it).
//! Flutter's `context.select((ScreenStates s) => ...)` over a record of the fields the build reads.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::provider::select;

use codex_tui_core::app::BackendLabel;
use codex_tui_core::commands::{CommandFlags, ServiceTierCommand};
use codex_tui_core::ids::{BackendId, ThreadId};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::chat::{Facts, StatusFacts};
use codex_tui_core::screens::select::models::Catalog;

/// The fields of the screens' states the chat's build reads.
#[derive(Clone, PartialEq)]
pub(super) struct Drawn {
    /// The backend that owns the thread.
    pub owner: BackendId,
    /// What the backend control offers (`SessionSlice::cycle_label`).
    pub offer: Option<BackendLabel>,
    /// What the status line shows (config and workspace facts).
    pub status: Rc<StatusFacts>,
    pub flags: CommandFlags,
    pub tiers: Vec<ServiceTierCommand>,
    pub catalog: Catalog,
    pub facts: Facts,
    pub raw_output: bool,
    /// How many warnings the dialogs retain (`⚠ N warnings`).
    pub warnings: usize,
}

/// The slice of the thread `thread`, selected: it wakes the page when one of these changes.
pub(super) fn drawn<S: 'static>(cx: &mut Cx<'_, S>, thread: &ThreadId) -> Drawn {
    let thread = thread.clone();
    select::<ScreenStates, _, _>(cx, move |states| Drawn {
        owner: states.agents.owner_of(thread.as_str()),
        offer: states.agents.cycle_offer(),
        status: states.chat.status.clone(),
        flags: states.chat.flags,
        tiers: states.chat.service_tiers.clone(),
        catalog: states.select.catalog.clone(),
        facts: states.chat.facts.clone(),
        raw_output: states.chat.raw_output,
        warnings: states.dialogs.retained.len(),
    })
}
