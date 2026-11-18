//! The skeleton's chat screen: the transcript as plain text, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::rc::Rc;

use rtwk::editing::{Autocomplete, Suggestion, TextArea};
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, Row};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::ListView;
use rtwk::layout::SizedBox;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::app::{Session, SessionSlice};
use codex_tui_rtwk_core::commands::{DispatchCx, Outcome, SlashCommand, Target, dispatch};
use codex_tui_rtwk_core::ids::{ThreadId, Ticket};
use codex_tui_rtwk_core::screens::popovers::{Direction, SearchStatus};
use codex_tui_rtwk_core::screens::{ScreenIntent, ScreenStates, chat, popovers};
use codex_tui_rtwk_core::threads::{Failure, Item, ThreadModel};

use crate::l10n::strings as l10n;
use crate::screens::chat::keys::{self, Action};
use crate::screens::chat::open::open;
use crate::screens::chat::popup;
use crate::screens::chat::strings as chat_strings;
use crate::theme;
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
    /// Escape was pressed on an empty composer: the next one starts
    /// backtrack (`app_backtrack.rs`).
    esc_primed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    Submit(Rc<str>),
    Quit,
    /// The hidden key: the debug menu over the chat.
    DebugMenu,
    /// A key action of the chat (`keys::defaults`).
    Action(Action),
    /// A suggestion was put in: a command is run, the others are text.
    Picked(Rc<str>),
    Escape,
    /// A key of history search.
    Search(SearchKey),
}

/// What a key does while reverse history search is open.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchKey {
    Char(Rc<str>),
    Backspace,
    Older,
    Newer,
    Accept,
    Cancel,
}

/// The key of history search for `key`, if it is one.
fn search_key(key: KeyPress) -> Option<ChatMsg> {
    let ctrl = KeyPress::char('r').with_mods(Mods::CTRL);
    let ctrl_s = KeyPress::char('s').with_mods(Mods::CTRL);
    let found = if key == ctrl {
        SearchKey::Older
    } else if key == ctrl_s {
        SearchKey::Newer
    } else if key.code == KeyCode::Enter && key.mods.is_empty() {
        SearchKey::Accept
    } else if key.code == KeyCode::Escape {
        SearchKey::Cancel
    } else if key.code == KeyCode::Backspace && key.mods.is_empty() {
        SearchKey::Backspace
    } else {
        SearchKey::Char(Rc::from(key.as_text()?))
    };
    Some(ChatMsg::Search(found))
}

/// The words of a typed failure.
fn failure_text(strings: &crate::l10n::strings::Shared, failure: &Failure) -> String {
    match failure {
        Failure::Message(message) => strings.error(message).to_string(),
        Failure::TurnFailed => strings.chat.turn_failed.to_string(),
        Failure::Lagged(skipped) => strings.lagged(*skipped).to_string(),
        Failure::Stopped => strings.chat.server_stopped.to_string(),
        Failure::ReplyMismatch => strings.chat.reply_mismatch.to_string(),
    }
}

/// The composer's prompt, as wide as its text: a trailing blank is a cell.
fn prompt(text: &'static str) -> AnyWidget {
    let width = i32::try_from(text.chars().count()).unwrap_or(2);
    SizedBox::new(Some(width), None, Some(Text::new(text).into())).into()
}

