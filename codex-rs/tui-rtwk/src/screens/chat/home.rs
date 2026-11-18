//! The skeleton's chat screen: the transcript as plain text, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::rc::Rc;

use rtwk::editing::TextArea;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::{Session, SessionSlice};
use crate::ids::{ThreadId, Ticket};
use crate::l10n::strings as l10n;
use crate::theme;
use crate::threads::{Failure, Item, ThreadModel};
use crate::ui::debug_menu::DebugMenu;
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

/// What the composer last saw of its thread: the thread and the ticket of the
/// last submit the model accepted (ids only, R14).
pub struct ChatState {
    seen: Option<(ThreadId, Option<Ticket>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    Submit(Rc<str>),
    Quit,
    /// The hidden key: the debug menu over the chat.
    DebugMenu,
}

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState { seen: None }
    }
}

impl_widget!(stateful eq Chat);

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn did_change_dependencies(&mut self, w: &Chat, cx: &mut Cx<'_, Self>) {
        // The composer clears when the model accepted (or queued) the
        // message, not when it was sent: a change of the last accepted
        // ticket in the same thread.
        let now = select::<ThreadModel, (ThreadId, Option<Ticket>), _>(cx, |thread| {
            (thread.id.clone(), thread.last_submit)
        });
        if let Some((thread, before)) = &self.seen
            && *thread == now.0
            && *before != now.1
            && now.1.is_some()
        {
            w.ui.composer.clear();
        }
        self.seen = Some(now);
    }

    fn update(&mut self, msg: ChatMsg, _w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        match msg {
            ChatMsg::Submit(text) => {
                // The decision is `apply`'s (R10): it starts a turn or queues
                // the message; the composer clears when it has.
                if !text.trim().is_empty() {
                    cx.emit(&hs.submit, text.to_string());
                }
            }
            ChatMsg::Quit => cx.emit(&hs.quit, ()),
            ChatMsg::DebugMenu => {
                drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(|_| DebugMenu.into())));
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Chat, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let thread = watch::<ThreadModel, _>(cx);
        let session = watch::<SessionSlice, _>(cx);
        let rows = thread.clone();
        let strings = l10n::of(cx);
        let row_tokens = tokens.clone();
        let row_strings = strings;
        let transcript = ListView::builder(thread.transcript.len(), move |index| {
            match rows.transcript.get(index) {
                Some((_, Item::User { text })) => {
                    Text::new(row_strings.user_line(text).to_string())
                        .style(row_tokens.bold())
                        .into()
                }
                Some((_, Item::Agent { text })) => {
                    Text::new(row_strings.agent_line(text).to_string()).into()
                }
                None => AnyWidget::default(),
            }
        })
        .stick_to_bottom(true);
        let status = match (&session.status, thread.is_running(), &thread.last_error) {
            (Session::Failed(message), _, _) => {
                Text::new(strings.error(message).to_string()).style(tokens.error())
            }
            (_, _, Some(Failure::Message(message))) => {
                Text::new(strings.error(message).to_string()).style(tokens.error())
            }
            (_, _, Some(Failure::TurnFailed)) => {
                Text::new(strings.chat.turn_failed).style(tokens.error())
            }
            (Session::Starting, _, _) => Text::new(strings.chat.starting).style(tokens.dim()),
            (Session::Ready, true, _) => Text::new(strings.chat.working).style(tokens.accent()),
            (Session::Ready, false, _) => Text::new(strings.chat.ready).style(tokens.dim()),
        };
        let composer = TextArea::new()
            .controller(w.ui.composer.clone())
            .placeholder(strings.chat.placeholder)
            .min_lines(1)
            .max_lines(6)
            .autofocus(true)
            .on_submitted(cx.map(ChatMsg::Submit));
        let mut body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, transcript))
            .child(status);
        for text in &thread.queue {
            body = body.child(Text::new(strings.queued(text).to_string()).style(tokens.dim()));
        }
        let body = body.child(composer);
        let quit = KeyPress::char('c').with_mods(Mods::CTRL);
        Focus::new(body)
            .bind(quit, cx.send(ChatMsg::Quit))
            .bind(KeyPress::code(KeyCode::F12), cx.send(ChatMsg::DebugMenu))
            .into()
    }
}
