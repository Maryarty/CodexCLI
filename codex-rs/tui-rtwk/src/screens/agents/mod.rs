//! `agents` (S21): the view half of the agents command center. One
//! `PageRoute` over the model (`ScreenStates.agents`): the header with its
//! tab strip, the grouped task list (a `ListView`, scrolled by item) beside
//! the details of the highlighted task, the key hints. Its model, events,
//! intents and RPCs are core's `codex_tui_core::screens::agents`; this file
//! holds the keys, the mouse and the widget state, `view.rs` the widgets.
//! Oracle: `tui/src/app/agent_center/` and `app/agents_overview_view.rs`.

use std::rc::Rc;

use rtwk::binding::AppBinding;
use rtwk::controls::SelectionController;
use rtwk::editing::TextEditingController;
use rtwk::focus::{Focus, FocusNode, KeyPress};
use rtwk::foundation::{Listening, ValueNotifier};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{Stack, StackFit};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, watch};
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::Intent as AgentsIntent;
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod layout;
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

/// `codex-rtwk agents`: the chat is the navigator's home as ever, and the
/// command center is pushed over it at once with nowhere to go back to; a
/// session opened from it pops the center and shows the chat.
pub fn standalone(chat: AnyWidget) -> AnyWidget {
    Standalone { chat }.into()
}

#[derive(Clone, Debug, PartialEq)]
struct Standalone {
    chat: AnyWidget,
}

struct StandaloneState;

impl StatefulWidget for Standalone {
    type State = StandaloneState;

    fn create_state(&self) -> StandaloneState {
        StandaloneState
    }
}

impl_widget!(stateful eq Standalone);

impl State for StandaloneState {
    type Widget = Standalone;
    type Msg = ();

    fn init_state(&mut self, _: &Standalone, cx: &mut Cx<'_, Self>) {
        let push = cx.send(());
        cx.emit(&push, ());
    }

    fn update(&mut self, _: (), _: &Standalone, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(|_| {
            Screen {
                can_return: false,
                open: true,
            }
            .into()
        })));
        Rebuild::No
    }

    fn build(&self, w: &Standalone, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.chat.clone()
    }
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

/// What the editing line is for; the controller's text is reset when it
/// changes (a rename starts from the task's name, a search from nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Editing {
    No,
    Search,
    Rename(String),
}

/// Widget state: the controllers rtwk's widgets keep the list's cursor,
/// the editing line's text, the list's focus and where it is scrolled in.
pub struct ScreenState {
    selection: SelectionController<view::RowKey>,
    edit: TextEditingController,
    list_focus: FocusNode,
    /// The screen's own focus: the keys have it while the help replaces the list.
    root_focus: FocusNode,
    edit_focus: FocusNode,
    /// The confirmation's two rows and the focus they take while it shows.
    confirm_selection: SelectionController<usize>,
    confirm_focus: FocusNode,
    confirming: bool,
    /// The failure dialog shows (it takes the focus).
    failing: bool,
    helping: bool,
    scroll: Rc<ValueNotifier<view::Scrolled>>,
    listening: Option<Listening>,
    editing: Editing,
    /// What the usage request of the highlighted task depends on, as the
    /// screen last saw it: a change asks the model for it (oracle: the
    /// repaint of the view asks).
    usage_seen: Option<UsageKey>,
    /// How many resumes the model had answered when the screen opened: one
    /// more means a session was opened from here.
    entered: Option<u32>,
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
    Key(KeyPress),
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
    /// The mode changed (help, editing): the keys go to what shows now.
    Refocus,
    /// Enter on a row of the confirmation.
    ConfirmPick(usize),
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selection: SelectionController::new(),
            edit: TextEditingController::default(),
            list_focus: FocusNode::new(),
            root_focus: FocusNode::new(),
            edit_focus: FocusNode::new(),
            confirm_selection: SelectionController::new(),
            confirm_focus: FocusNode::new(),
            confirming: false,
            failing: false,
            helping: false,
            scroll: Rc::new(ValueNotifier::new(view::Scrolled::default())),
            listening: None,
            editing: Editing::No,
            usage_seen: None,
            entered: None,
        }
    }
}

