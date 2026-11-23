//! The two dialogs of the command center, shown with `show_dialog` (rtwk's
//! overlay route: the modal focus scope, the barrier and the focus given
//! back to the list are the dialog's), never drawn from the screen's state.
//! Both take what identifies them (the confirmed removal; nothing for the
//! failure, which the model holds) and read the model under the screen.
//! Oracle: `confirm_agents_overview_action` (a centered selection view with
//! `SelectionViewParams::confirmation()`) and `run_agents_overview_action`
//! (the `LifecycleHeader` selection view of a refused removal).

use rtwk::focus::{Focus, KeyPress};
use rtwk::overlay::close_dialog;
use rtwk::provider::{read, select};
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::Intent as AgentsIntent;
use codex_tui_core::screens::agents::{Action, Model};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, Shared};

use super::{strings, view};
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::widgets::options::Options;

fn model_of<S>(cx: &mut Cx<'_, S>) -> Shared<Model> {
    select::<ScreenStates, Shared<Model>, _>(cx, |states| states.agents.clone())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Move(i64),
    Enter,
    Pick(usize),
}

/// The archive and delete confirmation; a yes goes to the screen through
/// `answer` (a no, Esc or the barrier only close it).
#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmDialog {
    pub ask: view::Ask,
    pub answer: Handler<bool>,
}

pub struct ConfirmDialogState {
    options: Options,
}

impl StatefulWidget for ConfirmDialog {
    type State = ConfirmDialogState;

    fn create_state(&self) -> ConfirmDialogState {
        ConfirmDialogState {
            options: Options::new(),
        }
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
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &ConfirmDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Move(delta) => self.options.step(delta, 2),
            Msg::Enter => Self::answer(self.options.cursor() == 1, w, cx),
            // A delete waits for Enter: the digit only moves the cursor.
            Msg::Pick(1) if w.ask.confirm.action == Action::Delete => self.options.set(1),
            Msg::Pick(index) => Self::answer(index == 1, w, cx),
        }
        Rebuild::No
    }

    fn build(&self, w: &ConfirmDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let look = view::Look::new(&Theme::of(cx), false);
        let panel = view::confirm_dialog(&look, strings, &num, &w.ask, &self.options);
        // The arrows, Enter and the digits are the dialog's; Esc is left to
        // the `Dialog`.
        Focus::new(panel)
            .can_request_focus(false)
            .bind(KeyPress::UP, cx.send(Msg::Move(-1)))
            .bind(KeyPress::DOWN, cx.send(Msg::Move(1)))
            .bind(KeyPress::ENTER, cx.send(Msg::Enter))
            .bind('1', cx.send(Msg::Pick(0)))
            .bind('2', cx.send(Msg::Pick(1)))
            .into()
    }
}

/// The refusal of a removal: one row, `Back to agents`. It closes when the
/// model drops the failure; its keys tell the model to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureDialog;

/// Esc, Enter or `1`: the refusal is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dismiss;

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
    type Msg = Dismiss;

    fn did_change_dependencies(&mut self, _: &FailureDialog, cx: &mut Cx<'_, Self>) {
        let failing = select::<ScreenStates, bool, _>(cx, |states| states.agents.failure.is_some());
        if !failing && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = failing;
    }

    fn update(&mut self, _: Dismiss, _: &FailureDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Agents(AgentsIntent::DismissFailure),
        );
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
            .bind(KeyPress::ESC, cx.send(Dismiss))
            .bind(KeyPress::ENTER, cx.send(Dismiss))
            .bind('1', cx.send(Dismiss))
            .into()
    }
}
