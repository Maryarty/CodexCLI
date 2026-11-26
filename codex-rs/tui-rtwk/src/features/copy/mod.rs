//! The view half of `/copy` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::copy` (the last response, the copy a row of the picker or Ctrl+O
//! asks). The picker sheet that shows `Wish::Picker` (its rows are cut from the response's
//! Markdown source: the whole response, then each code block and quote) is `picker`.

pub mod picker;
pub mod strings;
pub mod targets;

use std::rc::Rc;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::Scope;
use codex_tui_core::features::copy::{
    Copier, Intent, Note, Wish, copy_markdown, strip_line_directives,
};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::actions::GLOBAL_COPY;

use super::{FeatureAction, FeatureView, InPlace, Shown};
use crate::app::view::Hs;
use crate::pages::session::strings::Strings as SessionStrings;
use crate::pages::session::transcript::copy as copy_mode;
use crate::widgets::thread_ui::ThreadUi;

/// `/copy` on the chat's own transcript (oracle `show_copy_picker`: the transcript is owned):
/// copy mode begins over `ui`'s transcript, a cursor on the last response, and no sheet opens.
/// False where it cannot begin (Find or another mode of input holds the keys): the picker shows
/// as it does for a transcript the chat does not own. The presenter that shows `Wish::Picker`
/// asks this of the thread's `ThreadUi` first (`FeatureView::in_place`).
pub fn begins_mode(ui: &ThreadUi) -> bool {
    copy_mode::enter(
        ui,
        copy_mode::Cutter {
            pieces,
            whole: copy_markdown,
        },
    )
}

/// The code blocks and quotes written in `source`, the pieces the mode copies alone (a page does
/// not name this feature, so the mode is handed this).
fn pieces(source: &str) -> Vec<copy_mode::Piece> {
    targets::extract(source)
        .into_iter()
        .map(|target| match target {
            targets::Target::Code { content, .. } => copy_mode::Piece::Code(content),
            targets::Target::Quote(source) => {
                let copied = source
                    .split_inclusive('\n')
                    .map(strip_line_directives)
                    .collect();
                copy_mode::Piece::Quote { source, copied }
            }
        })
        .collect()
}

impl FeatureView for Copier {
    /// On the chat's own transcript `/copy` is copy mode, not the picker.
    fn in_place(wish: &Wish, ui: &ThreadUi) -> InPlace<Intent> {
        if matches!(wish, Wish::Status { .. }) {
            return InPlace::Not;
        }
        if begins_mode(ui) {
            return InPlace::Took;
        }
        match wish {
            // Find or another mode holds the keys: the oracle's `Nothing to copy`
            Wish::Mode { thread } => InPlace::Declined(Intent::Nothing {
                thread: thread.clone(),
            }),
            Wish::Picker { .. } | Wish::Status { .. } => InPlace::Not,
        }
    }

    /// The picker opens once for the wish, at the bottom of the chat; nothing keeps a scope's sheet
    /// open for it. `Wish::Mode` has no sheet: the mode began in place or the presenter says it
    /// could not (`Intent::Nothing`).
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let source = match wish.clone() {
            Wish::Picker { response, .. } => picker::Source::Response(response),
            Wish::Status { thread, card } => picker::Source::Status { thread, card },
            Wish::Mode { .. } => return None,
        };
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                picker::CopyPicker {
                    source: source.clone(),
                }
                .into()
            }),
            options: crate::sheets::sheet_options(),
        })
    }

    /// Ctrl+O (`copy_last_response_binding`): the last response goes to the clipboard.
    fn actions() -> Vec<FeatureAction> {
        vec![FeatureAction {
            id: GLOBAL_COPY,
            also: &[],
            run: |keyed| keyed.say(Intent::Last),
        }]
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

/// Commands handled or refused by the page still supersede the thread's Status copy source.
pub fn supersede_status_copy<S: 'static>(
    outcome: &codex_tui_core::commands::Outcome,
    thread: Option<&ThreadId>,
    cx: &mut Cx<'_, S>,
    hs: &Hs,
) {
    use codex_tui_core::commands::{Notice, Outcome, SessionIntent, SlashCommand};

    let supersedes = match outcome {
        Outcome::Open(_) | Outcome::Tier(_) | Outcome::Insert(_) | Outcome::Quit => true,
        Outcome::Session { intent, .. } => matches!(intent, SessionIntent::Vim),
        Outcome::Notice(notice) => {
            notice.is_recalled()
                && !matches!(
                    notice,
                    Notice::DisabledDuringTask(SlashCommand::Copy)
                        | Notice::UnavailableInSide(SlashCommand::Copy)
                        | Notice::UnavailableInReview(SlashCommand::Copy)
                        | Notice::SessionStarting(SlashCommand::Copy)
                        | Notice::NotAvailable(SlashCommand::Copy)
                )
        }
        Outcome::NotACommand => false,
    };
    if supersedes && let Some(thread) = thread {
        super::verb_by(
            cx,
            hs,
            codex_tui_core::features::supersede_copy_status(thread.clone()),
        );
    }
}
