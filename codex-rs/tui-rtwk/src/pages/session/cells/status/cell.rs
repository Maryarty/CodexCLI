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
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::format::format_tokens_compact;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::screens::status::{
    Account, Figure, LimitData, LimitRow, RowValue, StatusCard, UsageInterest,
};

use super::bar::{BAR_CELLS, bar};
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
            sheet(&card, held.as_ref().map(Option::as_ref), &look, segmenter_of(cx), width)
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
    Row { label: String, value: Value },
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
    // Room for the two-cell title mark even in a tiny terminal.
    let indent = if width >= INDENT + 2 { INDENT } else { 0 };
    let entries = entries(card, held, look);
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
    let flows = room <= 0;
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
        .child(if flows {
            flowing(entries, look, indent)
        } else {
            labelled(entries, look, indent, seg, room)
        })
        .into()
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
    Text::rich(spans).into()
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
    let head: AnyWidget = if with_bar {
        Row::new()
            .child(bar(i64::from(percent_left)))
            .child(Text::rich(line))
            .into()
    } else {
        Text::rich(line).into()
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(head);
    if let Some(reset) = reset.filter(|_| inline.is_none()) {
        column = column.child(Text::rich(vec![Span::styled(
            reset.to_owned(),
            look.faint.clone(),
        )]));
    }
    if let Some(details) = details {
        column = column.child(Text::rich(vec![Span::styled(
            details.to_owned(),
            look.faint.clone(),
        )]));
    }
    column.into()
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
