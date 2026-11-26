//! The `/status` card as a cell of the transcript (oracle `status/card.rs`
//! `StatusHistoryCell::content_lines`, under the `/status` the user typed):
//! the codex title, the usage link, then labelled rows.
//!
//! Flutter: a stateless widget over a `LayoutBuilder`; the rows are a `Grid`
//! of a label track sized to its content and a value track, two cells apart,
//! under a two-cell `Padding`. The width decides only which spelling a value
//! gets (the bar and the reset time of a limit; the rows as paragraphs when
//! no room is left for a value), measured with the tree's segmenter, as
//! `widgets/dots.rs` does for its cut.

use std::sync::Arc;

use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Grid, GridPlacement, GridTrack, MainAxisSize, Padding,
    Row, SizedBox,
};
use rtwk::provider::{read, select};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::format::{TextLine, format_tokens_compact, text_table};
use codex_tui_core::screens::status::{
    Account, Figure, LimitData, LimitRow, RowValue, StatusCard, UsageInterest,
};

use super::super::text::{Part, Room};
use super::bar::{self, BAR_CELLS, bar};
use super::strings::{self, Strings};
use super::usage;
use crate::app::theme::Tokens;
use crate::widgets::interest::Interest;
use crate::widgets::measure::{self, Measure};

/// The cells before a card's text, and between a label and its value.
const INDENT: i32 = 2;
const GAP: i32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct StatusCell {
    pub card: Arc<StatusCard>,
}

impl_widget!(stateless eq StatusCell);

impl StatelessWidget for StatusCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let look = Look {
            s: strings::of(cx),
            faint: tokens.dim().attrs(Attrs::DIM),
            accent: tokens.accent(),
            bold: tokens.bold(),
        };
        let card = self.card.clone();
        // what this card holds of the thread's usage, by the number of the `/status` that made it
        let number = card.number;
        let held = select::<FeatureStates, Option<Option<Figure>>, _>(cx, move |features| {
            features.thread_usage.card_figure(number)
        });
        let sheet = LayoutBuilder::new(move |cx, constraints| -> AnyWidget {
            let width = i32::max(constraints.max_width, 0);
            sheet(
                &card,
                held.as_ref().map(Option::as_ref),
                &look,
                segmenter_of(cx),
                width,
            )
        });
        // the card shows the usage of its thread: the feature asks for it for as long as it is drawn
        Interest::new(UsageInterest::Card(number), sheet).into()
    }
}

/// The words and the styles one build reads.
#[derive(Clone)]
pub(super) struct Look {
    pub(super) s: &'static Strings,
    /// Oracle `.dim()`.
    pub(super) faint: Style,
    /// The oracle's accent colour and the magenta of the typed command.
    accent: Style,
    bold: Style,
}

/// What a row's value is: spans of text, or a usage window.
pub(super) enum Value {
    Spans(Vec<Span>),
    Window {
        percent_left: i32,
        reset: Option<String>,
        details: Option<String>,
    },
}

pub(super) enum Entry {
    Row {
        label: String,
        value: Value,
    },
    Blank,
    /// A label the column is made wide enough for, with no row of its own (the oracle's
    /// `push_label` of what a figure will show).
    Reserve(String),
}

fn blank() -> AnyWidget {
    SizedBox::new(None, Some(1), None).into()
}

fn indented(indent: i32, child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::only(indent, 0, 0, 0), child).into()
}

