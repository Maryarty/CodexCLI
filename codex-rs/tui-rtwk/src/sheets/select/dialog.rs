//! The generic selection dialog (the oracle's `ListSelectionView`): a title
//! and subtitle, tabs, a search line, the rows with arrows for what is
//! hidden, a footer note and hint. In Flutter it is a `showModalBottomSheet`
//! whose content is a `Column(min)` with a tab strip over a `TabController`, a
//! `TextField` over a `TextEditingController`, and a `ListView` over a
//! `ScrollController` with the cursor in a `SelectionController`; what the
//! user picks goes back as an intent with an id (ADR-003: the sheet keeps no
//! copy of app state, only the controllers: cursor, scroll, tab and query).
//! The controllers are the only holders: the sheet has no window of its own
//! (no first row, no `top`), no index shadowing the tab controller's, and
//! reads the arrows and the page size from the list's position and viewport.
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::controls::{SelectionController, TabController};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::scroll::ListController;
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::list::{Browse, List, normalize_paste};
use codex_tui_core::screens::select::{Intent, Menu};

use super::lists;
use super::rows::{RowStyles, Rows, Words};
use super::side;
use super::strings;
use crate::app::theme;
use crate::app::view::Hs;
use crate::widgets::session_list::page_rows;
use crate::widgets::tab_strip::Tabs;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuDialog;

/// What the key callback must know to tell a key for the list from one for the
/// search field (it runs before the field sees the key).
#[derive(Clone, Copy, Default)]
struct Flags {
    searchable: bool,
    tabs: bool,
    query_empty: bool,
    toggle: bool,
    secondary: Option<char>,
}

pub struct MenuDialogState {
    shown: bool,
    ready: bool,
    node: FocusNode,
    query: TextEditingController,
    flags: Rc<Cell<Flags>>,
    /// The tabs' selection (`TabController`): the tab strip's taps and the
    /// arrow keys both set it, and the dialog follows through its own
    /// message. The only holder of the tab.
    tabs: Option<TabController>,
    /// The highlighted item (an index into the tab's items).
    cursor: SelectionController<usize>,
    /// The scroll of the rows: asked to show the cursor row, read for the
    /// arrows and the size of a page.
    scroll: ListController,
}

/// A key the list answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Top,
    Bottom,
    Accept,
    Cancel,
    Toggle,
    Secondary,
    Shortcut(char),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    Query,
    /// The tab controller moved to this tab.
    Tab(usize),
    /// A tap on this tab of the strip.
    TapTab(usize),
    Row(usize),
}

impl StatefulWidget for MenuDialog {
    type State = MenuDialogState;

    fn create_state(&self) -> MenuDialogState {
        MenuDialogState {
            shown: false,
            ready: false,
            node: FocusNode::new(),
            query: TextEditingController::new(""),
            flags: Rc::new(Cell::new(Flags::default())),
            tabs: None,
            cursor: SelectionController::new(),
            scroll: ListController::new(),
        }
    }
}

impl_widget!(stateful eq MenuDialog);

/// The act a key means, with the default list keymap of the oracle
/// (`keymap.rs`: `list.*`); none: the key is not the list's.
fn act_of(key: &KeyPress, flags: Flags) -> Option<Act> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    let letter = key.text().and_then(|text| {
        let mut chars = text.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first)
    });
    let plain_text = key.code == KeyCode::Char && !ctrl && !alt;
    // An active search reserves printable keys for the query.
    let navigates = !flags.searchable || !plain_text;
    if navigates {
        let nav = match (key.code, letter, ctrl) {
            (KeyCode::Up, _, false) | (KeyCode::Char, Some('p' | 'k'), true) => Some(Act::Up),
            (KeyCode::Char, Some('k'), false) if plain_text => Some(Act::Up),
            (KeyCode::Down, _, false) | (KeyCode::Char, Some('n' | 'j'), true) => Some(Act::Down),
            (KeyCode::Char, Some('j'), false) if plain_text => Some(Act::Down),
            (KeyCode::PageUp, _, false) | (KeyCode::Char, Some('b'), true) => Some(Act::PageUp),
            (KeyCode::PageDown, _, false) | (KeyCode::Char, Some('f'), true) => Some(Act::PageDown),
            (KeyCode::Home, _, false) => Some(Act::Top),
            (KeyCode::End, _, false) => Some(Act::Bottom),
            (KeyCode::Left, _, false)
            | (KeyCode::Char, Some('h'), true)
            | (KeyCode::Backspace, _, true)
                if flags.tabs =>
            {
                Some(Act::Left)
            }
            (KeyCode::Right, _, false) | (KeyCode::Char, Some('l'), true) if flags.tabs => {
                Some(Act::Right)
            }
            _ => None,
        };
        if nav.is_some() {
            return nav;
        }
    }
    if key.code == KeyCode::Backspace {
        return None;
    }
    if key.code == KeyCode::Char
        && letter == Some(' ')
        && key.mods.is_empty()
        && flags.toggle
        && (!flags.searchable || flags.query_empty)
    {
        return Some(Act::Toggle);
    }
    match key.code {
        KeyCode::Escape if key.mods.is_empty() => return Some(Act::Cancel),
        KeyCode::Enter if key.mods.is_empty() => return Some(Act::Accept),
        _ => {}
    }
    if navigates && plain_text && letter.is_some() && letter == flags.secondary {
        return Some(Act::Secondary);
    }
    if flags.searchable || !plain_text {
        return None;
    }
    letter.map(Act::Shortcut)
}

