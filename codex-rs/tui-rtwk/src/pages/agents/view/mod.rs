//! The command center's widgets, composed of rtwk primitives (`Row`,
//! `Column`, `Expanded`, `Padding`, `Divider`, `Text`, `SelectionList`,
//! `GestureDetector`): the header with its tab strip, the grouped rows, the
//! details pane, the footer. Every function takes what it draws (the core
//! `Center`, the strings, the look) and the handlers for the mouse; none
//! reads the model by itself and none counts a width: `Text` cuts and wraps,
//! the constraints give the room.
//! Oracle: `app/agent_center/{render,rows,hints}.rs`,
//! `app/agents_overview_view.rs` (`status`, `render_details`).

use std::rc::Rc;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::*;
use rtwk::AnyWidget;
use rtwk::BuildCx;
use rtwk::Handler;
use rtwk::LayoutBuilder;
use rtwk::StatelessWidget;
use rtwk::controls::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{ExcludeFocus, Focus, FocusNode, KeyBindings};
use rtwk::foundation::{ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{IgnorePointer, PointerEvent};
use rtwk::impl_widget;
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BorderType, BoxWidget, Center as Middle, Column, CrossAxisAlignment, Divider, EdgeInsets,
    Expanded, MainAxisSize, Offstage, Padding, Row, SizedBox, VerticalDivider,
};
use rtwk::overlay::Dialog;
use rtwk::scroll::ListController;
use rtwk::style::{Attrs, Span, Style, ThemeData};
use rtwk::text::{Text, TextAlign, TextWrap, segmenter_of};

use codex_tui_core::screens::Shared;
use codex_tui_core::screens::agents::{
    Action, Center, CenterRow, DetailLine, Failure, Group, GroupLabel, Grouping, Model, Notice,
    RowData, Sel, display_directory,
};

use super::help::{self, Ink};
use super::hints::Hints;
use super::path;
use super::strings::Strings;
use super::usage;
use crate::app::theme::ThreadPalette;
use crate::widgets::measure::{self as soft, Measure};
use crate::widgets::options::{OptionRow, OptionStyles, Options};
use crate::widgets::session_list::SessionList;
use crate::widgets::tab_strip::Tabs;
/// What the screen's own state decides the view shows (the model holds none
/// of it): the status tab, the help, and the editing line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ui {
    pub filter: usize,
    pub help: bool,
    pub searching: bool,
    /// A task's name is being edited.
    pub renaming: bool,
}

/// The number of status tabs.
pub const FILTERS: usize = 5;

impl Ui {
    pub fn editing(&self) -> bool {
        self.searching || self.renaming
    }
}

/// A removal waiting for the user's answer (oracle:
/// `confirm_agents_overview_action`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    pub thread: String,
    pub action: Action,
}

/// The styles of the screen, taken from the theme.
#[derive(Clone, PartialEq)]
pub struct Look {
    pub plain: Style,
    pub dim: Style,
    pub muted: Style,
    pub bold: Style,
    pub red: Style,
    pub green: Style,
    pub cyan: Style,
    pub blue: Style,
    pub selection: Style,
    pub tab: Style,
    pub accent: Style,
    /// The theme's accents for the per-task colours; `None` with
    /// `tui.status_line_use_colors` off.
    pub threads: Option<ThreadPalette>,
}

impl Look {
    pub fn new(data: &ThemeData, thread_colors: bool) -> Self {
        Self {
            plain: Style::new(),
            dim: data.dim(),
            muted: data.muted.clone(),
            bold: Style::new().attrs(Attrs::BOLD),
            red: data.error.clone(),
            green: data.success.clone(),
            cyan: data.info.clone(),
            blue: data.primary.clone(),
            selection: data.selection.clone(),
            tab: data.selection.clone().attrs(Attrs::BOLD),
            accent: data.accent.clone(),
            threads: thread_colors.then(|| ThreadPalette::of(data)),
        }
    }

    /// `base` in the colour of the thread `id`, when tasks have one.
    fn thread(&self, base: &Style, id: &str) -> Style {
        match self.threads.as_ref().and_then(|palette| palette.color(id)) {
            Some(color) => base.clone().fg(color),
            None => base.clone(),
        }
    }
}

type Line = Vec<Span>;

fn span(text: impl Into<String>, style: &Style) -> Span {
    Span::styled(text.into(), style.clone())
}

/// One row of spans, cut off with an ellipsis where the room ends.
fn one(spans: Line) -> AnyWidget {
    Text::rich_line(spans).into()
}

