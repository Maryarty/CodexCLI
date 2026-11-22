//! What a command line answered with, as a row of the transcript (oracle
//! history_cell/notices.rs `new_info_event` and `new_error_event`): an info
//! note is a dim bullet and the message, an error note is a red `■ ` and the
//! message.
//!
//! Flutter: a list item of the history, one `Text.rich`; not a `SnackBar`.

use rtwk::style::{Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{Note, Tone};

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteCell {
    pub note: Note,
}

impl_widget!(stateless eq NoteCell);

impl StatelessWidget for NoteCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let cells = super::strings::of(cx);
        let message = crate::pages::session::strings::of(cx).note(&self.note.key);
        match self.note.tone {
            Tone::Info => Text::rich(vec![
                Span::styled(cells.bullet, tokens.dim()),
                Span::styled(message, Style::new()),
            ])
            .into(),
            Tone::Done => Text::rich(vec![
                Span::styled(cells.check, tokens.success()),
                Span::styled(message, Style::new()),
            ])
            .into(),
            Tone::Error => Text::new(cells.error_line.replace("{}", &message))
                .style(tokens.error())
                .into(),
        }
    }
}

/// The row's text as the cell shows it (the bullet is a gutter of the
/// info row; the error row's `■ ` is part of its text, as `error::line`).
pub fn texts(
    note: &Note,
    chat: &crate::pages::session::strings::Strings,
    cells: &Strings,
) -> Vec<Part> {
    let message = chat.note(&note.key);
    match note.tone {
        Tone::Info => vec![
            Part {
                text: cells.bullet.to_owned(),
                gutter: true,
            },
            Part::text(message),
        ],
        Tone::Done => vec![
            Part {
                text: cells.check.to_owned(),
                gutter: true,
            },
            Part::text(message),
        ],
        Tone::Error => vec![Part::text(cells.error_line.replace("{}", &message))],
    }
}