impl State for MenuDialogState {
    type Widget = MenuDialog;
    type Msg = Msg;

    fn init_state(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let flags = self.flags.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            match act_of(&key, flags.get()) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            }
        })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.query.dispose();
        self.cursor.dispose();
    }

    fn did_change_dependencies(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let present = select::<ScreenStates, bool, _>(cx, |states| states.select.menu.is_some());
        if !present && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
        if present
            && !self.ready
            && let Some(list) = self.list(cx)
        {
            self.ready = true;
            if let Some(first) = list.first_tab() {
                let tabs = TabController::with_initial_index(list.tabs.len(), first);
                let send = cx.local_sender();
                let seen = tabs.clone();
                tabs.add_listener(Rc::new(move || send(Msg::Tab(seen.index()))));
                self.tabs = Some(tabs);
            }
            self.settle(&list, None);
        }
    }

    fn update(&mut self, msg: Msg, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(list) = self.list(cx) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Query => {
                let before = self.cursor.cursor();
                self.settle(&list, before);
            }
            Msg::Tab(_) => {
                // Another tab has its own rows and an empty search.
                self.query.set_text("");
                self.settle(&list, None);
            }
            Msg::TapTab(tab) => {
                if let Some(tabs) = &self.tabs {
                    tabs.set_index(tab);
                }
            }
            Msg::Row(item) => {
                self.cursor.set_cursor(Some(item));
                self.pick(&list, cx);
            }
            Msg::Act(act) => self.act(act, &list, cx),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(list) = self.list(cx) else {
            return AnyWidget::default();
        };
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let query = self.query.text();
        let browse = self.browse(&list, &query);
        let visible = browse.visible();
        let selected = self
            .cursor
            .cursor()
            .and_then(|item| browse.items().get(item));
        self.flags.set(Flags {
            searchable: list.searchable,
            tabs: !list.tabs.is_empty(),
            query_empty: query.is_empty(),
            toggle: selected.is_some_and(|item| item.toggle.is_some()),
            secondary: selected.and_then(|item| item.secondary.as_ref().map(|a| a.key)),
        });

        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };
        let mut pane = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let mut header = false;
        if let Some(title) = &list.title {
            header = true;
            pane = pane.child(inset(
                Text::new(title.clone())
                    .style(tokens.bold())
                    .wrap(TextWrap::Word)
                    .into(),
            ));
        }
        if let Some(subtitle) = &list.subtitle {
            header = true;
            pane = pane.child(inset(
                Text::new(subtitle.clone())
                    .style(tokens.dim())
                    .wrap(TextWrap::Word)
                    .into(),
            ));
        }
        if header {
            pane = pane.child(Text::new(String::new()));
        }
        if !list.tabs.is_empty() {
            pane = pane.child(inset(self.tab_strip(&list, &tokens, s, cx)));
            if list.searchable {
                pane = pane.child(Text::new(String::new()));
            }
        }
        if list.searchable {
            let placeholder = list.search_placeholder.clone().unwrap_or_default();
            pane = pane.child(
                Row::new().child(SizedBox::new(Some(2), None, None)).child(
                    rtwk::layout::Expanded::new(
                        1,
                        TextField::new()
                            .controller(self.query.clone())
                            .placeholder(placeholder)
                            .focus_node(self.node.clone())
                            .autofocus(true)
                            .on_changed(cx.map(|_: Rc<str>| Msg::Query))
                            .on_paste({
                                // Whitespace folds to one blank; nothing left: ignored.
                                let query = self.query.clone();
                                let send = cx.local_sender();
                                move |text| {
                                    if let Some(text) = normalize_paste(text) {
                                        query.insert(&text);
                                        send(Msg::Query);
                                    }
                                    true
                                }
                            }),
                    ),
                ),
            );
        }
        pane = pane.child(self.arrow(true, s.more_above, tokens.accent()));
        if visible.is_empty() {
            pane = pane.child(
                Text::line(s.no_matches.to_string()).style(tokens.dim().attrs(Attrs::ITALIC)),
            );
        } else {
            pane = pane.child(
                Rows {
                    rows: Rc::new(browse.rows(&visible)),
                    taps: Rc::new(
                        visible
                            .iter()
                            .map(|item| {
                                let item = *item;
                                cx.map(move |_: PointerEvent| Msg::Row(item))
                            })
                            .collect(),
                    ),
                    items: Rc::new(visible),
                    selection: self.cursor.clone(),
                    list: self.scroll.clone(),
                    max_rows: list.max_rows(),
                    styles: RowStyles {
                        cursor: tokens.accent().attrs(Attrs::BOLD),
                        label: Style::new(),
                        description: tokens.dim(),
                        disabled: tokens.dim(),
                    },
                    single_line: list.single_line,
                    name_width: list.name_column_width,
                    words: Words {
                        current: s.current,
                        disabled: s.disabled_mark,
                        reason_after: s.reason_after,
                        reason_alone: s.reason_alone,
                        reason_close: s.reason_close,
                        mark_on: s.mark_on,
                        mark_off: s.mark_off,
                    },
                }
                .view(),
            );
        }
        pane = pane.child(self.arrow(false, s.more_below, tokens.accent()));
        let mut body: AnyWidget = pane.into();
        if let Some(side) = list.side.clone() {
            body = side::with_preview(body, side, Style::new());
        }
        let mut pane = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(body);
        if let Some(note) = list.footer_note.clone() {
            pane = pane.child(inset(Text::new(note).wrap(TextWrap::Word).into()));
        }
        let hint = selected
            .and_then(|item| item.footer_hint.clone())
            .or_else(|| list.footer_hint.clone());
        if let Some(hint) = hint {
            pane = pane.child(Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::new(hint).style(tokens.dim()).wrap(TextWrap::Word),
            ));
        }
        let dialog = Dialog::bare(BoxWidget::new().style(Style::new()).child(pane));
        if list.searchable {
            Focus::new(dialog).can_request_focus(false).into()
        } else {
            Focus::new(dialog)
                .focus_node(self.node.clone())
                .autofocus()
                .into()
        }
    }
}

