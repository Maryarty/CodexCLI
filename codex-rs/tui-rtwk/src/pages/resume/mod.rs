//! `resume` (S18): the view half of the resume and fork picker. One
//! `PageRoute` over the model's rows (`ScreenStates.resume`): the header, the
//! filter/status/sort toolbar, the search line, the list of saved threads and
//! the footer with progress and key hints. Its model, events and intents are
//! core's `codex_tui_core::screens::resume`; the page keeps only what is the
//! view's: the search field's controller, which toolbar control has the
//! keyboard focus, and the list's selection and scroll (rtwk's controllers).
//! Oracle: `tui/src/resume_picker.rs` and `resume_picker/layout.rs`; the
//! chrome's rows are in `layout.rs`, the spellings picked by width in
//! `chrome.rs` and `toolbar.rs`, the rows in `rows.rs`, the keys in `keys.rs`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::controls::{SelectionController, TabController};
use rtwk::editing::{EditingKeys, TextEditingController, TextField};
use rtwk::focus::{ExcludeFocus, FocusNode};
use rtwk::foundation::{Listenable, ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::PointerEvent;
use rtwk::layout::{
    BorderType, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize, Offstage,
    Padding, Row, SizedBox, Stack,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::scroll::ListController;
use rtwk::style::Theme;
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::resume::{
    Action, Control, Density, Dir, Intent as ResumeIntent, Launch, Load, Pick, Row as Saved,
    Status, TranscriptState,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::widgets::session_list::{Move, Scrolled, SessionList, target};

mod chrome;
mod keys;
mod layout;
mod list;
mod look;
mod rows;
pub mod strings;
mod toolbar;

use look::Look;
use rows::Rows;
use strings::Strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    // One entry per fixture script: the screen on the model it makes.
    macro_rules! entry {
        ($name:literal, $description:literal) => {
            DebugEntry {
                name: $name,
                description: $description,
                script: include_str!(concat!("fixtures/", $name, ".jsonl")),
                open: EntryOpen::Route(|_: &Seeded| Screen { live: false }.into()),
            }
        };
    }
    vec![
        DebugEntry {
            name: "fake_rollouts",
            description: "a resume list from three fake rollouts",
            script: include_str!("fixtures/fake_rollouts.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| list::List.into()),
        },
        entry!(
            "table",
            "three rows: a cloud one, an unread one, a plain one"
        ),
        entry!("more_indicators", "five rows, to scroll down to the third"),
        entry!(
            "search_error",
            "a listing that failed: the message on the search line"
        ),
        entry!(
            "footer_wide",
            "no rows at 220 cells: the footer's wide hints"
        ),
        entry!("loading", "the first page is loading"),
        entry!("empty", "no saved sessions"),
        entry!(
            "toolbar_repo_cwd",
            "a cwd filter with a repository that has other checkouts"
        ),
        entry!(
            "dense_cwd",
            "one row under its own directory filter (ctrl+o: dense)"
        ),
        entry!("dense_narrow", "one row, to be dense at 48 cells"),
        entry!("dense_named", "a row with a name and a preview"),
        entry!(
            "dense_two_rows",
            "two rows under the cwd filter (dense, the second selected)"
        ),
        entry!("zebra_rows", "two comfortable rows, the first striped"),
        entry!(
            "expanded_session",
            "one row with its conversation read (ctrl+e)"
        ),
        entry!("narrow_session", "one row, to be read at 58 cells"),
        entry!("footer_times", "one row, updated 3h and created 5h ago"),
        entry!("footer_no_branch", "one row without a branch"),
        entry!("footer_long_branch", "one row on a long branch"),
        entry!("footer_long_cwd", "one row under a very long directory"),
        entry!(
            "footer_no_cwd",
            "one row under the directory filter's own directory"
        ),
        entry!(
            "footer_narrow",
            "an empty picker, to be opened at 119 cells"
        ),
        entry!(
            "footer_very_narrow",
            "an empty picker, to be opened at 38 cells"
        ),
        entry!(
            "four_rows",
            "four rows that overflow the list (ctrl+o: dense)"
        ),
        entry!("three_rows", "three rows item-0..item-2"),
        entry!("ten_rows", "ten rows item-0..item-9"),
        entry!("twenty_rows", "twenty rows item-0..item-19"),
        entry!(
            "end_ten_rows",
            "ten rows and a next cursor (end loads more)"
        ),
        entry!("search_query", "three rows to search and paste into"),
        entry!("one_row_next_page", "one row and a next cursor"),
        entry!("two_rows_alpha_beta", "rows alpha and beta"),
        entry!("no_cwd_filter", "two rows with no directory filter"),
        entry!("archive_footer", "one selected row (the archive hint)"),
        entry!(
            "archived_cwd",
            "one row under a directory filter; the status moves to Archived"
        ),
        entry!(
            "partial_failure",
            "a healthy row kept after one source failed"
        ),
        entry!("later_page_error", "one loaded row whose next page failed"),
        entry!("colors_unread", "an unread row named Named task"),
        entry!("compact_six_rows", "six rows under /tmp/codex (resume)"),
        entry!("compact_six_rows_fork", "six rows under /tmp/codex (fork)"),
        entry!(
            "toolbar_focus_narrow",
            "an empty picker scoped to /tmp/codex"
        ),
        entry!(
            "toolbar_focus_narrow_fork",
            "an empty fork picker scoped to /tmp/codex"
        ),
        entry!("transcript_loading", "two rows without times (ctrl+t)"),
        entry!("db_fallback", "the three rows of the store fallback"),
        entry!("linked_worktree", "a session under a linked checkout"),
        entry!(
            "expanded_details",
            "a named thread at a fixed time (ctrl+e)"
        ),
        entry!("archive_cloud", "a local and a cloud copy of one thread"),
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen { live: true }.into())
}

/// Opens the picker on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen { live: true }.into())
}

/// The picker. `live`: it lists the backend's sessions (opening asks for
/// them); a seeded one shows the rows its script made.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub live: bool,
}

/// Widget state: the selection (rtwk's controller owns the cursor and the
/// scroll), where the list is scrolled to (told by its notifications), the
/// search field's controller and focus, and which toolbar control the arrows
/// change (the oracle's `toolbar_focus`): an index into `Control::all`, held by
/// a `TabController`.
#[derive(Debug)]
pub struct ScreenState {
    selection: SelectionController<usize>,
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
    /// The rows of the list's region, its `more` lines included, as the
    /// last layout gave them (the oracle's `view_rows`).
    region: Rc<Cell<i32>>,
    query: TextEditingController,
    node: FocusNode,
    focus: TabController,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Enter.
    Accept,
    /// A tap on a row: the cursor goes there, a tap on the row it is on opens it.
    Row(usize),
    /// A key of the list moved the cursor.
    Move(Move),
    /// Tab (forward) and shift+tab: the next toolbar control takes the arrows.
    Focus(bool),
    /// Left and right: the focused control's value moves.
    Change(Dir),
    /// A tap on a toolbar value: that value is picked.
    Pick(Pick),
    /// The search field's text changed.
    Query,
    Backspace,
    Escape,
    Quit,
    CancelTranscript,
    Transcript,
    Expand,
    Density,
    Archive,
    /// The list has room for more rows than it has.
    Fit,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selection: SelectionController::with(false, [0], Some(0)),
            list: ListController::new(),
            scroll: Rc::new(ValueNotifier::new(Scrolled::default())),
            region: Rc::new(Cell::new(0)),
            query: TextEditingController::new(""),
            node: FocusNode::new(),
            focus: TabController::new(3),
        }
    }
}

