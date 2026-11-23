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
    Alone { ui }.into()
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
            "alone_tasks",
            "the composer alone, with two tasks the task search found"
        ),
    ]
}
