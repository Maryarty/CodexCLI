//! The status-line and terminal-title setups (the oracle's
//! `MultiSelectPicker`: `bottom_pane/multi_select_picker.rs`, configured by
//! `status_line_setup.rs` and `title_setup.rs`): a title and subtitle, a
//! search line, checkable rows with a description column, arrows for what is
//! hidden, the preview of what the checked rows make and a key hint. In
//! Flutter it is a bottom sheet whose content is a `Column(min)` over a
//! `TextField` with a `TextEditingController` (its `FocusNode.onKeyEvent`
//! takes the keys that are the list's: Space, the arrows), a `ListView`
//! over a `ListController` with the cursor in a `SelectionController` keyed
//! by the item's id. What is checked and in which order is the model's
//! (`Intent::Toggle`, `Intent::Move`, `Intent::Save`, ADR-003); the sheet
//! keeps the controllers only. The mouse (ADR-011): a tap moves the cursor
//! to a row, a tap on the row the cursor is on toggles it; the wheel is the
//! `ListView`'s own.

use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::list::normalize_paste;
use codex_tui_core::screens::settings::{
    Intent, Open, Setup, SetupItem, SetupKind, USE_THEME_COLORS,
};

use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::dots::dots_end;
use crate::widgets::session_list::{Move, Scrolled, page_rows, target};

/// Rows of the list shown at once (`MAX_POPUP_ROWS`).
const WINDOW: usize = 8;

/// One row of the list: an item (an index into `Setup::items`) or the rule
/// after an item with `section_break_after`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Item(usize),
    Break,
}

/// The rows a search leaves; the rule stays only above further rows
/// (`MultiSelectPicker::build_rows`).
fn entries(setup: &Setup, query: &str) -> Vec<Entry> {
    let visible = setup.visible(query);
    let mut rows = Vec::new();
    for (at, index) in visible.iter().enumerate() {
        rows.push(Entry::Item(*index));
        if setup.items[*index].section_break_after && at + 1 < visible.len() {
            rows.push(Entry::Break);
        }
    }
    rows
}

/// The key of a row in the cursor: the item's id; the rule has none.
fn key_of(setup: &Setup, row: Entry) -> String {
    match row {
        Entry::Item(index) => setup.items[index].id.clone(),
        Entry::Break => String::new(),
    }
}

/// The name a row shows: the colour preference has a name of its own, a
/// skill its display name.
fn name_of<'a>(s: &'a Strings, item: &'a SetupItem) -> &'a str {
    if item.id == USE_THEME_COLORS {
        s.use_theme_colors
    } else {
        item.name()
    }
}

/// The sheet of both setups: which one it is, the model says.
#[derive(Clone, Debug, PartialEq)]
pub struct SetupSheet;

/// Widget state: the controllers (R14).
pub struct SetupState {
    shown: bool,
    ready: bool,
    node: FocusNode,
    query: TextEditingController,
    cursor: SelectionController<String>,
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
}

/// What a key means to the list (the oracle's list keymap and `MultiSelectPicker::handle_key_event`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Toggle,
    Reorder(i32),
    Save,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    /// The search text changed.
    Query,
    /// A tap on the row at an index.
    Row(usize),
}

/// The single character a key types, if it types one.
pub(super) fn letter_of(key: &KeyPress) -> Option<char> {
    key.text().and_then(|text| {
        let mut chars = text.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first)
    })
}

/// How a key moves the cursor of a list (the oracle's list keymap).
pub(super) fn move_of(key: &KeyPress) -> Option<Move> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    match (key.code, letter_of(key), ctrl, alt) {
        (KeyCode::Up, _, false, false) | (KeyCode::Char, Some('p'), true, false) => {
            Some(Move::Step(false))
        }
        (KeyCode::Down, _, false, false) | (KeyCode::Char, Some('n'), true, false) => {
            Some(Move::Step(true))
        }
        (KeyCode::PageUp, _, false, false) => Some(Move::Page(false)),
        (KeyCode::PageDown, _, false, false) => Some(Move::Page(true)),
        (KeyCode::Home, _, false, false) => Some(Move::Edge(false)),
        (KeyCode::End, _, false, false) => Some(Move::Edge(true)),
        _ => None,
    }
}