impl_widget!(stateful eq Screen);

/// The controllers of the list and the handlers of the row taps.
#[derive(Clone)]
struct Controls {
    selection: SelectionController<usize>,
    list: ListController,
    scroll: Rc<ValueNotifier<Scrolled>>,
    region: Rc<Cell<i32>>,
    taps: Rc<Vec<Handler<PointerEvent>>>,
}

/// What the page's rows are built from, once per frame.
struct Frame {
    look: Look,
    strings: &'static Strings,
    screens: Rc<ScreenStates>,
    shown: Rc<Vec<usize>>,
    rows: Rows,
    focus: Control,
    field: AnyWidget,
    controls: Controls,
    taps: toolbar::Taps,
    query_empty: bool,
}

/// The header, the toolbar, the search line, the list and the footer, one
/// under the other with the oracle's collapsing gaps.
fn screen(frame: Rc<Frame>) -> AnyWidget {
    LayoutBuilder::new(move |_, constraints| {
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let height = u16::try_from(constraints.max_height).unwrap_or(0);
        let areas = layout::areas(height);
        let Frame {
            look,
            strings,
            screens,
            ..
        } = &*frame;
        let model = &screens.resume;
        let gap = |rows: u16| SizedBox::new(None, Some(i32::from(rows)), None);
        let chrome = |line: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::only(1, 0, 1, 0), line).into()
        };
        let header = match model.action {
            Action::Resume => strings.header,
            Action::Fork => strings.header_fork,
        };
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(chrome(
                Text::new(header)
                    .style(look.bold.clone())
                    .wrap(TextWrap::None)
                    .max_lines(1)
                    .overflow(TextOverflow::Ellipsis)
                    .into(),
            ))
            .child(gap(areas.toolbar - 1))
            .child(chrome(toolbar::toolbar(
                look,
                strings,
                toolbar::Facts::of(model, frame.focus),
                frame.taps.clone(),
            )))
            .child(gap(areas.search - areas.toolbar - 1))
            .child(chrome(
                SizedBox::new(None, Some(1), Some(search(&frame))).into(),
            ))
            .child(gap(areas.list - areas.search - 1))
            .child(SizedBox::new(
                None,
                Some(i32::from(areas.list_height)),
                Some(
                    Padding::new(
                        EdgeInsets::symmetric(i32::from(layout::LIST_INSET), 0),
                        list_of(&frame),
                    )
                    .into(),
                ),
            ))
            .child(footer(&frame, width))
            .into()
    })
    .into()
}

