//! The Centered presentation of a page of a select sheet (`ViewPresentation::Centered`, oracle
//! `view_stack.rs` `dialog_area`): the page floats in the middle of the sheet, at most 72 cells
//! wide and two cells in from its sides, over what was there: the page under it (the retained
//! parent) and the composer under the sheet, which the sheet's own barrier leaves unshaded.
//! `centered` is the page for a home or a replacement, `route` the page pushed over another one:
//! a route that is not opaque, so the page under it keeps showing around it. A page that
//! confirms what the page under it chose (`SelectionViewParams::confirmation`, the confirmation
//! of a recalled preset) is shown this way.

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
