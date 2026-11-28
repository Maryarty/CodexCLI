//! The composer alone (`composer/alone`) on a skill, plugin or task catalog: the states
//! of the oracle's composer harness after `set_skill_mentions`, `set_plugin_mentions` and
//! `on_task_search_result` (bottom_pane/chat_composer.rs `configure_*`). The catalogs
//! arrive as the popovers seed lines of each fixture, as the host's would.

use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, EntryOpen};
use crate::pages::session::composer::alone::Alone;

/// The composer and its footer over the seeded thread's draft.
fn alone(seeded: &Seeded) -> rtwk::AnyWidget {
    let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
    Alone::new(ui).into()
}

macro_rules! catalog {
    ($name:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $name, ".jsonl")),
            open: EntryOpen::Route(alone),
        }
    };
}

pub fn entries() -> Vec<DebugEntry> {
    vec![
        catalog!(
            "alone_skills",
            "the composer alone, with the skills of the oracle's skill popup tests"
        ),
        catalog!(
            "alone_skills_rustdoc",
            "the composer alone, with the skill `rustdoc` of the oracle's typed-punctuation test"
        ),
        catalog!(
            "alone_skills_confirmed",
            "the composer alone, with three skills and the picks the host remembers"
        ),
        catalog!(
            "alone_plugins",
            "the composer alone, with the plugins of the oracle's `@` plugin tests"
        ),
        catalog!(
            "alone_plugin_sample",
            "the composer alone, with the plugin `Sample Plugin` (skills, MCP server, app)"
        ),
        catalog!(
            "alone_plugin_opaque",
            "the composer alone, with the plugin `Postman` of an opaque config name"
        ),
        catalog!(
            "alone_skills_default",
            "the composer alone, with four skills and the plugin of the default `@` popup"
        ),
        catalog!(
            "alone_mentions_ten",
            "the composer alone, with ten skills of the oracle's scrolled `$` popup"
        ),
        catalog!(
            "alone_mention_types",
            "the composer alone, with a skill and an app named Google Calendar and a plugin"
        ),
        catalog!(
            "alone_tasks",
            "the composer alone, with two tasks the task search found"
        ),
    ]
}