fn sheet(
    card: &StatusCard,
    held: Option<Option<&Figure>>,
    look: &Look,
    seg: Measure,
    width: i32,
) -> AnyWidget {
    let s = look.s;
    let entries = entries(card, held, look);
    let (indent, room) = tracks(&entries, seg, width);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(vec![Span::styled(
            s.command,
            look.accent.clone(),
        )]))
        .child(blank())
        .child(indented(
            indent,
            Text::rich(vec![
                Span::styled(s.title_mark, look.accent.clone()),
                Span::styled(s.title_name, look.bold.clone()),
                Span::styled(
                    s.title_version(&card.version).to_string(),
                    look.faint.clone(),
                ),
            ]),
        ))
        .child(blank());
    if card.usage_link {
        let accent = look.accent.clone();
        column = column
            .child(indented(
                indent,
                Text::rich(vec![
                    Span::styled(s.usage_visit, accent.clone()),
                    Span::styled(s.usage_url, accent.clone().attrs(Attrs::UNDERLINE))
                        .link(s.usage_url),
                    Span::styled(s.usage_for, accent.clone()),
                ]),
            ))
            .child(indented(
                indent,
                Text::rich(vec![Span::styled(s.usage_info, accent)]),
            ))
            .child(blank());
    }
    column
        .child(if room <= 0 {
            flowing(entries, look, indent)
        } else {
            labelled(entries, look, indent, seg, room)
        })
        .into()
}

fn tracks(entries: &[Entry], seg: Measure, width: i32) -> (i32, i32) {
    // Room for the two-cell title mark even in a tiny terminal.
    let indent = if width >= INDENT + 2 { INDENT } else { 0 };
    let label_width = entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Row { label, .. } | Entry::Reserve(label) => {
                Some(i32::try_from(seg.width(label)).unwrap_or(i32::MAX))
            }
            Entry::Blank => None,
        })
        .max()
        .unwrap_or(0);
    // No room left for a value beside its label: each row flows as one
    // paragraph under the indent.
    let room = width - indent - label_width - GAP;
    (indent, room)
}

impl StatusCell {
    /// The status card's plain contents before terminal wrapping, for the copy picker.
    pub(crate) fn export_text<S: 'static>(card: &StatusCard, cx: &mut Cx<'_, S>) -> String {
        let s = strings::of(cx);
        let look = Look {
            s,
            faint: Style::default(),
            accent: Style::default(),
            bold: Style::default(),
        };
        let figure = read::<FeatureStates, _>(cx)
            .thread_usage
            .card_figure(card.number);
        let seg = segmenter_of(cx);
        let mut lines = vec![
            TextLine::Heading(format!(
                "{}{}{}",
                s.title_mark,
                s.title_name,
                s.title_version(&card.version)
            )),
            TextLine::Blank,
        ];
        if card.usage_link {
            lines.extend([
                TextLine::Paragraph(format!("{}{}{}", s.usage_visit, s.usage_url, s.usage_for)),
                TextLine::Paragraph(s.usage_info.to_owned()),
                TextLine::Blank,
            ]);
        }
        lines.extend(
            entries(card, figure.as_ref().map(Option::as_ref), &look)
                .into_iter()
                .map(|entry| match entry {
                    Entry::Blank => TextLine::Blank,
                    Entry::Reserve(label) => TextLine::Reserve(label),
                    Entry::Row { label, value } => {
                        let values = match value {
                            Value::Spans(spans) => vec![span_text(&spans)],
                            Value::Window {
                                percent_left,
                                reset,
                                details,
                            } => {
                                let (with_bar, spans) = window_spans(
                                    percent_left,
                                    reset.as_deref(),
                                    details.as_deref(),
                                    &look,
                                    seg,
                                    i32::MAX,
                                );
                                let mut values: Vec<_> =
                                    spans.iter().map(|spans| span_text(spans)).collect();
                                if with_bar && let Some(first) = values.first_mut() {
                                    first.insert_str(0, &bar::text(i64::from(percent_left)));
                                }
                                values
                            }
                        };
                        TextLine::Row { label, values }
                    }
                }),
        );
        text_table(lines, INDENT as usize, GAP as usize, |label| {
            seg.width(label)
        })
    }

    /// The status card's text nodes at the mounted width. The usage bar paints
    /// cells directly and contributes no selectable text.
    pub(crate) fn texts(
        card: &StatusCard,
        held: Option<Option<&Figure>>,
        strings: &'static Strings,
        room: Room,
    ) -> Vec<Part> {
        let look = Look {
            s: strings,
            faint: Style::default(),
            accent: Style::default(),
            bold: Style::default(),
        };
        let entries = entries(card, held, &look);
        let (_, value_room) = tracks(&entries, room.segmenter, room.width);
        let mut parts = vec![
            Part::text(strings.command),
            Part::text(format!(
                "{}{}{}",
                strings.title_mark,
                strings.title_name,
                strings.title_version(&card.version)
            )),
        ];
        if card.usage_link {
            parts.push(Part::text(format!(
                "{}{}{}",
                strings.usage_visit, strings.usage_url, strings.usage_for
            )));
            parts.push(Part::text(strings.usage_info));
        }
        for entry in entries {
            let Entry::Row { label, value } = entry else {
                continue;
            };
            if value_room <= 0 || overflows(&value, room.segmenter, value_room) {
                parts.push(Part::text(span_text(&paragraph_spans(label, value, &look))));
                continue;
            }
            parts.push(Part::text(label));
            match value {
                Value::Spans(spans) => parts.push(Part::text(span_text(&spans))),
                Value::Window {
                    percent_left,
                    reset,
                    details,
                } => {
                    let (_, lines) = window_spans(
                        percent_left,
                        reset.as_deref(),
                        details.as_deref(),
                        &look,
                        room.segmenter,
                        value_room,
                    );
                    parts.extend(lines.into_iter().map(|line| Part::text(span_text(&line))));
                }
            }
        }
        parts
    }
}

