//! Why a turn went wrong (oracle history_cell/notices.rs `new_error_event`):
//! one red row, `■ ` and the message, wrapped by the transcript.
//!
//! Flutter: a stateless widget over one `Text`.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCell {
    pub message: String,
}

impl_widget!(stateless eq ErrorCell);

impl StatelessWidget for ErrorCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        Text::new(strings.error_line.replace("{}", &self.message))
            .style(tokens.error())
            .into()
    }
}
