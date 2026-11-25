//! The banner of the threads that wait for an approval (S13, the oracle's
//! `bottom_pane/pending_thread_approvals.rs`): `! Approval needed in <thread>` per thread (three
//! at most, then `...`) over `/subagents to review requests`. The session page puts it above
//! the composer; this is its widget. Flutter: a `Column` of `Row`s of an icon and an `Expanded`
//! `Text` (the text wraps under itself, not under the icon).

use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::provider::select;
use rtwk::style::{Attrs, Span};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::app::{RequestQueue, SessionSlice};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::approvals::{labels_of, waiting_threads};

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

/// The banner over the composer of the chat: the threads other than the one shown that wait for
/// an approval (the oracle's `App::refresh_pending_thread_approvals`), read from the request
/// queue, the command center's thread list and the shown thread.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingThreadsBanner;

impl_widget!(stateless eq PendingThreadsBanner);

impl StatelessWidget for PendingThreadsBanner {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let waiting = select::<RequestQueue, Vec<ThreadId>, _>(cx, waiting_threads);
        let active =
            select::<SessionSlice, Option<ThreadId>, _>(cx, |session| session.active.clone());
        let threads =
            select::<ScreenStates, Vec<codex_app_server_protocol::Thread>, _>(cx, |states| {
                states.agents.threads.clone()
            });
        PendingThreadApprovals {
            threads: labels_of(&waiting, &threads, active.as_ref()),
        }
        .into()
    }
}