/// The search line: what is wrong when something is (the inline error, else a
/// failed first page), else the search field with its label once typed in.
/// The field stays in the tree while an error shows, so it keeps its focus.
fn search(frame: &Frame) -> AnyWidget {
    let (look, strings) = (&frame.look, frame.strings);
    let model = &frame.screens.resume;
    let problem = model
        .inline_error
        .as_ref()
        .map(|problem| strings.problem(problem, &frame.rows.num))
        .or_else(|| match &model.load {
            Load::Failed(message) => Some(message.clone()),
            _ => None,
        });
    let label = if frame.query_empty {
        None
    } else {
        Some(Text::line(strings.search_label))
    };
    let mut field = Row::new();
    if let Some(label) = label {
        field = field.child(label);
    }
    let field = field.child(Expanded::new(1, frame.field.clone()));
    let error = problem.clone().unwrap_or_default();
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Offstage::new(problem.is_some(), field))
        .child(Offstage::new(
            problem.is_none(),
            Text::new(error)
                .style(look.error.clone())
                .wrap(TextWrap::None)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis),
        ))
        .into()
}

/// The footer: the rule with the progress label, then the hint rows.
fn footer(frame: &Frame, width: usize) -> AnyWidget {
    let look = frame.look.clone();
    let model = &frame.screens.resume;
    let facts = chrome::HintFacts {
        loading: matches!(
            &model.transcript,
            Some(codex_tui_core::screens::resume::Transcript {
                state: TranscriptState::Loading,
                ..
            })
        ),
        fork: model.action == Action::Fork,
        archived: model.status == Status::Archived,
        searching: !model.query.is_empty(),
        in_session: matches!(model.launch, Launch::Session { .. }),
        dense: model.density == Density::Dense,
        archive: !frame.shown.is_empty() && model.archive_available(),
    };
    let [first, second] = chrome::hint_rows(frame.strings, facts);
    let (screens, shown, selection, scroll) = (
        frame.screens.clone(),
        frame.shown.clone(),
        frame.controls.selection.clone(),
        frame.controls.scroll.clone(),
    );
    let rule = {
        let look = look.clone();
        ListenableBuilder::new(selection.listenable(), move |_, _| {
            let (look, screens, shown, selection) = (
                look.clone(),
                screens.clone(),
                shown.clone(),
                selection.clone(),
            );
            ValueListenableBuilder::new(scroll.clone(), move |_, at: &Scrolled, _| {
                let model = &screens.resume;
                let rows = shown.len();
                let position = if rows == 0 {
                    0
                } else {
                    selection.cursor().map_or(1, |at| at + 1)
                };
                let total = if model.listing.is_loading() {
                    format!("{rows}…")
                } else {
                    rows.to_string()
                };
                // The loading line under the last row is not part of the
                // sessions the oracle's percent counts.
                let max = at.max - i64::from(rows > 0 && model.listing.is_loading());
                let percent = if max <= 0 {
                    100
                } else {
                    ((at.pixels.min(max) as f64 / max as f64) * 100.0).round() as u8
                };
                let label = chrome::progress_label(position, &total, percent, width);
                Row::new()
                    .child(Expanded::new(
                        1,
                        Divider::new()
                            .kind(BorderType::Single)
                            .style(look.dim.clone())
                            .merge(false),
                    ))
                    .child(Text::line(label).style(look.dim.clone()))
                    .child(SizedBox::new(
                        Some(1),
                        Some(1),
                        Some(
                            Divider::new()
                                .kind(BorderType::Single)
                                .style(look.dim.clone())
                                .merge(false)
                                .into(),
                        ),
                    ))
                    .into()
            })
            .into()
        })
    };
    let hint = |variants| {
        if facts.loading {
            chrome::plain_fit(variants, look.dim.clone(), "   ", 1)
        } else {
            crate::widgets::page_keys::footer_fit(
                variants,
                look.dim.clone(),
                look.dim.clone(),
                "   ",
                1,
            )
        }
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(rule)
        .child(hint(first))
        .child(hint(second))
        .into()
}

/// The saved threads: a `SessionList` of rows (title, meta lines, a blank
/// row between in the comfortable view), with the "more" rows above and below
/// while the list is scrolled, the loading line under the last row and the
/// overlay while a transcript loads.
fn list_of(frame: &Frame) -> AnyWidget {
    let rows = frame.rows.clone();
    let (look, strings) = (frame.look.clone(), frame.strings);
    let controls = frame.controls.clone();
    let screens = frame.screens.clone();
    let shown = frame.shown.clone();
    LayoutBuilder::new(move |cx, constraints| {
        let model = &screens.resume;
        let len = shown.len();
        let loading = matches!(
            &model.transcript,
            Some(codex_tui_core::screens::resume::Transcript {
                state: TranscriptState::Loading,
                ..
            })
        );
        let body: AnyWidget = if len == 0 {
            rows.empty()
        } else {
            let more = model.listing.is_loading() && len > 0;
            let Controls {
                selection,
                list,
                scroll,
                region,
                taps,
            } = controls.clone();
            region.set(constraints.max_height);
            let (cursor_of, region_of) = (selection.clone(), region);
            let rows = Rows {
                width: constraints.max_width,
                ..rows.clone()
            };
            let list_rows = SessionList {
                len: len + usize::from(more),
                key: Rc::new(|index| index),
                selectable: Some(Rc::new(move |index| index < len)),
                row: Rc::new(move |index, selected| {
                    if index < len {
                        rows.row(index, selected)
                    } else {
                        rows.loading_older()
                    }
                }),
                selection,
                list,
                scroll: scroll.clone(),
                focus: None,
                extent: None,
                taps: Some(taps),
            }
            .build();
            let shown_line = |text: &'static str| -> AnyWidget {
                Text::line(text).style(look.accent.clone()).into()
            };
            let none = || -> AnyWidget { SizedBox::new(Some(0), Some(0), None).into() };
            let has_more = model.has_more();
            let loading_more = model.listing.is_loading();
            let up = {
                let above = shown_line(strings.more_above);
                ValueListenableBuilder::new(scroll.clone(), move |_, at: &Scrolled, _| {
                    if at.above() { above.clone() } else { none() }
                })
            };
            let density = model.density;
            let down = {
                let below = shown_line(if loading_more {
                    strings.more_loading
                } else {
                    strings.more_below
                });
                // Whether rows lie below is the oracle's rule (`has_more_below`), which
                // depends on the cursor: the list's own extent cannot say, as the line
                // takes a row of the room it measures.
                let cursor = cursor_of;
                ValueListenableBuilder::new(scroll, move |_, at: &Scrolled, _| {
                    let (below, at) = (below.clone(), *at);
                    let (cursor, region) = (cursor.clone(), region_of.clone());
                    ListenableBuilder::new(cursor.listenable(), move |_, _| {
                        let top = rows_above(at.pixels, density).min(len.saturating_sub(1));
                        let view = usize::try_from(region.get()).unwrap_or(1);
                        if has_more
                            || more_below(top, cursor.cursor().unwrap_or(0), view, len, density)
                        {
                            below.clone()
                        } else {
                            SizedBox::new(Some(0), Some(0), None).into()
                        }
                    })
                    .into()
                })
            };
            ExcludeFocus::new(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(up)
                    .child(Expanded::new(1, list_rows))
                    .child(down),
            )
            .into()
        };
        if loading {
            Stack::new()
                .child(body)
                .child(rows::overlay(
                    &look,
                    strings,
                    i32::try_from(rtwk::text::segmenter_of(cx).width(strings.overlay)).unwrap_or(0),
                    (constraints.max_width, constraints.max_height),
                ))
                .into()
        } else {
            body
        }
    })
    .into()
}

