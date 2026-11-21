//! The web activity cell (oracle history_cell/search.rs `WebSearchCell`).

use rtwk::style::Span;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{WebAction, WebActivity};

use crate::app::theme::Tokens;

use super::one_row::one_row;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebCell {
    pub activity: WebActivity,
}

impl_widget!(stateless eq WebCell);

impl StatelessWidget for WebCell {
    /// Control characters of the detail read as blanks.
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let activity = &self.activity;
        let detail = activity.detail.replace(['\n', '\r', '\t'], strings.gap);
        let done = activity.completed;
        let (header, separator) = match activity.action {
            WebAction::OpenPage => (
                match (done, detail.is_empty()) {
                    (true, true) => strings.opened_page,
                    (true, false) => strings.opened,
                    (false, true) => strings.opening_page,
                    (false, false) => strings.opening,
                },
                strings.gap,
            ),
            WebAction::FindInPage { pattern } => (
                match (done, pattern) {
                    (true, true) => strings.searched_for,
                    (true, false) => strings.searched_page,
                    (false, true) => strings.searching_for,
                    (false, false) => strings.searching_page,
                },
                strings.gap,
            ),
            WebAction::Search { known: false } if !done && detail.is_empty() => {
                (strings.browsing, strings.gap)
            }
            WebAction::Search { .. } => (
                if done {
                    strings.searched_web
                } else {
                    strings.searching_web
                },
                strings.web_for,
            ),
        };
        let mut spans = vec![
            Span::styled(strings.bullet, tokens.dim()),
            Span::styled(header, tokens.bold()),
        ];
        if !detail.is_empty() {
            spans.push(Span::new(separator));
            spans.push(Span::new(detail));
        }
        one_row(spans)
    }
}
