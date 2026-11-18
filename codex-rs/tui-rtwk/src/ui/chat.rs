//! The skeleton's chat screen: the transcript as plain text, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::rc::Rc;

use rtwk::editing::TextArea;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded};
use rtwk::prelude::Mods;
use rtwk::provider::{read, watch};
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::{Session, SessionSlice};
use crate::theme;
use crate::threads::{Item, ThreadModel};
use crate::ui::presenter::Presenter;
use crate::ui::thread_ui::ThreadUi;
use crate::ui::view::Hs;

/// The chat screen of one thread's controllers.
#[derive(Clone)]
pub struct Chat {
    pub ui: Rc<ThreadUi>,
}

impl PartialEq for Chat {
    fn eq(&self, other: &Chat) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
    }
}

pub struct ChatState;

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    Submit(Rc<str>),
    Quit,
}

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState
    }
}

impl_widget!(stateful eq Chat);

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn update(&mut self, msg: ChatMsg, w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        match msg {
            ChatMsg::Submit(text) => {
                // The decision is `apply`'s (R10): a submit while a turn runs
                // is refused there. The composer's queueing of such a
                // message is S10's (parity pack, delta).
                if !text.trim().is_empty() {
                    cx.emit(&hs.submit, text.to_string());
                    w.ui.composer.clear();
                }
            }
            ChatMsg::Quit => cx.emit(&hs.quit, ()),
        }
        Rebuild::No
    }

    fn build(&self, w: &Chat, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let thread = watch::<ThreadModel, _>(cx);
        let session = watch::<SessionSlice, _>(cx);
        let rows = thread.clone();
        let transcript = ListView::builder(thread.transcript.len(), move |index| {
            match rows.transcript.get(index) {
                Some((_, Item::User { text })) => {
                    Text::new(format!("› {text}")).style(theme::bold()).into()
                }
                Some((_, Item::Agent { text })) => Text::new(format!("• {text}")).into(),
                None => AnyWidget::default(),
            }
        })
        .stick_to_bottom(true);
        let status = match (&session.status, thread.is_running(), &thread.last_error) {
            (Session::Failed(message), _, _) => {
                Text::new(format!("error: {message}")).style(theme::error())
            }
            (_, _, Some(message)) => Text::new(format!("error: {message}")).style(theme::error()),
            (Session::Starting, _, _) => Text::new("starting…").style(theme::dim()),
            (Session::Ready, true, _) => Text::new("working…").style(theme::accent()),
            (Session::Ready, false, _) => Text::new("ready").style(theme::dim()),
        };
        let composer = TextArea::new()
            .controller(w.ui.composer.clone())
            .placeholder("Ask Codex to do anything")
            .min_lines(1)
            .max_lines(6)
            .autofocus(true)
            .on_submitted(cx.map(ChatMsg::Submit));
        let body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, transcript))
            .child(status)
            .child(composer)
            .child(Presenter { child: None });
        let quit = KeyPress::char('c').with_mods(Mods::CTRL);
        Focus::new(body).bind(quit, cx.send(ChatMsg::Quit)).into()
    }
}
