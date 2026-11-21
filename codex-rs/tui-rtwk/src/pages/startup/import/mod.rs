//! `import` (S06): the view half of the import from other agents, a step of
//! the startup page (`onboarding::Step::Import`). The flow, its requests and
//! its choices are core's `codex_tui_core::screens::import`; this directory
//! draws what the model says: the source prompt (`source::SourcePrompt`, the
//! startup prompts' look over the shared `Options`) and the
//! review, a summary and a checklist of the detected items with the actions
//! under them. Oracle: `external_agent_config_migration/{mod,render,source}.rs`.

use std::rc::Rc;

use rtwk::focus::{Focus, KeyBindings};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

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
pub mod nav;
pub mod notices;
pub mod rows;
pub mod source;
pub mod strings;

use nav::{Action, Focus as Has, Key, Nav, View};
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

/// The step: asks for the detection when the flow has not started, opens the
/// source prompt when the model wants a source, and draws the review.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: the scroll of the checklist (rtwk's controller) and what the
/// view has told the model so far (R14: ids and counters, no model data).
#[derive(Debug)]
pub struct ScreenState {
    list: ListController,
    /// The review's navigation (the page, the focus, the cursor and the
    /// highlighted action): the view's own state, for the review `nav_for`.
    nav: Option<Nav>,
    nav_for: u64,
    /// The view and the row the checklist was last scrolled for.
    scrolled: (Option<View>, Option<usize>, bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// A key of the review: the `Nav` moves, an intent may come of it.
    Key(Key),
    /// An intent of the flow (cancel).
    Intent(ImportIntent),
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            list: ListController::new(),
            nav: None,
            nav_for: 0,
            scrolled: (None, None, false),
        }
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
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
                match &mut self.nav {
                    Some(nav) if self.nav_for == model.review_id => nav.settle(model),
                    _ => {
                        self.nav = Some(Nav::open(model));
                        self.nav_for = model.review_id;
                        self.scrolled = (None, None, false);
                    }
                }
                self.track(model, strings::of(cx), &shared::of(cx).num);
            }
            _ => {}
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Key(key) => {
                let screens = read::<ScreenStates, _>(cx);
                let model = &screens.import;
                let Some(nav) = &mut self.nav else {
                    return Rebuild::No;
                };
                if let Some(intent) = nav.key(model, key) {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.screen, ScreenIntent::Import(intent));
                }
                nav.settle(model);
                self.track(model, strings::of(cx), &shared::of(cx).num);
                return Rebuild::Yes;
            }
            Msg::Intent(intent) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Import(intent));
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let screens = watch::<ScreenStates, _>(cx);
        if screens.import.phase == Phase::Source {
            return source::SourcePrompt.into();
        }
        let nav = match (&self.nav, screens.import.phase) {
            (Some(nav), Phase::Review) if self.nav_for == screens.import.review_id => nav.clone(),
            _ => {
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
        };
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let look = Look {
            dim: tokens.dim().attrs(Attrs::DIM),
            bold: tokens.bold(),
            accent: tokens.accent(),
            selected: tokens.selected(),
            error: tokens.error(),
            section: tokens.dim().attrs(Attrs::DIM | Attrs::BOLD),
        };
        let body = review(&look, strings, num, screens, nav, self.list.clone());
        let key = |key: Key| cx.send(Msg::Key(key));
        let cancel = || cx.send(Msg::Intent(ImportIntent::Cancel));
        let mut bindings = vec![
            ("up", key(Key::Up)),
            ("k", key(Key::Up)),
            ("down", key(Key::Down)),
            ("j", key(Key::Down)),
            ("enter", key(Key::Confirm)),
            ("space", key(Key::Toggle)),
            ("c", key(Key::Customize)),
            ("b", key(Key::Back)),
            ("a", key(Key::SelectAll)),
            ("n", key(Key::SelectNone)),
            ("escape", key(Key::Escape)),
            ("ctrl+c", cancel()),
            ("ctrl+d", cancel()),
        ];
        for digit in 1..=9u8 {
            bindings.push((DIGITS[usize::from(digit) - 1], key(Key::Number(digit))));
        }
        KeyBindings::new(bindings, Focus::new(body).autofocus()).into()
    }
}

const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

impl ScreenState {
    /// The checklist's scroll follows the cursor when the items have the
    /// keys, and starts at the top on another page.
    fn track(&mut self, model: &import::Model, strings: &Strings, num: &rtwk::l10n::NumSyms) {
        let Some(nav) = &self.nav else {
            return;
        };
        let rows = rows::of(nav.view, model, strings, num);
        let key = (
            Some(nav.view),
            rows::cursor_row(&rows, nav.cursor),
            nav.focus == Has::Items,
        );
        if key.0 != self.scrolled.0 {
            self.list.jump_to_index(0);
        }
        if let (true, Some(row)) = (key.2, key.1) {
            self.list.ensure_index_visible(row);
        }
        self.scrolled = key;
    }
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
    nav: Nav,
    list: ListController,
) -> AnyWidget {
    let look = look.clone();
    let body = LayoutBuilder::new(move |_, constraints| {
        let model = &screens.import;
        let customize = nav.view == View::Customize;
        let actions = nav.actions(model);
        let rows = rows::of(nav.view, model, strings, &num);
        let intro = if customize {
            strings.intro_customize
        } else {
            strings.intro_summary
        };
        let error = usize::from(model.error.is_some());
        let actions_height = actions.len() + 1;
        let fixed = 1 + intro.len() + error + 1 + actions_height + 1;
        let room = usize::try_from(constraints.max_height)
            .unwrap_or(0)
            .saturating_sub(fixed);
        let list_height = rows.len().max(1).min(room);

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
        let items_focus = nav.focus == Has::Items;
        let cursor = nav.cursor;
        let rows = Rc::new(rows);
        let builder = {
            let (rows, look) = (rows.clone(), look.clone());
            move |at: usize| -> AnyWidget {
                row(&look, strings, &num, &rows[at], items_focus, cursor)
            }
        };
        column = column.child(SizedBox::new(
            None,
            Some(i32::try_from(list_height).unwrap_or(0)),
            Some(
                ListView::builder(rows.len(), builder)
                    .item_extent(1)
                    .controller(list.clone())
                    .into(),
            ),
        ));
        column = column.child(SizedBox::new(None, Some(1), None));
        column = column.child(wrapped(
            strings.selected(model.selected_count(), model.items.len(), &num),
            Style::new(),
        ));
        for (at, action) in actions.iter().enumerate() {
            let chosen = nav.focus == Has::Actions && nav.action == *action;
            let marker = if chosen { "›" } else { " " };
            let label = match action {
                Action::Proceed => strings.action_proceed,
                Action::Customize => strings.action_customize,
                Action::Skip => strings.action_skip,
                Action::Back => strings.action_back,
            };
            let style = if chosen {
                look.selected.clone()
            } else if nav.focus == Has::Actions {
                Style::new()
            } else {
                look.dim.clone()
            };
            column = column.child(line(format!("{marker} {}. {label}", at + 1), style));
        }
        column = column.child(footer(&look, strings, customize, nav.focus == Has::Actions));
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
