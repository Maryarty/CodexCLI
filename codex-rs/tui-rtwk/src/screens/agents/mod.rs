//! `agents` (S21): the view half of the agents command center. One
//! `PageRoute` over the model (`ScreenStates.agents`): the header with its
//! tab strip, the grouped task list (a `ListView`, scrolled by item) beside
//! the details of the highlighted task, the key hints. Its model, events,
//! intents and RPCs are core's `codex_tui_core::screens::agents`; this file
//! holds the keys, the mouse and the widget state, `view.rs` the widgets.
//! Oracle: `tui/src/app/agent_center/` and `app/agents_overview_view.rs`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::controls::SelectionController;
use rtwk::editing::TextEditingController;
use rtwk::focus::{ExcludeFocus, Focus, FocusNode, FocusScope, KeyBindings};
use rtwk::foundation::{Listening, ValueNotifier};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{
    Alignment, Column, CrossAxisAlignment, Expanded, Offstage, Stack, StackFit,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::{read, select};
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::Intent as AgentsIntent;
use codex_tui_core::screens::agents::{Action, Center, CenterRow, Lens, Model, Sel};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, Shared};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod dialogs;
mod help;
mod path;
mod soft;
mod tabs;
pub mod strings;
mod usage;
mod view;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "three tasks of one project; the first one needs you",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "attention",
            description: "a task whose child waits for an approval and a response",
            script: include_str!("fixtures/attention.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "navigation",
            description: "a ready, an errored and an unloaded task under the chat",
            script: include_str!("fixtures/navigation.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "search",
            description: "one task, for the search line",
            script: include_str!("fixtures/search.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "clip",
            description: "one task, for long text in the search and rename lines",
            script: include_str!("fixtures/clip.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging5",
            description: "five tasks of one project",
            script: include_str!("fixtures/paging5.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging30",
            description: "thirty tasks of two projects",
            script: include_str!("fixtures/paging30.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "confirm",
            description: "one task, for the archive and delete confirmations",
            script: include_str!("fixtures/confirm.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "progress",
            description: "three inactive tasks, for the archive and delete in progress",
            script: include_str!("fixtures/progress.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "columns",
            description: "six tasks of every status and age, pins, an unread one",
            script: include_str!("fixtures/columns.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "colors",
            description: "two unread tasks, ids 42 and 43, thread colours on",
            script: include_str!("fixtures/colors.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "colors_off",
            description: "two unread tasks, ids 42 and 43, thread colours off",
            script: include_str!("fixtures/colors_off.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "empty_narrow",
            description: "no tasks at 48 cells, with nowhere to go back to",
            script: include_str!("fixtures/empty_narrow.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "busy",
            description: "no tasks, a worktree is being created",
            script: include_str!("fixtures/busy.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "older_server",
            description: "no tasks, and the server is older than the client",
            script: include_str!("fixtures/older_server.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging30_offline",
            description: "thirty tasks while the connection is lost",
            script: include_str!("fixtures/paging30_offline.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| {
        Screen {
            can_return: true,
            open: true,
        }
        .into()
    })
}

/// Opens the command center on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| {
        Screen {
            can_return: true,
            open: true,
        }
        .into()
    })
}

/// `codex-rtwk agents`: the route of the command center with nowhere to go
/// back to (`ui::start_page::StartPage` pushes it over the session); a
/// session opened from it pops the center and shows the chat.
pub fn start_route() -> PageRoute<()> {
    PageRoute::new(|_| {
        Screen {
            can_return: false,
            open: true,
        }
        .into()
    })
}

/// The command center; `can_return` is whether a chat lies under it (the
/// oracle's `can_return_to_chat`: it adds the back hint).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub can_return: bool,
    /// Whether mounting asks for the tasks (a live screen); a seeded one
    /// shows its fixture as it is.
    pub open: bool,
}

/// The handlers of the rows' clicks.
type RowTaps = Rc<Vec<Handler<PointerEvent>>>;

/// What the screen shows: the list, the shortcut help in its place, or the
/// editing line above the list. The controller's text is reset when a line
/// opens (a rename starts from the task's name, a search from nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    Browse,
    Help,
    Search,
    Rename(String),
}

/// Widget state: the controllers rtwk's widgets keep the list's cursor,
/// the editing line's text, the list's focus and where it is scrolled in.
pub struct ScreenState {
    selection: SelectionController<view::RowKey>,
    edit: TextEditingController,
    list_focus: FocusNode,
    /// The empty list's line, and the help: each a focus stop that holds the
    /// keys while it shows.
    empty_focus: FocusNode,
    help_focus: FocusNode,
    edit_focus: FocusNode,
    /// The failure dialog was opened for the model's failure.
    failing: bool,
    /// The status tab.
    filter: usize,
    scroll: Rc<ValueNotifier<view::Scrolled>>,
    /// The row handlers made for the rows the list last showed.
    row_taps: RefCell<Option<(Vec<view::RowKey>, RowTaps)>>,
    listening: Option<Listening>,
    mode: Mode,
    /// What the usage request of the highlighted task depends on, as the
    /// screen last saw it: a change asks the model for it (oracle: the
    /// repaint of the view asks).
    usage_seen: Option<UsageKey>,
    /// How many resumes the model had answered when the screen opened: one
    /// more means a session was opened from here.
    entered: Option<u32>,
    /// The search text the screen last saw: a change highlights the first
    /// match (oracle: `edit_input`).
    search_seen: String,
    /// The cursor before a search began (oracle: `browse_selection`).
    browse: Option<view::RowKey>,
}

/// The center's model, the one part of the app's screens this screen reads: it
/// is rebuilt when this model changes, not when another screen's does.
pub(super) fn watched<S>(cx: &mut Cx<'_, S>) -> Shared<Model> {
    select::<ScreenStates, Shared<Model>, _>(cx, |states| states.agents.clone())
}

/// What the usage request depends on: the highlighted task, when its
/// estimate was fetched, whether a request is in flight, the clock.
#[derive(Clone, Debug, PartialEq, Eq)]
struct UsageKey {
    thread: Option<String>,
    fetched_at: Option<i64>,
    pending: bool,
    now: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    /// The list's cursor moved.
    Cursor,
    /// A row was clicked.
    Row(view::RowKey),
    Tab(usize),
    /// The search line's text changed.
    Search(String),
    /// Enter in the editing line.
    Submit(String),
    /// Enter on the list's cursor.
    Open,
    /// The confirmation dialog was answered yes: the removal.
    Confirmed(view::Confirm),
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selection: SelectionController::new(),
            edit: TextEditingController::default(),
            list_focus: FocusNode::new(),
            empty_focus: FocusNode::new(),
            help_focus: FocusNode::new(),
            edit_focus: FocusNode::new(),
            failing: false,
            filter: 0,
            scroll: Rc::new(ValueNotifier::new(view::Scrolled::default())),
            row_taps: RefCell::new(None),
            listening: None,
            mode: Mode::Browse,
            usage_seen: None,
            entered: None,
            search_seen: String::new(),
            browse: None,
        }
    }
}

impl_widget!(stateful eq Screen);

/// What a bound key means: a request to the model, or a change of the
/// screen's own state. The task a request is about is the list's cursor,
/// read when the key arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// `ctrl+c`: it cancels a line, otherwise it quits (the app's).
    Interrupt,
    Escape,
    Step { forward: bool },
    Filter { back: bool },
    Help,
    Search,
    Grouping,
    New,
    NewWorktree,
    Rename,
    Archive,
    Delete,
    Hide,
    Pin,
    Stop,
    Open,
    Fork,
    Right,
}

/// The keys of each mode (oracle: `handle_key_event`, `command_center_key`,
/// `rename_key` with the default keymap), bound with rtwk's `KeyBindings`:
/// the keys a focused widget takes (the arrows, the page keys, Home and End
/// of the list; the text of the editing line) never reach them. The list
/// keymap's `j`/`k` and ctrl+n/p/j/k step the highlight.
const HELP_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("?", Act::Help),
    ("ctrl+c", Act::Interrupt),
];
const SEARCH_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
    ("down", Act::Step { forward: true }),
    ("up", Act::Step { forward: false }),
];
const RENAME_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
];
const BROWSE_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("tab", Act::Filter { back: false }),
    ("shift+tab", Act::Filter { back: true }),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
    ("ctrl+j", Act::Step { forward: true }),
    ("ctrl+n", Act::Step { forward: true }),
    ("j", Act::Step { forward: true }),
    ("ctrl+k", Act::Step { forward: false }),
    ("ctrl+p", Act::Step { forward: false }),
    ("k", Act::Step { forward: false }),
    ("backspace", Act::Delete),
    ("right", Act::Right),
    ("?", Act::Help),
    ("g", Act::Grouping),
    ("/", Act::Search),
    ("n", Act::New),
    ("r", Act::Rename),
    ("a", Act::Archive),
    ("h", Act::Hide),
    ("p", Act::Pin),
    ("x", Act::Stop),
    ("o", Act::Open),
    ("f", Act::Fork),
    ("w", Act::NewWorktree),
];