/// The act of a key; none: the key is the search field's.
fn act_of(key: &KeyPress) -> Option<Act> {
    if let Some(mv) = move_of(key) {
        return Some(Act::Move(mv));
    }
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    match (key.code, letter_of(key), ctrl, alt) {
        // Left and Right reorder; with a search typed they move nothing (the
        // oracle's search has no caret to move).
        (KeyCode::Left, _, false, false) => Some(Act::Reorder(-1)),
        (KeyCode::Right, _, false, false) => Some(Act::Reorder(1)),
        (KeyCode::Char, Some(' '), false, false) if key.mods.is_empty() => Some(Act::Toggle),
        (KeyCode::Enter, _, false, false) => Some(Act::Save),
        (KeyCode::Escape, _, false, false) => Some(Act::Close),
        _ => None,
    }
}

impl StatefulWidget for SetupSheet {
    type State = SetupState;

    fn create_state(&self) -> SetupState {
        SetupState {
            shown: false,
            ready: false,
            node: FocusNode::new(),
            query: TextEditingController::new(""),
            cursor: SelectionController::new(),
            list: ListController::new(),
            scroll: Rc::new(ValueNotifier::new(Scrolled::default())),
        }
    }
}

impl_widget!(stateful eq SetupSheet);

/// The setup the model holds open.
fn open_setup(cx: &mut Cx<'_, SetupState>) -> Option<Setup> {
    select::<ScreenStates, Option<Setup>, _>(cx, |states| match &states.settings.open {
        Some(Open::Setup(setup)) => Some(super::items::filled(setup.clone())),
        _ => None,
    })
}

impl SetupState {
    /// The row the cursor is on.
    fn at(&self, setup: &Setup, rows: &[Entry]) -> Option<usize> {
        let key = self.cursor.cursor()?;
        rows.iter()
            .position(|row| matches!(row, Entry::Item(_)) && key_of(setup, *row) == key)
    }

    /// Puts the cursor on the row at `at` and keeps it in view.
    fn put(&self, setup: &Setup, rows: &[Entry], at: usize) {
        let key = key_of(setup, rows[at]);
        self.cursor.select(key.clone());
        self.cursor.set_cursor(Some(key));
        self.list.ensure_index_visible(at);
    }

    /// The rows changed (the search): the cursor stays on its item when it
    /// is still there, else goes to the first row.
    fn settle(&self, setup: &Setup, rows: &[Entry]) {
        let keep = self.at(setup, rows);
        let first = rows.iter().position(|row| matches!(row, Entry::Item(_)));
        if let Some(at) = keep.or(first) {
            self.put(setup, rows, at);
        } else {
            self.cursor.set_cursor(None);
        }
        let len = i64::try_from(rows.len()).unwrap_or(0);
        let window = i64::try_from(WINDOW).unwrap_or(8);
        self.scroll.set_value(Scrolled {
            pixels: 0,
            max: (len - window).max(0),
        });
    }

    fn act(&self, act: Act, setup: &Setup, rows: &[Entry], cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        let at = self.at(setup, rows);
        let id = at.map(|at| key_of(setup, rows[at]));
        match act {
            Act::Move(mv) => {
                let selectable = |row: usize| matches!(rows[row], Entry::Item(_));
                if let Some(to) = target(rows.len(), at, &selectable, mv, page_rows(&self.list)) {
                    self.put(setup, rows, to);
                }
            }
            Act::Toggle => {
                if let Some(id) = id {
                    cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Toggle(id)));
                }
            }
            Act::Reorder(delta) => {
                // Reordering is off while a search is typed, and for the
                // skills, which have no order.
                let ordered = setup.kind != SetupKind::Skills && self.query.text().is_empty();
                if let (Some(id), Some(at), true) = (id, at, ordered) {
                    cx.emit(
                        &hs.screen,
                        ScreenIntent::Settings(Intent::Move { id, delta }),
                    );
                    if let Some(to) = at.checked_add_signed(delta.signum() as isize)
                        && to < rows.len()
                    {
                        self.list.ensure_index_visible(to);
                    }
                }
            }
            // Enter saves the order and the checks; on the skills it flips
            // the row under the cursor like Space (each flip is saved).
            Act::Save if setup.kind == SetupKind::Skills => {
                if let Some(id) = id {
                    cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Toggle(id)));
                }
            }
            Act::Save => cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Save)),
            Act::Close => cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Close)),
        }
    }
}

