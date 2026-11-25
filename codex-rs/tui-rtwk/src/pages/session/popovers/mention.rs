//! The mention popups' options view, as Flutter's `RawAutocomplete` hands one
//! to `optionsViewBuilder`: widgets composed in the page's own layout above
//! the composer (the child the page gives it), not a floating overlay,
//! because the oracle's popup takes its rows from the transcript. The
//! options and the highlight are the draft's `Mentions` (the controllers the
//! key hook moves); this view only draws them and turns a tap on a row (or a
//! tab) into a message of its own state.
//!
//! The unified `@` popup (`mentions_v2/render.rs`): a header, the tab strip
//! (`widgets/tab_strip.rs`), the query, the rows (one `ListView` row each:
//! `›` mark, the name column as wide as the widest name, a description
//! column, the type tag at the right) between the `↑` and `↓` rows, and the
//! key hint. The `$` popup (`skill_popup.rs`): the rows, the category tag
//! after the name column, the hint. Widths are measured with the tree's
//! segmenter under a `LayoutBuilder` (composition: the app only picks the
//! spelling, rtwk lays out and cuts).

use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::foundation::{Listenable, ListenableBuilder, VoidCallback};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    FlexFit, Flexible, MainAxisSize, Row, SizedBox, UNBOUNDED,
};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Attrs, Span, Style, TerminalBrightness};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{
    AnyWidget, Cx, Handler, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget,
};

use codex_tui_core::data::mention::MentionKind;
use codex_tui_core::data::mention::{MentionType, Row as Candidate};
use codex_tui_core::kit::mentions::files::Empty;
use codex_tui_core::kit::mentions::row::{file_name_start, primary};
use codex_tui_core::kit::mentions::search_mode::SearchMode;
use codex_tui_core::kit::mentions::session::Active;

use super::strings::{self, Strings};
use crate::app::theme;
use crate::widgets::composer_draft::Mentions;
use crate::widgets::dots::dots_end;
use crate::widgets::tab_strip::Tabs;
use crate::widgets::thread_ui::ThreadUi;

/// The most rows either popup lists at once (`MAX_POPUP_ROWS`).
const MAX_ROWS: i32 = 8;
/// The rows the unified popup reserves: two header lines, the gaps, the
/// tabs, the query, two overflow rows and the hint around eight rows
/// (`mentions_v2::render::POPUP_HEIGHT`).
const POPUP_HEIGHT: i32 = MAX_ROWS + 9;
/// The cells the `$` popup keeps for descriptions, else it hides them
/// (`SelectionDescriptionLayout::should_hide_column`).
const MIN_DESCRIPTION: usize = 24;
/// The cells of the type tag (`TAG_WIDTH`: `"Plugin".len()`).
const TAG_CELLS: i32 = 6;

/// The flex factor of an open popup in the column it stands in: it takes the
/// rows the cells before it leave, ahead of any other flexible child (the
/// draft first, then the popup, then the rest: `Layout::vertical([Min(3),
/// Max(above_input), ..])`).
const OPEN_FLEX: i32 = 1000;

/// The popup of the draft `ui` holds, in its place among the children of a
/// column that gives it the rows left over: while closed it is a box of no
/// size, while open it takes up to its own height of what the other children
/// leave. The slot is keyed, and a `ListenableBuilder` of the draft's mentions
/// swaps its child, so the column is laid out again as the popup opens and
/// closes.
pub fn slot(ui: Rc<ThreadUi>) -> AnyWidget {
    let mentions = ui.draft.mentions();
    AnyWidget::from(ListenableBuilder::new(
        mentions.listenable(),
        move |_, _| {
            if !ui.draft.mentions().is_open() {
                return SizedBox::shrink().into();
            }
            Flexible {
                flex: OPEN_FLEX,
                fit: FlexFit::Loose,
                child: MentionPopup::new(ui.clone()).into(),
            }
            .into()
        },
    ))
    .with_key(Key::value("mention-popup"))
}

/// The popup of the draft `ui` holds, at most `room` rows high and no more
/// than it wants, as an inflexible child: for a column whose other children
/// are known (the composer harness: footer and draft rows), where the draft
/// then takes every row the popup leaves.
pub fn capped(ui: Rc<ThreadUi>, room: i32) -> AnyWidget {
    let mentions = ui.draft.mentions();
    AnyWidget::from(ListenableBuilder::new(
        mentions.listenable(),
        move |_, _| {
            if !ui.draft.mentions().is_open() {
                return SizedBox::shrink().into();
            }
            ConstrainedBox::new(
                BoxConstraints::new(0, UNBOUNDED, 0, room.max(0)),
                MentionPopup::new(ui.clone()),
            )
            .into()
        },
    ))
    .with_key(Key::value("mention-popup"))
}