/// One mode's keys as `KeyBindings` takes them: each sends its act.
fn table(cx: &mut Cx<'_, ScreenState>, keys: &[(&'static str, Act)]) -> view::Table {
    keys.iter()
        .map(|(name, act)| (*name, cx.send(Msg::Act(*act))))
        .collect()
}

impl ScreenState {
    /// The list's cursor as the highlight core knows.
    fn cursor(&self) -> Option<Sel> {
        self.selection.cursor().and_then(view::RowKey::into_sel)
    }

    /// Moves the list's cursor (the one copy of the highlight).
    fn put_cursor(&self, key: view::RowKey) {
        if self.selection.cursor().as_ref() != Some(&key) {
            self.selection.set_cursor(Some(key.clone()));
            self.selection.select(key);
        }
    }

    /// What the view shows that changes which rows count.
    fn lens(&self) -> Lens {
        Lens {
            filter: self.filter,
            searching: self.mode == Mode::Search,
            renaming: match &self.mode {
                Mode::Rename(target) => Some(target.clone()),
                _ => None,
            },
        }
    }

    /// What the widgets read of the screen's own state.
    fn ui(&self) -> view::Ui {
        view::Ui {
            filter: self.filter,
            help: self.mode == Mode::Help,
            searching: self.mode == Mode::Search,
            renaming: matches!(self.mode, Mode::Rename(_)),
        }
    }

