//! The small commands' host half: what `/debug-config` and `/ide` read of the host: the config's
//! layers and requirements as rows, and whether an IDE answers (its ipc is not carried: none
//! does).

use codex_tui_core::features::small_cmds::Fact;

use super::{Event, Feed, FeedCx};

/// The facts, at start and after every write of the config file.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    if matches!(event, Event::Startup | Event::ConfigWritten) {
        out.push(Feed::fact(Fact::debug_config(
            crate::debug_config::rows(cx.config)
                .into_iter()
                .map(|row| (row.text, row.style)),
        )));
        out.push(Feed::fact(Fact::ide_unavailable(
            crate::small::OPEN_IDE_HINT.to_owned(),
            crate::small::OPEN_IDE_HINT.to_owned(),
        )));
    }
}