fn span_text(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_ref()).collect()
}

/// The rows as a `Grid`: labels, two cells, values of `room` cells. A value
/// with a glyph wider than its track flows under its label instead, over both
/// columns (oracle `content_lines`: a wrapped line wider than the terminal
/// is wrapped again without the label column).
fn labelled(entries: Vec<Entry>, look: &Look, indent: i32, seg: Measure, room: i32) -> AnyWidget {
    let mut grid = Grid::new()
        .columns([GridTrack::content(), GridTrack::fr(1)])
        .column_gap(GAP);
    for entry in entries {
        match entry {
            Entry::Reserve(_) => {}
            Entry::Blank => {
                grid = grid
                    .child(SizedBox::fixed(0, 1))
                    .child(SizedBox::fixed(0, 1));
            }
            Entry::Row { label, value } if overflows(&value, seg, room) => {
                grid = grid.child(GridPlacement::new(paragraph(label, value, look)).span(1, 2));
            }
            Entry::Row { label, value } => {
                grid = grid
                    .child(Text::rich(vec![Span::styled(label, look.faint.clone())]))
                    .child(value_cell(value, look, seg, room));
            }
        }
    }
    indented(indent, grid)
}

/// Whether `Text` lays the value out wider than `room` cells: a glyph of two
/// cells in a track of one.
fn overflows(value: &Value, seg: Measure, room: i32) -> bool {
    let Value::Spans(spans) = value else {
        return false;
    };
    let text: String = spans.iter().map(|span| span.text.as_ref()).collect();
    let room = usize::try_from(room).unwrap_or(0);
    measure::rows(seg, &text, room)
        .iter()
        .any(|row| seg.width(row) > room)
}

/// The rows as paragraphs (the label, a gap, the value), for a width with no
/// room for the value track.
fn flowing(entries: Vec<Entry>, look: &Look, indent: i32) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for entry in entries {
        match entry {
            Entry::Reserve(_) => {}
            Entry::Blank => column = column.child(blank()),
            Entry::Row { label, value } => column = column.child(paragraph(label, value, look)),
        }
    }
    indented(indent, column)
}

/// One row as a paragraph: the label, a gap, the value.
fn paragraph(label: String, value: Value, look: &Look) -> AnyWidget {
    Text::rich(paragraph_spans(label, value, look)).into()
}

fn paragraph_spans(label: String, value: Value, look: &Look) -> Vec<Span> {
    let mut spans = vec![
        Span::styled(label, look.faint.clone()),
        Span::new(look.s.gap),
    ];
    match value {
        Value::Spans(value) => spans.extend(value),
        Value::Window {
            percent_left,
            reset,
            details,
        } => {
            spans.push(Span::new(look.s.percent_left(percent_left).to_string()));
            for text in reset.into_iter().chain(details) {
                spans.push(Span::new(look.s.gap));
                spans.push(Span::styled(text, look.faint.clone()));
            }
        }
    }
    spans
}

