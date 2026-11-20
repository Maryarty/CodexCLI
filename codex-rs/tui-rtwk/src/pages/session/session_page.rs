//! The session page: the transcript, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::binding::TerminalTitle;
use rtwk::controls::{Callout, Hyperlink, StatusKind};
use rtwk::editing::{Autocomplete, Suggestion, TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyBindings, KeyEventResult, KeyPress};
use rtwk::layout::{Alignment, BoxWidget, EdgeInsets, SizedBox, Stack};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, Row};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select, watch};
use rtwk::style::Theme;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{ConnectionState, RequestQueue, Session, SessionSlice};
use codex_tui_core::commands::{SlashCommand, Target};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::screens::popovers::{Direction, SearchStatus};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, popovers};
use codex_tui_core::threads::{Failure, ThreadModel, TurnState};

use crate::app::debug::menu::DebugMenu;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::keys::{self, Action};
use crate::pages::session::open::open;
use crate::pages::session::popup;
use crate::pages::session::strings as chat_strings;
use crate::sheets::dialogs::shortcut_panel;
use crate::widgets::page_keys;
use crate::widgets::status_line::{Notice, status_line, with_warning_notice};
use crate::widgets::thread_ui::ThreadUi;

use super::actions::*;
use super::activity::Activity;
use super::search_key::*;
use super::status_surfaces::StatusContext;
use super::transcript_pane::TranscriptPane;

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
    draft_empty: bool,
    /// The composer's focus node: the page sees `?` and `←` before the field
    /// does (`FocusNode::on_key_event`, the primary focus gets a key first).
    node: FocusNode,
    /// `Ctrl+X` was pressed: the next key completes the chord (the leader
    /// chords of `keymap/chords/leader_defaults.rs`).
    leader: Rc<Cell<bool>>,
    /// The backend control is visible, so its chord (`change_host`, `Ctrl+X
    /// b`) is live; read by the key handler.
    control: Rc<Cell<bool>>,
    /// The composer of the thread the chat shows now (the page outlives a
    /// switch of thread; lifecycle updates keep the key handler current).
    composer: Rc<RefCell<TextEditingController>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    /// `change_host`: the empty conversation moves to the next backend.
    Cycle,
    Submit(Rc<str>),
    QueueDraft,
    CtrlC,
    RetryWriter,
    ForkWriter,
    QuitReadOnly,
    CtrlD,
    DismissBanner(BackendId, Ticket),
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

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState {
            seen: None,
            esc_primed: false,
            shortcuts: false,
            draft_empty: self.ui.composer.text().is_empty(),
            node: FocusNode::new(),
            leader: Rc::new(Cell::new(false)),
            control: Rc::new(Cell::new(false)),
            composer: Rc::new(RefCell::new(self.ui.composer.clone())),
        }
    }
}

