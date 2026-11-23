//! The hooks browser (the oracle's `HooksBrowserView`:
//! `bottom_pane/hooks_browser_view.rs`, `hooks_browser_render.rs`): the
//! lifecycle hooks the server listed, by event. In Flutter it is a bottom sheet
//! whose content is a `Column(min)` of a header, a scrolling list with its
//! two marks, the details of the row and a footer hint, over two pages: the
//! events, and the hooks of one event. Each page's cursor is a
//! `SelectionController`, the list's scroll a `ListController` (the sheet owns
//! them; going to a page puts them where the oracle's fresh `ScrollState`
//! does), the keys come from the `FocusNode.onKeyEvent` of a `Focus`. Which
//! page is open and what Space and `t` changed before the app takes it are
//! the sheet's draft (`HooksDraft`); what a hook is, and what Space and `t`
//! ask of the app, are the model's (`Hooks`, `Edit::HookEnabled`,
//! `Edit::HookTrusted`, `Edit::HooksTrusted`, ADR-013). The mouse (ADR-011, beyond the oracle):
//! a tap puts the cursor on a row, a tap on the row the cursor is on does
//! what Enter does; the wheel scrolls the list.
//!
//! The events table has two spellings, picked from the measured width
//! (`LayoutBuilder`): under 72 cells of content (the oracle's threshold) it
//! is the narrow one, the counts in one column.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::SelectionController;
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    Flexible, MainAxisSize, Padding, Row, SizedBox, UNBOUNDED,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::select;
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextAlign, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::hooks::{
    EventRow, Hook, HookEvent, HookHandler, HookSource, HookTrust, Hooks,
};
use codex_tui_core::features::settings::{Offer, Which};
use codex_tui_core::keymap::Keymap;

use super::draft::{HooksDraft, HooksView};
use super::keys::{Keys, ListKey, is_interrupt, is_letter, is_space, list_key};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::widgets::dots::dots_end;
use crate::widgets::session_list::{Move, Scrolled, target};

/// The sheet of the hooks browser.
#[derive(Clone, Debug, PartialEq)]
pub struct HooksSheet;

/// Rows of a list shown at once (`MAX_POPUP_ROWS`).
const WINDOW: usize = 8;

/// The width of the events table's columns (`EVENT_COLUMN_WIDTH`,
/// `COUNT_COLUMN_WIDTH`), of the narrow one's name column, and of the cursor
/// marker; under `NARROW` cells of content the table is the narrow one.
const EVENT_COLUMN: i32 = 22;
const COUNT_COLUMN: i32 = 12;
const NARROW_NAME: i32 = 20;
const MARKER: i32 = 2;
const NARROW: i32 = 72;
/// What a surface leaves round its content: two cells either side.
const INSET: i32 = 2;
/// The most lines of a command in the details (`MAX_COMMAND_DETAIL_LINES`).
const COMMAND_LINES: usize = 3;

/// Widget state: the controllers (R14).
pub struct HooksState {
    keys: Keys,
    draft: HooksDraft,
    /// The page the controllers were last put for (none: no page yet).
    seen: Cell<Option<usize>>,
    node: FocusNode,
    /// The cursor of the events page: an index among the events.
    events: SelectionController<usize>,
    /// The cursor of the hooks page: an index among the hooks of the event.
    handlers: SelectionController<usize>,
    /// The list of the page open (one at a time).
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
}

/// What a key means to the browser (the oracle's list keymap and
/// `HooksBrowserView::handle_key_event`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Accept,
    Toggle,
    Trust,
    Close,
    /// Ctrl+C: the browser closes from either page.
    Dismiss,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    /// A tap on the row at an index.
    Row(usize),
}

fn act_of(keys: &Keymap, key: &KeyPress) -> Option<Act> {
    if is_space(key) {
        return Some(Act::Toggle);
    }
    if is_letter(key, 't') {
        return Some(Act::Trust);
    }
    if is_interrupt(key) {
        return Some(Act::Dismiss);
    }
    match list_key(keys, key, false)? {
        ListKey::Move(mv) => Some(Act::Move(mv)),
        ListKey::Accept => Some(Act::Accept),
        ListKey::Cancel => Some(Act::Close),
        ListKey::Left | ListKey::Right => None,
    }
}

