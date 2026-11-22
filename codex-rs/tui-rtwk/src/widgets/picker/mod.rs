//! The picker (the oracle's `ListSelectionView`): a title
//! and subtitle, tabs, a search line, the rows with arrows for what is
//! hidden, a footer note and hint. In Flutter it is the content of a
//! `showModalBottomSheet`: a `Column(min)` with a tab strip over a
//! `TabController`, a `TextField` over a `TextEditingController`, and a
//! `ListView` over a `ScrollController` with the cursor in a
//! `SelectionController`. It is props-driven (R14 allows props): the opener
//! hands it the `List` and a `Handler<Event>`; what the user picks goes back as
//! an `Event` with an id, the opener turns it into an intent (ADR-003: the
//! picker keeps no copy of app state, only the controllers: cursor, scroll,
//! tab and query).
//! The controllers are the only holders: the sheet has no window of its own
//! (no first row, no `top`), no index shadowing the tab controller's, and
//! reads the arrows and the page size from the list's position and viewport.
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::controls::{SelectionController, TabController};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyCombo, KeyEventResult, KeyPress};
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{BoxWidget, Expanded, Row, SizedBox};
use rtwk::overlay::Dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::scroll::ListController;
use rtwk::style::{Attrs, Style};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_BOTTOM, LIST_JUMP_TOP, LIST_MOVE_DOWN, LIST_MOVE_LEFT,
    LIST_MOVE_RIGHT, LIST_MOVE_UP, LIST_PAGE_DOWN, LIST_PAGE_UP,
};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::screens::select::list::{Browse, List, Role, Secondary, normalize_paste};

use crate::app::theme;
use crate::keymap::{keymap, label, pressed};
use crate::widgets::session_list::page_rows;
use crate::widgets::tab_strip::Tabs;

mod pane;
mod rows;
mod side;
pub mod strings;

use self::pane::Pane;
use self::rows::{RowStyles, Rows, Words};

/// What the user did in the picker: the opener turns each into its intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The row with this id was accepted.
    Choose(String),
    /// The secondary action of the row with this id.
    Secondary(String),
    /// The toggle of the row with this id was flipped.
    Toggle(String),
    /// The orderable row with this id moves one place (`true`: down).
    Reorder(String, bool),
    /// Enter on a multi-select: the toggled-on rows are the answer.
    Confirm,
    /// Esc, or Enter with nothing to pick.
    Cancel,
}

/// The picker: the list as props, one callback for what the user does.
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub list: Rc<List>,
    pub on_event: Handler<Event>,
}

/// What the key callback must know to tell a key for the list from one for the
/// search field (it runs before the field sees the key).
#[derive(Clone, Copy, Default)]
struct Flags {
    searchable: bool,
    tabs: bool,
    query_empty: bool,
    toggle: bool,
    ordering: bool,
    secondary: Option<char>,
}

pub struct PickerState {
    node: FocusNode,
    /// The id of the row that keeps the cursor when the list is next
    /// replaced (a reordered row).
    follow: Option<String>,
    query: TextEditingController,
    flags: Rc<Cell<Flags>>,
    /// The user's list keys (`tui.keymap.list.*`), read from the keymap the
    /// app provides each build; the key callback runs between builds.
    keys: Rc<RefCell<Rc<Keymap>>>,
    /// The tabs' selection (`TabController`): the tab strip's taps and the
    /// arrow keys both set it, and the dialog follows through its own
    /// message. The only holder of the tab.
    tabs: Option<TabController>,
    /// The highlighted item (an index into the tab's items).
    cursor: SelectionController<usize>,
    /// The scroll of the rows: asked to show the cursor row, read for the
    /// arrows and the size of a page.
    scroll: ListController,
    /// The rows the list showed when last heard from (a resize moves it).
    viewport: usize,
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
    /// The list's position moved: its window may have shrunk.
    Viewport,
}

impl StatefulWidget for Picker {
    type State = PickerState;

