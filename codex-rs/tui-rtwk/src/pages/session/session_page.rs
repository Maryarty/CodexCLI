//! The session page: the transcript, a status line and the composer, one `Component` (`Chat`,
//! its `ChatState` and `ChatMsg`).
//!
//! One owner, by reason: the page's state is shared. The key handler of the composer's node, the
//! build and the update all read and write the same cells (the draft's flags, the shortcut
//! overlay, the banner's gate, the fork in flight), so the type cannot be cut into components
//! without handing those cells around. What can be cut is cut: each concern is a file of
//! `session/`, a child module of this one (it sees the state's fields) that holds its code and
//! nothing of the state's shape. A package that changes a concern edits its file, not this one:
//!
//! - `frame`    what one build reads (providers, thread, drawn slice) and `tell`
//! - `input`    the composer node's key handler, the focus's bindings, the footer's hints
//! - `reader`   the transcript's pane, the selection's and browsing's keys
//! - `slot`     what stands where the composer is (composer, fork wait, locks) and its footer
//! - `bottom`   the bottom pane: status, banners, control, composer's place, selector
//! - `title`    the terminal title
//! - `overlay`  the blank sheet of a window too small to draw
//! - `send`     Enter, Tab's queue, a picked command, the edit-queued key
//! - `questions` the editor of the model's questions in the composer's place
//! - `escape`   Esc's ladder
//! - `leave`    Ctrl+C and Ctrl+D a feature takes first (the return from a side conversation)
//! - `presets`  the shortcut menu of `/presets` over the composer: its keys and its verbs
//! - `lock`     the keys of a read-only thread and of a provider-error lock
//! - `edges`    the model's one-shots
//!
//! What stays here: the types, the state's lifecycle, the update's routing (one line per
//! message, the short ones in place) and the build's order. Its line count is a ratchet
//! (`tests/rules/ui_registry.rs`, `hubs_only_shrink`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::binding::{AppBinding, ScreenMode};
use rtwk::editing::TextEditingController;
use rtwk::focus::FocusNode;
use rtwk::layout::{Stack, StackFit};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::commands::Target;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::Keymap;
use codex_tui_core::threads::ThreadModel;

use crate::app::debug::menu::DebugMenu;
use crate::app::view::Hs;
use crate::features::PageKey;
use crate::pages::session::key_actions::Keyed;
use crate::pages::session::open::open;
use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::directives::{Edge, Listener};
use crate::widgets::finder::Finder;
use crate::widgets::thread_ui::{Modes, TextSelection, ThreadUi};

use super::actions::reasoning_shortcut;
use super::banners::BannerGate;

#[path = "session/bottom.rs"]
mod bottom;
#[path = "session/edges.rs"]
mod edges;
#[path = "session/escape.rs"]
mod escape;
#[path = "session/frame.rs"]
mod frame;
#[path = "session/input.rs"]
mod input;
#[path = "session/leave.rs"]
mod leave;
#[path = "session/lock.rs"]
mod lock;
#[path = "session/msg.rs"]
mod msg;
#[path = "session/overlay.rs"]
mod overlay;
#[path = "session/presets.rs"]
mod presets;
#[path = "session/questions.rs"]
mod questions;
#[path = "session/reader.rs"]
mod reader;
#[path = "session/send.rs"]
mod send;
#[path = "session/slot.rs"]
mod slot;
#[path = "session/startup_confirm.rs"]
mod startup_confirm;
#[path = "session/title.rs"]
mod title;

pub use msg::ChatMsg;

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
    /// The editor of the model's questions: open or not, and its drafts.
    asking: questions::Asking,
    /// A queued command opened a sheet: the queue goes on when the page is in front again.
    resume: bool,
    /// The read-only thread this page asked to fork and heard no answer of
    /// (the oracle's `fork_in_progress`): its notice gives way to "Forking
    /// conversation…" and the keys wait. A failed fork ends it.
    forking: Option<Forking>,
    /// The shortcut overlay is expanded above the composer (the oracle's
    /// `FooterMode::ShortcutOverlay`).
    shortcuts: bool,
    /// Enter confirmed the startup draft: it is submitted when the session is ready.
    startup_confirm: Option<startup_confirm::Confirmed>,
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
    /// Its Find: the keys of a match being read are Find's.
    find: Rc<RefCell<Finder>>,
}

/// What keeps the banner's keys from the banner (`handle_inline_banner_key`).
#[derive(Clone, Copy, Debug, Default)]
struct BannerBlocked {
    /// A turn runs: the number keys and Esc are the turn's.
    running: bool,
    /// The shortcut overlay is open: Esc closes it.
    overlay: bool,
    /// Questions wait (the editor is closed) or messages are queued: the edit-queued key is the
    /// chat's before it is the field's (`Shift+Left` would select).
    edit: bool,
}

impl StatefulWidget for Chat {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState {
            edges: None,
            asking: questions::Asking::default(),
            resume: false,
            forking: None,
            shortcuts: false,
            startup_confirm: None,
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
            find: Rc::new(RefCell::new(self.ui.find.clone())),
        }
    }
}

impl_widget!(stateful eq Chat);

/// A fork in flight: the thread it copies (the failure of a fork is the app's `fork_failed`
/// signal, a directive the listener turns into `Edge::ForkFailed`).
#[derive(Clone, Debug, PartialEq)]
struct Forking {
    thread: ThreadId,
}

