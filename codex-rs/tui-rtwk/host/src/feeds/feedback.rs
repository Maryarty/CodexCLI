//! The feedback slice's host half: what `/feedback` reads of the config and the connection
//! (whether uploads are on, whether the workspace is remote: no ZIP can be saved here, the Codex
//! home the ZIPs go under) and its audience (an `@openai.com` account is an employee).

use codex_tui_core::app::AppEv;
use codex_tui_core::features::feedback::Fact;
use codex_tui_rtwk::services::backend::rpc::Ids;

use super::{Event, Feed, FeedCx};
use crate::in_process::sender;
use crate::startup::Target;

/// The config's fact at start and after every write, the account's at start and again when the
/// connection is back.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    if matches!(event, Event::Startup | Event::ConfigWritten) {
        out.push(Feed::fact(Fact::config(
            cx.config.feedback_enabled,
            cx.target == Target::Remote,
            Some(cx.config.codex_home.to_string_lossy().into_owned()),
        )));
    }
    if matches!(event, Event::Startup | Event::Reconnected) {
        let (ids, send): (Ids, _) = (cx.ids.clone(), sender(cx.handle.clone()));
        out.push(Feed::Job(Box::pin(async move {
            let email = crate::facts::account_email(ids, send).await;
            Some(AppEv::Feature(Fact::account(email.as_deref()).into()))
        })));
    }
}
