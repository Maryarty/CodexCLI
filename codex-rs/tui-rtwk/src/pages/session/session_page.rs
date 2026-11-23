//! The session page: the transcript, a status line
//! and the composer. The real chat (cells, popovers, backtrack) grows in the
//! `chat`, `cells`, `composer` and `transcript` slots.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::binding::TerminalTitle;
use rtwk::editing::TextEditingController;
use rtwk::focus::{Focus, FocusNode, KeyBindings, KeyCombo, KeyEventResult, KeyPress};
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Alignment, BoxWidget, EdgeInsets, SizedBox, Stack, StackFit};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::Mods;
use rtwk::provider::{read, select, watch};
use rtwk::style::{Style, Theme};
use rtwk::widgets::Text;
use rtwk::{
    AnyWidget, Cx, Handler, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget,
};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::app::{ConnectionState, RequestQueue, SessionSlice};
use codex_tui_core::commands::{SlashCommand, Target};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::backtrack::{EscStep, Targets};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Note, ThreadModel};

use crate::app::debug::menu::DebugMenu;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::composer::{Composer, clear_sent, submission};
use crate::pages::session::footer::{Footer, Line, Sparkle, StatusRows};
use codex_tui_core::keymap::actions::GLOBAL_CHANGE_HOST;
use codex_tui_core::keymap::{Binding, Keymap, fixed};

use crate::keymap::{combo_of, pressed_fixed};
use crate::pages::session::keys::{self, Action};
use crate::pages::session::open::open;
use crate::pages::session::popovers::mention;
use crate::pages::session::strings as chat_strings;
use crate::sheets::dialogs::shortcut_panel;
use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::directives::{Edge, Listener};
use crate::widgets::status_line::{Notice, status_spans, with_warning_notice};
use crate::widgets::thread_ui::{Modes, TextSelection, ThreadUi};

use super::actions::*;
use super::activity::{Run, StatusSlot};
use super::backtrack::{self, EditWatch};
use super::banners::{
    BackendBanners, BannerGate, BannerId, NoticeLine, ReserveSelector, reserve_of,
};
use super::live::browse::{self, Browse};
use super::live::plan_mode;
use super::live::provider_error;
use super::live::{put_back, seed_history};

use super::slice;
use super::status_surfaces::{StatusContext, TitleInputs};
use super::transcript;
use super::transcript::copy::CopyJob;
use super::transcript::selection::Key as SelectionKey;
use super::transcript_pane::Pane;

/// The window height under which the chat draws nothing.
const HIDDEN_BELOW_ROWS: i32 = 4;

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

/// What the page keeps beside its widgets (ids and flags only, R14): the model's one-shots reach it
/// as [`Edge`]s from the listener in its tree (`widgets/directives.rs`).
pub struct ChatState {
    /// The handler the listener tells the page's edges on, made once so the listener's widget
    /// stays equal across the page's rebuilds.
    edges: Option<Handler<Edge>>,
    /// The read-only thread this page asked to fork and heard no answer of
    /// (the oracle's `fork_in_progress`): its notice gives way to "Forking
    /// conversation…" and the keys wait. A failed fork ends it.
    forking: Option<Forking>,
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
    /// The footer's modal-editing label this page drew (none with it off).
    draft_vim: Option<&'static str>,
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
    /// The keymap the build last saw, for the key handler (it reads the keys of the actions it
    /// serves from it); written by the build.
    keymap: Rc<RefCell<Rc<Keymap>>>,
    /// The composer of the thread the chat shows now (the page outlives a
    /// switch of thread; lifecycle updates keep the key handler current).
    composer: Rc<RefCell<TextEditingController>>,
    /// Its draft: the two shortcuts wait while keys are held as a paste.
    draft: Rc<RefCell<ComposerDraft>>,
    /// The modes of its transcript: browsing the prompts takes keys before
    /// the field does.
    modes: Rc<RefCell<Modes>>,
    /// What the reader selected in its transcript: the copy keys and Esc are its while
    /// something is selected.
    selection: Rc<RefCell<TextSelection>>,
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
    /// A row of the banner shown was chosen (its number key).
    BannerPick(BannerId, usize),
    /// Esc dismissed the banner shown.
    BannerDismiss(BannerId),
    /// `change_host`: the empty conversation moves to the next backend.
    Cycle,
    Submit(Rc<str>),
    QueueDraft,
    /// The edit-queued key: the newest queued message goes back to the composer.
    EditQueued,
    /// The reasoning shortcuts: a step down or up (`handle_reasoning_shortcut`).
    Reasoning(codex_tui_core::screens::select::reasoning::Direction),
    CtrlC,
    RetryWriter,
    /// The provider-error notice's keys: `c` continues in a new chat, `n` starts
    /// a fresh one, Esc (and the third hint) opens the chats to resume.
    ProviderContinue,
    ProviderNew,
    ProviderOthers,
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
    /// A key that is the transcript selection's (copy, copy and follow, clear).
    Selection(SelectionKey),
    /// A suggestion was put in: a command is run, the others are text.
    Picked(Rc<str>),
    Escape,
    /// `?`: the shortcut overlay (the footer's hint).
    Shortcuts,
    /// `←` on an empty composer: the agents page (the footer's hint).
    Agents,
    /// The composer's text changed (the field's own notification).
    Changed(Rc<str>),
    /// One of the model's one-shots the listener told (`widgets/directives.rs`).
    Edge(Edge),
}

