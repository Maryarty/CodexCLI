//! `import` (S06): the view half of the import from other agents, a step of
//! the startup page (`onboarding::Step::Import`). The flow, its requests and
//! its choices are core's `codex_tui_core::screens::import`; this directory
//! draws what the model says: the source prompt (`source::SourcePrompt`, the
//! startup prompts' look over the shared `Options`) and the
//! review, a summary and a checklist of the detected items with the actions
//! under them. Oracle: `external_agent_config_migration/{mod,render,source}.rs`.

use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::focus::{Focus, FocusNode, KeyBindings};
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible, MainAxisSize, Padding, SizedBox,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::import::{self as import, Intent as ImportIntent, Mark, Phase};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::widgets::dots::dots_end;
use crate::widgets::session_list::{Move, target};

pub mod labels;
pub mod notices;
pub mod rows;
pub mod source;
pub mod strings;

use rows::ReviewRow;
use strings::Strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "source_prompt",
            description: "two sources have items: pick one",
            script: include_str!("fixtures/source_prompt.jsonl"),
            open: EntryOpen::Route(hosted),
        },
        DebugEntry {
            name: "review_with_memory",
            description: "five items, memory among them, all checked",
            script: include_str!("fixtures/review_with_memory.jsonl"),
            open: EntryOpen::Route(hosted),
        },
        DebugEntry {
            name: "review_cursor",
            description: "the one item of the second source",
            script: include_str!("fixtures/review_cursor.jsonl"),
            open: EntryOpen::Route(hosted),
        },
        DebugEntry {
            name: "review_sample",
            description: "four items, all checked",
            script: include_str!("fixtures/review_sample.jsonl"),
            open: EntryOpen::Route(hosted),
        },
        DebugEntry {
            name: "review_none_selected",
            description: "four items, none checked",
            script: include_str!("fixtures/review_none_selected.jsonl"),
            open: EntryOpen::Route(hosted),
        },
    ]
}

/// The step under a navigator of its own: the seeded model of a debug entry is
/// provided to the route, the selection sheet is a dialog of the nearest
/// navigator, and under the root one it would read the live model instead of
/// the seeded one (Flutter's nested `Navigator`).
fn hosted(_: &Seeded<'_>) -> AnyWidget {
    Navigator::new().home(Screen).into()
}

/// The route of the import step: a step of the startup page, not a page of
/// its own.
pub fn route() -> PageRoute<()> {
    crate::pages::startup::onboarding::route(crate::pages::startup::onboarding::Step::Import)
}

/// Opens the import on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// Which page of the review is up: the summary, or the checklist to customize.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Summary,
    Customize,
}

/// What an action row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    Proceed,
    Customize,
    Skip,
    Back,
}

/// The actions of the page, top to bottom: the summary offers to import only
/// while something is checked.
fn actions(view: View, checked: &[bool]) -> Vec<Act> {
    match view {
        View::Summary => {
            let mut actions = Vec::new();
            if checked.iter().any(|on| *on) {
                actions.push(Act::Proceed);
            }
            actions.extend([Act::Customize, Act::Skip]);
            actions
        }
        View::Customize => vec![Act::Back],
    }
}

