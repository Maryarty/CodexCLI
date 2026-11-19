//! The skeleton's chat screen: the transcript as plain text, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::rc::Rc;

use rtwk::editing::{Autocomplete, Suggestion, TextArea};
use rtwk::focus::{Focus, FocusNode, KeyBindings, KeyEventResult, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, Row};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select, watch};
use rtwk::scroll::ListView;
use rtwk::style::Theme;
use rtwk::layout::SizedBox;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::commands::{DispatchCx, Outcome, SlashCommand, Target, dispatch};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::screens::popovers::{Direction, SearchStatus};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, dialogs, popovers};
use codex_tui_core::threads::{Failure, Item, ThreadModel};

use crate::l10n::strings as l10n;
use crate::screens::chat::keys::{self, Action};
use crate::screens::chat::open::open;
use crate::screens::chat::popup;
use crate::screens::dialogs::shortcut_panel;
use crate::screens::chat::strings as chat_strings;
use crate::theme;
use crate::ui::page_keys;
use crate::ui::status_line::{Notice, status_line, with_warning_notice};
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
    /// The shortcut overlay is expanded above the composer (the oracle's
    /// `FooterMode::ShortcutOverlay`).
    shortcuts: bool,
    /// The composer's focus node: the page sees `?` and `←` before the field
    /// does (`FocusNode::on_key_event`, the primary focus gets a key first).
    node: FocusNode,
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
    /// `?`: the shortcut overlay (the footer's hint).
    Shortcuts,
    /// `←` on an empty composer: the agents page (the footer's hint).
    Agents,
    /// The composer's text changed (the field's own notification).
    Changed(Rc<str>),
}

/// What a key does while reverse history search is open.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchKey {
    Char(Rc<str>),
    Backspace,
    /// Ctrl+U: the query is emptied.
    Clear,
    Older,
    Newer,
    Accept,
    Cancel,
    /// Any other key: the search consumes it (`_ => (InputResult::None, true)`).
    Ignore,
}