impl ScreenState {
    /// The toolbar control the arrows change.
    fn control(&self, action: Action) -> Control {
        let all = Control::all(action);
        all[self.focus.index().min(all.len() - 1)]
    }

    /// The control the arrows change is `control`.
    fn focus_on(&self, action: Action, control: Control) {
        let at = Control::all(action).iter().position(|c| *c == control);
        self.focus.set_index(at.unwrap_or(0));
    }

    /// The row the cursor is on among those the list shows.
    fn cursor_row(&self, cx: &mut Cx<'_, Self>) -> Option<Saved> {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.resume;
        let at = self.selection.cursor()?;
        let index = *model.visible_indices().get(at)?;
        model.rows.get(index).cloned()
    }

    fn tell(&self, cx: &mut Cx<'_, Self>, intent: ResumeIntent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Resume(intent));
    }

    /// The cursor goes to `index` and the list shows it.
    fn put(&self, index: usize) {
        self.selection.set_cursor(Some(index));
        self.selection.select(index);
        self.list.ensure_index_visible(index);
    }

    /// The cursor and the scroll back at the first row (a new search, a new
    /// listing).
    fn top(&self) {
        self.selection.set_cursor(Some(0));
        self.selection.select(0);
        if self.list.has_clients() {
            self.list.jump_to_index(0);
        }
    }

