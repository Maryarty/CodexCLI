//! The host's feeds (ADR-012 "the host's feeds", arch-pillars P7): what the host reads of this
//! machine and of the connection, unasked, and tells a feature slice as its `Fact`. One file per
//! slice, `feeds/<f>.rs`, which names that slice's types and no other's; `in_process.rs` names no
//! slice, it tells `run` what happened (an [`Event`]) and delivers what comes back.
//!
//! The list is `host_feed_list!`: a slice that has a host half is a line of it, and its file has
//! one function, `feed(event, cx, out)`, which says what it wants to tell at which event (a
//! `match` on the events it follows; the others are no business of it). A fact is told at once, a
//! job is a read of the app server whose answer is told when it is there.
//!
//! In Flutter this is the repository layer's streams: each repository subscribes to the app
//! lifecycle events it cares about and the widgets read it through providers.

use std::sync::Arc;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::ServerNotification;
use codex_core::config::Config;
use codex_tui_core::app::AppEv;
use codex_tui_core::features::FeatureFact;
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::Target;

/// What happened that a feed may tell a slice about.
pub(crate) enum Event<'a> {
    /// The first connection is up: everything the slices read at start.
    Startup,
    /// The connection came back after a loss.
    Reconnected,
    /// A connection to a server other than the in-process one is up (at start, and again after a
    /// reconnect): the version it reports and whether it is the local background server.
    Connection {
        server_version: Option<&'a str>,
        local_daemon: bool,
    },
    /// A write of the config file went through: `FeedCx::config` is the config as it is now.
    ConfigWritten,
    /// The server's notification, before the app takes it.
    Notification(&'a ServerNotification),
}

/// What a feed reads to know what to tell.
pub(crate) struct FeedCx<'a> {
    pub config: &'a Arc<Config>,
    pub target: Target,
    /// The workspace is a remote machine's (not the daemon's).
    pub remote_workspace: bool,
    pub ids: &'a Ids,
    pub handle: &'a AppServerRequestHandle,
}

/// What a feed tells.
pub(crate) enum Feed {
    /// A fact, told now.
    Fact(FeatureFact),
    /// A read of the server whose answer (if any) is told when it arrives.
    Job(Boxed<Option<AppEv>>),
}

impl Feed {
    /// The fact `fact` of a slice.
    pub(crate) fn fact(fact: impl Into<FeatureFact>) -> Self {
        Feed::Fact(fact.into())
    }
}

/// The slices that have a host half: a `mod` and a call each.
macro_rules! host_feed_list {
    ($($feed:ident;)*) => {
        $(mod $feed;)*

        /// What every slice's feed tells at `event`, in the list's order.
        pub(crate) fn run(event: &Event<'_>, cx: &FeedCx<'_>) -> Vec<Feed> {
            let mut out = Vec::new();
            $($feed::feed(event, cx, &mut out);)*
            out
        }
    };
}

host_feed_list! {
    plugins;
    settings;
    feedback;
    small_cmds;
    security_setup;
    windows_sandbox;
    daemon;
}
