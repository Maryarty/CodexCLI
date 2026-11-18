//! `pager` (S16): the view half of the pager, the transcript pager (Ctrl+T)
//! and the static pager (`/diff`). It is one `PageRoute` over two kinds of
//! content: a static page when the model has one open, else the active
//! thread's transcript. Its model, events and intents are core's
//! `codex_tui_rtwk_core::screens::pager`. Oracle: `tui/src/pager_overlay.rs`
//! (`PagerView`, `StaticOverlay`) and `pager_overlay/transcript.rs`.

use std::rc::Rc;

use rtwk::foundation::{ValueListenableBuilder, ValueNotifier};
use rtwk::NotificationListener;
use rtwk::focus::Focus;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, SizedBox};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::watch;
use rtwk::scroll::{ListView, ScrollNotification, SingleChildScrollView};
use rtwk::style::Span;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::screens::ScreenStates;
use codex_tui_rtwk_core::screens::pager::HistoryState;
use codex_tui_rtwk_core::threads::{Item, ThreadModel};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme;
use crate::ui::seeded::Seeded;

pub mod strings;

macro_rules! entry {
    ($name:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $name, ".jsonl")),
            open: EntryOpen::Route(|_: &Seeded| Screen.into()),
        }
    };
}

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        entry!("static_basic", "the static pager over three lines"),
        entry!(
            "static_wraps_long_lines",
            "the static pager over one long line"
        ),
        entry!("static_long", "the static pager over thirty lines, to scroll"),
        entry!(
            "transcript_loading",
            "the transcript pager while older history loads"
        ),
        entry!(
            "transcript_partial",
            "the transcript pager with earlier messages available"
        ),
        entry!(
            "transcript_failed",
            "the transcript pager after the history failed"
        ),
        entry!(
            "transcript_complete",
            "the transcript pager over a complete history"
        ),
        entry!(
            "transcript_completed_stream",
            "the transcript pager over two completed cells"
        ),
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen.into())
}

/// Opens the pager on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// The pager screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: where the static page is scrolled to, as (offset, largest
/// offset), told by the scroll notifications (no controller, R14); `None`
/// until the first move, when the largest offset is estimated (an rtwk gap).
#[derive(Debug)]
pub struct ScreenState {
    scroll: Rc<ValueNotifier<Option<(i64, i64)>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            scroll: Rc::new(ValueNotifier::new(None)),
        }
    }
}

impl_widget!(stateful eq Screen);

/// The header: `/ ` repeated, with `/ <title>` over its start (oracle:
/// `PagerView::render_header`). Clipped to the row by the `Text`.
fn header(title: &str) -> String {
    let mut text = format!("/ {title}");
    let mut column = text.chars().count();
    while column < 400 {
        text.push(if column % 2 == 0 { '/' } else { ' ' });
        column += 1;
    }
    text
}

/// The rows of the screen that are not content: the header, the rule, the
/// hints and the empty row under them.
const CHROME_ROWS: i32 = 5;
/// The rows of the hints and the empty row under them.
const BOTTOM_ROWS: i32 = 3;

/// The largest scroll offset of `lines` wrapped at `width` words, in a
/// window of `rows`: what the page will report once it moves.
fn estimate_max(lines: &[String], width: i32, rows: i32) -> i64 {
    let width = width.max(1) as usize;
    let mut total = 0i64;
    for line in lines {
        let mut used = 0usize;
        let mut count = 1i64;
        for word in line.split(' ') {
            let w = word.chars().count();
            if used > 0 && used + 1 + w > width {
                count += 1;
                used = 0;
            }
            used += if used == 0 { 0 } else { 1 };
            let mut w = w;
            while used + w > width {
                count += 1;
                w -= (width - used).max(1).min(w);
                used = 0;
            }
            used += w;
        }
        total += count;
    }
    (total - i64::from(rows)).max(0)
}

/// The percentage of the bottom bar (oracle: `PagerView::render_bottom_bar`).
fn percent(offset: i64, max: i64) -> i64 {
    if max <= 0 {
        100
    } else {
        ((offset.min(max) as f64 / max as f64) * 100.0).round() as i64
    }
}

/// The rule with ` N% ` near its right end.
fn bar(width: i32, percent: i64) -> String {
    let label = format!(" {percent}% ");
    let label_width = label.chars().count() as i32;
    let width = width.max(0);
    if label_width + 1 > width {
        return "─".repeat(width as usize);
    }
    let left = width - label_width - 1;
    format!("{}{label}─", "─".repeat(left as usize))
}

