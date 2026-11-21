//! The tips under a request form (`key label`, dim, apart by ` | `): the oracle packs whole tips
//! into rows (`footer_hint::wrap_hint_rows`), so a row never ends in a separator. Flutter writes
//! it as a `Wrap` of the tips; `Wrap` has no separator that drops at the end of a run, so the
//! app only picks WHICH tips stand in which row from the measured width (`LayoutBuilder` +
//! `segmenter_of(cx)`, as `widgets/dots.rs` picks a spelling) and a `Column` of `Text` rows
//! lays them out and paints them.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

/// `tips` (each its spans) in rows as wide as the room, `separator` between the tips of a row.
pub fn rows(tips: Vec<Vec<Span>>, separator: &'static str, separator_style: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let cells = |spans: &[Span]| -> i32 {
            spans
                .iter()
                .map(|span| i32::try_from(segmenter.width(&span.text)).unwrap_or(i32::MAX))
                .sum()
        };
        let gap = i32::try_from(segmenter.width(separator)).unwrap_or(0);
        let room = constraints.max_width;
        let mut packed: Vec<Vec<Span>> = Vec::new();
        let mut used = 0;
        for tip in &tips {
            let width = cells(tip);
            match packed.last_mut() {
                Some(row) if used + gap + width <= room => {
                    row.push(Span::styled(separator, separator_style.clone()));
                    row.extend(tip.iter().cloned());
                    used += gap + width;
                }
                _ => {
                    packed.push(tip.clone());
                    used = width;
                }
            }
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for row in packed {
            column = column.child(Text::rich(row).wrap(TextWrap::Word));
        }
        column.into()
    })
    .into()
}