/// The key of history search for `key`, if it is one
/// (`history_search.rs`, `handle_history_search_key`): Ctrl+R and Up step to
/// an older match, Ctrl+S and Down to a newer one, Esc and Ctrl+C cancel
/// (the draft is restored), Enter accepts a match, Backspace and Ctrl+H
/// delete, Ctrl+U clears, other text extends the query.
fn search_key(key: KeyPress) -> Option<ChatMsg> {
    let ctrl = |c: char| KeyPress::char(c).with_mods(Mods::CTRL);
    let found = if key == ctrl('r') || key.code == KeyCode::Up {
        SearchKey::Older
    } else if key == ctrl('s') || key.code == KeyCode::Down {
        SearchKey::Newer
    } else if key.code == KeyCode::Escape
        || key == ctrl('c')
        || (key.mods.is_empty() && key.text() == Some("\u{3}"))
    {
        SearchKey::Cancel
    } else if key.code == KeyCode::Enter && key.mods.is_empty() {
        SearchKey::Accept
    } else if key.code == KeyCode::Backspace || key == ctrl('h') {
        SearchKey::Backspace
    } else if key == ctrl('u') {
        SearchKey::Clear
    } else if let Some(text) = key.as_text() {
        SearchKey::Char(Rc::from(text))
    } else if key.code == KeyCode::F12 {
        // The port's own hidden debug key is not the oracle's to swallow.
        return None;
    } else {
        SearchKey::Ignore
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
/// F2 and `/warnings`: the viewer over the retained warnings (nothing
/// retained opens nothing).
fn open_warnings(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    if read::<ScreenStates, _>(cx).dialogs.retained.is_empty() {
        return;
    }
    cx.emit(&hs.screen, ScreenIntent::from(dialogs::Intent::OpenWarnings));
    open(cx, Target::Dialogs);
}

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
        // TODO(session-core): `review_mode` is the thread's code-review state
        // (`review.is_review_mode`); false until the session models review.
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
            if opened.command == SlashCommand::Warnings {
                open_warnings(cx, hs);
            } else {
                open(cx, opened.target);
            }
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
            shortcuts: false,
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Chat);

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn init_state(&mut self, w: &Chat, cx: &mut Cx<'_, Self>) {
        // `?` (the shortcut overlay, `footer_shortcuts_default`) and `←`
        // (the agents page, `handle_agents_navigation`) on an EMPTY composer
        // belong to the page; with text they go to the field.
        let send = cx.local_sender();
        let composer = w.ui.composer.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            if !composer.text().is_empty() || key.mods.contains(Mods::CTRL) || key.mods.contains(Mods::ALT) {
                return KeyEventResult::Ignored;
            }
            match (key.code, key.text()) {
                (KeyCode::Char, Some("?")) => send(ChatMsg::Shortcuts),
                (KeyCode::Left, _) => send(ChatMsg::Agents),
                _ => return KeyEventResult::Ignored,
            }
            KeyEventResult::Handled
        })));
    }

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
        let overlay = self.shortcuts;
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
            ChatMsg::Changed(text) => {
                if self.shortcuts && !text.is_empty() {
                    // Any other edit puts the overlay away (`FooterMode`).
                    self.shortcuts = false;
                }
            }
            ChatMsg::Shortcuts => self.shortcuts = !self.shortcuts,
            ChatMsg::Agents => {
                if w.ui.composer.text().is_empty() {
                    open(cx, Target::Agents);
                }
            }
            ChatMsg::DebugMenu => {
                drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(|_| DebugMenu.into())));
            }
            ChatMsg::Action(action) => match action {
                Action::SearchPrevious => {
                    let draft = w.ui.composer.text().to_string();
                    let intent = popovers::Intent::SearchStart { draft };
                    cx.emit(&hs.screen, ScreenIntent::from(intent));
                }
                // `history_search_next` steps inside a search only
                // (`search_key`); outside one it opens nothing.
                Action::SearchNext => {}
                Action::ToggleSide => {
                    let notice = codex_tui_core::commands::Notice::NotAvailable(
                        SlashCommand::Side,
                    );
                    cx.emit(&hs.screen, ScreenIntent::from(chat::Intent::Notice(notice)));
                }
                Action::OpenWarnings => open_warnings(cx, &hs),
                other => {
                    if let Some(target) = other.target() {
                        open(cx, target);
                    }
                }
            },
            ChatMsg::Escape => {
                if self.shortcuts {
                    self.shortcuts = false;
                    return Rebuild::Yes;
                }
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
                    SearchKey::Char(text) => popovers::Intent::SearchAppend(text.to_string()),
                    SearchKey::Backspace => popovers::Intent::SearchBackspace,
                    SearchKey::Clear => popovers::Intent::SearchClear,
                    SearchKey::Older => popovers::Intent::SearchStep(Direction::Older),
                    SearchKey::Newer => popovers::Intent::SearchStep(Direction::Newer),
                    SearchKey::Accept => {
                        if let Some(entry) = search.preview(&states.popovers.history) {
                            w.ui.composer.reset(entry);
                        }
                        // Core ends the search only on a match.
                        popovers::Intent::SearchAccept
                    }
                    SearchKey::Cancel => popovers::Intent::SearchEnd,
                    SearchKey::Ignore => return Rebuild::No,
                };
                cx.emit(&hs.screen, ScreenIntent::from(intent));
            }
        }
        if overlay == self.shortcuts {
            Rebuild::No
        } else {
            Rebuild::Yes
        }
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
        let (panel, close) = shortcut_panel(cx);
        let chat_strings = chat_strings::of(cx);
        let states = watch::<ScreenStates, _>(cx);
        let search = states.popovers.search.clone();
        let (composer, footer): (AnyWidget, AnyWidget) = match &search {
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
                (
                    field.into(),
                    Text::new(chat_strings.search_footer(&search.query, status))
                        .style(tokens.dim())
                        .into(),
                )
            }
            None => {
                let area = TextArea::new()
                    .controller(w.ui.composer.clone())
                    .placeholder(strings.chat.placeholder)
                    .min_lines(1)
                    .max_lines(6)
                    .autofocus(true)
                    .focus_node(self.node.clone())
                    .on_changed(cx.map(ChatMsg::Changed))
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
                // The oracle's idle footer (`footer_shortcuts_default`): the hint of
                // the bound `?`, in dim, two cells in.
                let bar = page_keys::footer_with(
                    [
                        (strings.pages.key_agents, strings.pages.hint_agents),
                        (strings.pages.key_shortcuts, strings.pages.hint_shortcuts),
                    ],
                    tokens.dim(),
                    tokens.dim(),
                    " · ",
                );
                let footer = if self.shortcuts {
                    // Open: the footer is the close hint (`close_hint`).
                    Row::new()
                        .child(SizedBox::new(Some(2), None, None))
                        .child(Text::new(close).style(tokens.dim()))
                        .into()
                } else {
                    Row::new()
                        .child(SizedBox::new(Some(2), None, None))
                        .child(Expanded::new(1, bar))
                        .into()
                };
                (line.into(), footer)
            }
        };
        // `⚠ N warnings · f2 to view` at the footer's right (not while history
        // search owns the footer).
        let footer = if search.is_none() {
            let data = Theme::of(cx);
            with_warning_notice(
                footer,
                states.dialogs.retained.len(),
                Notice {
                    badge: chat_strings.warning_badge,
                    singular: chat_strings.warning_singular,
                    plural: chat_strings.warning_plural,
                    key: chat_strings.warning_key,
                    view: chat_strings.warning_view,
                    accent: data.warning.clone(),
                    secondary: tokens.dim(),
                    key_style: tokens.bold(),
                },
            )
        } else {
            footer
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
                    codex_tui_core::commands::Notice::NotAvailable(*command)
                }
            };
            // `add_info_message` is plain text, `add_error_message` is red.
            let text = Text::new(chat_strings.notice(&notice));
            body = body.child(if notice.is_info() {
                text
            } else {
                text.style(tokens.error())
            });
        }
        if self.shortcuts && search.is_none() {
            body = body.child(panel);
        }
        // The configured status line, two cells in, under the composer.
        // The items and their order are the config's, the texts the facts the
        // host read (core `StatusFacts`); the ones that need the session
        // (context, limits) have no value yet and are not drawn.
        let facts = &states.chat.status;
        let line = status_line(facts.items(), facts.colors, &tokens);
        let mut body = body.child(composer);
        if let Some(line) = line {
            body = body.child(
                Row::new()
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Expanded::new(1, line)),
            );
        }
        let body = body
            .child(footer);
        let quit = KeyPress::char('c').with_mods(Mods::CTRL);
        let mut focus = Focus::new(body)
            .bind(quit, cx.send(ChatMsg::Quit))
            .bind(KeyPress::code(KeyCode::F12), cx.send(ChatMsg::DebugMenu))
            .bind(KeyPress::ESC, cx.send(ChatMsg::Escape));
        for (action, key) in keys::defaults() {
            focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
        }
        // The footer's entries are these bindings' hints: every key it shows is
        // bound here.
        let pages = strings.pages;
        KeyBindings::new(
            [
                ("left", cx.send(ChatMsg::Agents)),
                (pages.key_shortcuts, cx.send(ChatMsg::Shortcuts)),
            ],
            focus,
        )
        .hints([
            ("left", pages.hint_agents),
            (pages.key_shortcuts, pages.hint_shortcuts),
        ])
            .into()
    }
}
