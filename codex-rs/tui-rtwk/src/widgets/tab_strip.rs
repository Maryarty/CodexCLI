//! The filled tab strip, shared by the command center's filters and the
//! selection sheet's tabs, the Flutter way (G36 and G45: not gaps,
//! `docs/rtwk-port/rtwk-gaps.md`): a `LayoutBuilder` measures the labels with
//! `segmenter_of(cx)` and a `Row` lays out the whole tabs one blank apart,
//! the overflow marks outside them and the active tab in the selection style
//! (oracle: `selection_tabs.rs` `render_filled_tab_bar`). The window of whole
//! tabs is a pure function, unit-tested at the oracle's widths.

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::LayoutBuilder;
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::Style;
use rtwk::text::Text;
use rtwk::text::segmenter_of;
use rtwk_core::unicode::TextSegmenter;

use crate::widgets::dots::dots_end;

/// The tabs shown at `width` cells: the whole tabs around the active one that
/// fit beside the marks, and what the marks show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strip {
    pub start: usize,
    pub end: usize,
    pub left: bool,
    pub right: bool,
    /// The width of each shown tab, clipped to the room.
    pub widths: Vec<usize>,
}

pub fn strip(seg: TextSegmenter, labels: &[String], active: usize, width: usize) -> Strip {
    let count = labels.len();
    let none = Strip {
        start: 0,
        end: 0,
        left: false,
        right: false,
        widths: Vec::new(),
    };
    if count == 0 || width == 0 {
        return none;
    }
    let widths: Vec<usize> = labels.iter().map(|l| seg.width(l) + 2).collect();
    let occupied = |start: usize, end: usize| {
        widths[start..end].iter().sum::<usize>() + end - start - 1
            + usize::from(start > 0) * 2
            + usize::from(end < count) * 2
    };
    let (mut start, mut end) = (active.min(count - 1), active.min(count - 1) + 1);
    while start > 0 && occupied(start - 1, end) <= width {
        start -= 1;
    }
    while end < count && occupied(start, end + 1) <= width {
        end += 1;
    }
    let left = start > 0 && width >= 5;
    let right = end < count && width >= 7;
    let mut x = if left { 2 } else { 0 };
    let limit = width - if right { 2 } else { 0 };
    let shown = widths[start..end]
        .iter()
        .map(|w| {
            let w = (*w).min(limit.saturating_sub(x));
            x += w + 1;
            w
        })
        .collect();
    Strip {
        start,
        end,
        left,
        right,
        widths: shown,
    }
}

/// The strip as a row of `Text`s in the cells it gets: the marks, the tabs a
/// blank apart (a tap on one selects it) and room between the last tab and
/// the right mark.
pub struct Tabs {
    pub labels: Rc<[String]>,
    pub active: usize,
    pub marks: (&'static str, &'static str),
    /// The active tab's style and the others'.
    pub styles: (Style, Style),
    pub taps: Vec<Handler<PointerEvent>>,
}

fn mark(text: &'static str, style: &Style) -> AnyWidget {
    let text = Text::line(text).style(style.clone());
    SizedBox::new(Some(1), Some(1), Some(text.into())).into()
}

impl Tabs {
    pub fn widget(self) -> AnyWidget {
        LayoutBuilder::new(move |cx, constraints| {
            self.row(
                segmenter_of(cx),
                usize::try_from(constraints.max_width).unwrap_or(0),
            )
        })
        .into()
    }

    fn row(&self, seg: TextSegmenter, width: usize) -> AnyWidget {
        let shown = strip(seg, &self.labels, self.active, width);
        let (selected, idle) = &self.styles;
        let blank = |cells: i32| -> AnyWidget { SizedBox::fixed(cells, 1).into() };
        let mut row = Row::new();
        if shown.left {
            row = row.child(mark(self.marks.0, idle)).child(blank(1));
        }
        for (offset, cells) in shown.widths.iter().enumerate() {
            let index = shown.start + offset;
            let style = if index == self.active { selected } else { idle };
            let label = dots_end(format!(" {} ", self.labels[index]), style.clone());
            let tab = SizedBox::new(Some(*cells as i32), Some(1), Some(label));
            if offset > 0 {
                row = row.child(blank(1));
            }
            row = row.child(match self.taps.get(index) {
                Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(tab).into(),
                None => AnyWidget::from(tab),
            });
        }
        if shown.right {
            row = row
                .child(Expanded::new(1, blank(0)))
                .child(mark(self.marks.1, idle));
        }
        row.into()
    }
}
