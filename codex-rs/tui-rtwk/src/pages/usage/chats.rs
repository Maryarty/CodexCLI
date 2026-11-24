//! The Chats report (oracle: `chat_panel.rs`, `task_panel.rs`): a business
//! account's chats by lifetime credits, a consumer account's by the metric
//! `s` picks. Which lines the report holds (`Model::chat_view`,
//! `Model::task_view`) and the cursor in them (`ChatPos`) are core's; the
//! words are `strings.rs`'s. A row is a `Row` of an `Expanded` title that the
//! end cuts with an ellipsis and its amounts in cells of fixed width; the row
//! under the cursor is filled with the selection style to the report's width.
//! What depends on the room (the dollar column stacking under the amounts, the
//! metrics all showing, the breakdown as a table) is picked from the measured
//! width, as the oracle does.

use std::rc::Rc;

use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BorderType, BoxWidget, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize,
    Padding, Row, SizedBox,
};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::Style;
use rtwk::text::{Text, TextAlign, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Handler, LayoutBuilder};

use codex_tui_core::screens::usage::View;
use codex_tui_core::screens::usage::chats::Pos as ChatPos;
use codex_tui_core::screens::usage::chats::{
    Amount, BUSINESS_ROW_MAX, CONSUMER_ROW_MAX, CREDITS_COLUMN, ChatLine, LEFT_ROOM, Metric,
    TABLE_FROM, TAG_COLUMN, TaskLine, Title, USD_COLUMN,
};

use super::cards::{Fit, Item};
use super::report::words;
use super::strings::Strings;
use super::{Look, gap};

/// A click on each row of the report (made by the widget that owns the
/// state, as many as the report has rows).
pub type RowTaps = Rc<Vec<Handler<PointerEvent>>>;

/// What the lines of one report are written from.
struct Say<'a> {
    strings: &'a Strings,
    num: &'a NumSyms,
    look: &'a Look,
    hour24: bool,
    taps: Option<&'a RowTaps>,
    /// The room the text wraps in; `width + pad` is the report's width.
    fit: Fit,
    pad: usize,
}

/// What a line needs of the lines before it.
#[derive(Default)]
struct Walk {
    /// The dollar column is there (business).
    usd: bool,
    /// The last row's title was cut by its end.
    cut: bool,
    /// The metrics offered with the width of each when they all show.
    columns: Option<Vec<usize>>,
    /// Where the sort metric is among the metrics offered.
    sorted: usize,
    /// The line just made is the row under the cursor, or one a click picks.
    fill: bool,
    tap: Option<usize>,
}

