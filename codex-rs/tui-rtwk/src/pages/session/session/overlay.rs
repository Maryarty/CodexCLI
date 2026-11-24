//! What lies over the page: a window of three rows or fewer shows nothing, the running state
//! included (`ui_snapshots_small_heights_idle`). The page keeps its layout and clips, and a
//! blank sheet laid over it by a layout builder that holds no state covers it, so that nothing
//! of the page's own state stands under a builder.

use rtwk::LayoutBuilder;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, Expanded, SizedBox, Stack, StackFit};
use rtwk::style::Style;

/// The window height under which the chat draws nothing.
const HIDDEN_BELOW_ROWS: i32 = 4;

/// The transcript above the bottom pane, and the blank sheet over both.
pub(super) fn page(transcript: Stack, body: Column) -> Stack {
    let page = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .overflow_intended(true)
        .child(Expanded::new(1, transcript))
        .child(body);
    let blank = LayoutBuilder::new(|_, constraints| {
        if constraints.max_height < HIDDEN_BELOW_ROWS {
            BoxWidget::new().style(Style::new()).into()
        } else {
            SizedBox::shrink().into()
        }
    });
    Stack::new().fit(StackFit::Expand).child(page).child(blank)
}
