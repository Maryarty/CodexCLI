//! The session page: the transcript, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::binding::TerminalTitle;
use rtwk::editing::TextEditingController;
use rtwk::focus::{Focus, FocusNode, KeyBindings, KeyEventResult, KeyPress};
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Alignment, BoxWidget, EdgeInsets, SizedBox, Stack};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select, watch};
use rtwk::style::Theme;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{ConnectionState, RequestQueue, SessionSlice};
use codex_tui_core::commands::{SlashCommand, Target};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::screens::backtrack::{EscStep, Targets};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, ScreenThreadIntent, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::debug::menu::DebugMenu;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::composer::footer::with_shell_mode;
use crate::pages::session::composer::footer::{FooterLine, footer};
use crate::pages::session::composer::{Composer, clear_sent, submission};
use crate::pages::session::keys::{self, Action};
use crate::pages::session::open::open;
use crate::pages::session::strings as chat_strings;
use crate::sheets::dialogs::shortcut_panel;
use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::status_line::{Notice, status_line, with_warning_notice};
use crate::widgets::thread_ui::{Modes, ThreadUi};

use super::actions::*;
use super::activity::{Run, StatusSlot};
use super::banners::{BackendBanners, BannerGate, NoticeLine, ReserveSelector, reserve_of};
use super::live::browse::{self, Browse};
use super::live::put_back;
use super::pending_input::PendingInput;
use super::status_surfaces::StatusContext;
use super::transcript;
use super::transcript_pane::Pane;

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
    /// The last restore (`Restore::seq`) the composer took from the model: a
    /// number is applied once, in the thread that made it.
    restored: Option<(ThreadId, Option<u64>)>,
    /// The shortcut overlay is expanded above the composer (the oracle's
    /// `FooterMode::ShortcutOverlay`).
    shortcuts: bool,
    draft_empty: bool,
    /// The revision of the composer's reverse history search (Ctrl+R) this
    /// page drew: its footer stands in the key footer's place, and a change of
    /// the search draws the page again.
    search_rev: u64,
    /// The composer is in shell mode: the footer label, no submit by the queue key.
    draft_shell: bool,
    /// The composer's focus node: the page sees `?` and `←` before the field
    /// does (`FocusNode::on_key_event`, the primary focus gets a key first).
    node: FocusNode,
    /// `Ctrl+X` was pressed: the next key completes the chord (the leader
    /// chords of `keymap/chords/leader_defaults.rs`).
    leader: Rc<Cell<bool>>,
    /// The backend control is visible, so its chord (`change_host`, `Ctrl+X
    /// b`) is live; read by the key handler.
    control: Rc<Cell<bool>>,
    /// The backend banner and the page: what it shows, and its rows' cursor.
    banner: BannerGate,
    /// A turn runs and the shortcut overlay is open: the banner's keys wait
    /// (`handle_inline_banner_key`); written by the build, read by the key
    /// handler.
    banner_blocked: Rc<Cell<BannerBlocked>>,
    /// The composer of the thread the chat shows now (the page outlives a
    /// switch of thread; lifecycle updates keep the key handler current).
    composer: Rc<RefCell<TextEditingController>>,
    /// Its draft: the two shortcuts wait while keys are held as a paste.
    draft: Rc<RefCell<ComposerDraft>>,
    /// The modes of its transcript: browsing the prompts takes keys before
    /// the field does.
    modes: Rc<RefCell<Modes>>,
}

