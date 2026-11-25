//! The transcript the page shows (the reader, with the welcome blossom over an empty chat) and
//! what its keys do: the selection's copy keys, browsing the prompts.

use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Alignment, Stack};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::screens::backtrack::Targets;
use codex_tui_core::threads::ThreadModel;

use crate::app::view::Hs;
use crate::pages::session::backtrack;
use crate::pages::session::live::browse::{self, Browse};
use crate::pages::session::transcript::copy::{self, CopyJob};
use crate::pages::session::transcript::selection::Key as SelectionKey;
use crate::pages::session::transcript_pane::Pane;

use super::frame::Frame;
use super::{Chat, ChatState};

/// A key that is the transcript selection's (copy, copy and follow, clear).
pub(super) fn selected(w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs, key: SelectionKey) {
    let selection = w.ui.selection.controller();
    if key == SelectionKey::Clear {
        selection.clear();
        return;
    }
    let text = selection.text();
    if !text.is_empty() {
        // a copy mode that ended on Enter keeps what its target copies as is (a response's
        // Markdown source, a code block): the copy of the selection it left is that
        let job = copy::mode_of(&w.ui.find).shape(CopyJob::selected(
            text,
            true,
            &read::<TuiSettings, _>(cx),
        ));
        let job = if key == SelectionKey::CopyAndFollow {
            job.following()
        } else {
            job
        };
        cx.emit(&hs.copy, job);
    }
}

/// A key of the mode that browses the prompts.
pub(super) fn browsed(w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs, step: Browse) {
    let modes = &w.ui.modes;
    let thread = read::<ThreadModel, _>(cx);
    let targets = Targets::of(thread.transcript(), thread.can_load_history());
    browse::take(&step, modes, &w.ui.follow, targets.count);
    // the prompt chosen: the transcript is the chat's again; the
    // edit itself is the thread's intent (`ThreadIntent::EditPrompt`)
    if step == Browse::Confirm
        && let Some(chosen) = modes.confirm(Some(thread.id()), targets.count)
    {
        modes.close();
        if let Some(intent) = backtrack::edit_intent(cx, &thread, &chosen) {
            cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
        }
    }
}

/// The reader, and over an empty chat the welcome blossom.
pub(super) fn pane(state: &ChatState, w: &Chat, frame: &Frame) -> Stack {
    // Ctrl+T shows the details in the chat's own transcript, the composer
    // below keeps the keys; browsing the prompts marks one in it. The
    // modes tick when either moves (Flutter's `ListenableBuilder`).
    let reader = {
        let (follow, modes) = (w.ui.follow.clone(), w.ui.modes.clone());
        let (selection, find) = (w.ui.selection.clone(), w.ui.find.clone());
        ListenableBuilder::new(w.ui.modes.listenable(), move |_, _| {
            Pane {
                follow: follow.clone(),
                selection: selection.clone(),
                find: find.clone(),
                detailed: modes.detailed(),
                mark: modes.selected(),
            }
            .into()
        })
    };
    let facts = &frame.drawn.status;
    let thread = &frame.thread;
    let mut transcript = Stack::new().alignment(Alignment::CENTER).child(reader);
    // The oracle's first screen (`first_screen_composer`): startup metadata only in the thread,
    // the ordinary composer empty, the rich transcript, no search or sheet of the reader over it.
    if facts.animations
        && facts.empty_state
        && state.draft_empty
        && !state.shortcuts
        && frame.searching.is_none()
        && !frame.drawn.raw_output
        && crate::pages::session::blossom::startup_only(thread.transcript())
        && thread.last_submit().is_none()
        && !thread.read_only()
    {
        // the detailed transcript (Ctrl+T) hides the blossom and gives it back when it closes
        let modes = w.ui.modes.clone();
        transcript = transcript.child(ListenableBuilder::new(
            w.ui.modes.listenable(),
            move |_, _| {
                if modes.detailed() {
                    AnyWidget::default()
                } else {
                    crate::pages::session::blossom::welcome()
                }
            },
        ));
    }
    transcript
}