/// The popup of the draft `ui` holds.
#[derive(Clone)]
pub struct MentionPopup {
    ui: Rc<ThreadUi>,
}

impl PartialEq for MentionPopup {
    fn eq(&self, other: &MentionPopup) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
    }
}

impl MentionPopup {
    pub fn new(ui: Rc<ThreadUi>) -> MentionPopup {
        MentionPopup { ui }
    }
}

impl StatefulWidget for MentionPopup {
    type State = PopupState;

    fn create_state(&self) -> PopupState {
        PopupState { listening: None }
    }
}

impl_widget!(stateful eq MentionPopup);

pub struct PopupState {
    listening: Option<(Mentions, VoidCallback)>,
}

/// What the rows and tabs of the popup tell it.
#[derive(Clone, Debug)]
pub enum PopupMsg {
    /// The popup's options or highlight changed.
    Changed,
    /// A tap on row `i`: the composer picks it.
    Choose(usize),
    /// A tap on tab `i`.
    Tab(usize),
}

impl PopupState {
    fn listen(&mut self, ui: &ThreadUi, tell: Rc<dyn Fn(PopupMsg)>) {
        self.unlisten();
        let mentions = ui.draft.mentions();
        let changed: VoidCallback = Rc::new(move || tell(PopupMsg::Changed));
        mentions.listenable().add_listener(changed.clone());
        self.listening = Some((mentions, changed));
    }

    fn unlisten(&mut self) {
        if let Some((mentions, changed)) = self.listening.take() {
            mentions.listenable().remove_listener(&changed);
        }
    }
}

impl State for PopupState {
    type Widget = MentionPopup;
    type Msg = PopupMsg;

    fn init_state(&mut self, w: &MentionPopup, cx: &mut Cx<'_, Self>) {
        let tell = cx.local_sender();
        self.listen(&w.ui, tell);
    }

    fn did_update_widget(&mut self, old: &MentionPopup, w: &MentionPopup, cx: &mut Cx<'_, Self>) {
        if !Rc::ptr_eq(&old.ui, &w.ui) {
            let tell = cx.local_sender();
            self.listen(&w.ui, tell);
        }
    }

    fn dispose(&mut self) {
        self.unlisten();
    }

    fn update(&mut self, msg: PopupMsg, w: &MentionPopup, _cx: &mut Cx<'_, Self>) -> Rebuild {
        let mentions = w.ui.draft.mentions();
        match msg {
            PopupMsg::Changed => Rebuild::Yes,
            PopupMsg::Choose(at) => {
                mentions.choose(at);
                Rebuild::No
            }
            PopupMsg::Tab(at) => {
                if let Some(mode) = SearchMode::ALL.get(at) {
                    mentions.set_search_mode(*mode);
                }
                Rebuild::No
            }
        }
    }

    fn build(&self, w: &MentionPopup, cx: &mut Cx<'_, Self>) -> rtwk::AnyWidget {
        let mentions = w.ui.draft.mentions();
        let Some(shown) = Shown::read(&mentions) else {
            return AnyWidget::default();
        };
        let tokens = theme::Tokens::of(cx);
        // Where the terminal told nothing of its background the oracle's rows
        // fall back to the terminal's own attributes (`selection_style`,
        // `accent_color_on`): the cursor row reversed, the `↑`/`↓` in the ink.
        let known = TerminalBrightness::background_of(cx).is_some();
        let look = Look {
            // the oracle's `.dim()`: the terminal's dim attribute
            dim: Style::new().attrs(Attrs::DIM),
            bold: tokens.bold(),
            accent: tokens.accent(),
            selected: if known {
                tokens.selected()
            } else {
                Style::new().attrs(Attrs::BOLD | Attrs::REVERSE)
            },
            info: tokens.info(),
            arrow: if known {
                tokens.accent_on(&Style::new())
            } else {
                Style::new()
            },
        };
        let table = strings::of(cx);
        let taps: Rc<Vec<Handler<PointerEvent>>> = Rc::new(
            (0..shown.len())
                .map(|at| cx.map(move |_: PointerEvent| PopupMsg::Choose(at)))
                .collect(),
        );
        let part = Part {
            cursor: mentions.cursor_controller().clone(),
            list: mentions.list().clone(),
            taps,
            look,
            table,
        };
        match shown {
            Shown::At(at) => {
                let tabs = (0..SearchMode::ALL.len())
                    .map(|tab| cx.map(move |_: PointerEvent| PopupMsg::Tab(tab)))
                    .collect();
                unified(at, part, tabs)
            }
            Shown::Dollar(rows) => dollar(rows, part),
        }
    }
}