fn fixed(width: i32, child: impl Into<AnyWidget>) -> AnyWidget {
    SizedBox::new(Some(width), Some(1), Some(child.into())).into()
}

fn gap(width: i32) -> AnyWidget {
    SizedBox::fixed(width, 1).into()
}

fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}

/// A row that leaves its last cell free.
fn short(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::only(0, 0, 1, 0), child).into()
}

fn top(child: impl Into<AnyWidget>) -> AnyWidget {
    Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(child)
        .into()
}

fn stacked(rows: Vec<AnyWidget>) -> AnyWidget {
    let mut column = Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for row in rows {
        column = column.child(row);
    }
    column.into()
}

/// The details pane's width and the gap before it; the two panes show from
/// `TWO_PANES` cells of body (oracle: `center_layout`).
const DETAILS_WIDTH: usize = 38;
const GAP_WIDTH: i32 = 3;
const TWO_PANES: i32 = 90;
/// The hint of the filter strip shows from this width of header.
const KEYS_FROM: i32 = 80;
/// The list shows the status and age columns from this width.
const METADATA: i32 = 57;
const STATUS_COLUMN: i32 = 11;
const AGE_COLUMN: i32 = 9;
/// The width of the voice badge that ends a title.
const BADGE: i32 = 8;

/// How a row of the list is told apart: the list keeps its cursor by it.
/// Headings and gaps are never the cursor's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowKey {
    Gap(usize),
    Group(usize),
    Task(String),
    ShowMore,
}

impl From<Sel> for RowKey {
    fn from(sel: Sel) -> Self {
        match sel {
            Sel::Task(id) => Self::Task(id),
            Sel::ShowMore => Self::ShowMore,
        }
    }
}

impl RowKey {
    /// The highlight this key stands for; none for a heading or a gap.
    pub fn into_sel(self) -> Option<Sel> {
        match self {
            Self::Task(id) => Some(Sel::Task(id)),
            Self::ShowMore => Some(Sel::ShowMore),
            Self::Gap(_) | Self::Group(_) => None,
        }
    }

    pub fn selectable(&self) -> bool {
        matches!(self, Self::Task(_) | Self::ShowMore)
    }
}

/// The key of entry `index` of the list.
pub fn key_of(center: &Center, index: usize) -> RowKey {
    match &center.entries[index] {
        CenterRow::Gap => RowKey::Gap(index),
        CenterRow::Group { .. } => RowKey::Group(index),
        CenterRow::Task(at) => RowKey::Task(center.rows[*at].id().to_owned()),
        CenterRow::ShowMore => RowKey::ShowMore,
    }
}

pub use crate::widgets::session_list::Scrolled;

/// What the widget keeps for rtwk's widgets: the controllers and the
/// handlers of the lines they tell.
#[derive(Clone)]
pub struct Controls {
    pub selection: SelectionController<RowKey>,
    pub list: ListController,
    pub edit: TextEditingController,
    pub focus: FocusNode,
    /// The stop of the empty list's line (it holds the keys when no row
    /// does).
    pub empty_focus: FocusNode,
    pub edit_focus: FocusNode,
    /// The keys each mode's widget binds.
    pub keys: Keys,
    pub scroll: Rc<ValueNotifier<Scrolled>>,
    pub search: Handler<Rc<str>>,
    pub submit: Handler<Rc<str>>,
}

/// A key table's handlers, name and handler (`KeyBindings`).
pub type Table = Vec<(String, Handler<()>)>;

/// The key tables of the modes: each binds on the widget the mode shows, so
/// the focus decides which one the key reaches.
#[derive(Clone)]
pub struct Keys {
    pub browse: Table,
    pub search: Table,
    pub rename: Table,
    pub help: Table,
}

/// The part of the controls the list takes.
#[derive(Clone)]
struct ListControls {
    selection: SelectionController<RowKey>,
    list: ListController,
    focus: FocusNode,
    empty_focus: FocusNode,
    scroll: Rc<ValueNotifier<Scrolled>>,
}

/// What the screen is told by a click: the handlers the widget made.
pub struct Taps {
    pub rows: Rc<Vec<Handler<PointerEvent>>>,
    pub tabs: Vec<Handler<PointerEvent>>,
}

mod details;
mod dialogs;
mod footer;
mod header;
mod list;

pub use details::details;
pub use dialogs::{Ask, confirm_dialog, confirm_name, failure_dialog, help};
pub use footer::{body, footer_row};
pub use header::header;
pub use list::list;
