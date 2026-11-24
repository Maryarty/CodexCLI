//! The cell of a recap (oracle `history_cell/notices.rs` `ThreadRecapHistoryCell`): `↳ Recap: `
//! hanging over the summary, then `Next: ` over the next action, every row italic and dim. When
//! the terminal cannot fit the hanging indent the heading stands on a row of its own and the text
//! takes the whole width. In Flutter it is a `Row` of a fixed gutter and the paragraphs, under a
//! `LayoutBuilder` that picks the narrow form; the gutter is not a selection.

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Attrs, Span};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::features::recap::Note;

use super::strings;
use crate::app::theme::Tokens;

/// The recap cell of a recap note; every other note is a line of words.
pub fn of(note: &Note) -> Option<AnyWidget> {
    let Note::Recap {
        summary,
        next_action,
    } = note
    else {
        return None;
    };
    Some(
        RecapCell {
            summary: summary.clone(),
            next_action: next_action.clone(),
        }
        .into(),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecapCell {
    summary: String,
    next_action: Option<String>,
}

impl_widget!(stateless eq RecapCell);

impl StatelessWidget for RecapCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let italic = tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC);
        let bold = tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC | Attrs::BOLD);
        let next = tokens
            .accent()
            .attrs(Attrs::DIM | Attrs::ITALIC | Attrs::BOLD);
        // the paragraphs: a source line each, the first of the action behind its label
        let mut paragraphs: Vec<Vec<Span>> = self
            .summary
            .lines()
            .map(|line| vec![Span::styled(line.to_owned(), italic.clone())])
            .collect();
        for (index, line) in self.next_action.iter().flat_map(|a| a.lines()).enumerate() {
            let mut spans = Vec::new();
            if index == 0 {
                spans.push(Span::styled(strings::NEXT, next.clone()));
            }
            spans.push(Span::styled(line.to_owned(), italic.clone()));
            paragraphs.push(spans);
        }
        let arrow = Span::styled(strings::ARROW, italic.clone());
        let label = Span::styled(strings::LABEL, bold.clone());
        let hang = i32::try_from(strings::INDENT.chars().count()).unwrap_or(0)
            + i32::try_from(strings::ARROW.chars().count() + strings::LABEL.chars().count())
                .unwrap_or(0);
        LayoutBuilder::new(move |_cx, constraints| {
            let width = constraints.max_width;
            let body = |paragraphs: &[Vec<Span>]| -> AnyWidget {
                let mut column = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch);
                for spans in paragraphs {
                    column = column.child(Text::rich(spans.clone()));
                }
                column.into()
            };
            if width - 2 <= hang {
                // `Keep the text readable when the terminal cannot fit the hanging indent`
                let heading = Text::rich(vec![
                    arrow.clone(),
                    Span::styled(strings::LABEL.trim_end().to_owned(), bold.clone()),
                ]);
                return Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(heading)
                    .child(body(&paragraphs))
                    .into();
            }
            let gutter = Text::rich(vec![
                Span::styled(strings::INDENT, italic.clone()),
                arrow.clone(),
                label.clone(),
            ]);
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(ExcludeSelection::new(SizedBox::new(
                    Some(hang),
                    None,
                    Some(gutter.into()),
                )))
                .child(Expanded::new(1, body(&paragraphs)))
                .into()
        })
        .into()
    }
}