/// The popup that is up, copied out of the session so a rebuild holds no
/// borrow.
enum Shown {
    At(Rc<At>),
    Dollar(Rc<[DollarRow]>),
}

struct At {
    rows: Vec<Candidate>,
    query: String,
    mode: SearchMode,
    empty: Empty,
}

/// One row of the `$` popup.
struct DollarRow {
    name: String,
    indices: Option<Vec<usize>>,
    description: Option<String>,
    kind: MentionKind,
}

impl Shown {
    fn read(mentions: &Mentions) -> Option<Shown> {
        mentions.read(|session| match session.active() {
            Active::None => None,
            Active::At(popup) => Some(Shown::At(Rc::new(At {
                rows: popup.rows().to_vec(),
                query: popup.query().to_string(),
                mode: popup.search_mode(),
                empty: popup.empty(),
            }))),
            Active::Dollar(popup) => Some(Shown::Dollar(
                popup
                    .rows()
                    .into_iter()
                    .filter_map(|m| {
                        let item = popup.items().get(m.index)?;
                        Some(DollarRow {
                            name: item.shown_name(),
                            indices: m.indices,
                            description: item.description.clone().filter(|d| !d.is_empty()),
                            kind: item.kind,
                        })
                    })
                    .collect(),
            )),
        })
    }

    fn len(&self) -> usize {
        match self {
            Shown::At(at) => at.rows.len(),
            Shown::Dollar(rows) => rows.len(),
        }
    }
}

#[derive(Clone)]
struct Look {
    dim: Style,
    bold: Style,
    accent: Style,
    /// The cursor row (`selection_style`).
    selected: Style,
    info: Style,
    /// The `↑` and `↓` rows (`accent_color_on` the painted fill).
    arrow: Style,
}

/// What both popups share: the controllers, the row taps, the looks.
#[derive(Clone)]
struct Part {
    cursor: SelectionController<usize>,
    list: ListController,
    taps: Rc<Vec<Handler<PointerEvent>>>,
    look: Look,
    table: &'static Strings,
}

impl Part {
    /// The rows as a `ListView` `height` high; `row` draws row `at`, told
    /// whether the cursor is on it. A tap on a row picks it; the cursor row
    /// is filled across the whole width (`line.style(selection_style())`).
    fn rows(
        &self,
        len: usize,
        height: i32,
        row: impl Fn(usize, bool) -> AnyWidget + 'static,
    ) -> AnyWidget {
        let (cursor, taps, selected) = (
            self.cursor.clone(),
            self.taps.clone(),
            self.look.selected.clone(),
        );
        let row = Rc::new(row);
        let list = ListView::builder(len, move |at| {
            let (row, cursor, selected) = (row.clone(), cursor.clone(), selected.clone());
            let built = ListenableBuilder::new(cursor.listenable(), move |_, _| {
                let on = cursor.cursor() == Some(at);
                let cells = row(at, on);
                if on {
                    BoxWidget::new().style(selected.clone()).child(cells).into()
                } else {
                    cells
                }
            });
            match taps.get(at) {
                Some(tap) => GestureDetector::new()
                    .on_tap(tap.clone())
                    .child(built)
                    .into(),
                None => built.into(),
            }
        })
        .item_extent(1)
        .controller(self.list.clone())
        .can_request_focus(false);
        SizedBox::new(None, Some(height), Some(list.into())).into()
    }

    /// The message of an empty list, `height` rows high.
    fn none(&self, spans: Vec<Span>, height: i32) -> AnyWidget {
        SizedBox::new(
            None,
            Some(height),
            Some(
                Text::rich_line(spans)
                    .overflow(TextOverflow::Ellipsis)
                    .into(),
            ),
        )
        .into()
    }