/// What keeps the banner's keys from the banner (`handle_inline_banner_key`).
#[derive(Clone, Copy, Debug, Default)]
struct BannerBlocked {
    /// A turn runs: the number keys and Esc are the turn's.
    running: bool,
    /// The shortcut overlay is open: Esc closes it.
    overlay: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    /// A row of the banner of `Ticket` was chosen (its number key).
    BannerPick(BackendId, Ticket, usize),
    /// Esc dismissed the banner of `Ticket`.
    BannerDismiss(BackendId, Ticket),
    /// `change_host`: the empty conversation moves to the next backend.
    Cycle,
    Submit(Rc<str>),
    QueueDraft,
    /// The edit-queued key: the newest queued message goes back to the composer.
    EditQueued,
    CtrlC,
    RetryWriter,
    ForkWriter,
    QuitReadOnly,
    CtrlD,
    /// The hidden key: the debug menu over the chat.
    DebugMenu,
    /// A key action of the chat (`keys::defaults`).
    Action(Action),
    /// PageUp (-1) and PageDown (1): the transcript a page.
    Page(i8),
    /// A key of the mode that browses the prompts.
    Browse(Browse),
    /// A suggestion was put in: a command is run, the others are text.
    Picked(Rc<str>),
    Escape,
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
            restored: None,
            shortcuts: false,
            draft_empty: self.ui.composer.text().is_empty(),
            search_rev: self.ui.draft.search_rev(),
            draft_shell: self.ui.draft.shell(),
            node: FocusNode::new(),
            leader: Rc::new(Cell::new(false)),
            control: Rc::new(Cell::new(false)),
            banner: BannerGate::new(),
            banner_blocked: Rc::new(Cell::new(BannerBlocked::default())),
            composer: Rc::new(RefCell::new(self.ui.composer.clone())),
            draft: Rc::new(RefCell::new(self.ui.draft.clone())),
            modes: Rc::new(RefCell::new(self.ui.modes.clone())),
        }
    }
}

impl_widget!(stateful eq Chat);

