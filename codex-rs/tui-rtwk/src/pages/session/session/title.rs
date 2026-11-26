//! The terminal title the page keeps telling (`window_title`): the status items, a spinner while
//! a turn runs or compacts, and the action-required prefix.

use rtwk::animation::SpinnerFrames;
use rtwk::binding::TerminalTitle;
use rtwk::layout::SizedBox;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::RequestQueue;

use crate::pages::session::status_surfaces::{self, TitleInputs};
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
    let waiting = frame.run.wait_only();
    if title.has_spinner() && !waiting {
        titles.push(window_title::WindowTitle { title }.into());
    } else {
        let steady_frame = SpinnerFrames::DOTS.frames.first().filter(|_| waiting);
        let text = title.text(steady_frame.map(AsRef::as_ref));
        if !text.is_empty()
            || facts
                .terminal_title
                .as_ref()
                .is_some_and(|items| !items.is_empty())
        {
            titles.push(TerminalTitle::new(text, window_title::empty()).into());
        }
    }
    // a title that names the credits or the cost wants them, whether or not it shows them yet
    let wanted = status_surfaces::title_interests(facts);
    if wanted.is_empty() {
        return titles;
    }
    let spare = || -> AnyWidget { SizedBox::new(Some(0), Some(0), None).into() };
    let child = titles.pop().unwrap_or_else(spare);
    titles.push(status_surfaces::raise(wanted, child));
    titles
}
