//! `agents` (S21): the view half of the agents command center. One
//! `PageRoute` over the model (`ScreenStates.agents`): the header with its
//! tab strip, the grouped task list (a `ListView`, scrolled by item) beside
//! the details of the highlighted task, the key hints. Its model, events,
//! intents and RPCs are core's `codex_tui_core::screens::agents`; this file
//! holds the keys, the mouse and the widget state, `view.rs` the widgets.
//! Oracle: `tui/src/app/agent_center/` and `app/agents_overview_view.rs`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::gestures::PointerEvent;
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, watch};
use rtwk::scroll::ListController;
use rtwk::style::Theme;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::agents::{CenterRow, Intent as AgentsIntent, Sel};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod layout;
pub mod strings;
mod view;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "three tasks of one project; the first one needs you",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: true }.into()),
        },
        DebugEntry {
            name: "attention",
            description: "a task whose child waits for an approval and a response",
            script: include_str!("fixtures/attention.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: true }.into()),
        },
        DebugEntry {
            name: "empty_narrow",
            description: "no tasks at 48 cells, with nowhere to go back to",
            script: include_str!("fixtures/empty_narrow.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: false }.into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen { can_return: true }.into())
}

/// Opens the command center on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen { can_return: true }.into())
}

/// The command center; `can_return` is whether a chat lies under it (the
/// oracle's `can_return_to_chat`: it adds the back hint).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub can_return: bool,
}

/// Widget state: the list's scroll (view state, R14) and the rows the last
/// layout showed (for the page keys).
pub struct ScreenState {
    list: ListController,
    page: Rc<Cell<usize>>,
    scroll: Rc<Cell<usize>>,
    /// The thread the screen was opened over: when the model's current thread
    /// is another (a resume answered), the screen has done its work.
    entered: Option<Option<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Key(KeyPress),
    Row(usize),
    Tab(usize),
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            list: ListController::new(),
            page: Rc::new(Cell::new(10)),
            scroll: Rc::new(Cell::new(0)),
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
/// for a key the screen does not take.
fn intent_of(key: &KeyPress, editing: bool, help: bool, page: usize) -> Option<AgentsIntent> {
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
        KeyCode::Up => return Some(AgentsIntent::Move { forward: false }),
        KeyCode::Down => return Some(AgentsIntent::Move { forward: true }),
        KeyCode::PageUp => return Some(AgentsIntent::Page { forward: false, height: page }),
        KeyCode::PageDown => return Some(AgentsIntent::Page { forward: true, height: page }),
        KeyCode::Home => return Some(AgentsIntent::Jump { top: true }),
        KeyCode::End => return Some(AgentsIntent::Jump { top: false }),
        KeyCode::Enter if !ctrl && !alt => return Some(AgentsIntent::Accept),
        KeyCode::Tab => return Some(AgentsIntent::CycleFilter { back: shift }),
        KeyCode::F3 => return Some(AgentsIntent::BeginSearch),
        _ => {}
    }
    if ctrl {
        return match key.ch {
            Some('p' | 'k') => Some(AgentsIntent::Move { forward: false }),
            Some('n' | 'j') => Some(AgentsIntent::Move { forward: true }),
            Some('c') => Some(AgentsIntent::CancelEditing),
            _ => None,
        };
    }
    if editing {
        return match key.code {
            KeyCode::Backspace => Some(AgentsIntent::Backspace),
            _ => text.map(|text| AgentsIntent::Type(text.to_owned())),
        };
    }
    match (key.code, text) {
        (KeyCode::Backspace, _) => Some(AgentsIntent::Delete),
        (_, Some("?")) => Some(AgentsIntent::ToggleHelp),
        (_, Some("k")) => Some(AgentsIntent::Move { forward: false }),
        (_, Some("j")) => Some(AgentsIntent::Move { forward: true }),
        (_, Some("g")) => Some(AgentsIntent::CycleGrouping),
        (_, Some("/")) => Some(AgentsIntent::BeginSearch),
        (_, Some("n")) => Some(AgentsIntent::New),
        (_, Some("r")) => Some(AgentsIntent::BeginRename),
        (_, Some("a")) => Some(AgentsIntent::Archive),
        (_, Some("h")) => Some(AgentsIntent::Hide),
        (_, Some("p")) => Some(AgentsIntent::TogglePin),
        (_, Some("x")) => Some(AgentsIntent::Stop),
        (_, Some("o")) => Some(AgentsIntent::Accept),
        _ => None,
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let current = watch::<ScreenStates, _>(cx).agents.current.clone();
        match &self.entered {
            None => self.entered = Some(current),
            Some(entered) if *entered != current && current.is_some() => {
                // A resume was answered: the chat shows the thread now.
                Navigator::maybe_pop(cx);
            }
            Some(_) => {}
        }
    }

