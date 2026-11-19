//! The session page in its loading state (open-instant): pushed over the
//! command center at once when a task is opened, while its resume is out;
//! the center pops it (and itself, on success) when the reply comes, as
//! Flutter's `Navigator.push` shows a page whose `FutureBuilder` waits.

use rtwk::navigation::{PageRoute, PopInvoked, PopScope};
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, StatelessWidget, impl_widget};

use crate::pages::agents::strings;

/// The route of the loading page; `popped` is told when it is popped (Esc
/// cancels the opening: Flutter's `PopScope.onPopInvokedWithResult`).
pub fn route(popped: Handler<PopInvoked>) -> PageRoute<()> {
    PageRoute::new(move |_| {
        Opening {
            popped: popped.clone(),
        }
        .into()
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Opening {
    popped: Handler<PopInvoked>,
}

impl_widget!(stateless eq Opening);

impl StatelessWidget for Opening {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        // Oracle: `agents_overview_loading.rs`, the bold line at the top left
        // of an otherwise empty screen, wrapped to the width.
        PopScope::new(Text::new(strings::of(cx).resuming).style(Style::new().attrs(Attrs::BOLD)))
            .on_pop_invoked_with_result(self.popped.clone())
            .into()
    }
}