/// What the page's drawing follows of its own state: an update that changes none of these draws
/// nothing again.
#[derive(PartialEq)]
struct Shown {
    shortcuts: bool,
    confirmed: bool,
    primed: bool,
    draft_empty: bool,
    draft_shell: bool,
    draft_vim: Option<&'static str>,
    search_rev: u64,
    detailed: bool,
}

impl ChatState {
    /// The fork the page waits on in this thread.
    fn forking_of(&self, thread: &ThreadModel) -> bool {
        self.forking
            .as_ref()
            .is_some_and(|fork| fork.thread == *thread.id())
    }

    fn shown(&self, modes: &Modes) -> Shown {
        Shown {
            shortcuts: self.shortcuts,
            confirmed: self.startup_confirm.is_some(),
            primed: modes.primed(),
            draft_empty: self.draft_empty,
            draft_shell: self.draft_shell,
            draft_vim: self.draft_vim,
            search_rev: self.search_rev,
            detailed: modes.detailed(),
        }
    }
}

impl State for ChatState {
    type Widget = Chat;
    type Msg = ChatMsg;

    fn init_state(&mut self, _: &Chat, cx: &mut Cx<'_, Self>) {
        self.edges = Some(cx.map(ChatMsg::Edge));
        input::install(self, cx);
    }

    fn did_update_widget(&mut self, old: &Chat, w: &Chat, _cx: &mut Cx<'_, Self>) {
        if old != w {
            *self.composer.borrow_mut() = w.ui.composer.clone();
            *self.draft.borrow_mut() = w.ui.draft.clone();
            *self.modes.borrow_mut() = w.ui.modes.clone();
            *self.selection.borrow_mut() = w.ui.selection.clone();
            *self.find.borrow_mut() = w.ui.find.clone();
            self.sync_draft(w);
            // the thread under the page changed: what it had open closes, and the fork it waited
            // on is another thread's
            old.ui.modes.close();
            self.leader.set(false);
            self.banner.quick_menu.set(false);
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
        let previous = self.shown(modes);
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
                banner.pick(row, &hs, cx);
                return Rebuild::No;
            }
            ChatMsg::BannerDismiss(banner) => {
                return banner.dismiss(&self.banner, &hs, cx);
            }
            ChatMsg::Submit(text) => self.submit(&text, w, cx, &hs, previous.primed),
            ChatMsg::Page(pages) => w.ui.follow.page(i64::from(pages)),
            ChatMsg::Selection(key) => {
                reader::selected(w, cx, &hs, key);
                return Rebuild::No;
            }
            ChatMsg::Browse(step) => reader::browsed(w, cx, &hs, step),
            ChatMsg::QueueDraft => self.queue_draft(w, cx, &hs),
            ChatMsg::EditQueued => return self.edit_key(cx, &hs),
            ChatMsg::Question(outcome) => return self.outcome(outcome, cx, &hs),
            ChatMsg::Reasoning(direction) => reasoning_shortcut(cx, &hs, direction),
            ChatMsg::Picked(text) => self.picked(&text, w, cx, &hs),
            ChatMsg::RetryWriter => lock::retry(cx, &hs),
            ChatMsg::ProviderContinue => lock::provider_continue(cx, &hs),
            ChatMsg::ProviderNew => lock::provider_new(cx, &hs),
            ChatMsg::ProviderOthers => {
                if lock::provider_others(modes, cx) {
                    return Rebuild::Yes;
                }
            }
            ChatMsg::ForkWriter => {
                if self.fork_writer(cx, &hs) {
                    return Rebuild::Yes;
                }
            }
            ChatMsg::QuitReadOnly => cx.emit(&hs.quit, ()),
            ChatMsg::CtrlD if leave::taken(PageKey::Quit, w, cx, &hs) => return Rebuild::No,
            ChatMsg::CtrlC => leave::ctrl_c(w, cx, &hs),
            ChatMsg::Presets(key) => return presets::key(key, self, w, cx, &hs),
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
                self.startup_edited(w);
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
            ChatMsg::Action(action) => action.run(cx, &Keyed { hs: &hs, ui: &w.ui }),
            ChatMsg::Escape => {
                if let Some(rebuild) = self.escape(w, cx, &hs) {
                    return rebuild;
                }
            }
        }
        if previous == self.shown(modes) {
            Rebuild::No
        } else {
            Rebuild::Yes
        }
    }

    fn build(&self, w: &Chat, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let frame = frame::Frame::read(self, w, cx);
        self.tell(w, &frame);
        // the sections, top to bottom: the transcript, the composer's place and the bottom pane
        // around it, the terminal title, and the blank sheet a window too small to draw lays over
        let inline_draft = super::transcript_header::startup_draft(&frame.session)
            && AppBinding::maybe_of(cx).is_some_and(|binding| binding.mode() == ScreenMode::Inline);
        let transcript = reader::pane(self, w, &frame, inline_draft);
        let slot = slot::slot(self, w, cx, &frame);
        let body = bottom::body(self, w, cx, &frame, slot);
        let body = bottom::with_titles(&frame, body, title::titles(cx, &frame));
        let page = overlay::page(transcript, body, inline_draft);
        let focus = input::focus(self, cx, &frame, page);
        // The listener stands outside the keyed focus: a key that changes in the same flush as a
        // directive (the continue key ends the lock and gives the words in one step) would end its
        // state, and the edge with it.
        let mut shown = Stack::new().fit(StackFit::Expand).child(focus);
        if let Some(on_edge) = &self.edges {
            shown = shown.child(Listener {
                on_edge: on_edge.clone(),
            });
        }
        input::hints(cx, &frame, AnyWidget::from(shown))
    }
}
