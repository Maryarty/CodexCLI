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
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::select::layout::{self, Sizes, Surface};
use codex_tui_core::screens::select::list::Side;

use super::rows::Rows;
use super::side;
use crate::widgets::measure;

/// Cells the popup keeps free on either side of its text.
const INSET: usize = 4;

/// What the pane is made of: every piece built, none placed yet.
pub struct Pane {
    pub surface: Surface,
    pub title: Option<(String, Style)>,
    pub subtitle: Option<(String, Style)>,
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
    pub hint: Option<(String, Style)>,
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
        let subtitle = self
            .subtitle
            .as_ref()
            .map_or(0, |(text, _)| lines(text, inner));
        // The rows sit beside a preview when the pane is wide enough.
        let rooms = self
            .side
            .as_ref()
            .map_or(width, |(side, _)| side::body_width(side, width));
        let measured = self.rows.as_ref().map(|rows| rows.measure(seg, rooms));
        let wanted = match (&self.rows, &measured) {
            (Some(rows), Some(layout)) => u16::try_from(rows.wanted(layout)).unwrap_or(u16::MAX),
            _ => 1,
        };
        let note = self.note.as_ref().map_or(0, |text| lines(text, inner));
        let preview = u16::from(self.preview.is_some());
        let hint = self
            .hint
            .as_ref()
            .map_or(0, |(text, _)| lines(text, width.saturating_sub(2)));
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
        // The header yields rows from its end: the subtitle first.
        let mut left = at.header;
        for (piece, rows) in [(&self.title, title), (&self.subtitle, subtitle)] {
            let Some((text, style)) = piece else { continue };
            let shown = rows.min(left);
            left -= shown;
            if shown > 0 {
                body = body.child(inset(
                    Text::new(text.clone())
                        .style(style.clone())
                        .wrap(TextWrap::Word)
                        .max_lines(usize::from(shown))
                        .into(),
                ));
            }
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
                _ => Text::line(self.empty.0.clone())
                    .style(self.empty.1.clone())
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
                        rows: u16,
                        style: &Style,
                        wrap: TextWrap,
                        inset_left: bool| {
            let shown = rows.min(left);
            left -= shown;
            if shown == 0 {
                return pane;
            }
            let text = Text::new(text.clone())
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
            pane = foot(pane, text, note, &Style::new(), TextWrap::Word, false);
        }
        if let Some(text) = &self.preview {
            pane = foot(pane, text, preview, &Style::new(), TextWrap::None, false);
        }
        if let Some((text, style)) = &self.hint {
            pane = foot(pane, text, hint, style, TextWrap::Word, true);
        }
        pane.into()
    }
}
