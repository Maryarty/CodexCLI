//! The host of the small dialogs: it holds the dialog the model has open and
//! draws the page the
//! model names and turns keys into intents.

use rtwk::focus::KeyPress;
use rtwk::overlay::close_dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::dialogs::{Intent, Open, PageIntent};

use super::{feedback, shortcuts, strings, warnings};
use crate::app::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct SmallDialog;

pub struct SmallDialogState {
    shown: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    Toggle,
    Page(PageIntent),
    Category(usize),
    Consent(bool),
}

impl StatefulWidget for SmallDialog {
    type State = SmallDialogState;

    fn create_state(&self) -> SmallDialogState {
        SmallDialogState { shown: false }
    }
}

impl_widget!(stateful eq SmallDialog);

impl State for SmallDialogState {
    type Widget = SmallDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &SmallDialog, cx: &mut Cx<'_, Self>) {
        let open = select::<ScreenStates, bool, _>(cx, |states| states.dialogs.open.is_some());
        if !open && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = open;
    }

    fn update(&mut self, msg: Msg, _: &SmallDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let intent = match msg {
            Msg::Close => {
                cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::Close));
                close_dialog::<_, ()>(cx, None);
                return Rebuild::No;
            }
            Msg::Toggle => Intent::ToggleKeyboard,
            Msg::Page(page) => Intent::Page(page),
            Msg::Category(index) => Intent::PickCategory(index),
            Msg::Consent(include_logs) => Intent::Consent(include_logs),
        };
        cx.emit(&hs.screen, ScreenIntent::Dialogs(intent));
        Rebuild::No
    }

    fn build(&self, _: &SmallDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let open =
            select::<ScreenStates, Option<Open>, _>(cx, |states| states.dialogs.open.clone());
        let Some(open) = &open else {
            return AnyWidget::default();
        };
        let s = strings::of(cx);
        let focus = match open {
            Open::Shortcuts(view) => shortcuts::build(*view, s, cx),
            Open::Warnings(warnings) => warnings::build(warnings, s, cx),
            Open::Feedback(step) => feedback::build(step, s, cx),
            // The text prompt (`/rename`) is built by the view task of chunk-commands.
            Open::Prompt(_) => return AnyWidget::default(),
        };
        focus.into()
    }
}

/// `KeyPress` of F2 (`open_warnings`).
pub(super) fn f2() -> KeyPress {
    KeyPress::code(KeyCode::F2)
}

/// `KeyPress` of Ctrl+O (`copy`).
pub(super) fn ctrl_o() -> KeyPress {
    KeyPress::char('o').with_mods(Mods::CTRL)
}
