//! Why a turn went wrong (oracle history_cell/notices.rs `new_error_event`):
//! one red row, `■ ` and the message, wrapped by the transcript.
//!
//! Flutter: a stateless widget over one `Text`.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// The row's text: the mark and the message.
pub fn line(message: &str, strings: &Strings) -> String {
    strings.error_line.replace("{}", message)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCell {
    pub message: String,
}

impl_widget!(stateless eq ErrorCell);

impl StatelessWidget for ErrorCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        Text::new(line(&self.message, strings))
            .style(tokens.error())
            .into()
    }
}

/// Why a turn went wrong, and an overloaded server that said nothing (high load).
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let message = match item {
        Item::Error { message } => message.clone(),
        Item::HighLoad => env.chat.high_load.to_string(),
        _ => return AnyWidget::default(),
    };
    below(place.separator, ErrorCell { message }.into())
}