fn value_cell(value: Value, look: &Look, seg: Measure, room: i32) -> AnyWidget {
    match value {
        Value::Spans(spans) => Text::rich(spans).into(),
        Value::Window {
            percent_left,
            reset,
            details,
        } => window(
            percent_left,
            reset.as_deref(),
            details.as_deref(),
            look,
            seg,
            room,
        ),
    }
}

/// A usage window: the bar and the share left on one line, the reset after
/// them when it fits and under them when it does not, then the details. On
/// a narrow value track the bar gives way to the percentage (oracle
/// `rate_limit_row_lines`).
fn window(
    percent_left: i32,
    reset: Option<&str>,
    details: Option<&str>,
    look: &Look,
    seg: Measure,
    room: i32,
) -> AnyWidget {
    let (with_bar, lines) = window_spans(percent_left, reset, details, look, seg, room);
    let mut lines = lines.into_iter();
    let first = Text::rich(lines.next().unwrap_or_default());
    let head: AnyWidget = if with_bar {
        Row::new()
            .child(bar(i64::from(percent_left)))
            .child(first)
            .into()
    } else {
        first.into()
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(head)
        .children(lines.map(Text::rich).map(AnyWidget::from))
        .into()
}

fn window_spans(
    percent_left: i32,
    reset: Option<&str>,
    details: Option<&str>,
    look: &Look,
    seg: Measure,
    room: i32,
) -> (bool, Vec<Vec<Span>>) {
    let width = |text: &str| i32::try_from(seg.width(text)).unwrap_or(i32::MAX);
    let summary = look.s.percent_left(percent_left).to_string();
    let with_bar = BAR_CELLS + 2 + 1 + width(&summary) <= room;
    let first = if with_bar {
        BAR_CELLS + 2 + 1 + width(&summary)
    } else {
        width(&summary)
    };
    let inline = reset.filter(|reset| first + 1 + width(reset) <= room);
    let mut line = vec![Span::new(if with_bar {
        [look.s.space, &summary].concat()
    } else {
        summary
    })];
    if let Some(reset) = inline {
        line.push(Span::styled(look.s.space, look.faint.clone()));
        line.push(Span::styled(reset.to_owned(), look.faint.clone()));
    }
    let mut lines = vec![line];
    if let Some(reset) = reset.filter(|_| inline.is_none()) {
        lines.push(vec![Span::styled(reset.to_owned(), look.faint.clone())]);
    }
    if let Some(details) = details {
        lines.push(vec![Span::styled(details.to_owned(), look.faint.clone())]);
    }
    (with_bar, lines)
}

fn plain(text: impl Into<String>) -> Value {
    Value::Spans(vec![Span::new(text.into())])
}

fn faint(look: &Look, text: impl Into<String>) -> Value {
    Value::Spans(vec![Span::styled(text.into(), look.faint.clone())])
}

/// The rows of the card in the oracle's order.
fn entries(card: &StatusCard, held: Option<Option<&Figure>>, look: &Look) -> Vec<Entry> {
    let s = look.s;
    let row = |label: &str, value: Value| Entry::Row {
        label: label.to_string(),
        value,
    };
    let mut entries = Vec::new();
    if let Some(remote) = &card.remote {
        let value = if remote.local_daemon {
            plain(s.server_local)
        } else {
            Value::Spans(vec![
                Span::new(remote.address.clone()),
                Span::styled(
                    s.parenthesised(&remote.version).to_string(),
                    look.faint.clone(),
                ),
            ])
        };
        entries.push(row(s.server, value));
        entries.push(Entry::Blank);
    }
    let mut model = vec![Span::new(card.model.clone())];
    let details: Vec<String> = card
        .model_details()
        .iter()
        .map(|detail| s.model_detail(detail))
        .collect();
    if !details.is_empty() {
        model.push(Span::styled(
            s.parenthesised(&details.join(s.details_separator))
                .to_string(),
            look.faint.clone(),
        ));
    }
    entries.push(row(s.model, Value::Spans(model)));
    if let Some(provider) = &card.provider {
        entries.push(row(s.model_provider, plain(provider.clone())));
    }
    entries.push(row(s.directory, plain(card.directory_text())));
    entries.push(row(
        s.permissions,
        plain(s.permissions_text(&card.permissions.label())),
    ));
    let agents = card.agents_summary();
    entries.push(row(
        s.agents,
        plain(if agents.is_empty() {
            s.agents_none.to_string()
        } else {
            agents.join(s.details_separator)
        }),
    ));
    if let Some(account) = &card.account {
        entries.push(row(s.account, plain(s.account_text(account))));
        if let Account::ChatGpt {
            workspace: Some(workspace),
            ..
        } = account
        {
            entries.push(row(s.workspace, plain(workspace.clone())));
        }
    }
    if let Some(name) = card.thread_name() {
        entries.push(row(s.thread_name, plain(name)));
    }
    if let Some(mode) = &card.collaboration_mode {
        entries.push(row(s.collaboration_mode, plain(mode.clone())));
    }
    if let Some(session) = &card.session_id {
        entries.push(row(s.session, plain(session.clone())));
    }
    if let Some(forked) = card.forked_from() {
        entries.push(row(s.forked_from, plain(forked)));
    }
    entries.push(Entry::Blank);
    if card.shows_token_usage() {
        let tokens = &card.tokens;
        entries.push(row(
            s.token_usage,
            Value::Spans(vec![
                Span::new(
                    s.tokens_head(&format_tokens_compact(tokens.blended_total()))
                        .to_string(),
                ),
                Span::styled(
                    s.tokens_detail(
                        &format_tokens_compact((tokens.input - tokens.cached_input).max(0)),
                        &format_tokens_compact(tokens.output),
                    )
                    .to_string(),
                    look.faint.clone(),
                ),
            ]),
        ));
    }
    if let Some(context) = &card.context {
        entries.push(row(
            s.context_window,
            Value::Spans(vec![
                Span::new(s.context_head(context.percent_left).to_string()),
                Span::styled(
                    s.context_detail(
                        &format_tokens_compact(context.used),
                        &format_tokens_compact(context.window),
                    )
                    .to_string(),
                    look.faint.clone(),
                ),
            ]),
        ));
    }
    let refreshing = card.limits.refreshing;
    match card.limit_data() {
        LimitData::Available(rows) => entries.extend(limit_rows(rows, card, look)),
        LimitData::Stale(rows) => {
            entries.extend(limit_rows(rows, card, look));
            entries.push(row(
                s.warning,
                faint(
                    look,
                    if refreshing {
                        s.limits_stale_refreshing
                    } else {
                        s.limits_stale
                    },
                ),
            ));
        }
        LimitData::Unavailable => entries.push(row(s.limits, faint(look, s.limits_unavailable))),
        LimitData::Missing => entries.push(row(
            s.limits,
            faint(
                look,
                if refreshing {
                    s.limits_missing_refreshing
                } else {
                    s.limits_missing
                },
            ),
        )),
    }
    entries.extend(usage::entries(card, held, look));
    entries
}

fn limit_rows(rows: Vec<LimitRow>, card: &StatusCard, look: &Look) -> Vec<Entry> {
    let s = look.s;
    rows.into_iter()
        .map(|row| Entry::Row {
            label: s.row_label(&row.label),
            value: match &row.value {
                RowValue::Window {
                    percent_used,
                    resets_at,
                    details,
                } => Value::Window {
                    percent_left: 100 - (*percent_used).clamp(0, 100),
                    reset: resets_at.map(|at| s.reset(&at, card.hour24)),
                    details: details
                        .as_ref()
                        .map(|spend| s.spend_used(&spend.used, &spend.limit)),
                },
                RowValue::Empty => Value::Spans(Vec::new()),
                RowValue::Credits(text) => plain(s.credits_text(text)),
            },
        })
        .collect()
}
