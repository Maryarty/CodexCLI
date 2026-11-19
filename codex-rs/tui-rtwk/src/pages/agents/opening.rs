//! The session page in its loading state (open-instant): pushed over the
//! command center at once when a task is opened, while its resume is out;
//! the center pops it (and itself, on success) when the reply comes, as
//! Flutter's `Navigator.push` shows a page whose `FutureBuilder` waits.

use rtwk::layout::Center;
use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme;
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
        let tokens = theme::Tokens::of(cx);
        Center::new(Text::new(strings::of(cx).resuming).style(tokens.dim())).into()
    }
}
