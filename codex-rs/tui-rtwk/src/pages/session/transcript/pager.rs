//! The static pager (oracle `pager_overlay.rs`: `PagerView`, `StaticOverlay`):
//! a titled, full-screen read of a program's text (`/diff`, the full-screen
//! approvals, provider details), scrolled by the pager's keys.
//!
//! Flutter's full-screen route of a `Column`: the title row, the scrolling
//! text with `~` rows where it ends, the rule with the percent scrolled
//! (`ListenableBuilder` over the `ScrollController`), the hint rows. The text
//! is the ANSI text of a program (`parse_ansi`), one `Text.rich` per line. The document is the
//! domain's directive (`transcript::Directive<PagerDoc>`), handed to the widget by whoever shows
//! it (the presenter, a debug entry); whether the pager is open is the route's or the dialog's.

use std::rc::Rc;
use std::time::Duration;

use rtwk::ansi::parse_ansi;
use rtwk::focus::{Focus, KeyPress};
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{
    BorderType, BoxWidget, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize, Padding,
    Row, SizedBox, Stack,
};
use rtwk::navigation::Navigator;
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    PAGER_CLOSE, PAGER_HALF_PAGE_DOWN, PAGER_HALF_PAGE_UP, PAGER_JUMP_BOTTOM, PAGER_JUMP_TOP,
    PAGER_PAGE_DOWN, PAGER_PAGE_UP, PAGER_SCROLL_DOWN, PAGER_SCROLL_UP,
};
use codex_tui_core::keymap::{Keymap, fixed};
use codex_tui_core::screens::transcript::{PagerAction, PagerDoc};

use super::TranscriptMode;
use super::strings::{self, Strings};
use crate::app::debug::{DebugEntry, EntryOpen};

/// The static pager over `doc`: a dialog the size of the screen (`/diff`: the presenter shows
/// the directive's document over the chat) or a page of a route (a debug entry).
#[derive(Clone, Debug, PartialEq)]
pub struct StaticPager {
    doc: PagerDoc,
    host: Host,
    /// Said when the document's action is taken (Enter on `EditConfiguration`): the opener's
    /// verb, then the pager closes.
    on_action: Option<Handler<()>>,
}

/// What the pager is shown in, so what closing it pops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Host {
    Dialog,
    Route,
}

impl StaticPager {
    /// The pager over `doc`, in a dialog: it covers the chat, `q` closes it.
    pub fn over(doc: PagerDoc) -> StaticPager {
        StaticPager {
            doc,
            host: Host::Dialog,
            on_action: None,
        }
    }

    /// The pager tells `handler` when its action is taken (provider details: `Edit
    /// configuration`), before it closes.
    pub fn on_action(mut self, handler: Handler<()>) -> StaticPager {
        self.on_action = Some(handler);
        self
    }

    /// The pager over `doc`, a page pushed on a navigator: `q` pops it.
    pub fn page(doc: PagerDoc) -> StaticPager {
        StaticPager {
            doc,
            host: Host::Route,
            on_action: None,
        }
    }
}

/// The pager the size of the screen: the alternate screen is cleared (`enter_alt_screen`), so
/// nothing of the chat shows through the cells the pager leaves blank.
pub fn covering(doc: PagerDoc) -> AnyWidget {
    covering_with(StaticPager::over(doc))
}

/// `covering` over a pager made by the caller (one with an `on_action`).
pub fn covering_with(pager: StaticPager) -> AnyWidget {
    BoxWidget::new().style(Style::new()).child(pager).into()
}

/// The dialog options of `covering`: no barrier to click away, no fade.
pub fn cover_options() -> DialogOptions {
    DialogOptions {
        barrier_dismissible: false,
        barrier_alpha: Some(0),
        fade_in: Duration::ZERO,
        ..DialogOptions::default()
    }
}

/// Widget state: the text's scroll controller only (R14).
pub struct PagerState {
    scroll: ScrollController,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Rows(i64),
    Pages(i64),
    HalfPages(i64),
    Top,
    Bottom,
    Close,
    Action,
}

impl StatefulWidget for StaticPager {
    type State = PagerState;

    fn create_state(&self) -> PagerState {
        PagerState {
            scroll: ScrollController::new(),
        }
    }
}

impl_widget!(stateful eq StaticPager);

impl PagerState {
    fn scroll_by(&self, rows: i64) {
        if let Ok(position) = self.scroll.try_position() {
            self.scroll.jump_to(position.pixels().saturating_add(rows));
        }
    }