/// The step: asks for the detection when the flow has not started, opens the
/// source prompt when the model wants a source, and draws the review.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state. The review is two regions, as a Flutter form of a checkbox
/// list over a button row would be: the checklist and the actions under it,
/// each a `Focus` node with its own keys and a `SelectionController` for its
/// cursor; what has the focus is the nodes' (`has_focus`), the checklist
/// scrolls with rtwk's `ListController`. The controllers are the only holders
/// of cursor and scroll; nothing of the model is kept here (R14: the page, the
/// controllers, the nodes, the review they are for).
#[derive(Debug)]
pub struct ScreenState {
    list: ListController,
    view: View,
    /// The review the page and the cursors are for, by its number.
    review_for: u64,
    /// The checklist's checks: the draft of the import (ADR-013: a toggle is the page's, the
    /// flow gets the checked indexes with `Proceed`), started from the model's preselection.
    checked: Vec<bool>,
    items: FocusNode,
    actions: FocusNode,
    /// The checklist's cursor (an index of the model's `items`, or of its
    /// `groups` in the summary) and the actions' (an index of `actions`).
    item: SelectionController<usize>,
    action: SelectionController<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The focus entered or left a region: the page is drawn again.
    Focus(bool),
    /// Up and Down in the checklist and in the actions: the cursor moves, past
    /// the end of one region the focus goes to the other.
    ItemUp,
    ItemDown,
    ActionUp,
    ActionDown,
    /// Space or Enter in the checklist.
    Toggle,
    /// Enter in the actions.
    Run,
    /// A digit: the action at that place.
    Number(u8),
    /// `c`: customize (summary only).
    Customize,
    /// `b`: back (checklist only).
    Back,
    /// `a` and `n` (checklist only).
    All(bool),
    /// Escape: cancel in the summary, back in the checklist.
    Escape,
    /// An intent of the flow (cancel).
    Intent(ImportIntent),
    /// A tap on the checklist's item (ADR-011): the cursor moves to it, a
    /// tap on the item under the cursor toggles it.
    TapItem(usize),
    /// A tap on an action: the cursor moves to it, a tap on the action under
    /// the cursor runs it.
    TapAction(usize),
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            list: ListController::new(),
            view: View::Summary,
            review_for: 0,
            checked: Vec::new(),
            items: FocusNode::new(),
            actions: FocusNode::new(),
            item: SelectionController::new(),
            action: SelectionController::new(),
        }
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        self.items.set_can_request_focus(false);
        let phase = read::<ScreenStates, _>(cx).import.phase;
        if matches!(phase, Phase::Idle | Phase::Done) {
            let hs = read::<Hs, _>(cx);
            cx.emit(
                &hs.screen,
                ScreenIntent::Import(ImportIntent::Open { cwd: None }),
            );
        }
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let model = flow(cx);
        if model.phase == Phase::Review {
            if self.review_for != model.review_id {
                // a review opens on its summary, the actions have the keys
                self.review_for = model.review_id;
                self.checked = model.preselected();
                self.show(View::Summary, &model);
            }
            self.settle(&model);
            self.track(&model, strings::of(cx), &shared::of(cx).num);
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = read::<ScreenStates, _>(cx);
        let model = &screens.import;
        let customize = self.view == View::Customize;
        let mut intent = None;
        match msg {
            Msg::Focus(_) => {}
            Msg::Intent(it) => intent = Some(it),
            Msg::ItemUp => self.walk(Region::Items, false, model),
            Msg::ItemDown => self.walk(Region::Items, true, model),
            Msg::ActionUp => self.walk(Region::Actions, false, model),
            Msg::ActionDown => self.walk(Region::Actions, true, model),
            Msg::Toggle => {
                if customize {
                    self.toggle(self.item.cursor().unwrap_or(0));
                }
            }
            Msg::Run => intent = self.run(model),
            Msg::TapItem(at) => {
                if !customize {
                } else if self.items.has_focus() && self.item.cursor() == Some(at) {
                    self.toggle(at);
                } else {
                    self.item.set_cursor(Some(at));
                    self.items.request_focus();
                }
            }
            Msg::TapAction(at) => {
                if self.actions.has_focus() && self.action.cursor() == Some(at) {
                    intent = self.run(model);
                } else {
                    self.action.set_cursor(Some(at));
                    self.actions.request_focus();
                }
            }
            Msg::Number(number) => {
                let at = usize::from(number).checked_sub(1);
                if let Some(at) = at.filter(|at| *at < actions(self.view, &self.checked).len()) {
                    self.action.set_cursor(Some(at));
                    self.actions.request_focus();
                    intent = self.run(model);
                }
            }
            Msg::Customize => {
                if !customize {
                    self.show(View::Customize, model);
                }
            }
            Msg::Back => {
                if customize {
                    self.show(View::Summary, model);
                }
            }
            Msg::All(on) => {
                if customize {
                    self.checked.iter_mut().for_each(|check| *check = on);
                }
            }
            Msg::Escape => {
                if customize {
                    self.show(View::Summary, model);
                } else {
                    intent = Some(ImportIntent::Cancel);
                }
            }
        }
        if let Some(intent) = intent {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Import(intent));
        }
        self.settle(model);
        self.track(model, strings::of(cx), &shared::of(cx).num);
        Rebuild::Yes
    }

    fn dispose(&mut self) {
        self.items.dispose();
        self.actions.dispose();
        self.item.dispose();
        self.action.dispose();
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let model = flow(cx);
        if model.phase == Phase::Source {
            return source::SourcePrompt.into();
        }
        if model.phase != Phase::Review || self.review_for != model.review_id {
            // The oracle draws nothing while it asks the server: the
            // screen below stays. Escape
            // ends a detection that has not answered (its answer is then
            // stale).
            let cancel = || cx.send(Msg::Intent(ImportIntent::Cancel));
            return KeyBindings::new(
                [("escape", cancel()), ("ctrl+c", cancel())],
                Focus::new(SizedBox::new(Some(0), Some(0), None)).autofocus(),
            )
            .into();
        }
        let tokens = theme::Tokens::of(cx);
        let look = Look {
            dim: tokens.dim().attrs(Attrs::DIM),
            bold: tokens.bold(),
            accent: tokens.accent(),
            selected: tokens.selected(),
            error: tokens.error(),
            section: tokens.dim().attrs(Attrs::DIM | Attrs::BOLD),
        };
        let go = |keys: &[&'static str], msg: Msg| -> Vec<(&'static str, Handler<()>)> {
            keys.iter()
                .map(|key| (*key, cx.send(msg.clone())))
                .collect()
        };
        let mut checklist = go(&["up", "k"], Msg::ItemUp);
        checklist.extend(go(&["down", "j"], Msg::ItemDown));
        checklist.extend(go(&["space", "enter"], Msg::Toggle));
        let mut buttons = go(&["up", "k"], Msg::ActionUp);
        buttons.extend(go(&["down", "j"], Msg::ActionDown));
        buttons.extend(go(&["enter"], Msg::Run));
        let mut page = go(&["c"], Msg::Customize);
        page.extend(go(&["b"], Msg::Back));
        page.extend(go(&["a"], Msg::All(true)));
        page.extend(go(&["n"], Msg::All(false)));
        page.extend(go(&["escape"], Msg::Escape));
        page.extend(go(&["ctrl+c", "ctrl+d"], Msg::Intent(ImportIntent::Cancel)));
        for digit in 1..=9u8 {
            page.push((DIGITS[usize::from(digit) - 1], cx.send(Msg::Number(digit))));
        }
        let regions = Regions {
            list: self.list.clone(),
            items: self.items.clone(),
            actions: self.actions.clone(),
            item: self.item.clone(),
            action: self.action.clone(),
            checklist,
            buttons,
            focus: (cx.map(Msg::Focus), cx.map(Msg::Focus)),
            item_taps: (0..self.item_count(&model))
                .map(|at| cx.map(move |_: PointerEvent| Msg::TapItem(at)))
                .collect(),
            action_taps: (0..actions(self.view, &self.checked).len())
                .map(|at| cx.map(move |_: PointerEvent| Msg::TapAction(at)))
                .collect(),
        };
        let body = review(
            &look,
            strings::of(cx),
            shared::of(cx).num,
            &model,
            &self.checked,
            self.view,
            regions,
        );
        KeyBindings::new(page, body).into()
    }
}

