//! The two dialogs of the command center, shown with `show_dialog` (rtwk's
//! overlay route: the modal focus scope, the barrier and the focus given
//! back to the list are the dialog's), never drawn from the screen's state.
//! Both take what identifies them (the confirmed removal; nothing for the
//! failure, which the model holds) and read the model under the screen.
//! Oracle: `confirm_agents_overview_action` (a centered selection view with
//! `SelectionViewParams::confirmation()`) and `run_agents_overview_action`
//! (the `LifecycleHeader` selection view of a refused removal).

use rtwk::controls::SelectionController;
use rtwk::focus::{Focus, KeyPress};
use rtwk::overlay::close_dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::Intent as AgentsIntent;
use codex_tui_core::screens::agents::{Action, Model};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, Shared};

use super::{strings, view};
use crate::l10n::strings as shared;
use crate::ui::view::Hs;

fn model_of<S>(cx: &mut Cx<'_, S>) -> Shared<Model> {
    select::<ScreenStates, Shared<Model>, _>(cx, |states| states.agents.clone())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Key(KeyPress),
    Pick(usize),
}

fn digit(key: KeyPress) -> Option<Msg> {
    let digit = key.as_text().is_some_and(|text| matches!(text, "1" | "2"));
    (digit && !key.mods.contains(Mods::CTRL)).then_some(Msg::Key(key))
}

/// The archive and delete confirmation; a yes goes to the screen through
/// `answer` (a no, Esc or the barrier only close it).
#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmDialog {
    pub ask: view::Ask,
    pub answer: Handler<bool>,
}

pub struct ConfirmDialogState {
    selection: SelectionController<usize>,
}

impl StatefulWidget for ConfirmDialog {
    type State = ConfirmDialogState;

    fn create_state(&self) -> ConfirmDialogState {
        let selection = SelectionController::new();
        selection.set_cursor(Some(0));
        ConfirmDialogState { selection }
    }
}

impl_widget!(stateful eq ConfirmDialog);

impl ConfirmDialogState {
    fn answer(yes: bool, w: &ConfirmDialog, cx: &mut Cx<'_, Self>) {
        if yes {
            cx.emit(&w.answer, true);
        }
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for ConfirmDialogState {
    type Widget = ConfirmDialog;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.selection.dispose();
    }

    fn update(&mut self, msg: Msg, w: &ConfirmDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Key(key) => {
                let Some(digit @ ("1" | "2")) =
                    key.as_text().filter(|_| !key.mods.contains(Mods::CTRL))
                else {
                    return Rebuild::No;
                };
                let index = usize::from(digit == "2");
                // A delete waits for Enter: the digit only moves the cursor.
                if index == 1 && w.ask.confirm.action == Action::Delete {
                    self.selection.set_cursor(Some(1));
                } else {
                    Self::answer(index == 1, w, cx);
                }
            }
            Msg::Pick(index) => Self::answer(index == 1, w, cx),
        }
        Rebuild::No
    }

    fn build(&self, w: &ConfirmDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let look = view::Look::new(&Theme::of(cx), false);
        let panel = view::confirm_dialog(
            &look,
            strings,
            &num,
            &w.ask,
            &self.selection,
            cx.map(Msg::Pick),
        );
        // Only the digits are the dialog's; Esc is left to the `Dialog`.
        Focus::new(panel).on_key(cx.keys(digit)).into()
    }
}

/// The refusal of a removal: one row, `Back to agents`. It closes when the
/// model drops the failure; its keys tell the model to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureDialog;

pub struct FailureDialogState {
    shown: bool,
}

impl StatefulWidget for FailureDialog {
    type State = FailureDialogState;

    fn create_state(&self) -> FailureDialogState {
        FailureDialogState { shown: false }
    }
}

impl_widget!(stateful eq FailureDialog);

impl State for FailureDialogState {
    type Widget = FailureDialog;
    type Msg = KeyPress;

    fn did_change_dependencies(&mut self, _: &FailureDialog, cx: &mut Cx<'_, Self>) {
        let failing =
            select::<ScreenStates, bool, _>(cx, |states| states.agents.failure.is_some());
        if !failing && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = failing;
    }

    fn update(&mut self, key: KeyPress, _: &FailureDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        if matches!(key.code, KeyCode::Escape | KeyCode::Enter)
            || key.as_text().is_some_and(|text| text == "1")
        {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::DismissFailure));
        }
        Rebuild::No
    }

    fn build(&self, _: &FailureDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let agents = model_of(cx);
        let look = view::Look::new(&Theme::of(cx), agents.thread_colors());
        let Some(failure) = &agents.failure else {
            return AnyWidget::default();
        };
        Focus::new(view::failure_dialog(&look, strings, failure))
            .autofocus()
            .on_key(cx.keys(Some))
            .into()
    }
}
