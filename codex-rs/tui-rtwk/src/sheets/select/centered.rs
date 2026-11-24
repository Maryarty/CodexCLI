//! The Centered presentation of a page of a select sheet (`ViewPresentation::Centered`, oracle
//! `view_stack.rs` `dialog_area`): the page floats in the middle of the sheet, at most 72 cells
//! wide and two cells in from its sides, over what was there: the page under it (the retained
//! parent) and the composer under the sheet, which the sheet's own barrier leaves unshaded.
//! `centered` is the page for a home or a replacement, `route` the page pushed over another one:
//! a route that is not opaque, so the page under it keeps showing around it. A sheet whose
//! oracle view is `SelectionViewParams::confirmation()` shows its confirmation this way; the
//! quick-dial confirmations are not one (`confirm_quick_dial` asks for `picker()`: the bottom
//! pane's own rows, which `select::quick` keeps).

use std::rc::Rc;

use rtwk::layout::{Align, Alignment, EdgeInsets, Padding};
use rtwk::navigation::{Route, RouteBehavior};
use rtwk::overlay::Dialog;
use rtwk::{AnyWidget, BuildCx};

/// The widest a Centered page is (`dialog_area`).
const WIDTH: i32 = 72;

/// `page` in the middle of its sheet, 72 cells wide at most, two cells in from the sides.
pub fn centered(page: impl Into<AnyWidget>) -> AnyWidget {
    Align::new(
        Alignment::CENTER,
        Padding::new(
            EdgeInsets::symmetric(2, 0),
            Dialog::bare(page).max_width(WIDTH),
        ),
    )
    .into()
}

/// The route of a page shown Centered over the one under it, which stays visible around it (a
/// dialog route of the sheet's navigator: not opaque). `page` builds it each time it is built.
pub fn route(page: impl Fn() -> AnyWidget + 'static) -> Route {
    Route::new(Floating(Rc::new(page)))
}

struct Floating(Rc<dyn Fn() -> AnyWidget>);

impl RouteBehavior for Floating {
    fn build(&self, _cx: &mut BuildCx<'_>) -> AnyWidget {
        centered((self.0)())
    }

    fn opaque(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rtwk::navigation::{Navigator, RouteBehavior};
    use rtwk::text::Text;
    use rtwk_test::{Options, Tester};

    use super::{Floating, centered};

    const LONG: &str =
        "Replace the saved preset with the line below and keep the others as they are";

    #[test]
    fn a_centered_page_floats_in_the_middle_two_cells_in_from_the_sides() {
        let mut t = Tester::with(Options::new().size(40, 16));
        t.pump_widget(Navigator::new().home(centered(Text::new(LONG))));
        let lines = t.lines();
        let drawn: Vec<(usize, &String)> = lines
            .iter()
            .enumerate()
            .filter(|(_, row)| !row.trim().is_empty())
            .collect();
        let first = drawn.first().map_or(0, |(row, _)| *row);
        assert!(
            (4..=8).contains(&first),
            "the middle of 16 rows, not an edge: row {first}\n{}",
            lines.join("\n")
        );
        for (_, row) in &drawn {
            let indent = row.chars().take_while(|c| *c == ' ').count();
            let width = row.trim_end().chars().count();
            assert!(
                indent >= 2 && width <= 38,
                "two cells in from both sides: {row:?}"
            );
        }
    }

    #[test]
    fn a_page_pushed_as_a_route_is_not_opaque_so_the_page_under_it_shows() {
        let floating = Floating(Rc::new(|| Text::new(LONG).into()));
        assert!(!floating.opaque());
    }
}
