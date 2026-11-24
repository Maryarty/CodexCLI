//! The terminal title the page keeps telling (`window_title`): the status items, a spinner while
//! a turn runs or compacts, and the action-required prefix.

use rtwk::binding::TerminalTitle;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::RequestQueue;

use crate::pages::session::status_surfaces::TitleInputs;
use crate::pages::session::window_title;

use super::ChatState;
use super::frame::Frame;

pub(super) fn titles(cx: &mut Cx<'_, ChatState>, frame: &Frame) -> Vec<AnyWidget> {
    let facts = &frame.drawn.status;
    let mut titles: Vec<AnyWidget> = Vec::new();
    let action_required = select::<RequestQueue, bool, _>(cx, |queue| !queue.entries.is_empty());
    let running = frame.run.running();
    let compacting = frame.run.compacting();
    let title = frame.context().title(
        facts,
        TitleInputs {
            spinner: facts.animations && facts.progress_effect && (running || compacting),
            action_required: action_required.then_some(frame.strings.chat.action_required),
        },
    );
    if title.has_spinner() {
        titles.push(window_title::WindowTitle { title }.into());
    } else if !title.text(None).is_empty()
        || facts
            .terminal_title
            .as_ref()
            .is_some_and(|items| !items.is_empty())
    {
        titles.push(TerminalTitle::new(title.text(None), window_title::empty()).into());
    }
    titles
}