/// The parts of the thread the page's shape follows: a change of one rebuilds it.
fn thread_shape(cx: &mut Cx<'_, ChatState>) {
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
}

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState {
            edges: None,
            forking: None,
            shortcuts: false,
            draft_empty: self.ui.composer.text().is_empty(),
            search_rev: self.ui.draft.search_rev(),
            draft_shell: self.ui.draft.shell(),
            draft_vim: self.ui.draft.vim_label(),
            node: FocusNode::new(),
            leader: Rc::new(Cell::new(false)),
            control: Rc::new(Cell::new(false)),
            banner: BannerGate::new(),
            banner_blocked: Rc::new(Cell::new(BannerBlocked::default())),
            keymap: Rc::new(RefCell::new(Rc::new(Keymap::defaults()))),
            composer: Rc::new(RefCell::new(self.ui.composer.clone())),
            draft: Rc::new(RefCell::new(self.ui.draft.clone())),
            modes: Rc::new(RefCell::new(self.ui.modes.clone())),
            selection: Rc::new(RefCell::new(self.ui.selection.clone())),
        }
    }
}

impl_widget!(stateful eq Chat);

/// A fork in flight: the thread it copies and the agents notice standing when
/// it was asked (a new error notice is its failure).
#[derive(Clone, Debug, PartialEq)]
struct Forking {
    thread: ThreadId,
    notice: Option<codex_tui_core::screens::agents::Notice>,
}

/// The chords of `change_host` (the leader's prefix and its completion, `Ctrl+X b` by default),
/// as the keymap resolved them.
fn host_chords(keymap: &Keymap) -> Vec<(KeyCombo, KeyCombo)> {
    keymap
        .bindings(GLOBAL_CHANGE_HOST)
        .into_iter()
        .filter_map(|binding| match binding {
            Binding::Chord { prefix, completion } => {
                Some((combo_of(&prefix)?, combo_of(&completion)?))
            }
            Binding::Single(_) => None,
        })
        .collect()
}

impl ChatState {
    /// The fork the page waits on in this thread.
    fn forking_of(&self, thread: &ThreadModel) -> bool {
        self.forking
            .as_ref()
            .is_some_and(|fork| fork.thread == *thread.id())
    }

    /// What the page keeps of the draft (the footer and the Tab binding
    /// read it): empty (shell mode's `!` is content), in shell mode, in
    /// history search.
    fn sync_draft(&mut self, w: &Chat) {
        self.search_rev = w.ui.draft.search_rev();
        self.draft_shell = w.ui.draft.shell();
        self.draft_vim = w.ui.draft.vim_label();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
    }

