//! The preview of the selection list: beside the rows when the popup is wide
//! enough (`Side::beside`), else below them. In Flutter it is a `LayoutBuilder`
//! choosing between a `Row` and a `Column` by the width it is given.

use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::style::Style;
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::select::list::{SIDE_GAP, Side};

/// Cells the popup keeps free on either side of its content.
const INSET: u16 = 2;

/// `body` with the preview `side` beside or below it.
pub fn with_preview(body: AnyWidget, side: Side, style: Style) -> AnyWidget {
    LayoutBuilder::new(move |_, constraints| {
        let width = u16::try_from(constraints.max_width.max(0)).unwrap_or(u16::MAX);
        let lines = |lines: &[String]| -> AnyWidget {
            lines
                .iter()
                .fold(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                    |column, line| column.child(Text::line(line.clone()).style(style.clone())),
                )
                .into()
        };
        match side.beside(width.saturating_sub(2 * INSET)) {
            Some(preview) => Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Expanded::new(1, body.clone()))
                .child(SizedBox::new(Some(i32::from(SIDE_GAP)), None, None))
                .child(SizedBox::new(
                    Some(i32::from(preview)),
                    None,
                    Some(lines(&side.lines)),
                ))
                .child(SizedBox::new(Some(i32::from(INSET)), None, None))
                .into(),
            None => Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(body.clone())
                .child(Text::new(String::new()))
                .child(Padding::new(
                    EdgeInsets::symmetric(INSET.into(), 0),
                    lines(side.below()),
                ))
                .into(),
        }
    })
    .into()
}
