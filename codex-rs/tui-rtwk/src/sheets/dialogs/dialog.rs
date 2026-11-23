//! The host of the small dialogs: one widget over the page it was opened for. The page is the
//! widget's own (ADR-013: which dialog is up, the keyboard page of the shortcut overlay, the
//! warnings viewer's page and the feedback step are drawn state, `State` here); the model holds
//! the warnings the session retains, the host's attachments and the prompt a feature asked for,
//! and takes the verbs the user's decisions are (`Decide`, `RequestNote`, `Submit`...).

use rtwk::editing::TextEditingController;
use rtwk::focus::{FocusNode, KeyBindings};
use rtwk::overlay::close_dialog;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{GLOBAL_COPY, GLOBAL_OPEN_WARNINGS};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::dialogs::{Attachments, Intent, NoteRequest, Suggestion};

use super::feedback::Step;
use super::shortcuts::ShortcutView;
use super::warnings::{Pages, Turn};
use super::{feedback, prompt, shortcuts, strings, warnings};
use crate::app::view::Hs;
use crate::keymap::{bindings, first_of_each, keymap};

/// The dialog a `SmallDialog` is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The text prompt a feature asked for: the model holds the request.
    Prompt,
    /// The shortcut overlay, list page.
    Shortcuts,
    /// The warnings viewer over the warnings the session retains when it opened.
    Warnings,
    /// `/feedback`: how was this?
    Feedback,
    /// `/feedback` past the category: include logs? (the debug menu's entry opens here).
    Consent,
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
    shown: bool,
    /// The text prompt's field and its focus (`prompt.rs`): UI state, the model keeps only
    /// whether the user typed.
    pub(super) text: TextEditingController,
    pub(super) node: FocusNode,
    /// The prompt's field was seeded with its initial text.
    seeded: bool,
    /// The suggestion put in the field.
    suggested: Option<String>,
    /// The shortcut overlay's page.
    shortcuts: ShortcutView,
    /// The warnings viewer's pages (frozen when it opened).
    warnings: Option<Pages>,
    /// The feedback flow's step.
    feedback: Step,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    Toggle,
    Turn(Turn),
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
            shortcuts: ShortcutView::List,
            warnings: None,
            feedback: match self.page {
                Page::Consent => {
                    Step::Consent(codex_tui_core::screens::dialogs::FeedbackCategory::Bug)
                }
                _ => Step::Category,
            },
        }
    }
}

impl_widget!(stateful eq SmallDialog);

impl SmallDialogState {
    /// Closes the dialog, the warnings viewer first saying what the user decided.
    fn finish(&mut self, page: Page, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        match page {
            Page::Prompt => cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::CancelPrompt)),
            Page::Warnings => {
                if let Some(decision) = self.warnings.as_ref().and_then(Pages::decision) {
                    cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::Decide(decision)));
                }
            }
            Page::Shortcuts | Page::Feedback | Page::Consent => {}
        }
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for SmallDialogState {
    type Widget = SmallDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, widget: &SmallDialog, cx: &mut Cx<'_, Self>) {
        match widget.page {
            Page::Prompt => {
                // The model's request: the dialog closes when the model drops it.
                let prompt = select::<
                    ScreenStates,
                    Option<codex_tui_core::screens::dialogs::Prompt>,
                    _,
                >(cx, |states| states.dialogs.prompt.clone());
                if prompt.is_none() && self.shown {
                    close_dialog::<_, ()>(cx, None);
                }
                self.shown = prompt.is_some();
                // A prompt's field starts with its initial text, and takes the generated
                // suggestion while the model says it applies (`apply_text_suggestion`: only
                // before the user typed).
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
            Page::Warnings => {
                if self.warnings.is_none() {
                    let retained = read::<ScreenStates, _>(cx).dialogs.retained.clone();
                    self.warnings = Some(Pages::new(retained));
                }
            }
            Page::Shortcuts | Page::Feedback | Page::Consent => {}
        }
    }

    fn dispose(&mut self) {
        self.text.dispose();
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
                        cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::CopyWarning(text)));
                        Rebuild::No
                    }
                    warnings::Outcome::Finish => {
                        self.finish(widget.page, cx);
                        Rebuild::No
                    }
                }
            }
            Msg::Category(index) => {
                if let (Step::Category, Some(category)) = (self.feedback, feedback::category(index))
                {
                    self.feedback = Step::Consent(category);
                    Rebuild::Yes
                } else {
                    Rebuild::No
                }
            }
            Msg::Consent(include_logs) => {
                if let Step::Consent(category) = self.feedback {
                    let note = NoteRequest {
                        category,
                        include_logs,
                    };
                    cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::RequestNote(note)));
                    close_dialog::<_, ()>(cx, None);
                }
                Rebuild::No
            }
            Msg::Typed => {
                cx.emit(&hs.screen, ScreenIntent::Dialogs(Intent::Typed));
                Rebuild::No
            }
            // A blank text is not an answer: the prompt stays open (`CustomPromptView`).
            Msg::Submit(text) if text.trim().is_empty() => Rebuild::No,
            Msg::Submit(text) => {
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Dialogs(Intent::Submit(text.trim().to_string())),
                );
                Rebuild::No
            }
        }
    }

    fn build(&self, widget: &SmallDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        match widget.page {
            Page::Prompt => {
                let open = select::<
                    ScreenStates,
                    Option<codex_tui_core::screens::dialogs::Prompt>,
                    _,
                >(cx, |states| states.dialogs.prompt.clone());
                match &open {
                    Some(open) => prompt::build(open, s, self, cx).into(),
                    None => AnyWidget::default(),
                }
            }
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
                    KeyBindings::new(first_of_each(named), warnings::build(pages, s, cx)).into()
                }
                None => AnyWidget::default(),
            },
            Page::Feedback | Page::Consent => {
                let attachments = select::<ScreenStates, Attachments, _>(cx, |states| {
                    states.dialogs.attachments.clone()
                });
                feedback::build(self.feedback, &attachments, s, cx).into()
            }
        }
    }
}