    fn update(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.agents;
        let hs = read::<Hs, _>(cx);
        let mut send = |intent: AgentsIntent| cx.emit(&hs.screen, ScreenIntent::Agents(intent));
        match msg {
            Msg::Key(key) => {
                let editing = model.searching || model.rename.is_some();
                if key.code == KeyCode::Escape && !editing && !model.help {
                    if w.can_return {
                        Navigator::maybe_pop(cx);
                    }
                    return Rebuild::No;
                }
                if let Some(intent) = intent_of(&key, editing, model.help, self.page.get()) {
                    send(intent);
                }
            }
            Msg::Row(index) => {
                let center = model.center();
                match center.entries.get(index) {
                    Some(CenterRow::Task(at)) => {
                        let id = center.rows[*at].id().to_owned();
                        if center.selected == Some(*at) {
                            send(AgentsIntent::Accept);
                        } else {
                            send(AgentsIntent::Select(Sel::Task(id)));
                        }
                    }
                    Some(CenterRow::ShowMore) => {
                        if center.show_more_selected {
                            send(AgentsIntent::Accept);
                        } else {
                            send(AgentsIntent::Select(Sel::ShowMore));
                        }
                    }
                    _ => {}
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
        let center = screens.agents.center();
        let taps = view::Taps {
            rows: (0..center.entries.len())
                .map(|index| cx.map(move |_: PointerEvent| Msg::Row(index)))
                .collect(),
            tabs: (0..5).map(|index| cx.map(move |_: PointerEvent| Msg::Tab(index))).collect(),
        };
        let can_return = w.can_return;
        let list = self.list.clone();
        let page = self.page.clone();
        let remembered = self.scroll.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = usize::try_from(constraints.max_height).unwrap_or(0);
            let model = &screens.agents;
            let areas = layout::areas(width, height);
            let editing = model.searching || model.rename.is_some();
            let list_rows = areas.body_height.saturating_sub(usize::from(editing));
            let viewport = list_rows.saturating_sub(2 * usize::from(list_rows >= 3));
            page.set(viewport);
            sync_scroll(&list, &remembered, &center, viewport);
            let mut column = rtwk::layout::Column::new()
                .cross_axis_alignment(rtwk::layout::CrossAxisAlignment::Start);
            if areas.header > 0 {
                column = column.child(view::header(&look, strings, &num, model, &center, width, &taps));
            }
            if model.help {
                column = column.child(view::help(&look, strings, &areas, width));
            } else {
                column = column.child(view::body(&look, strings, &num, model, &center, &areas, &list, &taps));
            }
            if areas.footer_row < height {
                column = column.child(view::footer_row(&look, strings, model, can_return, width));
            }
            column.into()
        });
        Focus::new(body).autofocus().on_key(cx.keys(on_key)).into()
    }
}

/// Keeps the highlighted entry in the window the way the oracle does
/// (`render_center_rows`): the window moves the least, and shows the heading
/// above a first task that scrolls into view from above.
fn sync_scroll(list: &ListController, remembered: &Cell<usize>, center: &codex_tui_core::screens::agents::Center, viewport: usize) {
    let Some(selected) = center.selected_entry() else {
        return;
    };
    let entries = center.entries.len();
    let current = if list.has_clients() {
        usize::try_from(list.offset()).unwrap_or(0)
    } else {
        remembered.get()
    };
    let group_above = selected > 0 && matches!(center.entries[selected - 1], CenterRow::Group { .. });
    let start = layout::scroll(current, selected, entries, viewport, group_above);
    remembered.set(start);
    if start != current {
        list.jump_to_index(start);
    }
}
