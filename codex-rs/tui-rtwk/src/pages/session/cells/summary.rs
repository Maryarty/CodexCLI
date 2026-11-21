//! One-row cells: web activity (oracle history_cell/search.rs), a viewed
//! image (patches.rs `ViewImageHistoryCell`) and session notices
//! (thread_transcript/other_items.rs). Flutter: one `Text.rich` with
//! `maxLines: 1, overflow: ellipsis` (the oracle's
//! `truncate_line_with_ellipsis_if_overflow`).

use rtwk::AnyWidget;
use rtwk::style::{Span, Style};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;

use codex_tui_core::threads::{Notice, WebAction, WebActivity};

use crate::app::theme::Tokens;

use super::strings::Strings;

fn one_row(spans: Vec<Span>) -> AnyWidget {
    Text::rich(spans)
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}

/// oracle `WebSearchCell::display_lines`; control characters read as blanks.
pub fn web(activity: &WebActivity, tokens: &Tokens, strings: &Strings) -> AnyWidget {
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

/// oracle `ViewImageHistoryCell::display_lines`.
pub fn viewed_image(name: &str, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    one_row(vec![
        Span::styled(strings.bullet, tokens.dim()),
        Span::styled(strings.viewed_image, tokens.bold()),
        Span::styled(name.replace(['\n', '\r', '\t'], strings.gap), tokens.dim()),
    ])
}

/// oracle `new_info_event` and `new_review_status_line`.
pub fn notice(notice: &Notice, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    let review = |(open, close): (&'static str, &'static str), review: &str| {
        Text::rich(vec![
            Span::new(open),
            Span::new(review.to_owned()),
            Span::new(close),
        ])
        .style(tokens.accent())
        .into()
    };
    match notice {
        Notice::ContextCompacted => Text::rich(vec![
            Span::styled(strings.bullet, tokens.dim()),
            Span::styled(strings.context_compacted, Style::new()),
        ])
        .into(),
        Notice::EnteredReview(text) => review(strings.review_started, text),
        Notice::ExitedReview(text) => review(strings.review_finished, text),
    }
}