    /// A row high, the glyph while rows are scrolled out of view above (or
    /// below): the oracle's `↑` and `↓` (`render_scroll_indicators`), read
    /// from the list's position, so they follow its scroll.
    fn arrow(&self, above: bool) -> AnyWidget {
        let list = self.list.clone();
        let watched = Rc::new((*self.list).clone());
        let glyph = if above {
            self.table.scroll_up
        } else {
            self.table.scroll_down
        };
        let style = self.look.arrow.clone();
        SizedBox::new(
            None,
            Some(1),
            Some(
                ListenableBuilder::new(watched, move |_, _| {
                    let shown = list.try_position().is_ok_and(|at| {
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

    /// The hint line: each key bold and its word dim, `left` and `right`
    /// cells in. One `Text` per key and per word: the text layout gives a
    /// blank between two styled spans the style of the span before it, and the
    /// oracle's blank after a key is dim. The last word is cut at the end;
    /// with `dots`, a line that does not fit is cut with a `…` on its last cell
    /// (`mentions_v2/footer.rs`), measured under a `LayoutBuilder`.
    fn hint(
        &self,
        parts: &[(&'static str, &'static str)],
        left: i32,
        right: i32,
        dots: bool,
    ) -> AnyWidget {
        let (bold, dim) = (self.look.bold.clone(), self.look.dim.clone());
        let parts = parts.to_vec();
        let line: AnyWidget = if dots {
            LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let room = usize::try_from(constraints.max_width).unwrap_or(0);
                let mut pieces: Vec<(String, Style)> = Vec::new();
                for (keys, word) in &parts {
                    pieces.push(((*keys).to_string(), bold.clone()));
                    pieces.push(((*word).to_string(), dim.clone()));
                }
                let total: usize = pieces.iter().map(|(text, _)| seg.width(text)).sum();
                if total > room {
                    pieces = cut_pieces(pieces, room, |text| seg.width(text));
                }
                let mut row = Row::new();
                for (text, style) in pieces {
                    row = row.child(Text::line(text).style(style));
                }
                row.into()
            })
            .into()
        } else {
            let mut row = Row::new();
            let last = parts.len().saturating_sub(1);
            for (at, (keys, word)) in parts.iter().enumerate() {
                row = row.child(Text::line(*keys).style(bold.clone()));
                let word = Text::line(*word).style(dim.clone());
                row = if at == last {
                    row.child(Flexible {
                        flex: 1,
                        fit: FlexFit::Loose,
                        child: word.into(),
                    })
                } else {
                    row.child(word)
                };
            }
            row.into()
        };
        SizedBox::new(
            None,
            Some(1),
            Some(
                BoxWidget::new()
                    .padding(EdgeInsets::only(left, 0, right, 0))
                    .child(line)
                    .into(),
            ),
        )
        .into()
    }
}

/// `pieces` cut to `room` cells with a `…` on the last cell, in the style of the piece it cuts.
fn cut_pieces(
    pieces: Vec<(String, Style)>,
    room: usize,
    width: impl Fn(&str) -> usize,
) -> Vec<(String, Style)> {
    let mut left = room.saturating_sub(1);
    let mut out = Vec::new();
    let mut style = Style::default();
    for (text, piece_style) in pieces {
        style = piece_style.clone();
        let mut kept = String::new();
        let mut cut = false;
        for ch in text.chars() {
            let cells = width(&ch.to_string());
            if cells > left {
                cut = true;
                break;
            }
            left -= cells;
            kept.push(ch);
        }
        if !kept.is_empty() {
            out.push((kept, piece_style));
        }
        if cut {
            break;
        }
    }
    if room > 0 {
        out.push(("\u{2026}".to_string(), style));
    }
    out
}

// ---- the unified `@` popup ---------------------------------------------------

/// The heights of the unified popup's parts in `content` rows (the popup
/// without its hint line): headers and tabs yield last, a row stays as long
/// as the chrome fits (`picker_areas` with the sizes of `render_popup`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sections {
    header: i32,
    header_gap: i32,
    tabs: i32,
    tab_gap: i32,
    search: i32,
    /// The height of the `↑` row and of the `↓` row.
    indicators: i32,
    rows: i32,
}

fn sections(content: i32) -> Sections {
    let (header_want, header_gap_want, tabs_want, search_want, rows_want) = (2, 1, 1, 1, MAX_ROWS);
    let tabs = tabs_want.min(content);
    let search = search_want.min(content - tabs).max(0);
    let minimum_rows = rows_want.min(1);
    let minimum_header = i32::from(content - tabs - search > i32::from(rows_want > 0));
    let header = header_want
        .min(content - tabs - search - minimum_rows)
        .max(minimum_header);
    let mut remaining = content - header - tabs - search;
    let indicators = i32::from(remaining >= minimum_rows + 2);
    remaining -= indicators * 2;
    let minimum_rows = rows_want.min(3).min(remaining);
    let header_gap = if header > 0 {
        header_gap_want.min(remaining - minimum_rows).max(0)
    } else {
        0
    };
    remaining -= header_gap;
    let tab_gap = i32::from(tabs > 0 && search > 0 && remaining > minimum_rows);
    remaining -= tab_gap;
    let rows = rows_want.min(remaining).max(0);
    Sections {
        header,
        header_gap,
        tabs,
        tab_gap,
        search,
        indicators,
        rows,
    }
}

fn unified(at: Rc<At>, part: Part, tab_taps: Vec<Handler<PointerEvent>>) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let height = constraints.max_height.min(POPUP_HEIGHT);
        if width == 0 || height <= 0 {
            return AnyWidget::default();
        }
        let table = part.table;
        let view = Rc::new(UnifiedRows::new(&at, &part, &|text| seg.width(text), width));
        let list = |rows: i32| -> AnyWidget {
            if at.rows.is_empty() {
                let message = match at.empty {
                    Empty::Loading => table.loading,
                    Empty::NoMatches => table.no_matches,
                };
                let spans = vec![
                    Span::new(table.mark_off),
                    Span::styled(message, Style::new().attrs(Attrs::ITALIC)),
                ];
                return part.none(spans, rows);
            }
            let view = view.clone();
            part.rows(at.rows.len(), rows, move |index, selected| {
                view.cells(index, selected)
            })
        };
        // at very short heights the selected result comes before any control
        if height < 4 {
            return list(height);
        }
        let s = sections(height - 1);
        let inset = |child: AnyWidget| -> AnyWidget {
            BoxWidget::new()
                .padding(EdgeInsets::only(2, 0, 2, 0))
                .child(child)
                .into()
        };
        let line = |text: &'static str, style: &Style| -> AnyWidget {
            SizedBox::new(
                None,
                Some(1),
                Some(inset(
                    Text::line(text)
                        .style(style.clone())
                        .overflow(TextOverflow::Clip)
                        .into(),
                )),
            )
            .into()
        };
        let gap = |rows: i32| -> AnyWidget { SizedBox::new(None, Some(rows), None).into() };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        if s.header > 0 {
            column = column.child(line(table.mentions_title, &part.look.bold));
        }
        if s.header > 1 {
            column = column.child(line(table.mentions_subtitle, &part.look.dim));
        }
        column = column.child(gap(s.header_gap));
        if s.tabs > 0 {
            let labels: Rc<[String]> = [table.tab_results, table.tab_filesystem, table.tab_plugins]
                .iter()
                .map(ToString::to_string)
                .collect();
            let strip = Tabs {
                labels,
                active: at.mode.index(),
                marks: (table.tab_left, table.tab_right),
                // the oracle's unknown-palette fallback (`active_tab_style`)
                styles: (
                    Style::new().attrs(Attrs::BOLD | Attrs::UNDERLINE),
                    part.look.dim.clone(),
                ),
                taps: tab_taps.clone(),
            }
            .widget();
            column = column.child(SizedBox::new(None, Some(1), Some(inset(strip))));
        }
        column = column.child(gap(s.tab_gap));
        if s.search > 0 {
            let query = if at.query.is_empty() {
                table.mentions_placeholder.to_string()
            } else {
                at.query.clone()
            };
            column = column.child(SizedBox::new(
                None,
                Some(1),
                Some(inset(dots_end(query, part.look.dim.clone()))),
            ));
        }
        if s.indicators > 0 {
            column = column.child(part.arrow(true));
        }
        column = column.child(list(s.rows));
        if s.indicators > 0 {
            column = column.child(part.arrow(false));
        }
        // the hint takes the last row of the popup
        let used =
            s.header + s.header_gap + s.tabs + s.tab_gap + s.search + s.indicators * 2 + s.rows;
        column = column.child(gap((height - 1 - used).max(0)));
        let keys = [
            (table.hint_insert_keys, table.hint_insert),
            (table.hint_close_key, table.hint_close_more),
            (table.hint_select_keys, table.hint_select),
            (table.hint_filter_keys, table.hint_filter),
        ];
        column.child(part.hint(&keys, 2, 2, true)).into()
    })
    .into()
}

