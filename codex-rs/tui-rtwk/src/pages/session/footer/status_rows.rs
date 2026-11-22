//! The status line under the composer (oracle `chat_composer/status_surface.rs`,
//! `composer_layout.rs` `FooterLayout`): the configured items wrapped over the
//! rows the frame leaves it, above the hint row.
//!
//! Flutter writes it as a `Text(maxLines: n)` where `n` comes from a
//! `LayoutBuilder` over the room the page gives its bottom pane: the composer
//! comes first (the draft keeps its rows), the hints keep theirs, the line
//! takes what a quarter of the pane allows (two to four rows) less those. The
//! draft's rows are read from its `TextEditingController` (`ListenableBuilder`,
//! so a growing draft takes rows from the line while it is typed), the hints'
//! from the footer's own fit; this widget cuts nothing, `Text` wraps and puts
//! the ellipsis on the last row.

use rtwk::editing::TextEditingController;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::Span;
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use crate::widgets::measure;

use super::{Footer, Line};

/// Cells the line starts in (`inset_footer_hint_area`).
const INDENT: i32 = 2;
/// Cells before the draft's text (`LIVE_PREFIX_COLS` and the margin).
const DRAFT_COLS: usize = 3;
/// The rows the composer's surface takes besides the draft's own (`input_height`).
const SURFACE_ROWS: usize = 2;

/// The status line `spans`, under a composer holding `composer`'s draft, over
/// the hints `hints` draws.
#[derive(Clone, PartialEq)]
pub struct StatusRows {
    pub spans: Vec<Span>,
    pub composer: TextEditingController,
    /// The footer under the line: its rows are kept before the line's.
    pub hints: Option<Footer>,
}

impl_widget!(stateless eq StatusRows);

/// The rows the line may take of a frame `frame` rows high (`FooterLayout`):
/// `input` rows of draft under the surface, `hints` rows wanted below.
pub fn rows_for(frame: usize, input: usize, hints: usize) -> usize {
    // the composer's room in a frame (`composer_height_limit`), and the quarter of it the
    // footer may take, two to four rows (`FooterLayout::new`)
    let room = (frame * 2 / 3).max(8).min(frame);
    let cap = ((room + 2) / 4).clamp(2, 4);
    let available = room.saturating_sub(input + SURFACE_ROWS);
    let limit = cap.min(available);
    // one row of the line is kept before the hints are placed (`allocate`)
    let budget = limit.saturating_sub(hints.min(1));
    let kept = budget.min(1);
    let hints = hints.min(limit - kept);
    limit - hints
}

impl StatelessWidget for StatusRows {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let hints = self.hints.clone();
        // the hints are measured by the same footer widget that draws them
        let spec = hints
            .as_ref()
            .filter(|footer| footer.line == Line::Passive)
            .map(|footer| footer.spec(cx));
        let spans = self.spans.clone();
        let composer = self.composer.clone();
        let draft = composer.clone();
        ListenableBuilder::new(draft.listenable(), move |_, _| {
            let spec = spec.clone();
            let spans = spans.clone();
            let composer = composer.clone();
            LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let width = usize::try_from(constraints.max_width).unwrap_or(0);
                let frame = usize::try_from(constraints.max_height).unwrap_or(0);
                let draft = composer.text();
                let input = measure::rows(seg, &draft, width.saturating_sub(DRAFT_COLS))
                    .len()
                    .max(1);
                let wanted = match &spec {
                    Some(spec) => spec.wanted_rows(&|text: &str| seg.width(text), width),
                    None => 1,
                };
                let rows = rows_for(frame, input, wanted);
                if rows == 0 {
                    return SizedBox::shrink().into();
                }
                Row::new()
                    .child(SizedBox::new(Some(INDENT), None, None))
                    .child(Expanded::new(
                        1,
                        Text::rich(spans.clone())
                            .max_lines(Some(rows))
                            .overflow(TextOverflow::Ellipsis),
                    ))
                    .into()
            })
            .into()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::rows_for;

    // oracle status_surface_tests.rs `status_rows_respect_host_budget_and_yield_to_the_draft`:
    // the host's rows go to the draft first (3 rows of draft, no hint row)
    #[test]
    fn the_draft_keeps_its_rows_before_the_line_takes_any() {
        assert_eq!(rows_for(7, 3, 0), 2);
        assert_eq!(rows_for(7, 5, 0), 0);
    }

    // `FooterLayout::new`: the cap is a quarter of the composer's room
    #[test]
    fn the_cap_follows_the_frame() {
        // idle composer (1 row of draft) and one hint row
        assert_eq!(rows_for(12, 1, 1), 1);
        assert_eq!(rows_for(15, 1, 1), 2);
        assert_eq!(rows_for(24, 1, 1), 3);
        assert_eq!(rows_for(40, 1, 1), 3);
        // no hint row: the line has the whole cap
        assert_eq!(rows_for(40, 1, 0), 4);
    }

    // two hint rows leave the line one
    #[test]
    fn two_hint_rows_leave_the_line_one() {
        assert_eq!(rows_for(40, 1, 2), 2);
    }
}