/// The flow's model, as the page draws it (a `select` of the import slice, not the whole
/// screen states).
fn flow(cx: &mut Cx<'_, ScreenState>) -> import::Model {
    select::<ScreenStates, import::Model, _>(cx, |states| states.import.clone())
}

const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// The two regions of the review.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Items,
    Actions,
}

impl ScreenState {
    /// How many entries the checklist has in the view: the groups of the
    /// summary, the items of the checklist.
    fn item_count(&self, model: &import::Model) -> usize {
        match self.view {
            View::Summary => model.groups.len(),
            View::Customize => model.items.len(),
        }
    }

    /// The page `view` is up, its first entry under the cursor: the checklist
    /// has the keys when it has entries (the oracle's `customize`), the
    /// actions otherwise.
    fn show(&mut self, view: View, model: &import::Model) {
        self.view = view;
        self.item.set_cursor(Some(0));
        self.action.set_cursor(Some(0));
        self.list.jump_to_index(0);
        let customize = view == View::Customize && !model.items.is_empty();
        self.items.set_can_request_focus(customize);
        if customize {
            self.items.request_focus();
        } else {
            self.actions.request_focus();
        }
    }

    /// Up or Down in `region`: the cursor goes to the next entry; past the end
    /// of the actions of the summary it goes round, past the end of either
    /// region of the checklist page the focus goes to the other one, at its
    /// first entry (forward) or its last (back).
    fn walk(&mut self, region: Region, forward: bool, model: &import::Model) {
        let (sel, len) = match region {
            Region::Items => (&self.item, self.item_count(model)),
            Region::Actions => (&self.action, actions(self.view, &self.checked).len()),
        };
        let from = sel.cursor().unwrap_or(0);
        let Some(to) = target(len, Some(from), &|_| true, Move::Step(forward), 1) else {
            return;
        };
        let wrapped = if forward { to <= from } else { to >= from };
        let ring = region == Region::Items || self.view == View::Customize;
        if !wrapped || !ring {
            sel.set_cursor(Some(to));
            return;
        }
        match region {
            Region::Items => {
                let count = actions(self.view, &self.checked).len();
                self.action
                    .set_cursor(Some(if forward { 0 } else { count.saturating_sub(1) }));
                self.actions.request_focus();
            }
            Region::Actions => {
                if model.items.is_empty() {
                    return;
                }
                let at = if forward { 0 } else { model.items.len() - 1 };
                self.item.set_cursor(Some(at));
                self.items.request_focus();
            }
        }
    }

