//! What lies over the page: a window of three rows or fewer shows nothing, the running state
//! included (`ui_snapshots_small_heights_idle`). The page keeps its layout and clips, and a
//! blank sheet laid over it by a layout builder that holds no state covers it, so that nothing
//! of the page's own state stands under a builder.

use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, Expanded, SizedBox, Stack, StackFit};
use rtwk::style::Style;
use rtwk::{AnyWidget, LayoutBuilder};

/// The window height under which the chat draws nothing.
const HIDDEN_BELOW_ROWS: i32 = 4;

/// The transcript above the bottom pane, and the blank sheet over both. An inline region is as
/// tall as the page asks (`inline_draft`: the startup draft, whose heading stands where the
/// transcript will), so the transcript has no flex there: a flex child would keep the rows of
/// the tallest frame (the footer's row, gone with the first key).
pub(super) fn page(transcript: Stack, body: Column, inline_draft: bool) -> Stack {
    let transcript: AnyWidget = if inline_draft {
        transcript.into()
    } else {
        Expanded::new(1, transcript).into()
    };
    let page = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .overflow_intended(true)
        .child(transcript)
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