/// What the rows of the unified popup are drawn from, measured once at one
/// width.
struct UnifiedRows {
    rows: Vec<Candidate>,
    look: Look,
    table: &'static Strings,
    /// The cells of the name column, the widest name bounded so a long one
    /// cannot crowd out the description.
    primary: i32,
}

impl UnifiedRows {
    fn new(at: &At, part: &Part, measure: &dyn Fn(&str) -> usize, width: usize) -> UnifiedRows {
        let table = part.table;
        // the mark, the gap before the type tag and the tag
        let chrome = measure(table.mark_off) + 2 + usize::try_from(TAG_CELLS).unwrap_or(0);
        let content = width.saturating_sub(chrome);
        let widest = at
            .rows
            .iter()
            .map(|row| measure(primary(row)))
            .max()
            .unwrap_or(0);
        UnifiedRows {
            rows: at.rows.clone(),
            look: part.look.clone(),
            table,
            primary: i32::try_from(widest.min(content.saturating_sub(2) / 2)).unwrap_or(0),
        }
    }

    fn tag_text(&self, kind: MentionType) -> &'static str {
        self.table.type_tags[match kind {
            MentionType::Plugin => 0,
            MentionType::Skill => 1,
            MentionType::Task => 2,
            MentionType::File => 3,
            MentionType::Directory => 4,
        }]
    }

    /// The style of a part of the row: the cursor row has one style for all
    /// (`selection_style`).
    fn style(&self, selected: bool, own: &Style) -> Style {
        if selected {
            self.look.selected.clone()
        } else {
            own.clone()
        }
    }

    fn cells(&self, index: usize, selected: bool) -> AnyWidget {
        let Some(row) = self.rows.get(index) else {
            return AnyWidget::default();
        };
        let look = &self.look;
        let mark = Text::line(if selected {
            self.table.mark_on
        } else {
            self.table.mark_off
        })
        .style(self.style(selected, &Style::new()));
        let tag_style = match row.mention_type {
            MentionType::Plugin => look.accent.clone(),
            MentionType::Skill => look.dim.clone(),
            MentionType::Task | MentionType::File => look.info.clone(),
            MentionType::Directory => Style::new(),
        };
        let tag =
            Text::line(self.tag_text(row.mention_type)).style(self.style(selected, &tag_style));
        let primary =
            Text::rich_line(self.primary_spans(row, selected)).overflow(TextOverflow::Ellipsis);
        let content: AnyWidget = match self.secondary(row, selected) {
            Some(secondary) => Row::new()
                .child(SizedBox::new(
                    Some(self.primary),
                    Some(1),
                    Some(primary.into()),
                ))
                .child(SizedBox::new(Some(2), None, None))
                .child(Expanded::new(
                    1,
                    Text::rich_line(secondary).overflow(TextOverflow::Ellipsis),
                ))
                .into(),
            None => primary.into(),
        };
        Row::new()
            .child(SizedBox::new(Some(2), Some(1), Some(mark.into())))
            .child(Expanded::new(1, content))
            .child(SizedBox::new(Some(2), None, None))
            .child(SizedBox::new(Some(TAG_CELLS), Some(1), Some(tag.into())))
            .into()
    }

    /// The name column: a file's name, else the display name with the
    /// matched letters bold.
    fn primary_spans(&self, row: &Candidate, selected: bool) -> Vec<Span> {
        let look = &self.look;
        if file_name_start(row).is_some() {
            let own = if row.mention_type == MentionType::File {
                look.info.clone()
            } else {
                Style::new()
            };
            return vec![Span::styled(
                primary(row).to_string(),
                self.style(selected, &own),
            )];
        }
        let own = match row.mention_type {
            MentionType::Plugin => look.accent.clone(),
            MentionType::Skill => look.dim.clone(),
            MentionType::Task => look.info.clone(),
            MentionType::File | MentionType::Directory => Style::new(),
        };
        matched(
            &row.display_name,
            row.match_indices.as_deref(),
            usize::MAX,
            &self.style(selected, &own),
        )
    }

    /// The description column: a file's directory (its matched letters bold)
    /// and description, else the description (a plugin without one says what
    /// it carries).
    fn secondary(&self, row: &Candidate, selected: bool) -> Option<Vec<Span>> {
        let dim = self.style(selected, &self.look.dim);
        let description = row
            .description
            .clone()
            .filter(|description| !description.is_empty())
            .or_else(|| {
                (row.mention_type == MentionType::Plugin)
                    .then(|| self.table.plugin_description(&row.capabilities))
            });
        let Some(start) = file_name_start(row) else {
            return description.map(|text| vec![Span::styled(text, dim)]);
        };
        let mut spans = if start == 0 {
            vec![Span::styled(self.table.current_dir, dim.clone())]
        } else {
            matched(&row.display_name, row.match_indices.as_deref(), start, &dim)
        };
        if let Some(text) = description {
            spans.push(Span::styled(self.table.description_gap, dim.clone()));
            spans.push(Span::styled(text, dim));
        }
        Some(spans)
    }
}

