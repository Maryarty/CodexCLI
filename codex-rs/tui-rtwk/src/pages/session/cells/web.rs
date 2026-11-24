//! The web activity cell (oracle history_cell/search.rs `WebSearchCell`): one clipped row in
//! the chat (`display_lines`), the wrapped rows under a `• ` gutter in the detailed transcript
//! (`transcript_lines`, a `PrefixedWrappedHistoryCell`).

use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{WebAction, WebActivity};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::one_row::one_row;
use super::strings::Strings;
use super::text::Part;
use super::tool_rows::glyph_row;
use codex_tui_core::threads::Item;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebCell {
    pub activity: WebActivity,
    /// The detailed transcript (Ctrl+T): the summary wraps.
    pub detailed: bool,
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

/// The texts of the row: the bullet is the row's first span in the chat, a gutter of its own in
/// the detailed transcript.
pub fn texts(activity: &WebActivity, strings: &Strings, detailed: bool) -> Vec<Part> {
    let (header, separator, detail) = words(activity, strings);
    let mut text = String::from(header);
    if !detail.is_empty() {
        text.extend([separator, &detail]);
    }
    if detailed {
        vec![Part::gutter("• "), Part::text(text)]
    } else {
        vec![Part::text([strings.bullet, &text].concat())]
    }
}

impl StatelessWidget for WebCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let (header, separator, detail) = words(&self.activity, strings);
        let mut spans = Vec::new();
        if !self.detailed {
            spans.push(Span::styled(strings.bullet, tokens.dim()));
        }
        spans.push(Span::styled(header, tokens.bold()));
        if !detail.is_empty() {
            spans.push(Span::new(separator));
            spans.push(Span::new(detail));
        }
        if self.detailed {
            return glyph_row("• ", Text::rich(spans).into(), tokens.dim());
        }
        one_row(spans)
    }
}

/// A web search.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Web(activity) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        WebCell {
            activity: activity.clone(),
            detailed: env.detailed,
        }
        .into(),
    )
}