impl StatefulWidget for HooksSheet {
    type State = HooksState;

    fn create_state(&self) -> HooksState {
        HooksState {
            keys: Keys::new(),
            draft: HooksDraft::default(),
            seen: Cell::new(None),
            node: FocusNode::new(),
            events: SelectionController::new(),
            handlers: SelectionController::new(),
            list: ListController::new(),
            scroll: Rc::new(ValueNotifier::new(Scrolled::default())),
        }
    }
}

impl_widget!(stateful eq HooksSheet);

/// The browser the model offers.
fn offered(cx: &mut Cx<'_, HooksState>) -> Option<Hooks> {
    select::<FeatureStates, Option<Hooks>, _>(cx, |states| {
        match states.settings.offers.get(&Which::Hooks) {
            Some(Offer::Hooks(browser)) => Some(browser.clone()),
            _ => None,
        }
    })
}

impl HooksState {
    /// The cursor of the page open, and how many rows it has.
    fn active(&self, hooks: &HooksView) -> (&SelectionController<usize>, usize) {
        match hooks.page {
            None => (&self.events, HookEvent::ALL.len()),
            Some(event) => (&self.handlers, hooks.of(event).len()),
        }
    }

    /// Puts the cursor on a row and keeps it in view.
    fn put(&self, cursor: &SelectionController<usize>, at: usize) {
        cursor.select(at);
        cursor.set_cursor(Some(at));
        self.list.ensure_index_visible(at);
    }

    /// The list is at its top, the marks as a window of `WINDOW` rows leaves
    /// them: the oracle's fresh `ScrollState` of a page.
    fn rewind(&self, rows: usize) {
        self.list.jump_to_index(0);
        self.scroll.set_value(Scrolled {
            pixels: 0,
            max: i64::try_from(rows.saturating_sub(WINDOW)).unwrap_or(0),
        });
    }

    /// The model went to another page (or opened): the cursor goes where the
    /// oracle's `open_selected_event` / `return_to_events` put it. Asked by
    /// the build (the model's change reaches no other hook of the state before
    /// the frame draws it) and by the first dependency change; the controllers
    /// are handles, so it takes `&self`.
    fn arrive(&self, hooks: &HooksView) {
        let page = hooks.page.map_or(0, |event| event.index() + 1);
        if self.seen.get() == Some(page) {
            return;
        }
        let first = self.seen.get().is_none();
        self.seen.set(Some(page));
        match hooks.page {
            None => {
                let at = if first {
                    hooks.initial_event()
                } else {
                    self.events.cursor().unwrap_or(0)
                };
                self.rewind(HookEvent::ALL.len());
                self.put(&self.events, at);
            }
            Some(event) => {
                let rows = hooks.of(event).len();
                self.rewind(rows);
                if rows > 0 {
                    self.put(&self.handlers, 0);
                } else {
                    self.handlers.set_cursor(None);
                }
            }
        }
    }

    /// The key of the hook under the cursor of the hooks page.
    fn hook_under_cursor<'a>(&self, hooks: &'a HooksView) -> Option<&'a Hook> {
        let event = hooks.page?;
        let at = self.handlers.cursor()?;
        hooks.of(event).get(at).copied()
    }

    /// `offered` is the browser as the model has it.
    fn act(&mut self, act: Act, offered: &Hooks, cx: &mut Cx<'_, Self>) {
        let hooks = self.draft.view(offered);
        let edit = match act {
            Act::Move(mv) => {
                let (cursor, len) = self.active(&hooks);
                if let Some(to) = target(len, cursor.cursor(), &|_| true, mv, WINDOW.min(len)) {
                    self.put(cursor, to);
                }
                None
            }
            Act::Accept if hooks.page.is_none() => {
                if let Some(at) = self.events.cursor() {
                    self.draft.page = Some(HookEvent::ALL[at]);
                }
                None
            }
            Act::Accept | Act::Toggle if hooks.page.is_some() => {
                let key = self.hook_under_cursor(&hooks).map(|hook| hook.key.clone());
                key.and_then(|key| self.draft.toggle(offered, &key))
            }
            Act::Accept | Act::Toggle => None,
            Act::Trust if hooks.page.is_none() => self.draft.trust_all(offered),
            Act::Trust => {
                let key = self.hook_under_cursor(&hooks).map(|hook| hook.key.clone());
                key.and_then(|key| self.draft.trust(offered, &key))
            }
            // Esc: the hooks of an event go back to the events; the events close.
            Act::Close if hooks.page.is_some() => {
                self.draft.page = None;
                None
            }
            Act::Close | Act::Dismiss => {
                close_dialog::<_, ()>(cx, None);
                None
            }
        };
        if let Some(edit) = edit {
            super::request(cx, edit);
        }
    }
}

