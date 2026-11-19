//! A line cut by its room with `…` after the last whole blank (N1,
//! `docs/rtwk-port/rtwk-gaps.md`: not a gap). `TextOverflow::Ellipsis` drops
//! the blanks before the `…`, where the oracle's cut keeps them (` All … `).
//! Flutter writes it as `Text(overflow: clip)` and a `…` shown only when the
//! `TextPainter` says the text overflows; here `LayoutBuilder` +
//! `segmenter_of(cx).width` + `Row[Expanded(clipped Text), Text("…")]`.

use rtwk::layout::{Expanded, Row};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

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
        Row::new()
            .child(Expanded::new(1, line))
            .child(Text::line("\u{2026}").style(style.clone()))
            .into()
    })
    .into()
}
