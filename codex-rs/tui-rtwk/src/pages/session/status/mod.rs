//! The bottom pane's activity slots above the composer: the Working row
//! (`../activity.rs`), the pending preview (`pending`) and the background-terminal
//! line (`terminals`), the oracle's `status_indicator_widget.rs`,
//! `bottom_pane/pending_input_preview.rs` and `bottom_pane/unified_exec_footer.rs`.

pub mod entries;
mod pending;
mod questions;
pub mod strings;
mod terminals;

pub(super) use pending::{Pending, PendingInput};
pub(super) use questions::QuestionSummary;
pub(super) use terminals::TerminalsLine;

use rtwk::style::{Attrs, Style};

/// The oracle's `.dim()`: the terminal's dim attribute on the cell's own colour.
pub(super) fn dim() -> Style {
    Style::new().attrs(Attrs::DIM)
}

/// The key in the hints, `.bold()` on the cell's own colour.
pub(super) fn bold() -> Style {
    Style::new().attrs(Attrs::BOLD)
}

/// The values the status-line setup (or, `title`, the terminal-title setup) previews for the
/// items `ids` (`status_surface_preview_data`): the live value of each, an empty text where its
/// placeholder is left out. Read from the model as it stands, outside the chat page's build.
pub fn setup_values<S: 'static>(
    cx: &mut rtwk::Cx<'_, S>,
    title: bool,
    ids: &[&str],
) -> std::collections::BTreeMap<String, String> {
    use rtwk::provider::read;

    use codex_tui_core::app::{ConnectionState, SessionSlice};
    use codex_tui_core::features::FeatureStates;
    use codex_tui_core::screens::ScreenStates;
    use codex_tui_core::screens::chat;
    use codex_tui_core::screens::status::Account;
    use codex_tui_core::threads::ThreadModel;

    let thread = read::<ThreadModel, _>(cx);
    let session = read::<SessionSlice, _>(cx);
    let states = read::<ScreenStates, _>(cx);
    let features = read::<FeatureStates, _>(cx);
    let strings = crate::l10n::strings::of(cx);
    let owner = states.agents.owner_of(thread.id().as_str());
    let slots = thread.screen_slots();
    let cost = features.thread_usage.figure_of(thread.id());
    let plan = match states.status.account {
        Some(Account::ChatGpt { plan, .. }) => plan,
        Some(Account::ApiKey) | None => None,
    };
    let run = super::activity::Run::of(&thread);
    let reconnecting = session.connections.get(&owner) == Some(&ConnectionState::Reconnecting);
    let context = super::status_surfaces::StatusContext {
        thread_id: session.active.as_ref(),
        thread: Some(&slots.chat),
        usage: Some(&slots.status),
        rate_limits: session.rate_limits.get(&owner),
        run_state: Some(run.word(&session, reconnecting)),
        strings,
        catalog: &states.select.catalog,
        chat: &states.chat.facts,
        raw_output: states.chat.raw_output,
        usage_pending: chat::facts::usage_pending(slots),
        cost: cost.as_ref(),
        plan,
    };
    context.preview_values(title, ids, &states.chat.status)
}
