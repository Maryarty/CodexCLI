//! The host of the small dialogs: one widget over the page it was opened for. The page is the
//! widget's own (ADR-013: which dialog is up, the keyboard page of the shortcut overlay, the
//! warnings viewer's page are drawn state, `State` here); the model holds
//! the warnings the session retains, the host's attachments and the prompt a feature asked for,
//! and takes the verbs the user's decisions are (`Decide`...). `/feedback` is the feature's own
//! flow (`features/feedback`).

use rtwk::focus::KeyBindings;
use rtwk::overlay::close_dialog;
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    GLOBAL_COPY, GLOBAL_OPEN_WARNINGS, LIST_CANCEL, LIST_MOVE_LEFT, LIST_MOVE_RIGHT,
};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::dialogs::Intent;

use super::shortcuts::ShortcutView;
use super::warnings::{Pages, Turn};
use super::{shortcuts, strings, warnings};
use crate::app::view::Hs;
use crate::keymap::{bindings, first_of_each, keymap};
use crate::pages::session::transcript::copy::CopyJob;

/// The dialog a `SmallDialog` is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The shortcut overlay, list page.
    Shortcuts,
    /// The warnings viewer over the warnings the session retains when it opened.
    Warnings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmallDialog {
    pub(super) page: Page,
}

impl SmallDialog {
    pub fn of(page: Page) -> Self {
        Self { page }
    }
}

pub struct SmallDialogState {
    /// The shortcut overlay's page.
    shortcuts: ShortcutView,
    /// The warnings viewer's pages (frozen when it opened).
    warnings: Option<Pages>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    Toggle,
    Turn(Turn),
}

impl StatefulWidget for SmallDialog {
    type State = SmallDialogState;

    fn create_state(&self) -> SmallDialogState {
        SmallDialogState {
            shortcuts: ShortcutView::List,
            warnings: None,
        }
    }
}

impl_widget!(stateful eq SmallDialog);

impl SmallDialogState {
    /// Closes the dialog, the warnings viewer first saying what the user decided.
    fn finish(&mut self, page: Page, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        match page {
            Page::Warnings => {
                if let Some(decision) = self.warnings.as_ref().and_then(Pages::decision) {
                    cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::Decide(decision)));
                }
            }
            Page::Shortcuts => {}
        }
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for SmallDialogState {
    type Widget = SmallDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, widget: &SmallDialog, cx: &mut Cx<'_, Self>) {
        match widget.page {
            Page::Warnings => {
                if self.warnings.is_none() {
                    let retained = read::<ScreenStates, _>(cx).dialogs.retained.clone();
                    self.warnings = Some(Pages::new(retained));
                }
            }
            Page::Shortcuts => {}
        }
    }

    fn update(&mut self, msg: Msg, widget: &SmallDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        match msg {
            Msg::Close => {
                self.finish(widget.page, cx);
                Rebuild::No
            }
            Msg::Toggle => {
                self.shortcuts = self.shortcuts.other();
                Rebuild::Yes
            }
            Msg::Turn(turn) => {
                let Some(pages) = &mut self.warnings else {
                    return Rebuild::No;
                };
                match pages.turn(turn) {
                    warnings::Outcome::Stay => Rebuild::Yes,
                    warnings::Outcome::Copy(text) => {
                        // the clipboard verb (oracle `AppEvent::CopyWarning`); the page's
                        // gap shows what came of it
                        cx.emit(&hs.copy, CopyJob::plain(text));
                        Rebuild::No
                    }
                    warnings::Outcome::Finish => {
                        self.finish(widget.page, cx);
                        Rebuild::No
                    }
                }
            }
        }
    }

    fn build(&self, widget: &SmallDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        match widget.page {
            Page::Shortcuts => shortcuts::build(self.shortcuts, s, cx).into(),
            Page::Warnings => match &self.warnings {
                Some(pages) => {
                    let keys = keymap(cx);
                    let mut named = Vec::new();
                    named.extend(bindings(&keys, GLOBAL_OPEN_WARNINGS, &cx.send(Msg::Close)));
                    named.extend(bindings(
                        &keys,
                        GLOBAL_COPY,
                        &cx.send(Msg::Turn(Turn::Copy)),
                    ));
                    // The list keymap's pages and cancel; plain k is the viewer's own (keep).
                    named.extend(bindings(
                        &keys,
                        LIST_MOVE_LEFT,
                        &cx.send(Msg::Turn(Turn::Previous)),
                    ));
                    named.extend(bindings(
                        &keys,
                        LIST_MOVE_RIGHT,
                        &cx.send(Msg::Turn(Turn::Next)),
                    ));
                    named.extend(
                        bindings(&keys, LIST_CANCEL, &cx.send(Msg::Close))
                            .into_iter()
                            .filter(|(name, _)| name != "k" && !name.starts_with("k ")),
                    );
                    KeyBindings::new(first_of_each(named), warnings::build(pages, s, cx)).into()
                }
                None => AnyWidget::default(),
            },
        }
    }
}
