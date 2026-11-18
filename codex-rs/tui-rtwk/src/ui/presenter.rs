//! The request presenter: the one place that opens a dialog for the head of
//! the model's request queue (ADR-002). It sits under the `Navigator`, so
//! the dialog finds the providers. The skeleton's dialog only dismisses the
//! request; the approvals slot replaces it.

use std::time::Duration;

use rtwk::controls::Button;
use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog, show_dialog};
use rtwk::provider::{Provider, read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::RequestQueue;
use crate::ids::RequestKey;
use crate::shell;
use crate::theme;
use crate::ui::view::Hs;

/// Shows the dialog of the queue's head; its child is what it wraps.
pub struct Presenter {
    pub child: Option<AnyWidget>,
}

pub struct PresenterState {
    shown: Option<RequestKey>,
    /// The last request shown: a snapshot that still holds it (the view
    /// has not rerun yet) is not a new request.
    last: Option<RequestKey>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PresenterMsg {
    Closed(RequestKey),
}

impl StatefulWidget for Presenter {
    type State = PresenterState;

    fn create_state(&self) -> PresenterState {
        PresenterState {
            shown: None,
            last: None,
        }
    }
}

impl_widget!(stateful Presenter);

impl PresenterState {
    fn sync(&mut self, head: Option<RequestKey>, cx: &mut Cx<'_, Self>) {
        let (None, Some(key)) = (&self.shown, head) else {
            return;
        };
        if self.last.as_ref() == Some(&key) {
            return;
        }
        self.shown = Some(key.clone());
        self.last = Some(key.clone());
        let shown = key.clone();
        let result = show_dialog::<(), _>(
            cx,
            move |_| DismissDialog { key: shown.clone() }.into(),
            DialogOptions {
                barrier_dismissible: false,
                fade_in: Duration::ZERO,
                ..DialogOptions::default()
            },
        );
        shell::on_dialog_closed(cx, result, PresenterMsg::Closed(key));
    }
}

impl State for PresenterState {
    type Widget = Presenter;
    type Msg = PresenterMsg;

    fn did_change_dependencies(&mut self, _: &Presenter, cx: &mut Cx<'_, Self>) {
        let head = select::<RequestQueue, Option<RequestKey>, _>(cx, |queue| {
            queue.head().map(|request| request.key.clone())
        });
        self.sync(head, cx);
    }

    fn update(&mut self, msg: PresenterMsg, _: &Presenter, cx: &mut Cx<'_, Self>) -> Rebuild {
        let PresenterMsg::Closed(key) = msg;
        if self.shown.as_ref() == Some(&key) {
            self.shown = None;
        }
        // A request that came while the dialog was closing.
        let head = read::<RequestQueue, _>(cx)
            .head()
            .map(|request| request.key.clone());
        self.sync(head, cx);
        Rebuild::No
    }

    fn build(&self, w: &Presenter, _: &mut Cx<'_, Self>) -> AnyWidget {
        match &w.child {
            Some(child) => child.clone(),
            None => AnyWidget::default(),
        }
    }
}

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
        // The dialog closes itself when its request left the queue.
        if Provider::<RequestQueue>::maybe_of(cx, false).is_none() {
            return;
        }
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
        Focus::new(
            Dialog::new(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(Text::new(format!("{} needs an answer.", w.key.id)))
                    .child(Text::new("This build cannot answer it yet.").style(theme::dim()))
                    .child(
                        Button::new("Dismiss")
                            .autofocus(true)
                            .on_pressed(cx.send(DismissMsg::Dismiss)),
                    ),
            )
            .title("Request"),
        )
        .can_request_focus(false)
        .bind(rtwk::focus::KeyPress::ESC, cx.send(DismissMsg::Dismiss))
        .into()
    }
}