impl MenuDialogState {
    /// The list the model holds as the dialog shows it (`lists::of`).
    fn list(&self, cx: &mut Cx<'_, Self>) -> Option<List> {
        let menu =
            select::<ScreenStates, Option<Menu>, _>(cx, |states| states.select.menu.clone())?;
        Some(lists::of(&menu, strings::of(cx)))
    }

    /// The tab the controller is on (none: the list has no tabs).
    fn tab(&self) -> Option<usize> {
        self.tabs.as_ref().map(TabController::index)
    }

    fn browse<'a>(&self, list: &'a List, query: &'a str) -> Browse<'a> {
        Browse {
            list,
            tab: self.tab(),
            query,
        }
    }

    /// The highlight after the rows changed (a query, a tab), the rows
    /// scrolled back to their start unless the cursor is further down.
    fn settle(&mut self, list: &List, previous: Option<usize>) {
        let query = self.query.text();
        let browse = self.browse(list, &query);
        let visible = browse.visible();
        let pos = browse.settle(&visible, previous, list.initial);
        self.cursor
            .set_cursor(pos.and_then(|pos| visible.get(pos).copied()));
        self.scroll.jump_to_index(0);
        if let Some(pos) = pos {
            self.scroll.ensure_index_visible(pos);
        }
    }

    fn act(&mut self, act: Act, list: &List, cx: &mut Cx<'_, Self>) {
        let query = self.query.text();
        let browse = self.browse(list, &query);
        let visible = browse.visible();
        let at = self
            .cursor
            .cursor()
            .and_then(|item| visible.iter().position(|v| *v == item));
        // A page is the rows the list shows.
        let window = page_rows(&self.scroll);
        let to = match act {
            Act::Up => browse.step(&visible, at, false),
            Act::Down => browse.step(&visible, at, true),
            Act::PageUp => browse.page(&visible, at, false, window),
            Act::PageDown => browse.page(&visible, at, true, window),
            Act::Top => browse.jump(&visible, true),
            Act::Bottom => browse.jump(&visible, false),
            Act::Left | Act::Right => {
                if let Some(tabs) = &self.tabs {
                    let len = list.tabs.len();
                    let now = tabs.index();
                    let next = if act == Act::Right {
                        now + 1
                    } else {
                        now + len - 1
                    };
                    tabs.set_index(next % len);
                }
                return;
            }
            Act::Accept if list.multi => return self.confirm(cx),
            Act::Accept => return self.accept(list, cx),
            Act::Cancel => return self.close(cx),
            Act::Toggle => return self.toggle(list, cx),
            Act::Secondary => return self.secondary(list, cx),
            Act::Shortcut(key) => {
                if let Some(item) = browse.shortcut(key) {
                    self.cursor.set_cursor(Some(item));
                    self.pick(list, cx);
                }
                return;
            }
        };
        if let Some(pos) = to {
            self.cursor.set_cursor(visible.get(pos).copied());
            self.scroll.ensure_index_visible(pos);
        }
    }

