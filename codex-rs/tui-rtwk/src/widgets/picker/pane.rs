//! The picker's pane: the pieces in their order, each as high as the rows of
//! the pane let it be. In Flutter it is a `LayoutBuilder` over a `Column`: the
//! builder reads the height it is given, asks the core's `layout` which of
//! the optional rows (the gaps, the arrows, the footer's second line) a short
//! pane keeps, and builds each piece at that height — header, tabs and search
//! stay, a result row stays, the rest yields. The sizes it asks with are
//! measured here (the rows a text takes at the width: `widgets/measure.rs`),
//! never cut by hand. Oracle: `selection_picker_layout.rs`, `render` of
//! `list_selection_view.rs`.

use std::rc::Rc;

use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use super::layout::{self, Sizes, Surface};
use super::list::Side;

use super::rows::Rows;
use super::side;
use crate::widgets::measure;

/// Cells the popup keeps free on either side of its text.
const INSET: usize = 4;

/// What the pane is made of: every piece built, none placed yet.
pub struct Pane {
    pub surface: Surface,
    pub title: Option<(String, Style)>,
    /// The paragraphs under the title (the subtitle, or the oracle's header
    /// `Paragraph`): runs in their own styles, a blank paragraph is a blank row.
    pub paragraphs: Vec<Vec<(String, Style)>>,
    /// The tab strip and the search line, when the list has them.
    pub tabs: Option<AnyWidget>,
    pub search: Option<AnyWidget>,
    /// The rows (none: the list is empty and says so).
    pub rows: Option<Rc<Rows>>,
    pub empty: (String, Style),
    pub above: AnyWidget,
    pub below: AnyWidget,
    pub side: Option<(Side, Style)>,
    /// The line naming what a multi-select has on, the footer note and hint.
    pub preview: Option<String>,
    pub note: Option<String>,
    /// The note in runs of their own styles and links (its text is `note`); none: `note` is plain.
    pub note_runs: Vec<(String, Style)>,
    pub hint: Option<(String, Style)>,
    /// What the pane keeps for the tallest tab (`reserve_result_rows`).
    pub reserved: Option<Reserved>,
}

/// The sizes a pane holds at whatever tab it shows: nothing jumps when the tab or the search
/// changes (the oracle's `header_height`, `footer_hint_lines`, `result_area_height`).
pub struct Reserved {
    /// The rows the result area keeps.
    pub rows: usize,
    /// The paragraphs (texts) of the header of each tab.
    pub headers: Vec<Vec<String>>,
    /// The texts a footer hint may be.
    pub hints: Vec<String>,
}