impl_widget!(stateful eq Screen);

fn on_key(key: KeyPress) -> Option<Msg> {
    (key.code != KeyCode::F12).then_some(Msg::Key(key))
}

/// The intent a key means on the screen (oracle: `handle_key_event`,
/// `command_center_key`, `rename_key` with the default keymap), or `None`
/// for a key the screen does not take. The list moves its own cursor by the
/// arrows, the page keys, Home and End; the editing line takes the text.
fn intent_of(key: &KeyPress, editing: bool, help: bool) -> Option<AgentsIntent> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    let shift = key.mods.contains(Mods::SHIFT);
    let text = key.as_text().filter(|_| !ctrl && !alt);
    if help {
        return match key.code {
            KeyCode::Escape => Some(AgentsIntent::CloseHelp),
            _ if text == Some("?") => Some(AgentsIntent::CloseHelp),
            _ => None,
        };
    }
    match key.code {
        KeyCode::Escape => return Some(AgentsIntent::CancelEditing),
        KeyCode::Tab if !editing => return Some(AgentsIntent::CycleFilter { back: shift }),
        KeyCode::F3 => return Some(AgentsIntent::BeginSearch),
        _ => {}
    }
    if ctrl {
        return (key.ch == Some('c')).then_some(AgentsIntent::CancelEditing);
    }
    if editing {
        return None;
    }
    match (key.code, text) {
        (KeyCode::Backspace, _) => Some(AgentsIntent::Delete),
        (KeyCode::Right, _) if !shift => Some(AgentsIntent::Right),
        (_, Some("?")) => Some(AgentsIntent::ToggleHelp),
        (_, Some("g")) => Some(AgentsIntent::CycleGrouping),
        (_, Some("/")) => Some(AgentsIntent::BeginSearch),
        (_, Some("n")) => Some(AgentsIntent::New),
        (_, Some("r")) => Some(AgentsIntent::BeginRename),
        (_, Some("a")) => Some(AgentsIntent::Archive),
        (_, Some("h")) => Some(AgentsIntent::Hide),
        (_, Some("p")) => Some(AgentsIntent::TogglePin),
        (_, Some("x")) => Some(AgentsIntent::Stop),
        (_, Some("o")) => Some(AgentsIntent::Accept),
        (_, Some("f")) => Some(AgentsIntent::Fork),
        (_, Some("w")) => Some(AgentsIntent::NewWorktree),
        _ => None,
    }
}

/// Whether a key steps the highlight (`Some(true)` down): the list keymap's
/// defaults beyond the arrows (oracle: `list.move_up`, `list.move_down`).
fn step_of(key: &KeyPress) -> Option<bool> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    match (key.ch, ctrl, alt) {
        (Some('j' | 'n'), true, false) => Some(true),
        (Some('k' | 'p'), true, false) => Some(false),
        (Some('j'), false, false) => Some(true),
        (Some('k'), false, false) => Some(false),
        _ => None,
    }
}