    fn editing_now(&self) -> bool {
        matches!(self.mode, Mode::Search | Mode::Rename(_))
    }

    fn send(cx: &mut Cx<'_, Self>, intent: AgentsIntent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Agents(intent));
    }

    /// The cursor goes to the task after `id` leaves (oracle:
    /// `selection_after_removal`, a pure function of the model).
    fn after_removal(&self, model: &Model, id: &str) {
        if let Some(next) = model.selection_after_removal(id, &[id.to_owned()], &self.lens()) {
            self.put_cursor(view::RowKey::Task(next));
        }
    }

    /// The answer to the confirmation: a yes asks the model to remove the
    /// task (the cursor leaves it at once).
    fn removal(&mut self, model: &Model, confirm: view::Confirm, cx: &mut Cx<'_, Self>) {
        self.after_removal(model, &confirm.thread);
        Self::send(
            cx,
            match confirm.action {
                Action::Archive => AgentsIntent::Archive(confirm.thread),
                Action::Delete => AgentsIntent::Delete(confirm.thread),
            },
        );
    }

    /// The editing line changes mode; its text is reset for the new one.
    fn set_mode(&mut self, mode: Mode, center: &Center) {
        if mode == self.mode {
            return;
        }
        match &mode {
            Mode::Search => {
                self.browse = self.selection.cursor();
                self.edit.reset("");
            }
            Mode::Rename(target) => {
                let name = center
                    .rows
                    .iter()
                    .find(|row| row.id() == target.as_str())
                    .and_then(|row| row.name.clone())
                    .unwrap_or_default();
                self.edit.reset(&name);
            }
            Mode::Browse | Mode::Help => {
                if self.mode == Mode::Search
                    && let Some(key) = self.browse.take()
                {
                    self.put_cursor(key);
                }
            }
        }
        self.mode = mode;
    }

