//! Probe of ADR-013 O1: a nested `Navigator` as the body of a bottom sheet (the sheet of
//! `/plugins`). The oracle's bottom pane is as tall as the view it shows, at the bottom edge.
//!
//! - A navigator in an `IntrinsicHeight` is as tall as its page when the page has an intrinsic
//!   height.
//! - It is not when the page is built by a `LayoutBuilder` from the height it is given (the
//!   picker's pane): a `LayoutBuilder` has no intrinsic height before it is built, so the
//!   `IntrinsicHeight` gives the page none and the page is built for that. Flutter has the same
//!   rule (its `LayoutBuilder` asserts on an intrinsic query); the probe pins what rtwk does.
//! - The shape the sheet has: the navigator fills the cap and the page is aligned at the bottom.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use rtwk::layout::{Align, Alignment, BoxWidget, IntrinsicHeight};
use rtwk::navigation::Navigator;
use rtwk::overlay::Dialog;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, State, StatefulWidget, impl_widget};
use rtwk_test::{Options, Tester};

fn screen(widget: impl Into<AnyWidget>) -> Vec<String> {
    let mut t = Tester::with(Options::new().size(20, 8));
    t.pump_widget(widget);
    t.lines()
}

fn shown(lines: &[String]) -> usize {
    lines.iter().filter(|line| !line.trim().is_empty()).count()
}

/// A page that is a stateful widget over a `LayoutBuilder`, as the picker is: its rows come from
/// the height it is given (at most three).
#[derive(Clone, Debug, PartialEq)]
struct Page;

struct PageState;

impl StatefulWidget for Page {
    type State = PageState;
    fn create_state(&self) -> PageState {
        PageState
    }
}

impl_widget!(stateful eq Page);

impl State for PageState {
    type Widget = Page;
    type Msg = ();
    fn build(&self, _: &Page, _: &mut Cx<'_, Self>) -> AnyWidget {
        Dialog::bare(BoxWidget::new().child(LayoutBuilder::new(|_, c| {
            let rows = usize::try_from(c.max_height.clamp(0, 3)).unwrap_or(0);
            Text::new(vec!["row"; rows].join("\n")).into()
        })))
        .into()
    }
}

#[test]
fn a_navigator_in_an_intrinsic_height_is_as_tall_as_a_page_that_has_an_intrinsic_height() {
    let nested = IntrinsicHeight::new(Navigator::new().home(Text::new("one\ntwo\nthree")));
    let lines = screen(Align::new(Alignment::BOTTOM_CENTER, nested));
    assert_eq!(shown(&lines), 3, "{lines:#?}");
}

#[test]
fn a_navigator_in_an_intrinsic_height_gives_a_layout_builder_page_no_height() {
    // What rtwk does today (the rows the page would show: 3): the page is built for the height
    // the `IntrinsicHeight` found, which is none before the build.
    let nested = IntrinsicHeight::new(Navigator::new().home(Page));
    let lines = screen(Align::new(Alignment::BOTTOM_CENTER, nested));
    assert_eq!(shown(&lines), 1, "O1: {lines:#?}");
}

#[test]
fn a_navigator_as_tall_as_the_cap_shows_a_page_aligned_at_the_bottom() {
    let page = Align::new(Alignment::BOTTOM_CENTER, Page);
    let lines = screen(Navigator::new().home(page));
    assert_eq!(shown(&lines), 3, "{lines:#?}");
    assert!(
        lines[lines.len() - 3..].iter().all(|line| line.contains("row")),
        "{lines:#?}"
    );
}