impl ScreenState {
    /// The keys go to what the mode shows once the frame that shows it is
    /// out: the widget a mode drops lets go of the focus with the frame, and
    /// the one it adds (or frees from `ExcludeFocus`) cannot take it before.
    fn refocus_after_frame(&self, cx: &mut Cx<'_, Self>) {
        let refocus = cx.map(|_: std::time::Duration| Msg::Refocus);
        AppBinding::of(cx).add_post_frame_callback(refocus);
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
        self.confirm_selection.dispose();
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.agents;
        match self.entered {
            None => self.entered = Some(model.resumes),
            Some(entered) if model.resumes > entered => {
                // A resume was answered: the chat shows the thread now.
                Navigator::maybe_pop(cx);
            }
            Some(_) => {}
        }
        let center = model.center();
        // The highlighted task is core's data; the list's cursor follows it.
        if let Some(entry) = center.selected_entry() {
            let key = view::key_of(&center, entry);
            if self.selection.cursor().as_ref() != Some(&key) {
                self.selection.set_cursor(Some(key.clone()));
                self.selection.select(key);
            }
        }
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
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::NeedUsage));
        }
        if model.confirm.is_some() != self.confirming {
            self.confirming = model.confirm.is_some();
            if self.confirming {
                self.confirm_selection.set_cursor(Some(0));
            }
            self.refocus_after_frame(cx);
        }
        if model.failure.is_some() != self.failing {
            self.failing = model.failure.is_some();
            self.refocus_after_frame(cx);
        }
        if model.help != self.helping {
            self.helping = model.help;
            self.refocus_after_frame(cx);
        }
        let editing = match (&model.rename, model.searching) {
            (Some(rename), _) => Editing::Rename(rename.target.clone()),
            (None, true) => Editing::Search,
            (None, false) => Editing::No,
        };
        if editing != self.editing {
            match &editing {
                Editing::Search => {
                    self.edit.reset("");
                }
                Editing::Rename(target) => {
                    let name = center
                        .rows
                        .iter()
                        .find(|row| row.id() == target.as_str())
                        .and_then(|row| row.thread.name.clone())
                        .unwrap_or_default();
                    self.edit.reset(&name);
                }
                Editing::No => {}
            }
            self.editing = editing;
            self.refocus_after_frame(cx);
        }
    }

    fn update(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.agents;
        let hs = read::<Hs, _>(cx);
        let mut send = |intent: AgentsIntent| cx.emit(&hs.screen, ScreenIntent::Agents(intent));
        let center = model.center();
        let selected = center
            .selected_entry()
            .map(|entry| view::key_of(&center, entry));
        match msg {
            Msg::Key(key) => {
                let editing = model.searching || model.rename.is_some();
                if model.failure.is_some() {
                    // The refusal's dialog has one row, `Back to agents`
                    // (oracle: a selection view with `dismiss_on_select`).
                    if matches!(key.code, KeyCode::Escape | KeyCode::Enter)
                        || key.as_text().is_some_and(|text| text == "1")
                    {
                        send(AgentsIntent::DismissFailure);
                    }
                    return Rebuild::No;
                }
                if let Some(confirm) = &model.confirm {
                    // The confirmation takes every key (oracle: the view on
                    // top of the bottom pane): Esc and the digits answer, the
                    // arrows and Enter are the list's.
                    match (
                        key.code,
                        key.as_text().filter(|_| !key.mods.contains(Mods::CTRL)),
                    ) {
                        (KeyCode::Escape, _) => send(AgentsIntent::Confirmed(false)),
                        (_, Some(digit @ ("1" | "2"))) => {
                            let index = usize::from(digit == "2");
                            // A delete waits for Enter: the digit only moves the cursor.
                            if index == 1
                                && confirm.action == codex_tui_core::screens::agents::Action::Delete
                            {
                                self.confirm_selection.set_cursor(Some(1));
                            } else {
                                send(AgentsIntent::Confirmed(index == 1));
                            }
                        }
                        _ => {}
                    }
                    return Rebuild::No;
                }
                if key.code == KeyCode::Escape && !editing && !model.help {
                    if w.can_return {
                        Navigator::maybe_pop(cx);
                    }
                    return Rebuild::No;
                }
                // While the search line has the focus, the arrows that bubble
                // up from it move the highlight (oracle: the list actions
                // stay live under the search line).
                if model.searching && model.rename.is_none() {
                    let forward = match key.code {
                        KeyCode::Down => Some(true),
                        KeyCode::Up => Some(false),
                        _ => None,
                    };
                    if let (Some(forward), Some(at)) = (forward, center.selected_entry())
                        && let Some(sel) = view::neighbour(&center, at, forward)
                    {
                        send(AgentsIntent::Select(sel));
                        return Rebuild::No;
                    }
                }
                // The list keymap's other defaults for a step: `j`/`k` and
                // Ctrl with N/P/J/K (the arrows are the list's own).
                if !editing
                    && !model.help
                    && let Some(forward) = step_of(&key)
                    && let Some(at) = center.selected_entry()
                    && let Some(sel) = view::neighbour(&center, at, forward)
                {
                    send(AgentsIntent::Select(sel));
                    return Rebuild::No;
                }
                if let Some(intent) = intent_of(&key, editing, model.help) {
                    send(intent);
                }
            }
            Msg::Refocus => {
                if model.failure.is_some() {
                    self.root_focus.request_focus();
                } else if model.confirm.is_some() {
                    self.confirm_focus.request_focus();
                } else if model.help {
                    self.root_focus.request_focus();
                } else if model.searching || model.rename.is_some() {
                    self.edit_focus.request_focus();
                } else {
                    self.list_focus.request_focus();
                }
                return Rebuild::No;
            }
            Msg::Cursor => {
                if let Some(l) = &self.listening {
                    l.handled();
                }
                let cursor = self.selection.cursor();
                if cursor != selected
                    && let Some(sel) = cursor.and_then(view::RowKey::into_sel)
                {
                    send(AgentsIntent::Select(sel));
                }
            }
            Msg::Row(key) => {
                // A click on the row that is highlighted opens it, on another
                // one highlights it (the list's cursor follows the model).
                if selected.as_ref() == Some(&key) {
                    send(AgentsIntent::Accept);
                } else if let Some(sel) = key.into_sel() {
                    send(AgentsIntent::Select(sel));
                }
            }
            Msg::Open => send(AgentsIntent::Accept),
            Msg::ConfirmPick(index) => send(AgentsIntent::Confirmed(index == 1)),
            Msg::Search(text) => send(AgentsIntent::SetSearch(text)),
            Msg::Submit(text) => {
                if model.rename.is_some() {
                    send(AgentsIntent::Commit(text));
                } else {
                    send(AgentsIntent::Accept);
                }
            }
            Msg::Tab(index) => send(AgentsIntent::SetFilter(index)),
        }
        Rebuild::No
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let look = view::Look::new(&Theme::of(cx));
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let center = Rc::new(screens.agents.center());
        let taps = view::Taps {
            rows: Rc::new(
                (0..center.entries.len())
                    .map(|index| {
                        let key = view::key_of(&center, index);
                        cx.map(move |_: PointerEvent| Msg::Row(key.clone()))
                    })
                    .collect(),
            ),
            tabs: (0..5)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Tab(index)))
                .collect(),
        };
        let controls = view::Controls {
            selection: self.selection.clone(),
            edit: self.edit.clone(),
            focus: self.list_focus.clone(),
            edit_focus: self.edit_focus.clone(),
            scroll: self.scroll.clone(),
            open: cx.map(|_: view::RowKey| Msg::Open),
            search: cx.map(|text: Rc<str>| Msg::Search(text.to_string())),
            submit: cx.map(|text: Rc<str>| Msg::Submit(text.to_string())),
            confirm_selection: self.confirm_selection.clone(),
            confirm_focus: self.confirm_focus.clone(),
            confirm_pick: cx.map(Msg::ConfirmPick),
        };
        let can_return = w.can_return;
        let body = LayoutBuilder::new(move |_, constraints| {
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = usize::try_from(constraints.max_height).unwrap_or(0);
            let model = &screens.agents;
            if let Some(failure) = &model.failure {
                return view::failure_dialog(&look, strings, failure, width);
            }
            let areas = layout::areas(width, height);
            let mut column = rtwk::layout::Column::new()
                .cross_axis_alignment(rtwk::layout::CrossAxisAlignment::Start);
            if areas.header > 0 {
                column = column.child(view::header(
                    &look, strings, &num, model, &center, width, &taps,
                ));
            }
            if model.help {
                column = column.child(view::help(&look, strings, model, &center, &areas, width));
            } else {
                column = column.child(view::body(
                    &look, strings, &num, &screens, &center, &areas, &controls, &taps,
                ));
            }
            if areas.footer_row < height {
                column = column.child(view::footer_row(&look, strings, model, can_return, width));
            }
            let Some(confirm) = &model.confirm else {
                return column.into();
            };
            Stack::new()
                .fit(StackFit::Expand)
                .child(column)
                .child(view::confirm_dialog(
                    &look,
                    strings,
                    &num,
                    model,
                    &center,
                    confirm,
                    (width, height),
                    &controls,
                ))
                .into()
        });
        Focus::new(body)
            .focus_node(self.root_focus.clone())
            .on_key(cx.keys(on_key))
            .into()
    }
}