    fn create_state(&self) -> PickerState {
        PickerState {
            node: FocusNode::new(),
            follow: None,
            query: TextEditingController::new(""),
            flags: Rc::new(Cell::new(Flags::default())),
            keys: Rc::new(RefCell::new(Rc::new(Keymap::defaults()))),
            tabs: None,
            cursor: SelectionController::new(),
            scroll: ListController::new(),
            viewport: 0,
        }
    }
}

impl_widget!(stateful eq Picker);

/// The list actions that move the cursor, each with the act it means.
const MOVES: [(ActionId, Act); 6] = [
    (LIST_MOVE_UP, Act::Up),
    (LIST_MOVE_DOWN, Act::Down),
    (LIST_PAGE_UP, Act::PageUp),
    (LIST_PAGE_DOWN, Act::PageDown),
    (LIST_JUMP_TOP, Act::Top),
    (LIST_JUMP_BOTTOM, Act::Bottom),
];

/// Every list action: a key bound to one of them is not free for a row's
/// toggle (`keymap.action_for(key).is_none()`).
const LIST_ACTIONS: [ActionId; 10] = [
    LIST_MOVE_UP,
    LIST_MOVE_DOWN,
    LIST_MOVE_LEFT,
    LIST_MOVE_RIGHT,
    LIST_PAGE_UP,
    LIST_PAGE_DOWN,
    LIST_JUMP_TOP,
    LIST_JUMP_BOTTOM,
    LIST_ACCEPT,
    LIST_CANCEL,
];

/// The act a key means under the user's list keymap
/// (`ListSelectionView::handle_key_event`); none: the key is not the list's.
/// A search line keeps the printable keys for its query, except those the
/// keymap binds to accept or cancel.
fn act_of(key: &KeyPress, flags: Flags, keys: &Keymap) -> Option<Act> {
    // A terminal sends ctrl+h as Backspace (the C0 character): it is the
    // keymap's `ctrl-h`, as the oracle reads control characters.
    let c0;
    let key = if key.code == KeyCode::Backspace && key.mods == Mods::CTRL {
        c0 = KeyCombo::parse("ctrl+h").map_or_else(|_| key.clone(), KeyCombo::press);
        &c0
    } else {
        key
    };
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
        if let Some((_, act)) = MOVES.iter().find(|(id, _)| pressed(keys, *id, key)) {
            return Some(*act);
        }
        if flags.tabs || flags.ordering {
            if pressed(keys, LIST_MOVE_LEFT, key) {
                return Some(Act::Left);
            }
            if pressed(keys, LIST_MOVE_RIGHT, key) {
                return Some(Act::Right);
            }
        }
    }
    if key.code == KeyCode::Backspace {
        return None;
    }
    if key.code == KeyCode::Char
        && letter == Some(' ')
        && key.mods.is_empty()
        && flags.toggle
        && !LIST_ACTIONS.iter().any(|id| pressed(keys, *id, key))
        && (!flags.searchable || flags.query_empty)
    {
        return Some(Act::Toggle);
    }
    if pressed(keys, LIST_CANCEL, key) {
        return Some(Act::Cancel);
    }
    if pressed(keys, LIST_ACCEPT, key) {
        return Some(Act::Accept);
    }
    if navigates && plain_text && letter.is_some() && letter == flags.secondary {
        return Some(Act::Secondary);
    }
    if flags.searchable || !plain_text {
        return None;
    }
    letter.map(Act::Shortcut)
}

/// The footer of a list that says what its keys are (`enter select · esc
/// back`): the first binding of accept and of cancel, as the keymap labels it;
/// an unbound action is left out.
fn standard_hint(keys: &Keymap, s: &strings::Strings) -> String {
    [(LIST_ACCEPT, s.hint_select), (LIST_CANCEL, s.hint_back)]
        .into_iter()
        .filter_map(|(id, word)| label(keys, id).map(|key| format!("{key} {word}")))
        .collect::<Vec<_>>()
        .join(s.hint_join)
}