impl Pane {
    pub fn view(self) -> AnyWidget {
        let pane = Rc::new(self);
        LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = u16::try_from(constraints.max_height.max(0)).unwrap_or(u16::MAX);
            pane.lay_out(seg, width, height)
        })
        .into()
    }

    fn lay_out(&self, seg: measure::Measure, width: usize, height: u16) -> AnyWidget {
        let lines = |text: &str, room: usize| {
            u16::try_from(measure::rows(seg, text, room.max(1)).len().max(1)).unwrap_or(u16::MAX)
        };
        let inner = width.saturating_sub(INSET);
        let title = self
            .title
            .as_ref()
            .map_or(0, |(text, _)| lines(text, inner));
        let rows_of: Vec<u16> = self
            .paragraphs
            .iter()
            .map(|runs| {
                let text: String = runs.iter().map(|(text, _)| text.as_str()).collect();
                lines(&text, inner)
            })
            .collect();
        let mut subtitle: u16 = rows_of.iter().sum();
        if let Some(reserved) = &self.reserved {
            let tallest = reserved
                .headers
                .iter()
                .map(|header| header.iter().map(|text| lines(text, inner)).sum::<u16>())
                .max()
                .unwrap_or(0);
            subtitle = subtitle.max(tallest);
        }
        // The rows sit beside a preview when the pane is wide enough.
        let rooms = self
            .side
            .as_ref()
            .map_or(width, |(side, _)| side::body_width(side, width));
        let measured = self.rows.as_ref().map(|rows| rows.measure(seg, rooms));
        let mut wanted = match (&self.rows, &measured) {
            (Some(rows), Some(layout)) => u16::try_from(rows.wanted(layout)).unwrap_or(u16::MAX),
            _ => 1,
        };
        if let Some(reserved) = &self.reserved {
            wanted = wanted.max(u16::try_from(reserved.rows).unwrap_or(u16::MAX));
        }
        let note = self.note.as_ref().map_or(0, |text| lines(text, inner));
        let preview = u16::from(self.preview.is_some());
        let mut hint = self
            .hint
            .as_ref()
            .map_or(0, |(text, _)| lines(text, width.saturating_sub(2)));
        if let (Some(reserved), true) = (&self.reserved, self.hint.is_some()) {
            for text in &reserved.hints {
                hint = hint.max(lines(text, width.saturating_sub(2)));
            }
        }
        let sizes = Sizes {
            header: title + subtitle,
            header_gap: 1,
            tabs: u16::from(self.tabs.is_some()),
            search: u16::from(self.search.is_some()),
            rows: wanted,
            side: 0,
        };
        let (top, content, footer) =
            layout::split(height, note + preview + hint, sizes, self.surface);
        let at = layout::heights(content, sizes, self.surface);

        let gap =
            |rows: u16| -> AnyWidget { SizedBox::new(None, Some(i32::from(rows)), None).into() };
        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };
        let mut body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        // The header yields rows from its end: the paragraphs under the title first.
        let mut left = at.header;
        let pieces = self
            .title
            .iter()
            .map(|(text, style)| (vec![(text.clone(), style.clone())], title))
            .chain(self.paragraphs.iter().cloned().zip(rows_of.iter().copied()));
        for (runs, rows) in pieces {
            let shown = rows.min(left);
            left -= shown;
            if shown > 0 {
                let spans: Vec<Span> = runs
                    .into_iter()
                    .map(|(text, style)| Span::styled(text, style))
                    .collect();
                body = body.child(inset(
                    Text::rich(spans)
                        .wrap(TextWrap::Word)
                        .max_lines(usize::from(shown))
                        .into(),
                ));
            }
        }
        if left > 0 {
            // A tab with a shorter header keeps the room of the tallest.
            body = body.child(gap(left));
        }
        if at.header_gap > 0 {
            body = body.child(gap(at.header_gap));
        }
        if let Some(tabs) = self.tabs.as_ref().filter(|_| at.tabs > 0) {
            body = body.child(inset(tabs.clone()));
        }
        if at.tab_gap > 0 {
            body = body.child(gap(at.tab_gap));
        }
        if let Some(search) = self.search.as_ref().filter(|_| at.search > 0) {
            body = body.child(search.clone());
        }
        if at.above > 0 {
            body = body.child(self.above.clone());
        }
        if at.rows > 0 {
            body = body.child(match (&self.rows, &measured) {
                (Some(rows), Some(layout)) => rows.list(layout, i32::from(at.rows)),
                _ => SizedBox::new(
                    None,
                    Some(i32::from(if self.reserved.is_some() { at.rows } else { 1 })),
                    Some(
                        Text::line(self.empty.0.clone())
                            .style(self.empty.1.clone())
                            .into(),
                    ),
                )
                .into(),
            });
        }
        if at.below > 0 {
            body = body.child(self.below.clone());
        }
        let mut body: AnyWidget = body.into();
        if let Some((side, style)) = &self.side {
            body = side::with_preview(body, side.clone(), style.clone());
        }
        let mut pane = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if top > 0 {
            pane = pane.child(gap(top));
        }
        pane = pane.child(body);
        // The footer yields rows from its end: the hint first.
        let mut left = footer;
        let mut foot = |pane: Column,
                        text: &String,
                        runs: &[(String, Style)],
                        rows: u16,
                        style: &Style,
                        wrap: TextWrap,
                        inset_left: bool| {
            let shown = rows.min(left);
            left -= shown;
            if shown == 0 {
                return pane;
            }
            let text = if runs.is_empty() {
                Text::new(text.clone())
            } else {
                Text::rich(
                    runs.iter()
                        .map(|(text, style)| Span::styled(text.clone(), style.clone()))
                        .collect::<Vec<_>>(),
                )
            }
            .style(style.clone())
            .wrap(wrap)
            .max_lines(usize::from(shown));
            if inset_left {
                pane.child(Padding::new(EdgeInsets::only(2, 0, 0, 0), text))
            } else {
                pane.child(inset(text.into()))
            }
        };
        if let Some(text) = &self.note {
            let runs = &self.note_runs;
            pane = foot(pane, text, runs, note, &Style::new(), TextWrap::Word, false);
        }
        if let Some(text) = &self.preview {
            let style = Style::new();
            pane = foot(pane, text, &[], preview, &style, TextWrap::None, false);
        }
        if let Some((text, style)) = &self.hint {
            pane = foot(pane, text, &[], hint, style, TextWrap::Word, true);
            if self.reserved.is_some() && left > 0 {
                pane = pane.child(gap(left));
            }
        }
        pane.into()
    }
}
