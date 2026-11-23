//! The skeleton's dialog for a request: it says which request needs an answer
//! and dismisses it. The approvals slice (S13) replaces it; the request
//! presenter asks `super::request_dialog` and knows nothing else.

use rtwk::controls::Button;
use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::RequestQueue;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::keymap::fixed;

use crate::app::theme;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;
use crate::l10n::strings as l10n;

/// The skeleton's dialog for a request: dismiss it.
#[derive(PartialEq)]
pub struct DismissDialog {
    pub key: RequestKey,
}

pub struct DismissDialogState;

#[derive(Clone, Debug, PartialEq)]
pub enum DismissMsg {
    Dismiss,
}

impl StatefulWidget for DismissDialog {
    type State = DismissDialogState;

    fn create_state(&self) -> DismissDialogState {
        DismissDialogState
    }
}

impl_widget!(stateful eq DismissDialog);

impl State for DismissDialogState {
    type Widget = DismissDialog;
    type Msg = DismissMsg;

    fn did_change_dependencies(&mut self, w: &DismissDialog, cx: &mut Cx<'_, Self>) {
        // The dialog closes itself when its request left the queue. The
        // queue is provided above every dialog: a missing provider fails
        // loudly here.
        let key = w.key.clone();
        let still = select::<RequestQueue, bool, _>(cx, move |queue| queue.contains(&key));
        if !still {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn update(&mut self, msg: DismissMsg, w: &DismissDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let DismissMsg::Dismiss = msg;
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.dismiss, w.key.clone());
        close_dialog(cx, Some(()));
        Rebuild::No
    }

    fn build(&self, w: &DismissDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let focus = Focus::new(
            Dialog::new(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(Text::new(
                        strings.needs_answer(w.key.id.as_str()).to_string(),
                    ))
                    .child(Text::new(strings.presenter.cannot_answer).style(tokens.dim()))
                    .child(
                        Button::new(strings.presenter.dismiss)
                            .autofocus(true)
                            .on_pressed(cx.send(DismissMsg::Dismiss)),
                    ),
            )
            .title(strings.presenter.title),
        )
        .can_request_focus(false);
        bind_fixed(focus, fixed::DISMISS, cx.send(DismissMsg::Dismiss)).into()
    }
}