impl Say<'_> {
    fn width(&self) -> usize {
        self.fit.width
    }

    fn secondary(&self, text: impl Into<String>) -> Item {
        words(text.into(), self.look.secondary.clone(), Some(self.fit))
    }

    fn plain(&self, text: impl Into<String>) -> Item {
        words(text.into(), Style::new(), Some(self.fit))
    }

    /// One row of text the end cuts with an ellipsis.
    fn cut(&self, text: impl Into<String>, style: &Style) -> Text {
        Text::new(text.into())
            .style(style.clone())
            .wrap(TextWrap::None)
            .overflow(TextOverflow::Ellipsis)
            .max_lines(1)
    }

    /// A value at the right of its cell.
    fn cell(&self, text: impl Into<String>, style: &Style, width: usize) -> AnyWidget {
        SizedBox::new(
            Some(width as i32),
            None,
            Some(
                self.cut(text, style)
                    .overflow(TextOverflow::Clip)
                    .text_align(TextAlign::Right)
                    .into(),
            ),
        )
        .into()
    }

    /// The title at the left and the values at the right, one cell between
    /// (oracle: `columns` after `truncate`).
    fn columns(&self, left: Text, right: Vec<AnyWidget>) -> Row {
        let mut row = Row::new()
            .child(Expanded::new(1, left))
            .child(SizedBox::fixed(1, 1));
        for child in right {
            row = row.child(child);
        }
        row
    }

    /// A line in the report's width, filled with the selection style when it
    /// is the row under the cursor (oracle: `select_row`), a click on it
    /// doing what the tap does.
    fn finish(&self, child: impl Into<AnyWidget>, fill: bool, tap: Option<usize>) -> AnyWidget {
        let padded = Padding::new(EdgeInsets::only(0, 0, self.pad as i32, 0), child.into());
        let inner: AnyWidget = if fill {
            BoxWidget::new()
                .style(self.look.selected.clone())
                .child(padded)
                .into()
        } else {
            padded.into()
        };
        let sized: AnyWidget =
            SizedBox::new(Some((self.width() + self.pad) as i32), None, Some(inner)).into();
        match tap.and_then(|index| self.taps.and_then(|taps| taps.get(index))) {
            Some(handler) => GestureDetector::new()
                .on_tap(handler.clone())
                .child(sized)
                .into(),
            None => sized,
        }
    }

    fn indented(&self, cells: i32, child: impl Into<AnyWidget>) -> AnyWidget {
        Padding::new(EdgeInsets::only(cells, 0, 0, 0), child.into()).into()
    }

    fn rule(&self, indent: i32) -> AnyWidget {
        self.indented(
            indent,
            Divider::new()
                .kind(BorderType::Single)
                .style(self.look.dim.clone())
                .merge(false),
        )
    }

    fn title(&self, title: &Title) -> String {
        match title {
            Title::Text(text) => text.clone(),
            Title::Unavailable => self.strings.chat_unavailable.to_owned(),
        }
    }

    fn mark(&self, selected: bool) -> &'static str {
        if selected {
            self.strings.chat_mark_on
        } else {
            self.strings.chat_mark_off
        }
    }

    fn not_reported(&self, text: &Option<String>) -> String {
        text.clone()
            .unwrap_or_else(|| self.strings.chat_not_reported.to_owned())
    }

    fn dash(&self, text: &Option<String>) -> String {
        text.clone()
            .unwrap_or_else(|| self.strings.chat_dash.to_owned())
    }

    /// A line of a business account's report.
    fn chat(&self, walk: &mut Walk, line: &ChatLine) -> Option<Item> {
        let Say {
            strings, look, num, ..
        } = self;
        let width = self.width();
        let seg = self.fit.seg;
        let table = width >= TABLE_FROM;
        let bold = &look.bold;
        let secondary = &look.secondary;
        Some(match line {
            ChatLine::Blurb => self.secondary(strings.chat_blurb),
            ChatLine::Message(note) => self.secondary(strings.note(note).to_owned()),
            ChatLine::NoRecent => self.secondary(strings.chat_none),
            ChatLine::Missing(count) => self.secondary(strings.chat_missing(*count, num)),
            ChatLine::Blank => Item::one(gap()),
            ChatLine::Head { usd } => {
                walk.usd = *usd;
                let head = |text: &'static str| self.cut(text, bold);
                let natural = |text: &'static str| -> AnyWidget {
                    Text::new(text)
                        .style(bold.clone())
                        .wrap(TextWrap::None)
                        .into()
                };
                let right = if *usd && !table {
                    vec![natural(strings.chat_head_stacked)]
                } else if *usd {
                    vec![
                        self.cell(strings.chat_head_usd, bold, USD_COLUMN),
                        SizedBox::fixed(2, 1).into(),
                        self.cell(strings.chat_head_credits, bold, CREDITS_COLUMN),
                    ]
                } else {
                    vec![natural(strings.chat_head_credits)]
                };
                Item::one(self.columns(head(strings.chat_head_chat), right))
            }
            ChatLine::Row {
                index,
                selected,
                title,
                amount,
            } => {
                let style = if *selected {
                    look.selected.clone()
                } else {
                    Style::new()
                };
                let label = format!(
                    "{}{}{}",
                    self.mark(*selected),
                    strings.blank,
                    self.title(title)
                );
                let stacked = walk.usd && !table;
                let (missing, widgets, amount_width) = match amount {
                    Amount::Credits(credits) => {
                        let text = self.dash(credits);
                        let room = seg.width(&text);
                        (credits.is_none(), vec![text], room)
                    }
                    Amount::Both { usd, credits } => (
                        credits.is_none(),
                        vec![self.dash(usd), self.dash(credits)],
                        USD_COLUMN + 2 + CREDITS_COLUMN,
                    ),
                };
                let tint = if *selected {
                    look.selected.clone()
                } else if missing && !stacked {
                    secondary.clone()
                } else {
                    Style::new()
                };
                let room = if stacked {
                    width
                } else {
                    width.saturating_sub(amount_width + 1)
                };
                walk.cut = seg.width(&label) > room;
                let head = self.cut(label, &style);
                let row: AnyWidget = if stacked {
                    let mut it = widgets.into_iter();
                    let (usd, credits) =
                        (it.next().unwrap_or_default(), it.next().unwrap_or_default());
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(head)
                        .child(
                            Row::new()
                                .child(SizedBox::fixed(2, 1))
                                .child(self.cut(usd, &style).overflow(TextOverflow::Clip))
                                .child(SizedBox::fixed(2, 1))
                                .child(self.cell(credits, &style, CREDITS_COLUMN)),
                        )
                        .into()
                } else if walk.usd {
                    let mut it = widgets.into_iter();
                    let (usd, credits) =
                        (it.next().unwrap_or_default(), it.next().unwrap_or_default());
                    self.columns(
                        head,
                        vec![
                            self.cell(usd, &tint, USD_COLUMN),
                            SizedBox::fixed(2, 1).into(),
                            self.cell(credits, &tint, CREDITS_COLUMN),
                        ],
                    )
                    .into()
                } else {
                    let text = widgets.into_iter().next().unwrap_or_default();
                    self.columns(
                        head,
                        vec![Text::new(text).style(tint).wrap(TextWrap::None).into()],
                    )
                    .into()
                };
                walk.fill = *selected;
                walk.tap = Some(*index);
                Item::new(row, if stacked { 2 } else { 1 })
            }
            ChatLine::Rule => Item::one(self.rule(2)),
            ChatLine::LongTitle(title) => {
                if !walk.cut {
                    return None;
                }
                let text = self.title(title);
                let rows = Fit {
                    width: width.saturating_sub(4),
                    ..self.fit
                }
                .rows(&text);
                Item::new(
                    self.indented(4, Text::new(text).style(bold.clone()).wrap(TextWrap::Word)),
                    rows,
                )
            }
            ChatLine::GroupHead => {
                let credits = Text::new(strings.chat_group_credits)
                    .style(bold.clone())
                    .wrap(TextWrap::None);
                if table {
                    Item::one(
                        Row::new()
                            .child(SizedBox::fixed(2, 1))
                            .child(Expanded::new(
                                1,
                                self.cut(strings.chat_group_model, secondary),
                            ))
                            .child(SizedBox::fixed(2, 1))
                            .child(self.cell_left(strings.chat_group_effort, secondary, TAG_COLUMN))
                            .child(SizedBox::fixed(2, 1))
                            .child(self.cell_left(strings.chat_group_speed, secondary, TAG_COLUMN))
                            .child(self.cell(strings.chat_group_credits, bold, CREDITS_COLUMN)),
                    )
                } else {
                    Item::one(self.columns(
                        self.cut(strings.chat_group_narrow, secondary),
                        vec![credits.into()],
                    ))
                }
            }
            ChatLine::Group {
                model,
                effort,
                speed,
                credits,
            } => {
                let (effort, speed) = (self.not_reported(effort), self.not_reported(speed));
                let model = self.not_reported(model);
                if table {
                    Item::one(
                        Row::new()
                            .cross_axis_alignment(CrossAxisAlignment::End)
                            .child(SizedBox::fixed(2, 1))
                            .child(Expanded::new(
                                1,
                                Text::new(model).style(bold.clone()).wrap(TextWrap::Word),
                            ))
                            .child(SizedBox::fixed(2, 1))
                            .child(self.cell_left(effort, secondary, TAG_COLUMN))
                            .child(SizedBox::fixed(2, 1))
                            .child(self.cell_left(speed, secondary, TAG_COLUMN))
                            .child(self.cell(credits.clone(), &Style::new(), CREDITS_COLUMN)),
                    )
                } else {
                    let label = strings.chat_indent(&model, num);
                    let fits = seg.width(&label) + seg.width(credits) < width;
                    let amount = Text::new(credits.clone()).wrap(TextWrap::None);
                    let first: AnyWidget = if fits {
                        self.columns(self.cut(label, bold), vec![amount.into()])
                            .into()
                    } else {
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(Text::new(label).style(bold.clone()).wrap(TextWrap::Word))
                            .child(self.columns(
                                self.cut(strings.chat_indent_blank, &Style::new()),
                                vec![amount.into()],
                            ))
                            .into()
                    };
                    Item::new(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(first)
                            .child(
                                self.cut(
                                    strings.chat_effort_speed(&effort, &speed, num),
                                    secondary,
                                ),
                            ),
                        2,
                    )
                }
            }
            ChatLine::NoBreakdown => self.secondary(strings.chat_no_breakdown),
            ChatLine::Zeros { count, shown } => {
                self.secondary(strings.chat_zero(*count, *shown, num))
            }
            ChatLine::Showing { first, last, count } => {
                self.secondary(strings.chat_showing(*first, *last, *count, num))
            }
            ChatLine::Coverage => self.secondary(strings.chat_coverage),
            ChatLine::CardBlurb => self.secondary(strings.chat_card_blurb),
            ChatLine::CardRow { title, credits } => {
                let text = self.dash(credits);
                Item::one(self.columns(
                    self.cut(self.title(title), &Style::new()),
                    vec![
                        Text::new(text)
                            .style(look.accent.clone())
                            .wrap(TextWrap::None)
                            .into(),
                    ],
                ))
            }
            ChatLine::CardCoverage => self.secondary(strings.chat_card_coverage),
            ChatLine::CardMissing => self.secondary(strings.chat_card_missing),
        })
    }

    /// A value at the left of its cell.
    fn cell_left(&self, text: impl Into<String>, style: &Style, width: usize) -> AnyWidget {
        SizedBox::new(
            Some(width as i32),
            None,
            Some(self.cut(text, style).overflow(TextOverflow::Clip).into()),
        )
        .into()
    }

    /// A line of a consumer account's report.
    fn task(&self, walk: &mut Walk, line: &TaskLine) -> Item {
        let Say {
            strings, look, num, ..
        } = self;
        let width = self.width();
        let seg = self.fit.seg;
        let bold = &look.bold;
        let secondary = &look.secondary;
        let metric_name = |metric: Metric| strings.task_metric(metric.index());
        match line {
            TaskLine::Blurb(metric) => {
                self.secondary(strings.task_blurb(metric_name(*metric), num))
            }
            TaskLine::Basis { balance } => self.secondary(if *balance {
                strings.task_basis_balance
            } else {
                strings.task_basis_limit
            }),
            TaskLine::Message(note) => self.plain(
                note.as_ref()
                    .map_or(strings.task_no_report, |note| strings.note(note))
                    .to_owned(),
            ),
            TaskLine::NoRecent => self.plain(strings.chat_none),
            TaskLine::Partial { partial, missing } => {
                self.secondary(strings.task_partial_ranking(*partial, *missing, num))
            }
            TaskLine::Blank => Item::one(gap()),
            TaskLine::Head {
                metrics,
                metric,
                widest,
            } => {
                let columns: Vec<usize> = metrics
                    .iter()
                    .zip(widest)
                    .map(|(shown, widest)| (*widest).max(seg.width(metric_name(*shown))))
                    .collect();
                let all = columns.iter().sum::<usize>() + columns.len() * 2 + LEFT_ROOM <= width;
                walk.columns = all.then_some(columns);
                walk.sorted = metrics
                    .iter()
                    .position(|shown| shown == metric)
                    .unwrap_or(0);
                let right = match &walk.columns {
                    Some(columns) => self.metric_cells(
                        metrics
                            .iter()
                            .map(|shown| Some(metric_name(*shown).to_owned()))
                            .collect(),
                        columns,
                        bold,
                    ),
                    None => vec![
                        Text::new(metric_name(*metric))
                            .style(bold.clone())
                            .wrap(TextWrap::None)
                            .into(),
                    ],
                };
                Item::one(self.columns(self.cut(strings.chat_head_chat, bold), right))
            }
            TaskLine::Row {
                index,
                selected,
                title,
                partial,
                cells,
            } => {
                let style = if *selected {
                    look.selected.clone()
                } else {
                    Style::new()
                };
                let label = format!(
                    "{}{}{}{}",
                    self.mark(*selected),
                    strings.blank,
                    self.title(title),
                    if *partial {
                        strings.task_partial_mark
                    } else {
                        ""
                    }
                );
                let right = match &walk.columns {
                    Some(columns) => self.metric_cells(cells.clone(), columns, &style),
                    None => {
                        // The sort metric's cell alone.
                        let value = cells.get(walk.sorted).cloned().flatten();
                        let tint = if value.is_none() && !*selected {
                            secondary.clone()
                        } else {
                            style.clone()
                        };
                        vec![
                            Text::new(self.dash(&value))
                                .style(tint)
                                .wrap(TextWrap::None)
                                .into(),
                        ]
                    }
                };
                walk.fill = *selected;
                walk.tap = Some(*index);
                Item::one(self.columns(self.cut(label, &style), right))
            }
            TaskLine::Rule => Item::one(self.rule(0)),
            TaskLine::DetailTitle(title) => {
                let text = self.title(title);
                let rows = Fit {
                    width: width.saturating_sub(2),
                    ..self.fit
                }
                .rows(&text);
                Item::new(
                    self.indented(2, Text::new(text).style(bold.clone()).wrap(TextWrap::Word)),
                    rows,
                )
            }
            TaskLine::Cell { metric, value } => Item::one(
                Row::new()
                    .child(Expanded::new(
                        1,
                        self.cut(
                            strings.chat_indent(metric_name(*metric), num),
                            &Style::new(),
                        ),
                    ))
                    .child(Text::new(self.dash(value)).wrap(TextWrap::None)),
            ),
            TaskLine::GroupHead(metric) => Item::one(self.columns(
                self.cut(strings.chat_group_narrow, bold),
                vec![
                    Text::new(metric_name(*metric))
                        .style(bold.clone())
                        .wrap(TextWrap::None)
                        .into(),
                ],
            )),
            TaskLine::Group {
                model,
                effort,
                speed,
                product,
                value,
            } => {
                let label = strings.chat_indent(&self.not_reported(model), num);
                Item::new(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(self.columns(
                            self.cut(label, bold),
                            vec![Text::new(self.dash(value)).wrap(TextWrap::None).into()],
                        ))
                        .child(self.cut(
                            strings.task_group_tags(
                                &self.not_reported(effort),
                                &self.not_reported(speed),
                                &self.not_reported(product),
                                num,
                            ),
                            secondary,
                        )),
                    2,
                )
            }
            TaskLine::Unavailable => self.secondary(strings.task_unavailable),
            TaskLine::ShowingTop { shown, count } => {
                self.plain(strings.task_showing_top(*shown, *count, num))
            }
            TaskLine::Showing { first, last, count } => {
                self.secondary(strings.chat_showing(*first, *last, *count, num))
            }
            TaskLine::Coverage => self.secondary(strings.task_coverage),
            TaskLine::Truncated => self.secondary(strings.task_truncated),
            TaskLine::Updated(at) => self.secondary(strings.task_updated(*at, self.hour24, num)),
        }
    }

    /// The cells of all the metrics, each at the right of its column, two
    /// cells between.
    fn metric_cells(
        &self,
        values: Vec<Option<String>>,
        columns: &[usize],
        style: &Style,
    ) -> Vec<AnyWidget> {
        let mut out = Vec::new();
        for (at, (value, width)) in values.iter().zip(columns).enumerate() {
            if at > 0 {
                out.push(SizedBox::fixed(2, 1).into());
            }
            out.push(self.cell(self.dash(value), style, *width));
        }
        out
    }
}

