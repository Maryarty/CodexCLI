//! The web activity cell (oracle history_cell/search.rs `WebSearchCell`).

use rtwk::style::Span;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{WebAction, WebActivity};

use crate::app::theme::Tokens;

use super::one_row::one_row;
use super::strings::Strings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebCell {
    pub activity: WebActivity,
}

impl_widget!(stateless eq WebCell);

/// The words of the row: the header, what joins it to the detail, the detail
/// (control characters of it read as blanks).
fn words(activity: &WebActivity, strings: &Strings) -> (&'static str, &'static str, String) {
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
    (header, separator, detail)
}

/// What the row says without its bullet (oracle `WebSearchCell::summary`): a turn's group of
/// calls names its last call so.
pub fn summary(activity: &WebActivity, strings: &Strings) -> String {
    let (header, separator, detail) = words(activity, strings);
    let mut text = header.to_owned();
    if !detail.is_empty() {
        text.extend([separator, &detail]);
    }
    text
}

/// The row's text: the bullet is the row's first span.
pub fn text(activity: &WebActivity, strings: &Strings) -> String {
    let (header, separator, detail) = words(activity, strings);
    let mut text = [strings.bullet, header].concat();
    if !detail.is_empty() {
        text.extend([separator, &detail]);
    }
    text
}

impl StatelessWidget for WebCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let (header, separator, detail) = words(&self.activity, strings);
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