impl State for HooksState {
    type Widget = HooksSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &HooksSheet, cx: &mut Cx<'_, Self>) {
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
        self.events.dispose();
        self.handlers.dispose();
    }

    fn did_change_dependencies(&mut self, _: &HooksSheet, cx: &mut Cx<'_, Self>) {
        self.keys.refresh(cx);
        if let Some(offered) = offered(cx) {
            self.arrive(&self.draft.view(&offered));
        }
    }

    fn update(&mut self, msg: Msg, _: &HooksSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(offered) = offered(cx) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Act(act) => self.act(act, &offered, cx),
            Msg::Row(row) => {
                let hooks = self.draft.view(&offered);
                let (cursor, len) = self.active(&hooks);
                if cursor.cursor() == Some(row) {
                    self.act(Act::Accept, &offered, cx);
                } else if row < len {
                    let cursor = cursor.clone();
                    self.put(&cursor, row);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &HooksSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let Some(offered) = offered(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let hooks = self.draft.view(&offered);
        self.arrive(&hooks);
        let surface = tokens.prompt_surface(cx, false);
        let hint = footer_hint(s, &hooks, self.hook_under_cursor(&hooks));
        let footer = Padding::new(
            EdgeInsets::only(INSET, 0, 0, 0),
            Text::line(hint.to_string()).style(tokens.dim()),
        );
        let cursor = match hooks.page {
            None => self.events.clone(),
            Some(_) => self.handlers.clone(),
        };
        let page = Page {
            s,
            tokens: tokens.clone(),
            hooks: Rc::new(hooks),
            cursor,
            list: self.list.clone(),
            scroll: self.scroll.clone(),
            taps: Rc::new(Vec::new()),
        };
        let taps: Rc<Vec<_>> = Rc::new(
            (0..HookEvent::ALL.len().max(page.hooks.hooks.len()))
                .map(|row| cx.map(move |_: PointerEvent| Msg::Row(row)))
                .collect(),
        );
        let page = Page { taps, ..page };
        // The page is laid out in the width the sheet has: the events table
        // picks its spelling from it.
        let body = LayoutBuilder::new(move |_, constraints| {
            let content = constraints.max_width - 2 * INSET;
            page.build(content)
        });
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Flexible::new(
                1,
                BoxWidget::new()
                    .style(surface)
                    .child(Padding::new(EdgeInsets::symmetric(0, 1), body)),
            ))
            .child(footer);
        // The footer row is the sheet's too: what it does not write is
        // cleared, not left to the page under it.
        Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .focus_node(self.node.clone())
        .autofocus()
        .into()
    }
}

/// The footer's hint (`render_footer`, the default keymap).
fn footer_hint(s: &Strings, hooks: &HooksView, hook: Option<&Hook>) -> &'static str {
    match hooks.page {
        None if hooks.review_total() > 0 => s.hooks_hint_events_review,
        None => s.hooks_hint_events,
        Some(_) => match hook {
            Some(hook) if hook.is_managed => s.hooks_hint_handlers_managed,
            Some(hook) if hook.needs_review() => s.hooks_hint_handlers_trust,
            Some(_) => s.hooks_hint_handlers_toggle,
            None => s.hooks_hint_handlers,
        },
    }
}

/// Everything a page is built of.
#[derive(Clone)]
struct Page {
    s: &'static Strings,
    tokens: Tokens,
    hooks: Rc<HooksView>,
    cursor: SelectionController<usize>,
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
    taps: Rc<Vec<Handler<PointerEvent>>>,
}