    /// One of the model's one-shots, told by the listener (`widgets/directives.rs`): each is
    /// carried out here, once, whatever else the page is doing in the same flush.
    fn edge(&mut self, edge: Edge, w: &Chat, cx: &mut Cx<'_, ChatState>) -> Rebuild {
        match edge {
            // The composer clears when the model accepted (or queued) the message, not when it
            // was sent.
            Edge::Submitted => {
                clear_sent(&w.ui);
                self.sync_draft(w);
                Rebuild::Yes
            }
            // Messages the model gave back (an interrupt, a failed steer, the edit-queued key)
            // go into the composer.
            Edge::Restore(restore) => {
                let table = crate::pages::session::composer::strings::of(cx);
                let states = read::<ScreenStates, _>(cx);
                let draft = w.ui.draft.submission_with(
                    &w.ui.composer,
                    |n| table.image(n),
                    states.popovers.sources().mentions,
                );
                put_back(&w.ui, &restore.onto(draft), |n| table.image(n));
                self.sync_draft(w);
                Rebuild::Yes
            }
            // The user messages of a resumed thread's turns seed the composer's history
            // (`replay_backend_thread_turns`).
            Edge::Replayed => {
                let model = read::<ThreadModel, _>(cx);
                seed_history(&w.ui, model.replayed_submissions());
                Rebuild::No
            }
            // The prompt an edit of the backtrack put back (`restore_user_message_to_composer`).
            Edge::Draft(message) => {
                let table = crate::pages::session::composer::strings::of(cx);
                put_back(&w.ui, &message, |n| table.image(n));
                self.sync_draft(w);
                Rebuild::Yes
            }
            Edge::Plan => {
                plan_mode::ask(cx);
                Rebuild::No
            }
            // The thread was archived or deleted (`/archive`, `/delete`): the chat is empty again
            // and the command center opens over it (`return_to_agents_after_removal`).
            Edge::Removed => {
                open(cx, Target::Agents);
                Rebuild::No
            }
            // The fork ends with an error of the server (the copy opens in another thread, which
            // `did_update_widget` takes as its end too).
            Edge::Notice(notice) => {
                let failed = matches!(notice, Some(codex_tui_core::screens::agents::Notice::Error(_)));
                if failed
                    && self
                        .forking
                        .as_ref()
                        .is_some_and(|fork| fork.notice != notice)
                {
                    self.forking = None;
                    return Rebuild::Yes;
                }
                Rebuild::No
            }
        }
    }
}

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn init_state(&mut self, _: &Chat, cx: &mut Cx<'_, Self>) {
        // `?` (the shortcut overlay, `footer_shortcuts_default`) and `←`
        // (the agents page, `handle_agents_navigation`) on an EMPTY composer
        // belong to the page; with text they go to the field.
        self.edges = Some(cx.map(ChatMsg::Edge));
        let send = cx.local_sender();
        let composer = self.composer.clone();
        let draft = self.draft.clone();
        let modes = self.modes.clone();
        let selection = self.selection.clone();
        let (leader, control) = (self.leader.clone(), self.control.clone());
        let (banner, blocked) = (self.banner.clone(), self.banner_blocked.clone());
        let keymap = self.keymap.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            // browsing the prompts: its keys come first, any other key ends it
            // and goes on to the field
            if modes.borrow().browsing() {
                match browse::key(&keymap.borrow(), &key) {
                    Some(step) => {
                        send(ChatMsg::Browse(step));
                        return KeyEventResult::Handled;
                    }
                    None => send(ChatMsg::Browse(Browse::Leave)),
                }
            }
            // the copy keys and Esc are the selection's while something is selected
            // (`handle_selection_key`): they come before the field's
            if selection.borrow().controller().has_selection()
                && let Some(step) = transcript::selection::key(&key)
            {
                send(ChatMsg::Selection(step));
                return KeyEventResult::Handled;
            }
            // `change_host` is the leader chord `Ctrl+X b`, live while the
            // backend control shows (`visible_new_backend_control`), and it
            // works with a draft in the composer.
            if control.get() {
                let chords = host_chords(&keymap.borrow());
                if leader.replace(false) {
                    if chords.iter().any(|(_, done)| done.accepts_press(&key)) {
                        send(ChatMsg::Cycle);
                        return KeyEventResult::Handled;
                    }
                } else if chords.iter().any(|(leader, _)| leader.accepts_press(&key)) {
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
                if pressed_fixed(fixed::DISMISS, &key) && keys.dismissible && !blocked.get().overlay
                {
                    send(ChatMsg::BannerDismiss(shown));
                    return KeyEventResult::Handled;
                }
                if let Some(row) = key
                    .text()
                    .and_then(|digit| digit.parse::<usize>().ok())
                    .filter(|n| (1..=keys.actions).contains(n))
                {
                    banner.options.set(row - 1);
                    send(ChatMsg::BannerPick(shown, row - 1));
                    return KeyEventResult::Handled;
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
            if pressed_fixed(fixed::OPEN_SHORTCUTS, &key) {
                send(ChatMsg::Shortcuts);
            } else if pressed_fixed(fixed::OPEN_AGENTS_FROM_LEFT, &key) {
                send(ChatMsg::Agents);
            } else {
                return KeyEventResult::Ignored;
            }
            KeyEventResult::Handled
        })));
    }

    fn did_update_widget(&mut self, old: &Chat, w: &Chat, _cx: &mut Cx<'_, Self>) {
        if old != w {
            *self.composer.borrow_mut() = w.ui.composer.clone();
            *self.draft.borrow_mut() = w.ui.draft.clone();
            *self.modes.borrow_mut() = w.ui.modes.clone();
            *self.selection.borrow_mut() = w.ui.selection.clone();
            self.sync_draft(w);
            // the thread under the page changed: what it had open closes, and the fork it waited
            // on is another thread's
            old.ui.modes.close();
            self.leader.set(false);
            self.forking = None;
        }
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.banner.options.dispose();
    }

    fn update(&mut self, msg: ChatMsg, w: &Chat, cx: &mut Cx<'_, Self>) -> Rebuild {
        // the model's one-shots are no input: no gate of the keys applies to them
        if let ChatMsg::Edge(edge) = msg {
            return self.edge(edge, w, cx);
        }
        let hs = read::<Hs, _>(cx);
        let modes = &w.ui.modes;
        let previous = (
            self.shortcuts,
            modes.primed(),
            self.draft_empty,
            self.draft_shell,
            self.draft_vim,
            self.search_rev,
            modes.detailed(),
        );
        // a fork in flight takes no key (`handle_key_event`)
        if self.forking_of(&read::<ThreadModel, _>(cx)) {
            return Rebuild::No;
        }
        // any other input between the two Escs ends the pair
        if !matches!(msg, ChatMsg::Escape) {
            modes.interrupt();
        }
        match msg {
            ChatMsg::BannerPick(banner, row) => {
                cx.emit(&hs.pick_banner, banner.picked(row));
                return Rebuild::No;
            }
            ChatMsg::BannerDismiss(banner) => {
                cx.emit(&hs.dismiss_banner, banner.dismissed());
                return Rebuild::No;
            }
            ChatMsg::Submit(text) => {
                // The same Enter may have run it through a popup's pick. A
                // draft in shell mode has the `!` as its mode, not as text: a
                // bare `!` is the help line (`submit_shell_command`). Remote
                // images alone are a message of their own.
                if !w.ui.composer.text().is_empty()
                    || w.ui.draft.shell()
                    || w.ui.draft.remote_count() > 0
                {
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
            ChatMsg::Selection(key) => {
                let selection = w.ui.selection.controller();
                if key == SelectionKey::Clear {
                    selection.clear();
                } else {
                    let text = selection.text();
                    if !text.is_empty() {
                        let job = CopyJob::selected(text, true, &read::<TuiSettings, _>(cx));
                        let job = if key == SelectionKey::CopyAndFollow {
                            job.following()
                        } else {
                            job
                        };
                        cx.emit(&hs.copy, job);
                    }
                }
                return Rebuild::No;
            }
            ChatMsg::Browse(step) => {
                let thread = read::<ThreadModel, _>(cx);
                let targets = Targets::of(thread.transcript(), thread.can_load_history());
                browse::take(&step, modes, &w.ui.follow, targets.count);
                // the prompt chosen: the transcript is the chat's again; the
                // edit itself is the thread's intent (`ThreadIntent::EditPrompt`)
                if step == Browse::Confirm
                    && let Some(chosen) = modes.confirm(Some(thread.id()), targets.count)
                {
                    modes.close();
                    if let Some(intent) = backtrack::edit_intent(cx, &thread, &chosen) {
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                }
            }
            ChatMsg::QueueDraft => {
                let draft = submission(&w.ui);
                if !draft.is_empty() || w.ui.draft.remote_count() > 0 {
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
            ChatMsg::Reasoning(direction) => reasoning_shortcut(cx, &hs, direction),
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
            ChatMsg::ProviderContinue => {
                // `ContinueProviderChat`: only the locked chat on screen, and only one
                // that was saved (`rollout_path`)
                let thread = read::<ThreadModel, _>(cx);
                let slots = thread.screen_slots();
                if thread.provider_error().is_some()
                    && let Some(path) = &slots.chat.path
                {
                    let message = chat_strings::of(cx)
                        .provider_continue_message(slots.chat.name.as_deref(), path);
                    cx.emit(
                        &hs.screen,
                        ScreenIntent::from(chat::Intent::Session {
                            thread: Some(thread.id().clone()),
                            command: SlashCommand::New,
                            intent: codex_tui_core::commands::SessionIntent::ContinueChat {
                                message,
                            },
                        }),
                    );
                }
            }
            ChatMsg::ProviderNew => {
                let thread = read::<ThreadModel, _>(cx);
                if thread.provider_error().is_some() {
                    cx.emit(
                        &hs.screen,
                        ScreenIntent::from(chat::Intent::Session {
                            thread: Some(thread.id().clone()),
                            command: SlashCommand::New,
                            intent: codex_tui_core::commands::SessionIntent::New { name: None },
                        }),
                    );
                }
            }
            ChatMsg::ProviderOthers => {
                if read::<ThreadModel, _>(cx).provider_error().is_some() {
                    modes.close();
                    open(cx, Target::Resume);
                    return Rebuild::Yes;
                }
            }
            ChatMsg::ForkWriter => {
                let thread = read::<ThreadModel, _>(cx);
                if thread.read_only() {
                    let intent =
                        codex_tui_core::screens::agents::Intent::Fork(thread.id().to_string());
                    cx.emit(&hs.screen, ScreenIntent::from(intent));
                    self.forking = Some(Forking {
                        thread: thread.id().clone(),
                        notice: read::<ScreenStates, _>(cx).agents.notice.clone(),
                    });
                    return Rebuild::Yes;
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
            ChatMsg::Edge(_) => {}
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
                    let note = Note::from(codex_tui_core::commands::Notice::NotAvailable(
                        SlashCommand::Side,
                    ));
                    let thread = read::<SessionSlice, _>(cx).active.clone();
                    cx.emit(
                        &hs.screen,
                        ScreenIntent::from(chat::Intent::Note { thread, note }),
                    );
                }
                // Alt+R (`apply_raw_output_mode(.., notify: false)`): the mode alone, where
                // `/raw` says which one it is now
                Action::ToggleRaw => {
                    let on = !read::<ScreenStates, _>(cx).chat.raw_output;
                    cx.emit(&hs.screen, ScreenIntent::from(chat::Intent::RawOutput(on)));
                }
                Action::OpenWarnings => open_warnings(cx),
                // `cycle_collaboration_mode`: the thread's own switch; the model refuses
                // it while a turn runs
                Action::CycleMode => {
                    let thread = read::<ThreadModel, _>(cx);
                    if !thread.read_only() && !refuses_parent_owned(cx, &hs) {
                        let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::CycleMode);
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                }
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
                // the chat is locked for its model provider: Esc opens the chats
                // to resume, whatever the reader shows (`OpenResumePicker`)
                if thread.provider_error().is_some() {
                    modes.close();
                    open(cx, Target::Resume);
                    return Rebuild::Yes;
                }
                // another process writes the thread: Esc leaves for the
                // command center, whatever the reader shows
                // (`ExternalWriter` arm of `handle_tui_event`)
                if thread.read_only() {
                    modes.close();
                    open(cx, Target::Agents);
                    return Rebuild::Yes;
                }
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
                self.draft_vim,
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
        let keymap = crate::keymap::keymap(cx);
        *self.keymap.borrow_mut() = keymap.clone();
        // what the page's shape follows (each build selects anew)
        thread_shape(cx);
        // the backend control's chord is live while the control shows
        let offer = select::<ScreenStates, _, _>(cx, |states| states.agents.cycle_offer());
        self.control.set(
            session.cycle_label(&thread, offer).is_some() && !w.ui.draft.searching(),
        );
        let drawn = slice::drawn(cx, thread.id());
        let backend = drawn.owner;
        let connection = session.connections.get(&backend);
        let slots = thread.screen_slots();
        let facts = &drawn.status;
        // what the sparkle needs to tell a command being typed from a message
        let tiers = codex_tui_core::screens::chat::facts::tier_commands(
            facts,
            slots.chat.model.as_deref(),
            &drawn.catalog,
            &drawn.tiers,
        );
        let mut command_flags = drawn.flags;
        command_flags.service_tier_commands_enabled = !tiers.is_empty();
        w.ui.sparkle.know(command_flags, tiers, self.draft_shell);
        // Reverse history search: the footer's row and what it takes from the page
        let searching = w.ui.draft.search_state();
        // Ctrl+T shows the details in the chat's own transcript, the composer
        // below keeps the keys; browsing the prompts marks one in it. The
        // modes tick when either moves (Flutter's `ListenableBuilder`).
        let reader = {
            let (follow, modes) = (w.ui.follow.clone(), w.ui.modes.clone());
            let selection = w.ui.selection.clone();
            ListenableBuilder::new(w.ui.modes.listenable(), move |_, _| {
                Pane {
                    follow: follow.clone(),
                    selection: selection.clone(),
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
        let (panel, _) = shortcut_panel(cx, running);
        let chat_strings = chat_strings::of(cx);
        let mut hints: Option<Footer> = None;
        let forking = self.forking_of(&thread);
        // the server refused the chat for its model provider: a notice stands
        // where the composer was, and the keys are the notice's
        let provider = thread
            .provider_error()
            .and_then(|error| provider_error::Lock::of(&serde_json::to_value(error).ok()?));
        let can_continue = slots.chat.path.is_some();
        let (composer, footer): (AnyWidget, AnyWidget) = if forking {
            // `bottom_pane_renderable` while `fork_in_progress`
            (
                rtwk::layout::Padding::new(
                    EdgeInsets::only(2, 1, 2, 1),
                    Text::new(chat_strings.forking).style(tokens.dim()),
                )
                .into(),
                SizedBox::shrink().into(),
            )
        } else if let Some(lock) = &provider {
            (
                provider_error::notice(cx, chat_strings, &tokens, lock, can_continue, None),
                provider_error::footer(cx, chat_strings, &tokens, can_continue),
            )
        } else if thread.read_only() {
            (
                super::external_writer::notice(cx, chat_strings, &tokens),
                super::external_writer::footer(cx, chat_strings, &tokens),
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
                    Line::Close
                } else if w.ui.modes.primed()
                    && Targets::of(thread.transcript(), thread.can_load_history()).count > 0
                {
                    Line::EscPrimed
                } else {
                    Line::Passive
                };
                let props = Footer {
                    line,
                    agents: true,
                    draft_empty: self.draft_empty,
                    shell: self.draft_shell,
                    vim: w
                        .ui
                        .draft
                        .with_vim(|vim| vim.is_enabled().then(|| vim.mode())),
                    hint_override: None,
                    flash: None,
                    on_queue: Some(cx.send(ChatMsg::QueueDraft)),
                    on_warnings: Some(cx.send(ChatMsg::Action(Action::OpenWarnings))),
                    on_close: Some(cx.send(ChatMsg::Shortcuts)),
                };
                hints = Some(props.clone());
                let footer: AnyWidget = props.into();
                if line == Line::Passive {
                    transcript::follow::footer(
                        cx,
                        w.ui.follow.clone(),
                        w.ui.selection.clone(),
                        self.draft_empty && !w.ui.modes.primed(),
                        footer,
                    )
                } else {
                    footer
                }
            };
            (composer.into(), footer)
        };
        // `⚠ N warnings · f2 to view` at the right of the read-only footer; the
        // composer's footer holds its own.
        let footer = if searching.is_none() && thread.read_only() && !forking {
            let data = Theme::of(cx);
            with_warning_notice(
                footer,
                drawn.warnings,
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
                || run.has_activity_rows()
                || matches!(session.status, codex_tui_core::app::Session::Starting));
        // The bottom pane (`bottom_pane_renderable`): everything below the
        // transcript, which gets the rows left; the reader's gap above the
        // composer is a flexible row, the first thing a short window clips.
        let mut body = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .overflow_intended(true)
            .child(StatusSlot)
            .child(NoticeLine)
            .child(EditWatch);
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
            catalog: &drawn.catalog,
            chat: &drawn.facts,
            raw_output: drawn.raw_output,
            usage_pending: chat::facts::usage_pending(slots),
        };
        let line = status_spans(context.items_for(facts), facts.colors, &tokens, |id| {
            context.link_for(id)
        });
        // The backend control above the composer (`rendering.rs`,
        // `visible_new_backend_control`): the displayed backend, and the key.
        let control = session
            .cycle_label(&thread, drawn.offer)
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
        let mut body = if thread.read_only() || provider.is_some() {
            body.child(composer)
        } else {
            // the row the reader's control takes (`composer_gap`); a popup
            // above the draft takes the rows before it does
            body.child({
                let gap = transcript::follow::gap(cx, w.ui.follow.clone(), status_above);
                let mentions = w.ui.draft.mentions();
                ListenableBuilder::new(mentions.listenable(), move |_, _| {
                    if mentions.is_open() {
                        SizedBox::shrink().into()
                    } else {
                        gap.clone()
                    }
                })
            })
            // the `@`/`$` popup stands right above the draft and takes the
            // rows the draft leaves, 17 at most (`ActivePopup::MentionV2`
            // above the composer: the draft first, then what remains)
            .child(mention::slot(w.ui.clone()))
            .child(
                // keyed: the blank rows and the status come and go above it
                // and the footer's type changes below, and an unkeyed child
                // between two changed neighbours is a new element (its state,
                // and the messages queued for it, lost)
                AnyWidget::from(Sparkle {
                    ui: w.ui.clone(),
                    focus: self.node.clone(),
                    eligible: slots.chat.sparkle_eligible(facts),
                    // the footer's overlay and search row, and a turn that runs,
                    // take the stage (`render_sparkle_at`)
                    hidden: self.shortcuts || running || searching.is_some(),
                    placeholder: strings.chat.placeholder,
                    child: BoxWidget::new()
                        .style(tokens.prompt_surface(cx, false))
                        .padding(EdgeInsets::only(0, 1, 1, 1))
                        .child(composer)
                        .into(),
                })
                .with_key(Key::value("composer")),
            )
        };
        if !line.is_empty() && !thread.read_only() && provider.is_none() {
            body = body.child(StatusRows {
                spans: line,
                composer: w.ui.composer.clone(),
                hints,
            });
        }
        let body = body.child(footer);
        // the terminal title keeps being told while the selector takes the bottom pane
        let mut titles: Vec<AnyWidget> = Vec::new();
        let action_required =
            select::<RequestQueue, bool, _>(cx, |queue| !queue.entries.is_empty());
        let title = context.title(
            facts,
            TitleInputs {
                spinner: facts.animations && facts.progress_effect && (running || compacting),
                action_required: action_required.then_some(strings.chat.action_required),
            },
        );
        if title.has_spinner() {
            titles.push(super::window_title::WindowTitle { title }.into());
        } else if !title.text(None).is_empty()
            || facts
                .terminal_title
                .as_ref()
                .is_some_and(|items| !items.is_empty())
        {
            titles.push(TerminalTitle::new(title.text(None), super::window_title::empty()).into());
        }
        // The Luna Reserve selector owns the bottom pane (`show_selection_view`): no composer,
        // no footer; the draft waits in `UiApp.ui`.
        let mut body = if reserve_of(&session, backend, facts.model.as_deref(), &thread).is_some() {
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
        // a window of three rows or fewer shows nothing, the running state
        // included (`ui_snapshots_small_heights_idle`): the page keeps its
        // layout and clips, and a blank sheet laid over it by a layout builder
        // that holds no state covers it, so that nothing of the page's own
        // state stands under a builder
        let page = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .overflow_intended(true)
            .child(Expanded::new(1, transcript))
            .child(body);
        let blank = LayoutBuilder::new(|_, constraints| {
            if constraints.max_height < HIDDEN_BELOW_ROWS {
                BoxWidget::new().style(Style::new()).into()
            } else {
                SizedBox::shrink().into()
            }
        });
        let mut body = Stack::new().fit(StackFit::Expand).child(page).child(blank);
        if let Some(on_edge) = &self.edges {
            body = body.child(Listener {
                on_edge: on_edge.clone(),
            });
        }
        let mut focus = Focus::new(body);
        for (id, msg) in [
            (fixed::INTERRUPT_OR_QUIT, ChatMsg::CtrlC),
            (fixed::QUIT_ON_EMPTY, ChatMsg::CtrlD),
            (fixed::ESCAPE, ChatMsg::Escape),
            (fixed::DEBUG_KEY, ChatMsg::DebugMenu),
            (fixed::TRANSCRIPT_PAGE_UP, ChatMsg::Page(-1)),
            (fixed::TRANSCRIPT_PAGE_DOWN, ChatMsg::Page(1)),
        ] {
            for key in crate::keymap::presses(&keymap, id) {
                focus = focus.bind(key, cx.send(msg.clone()));
            }
        }
        // Tab queues while a turn runs and submits otherwise, but not a
        // shell command (`queue_keys` in `handle_key_event_without_popup`).
        if (running || !self.draft_shell) && !self.draft_empty && searching.is_none() {
            for key in crate::keymap::presses(&keymap, fixed::TAB) {
                focus = focus.bind(key, cx.send(ChatMsg::QueueDraft));
            }
        }
        if thread.read_only() {
            focus = focus.can_request_focus(true).autofocus();
            for (id, msg) in [
                (fixed::RETRY_WRITER, ChatMsg::RetryWriter),
                (fixed::FORK_WRITER, ChatMsg::ForkWriter),
                (fixed::QUIT_READ_ONLY, ChatMsg::QuitReadOnly),
            ] {
                for key in crate::keymap::presses(&keymap, id) {
                    focus = focus.bind(key, cx.send(msg.clone()));
                }
            }
        }
        if provider.is_some() {
            focus = focus.can_request_focus(true).autofocus();
            for (key, msg) in provider_error::keys(&keymap, can_continue) {
                focus = focus.bind(key, cx.send(msg));
            }
        }
        for (action, key) in keys::presses(&keymap) {
            focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
        }
        // The footer's entries are these bindings' hints: every key it shows is
        // bound here.
        let pages = strings.pages;
        let focus =
            AnyWidget::from(focus).with_key(Key::value((thread.read_only(), provider.is_some())));
        let mut keys = vec![
            ("left".to_string(), cx.send(ChatMsg::Agents)),
            (pages.key_shortcuts.to_string(), cx.send(ChatMsg::Shortcuts)),
        ];
        keys.extend(crate::keymap::bindings(
            &keymap,
            codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
            &cx.send(ChatMsg::EditQueued),
        ));
        use codex_tui_core::screens::select::reasoning::Direction;
        keys.extend(crate::keymap::bindings(
            &keymap,
            codex_tui_core::keymap::actions::CHAT_DECREASE_REASONING_EFFORT,
            &cx.send(ChatMsg::Reasoning(Direction::Lower)),
        ));
        keys.extend(crate::keymap::bindings(
            &keymap,
            codex_tui_core::keymap::actions::CHAT_INCREASE_REASONING_EFFORT,
            &cx.send(ChatMsg::Reasoning(Direction::Raise)),
        ));
        KeyBindings::new(crate::keymap::first_of_each(keys), focus)
            .hints([
                ("left", pages.hint_agents),
                (pages.key_shortcuts, pages.hint_shortcuts),
            ])
            .into()
    }
}