/// The footer of a row with a second accept key (`SelectionSecondaryAction`'s
/// `footer_hint`): the written hint, or with an `accept` word the keys the
/// keymap names around it, an unbound action left out.
fn secondary_hint(keys: &Keymap, action: &Secondary, s: &strings::Strings) -> String {
    let Some(accept) = &action.accept else {
        return action.hint.clone();
    };
    [
        label(keys, LIST_ACCEPT).map(|key| (key, accept.as_str())),
        Some((action.key.to_string(), action.hint.as_str())),
        label(keys, LIST_CANCEL).map(|key| (key, s.hint_back)),
    ]
    .into_iter()
    .flatten()
    .map(|(key, word)| format!("{key} {word}"))
    .collect::<Vec<_>>()
    .join(s.hint_join)
}

impl State for PickerState {
    type Widget = Picker;
    type Msg = Msg;

    fn init_state(&mut self, w: &Picker, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let flags = self.flags.clone();
        let keys = self.keys.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            let keys = keys.borrow().clone();
            match act_of(&key, flags.get(), &keys) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            }
        })));
        if let Some(first) = w.list.first_tab() {
            let tabs = TabController::with_initial_index(w.list.tabs.len(), first);
            let send = cx.local_sender();
            let seen = tabs.clone();
            tabs.add_listener(Rc::new(move || send(Msg::Tab(seen.index()))));
            self.tabs = Some(tabs);
        }
        // A shorter window (a smaller terminal) keeps the cursor row in view:
        // Flutter's `Scrollable.ensureVisible` after a change of size.
        let send = cx.local_sender();
        self.scroll
            .add_listener(Rc::new(move || send(Msg::Viewport)));
        self.settle(&w.list, None);
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.query.dispose();
        self.cursor.dispose();
    }

    fn did_update_widget(&mut self, old: &Picker, w: &Picker, _: &mut Cx<'_, Self>) {
        if old.list == w.list {
            return;
        }
        let query = self.query.text();
        let browse = self.browse(&w.list, &query);
        if let Some(id) = self.follow.take()
            && let Some(item) = browse.items().iter().position(|item| item.id == id)
        {
            // A moved row keeps the cursor.
            let visible = browse.visible();
            self.cursor.set_cursor(Some(item));
            if let Some(pos) = visible.iter().position(|v| *v == item) {
                self.scroll
                    .ensure_index_visible(browse.entry_of(&visible, pos));
            }
        } else if let Some(item) = self.kept(&old.list, &browse) {
            // The rows changed under the cursor (a refresh): the row with the
            // id that was highlighted keeps it, wherever it now is.
            let visible = browse.visible();
            self.cursor.set_cursor(Some(item));
            if let Some(pos) = visible.iter().position(|v| *v == item) {
                self.scroll
                    .ensure_index_visible(browse.entry_of(&visible, pos));
            }
        } else if old.list.items_of(self.tab()).len() != w.list.items_of(self.tab()).len() {
            // What was highlighted is gone: keep the place where it can be.
            let before = self.cursor.cursor();
            self.settle(&w.list, before);
        }
    }

    fn update(&mut self, msg: Msg, w: &Picker, cx: &mut Cx<'_, Self>) -> Rebuild {
        let list = w.list.clone();
        let list = &*list;
        match msg {
            Msg::Query => {
                let before = self.cursor.cursor();
                self.settle(list, before);
            }
            Msg::Tab(_) => {
                // Another tab has its own rows and an empty search.
                self.query.set_text("");
                self.settle(list, None);
            }
            Msg::TapTab(tab) => {
                if let Some(tabs) = &self.tabs {
                    tabs.set_index(tab);
                }
            }
            Msg::Row(item) => {
                self.cursor.set_cursor(Some(item));
                self.pick(list, w, cx);
            }
            Msg::Act(act) => self.act(act, list, w, cx),
            Msg::Viewport => {
                let rows = page_rows(&self.scroll);
                if rows < std::mem::replace(&mut self.viewport, rows) {
                    self.show_cursor(list);
                }
                return Rebuild::No;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Picker, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let list = (*w.list).clone();
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let keys = keymap(cx);
        *self.keys.borrow_mut() = keys.clone();
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
            ordering: list.ordering && list.tabs.is_empty() && query.is_empty(),
            secondary: selected.and_then(|item| item.secondary.as_ref().map(|a| a.key)),
        });

        let search = list.searchable.then(|| self.search_line(&list, cx));
        let tabs = (!list.tabs.is_empty()).then(|| self.tab_strip(&list, &tokens, s, cx));
        // Nothing listed (a search without a match): no list under the controller, no arrows.
        let listed = !visible.is_empty();
        let rows = listed.then(|| {
            Rc::new(Rows {
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
                    cursor_description: tokens.accent(),
                    label: Style::new(),
                    description: tokens.dim(),
                    disabled: tokens.dim(),
                    key: Style::new().attrs(Attrs::UNDERLINE),
                    success: tokens.success().attrs(Attrs::ITALIC),
                    warning: tokens.warning().attrs(Attrs::ITALIC),
                    info: tokens.info(),
                },
                single_line: list.single_line,
                name_width: list.name_column_width,
                hide_description_below: list.hide_description_below.map(usize::from),
                words: Words {
                    current: s.current,
                    disabled: s.disabled_mark,
                    reason_after: s.reason_after,
                    // A list that hides its description column when narrow shows
                    // a disabled row's reason bare (`combined_description`).
                    reason_alone: if list.hide_description_below.is_some() {
                        ""
                    } else {
                        s.reason_alone
                    },
                    reason_close: s.reason_close,
                    mark_on: s.mark_on,
                    mark_off: s.mark_off,
                    section_break: s.section_break,
                },
            })
        });
        let hint = selected
            .and_then(|item| item.footer_hint.clone())
            .or_else(|| {
                let action = selected?.secondary.as_ref()?;
                Some(secondary_hint(&keys, action, s))
            })
            .or_else(|| list.footer_hint.clone())
            .or_else(|| list.standard_hint.then(|| standard_hint(&keys, s)));
        let pane = Pane {
            surface: list.surface(),
            title: list.title.clone().map(|title| (title, tokens.bold())),
            subtitle: list.subtitle.clone().map(|text| {
                let style = if list.subtitle_role == Role::Info {
                    tokens.info()
                } else {
                    tokens.dim()
                };
                (text, style)
            }),
            tabs,
            search,
            rows,
            empty: (s.no_matches.to_string(), tokens.dim().attrs(Attrs::ITALIC)),
            above: self.arrow(true, s.more_above, tokens.accent(), listed),
            below: self.arrow(false, s.more_below, tokens.accent(), listed),
            side: list.side.clone().map(|side| (side, Style::new())),
            preview: list.preview_names().map(|names| names.join(s.preview_join)),
            note: list.footer_note.clone(),
            hint: hint.map(|hint| (hint, tokens.dim())),
        };
        let dialog = Dialog::bare(BoxWidget::new().style(Style::new()).child(pane.view()));
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