impl Page {
    fn build(&self, content: i32) -> AnyWidget {
        match self.hooks.page {
            None => self.events_page(content < NARROW),
            Some(event) => self.handlers_page(event),
        }
    }

    fn event_label(&self, event: HookEvent) -> &'static str {
        self.s.hooks_event_labels[event.index()]
    }

    /// A line of text in a cell `width` wide.
    fn cell(text: String, style: Style, width: i32) -> AnyWidget {
        SizedBox::new(
            Some(width),
            Some(1),
            Some(Text::line(text).style(style).into()),
        )
        .into()
    }

    fn blank() -> AnyWidget {
        Text::new(String::new()).into()
    }

    /// The header's lines sit inside the surface's two cells.
    fn header(lines: Vec<AnyWidget>) -> AnyWidget {
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for line in lines {
            column = column.child(line);
        }
        Padding::new(EdgeInsets::symmetric(INSET, 0), column).into()
    }

    fn note(&self, text: String, style: Style) -> AnyWidget {
        Text::new(text).style(style).wrap(TextWrap::Word).into()
    }

    fn review_message(&self, count: usize) -> Option<String> {
        match count {
            0 => None,
            1 => Some(self.s.hooks_review_one.to_string()),
            count => Some(self.s.review_many(count).to_string()),
        }
    }

    /// The mark of rows above or below the window, in the accent colour; the
    /// row stays when it shows nothing.
    fn mark(&self, above: bool) -> AnyWidget {
        let (s, style) = (self.s, self.tokens.accent());
        ValueListenableBuilder::new(self.scroll.clone(), move |_, scrolled: &Scrolled, _| {
            let on = if above {
                scrolled.above()
            } else {
                scrolled.below()
            };
            let text = match (on, above) {
                (false, _) => "",
                (true, true) => s.more_above,
                (true, false) => s.more_below,
            };
            Text::line(text.to_string()).style(style.clone()).into()
        })
        .into()
    }

    /// The scrolling list of `count` rows between its two marks: at most
    /// `WINDOW` rows, fewer in a short terminal.
    fn list(
        &self,
        page: u32,
        count: usize,
        row: impl Fn(usize) -> AnyWidget + 'static,
    ) -> AnyWidget {
        let taps = self.taps.clone();
        let items = ListView::builder(count, move |at: usize| -> AnyWidget {
            let row = row(at);
            match taps.get(at) {
                Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(row).into(),
                None => row,
            }
        })
        .controller(self.list.clone())
        .item_extent(1);
        let scrolled = NotificationListener::<ScrollNotification>::new(items).on_notification({
            let scroll = self.scroll.clone();
            move |note| {
                scroll.set_value(Scrolled {
                    pixels: note.metrics.pixels,
                    max: note.metrics.max_scroll_extent,
                });
                false
            }
        });
        let shown = i32::try_from(count.min(WINDOW)).unwrap_or(1);
        // Flexible in the page: the header and the details keep their rows, the
        // list gives way in a short terminal.
        Flexible::new(
            1,
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .overflow_intended(true)
                .child(self.mark(true))
                .child(Flexible::new(
                    1,
                    ConstrainedBox::new(
                        BoxConstraints::new(0, UNBOUNDED, 0, shown),
                        // A page's list is its own (a fresh scroll position, as
                        // the oracle's fresh `ScrollState`).
                        SizedBox::new(
                            None,
                            Some(shown),
                            Some(AnyWidget::from(scrolled).with_key(page)),
                        ),
                    ),
                ))
                .child(self.mark(false)),
        )
        .into()
    }

    // ---- the events page

    fn events_page(&self, narrow: bool) -> AnyWidget {
        let (s, tokens) = (self.s, &self.tokens);
        let rows = self.hooks.rows();
        let show_review = !narrow && rows.iter().any(|row| row.needs_review > 0);
        let mut lines: Vec<AnyWidget> = vec![
            Text::line(s.hooks_title.to_string())
                .style(tokens.bold())
                .into(),
            self.note(s.hooks_subtitle.to_string(), tokens.dim()),
            Self::blank(),
        ];
        if let Some(message) = self.review_message(self.hooks.review_total()) {
            lines.push(self.note(
                format!("{}{message}", s.hooks_warning_mark),
                tokens.warning(),
            ));
            lines.push(Self::blank());
        }
        if !self.hooks.warnings.is_empty() || !self.hooks.errors.is_empty() {
            lines.push(
                Text::line(s.hooks_issues.to_string())
                    .style(tokens.bold())
                    .into(),
            );
            for warning in &self.hooks.warnings {
                lines.push(self.note(format!("{}{warning}", s.hooks_warning_mark), Style::new()));
            }
            for error in &self.hooks.errors {
                lines.push(self.note(
                    format!(
                        "{}{}",
                        s.hooks_error_mark,
                        s.hooks_error(&error.path, &error.message)
                    ),
                    tokens.error(),
                ));
            }
            lines.push(Self::blank());
        }
        lines.push(self.table_header(narrow, show_review));
        let page = self.clone();
        let rows = Rc::new(rows);
        let list = self.list(0, HookEvent::ALL.len(), move |at| {
            page.event_row(&rows[at], narrow, show_review)
        });
        // In a terminal shorter than the header, the table header and the two
        // marks the page is cut at the bottom, as the oracle's buffer is.
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .overflow_intended(true)
            .child(Self::header(lines))
            .child(list)
            .into()
    }

    fn table_header(&self, narrow: bool, show_review: bool) -> AnyWidget {
        let s = self.s;
        if narrow {
            let dim = self.tokens.dim();
            return Row::new()
                .child(Self::cell(
                    s.hooks_col_event.to_string(),
                    dim.clone(),
                    NARROW_NAME,
                ))
                .child(Text::line(s.hooks_col_counts.to_string()).style(dim))
                .into();
        }
        let plain = Style::new();
        let mut row = Row::new()
            .child(Self::cell(
                s.hooks_col_event.to_string(),
                plain.clone(),
                EVENT_COLUMN,
            ))
            .child(Self::cell(
                s.hooks_col_installed.to_string(),
                plain.clone(),
                COUNT_COLUMN,
            ))
            .child(Self::cell(
                s.hooks_col_active.to_string(),
                plain.clone(),
                COUNT_COLUMN,
            ));
        if show_review {
            row = row.child(Self::cell(
                s.hooks_col_review.to_string(),
                plain.clone(),
                COUNT_COLUMN,
            ));
        }
        row.child(Expanded::new(
            1,
            Text::line(s.hooks_col_description.to_string()).style(plain),
        ))
        .into()
    }

    /// A row of the events table; the one under the cursor is in the
    /// selection colour whole, the others dim but their name (the narrow
    /// table's rows to review are whole in the warning colour).
    fn event_row(&self, row: &EventRow, narrow: bool, show_review: bool) -> AnyWidget {
        let (s, tokens) = (self.s, &self.tokens);
        let index = row.event.index();
        let on = self.cursor.cursor() == Some(index);
        let marker = if on { s.cursor_marker } else { s.indent }.to_string();
        let name = self.event_label(row.event).to_string();
        let review = row.needs_review > 0;
        if narrow {
            let style = if on {
                tokens.selected()
            } else if review {
                tokens.warning()
            } else {
                Style::new()
            };
            let mark = if review { s.hooks_review_mark } else { "" };
            let counts = format!("{}{mark}", s.counts(row.active, row.installed));
            return Row::new()
                .child(Self::cell(marker, style.clone(), MARKER))
                .child(Self::cell(name, style.clone(), NARROW_NAME))
                .child(Text::line(counts).style(style))
                .into();
        }
        let (name_style, count_style, review_style, description_style) = if on {
            let selected = tokens.selected();
            (
                selected.clone(),
                selected.clone(),
                selected.clone(),
                selected,
            )
        } else {
            let dim = tokens.dim();
            (
                Style::new(),
                dim.clone(),
                if review {
                    tokens.warning()
                } else {
                    dim.clone()
                },
                dim,
            )
        };
        let mut line = Row::new()
            .child(Self::cell(marker, name_style.clone(), MARKER))
            .child(Self::cell(name, name_style, EVENT_COLUMN))
            .child(Self::cell(
                row.installed.to_string(),
                count_style.clone(),
                COUNT_COLUMN,
            ))
            .child(Self::cell(
                row.active.to_string(),
                count_style,
                COUNT_COLUMN,
            ));
        if show_review {
            line = line.child(Self::cell(
                row.needs_review.to_string(),
                review_style,
                COUNT_COLUMN,
            ));
        }
        line.child(Expanded::new(
            1,
            Text::line(s.hooks_event_descriptions[index].to_string()).style(description_style),
        ))
        .into()
    }

    // ---- the hooks page

    fn handlers_page(&self, event: HookEvent) -> AnyWidget {
        let (s, tokens) = (self.s, &self.tokens);
        let hooks: Vec<Hook> = self.hooks.of(event).into_iter().cloned().collect();
        let review = hooks.iter().filter(|hook| hook.needs_review()).count();
        let subtitle = match self.review_message(review) {
            Some(message) => self.note(message, tokens.warning()),
            None => self.note(s.hooks_handlers_subtitle.to_string(), tokens.dim()),
        };
        let header = Self::header(vec![
            Text::line(s.handlers_title(self.event_label(event)).to_string())
                .style(tokens.bold())
                .into(),
            subtitle,
        ]);
        // Cut at the bottom in a terminal shorter than the header, the marks
        // and the details (the oracle's buffer clips).
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .overflow_intended(true)
            .child(header);
        if hooks.is_empty() {
            // The message takes the first row of the body, the two rows of
            // the marks stay blank under it.
            column = column
                .child(Text::line(format!("{}{}", s.indent, s.hooks_none)).style(tokens.dim()))
                .child(Self::blank())
                .child(Self::blank());
            return column.into();
        }
        let selected = self
            .cursor
            .cursor()
            .and_then(|at: usize| hooks.get(at))
            .cloned();
        let index_width = hooks.len().to_string().len().max(2);
        let hooks = Rc::new(hooks);
        let page = self.clone();
        let rows = {
            let hooks = hooks.clone();
            move |at: usize| page.handler_row(&hooks[at], at, index_width)
        };
        let page = u32::try_from(event.index() + 1).unwrap_or(1);
        column = column.child(self.list(page, hooks.len(), rows));
        if let Some(hook) = selected {
            column = column.child(self.details(event, &hook));
        }
        // One row stays free under the details.
        column.child(Self::blank()).into()
    }

    fn handler_row(&self, hook: &Hook, at: usize, index_width: usize) -> AnyWidget {
        let (s, tokens) = (self.s, &self.tokens);
        let on = self.cursor.cursor() == Some(at);
        let review = hook.needs_review();
        let check = if review {
            s.hooks_check_review
        } else if hook.is_active() {
            s.check_on
        } else {
            s.check_off
        };
        let marker = if on { s.cursor_marker } else { s.indent };
        let title = hook
            .status_message
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or(s.hooks_unnamed);
        let suffix = match hook.trust {
            HookTrust::Modified => s.hooks_suffix_modified,
            HookTrust::Untrusted => s.hooks_suffix_new,
            HookTrust::Managed | HookTrust::Trusted => "",
        };
        let base = if on {
            tokens.selected()
        } else if review {
            tokens.warning()
        } else if hook.is_managed {
            tokens.dim()
        } else {
            Style::new()
        };
        let number = base.clone().attrs(Attrs::DIM);
        Row::new()
            .child(Text::line(format!("{marker}{check}")).style(base.clone()))
            .child(SizedBox::new(
                Some(i32::try_from(index_width).unwrap_or(2)),
                Some(1),
                Some(
                    Text::line((at + 1).to_string())
                        .style(number)
                        .text_align(TextAlign::Right)
                        .into(),
                ),
            ))
            .child(Expanded::new(
                1,
                dots_end(format!("{}{title}{suffix}", s.indent), base),
            ))
            .into()
    }

    // ---- the details of the hook under the cursor

    fn details(&self, event: HookEvent, hook: &Hook) -> AnyWidget {
        let s = self.s;
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(self.detail(s.hooks_label_event, self.event_label(event), None));
        if let Some(matcher) = &hook.matcher {
            column = column.child(self.detail(s.hooks_label_matcher, matcher, None));
        }
        column = column.child(self.detail(s.hooks_label_source, &self.source_of(hook), None));
        match &hook.handler {
            HookHandler::Command { command, is_async } => {
                column = column
                    .child(self.detail(s.hooks_label_command, command, Some(COMMAND_LINES)))
                    .child(self.detail(
                        s.hooks_label_mode,
                        if *is_async {
                            s.hooks_mode_async
                        } else {
                            s.hooks_mode_sync
                        },
                        None,
                    ));
            }
            HookHandler::McpTool { server, tool } => {
                column = column
                    .child(self.detail(s.hooks_label_server, server, None))
                    .child(self.detail(s.hooks_label_tool, tool, None));
            }
            HookHandler::Prompt => {
                column =
                    column.child(self.detail(s.hooks_label_handler, s.hooks_handler_prompt, None));
            }
            HookHandler::Agent => {
                column =
                    column.child(self.detail(s.hooks_label_handler, s.hooks_handler_agent, None));
            }
        }
        column = column.child(self.detail(
            s.hooks_label_timeout,
            &s.timeout(hook.timeout_sec).to_string(),
            None,
        ));
        if let Some(limit) = hook.additional_context_limit {
            let value = if limit == 0 {
                s.hooks_context_unlimited.to_string()
            } else {
                s.context_limit(limit).to_string()
            };
            column = column.child(self.detail(s.hooks_label_context, &value, None));
        }
        let trust = match hook.trust {
            HookTrust::Managed => s.hooks_trust_managed,
            HookTrust::Trusted => s.hooks_trust_trusted,
            HookTrust::Untrusted => s.hooks_trust_untrusted,
            HookTrust::Modified => s.hooks_trust_modified,
        };
        column = column.child(self.detail(s.hooks_label_trust, trust, None));
        Padding::new(EdgeInsets::symmetric(INSET, 0), column).into()
    }

    /// A row of the details: the label in a column of its own (ten cells at
    /// least), the value dim beside it, wrapped under its own column, in at
    /// most `lines` lines when it says (the last one cut with `…`).
    fn detail(&self, label: &str, value: &str, lines: Option<usize>) -> AnyWidget {
        let mut text = Text::new(value.to_string())
            .style(self.tokens.dim())
            .wrap(TextWrap::Word);
        if let Some(lines) = lines {
            text = text.max_lines(lines).overflow(TextOverflow::Ellipsis);
        }
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            // The label's own width and one cell, ten at least.
            .child(ConstrainedBox::new(
                BoxConstraints::new(10, UNBOUNDED, 0, UNBOUNDED),
                Text::line(format!("{label} ")),
            ))
            .child(Expanded::new(1, text))
            .into()
    }

    /// Where a hook comes from (`detail_source_value`): its environment, its
    /// plugin, the admin's config, or a file of the user's or the project's.
    fn source_of(&self, hook: &Hook) -> String {
        let s = self.s;
        if let Some(id) = &hook.environment_id {
            return s.source_of(s.hooks_source_environment, id).to_string();
        }
        let label = match hook.source {
            HookSource::Plugin => {
                return match &hook.plugin_id {
                    Some(id) => s.source_of(s.hooks_source_plugin_id, id).to_string(),
                    None => s.hooks_source_plugin.to_string(),
                };
            }
            HookSource::System
            | HookSource::Mdm
            | HookSource::CloudRequirements
            | HookSource::LegacyManagedConfigFile
            | HookSource::LegacyManagedConfigMdm => return s.hooks_source_admin.to_string(),
            HookSource::CloudManagedConfig => return s.hooks_source_cloud.to_string(),
            HookSource::User => s.hooks_source_user,
            HookSource::Project => s.hooks_source_project,
            HookSource::SessionFlags => s.hooks_source_session,
            HookSource::Unknown => s.hooks_source_unknown,
        };
        s.source_path(label, &hook.source_path).to_string()
    }
}