/// The first `take` characters of `text` in `style`, the ones at `indices`
/// bold, as the runs between them.
fn matched(text: &str, indices: Option<&[usize]>, take: usize, style: &Style) -> Vec<Span> {
    let bold = style.clone().attrs(Attrs::BOLD);
    let Some(indices) = indices else {
        let shown: String = text.chars().take(take).collect();
        return vec![Span::styled(shown, style.clone())];
    };
    let mut spans: Vec<Span> = Vec::new();
    let mut run = String::new();
    let mut run_bold = false;
    for (at, ch) in text.chars().enumerate().take(take) {
        let is_bold = indices.contains(&at);
        if is_bold != run_bold && !run.is_empty() {
            let own = if run_bold { &bold } else { style };
            spans.push(Span::styled(std::mem::take(&mut run), own.clone()));
        }
        run_bold = is_bold;
        run.push(ch);
    }
    if !run.is_empty() {
        let own = if run_bold { &bold } else { style };
        spans.push(Span::styled(run, own.clone()));
    }
    spans
}

// ---- the `$` popup ------------------------------------------------------------

/// Where the `$` popup's tag column starts for rows whose gutter and name
/// are `names` cells wide, at `width` (`compute_desc_col`: all rows; 0 hides
/// the descriptions when they would have too little room).
fn description_column(names: &[usize], width: usize) -> usize {
    if width <= 1 {
        return 0;
    }
    // at least 30% of the width stays for descriptions
    let most = (width - 1).min((width * 70 / 100).max(1));
    let widest = names.iter().copied().max().unwrap_or(0);
    let column = (widest + 2).min(most);
    if width.saturating_sub(column) < MIN_DESCRIPTION {
        0
    } else {
        column
    }
}