impl State for SetupState {
    type Widget = SetupSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &SetupSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key| match act_of(&key) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.query.dispose();
        self.cursor.dispose();
    }

    fn did_change_dependencies(&mut self, _: &SetupSheet, cx: &mut Cx<'_, Self>) {
        let setup = open_setup(cx);
        if setup.is_none() && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = setup.is_some();
        if let (Some(setup), false) = (&setup, self.ready) {
            self.ready = true;
            let rows = entries(setup, "");
            self.settle(setup, &rows);
        }
    }

    fn update(&mut self, msg: Msg, _: &SetupSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(setup) = open_setup(cx) else {
            return Rebuild::No;
        };
        let rows = entries(&setup, &self.query.text());
        match msg {
            Msg::Query => self.settle(&setup, &rows),
            Msg::Act(act) => self.act(act, &setup, &rows, cx),
            Msg::Row(row) => {
                if self.at(&setup, &rows) == Some(row) {
                    self.act(Act::Toggle, &setup, &rows, cx);
                } else if matches!(rows.get(row), Some(Entry::Item(_))) {
                    self.put(&setup, &rows, row);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &SetupSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let seg = segmenter_of(cx);
        let Some(setup) = open_setup(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let (title, subtitle, placeholder, hint) = match setup.kind {
            SetupKind::StatusLine => (
                s.status_title,
                s.status_subtitle,
                s.status_search_placeholder,
                s.status_hint,
            ),
            SetupKind::Title => (
                s.title_setup_title,
                s.title_setup_subtitle,
                s.status_search_placeholder,
                s.status_hint,
            ),
            SetupKind::Skills => (
                s.skills_title,
                s.skills_subtitle,
                s.skills_search_placeholder,
                s.skills_hint,
            ),
        };
        // The skills' list area is two rows taller than its rows
        // (`rows_height`: the rows, at most eight, plus two); the others'
        // is as tall as its rows.
        let spare: usize = if setup.kind == SetupKind::Skills {
            2
        } else {
            0
        };
        let setup = Rc::new(setup);
        let (query, cursor, list, scroll) = (
            self.query.clone(),
            self.cursor.clone(),
            self.list.clone(),
            self.scroll.clone(),
        );
        let taps: Rc<Vec<_>> = Rc::new(
            (0..setup.items.len() * 2)
                .map(|row| cx.map(move |_: PointerEvent| Msg::Row(row)))
                .collect(),
        );
        // The rows follow the search text; the marks follow the scroll.
        let body = {
            let (setup, tokens, scroll, cursor) = (setup.clone(), tokens.clone(), scroll, cursor);
            ListenableBuilder::new(self.query.listenable(), move |_, _| {
                let rows = Rc::new(entries(&setup, &query.text()));
                if rows.is_empty() {
                    return Text::line(format!("{}{}", s.indent, s.no_matches))
                        .style(tokens.dim())
                        .into();
                }
                // The name column is as wide as the widest name in view
                // (`ColumnWidthMode::AutoVisible`).
                let widths: Rc<Vec<usize>> = Rc::new(
                    rows.iter()
                        .map(|row| match row {
                            Entry::Item(index) => seg.width(name_of(s, &setup.items[*index])),
                            Entry::Break => 0,
                        })
                        .collect(),
                );
                let items = {
                    let (setup, rows, tokens, scroll, cursor, taps) = (
                        setup.clone(),
                        rows.clone(),
                        tokens.clone(),
                        scroll.clone(),
                        cursor.clone(),
                        taps.clone(),
                    );
                    ListView::builder(rows.len(), move |at: usize| -> AnyWidget {
                        let (setup, rows, tokens, widths, scroll, cursor) = (
                            setup.clone(),
                            rows.clone(),
                            tokens.clone(),
                            widths.clone(),
                            scroll.clone(),
                            cursor.clone(),
                        );
                        let row = ValueListenableBuilder::new(
                            scroll,
                            move |_, scrolled: &Scrolled, _| {
                                let top = usize::try_from(scrolled.pixels).unwrap_or(0);
                                let wide = widths.iter().skip(top).take(WINDOW).copied().max();
                                let wide = i32::try_from(wide.unwrap_or(0)).unwrap_or(0);
                                match rows[at] {
                                    Entry::Break => {
                                        Text::line(format!("{}{}", s.indent, s.section_break))
                                            .style(tokens.dim())
                                            .into()
                                    }
                                    Entry::Item(index) => {
                                        item_row(s, &setup, index, wide, &cursor, &tokens)
                                    }
                                }
                            },
                        );
                        match taps.get(at) {
                            Some(tap) => {
                                GestureDetector::new().on_tap(tap.clone()).child(row).into()
                            }
                            None => row.into(),
                        }
                    })
                    .controller(list.clone())
                    .item_extent(1)
                };
                let scrolled = NotificationListener::<ScrollNotification>::new(items)
                    .on_notification({
                        let scroll = scroll.clone();
                        move |note| {
                            scroll.set_value(Scrolled {
                                pixels: note.metrics.pixels,
                                max: note.metrics.max_scroll_extent,
                            });
                            false
                        }
                    });
                SizedBox::new(
                    None,
                    Some(i32::try_from(rows.len().min(WINDOW) + spare).unwrap_or(1)),
                    Some(scrolled.into()),
                )
                .into()
            })
        };
        let mark = |above: bool| -> AnyWidget {
            let style = tokens.dim();
            ValueListenableBuilder::new(self.scroll.clone(), move |_, scrolled: &Scrolled, _| {
                let on = if above {
                    scrolled.above()
                } else {
                    scrolled.below()
                };
                let text = if on {
                    if above { s.more_above } else { s.more_below }
                } else {
                    ""
                };
                Text::line(text.to_string()).style(style.clone()).into()
            })
            .into()
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::new(String::new()))
            .child(Text::line(format!("{}{title}", s.indent)).style(tokens.bold()))
            .child(Padding::new(
                EdgeInsets::symmetric(2, 0),
                Text::new(subtitle.to_string())
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            ))
            .child(Text::new(String::new()))
            .child(
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
                                // Whitespace folds to one blank; nothing left: ignored
                                // (`normalize_pasted_search_query`).
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
                    )),
            )
            .child(mark(true))
            .child(body)
            .child(mark(false));
        let preview = setup.preview();
        if !preview.is_empty() {
            column = column.child(Text::line(format!("{}{preview}", s.indent)));
        }
        column = column.child(Text::line(format!("{}{hint}", s.indent)).style(tokens.dim()));
        Dialog::bare(BoxWidget::new().style(Style::new()).child(column)).into()
    }
}

/// A row of an item: the marker of the cursor, the check, the name in a
/// column of `wide` cells and the description to the edge, cut with `…`.
fn item_row(
    s: &'static Strings,
    setup: &Setup,
    index: usize,
    wide: i32,
    cursor: &SelectionController<String>,
    tokens: &Tokens,
) -> AnyWidget {
    let item = &setup.items[index];
    let on = cursor.cursor().as_ref() == Some(&item.id);
    let lead = format!(
        "{}{}",
        if on { s.cursor_marker } else { s.indent },
        if item.enabled {
            s.check_on
        } else {
            s.check_off
        }
    );
    let description = if item.id == USE_THEME_COLORS {
        s.use_theme_colors_description
    } else {
        item.description.as_str()
    };
    let name_text = name_of(s, item).to_string();
    let selected = tokens.selected();
    let (lead_style, name_style, description) = if on {
        (
            selected.clone(),
            selected.clone(),
            dots_end(description.to_string(), selected),
        )
    } else {
        (
            Style::new(),
            Style::new(),
            dots_end(description.to_string(), tokens.dim()),
        )
    };
    // The name column is `wide` cells and a gap, but never more than the
    // row has after the check: a name longer than that is cut with `…` and
    // leaves no room for the description (`render_rows_single_line`).
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let taken = i32::try_from(seg.width(&lead)).unwrap_or(0);
        let column = (wide + 2).min((constraints.max_width - taken).max(0));
        Row::new()
            .child(Text::line(lead.clone()).style(lead_style.clone()))
            .child(SizedBox::new(
                Some(column),
                Some(1),
                Some(
                    Text::line(name_text.clone())
                        .style(name_style.clone())
                        .overflow(TextOverflow::Ellipsis)
                        .into(),
                ),
            ))
            .child(Expanded::new(1, description.clone()))
            .into()
    })
    .into()
}
