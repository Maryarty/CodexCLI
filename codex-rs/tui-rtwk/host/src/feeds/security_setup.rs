//! The security-setup slice's host half: the server's notice of this account, if it has one, and
//! the word that what the reminder holds is stale when the account is announced again.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::AppEv;
use codex_tui_core::features::security_setup::Fact;

use super::{Event, Feed, FeedCx};
use crate::in_process::sender;

/// The notice at start, and again (after invalidating the reminder) when the account was
/// announced again.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    let announced = matches!(
        event,
        Event::Notification(ServerNotification::AccountUpdated(_))
    );
    if !(matches!(event, Event::Startup) || announced)
        || !crate::security_setup::eligible(cx.config, cx.remote_workspace)
    {
        return;
    }
    if announced {
        out.push(Feed::fact(Fact::Invalidated));
    }
    let (config, ids, send) = (cx.config.clone(), cx.ids.clone(), sender(cx.handle.clone()));
    out.push(Feed::Job(Box::pin(async move {
        let notice = crate::security_setup::fetch(&config, &ids, send).await?;
        Some(AppEv::Feature(
            Fact::loaded(
                notice.account,
                notice.user,
                notice.title,
                notice.description,
                notice.label,
                notice.url,
            )
            .into(),
        ))
    })));
}
