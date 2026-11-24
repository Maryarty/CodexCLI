//! `agents` (S21): the view half of the agents command center. One
//! `PageRoute` over the model (`ScreenStates.agents`): the header with its
//! tab strip, the grouped task list (a `ListView`, scrolled by item) beside
//! the details of the highlighted task, the key hints. Its model, events,
//! intents and RPCs are core's `codex_tui_core::screens::agents`; this file
//! holds the keys, the mouse and the widget state, `view.rs` the widgets.
//! Oracle: `tui/src/app/agent_center/` and `app/agents_overview_view.rs`.

use std::rc::Rc;
use std::time::Duration;

use rtwk::controls::{SelectionController, TabController};
use rtwk::scroll::ListController;

use crate::widgets::session_list::{Move, page_rows, target};
use rtwk::editing::TextEditingController;
use rtwk::focus::{ExcludeFocus, Focus, FocusNode, FocusScope, KeyBindings};
use rtwk::foundation::{Listening, ValueNotifier};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{Alignment, Column, CrossAxisAlignment, Expanded, Offstage, Stack, StackFit};
use rtwk::navigation::{Navigator, PageRoute, PopInvoked, PopScope, Route};
use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::{read, select, watch};
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::Intent as AgentsIntent;
use codex_tui_core::screens::agents::{Action, Center, CenterRow, CenterThreads, Lens, Model, Sel};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, Shared};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::view::Hs;
use crate::l10n::strings as shared;

use hints::Hints;

mod debug;
mod dialogs;
mod help;
mod hints;
mod keys;
mod opening;
pub(crate) mod path;
mod state;
pub mod strings;
mod usage;
mod view;

/// `/subagents`: the picker of this session's subagent threads (oracle
/// `AppEvent::OpenAgentPicker`), a sheet over the chat.
pub fn open_picker<S: 'static>(cx: &mut Cx<'_, S>) {
    crate::sheets::select::agent_picker::open(cx);
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

/// The route of the command center with nowhere to go back to (the trust
/// prompt cancelled at launch opens it in place of the folder): a session
/// opened from it pops the center and shows the chat.
pub fn start_route() -> PageRoute<()> {
    PageRoute::new(|_| {
        Screen {
            can_return: false,
            open: true,
        }
        .into()
    })
}

/// `codex-rtwk agents`: as `start_route`, and the center tells `left` when it
/// is gone (Flutter: `PopScope.onPopInvokedWithResult`): `app::start_page::
/// StartPage` mounts the chat then, so its composer takes the focus.
pub fn start_route_then(left: Handler<PopInvoked>) -> PageRoute<()> {
    PageRoute::new(move |_| {
        PopScope::new(Screen {
            can_return: false,
            open: true,
        })
        .on_pop_invoked_with_result(left.clone())
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
enum CenterMode {
    Browse,
    Help,
    Search,
    Rename(String),
}

/// Widget state: the controllers rtwk's widgets keep the list's cursor,
/// the editing line's text, the list's focus and where it is scrolled in.
pub struct ScreenState {
    selection: SelectionController<view::RowKey>,
    list: ListController,
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
    tabs: TabController,
    scroll: Rc<ValueNotifier<view::Scrolled>>,
    /// The row handlers made for the rows the list last showed.
    row_taps: Option<(Vec<view::RowKey>, RowTaps)>,
    /// The handlers of the tab clicks, made once.
    tab_taps: Vec<Handler<PointerEvent>>,
    listening: Option<Listening>,
    mode: CenterMode,
    /// What the usage request of the highlighted task depends on, as the
    /// screen last saw it: a change asks the model for it (oracle: the
    /// repaint of the view asks).
    usage_seen: Option<UsageKey>,
    /// How many resumes the model had answered when the screen opened: one
    /// more means a session was opened from here.
    entered: Option<u32>,
    /// The loading page of a task being opened is on top of this screen.
    opening_pushed: bool,
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
    /// The confirmation dialog was answered yes: the removal.
    Confirmed(view::Confirm),
    /// The loading page of an opening was popped.
    OpeningClosed,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selection: SelectionController::new(),
            list: ListController::new(),
            edit: TextEditingController::default(),
            list_focus: FocusNode::new(),
            empty_focus: FocusNode::new(),
            help_focus: FocusNode::new(),
            edit_focus: FocusNode::new(),
            failing: false,
            tabs: TabController::new(view::FILTERS),
            scroll: Rc::new(ValueNotifier::new(view::Scrolled::default())),
            row_taps: None,
            tab_taps: Vec::new(),
            listening: None,
            mode: CenterMode::Browse,
            usage_seen: None,
            entered: None,
            opening_pushed: false,
            search_seen: String::new(),
            browse: None,
        }
    }
}

impl_widget!(stateful eq Screen);

pub use debug::debug_entries;
pub use keys::Act;
