//! The cell of a delivered report (oracle `feedback_view.rs` `feedback_success_cell`): a bullet,
//! the follow-up link on a line of its own as a hyperlink and the ids in bold. In Flutter it is
//! a `Text.rich` of spans with a link recognizer each.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::features::feedback::Note;

use super::strings;
use crate::app::theme::Tokens;

/// The success cell of an upload; every other note is a line of words.
pub fn of(note: &Note) -> Option<AnyWidget> {
    let Note::Uploaded {
        category,
        include_logs,
        report,
        audience,
    } = note
    else {
        return None;
    };
    Some(
        SuccessCell {
            lines: strings::success(*category, *include_logs, report, *audience),
            report: report.clone(),
        }
        .into(),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuccessCell {
    /// The words after the bullet, a line each.
    lines: Vec<String>,
    /// The id the report was filed under, in bold wherever it appears.
    report: String,
}

impl_widget!(stateless eq SuccessCell);

impl StatelessWidget for SuccessCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let link = tokens.accent().attrs(Attrs::UNDERLINE);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, line) in self.lines.iter().enumerate() {
            let mut spans = vec![Span::styled(
                if index == 0 { "\u{2022} " } else { "  " },
                if index == 0 { tokens.dim() } else { Default::default() },
            )];
            if line.starts_with("http") {
                spans.push(Span::styled(line.clone(), link.clone().link(line.as_str())));
            } else if let Some(url) = line.strip_prefix("Sentry URL: ") {
                spans.push(Span::new("Sentry URL: "));
                spans.push(Span::styled(url.to_owned(), link.clone().link(url)));
            } else if let Some((before, after)) = line
                .split_once(self.report.as_str())
                .filter(|_| index > 0)
            {
                spans.push(Span::new(before.to_owned()));
                spans.push(Span::styled(self.report.clone(), tokens.bold()));
                spans.push(Span::new(after.to_owned()));
            } else {
                spans.push(Span::new(line.clone()));
            }
            column = column.child(Text::rich(spans));
        }
        column.into()
    }
}