    /// The cursor moves and the list shows it by whole rows (G42): the
    /// oracle's rule, `ensure_selected_visible`, is that the first row of the
    /// window is the least that lets the rows from it to the cursor, blanks
    /// included, fit in the window less its `more` lines. Up and down stop at
    /// the ends, as the oracle's do.
    fn move_cursor(&self, cx: &mut Cx<'_, Self>, mv: Move) {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.resume;
        let len = model.visible_indices().len();
        if len == 0 {
            return;
        }
        let list = &self.list;
        let pixels = list.try_position().map_or(0, |position| position.pixels());
        let from = self.selection.cursor();
        // The window is the list's region of the last layout, the `more`
        // lines in it.
        let view = usize::try_from(self.region.get()).unwrap_or(1);
        let Some(to) = target(len, from, &|_| true, mv, view.max(1)) else {
            return;
        };
        let forward = matches!(mv, Move::Step(true));
        let backward = matches!(mv, Move::Step(false));
        let at = from.unwrap_or(0);
        if (forward && to <= at && from.is_some()) || (backward && to >= at && from.is_some()) {
            // no wrap round the ends
            self.tell(cx, ResumeIntent::Cursor(at));
            return;
        }
        self.selection.set_cursor(Some(to));
        self.selection.select(to);
        let prev = rows_above(pixels, model.density);
        list.jump_to_index(window_top(prev.min(len - 1), to, view, len, model.density));
        let intent = if matches!(mv, Move::Edge(true)) {
            ResumeIntent::End
        } else {
            ResumeIntent::Cursor(to)
        };
        self.tell(cx, intent);
    }

    /// Enter on the cursor's row: the app opens that thread (ADR-003), the
    /// picker goes away unless a restore comes first.
    fn accept(&self, cx: &mut Cx<'_, Self>) {
        let Some(row) = self.cursor_row(cx) else {
            return;
        };
        let archived = watch::<ScreenStates, _>(cx).resume.status == Status::Archived;
        let listed = !row.id.as_str().is_empty();
        self.tell(cx, ResumeIntent::Resume(row.id));
        if listed && !archived {
            Navigator::maybe_pop(cx);
        }
    }
}

