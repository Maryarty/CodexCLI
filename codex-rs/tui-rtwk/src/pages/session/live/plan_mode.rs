//! Plan mode on the chat page: the question "Implement this plan?" the model raises when a
//! turn in Plan mode ends with a proposed plan (`chat::plan_mode::Prompt`).
//!
//! Flutter: a `showModalBottomSheet` of a list with three choices, opened by the page that
//! listens to the controller (`BlocListener`: `widgets/directives.rs`); rtwk's generic selection sheet is that
//! sheet, so this file says WHAT the list holds and when the page opens it. The pick returns to
//! the model as `select::Intent::PlanAnswer` (`plan_mode::Answer`), and the model decides what
//! switching modes and sending the implement message mean.

use rtwk::Cx;
use rtwk::navigation::Route;
use rtwk::provider::read;

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::screens::chat::plan_mode::{Answer, Used};
use codex_tui_core::threads::ThreadModel;

use crate::pages::session::strings::{self, Strings};
use crate::sheets::select::Purpose;

/// The list of the prompt (`plan_implementation::selection_view_params`): three numbered rows,
/// the context's usage in the second row's note.
pub fn menu(s: &Strings, used: Option<Used>) -> List {
    let label = used.map(|used| s.plan_usage(used));
    let row = |id: &str, name: &str, note: String| Item {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(note),
        ..Item::default()
    };
    List {
        title: Some(s.plan_title.to_string()),
        items: vec![
            row(Answer::YES, s.plan_yes, s.plan_yes_note.to_string()),
            row(
                Answer::CLEAR,
                s.plan_clear,
                s.plan_clear_description(label.as_deref()),
            ),
            row(Answer::NO, s.plan_no, s.plan_no_note.to_string()),
        ],
        // `picker()`: stable columns, the notes placed from all rows.
        columns: Columns::AutoAllRows,
        standard_hint: true,
        ..List::default()
    }
}

/// The page's answer to a new prompt (`directives::Edge::Plan`, each number once, in the thread
/// that raised it): opens the sheet, unless another sheet is open (`no_modal_or_popup_active`:
/// the question is skipped, not kept waiting).
pub fn ask<S>(cx: &mut Cx<'_, S>) {
    // another sheet over the page (the route is not on top) is the oracle's other popup
    if !Route::is_current_of(cx) {
        return;
    }
    let thread = read::<ThreadModel, _>(cx);
    let status = &thread.screen_slots().status;
    let used = codex_tui_core::screens::chat::plan_mode::used(status.context, status.tokens);
    let list = menu(strings::of(cx), used);
    drop(crate::sheets::select::pick(
        cx,
        list,
        Purpose::PlanImplementation,
    ));
}
