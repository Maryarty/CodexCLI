//! The syntax theme picker (the oracle's `ListSelectionView` configured by
//! `theme_picker.rs`: `build_theme_picker_params`): a title, a subtitle, a
//! search line, the themes with a `›` on the cursor row, a sample diff as
//! the preview (beside the rows when the sheet is wide, below them when it is
//! not) and a key hint. In Flutter it is a bottom sheet whose content is a
//! `LayoutBuilder` choosing between a `Row` (rows, preview) and a `Column`
//! (rows, preview) by the width it is given, over a `TextField` with a
//! `TextEditingController` (its `FocusNode.onKeyEvent` takes the keys that
//! are the list's), a `ListView` over a `ListController` with the cursor in a
//! `SelectionController` keyed by the theme's name. The theme in force while
//! the picker is open is the model's (`Intent::SyntaxTheme` on every move of
//! the cursor and on Esc, which restores it; `Edit::ThemeSelected` keeps it,
//! ADR-013).
//! The mouse (ADR-011, beyond the oracle): a tap moves the cursor to a row, a
//! tap on the row the cursor is on picks it; the wheel is the `ListView`'s.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    Align, Alignment, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, IntrinsicHeight,
    MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::{Attrs, Span, Style, TerminalBrightness};
use rtwk::text::{Text, TextAlign, TextWrap, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk::{Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use crate::widgets::picker::list::{SIDE_GAP, Side, SideWidth};
use codex_tui_core::keymap::Keymap;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::{Edit, Intent, Offer, ThemePicker, Which};

use super::keys::{Keys, ListKey, list_key};
use crate::features::{verb, verb_by};
use super::strings::{self, Change, PreviewRow, Strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::session_list::{Move, Scrolled, page_rows, target};

/// Rows of the list shown at once (`MAX_POPUP_ROWS`).
const WINDOW: usize = 8;
/// The narrowest side panel that sits beside the rows
/// (`WIDE_PREVIEW_MIN_WIDTH`).
const WIDE_MIN: u16 = 44;
/// Cells the wide preview leaves free before its first column
/// (`WIDE_PREVIEW_LEFT_INSET`).
const WIDE_LEFT: i32 = 2;
/// Cells the sheet keeps free on either side of its content.
const INSET: u16 = 2;

/// The sheet of the theme picker.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSheet;

/// Widget state: the controllers (R14).
pub struct ThemeState {
    keys: Keys,
    ready: bool,
    node: FocusNode,
    query: TextEditingController,
    cursor: SelectionController<String>,
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
    /// The door to the app, kept: the unmounting element cannot read it (`dispose_with`).
    hs: Option<Rc<Hs>>,
    /// Esc, Enter or a pick has said which theme is in force: a sheet that leaves without it (a
    /// click on the barrier) restores the theme the picker opened on, as Esc does.
    settled: Cell<bool>,
}

/// What a key means to the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Pick,
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

/// The act of a key; none: the key is the search field's.
fn act_of(keys: &Keymap, key: &KeyPress) -> Option<Act> {
    match list_key(keys, key, true)? {
        ListKey::Move(mv) => Some(Act::Move(mv)),
        ListKey::Accept => Some(Act::Pick),
        ListKey::Cancel => Some(Act::Close),
        ListKey::Left | ListKey::Right => None,
    }
}

impl StatefulWidget for ThemeSheet {
    type State = ThemeState;

    fn create_state(&self) -> ThemeState {
        ThemeState {
            keys: Keys::new(),
            ready: false,
            node: FocusNode::new(),
            query: TextEditingController::new(""),
            cursor: SelectionController::new(),
            list: ListController::new(),
            hs: None,
            settled: Cell::new(false),
            scroll: Rc::new(ValueNotifier::new(Scrolled::default())),
        }
    }
}

impl_widget!(stateful eq ThemeSheet);

/// The picker the model offers, as the sheet draws it: the theme in force is the one the
/// config names when it is among the themes, else the terminal's default theme
/// (`build_theme_picker_params`: `effective_name`), so the cursor starts on it and the row says
/// "(current)".
fn open_picker(cx: &mut Cx<'_, ThemeState>) -> Option<ThemePicker> {
    let offered = offered_picker(cx)?;
    let in_force = offered
        .original
        .clone()
        .filter(|name| offered.themes.contains(name))
        .unwrap_or_else(|| adaptive_default(TerminalBrightness::background_of(cx)).to_owned());
    Some(ThemePicker {
        original: Some(in_force),
        ..offered
    })
}

/// The picker as the model offers it: `original` is the theme the config names, if any.
fn offered_picker(cx: &mut Cx<'_, ThemeState>) -> Option<ThemePicker> {
    select::<FeatureStates, Option<ThemePicker>, _>(cx, |states| {
        match states.settings.offers.get(&Which::Theme) {
            Some(Offer::Theme(picker)) => Some(picker.clone()),
            _ => None,
        }
    })
}

/// The theme a terminal gets when the config names none (`adaptive_default_theme_selection`):
/// catppuccin-mocha, catppuccin-latte when its background is light (luma above 128).
fn adaptive_default(terminal_bg: Option<u32>) -> &'static str {
    let light = terminal_bg.is_some_and(|rgb| {
        let [_, r, g, b] = rgb.to_be_bytes();
        0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b) > 128.0
    });
    if light {
        "catppuccin-latte"
    } else {
        "catppuccin-mocha"
    }
}

impl ThemeState {
    /// The row the cursor is on, as a position in `rows`.
    fn at(&self, picker: &ThemePicker, rows: &[usize]) -> Option<usize> {
        let key = self.cursor.cursor()?;
        rows.iter().position(|row| picker.themes[*row] == key)
    }

    /// Puts the cursor on the row at `at`, keeps it in view and, when the
    /// theme under it changed, shows that theme (`on_selection_changed`).
    fn put(&self, picker: &ThemePicker, rows: &[usize], at: usize, cx: &mut Cx<'_, Self>) {
        let name = picker.themes[rows[at]].clone();
        let moved = self.cursor.cursor().as_ref() != Some(&name);
        self.cursor.select(name.clone());
        self.cursor.set_cursor(Some(name.clone()));
        self.list.ensure_index_visible(at);
        if moved {
            verb(cx, Intent::SyntaxTheme(Some(name)));
        }
    }

    /// The rows changed (the search): the cursor stays on its theme when it
    /// is still there, else goes to the first row.
    fn settle(&self, picker: &ThemePicker, rows: &[usize], cx: &mut Cx<'_, Self>) {
        let keep = self.at(picker, rows);
        if let Some(at) = keep.or((!rows.is_empty()).then_some(0)) {
            self.put(picker, rows, at, cx);
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

    /// Esc: the theme in force is the one the picker opened on again (the config's, if it names
    /// one: none is the terminal's default).
    fn cancel(&self, cx: &mut Cx<'_, Self>) {
        self.settled.set(true);
        let original = offered_picker(cx).and_then(|picker| picker.original);
        verb(cx, Intent::SyntaxTheme(original));
        close_dialog::<_, ()>(cx, None);
    }

    fn act(&self, act: Act, picker: &ThemePicker, rows: &[usize], cx: &mut Cx<'_, Self>) {
        let at = self.at(picker, rows);
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(rows.len(), at, &|_| true, mv, page_rows(&self.list)) {
                    self.put(picker, rows, to, cx);
                }
            }
            // Enter with no row left cancels (`enter_with_no_matches_triggers_cancel_callback`).
            Act::Pick => match at {
                Some(at) => {
                    self.settled.set(true);
                    let theme = picker.themes[rows[at]].clone();
                    verb(cx, Intent::Request(Edit::ThemeSelected(theme)));
                    close_dialog::<_, ()>(cx, None);
                }
                None => self.cancel(cx),
            },
            Act::Close => self.cancel(cx),
        }
    }
}

impl State for ThemeState {
    type Widget = ThemeSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &ThemeSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let keys = self.keys.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            match act_of(&keys.get(), &key) {
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

    /// The sheet left the tree without a decision (a click on the barrier): the preview is over,
    /// the theme in force is the config's again (`Intent::SyntaxTheme(None)`).
    fn dispose_with(&mut self, cx: &mut Cx<'_, Self>) {
        if !self.settled.get() && let Some(hs) = &self.hs {
            verb_by(cx, hs, Intent::SyntaxTheme(None));
        }
        self.dispose();
    }

    fn did_change_dependencies(&mut self, _: &ThemeSheet, cx: &mut Cx<'_, Self>) {
        self.keys.refresh(cx);
        if self.hs.is_none() {
            self.hs = Some(read::<Hs, _>(cx));
        }
        let picker = open_picker(cx);
        if let (Some(picker), false) = (&picker, self.ready) {
            self.ready = true;
            // The cursor starts on the theme in force, without a preview of it.
            let at = picker.initial();
            if let Some(name) = picker.themes.get(at) {
                self.cursor.select(name.clone());
                self.cursor.set_cursor(Some(name.clone()));
                self.list.ensure_index_visible(at);
            }
            let rows = picker.visible("");
            let len = i64::try_from(rows.len()).unwrap_or(0);
            let window = i64::try_from(WINDOW).unwrap_or(8);
            self.scroll.set_value(Scrolled {
                pixels: 0,
                max: (len - window).max(0),
            });
        }
    }

    fn update(&mut self, msg: Msg, _: &ThemeSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(picker) = open_picker(cx) else {
            return Rebuild::No;
        };
        let rows = picker.visible(&self.query.text());
        match msg {
            Msg::Query => self.settle(&picker, &rows, cx),
            Msg::Act(act) => self.act(act, &picker, &rows, cx),
            Msg::Row(row) => {
                if self.at(&picker, &rows) == Some(row) {
                    self.act(Act::Pick, &picker, &rows, cx);
                } else if row < rows.len() {
                    self.put(&picker, &rows, row, cx);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &ThemeSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let seg = segmenter_of(cx);
        let syntax = SyntaxTheme::of(cx);
        let Some(picker) = open_picker(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let picker = Rc::new(picker);
        let taps: Rc<Vec<_>> = Rc::new(
            (0..picker.themes.len())
                .map(|row| cx.map(move |_: PointerEvent| Msg::Row(row)))
                .collect(),
        );
        let side = Side {
            width: SideWidth::Half,
            min_width: WIDE_MIN,
            ..Side::default()
        };
        let sheet = {
            let (tokens, query, cursor, list, scroll, node, change) = (
                tokens.clone(),
                self.query.clone(),
                self.cursor.clone(),
                self.list.clone(),
                self.scroll.clone(),
                self.node.clone(),
                cx.map(|_: Rc<str>| Msg::Query),
            );
            LayoutBuilder::new(move |_, constraints| {
                let width = u16::try_from(constraints.max_width.max(0)).unwrap_or(u16::MAX);
                let content = width.saturating_sub(2 * INSET);
                let beside = side.beside(content);
                // The subtitle names the directory of the custom themes when
                // that fits above the rows (`theme_picker_subtitle`).
                let room = match beside {
                    Some(preview) => content.saturating_sub(SIDE_GAP + preview),
                    None => content,
                };
                let subtitle = picker
                    .themes_dir
                    .as_deref()
                    .filter(|dir| dir.starts_with('~'))
                    .map(|dir| s.custom_dir(dir).to_string())
                    .filter(|text| seg.width(text) <= usize::from(room))
                    .unwrap_or_else(|| s.theme_subtitle.to_string());
                let body = header_and_rows(
                    s, &tokens, &subtitle, &picker, &query, &cursor, &list, &scroll, &node,
                    &change, &taps,
                );
                match beside {
                    Some(preview) => IntrinsicHeight::new(
                        Row::new()
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(Expanded::new(1, body))
                            .child(SizedBox::new(Some(i32::from(SIDE_GAP)), None, None))
                            .child(SizedBox::new(
                                Some(i32::from(preview)),
                                None,
                                Some(
                                    Align::new(
                                        Alignment::CENTER_LEFT,
                                        wide_preview(s, &tokens, &syntax),
                                    )
                                    .into(),
                                ),
                            ))
                            .child(SizedBox::new(Some(i32::from(INSET)), None, None)),
                    )
                    .into(),
                    None => Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(body)
                        .child(Text::new(String::new()))
                        .child(Padding::new(
                            EdgeInsets::only(i32::from(INSET), 0, 0, 0),
                            narrow_preview(s, &tokens, &syntax),
                        ))
                        .into(),
                }
            })
        };
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            // the panel's padding row: the preview centres below it, in the content area
            .child(Text::new(String::new()))
            .child(sheet)
            .child(Text::line(format!("{}{}", s.indent, s.popup_hint)).style(tokens.dim()));
        Dialog::bare(BoxWidget::new().style(Style::new()).child(column)).into()
    }
}

/// The title, the subtitle, the search line and the rows between their marks.
#[allow(clippy::too_many_arguments)]
fn header_and_rows(
    s: &'static Strings,
    tokens: &Tokens,
    subtitle: &str,
    picker: &Rc<ThemePicker>,
    query: &TextEditingController,
    cursor: &SelectionController<String>,
    list: &ListController,
    scroll: &Rc<ValueNotifier<Scrolled>>,
    node: &FocusNode,
    change: &rtwk::Handler<Rc<str>>,
    taps: &Rc<Vec<rtwk::Handler<PointerEvent>>>,
) -> AnyWidget {
    let rows_view = {
        let (picker, tokens, query, cursor, list, scroll, taps) = (
            picker.clone(),
            tokens.clone(),
            query.clone(),
            cursor.clone(),
            list.clone(),
            scroll.clone(),
            taps.clone(),
        );
        ListenableBuilder::new(query.listenable(), move |_, _| {
            let rows = Rc::new(picker.visible(&query.text()));
            if rows.is_empty() {
                return Text::line(s.no_matches.to_string())
                    .style(tokens.dim().attrs(Attrs::ITALIC))
                    .into();
            }
            let items = {
                let (picker, rows, tokens, cursor, taps) = (
                    picker.clone(),
                    rows.clone(),
                    tokens.clone(),
                    cursor.clone(),
                    taps.clone(),
                );
                ListView::builder(rows.len(), move |at: usize| -> AnyWidget {
                    let index = rows[at];
                    let row = theme_row(s, &picker, index, &cursor, &tokens);
                    match taps.get(at) {
                        Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(row).into(),
                        None => row,
                    }
                })
                .controller(list.clone())
                .item_extent(1)
            };
            let scrolled =
                NotificationListener::<ScrollNotification>::new(items).on_notification({
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
                Some(i32::try_from(rows.len().min(WINDOW)).unwrap_or(1)),
                Some(scrolled.into()),
            )
            .into()
        })
    };
    let mark = |above: bool| -> AnyWidget {
        let style = tokens.accent();
        ValueListenableBuilder::new(scroll.clone(), move |_, scrolled: &Scrolled, _| {
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
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(format!("{}{}", s.indent, s.theme_title)).style(tokens.bold()))
        .child(Text::line(format!("{}{subtitle}", s.indent)).style(tokens.dim()))
        .child(Text::new(String::new()))
        .child(
            Row::new()
                .child(SizedBox::new(Some(2), None, None))
                .child(Expanded::new(
                    1,
                    TextField::new()
                        .controller(query.clone())
                        .placeholder(s.theme_search_placeholder)
                        .focus_node(node.clone())
                        .autofocus(true)
                        .on_changed(change.clone()),
                )),
        )
        .child(mark(true))
        .child(rows_view)
        .child(mark(false))
        .into()
}

/// A row of a theme: the marker of the cursor, the name, "(custom)" for the
/// user's own files and "(current)" for the theme in force.
fn theme_row(
    s: &'static Strings,
    picker: &ThemePicker,
    index: usize,
    cursor: &SelectionController<String>,
    tokens: &Tokens,
) -> AnyWidget {
    let name = &picker.themes[index];
    let on = cursor.cursor().as_ref() == Some(name);
    let current = picker.original.as_deref() == Some(name.as_str());
    let text = format!(
        "{}{}{}{}",
        if on { s.cursor_marker } else { s.indent },
        name,
        if picker.is_custom(name) {
            s.theme_custom_mark
        } else {
            ""
        },
        if current { s.current_marker } else { "" },
    );
    let style = if on {
        tokens.accent().attrs(Attrs::BOLD)
    } else {
        Style::new()
    };
    Text::line(text).style(style).into()
}

/// The wide preview: all its lines, left inset 2, centred by the caller.
fn wide_preview(s: &'static Strings, tokens: &Tokens, syntax: &SyntaxTheme) -> AnyWidget {
    let rows = s.theme_wide;
    let lines = diff_lines(s, &rows, tokens, syntax);
    Padding::new(EdgeInsets::only(WIDE_LEFT, 0, 0, 0), lines).into()
}

/// The narrow preview below the rows: the four lines of the compact sample.
fn narrow_preview(s: &'static Strings, tokens: &Tokens, syntax: &SyntaxTheme) -> AnyWidget {
    let rows = s.theme_narrow;
    diff_lines(s, &rows, tokens, syntax)
}

/// The sample diff: the number column aligned right, the sign, the code
/// highlighted as Rust (`render_preview`).
fn diff_lines(
    s: &'static Strings,
    rows: &[PreviewRow],
    tokens: &Tokens,
    syntax: &SyntaxTheme,
) -> AnyWidget {
    let gutter = rows.iter().map(|row| row.no.len()).max().unwrap_or(1);
    let source = rows
        .iter()
        .map(|row| row.code)
        .collect::<Vec<_>>()
        .join("\n");
    let lexer = Highlighter::new(language_named("rust"), &source);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for (index, row) in rows.iter().enumerate() {
        let (sign, sign_style) = match row.change {
            Change::Added => ("+", tokens.success()),
            Change::Removed => ("-", tokens.error()),
            Change::Context => (" ", Style::new()),
        };
        let body: Vec<Span> = syntax
            .spans(row.code, lexer.tokens(index))
            .into_iter()
            .map(|span| Span::styled(span.text, span.style))
            .collect();
        let body = if body.is_empty() {
            vec![Span::styled(row.code.to_string(), sign_style.clone())]
        } else {
            body
        };
        let fixed = |columns: usize, child: AnyWidget| {
            SizedBox::new(Some(i32::try_from(columns).unwrap_or(1)), None, Some(child))
        };
        column = column.child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(fixed(
                    gutter,
                    Text::new(row.no.to_string())
                        .style(tokens.dim())
                        .text_align(TextAlign::Right)
                        .into(),
                ))
                .child(fixed(
                    2,
                    Text::rich(vec![
                        Span::new(s.diff_gutter.to_string()),
                        Span::styled(sign.to_string(), sign_style),
                    ])
                    .into(),
                ))
                .child(Expanded::new(1, Text::rich(body).wrap(TextWrap::None))),
        );
    }
    column.into()
}