fn dollar(rows: Rc<[DollarRow]>, part: Part) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let wanted = i32::try_from(rows.len())
            .unwrap_or(i32::MAX)
            .clamp(1, MAX_ROWS)
            + 3;
        let height = constraints.max_height.min(wanted);
        if width == 0 || height <= 0 {
            return AnyWidget::default();
        }
        let table = part.table;
        let view = Rc::new(DollarRows::new(
            &rows,
            &part,
            &|text| seg.width(text),
            width,
        ));
        let body = |height: i32| -> AnyWidget {
            if rows.is_empty() {
                let spans = vec![Span::styled(
                    table.no_matches,
                    part.look.dim.clone().attrs(Attrs::ITALIC),
                )];
                return part.none(spans, height);
            }
            let view = view.clone();
            part.rows(rows.len(), height, move |index, selected| {
                view.cells(index, selected)
            })
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        // the hint takes the last row; a list of three rows or more holds its
        // `↑` and `↓` rows (`picker_rows::render_rows_single_line`)
        if height > 1 {
            if height > 3 {
                column = column.child(part.arrow(true));
                column = column.child(body(height - 3));
                column = column.child(part.arrow(false));
            } else {
                column = column.child(body(height - 1));
            }
            let keys = [
                (table.skill_hint_insert_key, table.hint_insert),
                (table.hint_close_key, table.skill_hint_close),
            ];
            column = column.child(part.hint(&keys, 2, 0, false));
        } else {
            column = column.child(body(height));
        }
        column.into()
    })
    .into()
}

/// What the rows of the `$` popup are drawn from, measured once at one
/// width.
struct DollarRows {
    rows: Rc<[DollarRow]>,
    look: Look,
    table: &'static Strings,
    /// Per row: where its tag starts.
    starts: Vec<usize>,
    /// Whether the description column is shown (it hides when narrow).
    described: bool,
}

