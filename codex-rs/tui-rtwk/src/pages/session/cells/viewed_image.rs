//! A viewed image (oracle history_cell/patches.rs `ViewImageHistoryCell`).

use rtwk::style::Span;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

use super::one_row::one_row;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewedImageCell {
    pub name: String,
}

impl_widget!(stateless eq ViewedImageCell);

impl StatelessWidget for ViewedImageCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        one_row(vec![
            Span::styled(strings.bullet, tokens.dim()),
            Span::styled(strings.viewed_image, tokens.bold()),
            Span::styled(
                self.name.replace(['\n', '\r', '\t'], strings.gap),
                tokens.dim(),
            ),
        ])
    }
}