/// The lines between the list's top and the pixel offset `pixels` are, in
/// rows of the density (a comfortable row is two lines and a blank).
fn rows_above(pixels: i64, density: Density) -> usize {
    usize::try_from(pixels).unwrap_or(0) / row_lines(density)
}

/// The lines of a row with its blank row after it.
fn row_lines(density: Density) -> usize {
    match density {
        Density::Comfortable => 3,
        Density::Dense => 1,
    }
}

/// Whether rows lie below the window that starts at row `top` (resume_picker.rs
/// `has_more_below`, without its next-page test): the rows from `top` on, blanks
/// included, do not all fit in what the window has left after its `more` lines.
fn more_below(top: usize, selected: usize, view: usize, len: usize, density: Density) -> bool {
    let (body, blank) = match density {
        Density::Comfortable => (2, 1),
        Density::Dense => (1, 0),
    };
    let capacity = view
        .saturating_sub(usize::from(top > 0) + usize::from(selected + 1 < len))
        .max(1);
    let mut used = 0;
    for offset in 0..len.saturating_sub(top) {
        let separator = usize::from(offset > 0) * blank;
        if used + separator + body > capacity {
            return true;
        }
        used += separator + body;
    }
    false
}

/// The first row of the window once the cursor is on `selected`
/// (resume_picker.rs `ensure_selected_visible`, `available_content_rows`): the
/// rows between the top and the cursor, blanks included, fit in what the
/// window has left after its `more` lines.
fn window_top(prev: usize, selected: usize, view: usize, len: usize, density: Density) -> usize {
    let (body, blank) = match density {
        Density::Comfortable => (2, 1),
        Density::Dense => (1, 0),
    };
    let between = |end: usize, top: usize| (end - top + 1) * body + (end - top) * blank;
    let room = |top: usize| {
        view.saturating_sub(usize::from(top > 0) + usize::from(selected + 1 < len))
            .max(1)
    };
    let mut top = prev.min(selected);
    while top < selected && between(selected, top) > room(top) {
        top += 1;
    }
    top
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        let tell = cx.local_sender();
        self.scroll.add_listener(Rc::new(move || tell(Msg::Fit)));
        // Flutter's `controller.addListener`: the controller tells when the
        // list's extents or offset change, at the first layout, a density
        // switch or a resize too, which the list's scroll notifications
        // (a user's scroll only) do not.
        let (list, scroll) = (self.list.clone(), self.scroll.clone());
        self.list.add_listener(Rc::new(move || {
            // (two clients while a rebuilt list replaces the old one)
            if let Ok(position) = list.try_position() {
                scroll.set_value(Scrolled {
                    pixels: position.pixels(),
                    max: position.max_scroll_extent(),
                });
            }
        }));
        if w.live {
            self.tell(cx, ResumeIntent::Open);
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.resume;
        match msg {
            Msg::Accept => self.accept(cx),
            Msg::Row(index) => {
                if self.selection.cursor() == Some(index) {
                    self.accept(cx);
                } else {
                    self.put(index);
                    self.tell(cx, ResumeIntent::Cursor(index));
                }
            }
            Msg::Move(mv) => self.move_cursor(cx, mv),
            Msg::Focus(forward) => {
                let control = self.control(model.action);
                let next = if forward {
                    control.next(model.action)
                } else {
                    control.previous(model.action)
                };
                self.focus_on(model.action, next);
                return Rebuild::Yes;
            }
            Msg::Change(dir) => {
                let control = self.control(model.action);
                self.tell(cx, ResumeIntent::Change(control, dir));
                self.top();
            }
            Msg::Pick(pick) => {
                self.focus_on(model.action, pick.control());
                self.tell(cx, ResumeIntent::Pick(pick));
                self.top();
            }
            Msg::Query => {
                let text = self.query.text().to_string();
                self.tell(cx, ResumeIntent::Query(text));
                self.top();
                return Rebuild::Yes;
            }
            Msg::Backspace => {
                let mut text = self.query.text().to_string();
                text.pop();
                self.query.set_text(&text);
                self.tell(cx, ResumeIntent::Query(text));
                self.top();
                return Rebuild::Yes;
            }
            Msg::Escape => {
                if model.query.is_empty() {
                    Navigator::maybe_pop(cx);
                } else {
                    // The cursor stays on its row once the search is gone.
                    let row = self.cursor_row(cx).map(|row| row.key());
                    let all = model.visible_for("");
                    let at = row.and_then(|key| all.iter().position(|row| row.key() == key));
                    self.query.clear();
                    self.tell(cx, ResumeIntent::ClearQuery);
                    if let Some(at) = at {
                        self.put(at);
                    }
                    return Rebuild::Yes;
                }
            }
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.quit, ());
            }
            Msg::CancelTranscript => self.tell(cx, ResumeIntent::CancelTranscript),
            Msg::Transcript => {
                if let Some(row) = self.cursor_row(cx) {
                    self.tell(cx, ResumeIntent::OpenTranscript(row.id));
                }
            }
            Msg::Expand => {
                if let Some(row) = self.cursor_row(cx) {
                    self.tell(cx, ResumeIntent::Expand(row.id));
                }
            }
            Msg::Density => self.tell(cx, ResumeIntent::ToggleDensity),
            Msg::Archive => {
                if let Some(row) = self.cursor_row(cx) {
                    self.tell(cx, ResumeIntent::Archive(row.id));
                }
            }
            Msg::Fit => {
                let at = self.scroll.value();
                let len = model.visible_indices().len();
                if at.max <= 0 && len > 0 {
                    self.tell(cx, ResumeIntent::Room(len + 1));
                }
            }
        }
        Rebuild::No
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        // The rows change under the cursor (a page arrives, a filter turns):
        // it stays inside them.
        let screens = watch::<ScreenStates, _>(cx);
        let len = screens.resume.visible_indices().len();
        match self.selection.cursor() {
            Some(at) if len > 0 && at >= len => self.put(len - 1),
            None if len > 0 => self.selection.set_cursor(Some(0)),
            _ => {}
        }
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let look = Look::of(cx);
        let markdown = theme::Tokens::of(cx).transcript_markdown(cx);
        // The oracle cuts a title with three dots, rtwk's theme glyph is the
        // one-cell mark: the glyph for this screen.
        let mut data = (*Theme::of(cx)).clone();
        data.glyphs.ellipsis = "...".into();
        let model = &screens.resume;
        let focus = self.control(model.action);
        let shown = Rc::new(model.visible_indices());
        let tell = cx.local_sender();
        let taps = Rc::new(
            (0..shown.len() + 1)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Row(index)))
                .collect::<Vec<_>>(),
        );
        let toolbar_taps = toolbar::Taps::new(|pick| cx.map(move |()| Msg::Pick(pick)));
        let loading = matches!(
            &model.transcript,
            Some(codex_tui_core::screens::resume::Transcript {
                state: TranscriptState::Loading,
                ..
            })
        );
        let data_keys = keys::KeyData {
            tell: tell.clone(),
            loading,
        };
        let field = TextField::new()
            .controller(self.query.clone())
            .placeholder(strings.search_hint)
            .placeholder_style(look.dim.clone())
            .focus_node(self.node.clone())
            .autofocus(true)
            .key_hints(false)
            .on_changed(cx.map(|_: Rc<str>| Msg::Query))
            .on_paste({
                let query = self.query.clone();
                move |text| {
                    if loading {
                        return true;
                    }
                    if let Some(next) = codex_tui_core::screens::resume::pasted(&query.text(), text)
                    {
                        query.set_text(&next);
                        tell(Msg::Query);
                    }
                    true
                }
            });
        let field = EditingKeys::new(move |_, key, kcx| keys::take(&data_keys, key, kcx), field)
            .controller(self.query.clone());
        let home = screens.agents.home.as_deref().map(Rc::<str>::from);
        let frame = Frame {
            rows: Rows {
                look: look.clone(),
                strings,
                num,
                screens: screens.clone(),
                shown: shown.clone(),
                home,
                width: 0,
                markdown,
            },
            look,
            strings,
            screens,
            shown,
            focus,
            field: field.into(),
            controls: Controls {
                selection: self.selection.clone(),
                list: self.list.clone(),
                scroll: self.scroll.clone(),
                region: self.region.clone(),
                taps,
            },
            taps: toolbar_taps,
            query_empty: self.query.text().is_empty(),
        };
        Theme::new(data, screen(Rc::new(frame))).into()
    }
}