/// What the report is made of.
enum Lines {
    Chat(Vec<ChatLine>),
    Task(Vec<TaskLine>),
}

fn stacked(items: Vec<Item>) -> AnyWidget {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children(items.into_iter().map(|item| item.widget))
        .into()
}

fn items(lines: &Lines, say: &Say) -> Vec<Item> {
    let mut walk = Walk::default();
    let mut made = Vec::new();
    let mut each = |walk: &mut Walk, item: Option<Item>| {
        if let Some(item) = item {
            let widget = say.finish(item.widget, walk.fill, walk.tap);
            made.push(Item::new(widget, item.rows));
        }
        walk.fill = false;
        walk.tap = None;
    };
    match lines {
        Lines::Chat(lines) => {
            for line in lines {
                let item = say.chat(&mut walk, line);
                each(&mut walk, item);
            }
        }
        Lines::Task(lines) => {
            for line in lines {
                let item = say.task(&mut walk, line);
                each(&mut walk, Some(item));
            }
        }
    }
    made
}

/// The room a report of this kind has in `width`: the report's width and
/// what the text wraps in, one cell short of it (oracle: `clamp`).
fn room(business: bool, width: usize, fit: Fit) -> (Fit, usize) {
    let row = width.clamp(
        1,
        if business {
            BUSINESS_ROW_MAX
        } else {
            CONSUMER_ROW_MAX
        },
    );
    let wrap = row.saturating_sub(1).max(1);
    (Fit { width: wrap, ..fit }, row - wrap)
}