/// A line the user sent from the composer: a command opens its screen, asks
/// the session or says why not (the decision is core's `dispatch`); anything
/// else is a message for the model.
fn run_line(line: &str, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    // An empty line is nothing to run: the composer was already taken by the
    // pick or the submit that came with the same Enter.
    if line.trim().is_empty() {
        return;
    }
    let states = read::<ScreenStates, _>(cx);
    let thread = read::<ThreadModel, _>(cx);
    let session = read::<SessionSlice, _>(cx);
    let facts = DispatchCx {
        flags: states.chat.flags,
        task_running: thread.is_running(),
        side_conversation: states.chat.flags.side_conversation_active,
        review_mode: false,
        thread_started: matches!(session.status, Session::Ready),
        has_backend_auth: states.chat.signed_in,
    };
    let screen = |cx: &mut Cx<'_, ChatState>, intent: chat::Intent| {
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let record = |cx: &mut Cx<'_, ChatState>| {
        let intent = popovers::Intent::Record(line.to_string());
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    };
    let outcome = dispatch(line, &facts);
    if states.chat.notice.is_some() && !matches!(outcome, Outcome::Notice(_)) {
        screen(cx, chat::Intent::ClearNotice);
    }
    match outcome {
        Outcome::NotACommand => {
            // The decision is `apply`'s (R10): it starts a turn or queues
            // the message; the composer clears when it has.
            if !line.trim().is_empty() {
                record(cx);
                cx.emit(&hs.submit, line.to_string());
            }
        }
        Outcome::Open(opened) => {
            record(cx);
            w.ui.composer.clear();
            open(cx, opened.target);
        }
        Outcome::Session { command, intent } => {
            record(cx);
            w.ui.composer.clear();
            screen(cx, chat::Intent::Session { command, intent });
        }
        Outcome::Insert(text) => w.ui.composer.reset(text),
        Outcome::Quit => cx.emit(&hs.quit, ()),
        // The line stays in the composer to be corrected.
        Outcome::Notice(notice) => screen(cx, chat::Intent::Notice(notice)),
    }
}

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState {
            seen: None,
            esc_primed: false,
        }
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

    fn update(&mut self, msg: ChatMsg, w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        if !matches!(msg, ChatMsg::Escape) {
            self.esc_primed = false;
        }
        match msg {
            ChatMsg::Submit(text) => {
                // The same Enter may have run it through a popup's pick.
                if !w.ui.composer.text().is_empty() {
                    run_line(&text, w, cx, &hs);
                }
            }
            ChatMsg::Picked(text) => {
                // A command picked in the popup runs; what `$` and `@` put in
                // is text and stays in the composer.
                // What runs is the composer's text as the pick left it: a
                // pick that raced with typing (the popup was one key behind)
                // then runs the line as it stands.
                if text.starts_with('/') {
                    let line = w.ui.composer.text();
                    run_line(&line, w, cx, &hs);
                }
            }
            ChatMsg::Quit => cx.emit(&hs.quit, ()),
            ChatMsg::DebugMenu => {
                drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(|_| DebugMenu.into())));
            }
            ChatMsg::Action(action) => match action {
                Action::SearchPrevious | Action::SearchNext => {
                    let draft = w.ui.composer.text().to_string();
                    let intent = popovers::Intent::SearchStart { draft };
                    cx.emit(&hs.screen, ScreenIntent::from(intent));
                }
                Action::ToggleSide => {
                    let notice = codex_tui_rtwk_core::commands::Notice::NotAvailable(
                        SlashCommand::Side,
                    );
                    cx.emit(&hs.screen, ScreenIntent::from(chat::Intent::Notice(notice)));
                }
                other => {
                    if let Some(target) = other.target() {
                        open(cx, target);
                    }
                }
            },
            ChatMsg::Escape => {
                // The chat as a page over another (a debug entry) goes back.
                if Navigator::maybe_pop(cx) {
                    return Rebuild::No;
                }
                if !w.ui.composer.text().is_empty() {
                    self.esc_primed = false;
                } else if self.esc_primed {
                    self.esc_primed = false;
                    open(cx, Target::Backtrack);
                } else {
                    self.esc_primed = true;
                }
            }
            ChatMsg::Search(key) => {
                let states = read::<ScreenStates, _>(cx);
                let Some(search) = &states.popovers.search else {
                    return Rebuild::No;
                };
                let intent = match key {
                    SearchKey::Char(text) => {
                        popovers::Intent::SearchEdit(format!("{}{text}", search.query))
                    }
                    SearchKey::Backspace => {
                        let mut query = search.query.clone();
                        query.pop();
                        popovers::Intent::SearchEdit(query)
                    }
                    SearchKey::Older => popovers::Intent::SearchStep(Direction::Older),
                    SearchKey::Newer => popovers::Intent::SearchStep(Direction::Newer),
                    SearchKey::Accept => {
                        if let Some(entry) = search.preview(&states.popovers.history) {
                            w.ui.composer.reset(entry);
                        }
                        popovers::Intent::SearchEnd
                    }
                    SearchKey::Cancel => popovers::Intent::SearchEnd,
                };
                cx.emit(&hs.screen, ScreenIntent::from(intent));
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
        // The session's failure first, then the thread's last one.
        let failure = match (&session.status, &thread.last_error) {
            (Session::Failed(failure), _) | (_, Some(failure)) => Some(failure),
            _ => None,
        };
        let status = match (failure, &session.status, thread.is_running()) {
            (Some(failure), _, _) => {
                Text::new(failure_text(strings, failure)).style(tokens.error())
            }
            (None, Session::Starting, _) => Text::new(strings.chat.starting).style(tokens.dim()),
            (None, _, true) => Text::new(strings.chat.working).style(tokens.accent()),
            (None, _, false) => Text::new(strings.chat.ready).style(tokens.dim()),
        };
        let chat_strings = chat_strings::of(cx);
        let states = watch::<ScreenStates, _>(cx);
        let search = states.popovers.search.clone();
        let (composer, footer): (AnyWidget, String) = match &search {
            Some(search) => {
                // Reverse history search: the preview stands where the text
                // is, the keys are the search's (`history_search.rs`).
                let shown = search
                    .preview(&states.popovers.history)
                    .unwrap_or(&search.draft)
                    .to_string();
                let status = match search.status {
                    SearchStatus::Idle => chat_strings::Search::Idle,
                    SearchStatus::Match => chat_strings::Search::Match,
                    SearchStatus::NoMatch => chat_strings::Search::NoMatch,
                };
                let line = Row::new()
                    .child(prompt(chat_strings.prompt))
                    .child(Expanded::new(1, Text::new(shown)));
                let field = Focus::new(line)
                    .can_request_focus(true)
                    .autofocus()
                    .on_key(cx.keys(search_key));
                (field.into(), chat_strings.search_footer(&search.query, status))
            }
            None => {
                let area = TextArea::new()
                    .controller(w.ui.composer.clone())
                    .placeholder(strings.chat.placeholder)
                    .min_lines(1)
                    .max_lines(6)
                    .autofocus(true)
                    .on_submitted(cx.map(ChatMsg::Submit));
                let data = states.clone();
                let table = chat_strings;
                let field = Autocomplete::new(
                    w.ui.composer.clone(),
                    move |before: &str| popup::suggestions(&data, table, before),
                    area,
                )
                .rows(popup::ROWS)
                .on_selected(cx.map(|picked: Suggestion| ChatMsg::Picked(Rc::from(&*picked.text))));
                let line = Row::new()
                    .child(prompt(chat_strings.prompt))
                    .child(Expanded::new(1, field));
                (line.into(), strings.chat.keys.to_string())
            }
        };
        let mut body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, transcript))
            .child(status);
        for text in &thread.queue {
            body = body.child(Text::new(strings.queued(text).to_string()).style(tokens.dim()));
        }
        if let Some(notice) = &states.chat.notice {
            let notice = match notice {
                chat::ChatNotice::Command(notice) => notice.clone(),
                chat::ChatNotice::NotAvailable(command) => {
                    codex_tui_rtwk_core::commands::Notice::NotAvailable(*command)
                }
            };
            body = body.child(Text::new(chat_strings.notice(&notice)).style(tokens.error()));
        }
        let body = body
            .child(composer)
            .child(Text::new(footer).style(tokens.dim()));
        let quit = KeyPress::char('c').with_mods(Mods::CTRL);
        let mut focus = Focus::new(body)
            .bind(quit, cx.send(ChatMsg::Quit))
            .bind(KeyPress::code(KeyCode::F12), cx.send(ChatMsg::DebugMenu))
            .bind(KeyPress::ESC, cx.send(ChatMsg::Escape));
        for (action, key) in keys::defaults() {
            focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
        }
        focus.into()
    }
}