impl PickerState {
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

    /// The index in the new rows of the row highlighted in `old`, matched by
    /// id (a keyed list in Flutter), when it moved.
    fn kept(&self, old: &List, browse: &Browse<'_>) -> Option<usize> {
        let at = self.cursor.cursor()?;
        let before = old
            .items_of(self.tab())
            .get(at)
            .filter(|item| !item.id.is_empty())?;
        let found = browse
            .items()
            .iter()
            .position(|item| item.id == before.id)?;
        (found != at).then_some(found)
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
            self.scroll
                .ensure_index_visible(browse.entry_of(&visible, pos));
        }
    }

    /// Scrolls the least that shows the cursor row.
    fn show_cursor(&self, list: &List) {
        let query = self.query.text();
        let browse = self.browse(list, &query);
        let visible = browse.visible();
        let at = self
            .cursor
            .cursor()
            .and_then(|item| visible.iter().position(|v| *v == item));
        if let Some(pos) = at {
            self.scroll
                .ensure_index_visible(browse.entry_of(&visible, pos));
        }
    }

    fn act(&mut self, act: Act, list: &List, w: &Picker, cx: &mut Cx<'_, Self>) {
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
                    return;
                }
                if list.ordering {
                    let down = act == Act::Right;
                    if let Some(id) = self.id(list).filter(|id| list.can_reorder(id, down)) {
                        // The moved row keeps the cursor when the rows return.
                        self.follow = Some(id.clone());
                        Self::send(w, cx, Event::Reorder(id, down));
                    }
                }
                return;
            }
            Act::Accept if list.multi => return Self::send(w, cx, Event::Confirm),
            Act::Accept => return self.accept(list, w, cx),
            Act::Cancel => return Self::send(w, cx, Event::Cancel),
            Act::Toggle => return self.toggle(list, w, cx),
            Act::Secondary => return self.secondary(list, w, cx),
            Act::Shortcut(key) => {
                if let Some(item) = browse.shortcut(key) {
                    self.cursor.set_cursor(Some(item));
                    self.pick(list, w, cx);
                }
                return;
            }
        };
        if let Some(pos) = to {
            self.cursor.set_cursor(visible.get(pos).copied());
            self.scroll
                .ensure_index_visible(browse.entry_of(&visible, pos));
        }
    }

    fn id(&self, list: &List) -> Option<String> {
        self.cursor
            .cursor()
            .map(|item| list.id_of(self.tab(), item))
    }

    fn send(w: &Picker, cx: &mut Cx<'_, Self>, event: Event) {
        cx.emit(&w.on_event, event);
    }

    /// A click or a shortcut on a row: accepts it, and in a multi-select
    /// flips it (the choice is made by Enter).
    fn pick(&mut self, list: &List, w: &Picker, cx: &mut Cx<'_, Self>) {
        if list.multi {
            self.toggle(list, w, cx);
        } else {
            self.accept(list, w, cx);
        }
    }

    fn accept(&mut self, list: &List, w: &Picker, cx: &mut Cx<'_, Self>) {
        let Some(item) = self.cursor.cursor() else {
            // Nothing matches: the oracle takes it for a cancel.
            return Self::send(w, cx, Event::Cancel);
        };
        let disabled = list
            .items_of(self.tab())
            .get(item)
            .is_none_or(codex_tui_core::screens::select::list::Item::is_disabled);
        if disabled {
            return;
        }
        Self::send(w, cx, Event::Choose(list.id_of(self.tab(), item)));
    }

    fn toggle(&mut self, list: &List, w: &Picker, cx: &mut Cx<'_, Self>) {
        if let Some(id) = self.id(list) {
            Self::send(w, cx, Event::Toggle(id));
        }
    }

    fn secondary(&mut self, list: &List, w: &Picker, cx: &mut Cx<'_, Self>) {
        if let Some(id) = self.id(list) {
            Self::send(w, cx, Event::Secondary(id));
        }
    }

    /// The search line: a `TextField` over the query controller, two cells in
    /// on both sides.
    fn search_line(&self, list: &List, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let placeholder = list.search_placeholder.clone().unwrap_or_default();
        Row::new()
            .child(SizedBox::new(Some(2), None, None))
            .child(Expanded::new(
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
            ))
            .child(SizedBox::new(Some(2), None, None))
            .into()
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
    fn arrow(&self, above: bool, glyph: &'static str, style: Style, listed: bool) -> AnyWidget {
        let scroll = self.scroll.clone();
        let watched = Rc::new((*self.scroll).clone());
        SizedBox::new(
            None,
            Some(1),
            Some(
                ListenableBuilder::new(watched, move |_, _| {
                    let shown = listed
                        && scroll.try_position().is_ok_and(|at| {
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