/// What the Chats card shows: the first rows.
pub fn card(
    model: &View<'_>,
    pos: &ChatPos,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    fit: Fit,
) -> Vec<Item> {
    let say = |fit: Fit, pad: usize| Say {
        strings,
        num,
        look,
        hour24: model.hour24,
        taps: None,
        fit,
        pad,
    };
    if model.business() {
        // The business card has no margin: its rows take the card's width.
        items(&Lines::Chat(model.chat_card()), &say(fit, 0))
    } else {
        let (fit, pad) = room(false, fit.width, fit);
        items(&Lines::Task(model.task_view(pos, false)), &say(fit, pad))
    }
}

/// The focused Chats report: every row, the details of the open one, under
/// one scroll.
pub fn body(
    model: &View<'_>,
    pos: &ChatPos,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    taps: Option<RowTaps>,
    scroll: ScrollController,
) -> AnyWidget {
    let business = model.business();
    let lines = if business {
        Lines::Chat(model.chat_view(pos))
    } else {
        Lines::Task(model.task_view(pos, true))
    };
    let (strings, num, look, hour24) = (*strings, *num, look.clone(), model.hour24);
    LayoutBuilder::new(move |cx, constraints| {
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let fit = Fit {
            seg: segmenter_of(cx),
            width,
            plot_rows: 0,
        };
        let (fit, pad) = room(business, width, fit);
        let say = Say {
            strings: &strings,
            num: &num,
            look: &look,
            hour24,
            taps: taps.as_ref(),
            fit,
            pad,
        };
        SingleChildScrollView::new(stacked(items(&lines, &say)))
            .controller(scroll.clone())
            .into()
    })
    .into()
}