    /// Enter on a row: the task opens (a search closes), "Show more" asks for
    /// the next rows or matches.
    fn accept(&mut self, model: &Model, sel: Sel, center: &Center, cx: &mut Cx<'_, Self>) {
        let searching = self.mode == Mode::Search;
        match sel {
            Sel::ShowMore if searching => Self::send(cx, AgentsIntent::MoreMatches(self.lens())),
            Sel::ShowMore => Self::send(cx, AgentsIntent::Accept(Sel::ShowMore)),
            Sel::Task(_) if matches!(self.mode, Mode::Rename(_)) => {}
            Sel::Task(id) => {
                Self::send(cx, AgentsIntent::Accept(Sel::Task(id)));
                if searching && !model.held() {
                    self.set_mode(Mode::Browse, center);
                    Self::send(cx, AgentsIntent::ClearSearch);
                }
            }
        }
    }

    /// Esc or ctrl+c on an open line: a search is cleared, the line closes.
    fn cancel_editing(&mut self, _model: &Model, center: &Center, cx: &mut Cx<'_, Self>) {
        if self.mode == Mode::Search {
            Self::send(cx, AgentsIntent::ClearSearch);
        }
        self.set_mode(Mode::Browse, center);
    }

    /// Opens the confirmation of a removal (a dialog of rtwk's overlay).
    fn ask_removal(&mut self, model: &Model, confirm: view::Confirm, cx: &mut Cx<'_, Self>) {
        let ask = view::Ask {
            name: view::confirm_name(strings::of(cx), model, &confirm),
            cloud: model.cloud,
            confirm: confirm.clone(),
        };
        let answer = cx.map(move |_: bool| Msg::Confirmed(confirm.clone()));
        drop(show_dialog::<(), _>(
            cx,
            move |_| {
                dialogs::ConfirmDialog {
                    ask: ask.clone(),
                    answer: answer.clone(),
                }
                .into()
            },
            DialogOptions {
                barrier_alpha: Some(0),
                fade_in: Duration::ZERO,
                ..DialogOptions::default()
            },
        ));
    }

    /// The usage of the highlighted task is asked for when what it depends
    /// on changes: the task, its fetch, the clock (oracle: the repaint that
    /// follows a move asks).
    fn ask_usage(&mut self, model: &Model, center: &Center, cx: &mut Cx<'_, Self>) {
        let thread = center.selected_row().map(|row| row.id().to_owned());
        let usage_key = UsageKey {
            fetched_at: thread
                .as_ref()
                .and_then(|id| model.usage.get(id))
                .and_then(|usage| usage.fetched_at),
            thread,
            pending: model.pending_usage.is_some(),
            now: model.now,
        };
        if self.usage_seen.as_ref() != Some(&usage_key) {
            self.usage_seen = Some(usage_key);
            let thread = self.usage_seen.as_ref().and_then(|key| key.thread.clone());
            Self::send(cx, AgentsIntent::NeedUsage(thread));
        }
    }

