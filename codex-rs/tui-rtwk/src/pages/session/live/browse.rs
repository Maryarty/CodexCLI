//! The keys of browsing prompts on the owned screen (`handle_backtrack_preview_key`,
//! `navigate_pager`): while the second Esc's mode is on, they come before the
//! composer's; any other key or paste ends browsing and goes on to the composer
//! (`app.rs`, "leave browsing before unhandled editing input").
//!
//! Flutter's `FocusNode.onKeyEvent` of the composer's node: the primary focus
//! gets a key first, so the mode's keys are read there, never by an ancestor.

use rtwk::focus::KeyPress;

use codex_tui_core::keymap::fixed;
use codex_tui_core::keymap::{ActionId, Keymap};

use crate::keymap::presses;
use crate::pages::session::keys::Action;
use crate::pages::session::transcript::pager;
use crate::widgets::thread_ui::{Follow, Modes};

/// One step of the browsing mode.
#[derive(Clone, Debug, PartialEq)]
pub enum Browse {
    /// A key of the pager: the transcript scrolls.
    Pager(pager::Msg),
    /// Left: the next older prompt.
    Older,
    /// Right: the next newer prompt.
    Newer,
    /// Enter: the prompt marked is the one to edit.
    Confirm,
    /// Ctrl+T: the transcript shows its details, or hides them; browsing goes on.
    Details,
    /// Esc: browsing ends, the transcript is as it was.
    Cancel,
    /// Another key or a paste: browsing ends, the key goes on to the composer.
    Leave,
}

/// What a key means while prompts are browsed: none when it is the composer's.
pub fn key(keymap: &Keymap, press: &KeyPress) -> Option<Browse> {
    let step = |id: ActionId, step: Browse| {
        presses(keymap, id)
            .into_iter()
            .map(move |key| (key, step.clone()))
            .collect::<Vec<_>>()
    };
    [
        step(fixed::DISMISS, Browse::Cancel),
        step(fixed::ENTER, Browse::Confirm),
        step(fixed::BROWSE_OLDER, Browse::Older),
        step(fixed::BROWSE_OLDER_VIM, Browse::Older),
        step(fixed::BROWSE_NEWER, Browse::Newer),
        step(fixed::BROWSE_NEWER_VIM, Browse::Newer),
    ]
    .into_iter()
    .flatten()
    .chain(
        pager::bindings(keymap, false)
            .into_iter()
            .map(|(key, msg)| (key, Browse::Pager(msg))),
    )
    .chain(step(Action::OpenTranscript.id(), Browse::Details))
    .find(|(key, _)| key == press)
    .map(|(_, step)| step)
}

/// Takes the step: the prompt marked moves (`prompts`: how many there are),
/// the transcript scrolls, browsing ends. Enter is the chat's: it knows the
/// thread the prompt is of.
pub fn take(step: &Browse, modes: &Modes, follow: &Follow, prompts: usize) {
    match step {
        Browse::Pager(pager::Msg::Rows(rows)) => follow.scroll_by(*rows),
        Browse::Pager(pager::Msg::Pages(pages)) => follow.page(*pages),
        Browse::Pager(pager::Msg::HalfPages(pages)) => follow.half_page(*pages),
        Browse::Pager(pager::Msg::Top) => follow.jump_to_first(),
        Browse::Pager(pager::Msg::Bottom) => follow.jump_to_latest(),
        // `q` and Ctrl+C: the detailed transcript is done (`close_transcript_overlay`)
        Browse::Pager(pager::Msg::Close | pager::Msg::Action) => modes.close(),
        Browse::Older => {
            modes.older(prompts);
        }
        Browse::Newer => {
            modes.newer(prompts);
        }
        Browse::Details => modes.toggle_detailed(),
        Browse::Cancel | Browse::Leave => modes.leave_browsing(),
        Browse::Confirm => {}
    }
}
