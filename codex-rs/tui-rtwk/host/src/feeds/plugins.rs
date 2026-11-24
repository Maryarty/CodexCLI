//! The plugins slice's host half: what `/plugins` reads of the config (the features it depends
//! on, the home directory the personal marketplace is under, and the marketplaces the user
//! configured: the ones a tab can remove; the git ones it can upgrade).

use codex_core::config::Config;
use codex_features::Feature;
use codex_tui_core::features::plugins::Fact;

use super::{Event, Feed, FeedCx};

/// The slice's fact, at start.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    if matches!(event, Event::Startup) {
        out.push(Feed::fact(fact(cx.config)));
    }
}

fn fact(config: &Config) -> Fact {
    let stack = &config.config_layer_stack;
    let user = stack.effective_user_config();
    let git = |name: &str| {
        stack
            .get_active_user_layer()
            .and_then(|layer| layer.config.get("marketplaces"))
            .and_then(|marketplaces| marketplaces.get(name))
            .and_then(|marketplace| marketplace.get("source_type"))
            .and_then(|kind| kind.as_str())
            .is_some_and(|kind| kind == "git")
    };
    let configured: Vec<(String, bool)> = user
        .as_ref()
        .and_then(|user| user.get("marketplaces"))
        .and_then(|marketplaces| marketplaces.as_table())
        .map(|table| table.keys().map(|name| (name.clone(), git(name))).collect())
        .unwrap_or_default();
    Fact::config(
        config.features.enabled(Feature::Plugins),
        config.features.enabled(Feature::RemotePlugin),
        config.features.enabled(Feature::PluginSharing),
        std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned()),
        configured,
    )
}