    /// The cursor, the keys' focus and the usage request follow the rows the
    /// model and the tab show.
    fn sync(&mut self, model: &Model, cx: &mut Cx<'_, Self>) -> Center {
        let center = model.center_at(self.cursor().as_ref(), &self.lens());
        if let Some(entry) = center.selected_entry() {
            self.put_cursor(view::key_of(&center, entry));
        } else if center.entries.is_empty() && self.selection.cursor().is_some() {
            // Oracle (`reconcile_command_center_selection`): nothing
            // selectable sets the selection to `usize::MAX`, so the first
            // row is the highlight when rows come back.
            self.selection.set_cursor(None);
        }
        self.ask_usage(model, &center, cx);
        center
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        self.listening = Some(Listening::new(cx, self.selection.listenable(), Msg::Cursor));
        if w.open {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::Open));
        }
    }

    fn dispose(&mut self) {
        if let Some(listening) = &self.listening {
            listening.cancel();
        }
        self.selection.dispose();
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let agents = watched(cx);
        let model = &*agents;
        match self.entered {
            None => self.entered = Some(model.resumes),
            Some(entered) if model.resumes > entered => {
                // A resume was answered: the chat shows the thread now.
                Navigator::maybe_pop(cx);
            }
            Some(_) => {}
        }
        // The list's cursor is the only copy of the highlight; a cursor whose
        // row left the list moves to the default row.
        if model.search != self.search_seen {
            self.search_seen = model.search.clone();
            let center = model.center_at(None, &self.lens());
            if let Some(index) = center
                .entries
                .iter()
                .position(|entry| matches!(entry, CenterRow::Task(_)))
            {
                self.put_cursor(view::key_of(&center, index));
            }
        }
        self.sync(model, cx);
        if model.failure.is_some() != self.failing {
            self.failing = model.failure.is_some();
            if self.failing {
                // The refusal's view replaces the screen's content (the
                // build draws none while it shows) at the top left.
                drop(show_dialog::<(), _>(
                    cx,
                    |_| dialogs::FailureDialog.into(),
                    DialogOptions {
                        alignment: Alignment::TOP_LEFT,
                        barrier_dismissible: false,
                        barrier_alpha: Some(0),
                        fade_in: Duration::ZERO,
                        ..DialogOptions::default()
                    },
                ));
            }
        }
    }

    fn update(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let agents = read::<ScreenStates, _>(cx).agents.clone();
        let model = &*agents;
        // The list's cursor is read here, when the key or click arrives: it
        // moves at once, the model only at the flush.
        let selected = self.selection.cursor();
        let at = selected.clone().and_then(view::RowKey::into_sel);
        let center = model.center_at(at.as_ref(), &self.lens());
        match msg {
            Msg::Act(act) => {
                let editing = self.editing_now();
                let task = || match &at {
                    Some(Sel::Task(id)) => Some(id.clone()),
                    _ => None,
                };
                match act {
                    Act::Interrupt => {
                        // `on_ctrl_c` takes the key while a line is open (it
                        // cancels it) or a removal runs; otherwise it is the
                        // app's: quit.
                        if editing {
                            self.cancel_editing(model, &center, cx);
                            return Rebuild::Yes;
                        }
                        if !matches!(
                            model.notice,
                            Some(codex_tui_core::screens::agents::Notice::Removing(_))
                        ) {
                            let hs = read::<Hs, _>(cx);
                            cx.emit(&hs.quit, ());
                        }
                    }
                    Act::Escape if self.mode == Mode::Help => {
                        self.mode = Mode::Browse;
                        return Rebuild::Yes;
                    }
                    Act::Escape if editing => {
                        self.cancel_editing(model, &center, cx);
                        return Rebuild::Yes;
                    }
                    Act::Escape => {
                        if w.can_return {
                            Navigator::maybe_pop(cx);
                        }
                    }
                    Act::Step { forward } => {
                        if let Some(from) = center.selected_entry()
                            && let Some(sel) = view::neighbour(&center, from, forward)
                        {
                            self.put_cursor(view::RowKey::from(sel));
                            return Rebuild::Yes;
                        }
                    }
                    Act::Filter { back } => {
                        let count = view::FILTERS;
                        self.filter = (self.filter + if back { count - 1 } else { 1 }) % count;
                        self.sync(model, cx);
                        return Rebuild::Yes;
                    }
                    Act::Help => {
                        self.mode = if self.mode == Mode::Help {
                            Mode::Browse
                        } else {
                            Mode::Help
                        };
                        return Rebuild::Yes;
                    }
                    Act::Search => {
                        if !matches!(self.mode, Mode::Rename(_)) {
                            self.set_mode(Mode::Search, &center);
                            return Rebuild::Yes;
                        }
                    }
                    Act::Rename => {
                        if let Some(id) = task()
                            && !model.held()
                            && model.row(&id).is_some()
                        {
                            Self::send(cx, AgentsIntent::ClearSearch);
                            self.set_mode(Mode::Rename(id), &center);
                            return Rebuild::Yes;
                        }
                    }
                    Act::Archive | Act::Delete => {
                        if let Some(thread) = task()
                            && !model.held()
                            && model.row(&thread).is_some()
                        {
                            let action = if act == Act::Archive {
                                Action::Archive
                            } else {
                                Action::Delete
                            };
                            self.ask_removal(model, view::Confirm { thread, action }, cx);
                        }
                    }
                    Act::Right => {
                        if let Some(sel) = at {
                            self.accept(model, sel, &center, cx);
                        }
                    }
                    Act::Open => {
                        if let Some(sel) = at {
                            Self::send(cx, AgentsIntent::Accept(sel));
                        }
                    }
                    Act::Grouping => Self::send(cx, AgentsIntent::CycleGrouping),
                    Act::New => Self::send(cx, AgentsIntent::New(task())),
                    Act::NewWorktree => Self::send(cx, AgentsIntent::NewWorktree(task())),
                    Act::Hide | Act::Pin | Act::Stop | Act::Fork => {
                        if let Some(id) = task() {
                            if act == Act::Hide {
                                self.after_removal(model, &id);
                            }
                            Self::send(
                                cx,
                                match act {
                                    Act::Hide => AgentsIntent::Hide(id),
                                    Act::Pin => AgentsIntent::TogglePin(id),
                                    Act::Stop => AgentsIntent::Stop(id),
                                    _ => AgentsIntent::Fork(id),
                                },
                            );
                        }
                    }
                }
            }
            Msg::Cursor => {
                if let Some(l) = &self.listening {
                    l.handled();
                }
                // The cursor is the list's own: only the rows that show it
                // (details, keys) are rebuilt; the usage of the task it
                // lands on is asked for at once.
                self.ask_usage(model, &center, cx);
                return Rebuild::Yes;
            }
            Msg::Row(key) => {
                // A click on the row that is highlighted opens it, on another
                // one highlights it (the list's cursor follows the model).
                if selected.as_ref() == Some(&key) {
                    if let Some(sel) = at {
                        self.accept(model, sel, &center, cx);
                        return Rebuild::Yes;
                    }
                } else if key.clone().into_sel().is_some()
                    && !matches!(self.mode, Mode::Rename(_))
                {
                    self.put_cursor(key);
                    return Rebuild::Yes;
                }
            }
            Msg::Open => {
                if let Some(sel) = at {
                    self.accept(model, sel, &center, cx);
                    return Rebuild::Yes;
                }
            }
            Msg::Confirmed(confirm) => {
                self.removal(model, confirm, cx);
                return Rebuild::Yes;
            }
            Msg::Search(text) => {
                if self.mode == Mode::Search {
                    let lens = self.lens();
                    Self::send(cx, AgentsIntent::SetSearch { text, lens });
                }
            }
            Msg::Submit(text) => {
                if let Mode::Rename(thread) = self.mode.clone() {
                    // An empty name keeps the line open.
                    if !text.trim().is_empty() {
                        Self::send(cx, AgentsIntent::Rename { thread, name: text });
                        self.set_mode(Mode::Browse, &center);
                        return Rebuild::Yes;
                    }
                } else if let Some(sel) = at {
                    self.accept(model, sel, &center, cx);
                    return Rebuild::Yes;
                }
            }
            Msg::Tab(index) => {
                self.filter = index.min(view::FILTERS - 1);
                self.sync(model, cx);
                return Rebuild::Yes;
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let agents = watched(cx);
        let look = view::Look::new(&Theme::of(cx), agents.thread_colors());
        let center = Rc::new(agents.center_at(self.cursor().as_ref(), &self.lens()));
        let ui = self.ui();
        // The row handlers are kept while the rows are the same ones: a
        // handler made anew is never equal to the last, and the list would
        // not compare equal (a cursor that moves leaves it as it is).
        let keys: Vec<view::RowKey> = (0..center.entries.len())
            .map(|index| view::key_of(&center, index))
            .collect();
        let rows = {
            let mut kept = self.row_taps.borrow_mut();
            match &*kept {
                Some((was, taps)) if *was == keys => taps.clone(),
                _ => {
                    let taps = Rc::new(
                        keys.iter()
                            .map(|key| {
                                let key = key.clone();
                                cx.map(move |_: PointerEvent| Msg::Row(key.clone()))
                            })
                            .collect::<Vec<_>>(),
                    );
                    *kept = Some((keys, taps.clone()));
                    taps
                }
            }
        };
        let taps = view::Taps {
            rows,
            tabs: (0..5)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Tab(index)))
                .collect(),
        };
        let controls = view::Controls {
            selection: self.selection.clone(),
            edit: self.edit.clone(),
            focus: self.list_focus.clone(),
            empty_focus: self.empty_focus.clone(),
            keys: view::Keys {
                browse: table(cx, BROWSE_KEYS),
                search: table(cx, SEARCH_KEYS),
                rename: table(cx, RENAME_KEYS),
                help: table(cx, HELP_KEYS),
            },
            edit_focus: self.edit_focus.clone(),
            scroll: self.scroll.clone(),
            open: rtwk::map_static!(cx, view::RowKey => Msg, |_| Msg::Open),
            search: cx.map(|text: Rc<str>| Msg::Search(text.to_string())),
            submit: cx.map(|text: Rc<str>| Msg::Submit(text.to_string())),
        };
        let can_return = w.can_return;
        let help_focus = self.help_focus.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let model = &*agents;
            if model.failure.is_some() {
                return AnyWidget::default();
            }
            // The header takes three rows from six, the footer one from two.
            let mut column = Column::new()
                .overflow_intended(true)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            if constraints.max_height >= 6 {
                column = column.child(view::header(&look, strings, &num, model, &ui, &center, &taps));
            }
            // Both stay mounted (the list keeps its cursor and scroll); the
            // help takes the room and the keys while it shows.
            // Both panes stay mounted, one offstage and out of the focus
            // (the list keeps its cursor and scroll); the help takes the
            // room and the keys while it shows.
            let help = ui.help;
            let body = view::body(&look, strings, &num, &agents, &ui, &center, &controls, &taps);
            let help_pane = KeyBindings::new(
                controls.keys.help.clone(),
                Focus::new(view::help(&look, strings, model, &center))
                    .focus_node(help_focus.clone()),
            );
            let stack = Stack::new()
                .fit(StackFit::Expand)
                .child(Offstage::new(help, ExcludeFocus::new(body).excluding(help)))
                .child(Offstage::new(
                    !help,
                    ExcludeFocus::new(help_pane).excluding(!help),
                ));
            column = column.child(Expanded::new(1, stack));
            if constraints.max_height >= 2 {
                column = column.child(view::footer_row(&look, strings, model, &ui, can_return));
            }
            column.into()
        });
        FocusScope::new(body).into()
    }
}
