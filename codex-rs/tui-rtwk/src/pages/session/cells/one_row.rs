//! A cell of one row: `Text.rich` with `maxLines: 1, overflow: ellipsis` (the
//! oracle's `truncate_line_with_ellipsis_if_overflow`).

use rtwk::AnyWidget;
use rtwk::style::Span;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;

pub fn one_row(spans: Vec<Span>) -> AnyWidget {
    Text::rich(spans)
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}