impl_widget!(stateful eq Chat);

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn init_state(&mut self, _: &Chat, cx: &mut Cx<'_, Self>) {
        // `?` (the shortcut overlay, `footer_shortcuts_default`) and `←`
        // (the agents page, `handle_agents_navigation`) on an EMPTY composer
        // belong to the page; with text they go to the field.
        let send = cx.local_sender();
        let composer = self.composer.clone();
        let (leader, control) = (self.leader.clone(), self.control.clone());
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            // `change_host` is the leader chord `Ctrl+X b`, live while the
            // backend control shows (`visible_new_backend_control`), and it
            // works with a draft in the composer.
            if control.get() {
                if leader.replace(false) {
                    if key.mods.is_empty() && key.text() == Some("b") {
                        send(ChatMsg::Cycle);
                        return KeyEventResult::Handled;
                    }
                } else if key == KeyPress::char('x').with_mods(Mods::CTRL) {
                    leader.set(true);
                    return KeyEventResult::Handled;
                }
            } else {
                leader.set(false);
            }
            if !composer.borrow().text().is_empty()
                || key.mods.contains(Mods::CTRL)
                || key.mods.contains(Mods::ALT)
            {
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

    fn did_update_widget(&mut self, old: &Chat, w: &Chat, _cx: &mut Cx<'_, Self>) {
        if old != w {
            *self.composer.borrow_mut() = w.ui.composer.clone();
            self.draft_empty = w.ui.composer.text().is_empty();
            self.esc_primed = false;
            self.leader.set(false);
        }
    }

    fn did_change_dependencies(&mut self, w: &Chat, cx: &mut Cx<'_, Self>) {
        // The composer clears when the model accepted (or queued) the
        // message, not when it was sent: a change of the last accepted
        // ticket in the same thread.
        let now = select::<ThreadModel, (ThreadId, Option<Ticket>), _>(cx, |thread| {
            (thread.id().clone(), thread.last_submit())
        });
        if let Some((thread, before)) = &self.seen
            && *thread == now.0
            && *before != now.1
            && now.1.is_some()
        {
            w.ui.composer.clear();
        }
        self.seen = Some(now);
        let states = watch::<ScreenStates, _>(cx);
        let session = watch::<SessionSlice, _>(cx);
        let thread = read::<ThreadModel, _>(cx);
        select::<ThreadModel, _, _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.turn().clone(),
                thread.can_interrupt(),
                thread.read_only(),
                thread.transcript().is_empty(),
                thread.queue().len(),
                thread.last_error().cloned(),
                thread.screen_slots().chat.clone(),
                thread.screen_slots().status.clone(),
            )
        });
        self.control.set(
            session.cycle_label(&thread, &states.agents).is_some()
                && states.popovers.search.is_none(),
        );
    }

    fn update(&mut self, msg: ChatMsg, w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let previous = (self.shortcuts, self.esc_primed, self.draft_empty);
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
            ChatMsg::QueueDraft => {
                let draft = w.ui.composer.text();
                if !draft.is_empty() {
                    run_line(&draft, w, cx, &hs);
                }
            }
            ChatMsg::Picked(text) => {
                // `Autocomplete::on_selected`: a command picked in the popup
                // runs as the pick names it (the command clears the composer
                // itself); what `$` and `@` put in is text and stays.
                if text.starts_with('/') {
                    run_line(&text, w, cx, &hs);
                }
            }
            ChatMsg::RetryWriter => {
                let thread = read::<ThreadModel, _>(cx);
                if thread.read_only() {
                    cx.emit(&hs.open_thread, thread.id().clone());
                }
            }
            ChatMsg::ForkWriter => {
                let thread = read::<ThreadModel, _>(cx);
                if thread.read_only() {
                    let intent =
                        codex_tui_core::screens::agents::Intent::Fork(thread.id().to_string());
                    cx.emit(&hs.screen, ScreenIntent::from(intent));
                }
            }
            ChatMsg::QuitReadOnly => cx.emit(&hs.quit, ()),
            ChatMsg::CtrlC => {
                let thread = read::<ThreadModel, _>(cx);
                if thread.can_interrupt() {
                    cx.emit(&hs.interrupt, thread.id().clone());
                } else {
                    cx.emit(&hs.quit, ());
                }
            }
            ChatMsg::CtrlD => {
                if w.ui.composer.text().is_empty() {
                    let thread = read::<ThreadModel, _>(cx);
                    if !thread.is_running() {
                        cx.emit(&hs.quit, ());
                    }
                }
            }
            ChatMsg::Changed(text) => {
                self.draft_empty = text.is_empty();
                if self.shortcuts && !text.is_empty() {
                    // Any other edit puts the overlay away (`FooterMode`).
                    self.shortcuts = false;
                }
            }
            ChatMsg::DismissBanner(backend, ticket) => {
                cx.emit(&hs.dismiss_banner, (backend, ticket))
            }
            ChatMsg::Cycle if !self.shortcuts => cx.emit(&hs.cycle, ()),
            ChatMsg::Cycle => {}
            ChatMsg::Shortcuts => self.shortcuts = !self.shortcuts,
            ChatMsg::Agents => {
                if w.ui.composer.text().is_empty() || read::<ThreadModel, _>(cx).read_only() {
                    open(cx, Target::Agents);
                }
            }
            ChatMsg::DebugMenu => {
                drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(|_| DebugMenu.into())));
            }
            ChatMsg::Action(action) => match action {
                Action::OpenTranscript if read::<ThreadModel, _>(cx).read_only() => {
                    super::external_writer::open_transcript(cx);
                }
                Action::SearchPrevious => {
                    let draft = w.ui.composer.text().to_string();
                    let intent = popovers::Intent::SearchStart { draft };
                    cx.emit(&hs.screen, ScreenIntent::from(intent));
                }
                // `history_search_next` steps inside a search only
                // (`search_key`); outside one it opens nothing.
                Action::SearchNext => {}
                Action::ToggleSide => {
                    let notice = codex_tui_core::commands::Notice::NotAvailable(SlashCommand::Side);
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
                let thread = read::<ThreadModel, _>(cx);
                if thread.can_interrupt() {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.interrupt, thread.id().clone());
                    return Rebuild::No;
                }
                if thread.read_only() {
                    open(cx, Target::Agents);
                    return Rebuild::No;
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
        if previous == (self.shortcuts, self.esc_primed, self.draft_empty) {
            Rebuild::No
        } else {
            Rebuild::Yes
        }
    }

    fn build(&self, w: &Chat, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let thread = read::<ThreadModel, _>(cx);
        let session = watch::<SessionSlice, _>(cx);
        let strings = l10n::of(cx);
        let states = watch::<ScreenStates, _>(cx);
        let backend = states.agents.owner_of(thread.id().as_str());
        let connection = session.connections.get(&backend);
        let slots = thread.screen_slots();
        let facts = &states.chat.status;
        let mut transcript = Stack::new()
            .alignment(Alignment::CENTER)
            .child(TranscriptPane(w.ui.transcript.clone()));
        if facts.animations
            && facts.empty_state
            && self.draft_empty
            && !self.shortcuts
            && states.popovers.search.is_none()
            && thread.transcript().is_empty()
            && thread.last_submit().is_none()
            && !thread.read_only()
        {
            transcript = transcript.child(super::blossom::welcome());
        }
        // The session's failure first, then the thread's last one.
        let screen_error = thread.screen_slots().chat.error.clone().map(Failure::from);
        let failure = match (
            &session.status,
            thread.last_error().or(screen_error.as_ref()),
        ) {
            (Session::Failed(failure), _) | (_, Some(failure)) => Some(failure),
            _ => None,
        };
        let running = match thread.turn() {
            TurnState::Running(running) => Some(running),
            TurnState::Idle => None,
        };
        let compacting = slots.chat.is_compacting();
        let status: Option<AnyWidget> = if thread.read_only() {
            None
        } else if connection == Some(&ConnectionState::Reconnecting) {
            Some(
                Text::new(strings.chat.reconnecting)
                    .style(tokens.dim())
                    .into(),
            )
        } else if let Some(failure) = failure {
            Some(
                Text::new(failure_text(strings, failure))
                    .style(tokens.error())
                    .into(),
            )
        } else if session.status == Session::Starting {
            Some(Text::new(strings.chat.starting).style(tokens.dim()).into())
        } else if running.is_some() || compacting {
            let offset_seconds = running
                .and_then(|running| running.started_at)
                .filter(|started| *started > 0)
                .map(|started| {
                    u64::try_from(states.agents.now.saturating_sub(started)).unwrap_or(0)
                })
                .unwrap_or(0);
            Some(
                AnyWidget::from(Activity {
                    show_interrupt: thread.can_interrupt(),
                    spinner: facts.animations && facts.progress_effect,
                    shimmer: facts.animations && facts.shimmer_effect,
                    header: if compacting {
                        strings.chat.compacting
                    } else if slots.chat.activity == chat::ActivityPhase::Thinking {
                        strings.chat.thinking
                    } else {
                        strings.chat.working
                    },
                    step: slots.chat.step.clone(),
                    offset_seconds,
                })
                .with_key(Key::value((
                    thread.id().clone(),
                    running.map(|running| running.turn.clone()),
                ))),
            )
        } else {
            None
        };
        let (panel, close) = shortcut_panel(cx);
        let chat_strings = chat_strings::of(cx);
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
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(prompt(chat_strings.prompt, tokens.bold()))
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
            None if thread.read_only() => (
                super::external_writer::notice(cx, chat_strings, &tokens),
                super::external_writer::footer(chat_strings, &tokens),
            ),
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
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(prompt(chat_strings.prompt, tokens.bold()))
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
                let hint = if self.esc_primed && !thread.transcript().is_empty() {
                    Some(strings.chat.backtrack_hint)
                } else if running.is_some() && !self.draft_empty {
                    Some(strings.chat.queue_hint)
                } else {
                    None
                };
                let footer = if self.shortcuts {
                    // Open: the footer is the close hint (`close_hint`).
                    Row::new()
                        .child(SizedBox::new(Some(2), None, None))
                        .child(Text::new(close).style(tokens.dim()))
                        .into()
                } else if let Some(hint) = hint {
                    Row::new()
                        .child(SizedBox::new(Some(2), None, None))
                        .child(Text::line(hint).style(tokens.dim()))
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
            .child(Expanded::new(1, transcript));
        if let Some(status) = status {
            if !thread.transcript().is_empty() {
                body = body.child(SizedBox::new(None, Some(1), None));
            }
            body = body.child(status);
        }
        const QUEUE_PREVIEW: usize = 3;
        for text in thread.queue().iter().take(QUEUE_PREVIEW) {
            body = body.child(
                Text::new(strings.queued(text).to_string())
                    .max_lines(Some(2))
                    .overflow(TextOverflow::Ellipsis)
                    .style(tokens.dim()),
            );
        }
        let remaining = thread.queue().len().saturating_sub(QUEUE_PREVIEW);
        if remaining > 0 {
            body = body.child(Text::line(strings.more_queued(remaining)).style(tokens.dim()));
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
        let run_state = if compacting {
            "Compacting"
        } else if running.is_some() {
            match slots.chat.activity {
                chat::ActivityPhase::Working => "Working",
                chat::ActivityPhase::Thinking => "Thinking",
            }
        } else if connection == Some(&ConnectionState::Reconnecting) {
            "Reconnecting"
        } else if failure.is_some() {
            "Error"
        } else {
            "Ready"
        };
        let context = StatusContext {
            thread_id: session.active.as_ref(),
            thread: Some(&slots.chat),
            usage: Some(&slots.status),
            rate_limits: session.rate_limits.get(&backend),
            run_state: Some(run_state),
            strings,
        };
        let line = status_line(context.items_for(facts), facts.colors, &tokens);
        // The backend control above the composer (`rendering.rs`,
        // `visible_new_backend_control`): the displayed backend, and the key.
        let control = session
            .cycle_label(&thread, &states.agents)
            .filter(|_| search.is_none() && !self.shortcuts);
        let mut body = body;
        if let Some(label) = control {
            body = body.child(
                Row::new()
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Text::new(chat_strings.backend(label)))
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Text::new(chat_strings.backend_key))
                    .child(Text::new(chat_strings.backend_cycle).style(tokens.dim())),
            );
        }
        let active_model = slots.chat.model.as_deref().or(facts.model.as_deref());
        if let Some(occurrence) = session
            .backend_banners
            .get(&backend)
            .filter(|occurrence| occurrence.is_visible(active_model))
        {
            let banner = &occurrence.banner;
            let (title, description) = if banner.reserve_pending(active_model) {
                (
                    strings.chat.usage_limit_title,
                    strings.chat.usage_limit_detail,
                )
            } else {
                (banner.title(), banner.description())
            };
            let mut callout = Callout::text(description.to_owned())
                .title(title.to_owned())
                .kind(StatusKind::Warning)
                .actions(
                    banner
                        .links()
                        .map(|(label, url)| Hyperlink::new(url).text(label.to_owned()).into()),
                );
            if occurrence.is_dismissible() {
                let ticket = occurrence.ticket;
                callout = callout.on_dismissed(cx.send(ChatMsg::DismissBanner(backend, ticket)));
            }
            body = body.child(AnyWidget::from(callout).with_key(Key::value(occurrence.ticket)));
        }
        if let Some(notice) = session.backend_notices.get(&backend) {
            body = body.child(Text::new(notice.clone()).style(tokens.dim()));
        }
        let mut body = if thread.read_only() {
            body.child(composer)
        } else {
            body.child(
                BoxWidget::new()
                    .style(tokens.prompt_surface(cx, false))
                    .padding(EdgeInsets::only(0, 1, 1, 1))
                    .child(composer),
            )
        };
        if let Some(line) = line.filter(|_| !thread.read_only()) {
            body = body.child(
                Row::new()
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Expanded::new(1, line)),
            );
        }
        let mut body = body.child(footer);
        let action_required =
            select::<RequestQueue, bool, _>(cx, |queue| !queue.entries.is_empty());
        let action = action_required.then_some(strings.chat.action_required);
        if action.is_none()
            && facts.animations
            && facts.progress_effect
            && (running.is_some() || compacting)
            && StatusContext::title_has_activity(facts)
        {
            let mut parts = context.title_parts(facts, Some(""));
            if let Some(index) = parts.iter().position(String::is_empty) {
                let after = parts.split_off(index + 1);
                parts.pop();
                body = body.child(super::window_title::WindowTitle {
                    before: parts,
                    after,
                });
            }
        } else {
            let parts = context.title_parts(facts, action);
            if !parts.is_empty()
                || facts
                    .terminal_title
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
            {
                body = body.child(TerminalTitle::new(
                    parts.join(" · "),
                    super::window_title::empty(),
                ));
            }
        }
        let mut focus = Focus::new(body)
            .bind(
                KeyPress::char('c').with_mods(Mods::CTRL),
                cx.send(ChatMsg::CtrlC),
            )
            .bind(
                KeyPress::char('d').with_mods(Mods::CTRL),
                cx.send(ChatMsg::CtrlD),
            )
            .bind(KeyPress::code(KeyCode::F12), cx.send(ChatMsg::DebugMenu))
            .bind(KeyPress::ESC, cx.send(ChatMsg::Escape));
        if running.is_some() && !self.draft_empty && search.is_none() {
            focus = focus.bind(KeyPress::code(KeyCode::Tab), cx.send(ChatMsg::QueueDraft));
        }
        if thread.read_only() {
            focus = focus
                .can_request_focus(true)
                .autofocus()
                .bind(KeyPress::char('r'), cx.send(ChatMsg::RetryWriter))
                .bind(KeyPress::char('f'), cx.send(ChatMsg::ForkWriter))
                .bind(KeyPress::char('q'), cx.send(ChatMsg::QuitReadOnly));
        }
        for (action, key) in keys::defaults() {
            focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
        }
        // The footer's entries are these bindings' hints: every key it shows is
        // bound here.
        let pages = strings.pages;
        let focus = AnyWidget::from(focus).with_key(Key::value(thread.read_only()));
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
