//! The confirmation of `/archive` and `/delete` (`SelectionViewParams::confirmation`): a question
//! with two rows, No first, centered on the screen and at most 72 cells wide. In Flutter it is
//! `showDialog<bool>` over an `AlertDialog`: the question is the constructor argument, the cursor
//! is the picker's, and the only thing that leaves it is the answer Yes, which reaches the app as
//! a verb (`Intent::Confirmed`); No and Esc close it and say nothing. Nothing of it lives in the
//! model once it is shown (ADR-013: a page is the sheet's, the model only asked for it).
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::rc::Rc;

use rtwk::layout::{Alignment, EdgeInsets, Padding};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::{Confirm, Intent};

use super::{lists, strings};
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

/// The sheet of one confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmSheet {
    pub confirm: Confirm,
}

pub struct ConfirmSheetState;

impl StatefulWidget for ConfirmSheet {
    type State = ConfirmSheetState;

    fn create_state(&self) -> ConfirmSheetState {
        ConfirmSheetState
    }
}

impl_widget!(stateful eq ConfirmSheet);

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl State for ConfirmSheetState {
    type Widget = ConfirmSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &ConfirmSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == Confirm::YES => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Confirmed(w.confirm)));
                close_dialog::<_, ()>(cx, None);
            }
            Event::Choose(id) if id == Confirm::NO => close_dialog::<_, ()>(cx, None),
            Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &ConfirmSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let list = lists::confirmation(w.confirm, strings::of(cx));
        let picker = Picker::new(Rc::new(list), cx.map(Msg::Picked));
        // A confirmation floats: 72 cells wide at most and two cells in from the screen
        // (`dialog_area` of the oracle's view stack).
        Padding::new(
            EdgeInsets::symmetric(2, 0),
            Dialog::bare(picker).max_width(72),
        )
        .into()
    }
}

/// The options a confirmation is shown with: centered, not at the bottom (the oracle's
/// `ViewPresentation::Centered`), built under the providers of the page that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        alignment: Alignment::CENTER,
        ..crate::sheets::sheet_options()
    }
}