    fn id(&self, list: &List) -> Option<String> {
        self.cursor
            .cursor()
            .map(|item| list.id_of(self.tab(), item))
    }

    /// A click or a shortcut on a row: accepts it, and in a multi-select
    /// flips it (the choice is made by `confirm`).
    fn pick(&mut self, list: &List, cx: &mut Cx<'_, Self>) {
        if list.multi {
            self.toggle(list, cx);
        } else {
            self.accept(list, cx);
        }
    }

    /// Enter on a multi-select: every toggled-on item is the answer.
    fn confirm(&mut self, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Select(Intent::Confirm));
    }

    fn accept(&mut self, list: &List, cx: &mut Cx<'_, Self>) {
        let Some(item) = self.cursor.cursor() else {
            // Nothing matches: the oracle takes it for a cancel.
            return self.close(cx);
        };
        let hs = read::<Hs, _>(cx);
        let is_list = select::<ScreenStates, bool, _>(cx, |states| {
            matches!(states.select.menu, Some(Menu::List(_)))
        });
        let disabled = list
            .items_of(self.tab())
            .get(item)
            .is_none_or(codex_tui_core::screens::select::list::Item::is_disabled);
        if disabled {
            return;
        }
        let intent = if is_list {
            Intent::Choose(list.id_of(self.tab(), item))
        } else {
            Intent::Pick(item)
        };
        cx.emit(&hs.screen, ScreenIntent::Select(intent));
    }

    fn toggle(&mut self, list: &List, cx: &mut Cx<'_, Self>) {
        if let Some(id) = self.id(list) {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Select(Intent::Toggle(id)));
        }
    }

    fn secondary(&mut self, list: &List, cx: &mut Cx<'_, Self>) {
        if let Some(id) = self.id(list) {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Select(Intent::Secondary(id)));
        }
    }

    fn close(&mut self, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Select(Intent::Close));
        close_dialog::<_, ()>(cx, None);
    }

    /// The tab labels, the active one marked: the shared filled strip of whole
    /// tabs (`widgets/tab_strip.rs`, oracle `render_filled_tab_bar`) over the
    /// dialog's `TabController`; a tap on a tab sets it.
    fn tab_strip(
        &self,
        list: &List,
        tokens: &theme::Tokens,
        s: &'static strings::Strings,
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let Some(tabs) = &self.tabs else {
            return AnyWidget::default();
        };
        Tabs {
            labels: list.tabs.iter().map(|tab| tab.label.clone()).collect(),
            active: tabs.index(),
            marks: (s.tab_left, s.tab_right),
            // The oracle's unknown-palette fallback (`active_tab_style`): bold and
            // underlined on the terminal's own colours.
            styles: (
                Style::new().attrs(Attrs::BOLD | Attrs::UNDERLINE),
                tokens.dim(),
            ),
            taps: (0..list.tabs.len())
                .map(|tab| cx.map(move |_: PointerEvent| Msg::TapTab(tab)))
                .collect(),
        }
        .widget()
    }

    /// A row high, `glyph` while rows are scrolled out of view above (or
    /// below): the oracle's `↑` and `↓` of a picker, read from the list's
    /// position, so they follow its scroll.
    fn arrow(&self, above: bool, glyph: &'static str, style: Style) -> AnyWidget {
        let scroll = self.scroll.clone();
        let watched = Rc::new((*self.scroll).clone());
        SizedBox::new(
            None,
            Some(1),
            Some(
                ListenableBuilder::new(watched, move |_, _| {
                    let shown = scroll.try_position().is_ok_and(|at| {
                        if above {
                            at.pixels() > 0
                        } else {
                            at.pixels() < at.max_scroll_extent()
                        }
                    });
                    Text::line(if shown { glyph } else { "" })
                        .style(style.clone())
                        .into()
                })
                .into(),
            ),
        )
        .into()
    }
}
