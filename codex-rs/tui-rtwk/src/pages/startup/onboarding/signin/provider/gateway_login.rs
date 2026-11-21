//! The gateway's own sign-in once it is the selected provider
//! (`onboarding/gateway_login.rs`, `SignInState::GatewayOAuth`,
//! `GatewayOAuthFailed`): the title, the page the browser opens (a link) and
//! how to quit or retry. In Flutter these are `Text` rows of the step's body;
//! the model's state decides which.

use rtwk::AnyWidget;
use rtwk::style::Span;

use super::super::{Line, Look, paragraph};
use super::strings::Strings;

/// `Starting gateway sign-in…`, or the title and the page's link, and the
/// way out.
pub(in super::super) fn browser(
    strings: &'static Strings,
    look: &Look,
    url: Option<&str>,
) -> AnyWidget {
    let title = if url.is_some() {
        strings.gateway_oauth_browser
    } else {
        strings.gateway_oauth_starting
    };
    let mut lines: Vec<Line> = vec![vec![Span::styled(title, look.key.clone())]];
    if let Some(url) = url {
        lines.push(vec![
            Span::styled(url.to_owned(), look.cyan.clone())
                .underline()
                .link(url.to_owned()),
        ]);
    }
    lines.push(vec![Span::styled(
        strings.gateway_oauth_quit,
        look.dim.clone(),
    )]);
    paragraph(lines)
}

/// The failure, red, and how to retry.
pub(in super::super) fn failed(strings: &'static Strings, look: &Look, message: &str) -> AnyWidget {
    paragraph(vec![
        vec![Span::styled(message.to_owned(), look.red.clone())],
        vec![Span::styled(strings.gateway_oauth_retry, look.dim.clone())],
    ])
}
