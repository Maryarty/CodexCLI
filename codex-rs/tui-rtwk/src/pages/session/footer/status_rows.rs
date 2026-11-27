//! The status and hint rows share the space below the draft (oracle
//! `chat_composer/composer_layout.rs` `FooterLayout`).
//!
//! Flutter writes it as a `Text(maxLines: n)` where `n` comes from a
//! `LayoutBuilder` over the room the page gives its bottom pane: the composer
//! comes first (the draft keeps its rows), the hints keep theirs, the line
//! takes what a quarter of the pane allows (two to four rows) less those. The
//! draft's rows are read from its `TextEditingController` (`ListenableBuilder`,
//! so a growing draft takes rows from the line while it is typed), the hints'
//! from the footer's own fit; this widget cuts nothing, `Text` wraps and puts
//! the ellipsis on the last row.

use rtwk::binding::{AppBinding, ScreenMode};
use rtwk::editing::TextEditingController;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::Provider;
use rtwk::style::Span;
use rtwk::text::segmenter_of;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use crate::widgets::measure;

use super::effort::StatusLine;
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
    pub footer: Option<AnyWidget>,
}

impl_widget!(stateless eq StatusRows);

/// The number of rows the passive hints may use to select their layout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct HintRows(pub usize);

fn allocate(
    frame: usize,
    input: usize,
    wanted_hints: usize,
    has_status: bool,
    inline: bool,
) -> (usize, usize) {
    let room = (frame * 2 / 3).max(8).min(frame);
    let cap = if inline {
        2
    } else {
        // Keep room for a status and both hint rows when the width calls for them.
        let minimum = 2 + usize::from(wanted_hints > 1);
        ((room + 2) / 4).clamp(minimum, 4)
    };
    let available = room.saturating_sub(input + SURFACE_ROWS);
    let limit = cap.min(available);
    let reserve_status = usize::from(has_status && limit > wanted_hints.min(1));
    let hints = wanted_hints.min(limit - reserve_status);
    let status = if has_status { limit - hints } else { 0 };
    (status, hints)
}

impl StatelessWidget for StatusRows {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let hints = self.hints.clone();
        // the hints are measured by the same footer widget that draws them
        let spec = hints
            .as_ref()
            .filter(|footer| footer.line == Line::Passive)
            .map(|footer| footer.spec(cx));
        let inline =
            AppBinding::maybe_of(cx).is_some_and(|binding| binding.mode() == ScreenMode::Inline);
        let footer = self.footer.clone();
        let spans = self.spans.clone();
        let composer = self.composer.clone();
        let draft = composer.clone();
        ListenableBuilder::new(draft.listenable(), move |_, _| {
            let footer = footer.clone();
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
                let (rows, hints) = allocate(frame, input, wanted, !spans.is_empty(), inline);
                let mut layout = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start);
                if rows > 0 {
                    layout = layout.child(
                        Row::new()
                            .child(SizedBox::new(Some(INDENT), None, None))
                            .child(Expanded::new(
                                1,
                                StatusLine {
                                    spans: spans.clone(),
                                    rows,
                                },
                            )),
                    );
                }
                if let Some(footer) = &footer {
                    layout = layout.child(
                        Provider::value(HintRows(hints))
                            .update_should_notify(|old, new| old != new)
                            .child(footer.clone()),
                    );
                }
                layout.into()
            })
            .into()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::allocate;

    fn rows_for(frame: usize, input: usize, hints: usize) -> usize {
        allocate(frame, input, hints, true, false).0
    }

    #[test]
    fn a_running_draft_keeps_queue_hints_when_the_viewport_shrinks() {
        assert_eq!(allocate(6, 1, 2, true, false), (1, 2));
        assert_eq!(allocate(5, 1, 2, true, false), (1, 1));
        assert_eq!(allocate(7, 1, 2, true, true), (1, 1));
        assert_eq!(allocate(4, 1, 2, true, true), (0, 1));
    }

    #[test]
    fn extra_fullscreen_height_never_reduces_the_footer_budget() {
        let mut previous = allocate(4, 1, 2, true, false);
        for height in 5..=24 {
            let next = allocate(height, 1, 2, true, false);
            assert!(next.0 >= previous.0 && next.1 >= previous.1);
            previous = next;
        }
    }

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
