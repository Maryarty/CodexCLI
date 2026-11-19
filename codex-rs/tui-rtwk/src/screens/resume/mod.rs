//! `resume` (S18): the view half of the resume and fork picker. One
//! `PageRoute` over the model's rows (`ScreenStates.resume`): the header, the
//! filter/status/sort toolbar, the search line, the list of saved threads with
//! their meta lines and the footer with progress and key hints. Its model,
//! events and intents are core's `codex_tui_core::screens::resume`.
//! Oracle: `tui/src/resume_picker.rs` and `resume_picker/layout.rs`; the chrome's rows
//! and ages are in `layout.rs`, what rtwk cannot pick by width in `fit.rs` (G39).

use std::rc::Rc;

use rtwk::controls::{SelectionController, SelectionList};
use rtwk::focus::{Focus, KeyBindings};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::layout::{
    BorderType, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox, Wrap,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::scroll::ScrollNotification;
use rtwk::style::{Attrs, Span, Style, Theme};
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{
    AnyWidget, Cx, LayoutBuilder, NotificationListener, Rebuild, State, StatefulWidget, impl_widget,
};

use codex_tui_core::screens::resume::{Intent as ResumeIntent, Load, Model, Row as Saved};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod fit;
mod layout;
mod list;
pub mod strings;

use strings::Strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "fake_rollouts",
            description: "a resume list from three fake rollouts",
            script: include_str!("fixtures/fake_rollouts.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| list::List.into()),
        },
        DebugEntry {
            name: "table",
            description: "three rows: a cloud one, an unread one, a plain one",
            script: include_str!("fixtures/table.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        },
        DebugEntry {
            name: "more_indicators",
            description: "five rows, to scroll down to the third",
            script: include_str!("fixtures/more_indicators.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen.into()
            }),
        },
        DebugEntry {
            name: "search_error",
            description: "a listing that failed: the message on the search line",
            script: include_str!("fixtures/search_error.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        },
        DebugEntry {
            name: "footer_wide",
            description: "no rows at 220 cells: the footer's wide hints",
            script: include_str!("fixtures/footer_wide.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        },
        DebugEntry {
            name: "loading",
            description: "the first page is loading",
            script: include_str!("fixtures/loading.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        },
        DebugEntry {
            name: "empty",
            description: "no saved sessions",
            script: include_str!("fixtures/empty.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen.into())
}

/// Opens the picker on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// The picker.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: the selection (rtwk's controller owns the cursor and the
/// scroll) and where the list is scrolled to, told by its notifications.
#[derive(Debug)]
pub struct ScreenState {
    selection: SelectionController<usize>,
    scroll: Rc<ValueNotifier<Scrolled>>,
}

/// The list's scroll metrics: the rows above the window, the rows below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Scrolled {
    pixels: i64,
    max: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Accept(usize),
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selection: SelectionController::with(false, [0], Some(0)),
            scroll: Rc::new(ValueNotifier::new(Scrolled::default())),
        }
    }
}

impl_widget!(stateful eq Screen);

/// What a row of the list draws with.
#[derive(Clone)]
struct Look {
    dim: Style,
    italic: Style,
    bold: Style,
    accent: Style,
    error: Style,
}

/// The header, the toolbar, the search line, the list and the footer, one
/// under the other with the oracle's collapsing gaps.
fn screen(
    cx_look: &Look,
    strings: &'static Strings,
    num: rtwk::l10n::NumSyms,
    screens: Rc<ScreenStates>,
    selection: SelectionController<usize>,
    scroll: Rc<ValueNotifier<Scrolled>>,
    accept: rtwk::Handler<usize>,
) -> AnyWidget {
    let look = cx_look.clone();
    LayoutBuilder::new(move |_, constraints| {
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let height = u16::try_from(constraints.max_height).unwrap_or(0);
        let areas = layout::areas(height);
        let model = &screens.resume;
        let gap = |rows: u16| SizedBox::new(None, Some(i32::from(rows)), None);
        let chrome = |line: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::only(1, 0, 1, 0), line).into()
        };
        let toolbar: Vec<Span> = fit::toolbar(strings, width.saturating_sub(2))
            .into_iter()
            .map(|(text, active)| Span::styled(text, if active { look.bold.clone() } else { look.dim.clone() }))
            .collect();
        let search = match &model.load {
            Load::Failed(message) => Text::new(message.clone()).style(look.error.clone()),
            _ => Text::new(strings.search_hint).style(look.dim.clone()),
        };
        let one = |text: Text| text.wrap(TextWrap::None).max_lines(1).overflow(TextOverflow::Ellipsis);
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(chrome(one(Text::new(strings.header).style(look.bold.clone())).into()))
            .child(gap(areas.toolbar - 1))
            .child(chrome(one(Text::rich(toolbar).wrap(TextWrap::None)).into()))
            .child(gap(areas.search - areas.toolbar - 1))
            .child(chrome(one(search).into()))
            .child(gap(areas.list - areas.search - 1))
            .child(SizedBox::new(
                None,
                Some(i32::from(areas.list_height)),
                Some(
                    Padding::new(
                        EdgeInsets::only(i32::from(layout::LIST_INSET), 0, 0, 0),
                        list_of(&look, strings, num, screens.clone(), selection.clone(), scroll.clone(), accept.clone()),
                    )
                    .into(),
                ),
            ))
            .child(footer(&look, strings, screens.clone(), selection.clone(), scroll.clone(), width))
            .into()
    })
    .into()
}

/// The footer: the rule with the progress label, then the hint rows.
fn footer(
    look: &Look,
    strings: &'static Strings,
    screens: Rc<ScreenStates>,
    selection: SelectionController<usize>,
    scroll: Rc<ValueNotifier<Scrolled>>,
    width: usize,
) -> AnyWidget {
    let look = look.clone();
    let [first, second] = fit::hint_rows(strings, width);
    let rule = {
        let look = look.clone();
        ListenableBuilder::new(selection.listenable(), move |_, _| {
            let (look, screens, selection) = (look.clone(), screens.clone(), selection.clone());
            ValueListenableBuilder::new(scroll.clone(), move |_, at: &Scrolled, _| {
                let model = &screens.resume;
                let rows = model.rows.len();
                let position = if rows == 0 { 0 } else { selection.cursor().map_or(1, |at| at + 1) };
                let total = if matches!(model.load, Load::Loading) { format!("{rows}…") } else { rows.to_string() };
                let percent = if at.max <= 0 { 100 } else { ((at.pixels as f64 / at.max as f64) * 100.0).round() as u8 };
                let label = fit::progress_label(position, &total, percent, width);
                Row::new()
                    .child(Expanded::new(1, Divider::new().kind(BorderType::Single).style(look.dim.clone()).merge(false)))
                    .child(Text::line(label).style(look.dim.clone()))
                    .child(SizedBox::new(Some(1), Some(1), Some(Divider::new().kind(BorderType::Single).style(look.dim.clone()).merge(false).into())))
                    .into()
            })
            .into()
        })
    };
    let hint = |text: String| Text::line(text).style(look.dim.clone()).wrap(TextWrap::None);
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(rule)
        .child(hint(first))
        .child(hint(second))
        .into()
}

/// The saved threads: a `SelectionList` of rows (title, meta lines, a blank
/// row between), with the "more" rows above and below while the list is
/// scrolled.
fn list_of(
    look: &Look,
    strings: &'static Strings,
    num: rtwk::l10n::NumSyms,
    screens: Rc<ScreenStates>,
    selection: SelectionController<usize>,
    scroll: Rc<ValueNotifier<Scrolled>>,
    accept: rtwk::Handler<usize>,
) -> AnyWidget {
    let look = look.clone();
    LayoutBuilder::new(move |_, constraints| {
        let model = &screens.resume;
        if model.rows.is_empty() {
            let text = match model.load {
                Load::Loading => strings.loading,
                _ => strings.empty,
            };
            return Text::new(text).style(look.italic.clone()).into();
        }
        let width = constraints.max_width;
        let builder = {
            let (look, screens, selection) = (look.clone(), screens.clone(), selection.clone());
            move |index: usize| -> AnyWidget {
                let (look, screens, selection) = (look.clone(), screens.clone(), selection.clone());
                ListenableBuilder::new(selection.clone().listenable(), move |_, _| {
                    let selected = selection.cursor() == Some(index);
                    item(&look, strings, num, &screens.resume, index, selected, width)
                })
                .into()
            }
        };
        let rows = SelectionList::new(model.rows.len(), |index| index, builder)
            .controller(selection.clone())
            .on_submitted(accept.clone())
            .autofocus(true)
            .bar(false)
            .idle_style(Style::new())
            .selected_style(Style::new())
            .disabled_style(Style::new())
            .cursor_style(Style::new());
        let notify = scroll.clone();
        let rows = NotificationListener::<ScrollNotification>::new(rows).on_notification(move |note| {
            notify.set_value(Scrolled {
                pixels: note.metrics.pixels,
                max: note.metrics.max_scroll_extent,
            });
            false
        });
        let more = |text: &'static str, down: bool| -> AnyWidget {
            let style = look.accent.clone();
            ValueListenableBuilder::new(scroll.clone(), move |_, at: &Scrolled, _| {
                let show = if down { at.pixels < at.max } else { at.pixels > 0 };
                if show {
                    Text::line(text).style(style.clone()).into()
                } else {
                    SizedBox::new(Some(0), Some(0), None).into()
                }
            })
            .into()
        };
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(more(strings.more_above, false))
            .child(Expanded::new(1, rows))
            .child(more(strings.more_below, true))
            .into()
    })
    .into()
}

/// One saved thread (oracle: `render_list`): the marker, the unread dot and
/// the title, the meta lines, a blank row to the next one.
fn item(
    look: &Look,
    strings: &Strings,
    num: rtwk::l10n::NumSyms,
    model: &Model,
    index: usize,
    selected: bool,
    width: i32,
) -> AnyWidget {
    let row: &Saved = &model.rows[index];
    let marker = if selected { strings.marker_selected } else { strings.marker_other };
    let marker_style = if selected { look.accent.clone() } else { Style::new() };
    let title_style = if selected { look.bold.clone() } else { Style::new() };
    let mut title = Row::new().child(Text::line(marker).style(marker_style));
    if row.unread {
        title = title.child(Text::line(strings.unread_dot).style(look.accent.clone()));
    }
    title = title.child(Expanded::new(
        1,
        Text::new(row.display_preview().to_owned())
            .style(title_style)
            .wrap(TextWrap::None)
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis),
    ));
    let date = strings.age(layout::age(model.clock, row.updated.or(row.created)), &num);
    let part = |icon: &'static str, text: &Option<String>, none: &'static str| -> Text {
        let (text, style) = match text {
            Some(text) => (text.clone(), look.dim.clone()),
            None => (none.to_owned(), look.italic.clone()),
        };
        Text::rich([Span::styled(icon, look.dim.clone()), Span::styled(text, style)])
            .wrap(TextWrap::None)
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis)
    };
    let cwd_width = fit_cwd(width);
    let meta = Wrap::new()
        .spacing(2)
        .run_spacing(0)
        .child(SizedBox::new(Some(12), Some(1), Some(Text::line(date).style(look.dim.clone()).into())))
        .child(SizedBox::new(Some(cwd_width), Some(1), Some(part(strings.cwd_icon, &row.cwd, strings.no_cwd).into())))
        .child(part(strings.branch_icon, &row.branch, strings.no_branch));
    let mut column = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(title)
        .child(Padding::new(EdgeInsets::only(2, 0, 0, 0), meta));
    if index + 1 < model.rows.len() {
        column = column.child(SizedBox::new(None, Some(1), None));
    }
    column.into()
}

/// The cwd column of a meta line (oracle: `cwd_column_width`).
fn fit_cwd(width: i32) -> i32 {
    ((width - 2 - 12 - 4) / 2).clamp(30, 72)
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Accept(index) => {
                let screens = watch::<ScreenStates, _>(cx);
                if let Some(row) = screens.resume.rows.get(index) {
                    let hs = read::<Hs, _>(cx);
                    let intent = ScreenIntent::Resume(ResumeIntent::Resume(row.id.clone()));
                    cx.emit(&hs.screen, intent);
                }
            }
            Msg::Close => {
                Navigator::maybe_pop(cx);
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let look = Look {
            dim: tokens.dim(),
            italic: tokens.dim().attrs(Attrs::ITALIC),
            bold: tokens.bold(),
            accent: tokens.accent(),
            error: tokens.error(),
        };
        // The oracle cuts a title with three dots, rtwk's theme glyph is the
        // one-cell mark: the glyph for this screen.
        let mut data = (*Theme::of(cx)).clone();
        data.glyphs.ellipsis = "...".into();
        let accept = cx.map(Msg::Accept);
        let body = screen(&look, strings, num, screens, self.selection.clone(), self.scroll.clone(), accept);
        let keys = KeyBindings::new(
            [("escape", cx.send(Msg::Close))],
            Focus::new(body),
        );
        Theme::new(data, keys).into()
    }
}