impl ChatState {
    /// What the page keeps of the draft (the footer and the Tab binding
    /// read it): empty (shell mode's `!` is content), in shell mode, in
    /// history search.
    fn sync_draft(&mut self, w: &Chat) {
        self.search_rev = w.ui.draft.search_rev();
        self.draft_shell = w.ui.draft.shell();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
    }
}

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn init_state(&mut self, _: &Chat, cx: &mut Cx<'_, Self>) {
        // `?` (the shortcut overlay, `footer_shortcuts_default`) and `←`
        // (the agents page, `handle_agents_navigation`) on an EMPTY composer
        // belong to the page; with text they go to the field.
        let send = cx.local_sender();
        let composer = self.composer.clone();
        let draft = self.draft.clone();
        let modes = self.modes.clone();
        let (leader, control) = (self.leader.clone(), self.control.clone());
        let (banner, blocked) = (self.banner.clone(), self.banner_blocked.clone());
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            // browsing the prompts: its keys come first, any other key ends it
            // and goes on to the field
            if modes.borrow().browsing() {
                match browse::key(&key) {
                    Some(step) => {
                        send(ChatMsg::Browse(step));
                        return KeyEventResult::Handled;
                    }
                    None => send(ChatMsg::Browse(Browse::Leave)),
                }
            }
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
            if let Some(pages) = transcript::follow::pager_key(&key, &composer.borrow()) {
                send(ChatMsg::Page(pages));
                return KeyEventResult::Handled;
            }
            // The banner's numbers and Esc: only on an empty composer, off a
            // paste, with no turn running and no modifier
            // (`handle_inline_banner_key`; a popup needs text in the field).
            if composer.borrow().text().is_empty()
                && !draft.borrow().in_burst()
                && key.mods.is_empty()
                && let Some(shown) = banner.keys.get().backend
                && !blocked.get().running
            {
                let keys = banner.keys.get();
                match (key.code, key.text()) {
                    (KeyCode::Escape, _) if keys.dismissible && !blocked.get().overlay => {
                        send(ChatMsg::BannerDismiss(shown.0, shown.1));
                        return KeyEventResult::Handled;
                    }
                    (KeyCode::Char, Some(digit)) => {
                        if let Some(row) = digit
                            .parse::<usize>()
                            .ok()
                            .filter(|n| (1..=keys.actions).contains(n))
                        {
                            banner.options.set(row - 1);
                            send(ChatMsg::BannerPick(shown.0, shown.1, row - 1));
                            return KeyEventResult::Handled;
                        }
                    }
                    _ => {}
                }
            }
            // keys held as a paste are the paste's, not the shortcuts'
            // (`is_in_paste_burst`)
            if !composer.borrow().text().is_empty()
                || draft.borrow().in_burst()
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
            *self.draft.borrow_mut() = w.ui.draft.clone();
            *self.modes.borrow_mut() = w.ui.modes.clone();
            self.sync_draft(w);
            // the thread under the page changed: what it had open closes
            old.ui.modes.close();
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
            clear_sent(&w.ui);
            self.sync_draft(w);
        }
        self.seen = Some(now);
        // Messages the model gave back (an interrupt, a failed steer, the
        // edit-queued key) go into the composer, once each.
        let restore = select::<ThreadModel, (ThreadId, Option<u64>), _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.restore().map(codex_tui_core::threads::Restore::seq),
            )
        });
        if let Some((thread, before)) = &self.restored
            && *thread == restore.0
            && *before != restore.1
            && restore.1.is_some()
        {
            let model = read::<ThreadModel, _>(cx);
            if let Some(restore) = model.restore() {
                let table = crate::pages::session::composer::strings::of(cx);
                let draft = w.ui.draft.submission(&w.ui.composer, |n| table.image(n));
                put_back(&w.ui, &restore.onto(draft), |n| table.image(n));
                self.sync_draft(w);
            }
        }
        self.restored = Some(restore);
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
                thread.pending_steers().len(),
                thread.rejected_steers().len(),
                thread.last_error().cloned(),
                thread.screen_slots().chat.clone(),
                thread.screen_slots().status.clone(),
            )
        });
        self.control
            .set(session.cycle_label(&thread, &states.agents).is_some() && !w.ui.draft.searching());
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.banner.options.dispose();
    }

    fn update(&mut self, msg: ChatMsg, w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let modes = &w.ui.modes;
        let previous = (
            self.shortcuts,
            modes.primed(),
            self.draft_empty,
            self.draft_shell,
            self.search_rev,
            modes.detailed(),
        );
        // any other input between the two Escs ends the pair
        if !matches!(msg, ChatMsg::Escape) {
            modes.interrupt();
        }
        match msg {
            ChatMsg::BannerPick(backend, ticket, row) => {
                cx.emit(&hs.pick_banner, (backend, ticket, row));
                return Rebuild::No;
            }
            ChatMsg::BannerDismiss(backend, ticket) => {
                cx.emit(&hs.dismiss_banner, (backend, ticket));
                return Rebuild::No;
            }
            ChatMsg::Submit(text) => {
                // The same Enter may have run it through a popup's pick.
                if !w.ui.composer.text().is_empty() {
                    run_line(&text, w, cx, &hs, Delivery::Submit);
                    // a command that took the line cleared the composer
                    self.sync_draft(w);
                } else if !previous.1 {
                    // an Enter on the empty composer is the reader's way
                    // back (`empty_enter_returns_to_latest`)
                    w.ui.follow.jump_to_latest();
                }
            }
            ChatMsg::Page(pages) => w.ui.follow.page(i64::from(pages)),
            ChatMsg::Browse(step) => {
                let thread = read::<ThreadModel, _>(cx);
                let targets = Targets::of(thread.transcript(), thread.can_load_history());
                browse::take(&step, modes, &w.ui.follow, targets.count);
                // the prompt chosen: the transcript is the chat's again; the
                // edit itself is the thread's intent (`ThreadIntent::EditPrompt`)
                if step == Browse::Confirm
                    && modes.confirm(Some(thread.id()), targets.count).is_some()
                {
                    modes.close();
                }
            }
            ChatMsg::QueueDraft => {
                let draft = submission(&w.ui);
                if !draft.is_empty() {
                    // Tab queues behind a running turn and submits otherwise.
                    let running = read::<ThreadModel, _>(cx).is_running();
                    let send = if running {
                        Delivery::Queue
                    } else {
                        Delivery::Submit
                    };
                    run_line(&draft, w, cx, &hs, send);
                    self.sync_draft(w);
                }
            }
            ChatMsg::EditQueued => {
                // `has_queued_follow_up_messages`: something waits behind the turn.
                let thread = read::<ThreadModel, _>(cx);
                if !thread.queue().is_empty() || !thread.rejected_steers().is_empty() {
                    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::EditQueued);
                    cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                }
            }
            ChatMsg::Picked(text) => {
                // `Autocomplete::on_selected`: a command picked in the popup
                // runs as the pick names it (the command clears the composer
                // itself); what `$` and `@` put in is text and stays.
                if text.starts_with('/') {
                    run_line(&text, w, cx, &hs, Delivery::Submit);
                    self.sync_draft(w);
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
                self.sync_draft(w);
                // a paste or an edit while prompts are browsed ends browsing
                modes.leave_browsing();
                if self.shortcuts && !text.is_empty() {
                    // Any other edit puts the overlay away (`FooterMode`).
                    self.shortcuts = false;
                }
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
                Action::OpenTranscript => modes.toggle_detailed(),
                // the find prompt is the sheet's (steps 4-7 of the transcript);
                // F3 opens the sheet it will search
                Action::FindTranscript => modes.open(),
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
                // the detailed transcript with an empty composer: Esc begins
                // browsing the prompts, and with none to browse the transcript
                // closes and the chat says so (`begin_overlay_backtrack_preview`)
                if modes.detailed() && w.ui.composer.text().is_empty() {
                    let targets = Targets::of(thread.transcript(), thread.can_load_history());
                    if modes.esc(Some(thread.id()), targets) == EscStep::NoPrevious {
                        modes.close();
                        let intent =
                            ScreenThreadIntent::Chat(chat::ThreadIntent::NoPreviousMessage);
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                    return Rebuild::Yes;
                }
                // Esc is the reader's way back before it is the chat's
                // (`handle_scroll_key`); on an empty composer it also primes
                // the backtrack, below.
                if !thread.read_only() && w.ui.follow.away() {
                    w.ui.follow.jump_to_latest();
                    if !w.ui.composer.text().is_empty() || thread.can_interrupt() {
                        return Rebuild::No;
                    }
                }
                if thread.can_interrupt() {
                    let hs = read::<Hs, _>(cx);
                    // Esc with steers waiting interrupts and sends them as one
                    // message (`submit_pending_steers_after_interrupt`).
                    // In review mode a waiting rejected steer takes the same
                    // path: the core answers with the review warning.
                    let waiting = !thread.pending_steers().is_empty()
                        || (thread.screen_slots().chat.review_mode()
                            && !thread.rejected_steers().is_empty());
                    if !waiting {
                        cx.emit(&hs.interrupt, thread.id().clone());
                    } else {
                        let intent =
                            ScreenThreadIntent::Chat(chat::ThreadIntent::InterruptForSteers);
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
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
                // the first Esc primes, the second opens the detailed transcript
                // with the newest prompt marked (`handle_backtrack_esc_key`); while
                // an edit is under way no other starts
                if !thread.screen_slots().backtrack.editing() {
                    let targets = Targets::of(thread.transcript(), thread.can_load_history());
                    let empty = w.ui.composer.text().is_empty();
                    if modes.chat_esc(Some(thread.id()), empty, targets) == EscStep::NoPrevious {
                        let intent =
                            ScreenThreadIntent::Chat(chat::ThreadIntent::NoPreviousMessage);
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                }
            }
        }
        if previous
            == (
                self.shortcuts,
                modes.primed(),
                self.draft_empty,
                self.draft_shell,
                self.search_rev,
                modes.detailed(),
            )
        {
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
        // Reverse history search: the footer's row and what it takes from the page
        let searching = w.ui.draft.search_state();
        // Ctrl+T shows the details in the chat's own transcript, the composer
        // below keeps the keys; browsing the prompts marks one in it. The
        // modes tick when either moves (Flutter's `ListenableBuilder`).
        let reader = {
            let (follow, modes) = (w.ui.follow.clone(), w.ui.modes.clone());
            ListenableBuilder::new(w.ui.modes.listenable(), move |_, _| {
                Pane {
                    follow: follow.clone(),
                    detailed: modes.detailed(),
                    mark: modes.selected(),
                }
                .into()
            })
        };
        let mut transcript = Stack::new().alignment(Alignment::CENTER).child(reader);
        if facts.animations
            && facts.empty_state
            && self.draft_empty
            && !self.shortcuts
            && searching.is_none()
            && thread.transcript().is_empty()
            && thread.last_submit().is_none()
            && !thread.read_only()
        {
            transcript = transcript.child(super::blossom::welcome());
        }
        let run = Run::of(&thread);
        let reconnecting = connection == Some(&ConnectionState::Reconnecting);
        let running = run.running();
        let compacting = run.compacting();
        let (panel, close) = shortcut_panel(cx);
        let chat_strings = chat_strings::of(cx);

        let (composer, footer): (AnyWidget, AnyWidget) = if thread.read_only() {
            (
                super::external_writer::notice(cx, chat_strings, &tokens),
                super::external_writer::footer(chat_strings, &tokens),
            )
        } else {
            let composer = Composer::new(w.ui.clone(), self.node.clone())
                .on_changed(cx.map(ChatMsg::Changed))
                .on_submitted(cx.map(ChatMsg::Submit))
                .on_picked(cx.map(ChatMsg::Picked));
            let footer = if let Some((query, status)) = searching.clone() {
                // Reverse history search: its row is the footer
                // (`history_search_footer_line`).
                super::composer::search::footer(cx, query, status)
            } else {
                let line = if self.shortcuts {
                    // Open: the footer is the close hint (`close_hint`).
                    FooterLine::Close(close)
                } else if w.ui.modes.primed()
                    && Targets::of(thread.transcript(), thread.can_load_history()).count > 0
                {
                    FooterLine::Hint(strings.chat.backtrack_hint)
                } else if running && !self.draft_empty {
                    FooterLine::Hint(strings.chat.queue_hint)
                } else {
                    FooterLine::Keys
                };
                let footer = footer(cx, line);
                let footer = if line == FooterLine::Keys {
                    transcript::follow::footer(
                        cx,
                        w.ui.follow.clone(),
                        self.draft_empty && !w.ui.modes.primed(),
                        footer,
                    )
                } else {
                    footer
                };
                if self.draft_shell {
                    with_shell_mode(cx, footer)
                } else {
                    footer
                }
            };
            (composer.into(), footer)
        };
        // `⚠ N warnings · f2 to view` at the footer's right (not while history
        // search owns the footer).
        let footer = if searching.is_none() {
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
        // The status row, the queued messages and the notices are widgets of
        // their own (`activity`, `pending_input`, `banners`): each reads what
        // it shows from the providers.
        // Whether `StatusSlot` shows a row: the reader's gap then sets itself
        // apart from it with a blank row.
        let status_above = !run.read_only
            && (run.word(&session, reconnecting) != "Ready"
                || matches!(session.status, codex_tui_core::app::Session::Starting));
        // The bottom pane (`bottom_pane_renderable`): everything below the
        // transcript, which gets the rows left; the reader's gap above the
        // composer is a flexible row, the first thing a short window clips.
        let mut body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .child(StatusSlot)
            .child(PendingInput)
            .child(NoticeLine);
        if self.shortcuts && searching.is_none() {
            body = body.child(panel);
        }
        let context = StatusContext {
            thread_id: session.active.as_ref(),
            thread: Some(&slots.chat),
            usage: Some(&slots.status),
            rate_limits: session.rate_limits.get(&backend),
            run_state: Some(run.word(&session, reconnecting)),
            strings,
        };
        let line = status_line(context.items_for(facts), facts.colors, &tokens);
        // The backend control above the composer (`rendering.rs`,
        // `visible_new_backend_control`): the displayed backend, and the key.
        let control = session
            .cycle_label(&thread, &states.agents)
            .filter(|_| searching.is_none() && !self.shortcuts);
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
        self.banner_blocked.set(BannerBlocked {
            running,
            overlay: self.shortcuts,
        });
        let body = body.child(BackendBanners {
            gate: self.banner.clone(),
        });
        let mut body = if thread.read_only() {
            body.child(composer)
        } else {
            // the row the reader's control takes (`composer_gap`)
            body.child(transcript::follow::gap(
                cx,
                w.ui.follow.clone(),
                status_above,
            ))
            .child(
                // keyed: the blank rows and the status come and go above it
                // and the footer's type changes below, and an unkeyed child
                // between two changed neighbours is a new element (its state,
                // and the messages queued for it, lost)
                AnyWidget::from(
                    BoxWidget::new()
                        .style(tokens.prompt_surface(cx, false))
                        .padding(EdgeInsets::only(0, 1, 1, 1))
                        .child(composer),
                )
                .with_key(Key::value("composer")),
            )
        };
        if let Some(line) = line.filter(|_| !thread.read_only()) {
            body = body.child(
                Row::new()
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Expanded::new(1, line)),
            );
        }
        let body = body.child(footer);
        // the terminal title keeps being told while the selector takes the bottom pane
        let mut titles: Vec<AnyWidget> = Vec::new();
        let action_required =
            select::<RequestQueue, bool, _>(cx, |queue| !queue.entries.is_empty());
        let action = action_required.then_some(strings.chat.action_required);
        if action.is_none()
            && facts.animations
            && facts.progress_effect
            && (running || compacting)
            && StatusContext::title_has_activity(facts)
        {
            let mut parts = context.title_parts(facts, Some(""));
            if let Some(index) = parts.iter().position(String::is_empty) {
                let after = parts.split_off(index + 1);
                parts.pop();
                titles.push(
                    super::window_title::WindowTitle {
                        before: parts,
                        after,
                    }
                    .into(),
                );
            }
        } else {
            let parts = context.title_parts(facts, action);
            if !parts.is_empty()
                || facts
                    .terminal_title
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
            {
                titles.push(
                    TerminalTitle::new(parts.join(" · "), super::window_title::empty()).into(),
                );
            }
        }
        // The Luna Reserve selector owns the bottom pane (`show_selection_view`): no composer,
        // no footer; the draft waits in `UiApp.ui`.
        let mut body = if reserve_of(&session, &states, &thread).is_some() {
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .main_axis_size(MainAxisSize::Min)
                .child(ReserveSelector)
        } else {
            body
        };
        for title in titles {
            body = body.child(title);
        }
        let body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, transcript))
            .child(body);
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
            .bind(KeyPress::ESC, cx.send(ChatMsg::Escape))
            .bind(KeyPress::code(KeyCode::PageUp), cx.send(ChatMsg::Page(-1)))
            .bind(KeyPress::code(KeyCode::PageDown), cx.send(ChatMsg::Page(1)));
        // Tab queues while a turn runs and submits otherwise, but not a
        // shell command (`queue_keys` in `handle_key_event_without_popup`).
        if (running || !self.draft_shell) && !self.draft_empty && searching.is_none() {
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
        let keymap = crate::keymap::keymap(cx);
        let mut keys = vec![
            ("left".to_string(), cx.send(ChatMsg::Agents)),
            (pages.key_shortcuts.to_string(), cx.send(ChatMsg::Shortcuts)),
        ];
        keys.extend(crate::keymap::bindings(
            &keymap,
            codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
            &cx.send(ChatMsg::EditQueued),
        ));
        KeyBindings::new(crate::keymap::first_of_each(keys), focus)
            .hints([
                ("left", pages.hint_agents),
                (pages.key_shortcuts, pages.hint_shortcuts),
            ])
            .into()
    }
}
