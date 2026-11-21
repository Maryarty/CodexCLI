//! `import` (S06): the view half of the import from other agents, a step of
//! the startup page (`onboarding::Step::Import`). The flow, its requests and
//! its choices are core's `codex_tui_core::screens::import`; this directory
//! draws what the model says: the source prompt (`source::SourcePrompt`, the
//! startup prompts' look over the shared `Options`) and the
//! review, a summary and a checklist of the detected items with the actions
//! under them. Oracle: `external_agent_config_migration/{mod,render,source}.rs`.

use std::rc::Rc;

use rtwk::focus::{Focus, FocusNode, KeyBindings};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::import::{self as import, Intent as ImportIntent, Mark, Phase};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as shared;

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
fn actions(view: View, model: &import::Model) -> Vec<Act> {
    match view {
        View::Summary => {
            let mut actions = Vec::new();
            if model.proceed_enabled() {
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
/// each a `Focus` node with its own keys; what has the focus is the nodes'
/// (`has_focus`), the cursor of a region is the row its keys moved to, and the
/// checklist scrolls with rtwk's controller. Nothing of the model is kept
/// here (R14: the page, two cursors, the nodes, the review they are for).
#[derive(Debug)]
pub struct ScreenState {
    list: ListController,
    view: View,
    /// The review (`Model::review_id`) the page and the cursors are for.
    review_for: u64,
    items: FocusNode,
    actions: FocusNode,
    /// The checklist's cursor (an index of the model's `items`, or of its
    /// `groups` in the summary) and the action's (an index of `actions`).
    item: usize,
    action: usize,
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
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            list: ListController::new(),
            view: View::Summary,
            review_for: 0,
            items: FocusNode::new(),
            actions: FocusNode::new(),
            item: 0,
            action: 0,
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
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.import;
        match model.phase {
            Phase::Done => {
                Navigator::maybe_pop(cx);
            }
            Phase::Review => {
                if self.review_for != model.review_id {
                    // a review opens on its summary, the actions have the keys
                    self.review_for = model.review_id;
                    self.show(View::Summary, model);
                }
                self.settle(model);
                self.track(model, strings::of(cx), &shared::of(cx).num);
            }
            _ => {}
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
            Msg::ItemUp => match self.item.checked_sub(1) {
                Some(at) => self.item = at,
                None => self.go_to_actions(model, false),
            },
            Msg::ItemDown => {
                if self.item + 1 < self.item_count(model) {
                    self.item += 1;
                } else {
                    self.go_to_actions(model, true);
                }
            }
            Msg::ActionUp => match self.action.checked_sub(1) {
                Some(at) => self.action = at,
                None if customize => self.go_to_items(model, false),
                None => self.action = actions(self.view, model).len().saturating_sub(1),
            },
            Msg::ActionDown => {
                if self.action + 1 < actions(self.view, model).len() {
                    self.action += 1;
                } else if customize {
                    self.go_to_items(model, true);
                } else {
                    self.action = 0;
                }
            }
            Msg::Toggle => intent = customize.then_some(ImportIntent::Toggle(self.item)),
            Msg::Run => intent = self.run(model),
            Msg::Number(number) => {
                let at = usize::from(number).checked_sub(1);
                if let Some(at) = at.filter(|at| *at < actions(self.view, model).len()) {
                    self.action = at;
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
            Msg::All(on) => intent = customize.then_some(ImportIntent::SetAll(on)),
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
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let screens = watch::<ScreenStates, _>(cx);
        if screens.import.phase == Phase::Source {
            return source::SourcePrompt.into();
        }
        if screens.import.phase != Phase::Review || self.review_for != screens.import.review_id {
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
            checklist,
            buttons,
            focus: (cx.map(Msg::Focus), cx.map(Msg::Focus)),
        };
        let cursors = (self.item, self.action);
        let body = review(
            &look,
            strings::of(cx),
            shared::of(cx).num,
            screens,
            self.view,
            cursors,
            regions,
        );
        KeyBindings::new(page, body).into()
    }
}

const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

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
        self.item = 0;
        self.action = 0;
        self.list.jump_to_index(0);
        let customize = view == View::Customize && !model.items.is_empty();
        self.items.set_can_request_focus(customize);
        if customize {
            self.items.request_focus();
        } else {
            self.actions.request_focus();
        }
    }

    /// Past the end of the checklist: the focus goes to the actions, at their
    /// first or their last.
    fn go_to_actions(&mut self, model: &import::Model, first: bool) {
        let count = actions(self.view, model).len();
        self.action = if first { 0 } else { count.saturating_sub(1) };
        self.actions.request_focus();
    }

    /// Past the end of the actions of the checklist's page: back into the
    /// checklist, at its first entry or its last.
    fn go_to_items(&mut self, model: &import::Model, first: bool) {
        if model.items.is_empty() {
            return;
        }
        self.item = if first { 0 } else { model.items.len() - 1 };
        self.items.request_focus();
    }

    /// What the action under the cursor does.
    fn run(&mut self, model: &import::Model) -> Option<ImportIntent> {
        match actions(self.view, model).get(self.action)? {
            Act::Proceed => Some(ImportIntent::Proceed),
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
        self.action = self
            .action
            .min(actions(self.view, model).len().saturating_sub(1));
        self.item = self.item.min(self.item_count(model).saturating_sub(1));
    }

    /// The checklist's scroll follows the cursor while the checklist has the
    /// keys.
    fn track(&self, model: &import::Model, strings: &Strings, num: &rtwk::l10n::NumSyms) {
        if !self.items.has_focus() {
            return;
        }
        let rows = rows::of(self.view, model, strings, num);
        if let Some(row) = rows::cursor_row(&rows, Some(self.item)) {
            self.list.ensure_index_visible(row);
        }
    }
}

/// What the review is made of beside the model: the regions' nodes, their
/// keys, the checklist's scroll.
struct Regions {
    list: ListController,
    items: FocusNode,
    actions: FocusNode,
    checklist: Vec<(&'static str, Handler<()>)>,
    buttons: Vec<(&'static str, Handler<()>)>,
    focus: (Handler<bool>, Handler<bool>),
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
/// a blank row, the actions and the footer, one under the other. The list
/// takes what the screen has left, down to one row.
fn review(
    look: &Look,
    strings: &'static Strings,
    num: rtwk::l10n::NumSyms,
    screens: Rc<ScreenStates>,
    view: View,
    (item, action): (usize, usize),
    regions: Regions,
) -> AnyWidget {
    let look = look.clone();
    let body = LayoutBuilder::new(move |_, constraints| {
        let model = &screens.import;
        let customize = view == View::Customize;
        let acts = actions(view, model);
        let rows = rows::of(view, model, strings, &num);
        let intro = if customize {
            strings.intro_customize
        } else {
            strings.intro_summary
        };
        let error = usize::from(model.error.is_some());
        let actions_height = acts.len() + 1;
        let fixed = 1 + intro.len() + error + 1 + actions_height + 1;
        let room = usize::try_from(constraints.max_height)
            .unwrap_or(0)
            .saturating_sub(fixed);
        let list_height = rows.len().max(1).min(room);
        let items_focus = regions.items.has_focus();
        let actions_focus = regions.actions.has_focus();

        let line = |text: String, style: Style| -> AnyWidget {
            Text::new(text)
                .style(style)
                .wrap(TextWrap::None)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .into()
        };
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
        let cursor = Some(item);
        let rows = Rc::new(rows);
        let builder = {
            let (rows, look) = (rows.clone(), look.clone());
            move |at: usize| -> AnyWidget {
                row(&look, strings, &num, &rows[at], items_focus, cursor)
            }
        };
        let list = ListView::builder(rows.len(), builder)
            .item_extent(1)
            .controller(regions.list.clone());
        let checklist = KeyBindings::new(
            regions.checklist.clone(),
            Focus::new(list)
                .focus_node(regions.items.clone())
                .on_focus_change(regions.focus.0.clone()),
        );
        column = column.child(SizedBox::new(
            None,
            Some(i32::try_from(list_height).unwrap_or(0)),
            Some(checklist.into()),
        ));
        column = column.child(SizedBox::new(None, Some(1), None));
        column = column.child(wrapped(
            strings.selected(model.selected_count(), model.items.len(), &num),
            Style::new(),
        ));
        let mut buttons = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (at, act) in acts.iter().enumerate() {
            let chosen = actions_focus && action == at;
            let marker = if chosen { "›" } else { " " };
            let label = match act {
                Act::Proceed => strings.action_proceed,
                Act::Customize => strings.action_customize,
                Act::Skip => strings.action_skip,
                Act::Back => strings.action_back,
            };
            let style = if chosen {
                look.selected.clone()
            } else if actions_focus {
                Style::new()
            } else {
                look.dim.clone()
            };
            buttons = buttons.child(line(format!("{marker} {}. {label}", at + 1), style));
        }
        let buttons = KeyBindings::new(
            regions.buttons.clone(),
            Focus::new(buttons)
                .focus_node(regions.actions.clone())
                .on_focus_change(regions.focus.1.clone())
                .autofocus(),
        );
        column = column.child(buttons);
        column = column.child(footer(&look, strings, customize, actions_focus));
        column.into()
    });
    Padding::new(EdgeInsets::only(2, 1, 2, 1), body).into()
}

/// One row of the list: the checkbox lines carry the cursor when the items
/// have the focus; the detail and section lines are dim.
fn row(
    look: &Look,
    strings: &Strings,
    num: &rtwk::l10n::NumSyms,
    row: &ReviewRow,
    items_focus: bool,
    cursor: Option<usize>,
) -> AnyWidget {
    let line = |text: String, style: Style| -> AnyWidget {
        Text::new(text)
            .style(style)
            .wrap(TextWrap::None)
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis)
            .into()
    };
    let check = |mark: Mark| match mark {
        Mark::All => strings.check_all,
        Mark::Some => strings.check_some,
        Mark::None => strings.check_none,
    };
    let entry = |index: usize, body: String| -> AnyWidget {
        if items_focus && cursor == Some(index) {
            line(format!("{}{body}", strings.cursor), look.selected.clone())
        } else {
            line(format!("{}{body}", strings.no_cursor), Style::new())
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