    fn toggle(&mut self, at: usize) {
        if let Some(check) = self.checked.get_mut(at) {
            *check = !*check;
        }
    }

    /// What the action under the cursor does.
    fn run(&mut self, model: &import::Model) -> Option<ImportIntent> {
        match actions(self.view, &self.checked).get(self.action.cursor().unwrap_or(0))? {
            Act::Proceed => Some(ImportIntent::Proceed(
                (0..self.checked.len())
                    .filter(|at| self.checked[*at])
                    .collect(),
            )),
            Act::Skip => Some(ImportIntent::Cancel),
            Act::Customize => {
                self.show(View::Customize, model);
                None
            }
            Act::Back => {
                self.show(View::Summary, model);
                None
            }
        }
    }

    /// The cursors are on rows that exist (a toggle can take the first action
    /// away).
    fn settle(&mut self, model: &import::Model) {
        let last = |len: usize| len.saturating_sub(1);
        let action = self.action.cursor().unwrap_or(0);
        self.action.set_cursor(Some(
            action.min(last(actions(self.view, &self.checked).len())),
        ));
        let item = self.item.cursor().unwrap_or(0);
        self.item
            .set_cursor(Some(item.min(last(self.item_count(model)))));
    }

    /// The checklist's scroll follows the cursor while the checklist has the
    /// keys.
    fn track(&self, model: &import::Model, strings: &Strings, num: &rtwk::l10n::NumSyms) {
        if !self.items.has_focus() {
            return;
        }
        let rows = rows::of(self.view, model, &self.checked, strings, num);
        if let Some(row) = rows::cursor_row(&rows, self.item.cursor()) {
            self.list.ensure_index_visible(row);
        }
    }
}