    /// One page of the text's rows (`PagerView::page_height`).
    fn page(&self) -> i64 {
        self.scroll
            .try_position()
            .map_or(1, |position| position.viewport_dimension())
    }
}

impl State for PagerState {
    type Widget = StaticPager;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &StaticPager, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Rows(rows) => self.scroll_by(rows),
            Msg::Pages(pages) => self.scroll_by(pages.saturating_mul(self.page())),
            Msg::HalfPages(pages) => {
                self.scroll_by(pages.saturating_mul(self.page().saturating_add(1) / 2));
            }
            Msg::Top => self.scroll.jump_to(0),
            Msg::Bottom => self.scroll.jump_to(i64::MAX),
            // the action's effect is the opener's, a later verb of its feature; closing is
            // the pager's, popping what it was shown in
            Msg::Close | Msg::Action => {
                if let (Msg::Action, Some(handler)) = (&msg, &w.on_action) {
                    cx.emit(handler, ());
                }
                match w.host {
                    Host::Dialog => close_dialog::<_, ()>(cx, None),
                    Host::Route => {
                        Navigator::maybe_pop(cx);
                    }
                }
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &StaticPager, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let table = strings::of(cx);
        let dim = crate::app::theme::Tokens::of(cx).dim();
        let doc = w.doc.clone();
        let action = doc.action;
        let column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(header(&doc.title, table, dim.clone()))
            .child(Expanded::new(
                1,
                body(&doc.text, self.scroll.clone(), table),
            ))
            .child(rule(self.scroll.clone(), table, dim.clone()))
            .child(hints(action, table, dim));
        let mut focus = Focus::new(column).autofocus();
        let keymap = crate::keymap::keymap(cx);
        for (key, msg) in bindings(&keymap, action.is_some()) {
            focus = focus.bind(key, cx.send(msg));
        }
        focus.into()
    }
}

/// The pager's keys (`PagerKeymap`: the keymap's `pager` actions, the oracle's defaults unless
/// rebound), then the action's Enter. A key two actions name is the first one's.
pub(in crate::pages::session) fn bindings(
    keymap: &Keymap,
    has_action: bool,
) -> Vec<(KeyPress, Msg)> {
    let mut keys: Vec<(KeyPress, Msg)> = Vec::new();
    let mut add = |id, msg: Msg| {
        for key in crate::keymap::presses(keymap, id) {
            if keys.iter().all(|(taken, _)| *taken != key) {
                keys.push((key, msg.clone()));
            }
        }
    };
    add(PAGER_SCROLL_UP, Msg::Rows(-1));
    add(PAGER_SCROLL_DOWN, Msg::Rows(1));
    add(PAGER_PAGE_UP, Msg::Pages(-1));
    add(PAGER_PAGE_DOWN, Msg::Pages(1));
    add(PAGER_HALF_PAGE_UP, Msg::HalfPages(-1));
    add(PAGER_HALF_PAGE_DOWN, Msg::HalfPages(1));
    add(PAGER_JUMP_TOP, Msg::Top);
    add(PAGER_JUMP_BOTTOM, Msg::Bottom);
    add(PAGER_CLOSE, Msg::Close);
    if has_action {
        add(fixed::ENTER, Msg::Action);
    }
    keys
}

/// `/ ` over the row, the title over its start (`render_header`): the run
/// of the table clipped by the row, the title in a `Stack` over it.
pub(super) fn header(title: &str, table: &'static Strings, style: Style) -> AnyWidget {
    let title = format!("{}{title}", table.pager_title_lead);
    SizedBox::new(
        None,
        Some(1),
        Some(
            Stack::new()
                .child(
                    Text::line(table.pager_slashes)
                        .style(style.clone())
                        .overflow(TextOverflow::Clip),
                )
                .child(Text::line(title).style(style).overflow(TextOverflow::Clip))
                .into(),
        ),
    )
    .into()
}

/// The text, wrapped, scrolled; `~` rows below where it ends.
fn body(text: &str, scroll: ScrollController, table: &'static Strings) -> AnyWidget {
    let lines: Vec<AnyWidget> = parse_ansi(text)
        .into_iter()
        .map(|spans| Text::rich(spans).into())
        .collect();
    let lines = Rc::new(lines);
    LayoutBuilder::new(move |_, constraints| {
        let filler = Column::new().children(
            (0..constraints.max_height.max(0)).map(|_| Text::line(table.pager_filler).into()),
        );
        // The text fills its own rows with blanks over the filler (`Clear`
        // then the filler from where the text ends).
        let text = rtwk::layout::BoxWidget::new().style(Style::new()).child(
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(lines.iter().cloned()),
        );
        Stack::new()
            .child(filler)
            .child(SingleChildScrollView::new(text).controller(scroll.clone()))
            .into()
    })
    .into()
}

