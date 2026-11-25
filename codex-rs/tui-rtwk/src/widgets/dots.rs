//! A line cut by its room with `…` after the last whole blank (N1,
//! `docs/rtwk-port/rtwk-gaps.md`: not a gap). `TextOverflow::Ellipsis` drops
//! the blanks before the `…`, where the oracle's cut keeps them (` All … `).
//! Flutter writes it as `Text(overflow: clip)` and a `…` shown only when the
//! `TextPainter` says the text overflows; here `LayoutBuilder` +
//! `segmenter_of(cx)` (`width`, `cut_head_cells`) + `Row[SizedBox(clipped Text), Text("…")]`.

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Row, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_core::unicode::cut_head_cells;

/// `text` in the room it gets; when it does not fit, the end is cut and a `…`
/// takes the last cell.
pub fn dots_end(text: String, style: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let line = Text::line(text.clone())
            .style(style.clone())
            .overflow(TextOverflow::Clip);
        if segmenter_of(cx).width(&text) as i32 <= constraints.max_width {
            return line.into();
        }
        // The `…` follows the last whole grapheme that fits (a wide one that
        // does not leaves a cell blank, as the oracle's cut does): the width
        // is measured, the text clips itself.
        let seg = segmenter_of(cx);
        let room = usize::try_from(constraints.max_width.saturating_sub(1)).unwrap_or(0);
        let shown = cut_head_cells(&seg, &text, room, false);
        Row::new()
            .child(SizedBox::new(
                i32::try_from(shown).ok(),
                Some(1),
                Some(line.into()),
            ))
            .child(ExcludeSelection::new(
                Text::line("\u{2026}").style(style.clone()),
            ))
            .into()
    })
    .into()
}

/// A row of spans in the room it gets (`dots_end` for several styles; oracle `clipped_line`): when it
/// does not fit, the end is cut and a `…` in the style of the last span kept takes the last cell,
/// the blanks before it stay.
pub fn dots_spans(spans: Vec<Span>) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let line = Text::rich_line(spans.clone()).overflow(TextOverflow::Clip);
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        if seg.width(&text) as i32 <= constraints.max_width {
            return line.into();
        }
        let room = usize::try_from(constraints.max_width.saturating_sub(1)).unwrap_or(0);
        let shown = cut_head_cells(&seg, &text, room, false);
        // The span the cut falls in is the last one kept.
        let mut left = shown;
        let mut style = Style::default();
        for span in &spans {
            style = span.style.clone();
            let width = seg.width(&span.text);
            if left <= width {
                break;
            }
            left -= width;
        }
        Row::new()
            .child(SizedBox::new(
                i32::try_from(shown).ok(),
                Some(1),
                Some(line.into()),
            ))
            .child(ExcludeSelection::new(Text::line("\u{2026}").style(style)))
            .into()
    })
    .into()
}

/// [`dots_spans`] with the `…` always among the nodes of the line: a cell whose text the app
/// reads before it is mounted (Find over the model) cannot know whether the room cuts the line, and
/// a `…` that is a `Text` only when it cuts would move every match after it by that node. Not cut,
/// the `…` is a node a cell wide of no width (clipped, nothing drawn); cut, it is the one `dots_spans`
/// draws. Either way it is a gutter of one character.
pub fn dots_spans_kept(spans: Vec<Span>) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let line = Text::rich_line(spans.clone()).overflow(TextOverflow::Clip);
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        let last = spans
            .last()
            .map(|span| span.style.clone())
            .unwrap_or_default();
        if seg.width(&text) as i32 <= constraints.max_width {
            return Row::new()
                .child(line)
                .child(ExcludeSelection::new(SizedBox::new(
                    Some(0),
                    Some(1),
                    Some(
                        Text::line("\u{2026}")
                            .style(last)
                            .overflow(TextOverflow::Clip)
                            .into(),
                    ),
                )))
                .into();
        }
        dots_spans(spans.clone())
    })
    .into()
}
