//! What the footer says instead of its passive hints: a flash line or the
//! hint override (oracle `chat_composer.rs` footer render: `flash_visible`,
//! `hint_override`, `footer_hint_items_line`). The lead takes the left side;
//! the indicators of the right side (the context meter, the modes) stay while
//! both fit (`can_show_left_with_context`), and only on the passive line.

use rtwk::AnyWidget;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::Span;
use rtwk::widgets::Text;

use super::flash::{Flash, Tone};
use super::hints::Kind;
use super::{Form, INDENT, RIGHT_CLEAR, Spec, room_of};
use crate::widgets::status_line::SEPARATOR;

/// The left side's replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Lead {
    Flash(Flash),
    /// `(key, label)` items; none draws nothing.
    Hints(Vec<(String, String)>),
}

impl Spec {
    /// The lead as one spelling; `None` for an empty hint list.
    fn lead_form(&self, lead: &Lead, width: &dyn Fn(&str) -> usize) -> Option<Form> {
        let spans = match lead {
            Lead::Flash(flash) => vec![match flash.tone {
                Tone::Plain => Span::new(flash.text.clone()),
                Tone::Error => Span::styled(flash.text.clone(), self.tokens.error()),
            }],
            Lead::Hints(items) => {
                let mut spans = Vec::new();
                for (index, (key, label)) in items.iter().enumerate() {
                    if index > 0 {
                        spans.push(Span::styled(SEPARATOR, self.tokens.dim()));
                    }
                    spans.push(Span::styled(key.clone(), self.tokens.bold()));
                    spans.push(Span::styled(
                        format!("{}{label}", self.own.key_gap),
                        self.tokens.dim(),
                    ));
                }
                spans
            }
        };
        if spans.is_empty() {
            return None;
        }
        Some(Form {
            cells: spans.iter().map(|span| width(&span.text)).sum(),
            widget: Text::rich_line(spans).into(),
        })
    }

    /// The footer at `total` cells with `lead` on the left; `passive` lets the
    /// right-side indicators show beside it.
    pub(super) fn lead_row(
        &self,
        lead: &Lead,
        passive: bool,
        width: &dyn Fn(&str) -> usize,
        total: usize,
    ) -> AnyWidget {
        let lead = self.lead_form(lead, width);
        let room = room_of(total);
        let lead_cells = lead.as_ref().map_or(0, |form| form.cells);
        let mut right: Vec<Form> = Vec::new();
        if passive {
            for (kind, forms) in self.forms(width) {
                // the warning notice yields to a flash and a hint override (`show_warning_notice`)
                if !kind.left()
                    && kind != Kind::Warning
                    && let Some(form) = forms.into_iter().next()
                {
                    right.push(form);
                }
            }
        }
        let join = width(SEPARATOR);
        let right_cells = right.iter().map(|form| form.cells).sum::<usize>()
            + join * right.len().saturating_sub(1);
        // the right side is dropped whole when it does not fit beside the lead
        if lead_cells + usize::from(lead_cells > 0) + right_cells > room {
            right.clear();
        }
        let mut line = Row::new().child(SizedBox::new(Some(INDENT), Some(1), None));
        if right.is_empty() {
            if let Some(lead) = lead {
                line = line.child(Expanded::new(1, lead.widget));
            }
            return line.into();
        }
        if let Some(lead) = lead {
            line = line.child(SizedBox::new(
                i32::try_from(lead.cells).ok(),
                Some(1),
                Some(lead.widget),
            ));
        }
        line = line.child(Expanded::new(1, SizedBox::shrink()));
        for (index, form) in right.into_iter().enumerate() {
            if index > 0 {
                line = line.child(Text::line(SEPARATOR).style(self.tokens.dim()));
            }
            line = line.child(SizedBox::new(
                i32::try_from(form.cells).ok(),
                Some(1),
                Some(form.widget),
            ));
        }
        line.child(SizedBox::new(Some(RIGHT_CLEAR), None, None))
            .into()
    }
}