/// `─` over the row with ` N% ` one cell from its end (`render_bottom_bar`):
/// a `Divider` taking the room, the label, the last `─`.
fn rule(scroll: ScrollController, table: &'static Strings, style: Style) -> AnyWidget {
    ListenableBuilder::new(Rc::new(scroll.clone()), move |_, _| {
        let percent = match scroll.try_position() {
            Ok(position) if position.max_scroll_extent() > 0 => {
                let ratio = position.pixels().min(position.max_scroll_extent()) as f64
                    / position.max_scroll_extent() as f64;
                (ratio * 100.0).round() as u8
            }
            _ => 100,
        };
        let [before, after] = table.pager_percent;
        SizedBox::new(
            None,
            Some(1),
            Some(
                Row::new()
                    .child(Expanded::new(
                        1,
                        Divider::new()
                            .kind(BorderType::Single)
                            .style(style.clone())
                            .merge(false),
                    ))
                    .child(Text::line(format!("{before}{percent}{after}")).style(style.clone()))
                    .child(Text::line(table.pager_rule).style(style.clone()))
                    .into(),
            ),
        )
        .into()
    })
    .into()
}

/// The keys of a pair and what they do (`render_key_hints`).
pub(super) type Pair = (Vec<&'static str>, &'static str);

/// The navigation pairs (`render_navigation_hints`): scroll, page, jump.
pub(super) fn navigation(table: &'static Strings) -> Vec<Pair> {
    vec![
        (vec![table.key_up, table.key_down], table.hint_scroll),
        (
            vec![table.key_page_up, table.key_page_down],
            table.hint_page,
        ),
        (vec![table.key_home, table.key_end], table.hint_jump),
    ]
}

/// One row of key hints: a blank, then the pairs joined by `hint_joiner`
/// (`render_key_hints`).
pub(super) fn hint_row(pairs: &[Pair], table: &'static Strings, style: Style) -> AnyWidget {
    let mut spans = vec![Span::new(table.hint_gap)];
    for (i, (keys, what)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::new(table.hint_joiner));
        }
        for (k, key) in keys.iter().enumerate() {
            if k > 0 {
                spans.push(Span::styled(table.hint_slash, style.clone()));
            }
            spans.push(Span::styled(*key, style.clone().attrs(Attrs::BOLD)));
        }
        spans.push(Span::new(table.hint_gap));
        spans.push(Span::new(*what));
    }
    Text::rich_line(spans)
        .style(style)
        .overflow(TextOverflow::Clip)
        .into()
}

/// The two hint rows and a blank one (`HINTS_HEIGHT` 3).
fn hints(action: Option<PagerAction>, table: &'static Strings, style: Style) -> AnyWidget {
    let mut second = vec![(vec![table.key_close], table.hint_close)];
    if let Some(PagerAction::EditConfiguration) = action {
        second.push((vec![table.key_enter], table.action_edit_configuration));
    }
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(hint_row(&navigation(table), table, style.clone()))
        .child(hint_row(&second, table, style))
        .child(Padding::new(
            EdgeInsets::ZERO,
            SizedBox::new(None, Some(1), None),
        ))
        .into()
}

/// The pager as a mode of the transcript (`mode_list!`).
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "pager";

    fn debug_entries() -> Vec<DebugEntry> {
        [
            (
                "pager_basic",
                "the static pager, three short lines",
                include_str!("fixtures/pager_basic.jsonl"),
            ),
            (
                "pager_wraps_long_lines",
                "the static pager, one long line wrapped",
                include_str!("fixtures/pager_wraps_long_lines.jsonl"),
            ),
            (
                "pager_long",
                "the static pager, forty numbered lines",
                include_str!("fixtures/pager_long.jsonl"),
            ),
            (
                "pager_colors",
                "the static pager, the colours of a diff and a blank line",
                include_str!("fixtures/pager_colors.jsonl"),
            ),
            (
                "pager_action",
                "the static pager with an action on Enter",
                include_str!("fixtures/pager_action.jsonl"),
            ),
        ]
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Route(|seeded| {
                // the document the script's event queued: nobody presents it in a debug page
                let doc = seeded
                    .core
                    .screens()
                    .transcript
                    .pending()
                    .last()
                    .map(|directive| directive.value.clone())
                    .unwrap_or(PagerDoc {
                        title: String::new(),
                        text: String::new(),
                        action: None,
                    });
                StaticPager::page(doc).into()
            }),
        })
        .collect()
    }
}
