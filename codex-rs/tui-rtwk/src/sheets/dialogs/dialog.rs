//! The host of the small dialogs: it holds the dialog the model has open and
//! draws the page the
//! model names and turns keys into intents.

use rtwk::editing::TextEditingController;
use rtwk::focus::{FocusNode, KeyPress};
use rtwk::overlay::close_dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::dialogs::{Intent, Open, PageIntent, Suggestion};

use super::{feedback, prompt, shortcuts, strings, warnings};
use crate::app::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct SmallDialog;

pub struct SmallDialogState {
    shown: bool,
    /// The text prompt's field and its focus (`prompt.rs`): UI state, the model keeps only
    /// whether the user typed.
    pub(super) text: TextEditingController,
    pub(super) node: FocusNode,
    /// The prompt's field was seeded with its initial text.
    seeded: bool,
    /// The suggestion put in the field.
    suggested: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    Toggle,
    Page(PageIntent),
    Category(usize),
    Consent(bool),
    /// The prompt's field changed by the user.
    Typed,
    /// Enter in the prompt's field.
    Submit(std::rc::Rc<str>),
}

impl StatefulWidget for SmallDialog {
    type State = SmallDialogState;

    fn create_state(&self) -> SmallDialogState {
        SmallDialogState {
            shown: false,
            text: TextEditingController::new(""),
            node: FocusNode::new(),
            seeded: false,
            suggested: None,
        }
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
        // A prompt's field starts with its initial text, and takes the generated suggestion
        // while the model says it applies (`apply_text_suggestion`: only before the user typed).
        let prompt = select::<ScreenStates, Option<codex_tui_core::screens::dialogs::Prompt>, _>(
            cx,
            |states| match &states.dialogs.open {
                Some(Open::Prompt(prompt)) => Some(prompt.clone()),
                _ => None,
            },
        );
        if let Some(prompt) = prompt {
            if !self.seeded {
                self.seeded = true;
                self.text.reset(&prompt.initial);
            }
            if let Suggestion::Applied(text) = &prompt.suggestion
                && self.suggested.as_ref() != Some(text)
            {
                self.suggested = Some(text.clone());
                self.text.reset(text);
            }
        }
    }

    fn dispose(&mut self) {
        self.text.dispose();
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
            Msg::Typed => Intent::Typed,
            // A blank text is not an answer: the prompt stays open (`CustomPromptView`).
            Msg::Submit(text) if text.trim().is_empty() => return Rebuild::No,
            Msg::Submit(text) => Intent::Submit(text.trim().to_string()),
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
            Open::Prompt(open) => prompt::build(open, s, self, cx),
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
