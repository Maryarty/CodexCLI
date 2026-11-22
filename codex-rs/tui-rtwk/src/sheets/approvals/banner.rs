//! The banner of the threads that wait for an approval (S13, the oracle's
//! `bottom_pane/pending_thread_approvals.rs`): `! Approval needed in <thread>` per thread (three
//! at most, then `...`) over `/subagents to review requests`. The session page puts it above
//! the composer; this is its widget. Flutter: a `Column` of `Row`s of an icon and an `Expanded`
//! `Text` (the text wraps under itself, not under the icon).

use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::style::{Attrs, Span};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use super::strings;
use crate::app::theme::Tokens;

/// The threads shown by name; the rest is `...`.
const SHOWN: usize = 3;

/// The labels of the inactive threads that wait for an approval (core's
/// `screens::approvals::pending_threads`), oldest request first.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingThreadApprovals {
    pub threads: Vec<String>,
}

impl_widget!(stateless eq PendingThreadApprovals);

impl StatelessWidget for PendingThreadApprovals {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if self.threads.is_empty() {
            return column.into();
        }
        for thread in self.threads.iter().take(SHOWN) {
            column = column.child(
                Row::new()
                    .child(Padding::new(
                        EdgeInsets::only(2, 0, 0, 0),
                        Text::new(s.banner_mark).style(tokens.error().attrs(Attrs::BOLD)),
                    ))
                    .child(Expanded::new(
                        1,
                        Padding::new(
                            EdgeInsets::only(1, 0, 0, 0),
                            Text::new(s.banner_needed(thread).to_string()).wrap(TextWrap::Word),
                        ),
                    )),
            );
        }
        if self.threads.len() > SHOWN {
            column = column.child(Padding::new(
                EdgeInsets::only(4, 0, 0, 0),
                Text::new(s.banner_more).style(tokens.dim().attrs(Attrs::ITALIC)),
            ));
        }
        column
            .child(Padding::new(
                EdgeInsets::only(4, 0, 0, 0),
                Text::rich(vec![
                    Span::styled(
                        s.banner_command,
                        tokens.info().attrs(Attrs::BOLD).attrs(Attrs::DIM),
                    ),
                    Span::styled(s.banner_review, tokens.dim()),
                ]),
            ))
            .into()
    }
}