/// What the review is made of beside the model: the regions' nodes, their
/// cursors, their keys, the checklist's scroll.
struct Regions {
    list: ListController,
    items: FocusNode,
    actions: FocusNode,
    item: SelectionController<usize>,
    action: SelectionController<usize>,
    checklist: Vec<(&'static str, Handler<()>)>,
    buttons: Vec<(&'static str, Handler<()>)>,
    focus: (Handler<bool>, Handler<bool>),
    /// The taps of the checklist's items and of the actions, by index.
    item_taps: Vec<Handler<PointerEvent>>,
    action_taps: Vec<Handler<PointerEvent>>,
}

/// What the review draws with.
#[derive(Clone)]
struct Look {
    dim: Style,
    bold: Style,
    accent: Style,
    selected: Style,
    error: Style,
    section: Style,
}

/// The oracle's `render_ref`: the heading, the intro, the error, the list,
/// a blank row, the actions and the footer, one under the other. The list is
/// the `Flexible` of the column: the room the other lines leave, down to
/// nothing, and no more than its rows need.
fn review(
    look: &Look,
    strings: &'static Strings,
    num: rtwk::l10n::NumSyms,
    model: &import::Model,
    checked: &[bool],
    view: View,
    regions: Regions,
) -> AnyWidget {
    let customize = view == View::Customize;
    let acts = actions(view, checked);
    let rows = rows::of(view, model, checked, strings, &num);
    let intro = if customize {
        strings.intro_customize
    } else {
        strings.intro_summary
    };
    let items_focus = regions.items.has_focus();
    let actions_focus = regions.actions.has_focus();

    let heading = Text::rich([
        Span::styled(strings.heading_marker, Style::new()),
        Span::styled(
            if customize {
                strings.heading_customize
            } else {
                strings.heading_summary
            },
            look.bold.clone(),
        ),
    ]);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(heading);
    // a wrapping paragraph in an area of one row per line: what wraps
    // below the area is cut (the oracle's `Paragraph` in a fixed area)
    let wrapped = |text: String, style: Style| -> AnyWidget {
        SizedBox::new(None, Some(1), Some(Text::new(text).style(style).into())).into()
    };
    for text in intro {
        column = column.child(wrapped((*text).to_string(), Style::new()));
    }
    if let Some(message) = &model.error {
        column = column.child(wrapped(
            strings.import_failed(message, &num),
            look.error.clone(),
        ));
    }
    let rows = Rc::new(rows);
    let builder = {
        let (rows, look, item) = (rows.clone(), look.clone(), regions.item.clone());
        let taps = Rc::new(regions.item_taps.clone());
        move |at: usize| -> AnyWidget {
            row(&look, strings, &num, &rows[at], items_focus, &item, &taps)
        }
    };
    let list = ListView::builder(rows.len(), builder)
        .item_extent(1)
        .shrink_wrap(true)
        .controller(regions.list.clone());
    let checklist = KeyBindings::new(
        regions.checklist.clone(),
        Focus::new(list)
            .focus_node(regions.items.clone())
            .on_focus_change(regions.focus.0.clone()),
    );
    // The list is as tall as its rows and no taller than the room the
    // `Flexible` gets (`ListView.shrinkWrap`).
    column = column.child(Flexible {
        fit: FlexFit::Loose,
        ..Flexible::new(1, checklist)
    });
    column = column.child(SizedBox::new(None, Some(1), None));
    column = column.child(wrapped(
        strings.selected(
            checked.iter().filter(|on| **on).count(),
            model.items.len(),
            &num,
        ),
        Style::new(),
    ));
    let mut buttons = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for (at, act) in acts.iter().enumerate() {
        let (look, action) = (look.clone(), regions.action.clone());
        let label = match act {
            Act::Proceed => strings.action_proceed,
            Act::Customize => strings.action_customize,
            Act::Skip => strings.action_skip,
            Act::Back => strings.action_back,
        };
        let button = ListenableBuilder::new(regions.action.listenable(), move |_, _| {
            let chosen = actions_focus && action.cursor() == Some(at);
            let marker = if chosen { "›" } else { " " };
            let style = if chosen {
                look.selected.clone()
            } else if actions_focus {
                Style::new()
            } else {
                look.dim.clone()
            };
            let text = format!("{marker} {}. {label}", at + 1);
            Text::new(text)
                .style(style)
                .wrap(TextWrap::None)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .into()
        });
        buttons = buttons.child(match regions.action_taps.get(at) {
            Some(tap) => GestureDetector::new()
                .behavior(HitTestBehavior::Opaque)
                .on_tap(tap.clone())
                .child(button)
                .into(),
            None => AnyWidget::from(button),
        });
    }
    let buttons = KeyBindings::new(
        regions.buttons.clone(),
        Focus::new(buttons)
            .focus_node(regions.actions.clone())
            .on_focus_change(regions.focus.1.clone())
            .autofocus(),
    );
    column = column.child(buttons);
    column = column.child(footer(look, strings, customize, actions_focus));
    Padding::new(EdgeInsets::only(2, 1, 2, 1), column).into()
}

/// One row of the list: the checkbox lines carry the cursor when the items
/// have the focus; the detail and section lines are dim.
fn row(
    look: &Look,
    strings: &'static Strings,
    num: &rtwk::l10n::NumSyms,
    row: &ReviewRow,
    items_focus: bool,
    item: &SelectionController<usize>,
    taps: &[Handler<PointerEvent>],
) -> AnyWidget {
    // The oracle's cut keeps the blank before the `…` (N1: `dots_end`).
    let line = |text: String, style: Style| -> AnyWidget { dots_end(text, style) };
    let check = |mark: Mark| match mark {
        Mark::All => strings.check_all,
        Mark::Some => strings.check_some,
        Mark::None => strings.check_none,
    };
    // A row the cursor may rest on reads the controller (Flutter's
    // `ListenableBuilder` over `SelectionController.isSelected`).
    let entry = |index: usize, body: String| -> AnyWidget {
        let (look, strings, item) = (look.clone(), strings, item.clone());
        let tap = taps.get(index).cloned();
        let entry = ListenableBuilder::new(item.listenable(), move |_, _| {
            let text = |mark: &str| format!("{mark}{body}");
            let (text, style) = if items_focus && item.cursor() == Some(index) {
                (text(strings.cursor), look.selected.clone())
            } else {
                (text(strings.no_cursor), Style::new())
            };
            Text::new(text)
                .style(style)
                .wrap(TextWrap::None)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .into()
        });
        match tap {
            Some(tap) => GestureDetector::new()
                .behavior(HitTestBehavior::Opaque)
                .on_tap(tap)
                .child(entry)
                .into(),
            None => entry.into(),
        }
    };
    match row {
        ReviewRow::Group { index, mark, label } => {
            entry(*index, format!("[{}] {label}", check(*mark)))
        }
        ReviewRow::Entry {
            index,
            enabled,
            label,
        } => {
            let mark = if *enabled { Mark::All } else { Mark::None };
            entry(*index, format!("[{}] {label}", check(mark)))
        }
        ReviewRow::Detail(text) => {
            line(format!("{}{text}", strings.detail_indent), look.dim.clone())
        }
        ReviewRow::Importing(summary) => {
            let text = summary.as_deref().map_or_else(
                || strings.importing_none.to_owned(),
                |summary| strings.importing(summary, num),
            );
            line(format!("{}{text}", strings.detail_indent), look.dim.clone())
        }
        ReviewRow::Section(None) => line(strings.section_home.to_owned(), look.section.clone()),
        ReviewRow::Section(Some(dir)) => Text::rich([
            Span::styled(strings.section_project, look.section.clone()),
            Span::styled(dir.clone(), look.dim.clone()),
        ])
        .wrap(TextWrap::None)
        .max_lines(1)
        .overflow(TextOverflow::Ellipsis)
        .into(),
        ReviewRow::Gap => line(String::new(), Style::new()),
    }
}

/// The hint under the actions: the keys of the view the focus is in.
fn footer(look: &Look, strings: &Strings, customize: bool, on_actions: bool) -> AnyWidget {
    let dim = |text: &'static str| Span::styled(text, look.dim.clone());
    let key = |text: &'static str| Span::styled(text, look.accent.clone());
    let moves = [
        dim(strings.key_up),
        dim(strings.slash),
        dim(strings.key_down),
        dim(strings.hint_move),
    ];
    let spans: Vec<Span> = if !customize {
        [dim(strings.hint_use)]
            .into_iter()
            .chain(moves)
            .chain([
                dim(strings.key_enter),
                dim(strings.hint_select),
                key(strings.key_customize),
                dim(strings.hint_customize),
            ])
            .collect()
    } else if on_actions {
        [
            dim(strings.hint_press),
            dim(strings.key_enter),
            dim(strings.hint_continue),
        ]
        .into_iter()
        .chain(moves)
        .chain([key(strings.key_back), dim(strings.hint_back)])
        .collect()
    } else {
        [dim(strings.hint_use)]
            .into_iter()
            .chain(moves)
            .chain([
                dim(strings.key_space),
                dim(strings.hint_toggle),
                key(strings.key_back),
                dim(strings.hint_back),
            ])
            .collect()
    };
    Text::rich(spans)
        .wrap(TextWrap::None)
        .max_lines(1)
        .overflow(TextOverflow::Ellipsis)
        .into()
}