fn one_line(text: String) -> Text {
    Text::new(text).wrap(TextWrap::None).max_lines(1)
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Close => {
                Navigator::maybe_pop(cx);
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let shared = shared::of(cx);
        let screens = watch::<ScreenStates, _>(cx);
        let thread = watch::<ThreadModel, _>(cx);
        let dim = tokens.dim();
        let pager = &screens.pager;

        let (title, content, static_page): (String, AnyWidget, bool) = match &pager.page {
            Some(page) => (
                page.title.clone(),
                {
                    let scroll = self.scroll.clone();
                    NotificationListener::<ScrollNotification>::new(
                        SingleChildScrollView::new(
                            Text::new(page.lines.join("\n")).wrap(TextWrap::Word),
                        )
                        .autofocus(true),
                    )
                    .on_notification(move |n| {
                        let m = &n.metrics;
                        scroll.set_value(Some((m.pixels, m.max_scroll_extent)));
                        false
                    })
                    .into()
                },
                true,
            ),
            None => {
                let rows = thread;
                let user = shared;
                let list = ListView::builder(rows.transcript.len(), move |index| {
                    match rows.transcript.get(index) {
                        Some((_, Item::User { text })) => {
                            Text::new(user.user_line(text).to_string()).into()
                        }
                        Some((_, Item::Agent { text })) => Text::new(text.trim_end().to_string()).into(),
                        None => AnyWidget::default(),
                    }
                })
                .autofocus(true);
                (strings.transcript_title.to_string(), list.into(), false)
            }
        };

        // The row under the content: the rule and percentage of the static
        // pager, the history status of the transcript pager.
        let lines: Vec<String> = pager.page.as_ref().map(|p| p.lines.clone()).unwrap_or_default();
        let scroll = self.scroll.clone();
        let rule_style = dim.clone();
        let status_for = move |width: i32, height: i32| -> AnyWidget {
            let rule_style = rule_style.clone();
            let lines = lines.clone();
            ValueListenableBuilder::new(scroll.clone(), move |_, at, _| {
                let (offset, max) =
                    at.unwrap_or_else(|| (0, estimate_max(&lines, width, height - CHROME_ROWS)));
                one_line(bar(width, percent(offset, max)))
                    .style(rule_style.clone())
                    .into()
            })
            .into()
        };
        let status: Option<AnyWidget> = if static_page {
            None
        } else {
            let history = pager.history;
            let mut spans = Vec::new();
            match history {
                HistoryState::LoadingOlder | HistoryState::LoadingBeginning => {
                    spans.push(Span::styled(strings.loading_marker, tokens.accent()));
                    spans.push(Span::new(strings.loading));
                }
                HistoryState::Failed => spans.push(Span::styled(strings.retry, dim.clone())),
                HistoryState::Partial => {
                    spans.push(Span::styled(strings.earlier, dim.clone()));
                    spans.push(Span::styled(strings.select, dim.clone()));
                }
                HistoryState::Complete => {
                    spans.push(Span::styled(strings.select, dim.clone()));
                }
            }
            Some(Text::rich(spans).wrap(TextWrap::None).max_lines(1).into())
        };

        // The two hint rows and the empty row under them (the oracle's
        // `HINTS_HEIGHT` of three); the transcript pager has none while its
        // history loads or failed.
        let hints = if static_page || !pager.history_blocks_hints() {
            let close = if static_page {
                strings.close_static
            } else {
                strings.close_transcript
            };
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(one_line(strings.navigation.to_string()).style(dim.clone()))
                .child(one_line(close.to_string()).style(dim.clone()))
        } else {
            Column::new().cross_axis_alignment(CrossAxisAlignment::Start)
        };

        let head = one_line(header(&title)).style(dim);
        let hints: AnyWidget = SizedBox::new(None, Some(BOTTOM_ROWS), Some(hints.into())).into();
        Focus::new(LayoutBuilder::new(move |_, constraints| {
            let status = status
                .clone()
                .unwrap_or_else(|| status_for(constraints.max_width, constraints.max_height));
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(head.clone())
                .child(Expanded::new(1, content.clone()))
                .child(status)
                .child(hints.clone())
                .into()
        }))
        .bind('q', cx.send(Msg::Close))
        .into()
    }
}
