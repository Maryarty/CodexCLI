//! The view half of `/copy` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::copy` (the last response, the copy a row of the picker or Ctrl+O
//! asks). The picker sheet that shows `Wish::Picker` (its rows are cut from the response's
//! Markdown source: the whole response, then each code block and quote) is `picker`.

pub mod picker;
pub mod strings;
pub mod targets;

use std::rc::Rc;

use rtwk::AnyWidget;

use codex_tui_core::features::Scope;
use codex_tui_core::features::copy::{
    Copier, Intent, Note, Wish, copy_markdown, strip_line_directives,
};
use codex_tui_core::keymap::actions::GLOBAL_COPY;

use super::{FeatureAction, FeatureView, Shown};
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
    fn in_place(_wish: &Wish, ui: &ThreadUi) -> bool {
        begins_mode(ui)
    }

    /// The picker opens once for the wish, at the bottom of the chat; nothing keeps a scope's sheet
    /// open for it.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Picker { response, .. } = wish.clone();
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                picker::CopyPicker {
                    response: response.clone(),
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
