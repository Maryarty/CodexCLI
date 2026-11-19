//! The session page in its loading state (open-instant): pushed over the
//! command center at once when a task is opened, while its resume is out;
//! the center pops it (and itself, on success) when the reply comes, as
//! Flutter's `Navigator.push` shows a page whose `FutureBuilder` waits.

use rtwk::navigation::PageRoute;
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::pages::agents::strings;

/// The route of the loading page.
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Opening.into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening;

impl_widget!(stateless eq Opening);

impl StatelessWidget for Opening {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        // Oracle: `agents_overview_loading.rs`, the bold line at the top left
        // of an otherwise empty screen, wrapped to the width.
        Text::new(strings::of(cx).resuming)
            .style(Style::new().attrs(Attrs::BOLD))
            .into()
    }
}