impl DollarRows {
    fn new(
        rows: &Rc<[DollarRow]>,
        part: &Part,
        measure: &dyn Fn(&str) -> usize,
        width: usize,
    ) -> DollarRows {
        let table = part.table;
        let tag_of = |kind: MentionKind| match kind {
            MentionKind::App => table.tag_app,
            MentionKind::Skill | MentionKind::Plugin => table.tag_skill,
        };
        let mark = measure(table.mark_off);
        let names: Vec<usize> = rows.iter().map(|row| mark + measure(&row.name)).collect();
        let column = description_column(&names, width);
        let starts = names
            .iter()
            .zip(rows.iter())
            .map(|(name, row)| {
                // the description column, or just after the name, but
                // always inside the row
                let wanted = if column > 0 { column } else { name + 2 };
                wanted.min(width.saturating_sub(measure(tag_of(row.kind))))
            })
            .collect();
        DollarRows {
            rows: rows.clone(),
            look: part.look.clone(),
            table,
            starts,
            described: column > 0,
        }
    }

    fn cells(&self, index: usize, selected: bool) -> AnyWidget {
        let (Some(row), Some(start)) = (self.rows.get(index), self.starts.get(index)) else {
            return AnyWidget::default();
        };
        let look = &self.look;
        // the cells between the mark and the tag, less a gap of two
        let name_cells = i32::try_from(start.saturating_sub(2 + 2)).unwrap_or(0);
        let (name_style, side_style) = if selected {
            (
                look.selected.clone(),
                // a dim span loses its bold on the cursor row
                look.selected.clone().attrs_off(Attrs::BOLD),
            )
        } else {
            (Style::new(), look.dim.clone())
        };
        let mark = Text::line(if selected {
            self.table.mark_on
        } else {
            self.table.mark_off
        })
        .style(if selected {
            look.selected.clone()
        } else {
            Style::new()
        });
        let name = Text::rich_line(matched(
            &row.name,
            row.indices.as_deref(),
            usize::MAX,
            &name_style,
        ))
        .overflow(TextOverflow::Ellipsis);
        let tag = Text::line(match row.kind {
            MentionKind::App => self.table.tag_app,
            MentionKind::Skill | MentionKind::Plugin => self.table.tag_skill,
        })
        .style(side_style.clone());
        let mut line = Row::new()
            .child(SizedBox::new(Some(2), Some(1), Some(mark.into())))
            .child(SizedBox::new(Some(name_cells), Some(1), Some(name.into())))
            .child(SizedBox::new(Some(2), None, None))
            .child(tag);
        if self.described
            && let Some(description) = &row.description
        {
            line = line
                .child(Text::line(" ").style(side_style.clone()))
                .child(Expanded::new(
                    1,
                    Text::line(description.clone())
                        .style(side_style)
                        .overflow(TextOverflow::Ellipsis),
                ));
        }
        line.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(content: i32) -> (i32, i32, i32, i32, i32, i32, i32) {
        let s = sections(content);
        (
            s.header,
            s.header_gap,
            s.tabs,
            s.tab_gap,
            s.search,
            s.indicators,
            s.rows,
        )
    }

    #[test]
    fn a_tall_popup_has_every_part_and_eight_rows() {
        assert_eq!(at(16), (2, 1, 1, 1, 1, 1, 8));
    }

    #[test]
    fn a_short_popup_keeps_its_chrome_before_its_rows() {
        // the snapshot of the oracle at five lines: a header line, the tabs,
        // the query and one row, no gaps and no overflow rows
        assert_eq!(at(4), (1, 0, 1, 0, 1, 0, 1));
    }

    #[test]
    fn the_overflow_rows_come_with_three_rows() {
        let s = sections(9);
        assert_eq!(s.indicators, 1);
        assert!(s.rows >= 3);
    }

    #[test]
    fn the_description_column_hides_when_narrow() {
        // 100 wide: the widest name (22) and two blanks, the rest is plenty
        assert_eq!(description_column(&[22, 10], 100), 24);
        // 40 wide: 40 - 24 is 16, less than the 24 the descriptions need
        assert_eq!(description_column(&[22, 10], 40), 0);
        // the column is never wider than 70% of the width
        assert_eq!(description_column(&[90], 100), 70);
    }
}
