//! The dashboard (`z`, oracle: `analytics/dashboard.rs`, `render.rs`): the
//! reports as bordered cards, two to a row from 104 cells, under one scroll.
//! How many cards sit in a row and how tall each is for the room is core's
//! (`dashboard::Geometry`); a card is a `BoxWidget` whose top edge carries
//! the title in two styles, over a content area that a report that is
//! taller than it is cut off at, its last row the words that say so. The
//! rows the content takes are what each part of it says (`Item`), the
//! rows of a wrapped text what `measure::rows` counts (G38). Moving to
//! another card scrolls the list to its row (`Reveal`).

use rtwk::gestures::GestureDetector;
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    Alignment, Border, BorderType, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize,
    OverflowBox, Positioned, Row, SizedBox, Stack, StackFit,
};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::usage::chats::Pos as ChatPos;
use codex_tui_core::screens::usage::dashboard::{CARD_GAP, Geometry};
use codex_tui_core::screens::usage::plan::Pos as PlanPos;
use codex_tui_core::screens::usage::{Chosen, Section};

use super::strings::Strings;
use super::{Look, SECTIONS, Taps, chats, gap, line, overview, plan, report};
use crate::widgets::measure::{self, Measure};

/// A piece of a card's content and the rows it takes.
#[derive(Clone)]
pub struct Item {
    pub widget: AnyWidget,
    pub rows: usize,
}

impl Item {
    pub fn new(widget: impl Into<AnyWidget>, rows: usize) -> Self {
        Self {
            widget: widget.into(),
            rows,
        }
    }

    /// A widget of one row.
    pub fn one(widget: impl Into<AnyWidget>) -> Self {
        Self::new(widget, 1)
    }
}

/// The room a card's content is laid out in: what wraps a text at the
/// card's width and counts the rows it takes, and the rows of the plot.
#[derive(Clone, Copy)]
pub struct Fit {
    pub seg: Measure,
    pub width: usize,
    pub plot_rows: usize,
}

impl Fit {
    /// The rows `text` takes wrapped by word (at least one).
    pub fn rows(&self, text: &str) -> usize {
        measure::rows(self.seg, text, self.width).len().max(1)
    }
}

/// What the dashboard is drawn from (borrowed from the page for one build).
#[derive(Clone)]
pub struct Deck {
    pub model: Chosen,
    pub strings: Strings,
    pub num: NumSyms,
    pub look: Look,
    /// The selected day of each report.
    pub cursors: [usize; SECTIONS],
    pub list: ListController,
    /// The Plan report's rows (core's), shown on its card.
    pub plan: PlanPos,
    /// The chats report's rows (core's), shown on its card.
    pub chat: ChatPos,
    pub taps: Taps,
}

/// The dashboard in the room the body gets.
pub fn dashboard(deck: Deck) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let sections = deck.model.model.visible_sections();
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let height = usize::try_from(constraints.max_height).unwrap_or(0);
        let geometry = Geometry::new(width, height, sections.len());
        let fit = Fit {
            seg: segmenter_of(cx),
            width: geometry.inner_width(),
            plot_rows: geometry.plot_rows(),
        };
        let selected = sections
            .iter()
            .position(|section| *section == deck.model.sel.section)
            .unwrap_or(0);
        // A card row, a blank row, a card row: the list's items.
        let mut items: Vec<AnyWidget> = Vec::new();
        for row in 0..geometry.rows {
            if row > 0 {
                items.push(gap());
            }
            let mut cards = Row::new().cross_axis_alignment(CrossAxisAlignment::Start);
            for (at, card) in geometry.cards_of(row).enumerate() {
                if at > 0 {
                    cards = cards.child(SizedBox::fixed(CARD_GAP as i32, 1));
                }
                cards = cards.child(SizedBox::new(
                    Some(geometry.card_width as i32),
                    Some(geometry.card_height as i32),
                    Some(deck.card(sections[card], card, selected == card, &geometry, fit)),
                ));
            }
            items.push(cards.into());
        }
        Reveal {
            child: ListView::new(items).controller(deck.list.clone()).into(),
            list: deck.list.clone(),
            index: 2 * geometry.row_of(selected),
        }
        .into()
    })
    .into()
}

