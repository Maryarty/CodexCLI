//! The key footer under the composer: the page's key hints, or one hint
//! that stands for them (Esc's backtrack, Tab's queue), or the shortcut
//! overlay's close hint, two cells in. Oracle: `bottom_pane/footer.rs`.

use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use crate::app::theme;
use crate::l10n::strings as l10n;
use crate::widgets::page_keys;

/// What the footer says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FooterLine {
    /// The page's key hints.
    Keys,
    /// One hint in their place.
    Hint(&'static str),
    /// The shortcut overlay is open: how to close it.
    Close(&'static str),
}

/// The footer line `line`.
pub fn footer<S>(cx: &mut Cx<'_, S>, line: FooterLine) -> AnyWidget {
    let tokens = theme::Tokens::of(cx);
    let strings = l10n::of(cx);
    let body: AnyWidget = match line {
        FooterLine::Close(close) => Text::new(close).style(tokens.dim()).into(),
        FooterLine::Hint(hint) => Text::line(hint).style(tokens.dim()).into(),
        FooterLine::Keys => Expanded::new(
            1,
            page_keys::footer_with(
                [
                    (strings.pages.key_agents, strings.pages.hint_agents),
                    (strings.pages.key_shortcuts, strings.pages.hint_shortcuts),
                ],
                tokens.bold(),
                tokens.dim(),
                " · ",
            ),
        )
        .into(),
    };
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        .child(body)
        .into()
}

/// `footer` with shell mode's label at its right, one cell in
/// (`shell_mode_footer_line`).
pub fn with_shell_mode<S>(cx: &mut Cx<'_, S>, footer: AnyWidget) -> AnyWidget {
    let tokens = theme::Tokens::of(cx);
    let label = super::strings::of(cx).shell_mode;
    Row::new()
        .child(Expanded::new(1, footer))
        .child(Text::new(label).style(tokens.shell()))
        .child(SizedBox::new(Some(1), None, None))
        .into()
}