impl Deck {
    /// The card of `section`, the `card`-th, a tap on it selects it and two
    /// open its report.
    fn card(
        &self,
        section: Section,
        card: usize,
        focused: bool,
        geometry: &Geometry,
        fit: Fit,
    ) -> AnyWidget {
        let Deck {
            model,
            strings,
            num,
            look,
            ..
        } = self;
        let border = if focused {
            look.accent.clone()
        } else {
            look.dim.clone()
        };
        let content = self.content(section, geometry, fit);
        let name = super::tab_name(strings, &model.model, section);
        let label = strings.card_title(card as u64 + 1, name, num);
        let title = vec![
            Span::styled(strings.blank, Style::new()),
            if focused {
                Span::styled(strings.card_mark, look.accent.clone())
            } else {
                Span::styled(strings.blank, Style::new())
            },
            Span::styled(
                label,
                if focused {
                    look.bold.clone()
                } else {
                    look.secondary.clone()
                },
            ),
        ];
        let frame = BoxWidget::new()
            .border(Border::all(border).kind(BorderType::Rounded))
            .padding(EdgeInsets::all(1))
            .child(content);
        // The title sits on the top edge from the third cell to the one
        // before the corner, cut off with an ellipsis (the box's own title
        // takes one style).
        let titled = Stack::new().fit(StackFit::Expand).child(frame).child(
            Positioned::new(
                Text::rich(title)
                    .wrap(TextWrap::None)
                    .overflow(TextOverflow::Ellipsis)
                    .max_lines(1),
            )
            .left(2)
            .top(0)
            .width((geometry.card_width as i32 - 3).max(0))
            .height(1),
        );
        self.tapped(titled.into(), card)
    }

    /// A tap on the card selects it; two open its report.
    fn tapped(&self, card: AnyWidget, index: usize) -> AnyWidget {
        let (Some(select), Some(open)) = (self.taps.tabs.get(index), self.taps.open.get(index))
        else {
            return card;
        };
        GestureDetector::new()
            .on_tap(select.clone())
            .on_double_tap(open.clone())
            .child(card)
            .into()
    }

    /// What a card of `section` says, cut at the card's rows.
    fn content(&self, section: Section, geometry: &Geometry, fit: Fit) -> AnyWidget {
        let Deck {
            model,
            strings,
            num,
            look,
            cursors,
            ..
        } = self;
        let model = &model.view();
        let lead = model.card_lead(section);
        let mut items: Vec<Item> = Vec::new();
        let mut say = |shown: String| {
            items.push(report::words(shown, look.secondary.clone(), Some(fit)));
        };
        if let Some(long) = lead.range {
            let filter = lead
                .model
                .as_ref()
                .map(|name| name.as_deref().unwrap_or(strings.all_models));
            say(strings.card_range(long, filter, num));
        }
        if let Some(group) = lead.group {
            say(strings.card_group(model.group_name(section, group), num));
        }
        let shown = report::Shown {
            cursor: cursors[section as usize],
            detail: None,
            zoomed: false,
        };
        items.extend(match section {
            Section::Summary => overview::card(model, strings, num, look, fit),
            Section::Plan => plan::card(model, &self.plan, strings, num, look, fit),
            Section::Chats => chats::card(model, &self.chat, strings, num, look, fit),
            section => report::card(model, section, strings, num, look, shown, fit),
        });
        let inner = geometry.inner_height();
        let rows: usize = items.iter().map(|item| item.rows).sum();
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children(items.into_iter().map(|item| item.widget));
        if rows <= inner {
            return column.into();
        }
        // Cut where the card ends: its last row says so.
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(
                None,
                Some(inner as i32 - 1),
                Some(
                    OverflowBox::new(column)
                        .alignment(Alignment::TOP_LEFT)
                        .min_height(0)
                        .max_height(rows as i32)
                        .into(),
                ),
            ))
            .child(line(strings.card_maximize, look.secondary.clone()))
            .into()
    }
}

/// Keeps one item of a list in view: scrolls to it when it is first shown
/// and whenever the item changes (Flutter: `Scrollable.ensureVisible` in
/// `didUpdateWidget`). The wheel and the page keys move the list freely in
/// between.
#[derive(Clone)]
struct Reveal {
    child: AnyWidget,
    list: ListController,
    index: usize,
}

impl PartialEq for Reveal {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.list == other.list && self.child == other.child
    }
}

struct RevealState;

impl StatefulWidget for Reveal {
    type State = RevealState;

    fn create_state(&self) -> RevealState {
        RevealState
    }
}

impl State for RevealState {
    type Widget = Reveal;
    type Msg = ();

    fn init_state(&mut self, w: &Reveal, _: &mut Cx<'_, Self>) {
        w.list.ensure_index_visible(w.index);
    }

    fn did_update_widget(&mut self, old: &Reveal, w: &Reveal, _: &mut Cx<'_, Self>) {
        if old.index != w.index {
            w.list.ensure_index_visible(w.index);
        }
    }

    fn update(&mut self, (): (), _: &Reveal, _: &mut Cx<'_, Self>) -> Rebuild {
        Rebuild::No
    }

    fn build(&self, w: &Reveal, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.child.clone()
    }
}

impl_widget!(stateful eq Reveal);
